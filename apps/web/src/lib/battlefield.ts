import { Application, Container, Graphics, Text } from 'pixi.js';
import type { Catalog, Scenario, Snapshot, UnitPlacement, UnitSnapshot, Point } from './types';

export interface Camera { x: number; y: number; zoom: number }
export interface EditorOptions {
  onStart: (point: Point) => void; onMove: (point: Point) => void; onEnd: (point: Point) => void;
  camera?: Camera; onCameraChange?: (camera: Camera) => void;
  issues?: { severity: 'error' | 'warning'; position: Point | null }[]; steepEdges?: Point[];
  brushRadius?: number; brushAtVertices?: boolean; resizePreview?: {width: number; height: number};
  selection?: {kind: 'unit' | 'obstacle' | 'patrol'; index: number} | null;
}

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
  private mapRef?: Scenario['map'];
  private overlayRef?: boolean;
  private annotations = new Graphics();
  private patrol = new Graphics();
  private editor?: EditorOptions;
  private camera: Camera = {x: 0, y: 0, zoom: 1};
  private pointer: number | null = null;
  private panning = false;
  private suppressClick = false;
  private maxTextureSize = 4096;
  private brush = new Graphics();
  private editorFeedback = new Graphics();
  private lastPointer = {x: 0, y: 0};
  private terrainResolution = 0;
  private snapshotTick: number | null = null;
  private running = false;
  private effectUntil = 0;
  private effectLines: Array<{ from: UnitView; to: UnitView }> = [];

  constructor(private host: HTMLElement, private onClick: (x: number, y: number) => void, private onHover: (x: number, y: number) => void) {}

  async init() {
    await this.app.init({ preference: 'webgl', background: '#a49b77', antialias: true, resolution: Math.min(window.devicePixelRatio || 1, 2), autoDensity: true });
    if (this.destroyed) { this.app.destroy(true); return; }
    const gl = (this.app.renderer as unknown as {gl?: WebGLRenderingContext}).gl;
    if (gl) this.maxTextureSize = Math.min(4096, gl.getParameter(gl.MAX_TEXTURE_SIZE));
    this.host.appendChild(this.app.canvas);
    this.app.canvas.setAttribute('aria-label', 'Battlefield map');
    this.scene.addChild(this.terrain, this.patrol, this.annotations, this.editorFeedback, this.brush, this.range, this.effects);
    this.app.stage.addChild(this.scene);
    this.app.canvas.addEventListener('click', this.click);
    this.app.canvas.addEventListener('pointermove', this.hover);
    this.app.canvas.addEventListener('pointerdown', this.pointerDown);
    this.app.canvas.addEventListener('pointerup', this.pointerUp);
    this.app.canvas.addEventListener('pointercancel', this.pointerUp);
    this.app.canvas.addEventListener('wheel', this.wheel, {passive:false});
    this.app.canvas.addEventListener('contextmenu', this.contextMenu);
    this.observer = new ResizeObserver(() => this.resize());
    this.observer.observe(this.host);
    this.app.ticker.add(this.animate);
    this.resize();
  }

  update(scenario: Scenario, snapshot: Snapshot | null, selected: number | null, showHp: boolean, maxHp: number, running: boolean, catalog: Catalog, overlay: boolean) {
    this.scenario = scenario;
    this.running = running;
    if (!this.app.renderer || this.destroyed) return;
    if (scenario.map !== this.mapRef || overlay !== this.overlayRef) {
      const dimensionsChanged = !this.mapRef || this.mapRef.width !== scenario.map.width || this.mapRef.height !== scenario.map.height;
      this.mapRef = scenario.map; this.overlayRef = overlay;
      if (dimensionsChanged) this.camera = {x: scenario.map.width / 2, y: scenario.map.height / 2, zoom: 1};
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
      const appearance = JSON.stringify([scenario.factions.indexOf(placement.faction), selected === placement.id, state?.behavior, state?.hp, showHp, maxHp]);
      if (view.appearance !== appearance) {
        view.appearance = appearance;
        this.drawUnit(view.graphic, placement, state, selected === placement.id, showHp, maxHp);
      }
    }
    this.snapshotTick = snapshot?.tick ?? null;
    this.range.clear();
    const selection = !snapshot ? scenario.units.find(unit => unit.id === selected) : undefined;
    if (selection) this.range.circle(selection.position.x, selection.position.y, selection.doctrine.engagementDistance).stroke({color:selection.faction === scenario.factions[0] ? 0x315f62 : 0xa24f32, alpha:0.45, width:0.06});
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
        const subdivisions = map.width * map.height > 4096 ? 1 : 4;
        for (let sy = 0; sy < subdivisions; sy++) for (let sx = 0; sx < subdivisions; sx++) {
          const u = (sx + 0.5) / subdivisions, v = (sy + 0.5) / subdivisions;
          const elevation = u >= v ? a + (b - a) * u + (d - b) * v : a + (d - c) * u + (c - a) * v;
          this.terrain.rect(x + sx / subdivisions, y + sy / subdivisions, 1 / subdivisions, 1 / subdivisions).fill({color: 0xffffff, alpha: 0.22 * (elevation - min) / (max - min)});
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
        // Keep contour work bounded for steep, large authored maps.
        const interval = Math.max(1, Math.ceil((max - min) * map.width * map.height / 100000));
        for (let level = Math.ceil(lo / interval) * interval; level <= hi; level += interval) { contour([a, b, d], level); contour([a, d, c], level); }
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
    const color = placement.faction === this.scenario?.factions[0] ? 0x315f62 : 0xa24f32;
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
    const scale = Math.min(width / this.scenario.map.width, height / this.scenario.map.height) * this.camera.zoom;
    const halfX = width / (2 * scale), halfY = height / (2 * scale);
    this.camera.x = halfX >= this.scenario.map.width / 2 ? this.scenario.map.width / 2 : Math.max(halfX, Math.min(this.scenario.map.width - halfX, this.camera.x));
    this.camera.y = halfY >= this.scenario.map.height / 2 ? this.scenario.map.height / 2 : Math.max(halfY, Math.min(this.scenario.map.height - halfY, this.camera.y));
    this.scene.position.set(width / 2 - this.camera.x * scale, height / 2 - this.camera.y * scale);
    this.scene.scale.set(scale);
    // The map is static between edits; cache it at the current display resolution.
    const resolution = Math.max(0.25, Math.min(Math.ceil(scale * this.app.renderer.resolution), this.maxTextureSize / Math.max(this.scenario.map.width, this.scenario.map.height)));
    if (resolution !== this.terrainResolution) {
      this.terrainResolution = resolution;
      this.terrain.cacheAsTexture({resolution, antialias: true});
    }
  }

  setEditor(options: EditorOptions) {
    this.editor = options;
    if (options.camera && (options.camera.x !== this.camera.x || options.camera.y !== this.camera.y || options.camera.zoom !== this.camera.zoom)) {
      this.camera = {...options.camera}; this.resize();
    }
    this.annotations.clear();
    this.editorFeedback.clear();
    const map = this.scenario?.map;
    if (map && options.resizePreview) {
      const w = Math.min(map.width, Math.max(0, options.resizePreview.width)), h = Math.min(map.height, Math.max(0, options.resizePreview.height));
      if (w < map.width) this.editorFeedback.rect(w, 0, map.width - w, map.height).fill({color:0x222222, alpha:0.5});
      if (h < map.height) this.editorFeedback.rect(0, h, w, map.height - h).fill({color:0x222222, alpha:0.5});
      this.editorFeedback.rect(0, 0, w, h).stroke({color:0xffffff, width:0.12});
    }
    const selection = options.selection;
    if (map && selection?.kind === 'obstacle') { const o = map.obstacles[selection.index]; if (o) this.editorFeedback.rect(o.x, o.y, o.width, o.height).stroke({color:0xffffff, width:0.12}); }
    if (map && selection?.kind === 'patrol') { const p = map.patrol[selection.index]; if (p) this.editorFeedback.circle(p.x, p.y, 0.4).stroke({color:0xffffff, width:0.12}); }
    this.patrol.clear();
    const points = this.scenario?.map.patrol ?? [];
    if (points.length) {
      this.patrol.moveTo(points[0].x, points[0].y);
      for (const point of points.slice(1)) this.patrol.lineTo(point.x, point.y);
      this.patrol.stroke({color:0xead58f, width:0.1});
      for (const point of points) this.patrol.circle(point.x, point.y, 0.2).fill(0xead58f).stroke({color:0x30382d, width:0.06});
    }
    const visible = (point: Point) => { const x = point.x * this.scene.scale.x + this.scene.x, y = point.y * this.scene.scale.y + this.scene.y; return x >= -20 && y >= -20 && x <= this.host.clientWidth + 20 && y <= this.host.clientHeight + 20; };
    // Dense cliffs can contain tens of thousands of edges. Aggregate only at
    // display resolution so every affected area retains a warning marker.
    const bins = new Set<string>();
    const binPixels = 16;
    for (const point of options.steepEdges ?? []) {
      if (!visible(point)) continue;
      const screenX = point.x * this.scene.scale.x + this.scene.x;
      const screenY = point.y * this.scene.scale.y + this.scene.y;
      if ((options.steepEdges?.length ?? 0) <= 2000) {
        this.annotations.circle(point.x, point.y, 0.16);
        bins.add(`${point.x},${point.y}`);
        continue;
      }
      const bx = Math.floor(screenX / binPixels), by = Math.floor(screenY / binPixels);
      const key = `${bx},${by}`;
      if (bins.has(key)) continue;
      bins.add(key);
      const x = ((bx + 0.5) * binPixels - this.scene.x) / this.scene.scale.x;
      const y = ((by + 0.5) * binPixels - this.scene.y) / this.scene.scale.y;
      this.annotations.circle(x, y, Math.min(0.16, 4 / this.scene.scale.x));
    }
    if (bins.size) this.annotations.fill({color:0xf0aa34, alpha:0.85});
    for (const issue of options.issues ?? []) if (issue.position && visible(issue.position)) this.annotations.circle(issue.position.x, issue.position.y, 0.45).stroke({color:issue.severity === 'error' ? 0xde4039 : 0xf0aa34, width:0.12});
  }
  fit() {
    if (!this.scenario) return;
    this.camera = {x:this.scenario.map.width / 2, y:this.scenario.map.height / 2, zoom:1};
    this.resize(); this.editor?.onCameraChange?.({...this.camera});
  }
  zoomBy(factor: number) { this.zoomAt(factor, this.host.clientWidth / 2, this.host.clientHeight / 2); }
  private zoomAt(factor: number, x: number, y: number) {
    if (!this.scenario) return;
    const old = this.scene.scale.x;
    const worldX = (x - this.scene.x) / old, worldY = (y - this.scene.y) / old;
    this.camera.zoom = Math.max(1, Math.min(16, this.camera.zoom * factor));
    const scale = Math.min(this.host.clientWidth / this.scenario.map.width, this.host.clientHeight / this.scenario.map.height) * this.camera.zoom;
    this.camera.x = worldX - (x - this.host.clientWidth / 2) / scale;
    this.camera.y = worldY - (y - this.host.clientHeight / 2) / scale;
    this.resize(); this.editor?.onCameraChange?.({...this.camera});
  }
  private coordinates(event: MouseEvent, clamp = false) {
    if (!this.scenario) return null;
    const rect = this.app.canvas.getBoundingClientRect();
    let x = (event.clientX - rect.left - this.scene.x) / this.scene.scale.x;
    let y = (event.clientY - rect.top - this.scene.y) / this.scene.scale.y;
    if (clamp) { x = Math.max(0, Math.min(this.scenario.map.width - 0.001, x)); y = Math.max(0, Math.min(this.scenario.map.height - 0.001, y)); }
    return x >= 0 && y >= 0 && x < this.scenario.map.width && y < this.scenario.map.height ? {x, y} : null;
  }
  private click = (event: MouseEvent) => { if (this.suppressClick) { this.suppressClick = false; return; } if (this.editor || event.button !== 0) return; const point = this.coordinates(event); if (point) this.onClick(point.x, point.y); };
  private hover = (event: PointerEvent) => {
    if (this.pointer === event.pointerId) {
      if (this.panning) { this.camera.x -= (event.clientX - this.lastPointer.x) / this.scene.scale.x; this.camera.y -= (event.clientY - this.lastPointer.y) / this.scene.scale.y; this.resize(); this.editor?.onCameraChange?.({...this.camera}); }
      else { const point = this.coordinates(event, true); if (point) this.editor?.onMove(point); }
      this.lastPointer = {x:event.clientX, y:event.clientY};
    }
    const point = this.coordinates(event);
    this.brush.clear();
    if (point && this.editor) {
      const radius = Math.max(0, this.editor.brushRadius ?? 0);
      const vertex = this.editor.brushAtVertices ?? false;
      const x = vertex ? Math.round(point.x) : Math.floor(point.x), y = vertex ? Math.round(point.y) : Math.floor(point.y);
      this.brush.rect(x - radius - (vertex ? 0.15 : 0), y - radius - (vertex ? 0.15 : 0), 2 * radius + (vertex ? 0.3 : 1), 2 * radius + (vertex ? 0.3 : 1)).stroke({color:0xffffff, width:0.08, alpha:0.9});
    }
    if (point) this.onHover(Math.floor(point.x), Math.floor(point.y));
  };
  private pointerDown = (event: PointerEvent) => {
    const point = this.coordinates(event); if (!point) return;
    this.panning = event.button === 1 || event.button === 2;
    if (!this.panning && (!this.editor || event.button !== 0)) return;
    event.preventDefault(); this.pointer = event.pointerId; this.lastPointer = {x:event.clientX,y:event.clientY};
    this.app.canvas.setPointerCapture(event.pointerId); if (!this.panning) this.editor?.onStart(point);
  };
  private pointerUp = (event: PointerEvent) => {
    if (this.pointer !== event.pointerId) return;
    const point = this.coordinates(event, true); if (!this.panning && point) this.editor?.onEnd(point);
    if (this.panning) { this.suppressClick = true; setTimeout(() => { this.suppressClick = false; }, 0); }
    this.pointer = null; this.panning = false;
  };
  private wheel = (event: WheelEvent) => { event.preventDefault(); const rect = this.app.canvas.getBoundingClientRect(); this.zoomAt(event.deltaY < 0 ? 1.2 : 1 / 1.2, event.clientX - rect.left, event.clientY - rect.top); };
  private contextMenu = (event: MouseEvent) => event.preventDefault();

  destroy() {
    this.destroyed = true; this.observer?.disconnect();
    if (this.app.renderer) {
      this.app.ticker.remove(this.animate);
      this.app.canvas.removeEventListener('click', this.click);
      this.app.canvas.removeEventListener('pointermove', this.hover);
      this.app.canvas.removeEventListener('pointerdown', this.pointerDown);
      this.app.canvas.removeEventListener('pointerup', this.pointerUp);
      this.app.canvas.removeEventListener('pointercancel', this.pointerUp);
      this.app.canvas.removeEventListener('wheel', this.wheel);
      this.app.canvas.removeEventListener('contextmenu', this.contextMenu);
      this.app.destroy(true, {children:true});
    }
  }
}
