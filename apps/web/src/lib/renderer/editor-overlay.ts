/**
 * MapMaker editor presentation overlay: patrol path, resize preview mask, obstacle/patrol
 * selection outline, display-binned steep-slope markers, validation issue rings, and brush cursor.
 */

import { Container, Graphics } from 'pixi.js';
import type { AnalysisIssue, Point, Scenario } from '../types';
import type { Camera, MapView } from './map-view';

export interface EditorOptions {
  onStart: (point: Point) => void;
  onMove: (point: Point) => void;
  onEnd: (point: Point) => void;
  camera?: Camera;
  onCameraChange?: (camera: Camera) => void;
  issues?: { severity: AnalysisIssue['severity']; position: Point | null }[];
  steepEdges?: Point[];
  brushRadius?: number;
  brushAtVertices?: boolean;
  resizePreview?: { width: number; height: number };
  selection?: { kind: 'unit' | 'obstacle' | 'patrol'; index: number } | null;
}

export class EditorOverlay {
  readonly patrol = new Graphics();
  readonly annotations = new Graphics();
  readonly editorFeedback = new Graphics();
  readonly brush = new Graphics();

  options?: EditorOptions;

  constructor(private readonly mapView: MapView) {}

  get layers(): Container[] {
    return [this.patrol, this.annotations, this.editorFeedback, this.brush];
  }

  setEditor(options: EditorOptions, scenario: Scenario | undefined) {
    this.options = options;
    if (options.camera) {
      this.mapView.setCamera(options.camera);
    }
    this.annotations.clear();
    this.editorFeedback.clear();
    const map = scenario?.map;
    if (map && options.resizePreview) {
      const w = Math.min(map.width, Math.max(0, options.resizePreview.width));
      const h = Math.min(map.height, Math.max(0, options.resizePreview.height));
      if (w < map.width) {
        this.editorFeedback
          .rect(w, 0, map.width - w, map.height)
          .fill({ color: 0x222222, alpha: 0.5 });
      }
      if (h < map.height) {
        this.editorFeedback.rect(0, h, w, map.height - h).fill({ color: 0x222222, alpha: 0.5 });
      }
      this.editorFeedback.rect(0, 0, w, h).stroke({ color: 0xffffff, width: 0.12 });
    }
    const selection = options.selection;
    if (map && selection?.kind === 'obstacle') {
      const o = map.obstacles[selection.index];
      if (o) {
        this.editorFeedback
          .rect(o.x, o.y, o.width, o.height)
          .stroke({ color: 0xffffff, width: 0.12 });
      }
    }
    if (map && selection?.kind === 'patrol') {
      const p = map.patrol[selection.index];
      if (p) this.editorFeedback.circle(p.x, p.y, 0.4).stroke({ color: 0xffffff, width: 0.12 });
    }
    this.patrol.clear();
    const points = scenario?.map.patrol ?? [];
    if (points.length) {
      this.patrol.moveTo(points[0].x, points[0].y);
      for (const point of points.slice(1)) this.patrol.lineTo(point.x, point.y);
      this.patrol.stroke({ color: 0xead58f, width: 0.1 });
      for (const point of points) {
        this.patrol
          .circle(point.x, point.y, 0.2)
          .fill(0xead58f)
          .stroke({ color: 0x30382d, width: 0.06 });
      }
    }
    // Dense cliffs can contain tens of thousands of edges. Aggregate only at
    // display resolution so every affected area retains a warning marker.
    const bins = new Set<string>();
    const binPixels = 16;
    const scaleX = this.mapView.scene.scale.x;
    const sceneX = this.mapView.scene.x;
    const sceneY = this.mapView.scene.y;
    for (const point of options.steepEdges ?? []) {
      if (!this.mapView.isPointNearViewport(point)) continue;
      if ((options.steepEdges?.length ?? 0) <= 2000) {
        this.annotations.circle(point.x, point.y, 0.16);
        bins.add(`${point.x},${point.y}`);
        continue;
      }
      const screenX = point.x * scaleX + sceneX;
      const screenY = point.y * scaleX + sceneY;
      const bx = Math.floor(screenX / binPixels);
      const by = Math.floor(screenY / binPixels);
      const key = `${bx},${by}`;
      if (bins.has(key)) continue;
      bins.add(key);
      const x = ((bx + 0.5) * binPixels - sceneX) / scaleX;
      const y = ((by + 0.5) * binPixels - sceneY) / scaleX;
      this.annotations.circle(x, y, Math.min(0.16, 4 / scaleX));
    }
    if (bins.size) this.annotations.fill({ color: 0xf0aa34, alpha: 0.85 });
    for (const issue of options.issues ?? []) {
      if (issue.position && this.mapView.isPointNearViewport(issue.position)) {
        this.annotations
          .circle(issue.position.x, issue.position.y, 0.45)
          .stroke({ color: issue.severity === 'error' ? 0xde4039 : 0xf0aa34, width: 0.12 });
      }
    }
  }

  updateBrush(point: Point | null) {
    this.brush.clear();
    if (!point || !this.options) return;
    const radius = Math.max(0, this.options.brushRadius ?? 0);
    const vertex = this.options.brushAtVertices ?? false;
    const x = vertex ? Math.round(point.x) : Math.floor(point.x);
    const y = vertex ? Math.round(point.y) : Math.floor(point.y);
    this.brush
      .rect(
        x - radius - (vertex ? 0.15 : 0),
        y - radius - (vertex ? 0.15 : 0),
        2 * radius + (vertex ? 0.3 : 1),
        2 * radius + (vertex ? 0.3 : 1),
      )
      .stroke({ color: 0xffffff, width: 0.08, alpha: 0.9 });
  }
}
