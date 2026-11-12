import { Application, Container, Graphics, Text } from 'pixi.js';
import type { Catalog, Scenario, Snapshot, UnitPlacement, UnitSnapshot } from './types';

interface UnitView {
  container: Container;
  graphic: Graphics;
  from: { x: number; y: number };
  target: { x: number; y: number };
  started: number;
  appearance: string;
  shots: number;
}

/** A presentation of scenario and snapshots. Interpolation never changes simulation state. */
export class Battlefield {
  private app = new Application();
  private scene = new Container();
  private terrain = new Graphics();
  private range = new Graphics();
  private effects = new Graphics();
  private units = new Map<number, UnitView>();
  private observer?: ResizeObserver;
  private destroyed = false;
  private scenario?: Scenario;
  private mapKey = '';
  private terrainResolution = 0;
  private snapshotTick: number | null = null;
  private running = false;
  private effectUntil = 0;
  private effectLines: Array<{ from: UnitView; to: UnitView }> = [];

  constructor(private host: HTMLElement, private onClick: (x: number, y: number) => void, private onHover: (x: number, y: number) => void) {}

  async init() {
    await this.app.init({ preference: 'webgl', background: '#a49b77', antialias: true, resolution: Math.min(window.devicePixelRatio || 1, 2), autoDensity: true });
    if (this.destroyed) { this.app.destroy(true); return; }
    this.host.appendChild(this.app.canvas);
    this.app.canvas.setAttribute('aria-label', 'Battlefield map');
    this.scene.addChild(this.terrain, this.range, this.effects);
    this.app.stage.addChild(this.scene);
    this.app.canvas.addEventListener('click', this.click);
    this.app.canvas.addEventListener('pointermove', this.hover);
    this.observer = new ResizeObserver(() => this.resize());
    this.observer.observe(this.host);
    this.app.ticker.add(this.animate);
    this.resize();
  }

  update(scenario: Scenario, snapshot: Snapshot | null, selected: number | null, showHp: boolean, maxHp: number, running: boolean, catalog: Catalog, overlay: boolean) {
    this.scenario = scenario;
    this.running = running;
    if (!this.app.renderer || this.destroyed) return;
    const mapKey = JSON.stringify([scenario.map, overlay]);
    if (mapKey !== this.mapKey) {
      this.mapKey = mapKey;
      this.drawTerrain(scenario, catalog, overlay);
      this.resize();
    }
    const live = new Set(scenario.units.map(unit => unit.id));
    for (const [id, view] of this.units) {
      if (!live.has(id)) { view.container.destroy({children: true}); this.units.delete(id); }
    }
    const snapshots = new Map((snapshot?.units ?? []).map(unit => [unit.id, unit]));
    const newTick = snapshot !== null && snapshot.tick !== this.snapshotTick;
    const now = performance.now();
    const fired: Array<{ shooter: number; target: number }> = [];
    for (const placement of scenario.units) {
      const state = snapshots.get(placement.id);
      let view = this.units.get(placement.id);
      if (!view) {
        const container = new Container();
        const graphic = new Graphics();
        // Rasterize at a readable pixel size, then scale into world units.
        const label = new Text({text: String(placement.id), resolution: 2, style:{fontFamily:'sans-serif', fontSize:16, fill:0x292d21, fontWeight:'bold'}});
        label.scale.set(0.025);
        label.anchor.set(0.5, 0); label.position.set(0, 0.4);
        container.addChild(graphic, label); this.scene.addChild(container);
        const position = state?.position ?? placement.position;
        container.position.set(position.x, position.y);
        view = {container, graphic, from:{...position}, target:{...position}, started:now, appearance:'', shots:state?.shotsFired ?? 0};
        this.units.set(placement.id, view);
      }
      if (!snapshot) {
        view.from = {...placement.position}; view.target = {...placement.position};
        view.container.position.set(placement.position.x, placement.position.y); view.shots = 0;
      } else if (newTick && state) {
        view.from = {x:view.container.x, y:view.container.y}; view.target = {...state.position}; view.started = now;
        if (!running) view.container.position.set(state.position.x, state.position.y);
        if (state.shotsFired > view.shots && state.targetId !== null) fired.push({shooter:state.id, target:state.targetId});
        view.shots = state.shotsFired;
      }
      if (!running && state) view.container.position.set(state.position.x, state.position.y);
      const appearance = JSON.stringify([placement.faction, selected === placement.id, state?.behavior, state?.hp, showHp, maxHp]);
      if (view.appearance !== appearance) {
        view.appearance = appearance;
        this.drawUnit(view.graphic, placement, state, selected === placement.id, showHp, maxHp);
      }
    }
    this.snapshotTick = snapshot?.tick ?? null;
    this.range.clear();
    const selection = !snapshot ? scenario.units.find(unit => unit.id === selected) : undefined;
    if (selection) this.range.circle(selection.position.x, selection.position.y, selection.doctrine.engagementDistance).stroke({color:selection.faction === 0 ? 0x315f62 : 0xa24f32, alpha:0.45, width:0.06});
    if (fired.length) {
      this.effectLines = fired.flatMap(({shooter, target}) => {
        const from = this.units.get(shooter), to = this.units.get(target);
        return from && to ? [{from, to}] : [];
      });
      this.effectUntil = now + 250;
    }
    if (!running) { this.effects.clear(); this.effectLines = []; }
  }

  private drawTerrain(scenario: Scenario, catalog: Catalog, overlay: boolean) {
    const {map} = scenario;
    this.terrain.clear();
    const colors = new Map(catalog.surfaces.map(surface => [surface.id, surface.color]));
    const min = Math.min(...map.elevations), max = Math.max(...map.elevations);
    for (let y = 0; y < map.height; y++) for (let x = 0; x < map.width; x++) {
      const surface = map.surfaces[y * map.width + x];
      this.terrain.rect(x, y, 1, 1).fill(colors.get(surface) ?? 0xaaa27f);
      if (overlay && max > min) {
        const a = map.elevations[y * (map.width + 1) + x], b = map.elevations[y * (map.width + 1) + x + 1];
        const c = map.elevations[(y + 1) * (map.width + 1) + x], d = map.elevations[(y + 1) * (map.width + 1) + x + 1];
        // Sample the same two linear triangles as the simulation for smooth visual shading.
        for (let sy = 0; sy < 4; sy++) for (let sx = 0; sx < 4; sx++) {
          const u = (sx + 0.5) / 4, v = (sy + 0.5) / 4;
          const elevation = u >= v ? a + (b - a) * u + (d - b) * v : a + (d - c) * u + (c - a) * v;
          this.terrain.rect(x + sx / 4, y + sy / 4, 0.25, 0.25).fill({color: 0xffffff, alpha: 0.22 * (elevation - min) / (max - min)});
        }
      }
      if (surface === 'brush' || surface === 'woodland') {
        this.terrain.circle(x + 0.3, y + 0.3, 0.06).circle(x + 0.7, y + 0.65, 0.08).fill({color: 0x263b27, alpha: 0.45});
      } else if (surface === 'mud') {
        this.terrain.moveTo(x + 0.2, y + 0.7).lineTo(x + 0.5, y + 0.4).stroke({color: 0x463c30, alpha: 0.35, width: 0.05});
      }
    }
    if (overlay) {
      // Intersect each constant-height plane with the map's NW-SE triangles.
      type Vertex = {x: number; y: number; z: number};
      const contour = (vertices: Vertex[], level: number) => {
        const points: {x: number; y: number}[] = [];
        for (let i = 0; i < 3; i++) {
          const a = vertices[i], b = vertices[(i + 1) % 3];
          if ((a.z < level && b.z >= level) || (b.z < level && a.z >= level)) {
            const t = (level - a.z) / (b.z - a.z);
            points.push({x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t});
          }
        }
        if (points.length === 2) this.terrain.moveTo(points[0].x, points[0].y).lineTo(points[1].x, points[1].y);
      };
      for (let y = 0; y < map.height; y++) for (let x = 0; x < map.width; x++) {
        const at = (vx: number, vy: number): Vertex => ({x: vx, y: vy, z: map.elevations[vy * (map.width + 1) + vx]});
        const a = at(x, y), b = at(x + 1, y), c = at(x, y + 1), d = at(x + 1, y + 1);
        const lo = Math.ceil(Math.min(a.z, b.z, c.z, d.z)), hi = Math.floor(Math.max(a.z, b.z, c.z, d.z));
        for (let level = lo; level <= hi; level++) { contour([a, b, d], level); contour([a, d, c], level); }
      }
      this.terrain.stroke({color:0x343e30, alpha:0.45, width:0.05});
      for (let x = 0; x <= map.width; x++) this.terrain.moveTo(x, 0).lineTo(x, map.height);
      for (let y = 0; y <= map.height; y++) this.terrain.moveTo(0, y).lineTo(map.width, y);
      this.terrain.stroke({color:0x404a38, alpha:0.22, width:0.025});
    }
    for (const obstacle of map.obstacles) {
      const full = obstacle.heightMetres >= catalog.fullObstacleHeight;
      this.terrain.rect(obstacle.x + 0.04, obstacle.y + 0.04, obstacle.width - 0.08, obstacle.height - 0.08)
        .fill(full ? 0x555b4b : 0x91815f).stroke({color:0x373e34, width:0.07});
      if (!full) this.terrain.moveTo(obstacle.x + 0.1, obstacle.y + 0.5).lineTo(obstacle.x + obstacle.width - 0.1, obstacle.y + 0.5).stroke({color:0xe5d6b1, width:0.08});
    }
    this.terrain.updateCacheTexture();
  }

  private drawUnit(g: Graphics, placement: UnitPlacement, state: UnitSnapshot | undefined, selected: boolean, showHp: boolean, maxHp: number) {
    const color = placement.faction === 0 ? 0x315f62 : 0xa24f32;
    g.clear().circle(0, 0, 0.38).fill({color:0x20271f, alpha:0.3});
    g.circle(0, -0.05, 0.3).fill(state?.behavior === 'Dead' ? 0x777368 : color).stroke({color:0xf4ead0, width:0.07});
    g.circle(0, -0.08, 0.1).fill(0xe5d6b1);
    if (selected) g.circle(0, 0, 0.48).stroke({color:0xf6e3ad, width:0.07});
    if (showHp && state) {
      g.rect(-0.4, -0.65, 0.8, 0.09).fill(0x302f25);
      g.rect(-0.4, -0.65, 0.8 * Math.max(0, Math.min(1, state.hp / maxHp)), 0.09).fill(0xe4c77e);
    }
  }

  private animate = () => {
    if (!this.running) return;
    const now = performance.now();
    for (const view of this.units.values()) {
      const t = Math.min(1, (now - view.started) / 100);
      view.container.position.set(view.from.x + (view.target.x - view.from.x) * t, view.from.y + (view.target.y - view.from.y) * t);
    }
    this.effects.clear();
    if (now < this.effectUntil) {
      for (const {from, to} of this.effectLines) this.effects.moveTo(from.container.x, from.container.y).lineTo(to.container.x, to.container.y).stroke({color:0xfce4a2, width:0.09, alpha:0.9});
    } else this.effectLines = [];
  };

  private resize() {
    if (this.destroyed || !this.app.renderer || !this.scenario) return;
    const width = Math.max(1, this.host.clientWidth), height = Math.max(1, this.host.clientHeight);
    this.app.renderer.resize(width, height);
    const scale = Math.min(width / this.scenario.map.width, height / this.scenario.map.height);
    this.scene.position.set((width - this.scenario.map.width * scale) / 2, (height - this.scenario.map.height * scale) / 2);
    this.scene.scale.set(scale);
    // The map is static between edits; cache it at the current display resolution.
    const resolution = Math.max(1, Math.ceil(scale * this.app.renderer.resolution));
    if (resolution !== this.terrainResolution) {
      this.terrainResolution = resolution;
      this.terrain.cacheAsTexture({resolution, antialias: true});
    }
  }

  private coordinates(event: MouseEvent) {
    if (!this.scenario) return null;
    const rect = this.app.canvas.getBoundingClientRect();
    const scale = Math.min(rect.width / this.scenario.map.width, rect.height / this.scenario.map.height);
    const x = (event.clientX - rect.left - (rect.width - scale * this.scenario.map.width) / 2) / scale;
    const y = (event.clientY - rect.top - (rect.height - scale * this.scenario.map.height) / 2) / scale;
    return x >= 0 && y >= 0 && x < this.scenario.map.width && y < this.scenario.map.height ? {x, y} : null;
  }
  private click = (event: MouseEvent) => { const point = this.coordinates(event); if (point) this.onClick(point.x, point.y); };
  private hover = (event: MouseEvent) => { const point = this.coordinates(event); if (point) this.onHover(Math.floor(point.x), Math.floor(point.y)); };

  destroy() {
    this.destroyed = true; this.observer?.disconnect();
    if (this.app.renderer) {
      this.app.ticker.remove(this.animate);
      this.app.canvas.removeEventListener('click', this.click);
      this.app.canvas.removeEventListener('pointermove', this.hover);
      this.app.destroy(true, {children:true});
    }
  }
}
