/**
 * Battle presentation layer: texture-batched unit sprites, batched HP/selection overlay,
 * engagement range indicator, firing tracers, interpolation, and camera viewport culling.
 *
 * Invariant: consumes snapshots (`Snapshot` or `CompactSnapshotView`) read-only; never changes state.
 */

import { Container, Graphics, Rectangle, Sprite, Text, Texture } from 'pixi.js';
import { isCompactFrame, type LiveFrame } from '../compact-snapshot';
import { factionColor, unitDefinition } from '../factions';
import type { Catalog, Scenario } from '../types';
import type { MapView } from './map-view';

const DEAD_COLOR = 0x777368;
const MAX_ALL_UNIT_LABELS = 64;

interface UnitView {
  id: number;
  sprite: Sprite;
  label: Text | null;
  from: { x: number; y: number };
  target: { x: number; y: number };
  x: number;
  y: number;
  started: number;
  textureColor: number;
  hp: number;
  maxHp: number;
  hasState: boolean;
  shots: number;
  visible: boolean;
}

export class BattleLayer {
  readonly range = new Graphics();
  readonly unitsLayer = new Container();
  readonly hpLayer = new Graphics();
  readonly labelsLayer = new Container();
  readonly effects = new Graphics();

  private unitTextures = new Map<number, Texture>();
  private units = new Map<number, UnitView>();
  private selectedId: number | null = null;
  private showHp = true;
  private snapshotTick: number | null = null;
  private running = false;
  private effectUntil = 0;
  private effectLines: Array<{ from: UnitView; to: UnitView }> = [];

  constructor(private readonly mapView: MapView) {}

  get layers(): Container[] {
    return [this.range, this.unitsLayer, this.hpLayer, this.labelsLayer, this.effects];
  }

  private unitTexture(color: number): Texture {
    const cached = this.unitTextures.get(color);
    if (cached) return cached;
    const g = new Graphics();
    // Bake in 100x100 px template centered at (50, 50); sprites scale by 0.01 into world units.
    g.circle(50, 50, 38).fill({ color: 0x20271f, alpha: 0.3 });
    g.circle(50, 45, 30).fill(color).stroke({ color: 0xf4ead0, width: 7 });
    g.circle(50, 42, 10).fill(0xe5d6b1);
    const texture = this.mapView.app.renderer.generateTexture({
      target: g,
      frame: new Rectangle(0, 0, 100, 100),
      resolution: 2,
      antialias: true,
    });
    g.destroy();
    this.unitTextures.set(color, texture);
    return texture;
  }

  private createLabel(id: number): Text {
    const label = new Text({
      text: String(id),
      resolution: 2,
      style: { fontFamily: 'sans-serif', fontSize: 16, fill: 0x292d21, fontWeight: 'bold' },
    });
    label.scale.set(0.025);
    label.anchor.set(0.5, 0);
    this.labelsLayer.addChild(label);
    return label;
  }

  private showUnitLabels(): boolean {
    return this.units.size <= MAX_ALL_UNIT_LABELS && this.mapView.scene.scale.x >= 8;
  }

  private syncUnitVisibility(view: UnitView, showLabels: boolean) {
    const visible = this.mapView.inView(view.x, view.y);
    view.visible = visible;
    view.sprite.visible = visible;
    if (visible) {
      view.sprite.position.set(view.x, view.y);
    }
    if (view.label) {
      const labelVisible = visible && (showLabels || view.id === this.selectedId);
      view.label.visible = labelVisible;
      if (labelVisible) view.label.position.set(view.x, view.y + 0.4);
    }
  }

  private drawUnitOverlays() {
    this.hpLayer.clear();
    let visibleUnits = 0;
    let culledUnits = 0;
    for (const view of this.units.values()) {
      if (!view.visible) {
        culledUnits++;
        continue;
      }
      visibleUnits++;
      if (view.id === this.selectedId) {
        this.hpLayer.circle(view.x, view.y, 0.48).stroke({ color: 0xf6e3ad, width: 0.07 });
      }
      if (this.showHp && view.hasState) {
        const ratio = Math.max(0, Math.min(1, view.hp / view.maxHp));
        this.hpLayer.rect(view.x - 0.4, view.y - 0.65, 0.8, 0.09).fill(0x302f25);
        this.hpLayer.rect(view.x - 0.4, view.y - 0.65, 0.8 * ratio, 0.09).fill(0xe4c77e);
      }
    }
    this.mapView.host.dataset.visibleUnits = String(visibleUnits);
    this.mapView.host.dataset.culledUnits = String(culledUnits);
    this.mapView.host.dataset.unitTextures = String(this.unitTextures.size);
  }

  applyCulling() {
    const showLabels = this.showUnitLabels();
    for (const view of this.units.values()) {
      this.syncUnitVisibility(view, showLabels);
    }
    this.drawUnitOverlays();
  }

  update(
    scenario: Scenario,
    snapshot: LiveFrame | null,
    selected: number | null,
    showHp: boolean,
    running: boolean,
    catalog: Catalog,
  ) {
    this.running = running;
    this.selectedId = selected;
    this.showHp = showHp;
    if (!this.mapView.ready) return;

    const live = new Set(scenario.units.map((unit) => unit.id));
    for (const [id, view] of this.units) {
      if (!live.has(id)) {
        view.sprite.destroy();
        view.label?.destroy();
        this.units.delete(id);
      }
    }
    const compact = isCompactFrame(snapshot) ? snapshot : null;
    const snapshots =
      snapshot && !isCompactFrame(snapshot)
        ? new Map(snapshot.units.map((unit) => [unit.id, unit]))
        : null;
    const newTick = snapshot !== null && snapshot.tick !== this.snapshotTick;
    const now = performance.now();
    const fired: Array<{ shooter: number; target: number }> = [];
    const allocateLabels = scenario.units.length <= MAX_ALL_UNIT_LABELS;
    const showLabels = allocateLabels && this.mapView.scene.scale.x >= 8;

    for (const placement of scenario.units) {
      let hasState = false;
      let stateX = placement.position.x;
      let stateY = placement.position.y;
      let stateHp = 0;
      let stateDead = false;
      let stateShots = 0;
      let stateTarget: number | null = null;

      if (compact) {
        const idx = compact.indexOf(placement.id);
        if (idx !== undefined) {
          hasState = true;
          stateX = compact.x(idx);
          stateY = compact.y(idx);
          stateHp = compact.hp(idx);
          stateDead = compact.behavior(idx) === 'dead';
          stateShots = compact.shotsFired(idx);
          stateTarget = compact.targetId(idx);
        }
      } else if (snapshots) {
        const st = snapshots.get(placement.id);
        if (st) {
          hasState = true;
          stateX = st.position.x;
          stateY = st.position.y;
          stateHp = st.hp;
          stateDead = st.behavior === 'dead';
          stateShots = st.shotsFired;
          stateTarget = st.targetId;
        }
      }

      const baseColor = factionColor(scenario.factions, placement.faction);
      const textureColor = stateDead ? DEAD_COLOR : baseColor;
      const unitMaxHp = unitDefinition(catalog, placement.definitionId).maxHp;
      let view = this.units.get(placement.id);
      if (!view) {
        const sprite = new Sprite(this.unitTexture(textureColor));
        sprite.anchor.set(0.5, 0.5);
        sprite.scale.set(0.01);
        this.unitsLayer.addChild(sprite);
        const label =
          allocateLabels || selected === placement.id ? this.createLabel(placement.id) : null;
        view = {
          id: placement.id,
          sprite,
          label,
          from: { x: stateX, y: stateY },
          target: { x: stateX, y: stateY },
          x: stateX,
          y: stateY,
          started: now,
          textureColor,
          hp: stateHp,
          maxHp: unitMaxHp,
          hasState,
          shots: stateShots,
          visible: true,
        };
        this.units.set(placement.id, view);
      }
      if (!view.label && (allocateLabels || selected === placement.id)) {
        view.label = this.createLabel(placement.id);
      }
      if (view.textureColor !== textureColor) {
        view.textureColor = textureColor;
        view.sprite.texture = this.unitTexture(textureColor);
      }
      if (!snapshot) {
        view.from.x = placement.position.x;
        view.from.y = placement.position.y;
        view.target.x = placement.position.x;
        view.target.y = placement.position.y;
        view.x = placement.position.x;
        view.y = placement.position.y;
        view.shots = 0;
      } else if (newTick && hasState) {
        view.from.x = view.x;
        view.from.y = view.y;
        view.target.x = stateX;
        view.target.y = stateY;
        view.started = now;
        if (!running) {
          view.x = stateX;
          view.y = stateY;
        }
        if (stateShots > view.shots && stateTarget !== null) {
          fired.push({ shooter: placement.id, target: stateTarget });
        }
        view.shots = stateShots;
      }
      if (!running && hasState) {
        view.x = stateX;
        view.y = stateY;
      }
      view.hp = stateHp;
      view.maxHp = unitMaxHp;
      view.hasState = hasState;
      this.syncUnitVisibility(view, showLabels);
    }

    this.drawUnitOverlays();
    this.snapshotTick = snapshot?.tick ?? null;
    this.range.clear();
    const selection = !snapshot ? scenario.units.find((unit) => unit.id === selected) : undefined;
    if (selection) {
      this.range
        .circle(selection.position.x, selection.position.y, selection.doctrine.engagementDistance)
        .stroke({
          color: factionColor(scenario.factions, selection.faction),
          alpha: 0.45,
          width: 0.06,
        });
    }
    if (fired.length) {
      this.effectLines = fired.flatMap(({ shooter, target }) => {
        const from = this.units.get(shooter);
        const to = this.units.get(target);
        return from && to ? [{ from, to }] : [];
      });
      this.effectUntil = now + 250;
    }
    if (!running) {
      this.effects.clear();
      this.effectLines = [];
    }
  }

  animate = () => {
    if (!this.running) return;
    const now = performance.now();
    const showLabels = this.showUnitLabels();
    for (const view of this.units.values()) {
      const t = Math.min(1, (now - view.started) / 100);
      view.x = view.from.x + (view.target.x - view.from.x) * t;
      view.y = view.from.y + (view.target.y - view.from.y) * t;
      this.syncUnitVisibility(view, showLabels);
    }
    this.drawUnitOverlays();
    this.effects.clear();
    if (now < this.effectUntil) {
      for (const { from, to } of this.effectLines) {
        if (!from.visible && !to.visible) continue;
        this.effects
          .moveTo(from.x, from.y)
          .lineTo(to.x, to.y)
          .stroke({ color: 0xfce4a2, width: 0.09, alpha: 0.9 });
      }
    } else {
      this.effectLines = [];
    }
  };

  destroyTextures() {
    for (const texture of this.unitTextures.values()) texture.destroy(true);
    this.unitTextures.clear();
  }
}
