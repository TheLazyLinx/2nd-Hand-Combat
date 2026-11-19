/**
 * Public Battlefield renderer facade composing the shared `MapView` (camera & terrain cache),
 * `BattleLayer` (batched sprites, HP overlays, interpolation, viewport culling), and
 * `EditorOverlay` (MapMaker gizmos, brush preview, steep-slope and validation annotations).
 */

import type { LiveFrame } from './compact-snapshot';
import { BattleLayer } from './renderer/battle-layer';
import { EditorOverlay, type EditorOptions } from './renderer/editor-overlay';
import { MapView, type Camera } from './renderer/map-view';
import type { Catalog, Scenario } from './types';

export type { Camera, EditorOptions };

/** A presentation of scenario and snapshots. Interpolation never changes simulation state. */
export class Battlefield {
  private readonly mapView: MapView;
  private readonly battleLayer: BattleLayer;
  private readonly editorOverlay: EditorOverlay;

  constructor(
    host: HTMLElement,
    onClick: (x: number, y: number) => void,
    onHover: (x: number, y: number) => void,
  ) {
    this.mapView = new MapView(host, {
      onClick,
      onHover,
      onPointerWorld: (point) => this.editorOverlay.updateBrush(point),
      onViewportChange: () => this.battleLayer.applyCulling(),
      onGestureStart: (point) => this.editorOverlay.options?.onStart(point),
      onGestureMove: (point) => this.editorOverlay.options?.onMove(point),
      onGestureEnd: (point) => this.editorOverlay.options?.onEnd(point),
      onCameraChange: (camera) => this.editorOverlay.options?.onCameraChange?.(camera),
      isEditorActive: () => !!this.editorOverlay.options,
    });
    this.battleLayer = new BattleLayer(this.mapView);
    this.editorOverlay = new EditorOverlay(this.mapView);
  }

  async init() {
    await this.mapView.init([...this.editorOverlay.layers, ...this.battleLayer.layers]);
    if (this.mapView.ready) {
      this.mapView.app.ticker.add(this.battleLayer.animate);
    }
  }

  update(
    scenario: Scenario,
    snapshot: LiveFrame | null,
    selected: number | null,
    showHp: boolean,
    running: boolean,
    catalog: Catalog,
    overlay: boolean,
  ) {
    this.mapView.syncTerrain(scenario, catalog, overlay);
    this.battleLayer.update(scenario, snapshot, selected, showHp, running, catalog);
  }

  setEditor(options: EditorOptions) {
    this.editorOverlay.setEditor(options, this.mapView.currentScenario);
  }

  fit() {
    this.mapView.fit();
  }

  zoomBy(factor: number) {
    this.mapView.zoomBy(factor);
  }

  destroy() {
    this.mapView.destroy(() => {
      this.mapView.app.ticker.remove(this.battleLayer.animate);
      this.battleLayer.destroyTextures();
    });
  }
}
