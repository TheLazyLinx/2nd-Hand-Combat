/**
 * Shared PixiJS map canvas: owns the WebGL Application, camera pan/zoom, cached static terrain
 * presentation (surfaces, elevation shading, contour lines, obstacles), and world/screen mapping.
 *
 * Invariant: presentation only — never modifies simulation state or imports Worker/UI modules.
 */

import { Application, Container, Graphics } from 'pixi.js';
import type { Catalog, Point, Scenario } from '../types';

export interface Camera {
  x: number;
  y: number;
  zoom: number;
}

export interface ViewBounds {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
}

export interface PointerCallbacks {
  onClick: (x: number, y: number) => void;
  onHover: (cellX: number, cellY: number) => void;
  onPointerWorld?: (point: Point | null) => void;
  onViewportChange?: () => void;
  onGestureStart?: (point: Point) => void;
  onGestureMove?: (point: Point) => void;
  onGestureEnd?: (point: Point) => void;
  onCameraChange?: (camera: Camera) => void;
  isEditorActive?: () => boolean;
}

export class MapView {
  readonly app = new Application();
  readonly scene = new Container();
  readonly terrain = new Graphics();

  camera: Camera = { x: 0, y: 0, zoom: 1 };
  viewBounds: ViewBounds = { minX: -Infinity, maxX: Infinity, minY: -Infinity, maxY: Infinity };
  destroyed = false;

  private observer?: ResizeObserver;
  private scenario?: Scenario;
  private mapRef?: Scenario['map'];
  private overlayRef?: boolean;
  private maxTextureSize = 4096;
  private terrainResolution = 0;
  private pointer: number | null = null;
  private panning = false;
  private suppressClick = false;
  private lastPointer = { x: 0, y: 0 };

  constructor(
    readonly host: HTMLElement,
    private readonly callbacks: PointerCallbacks,
  ) {}

  async init(layers: Container[]) {
    await this.app.init({
      preference: 'webgl',
      background: '#a49b77',
      antialias: true,
      resolution: Math.min(window.devicePixelRatio || 1, 2),
      autoDensity: true,
    });
    if (this.destroyed) {
      this.app.destroy(true);
      return;
    }
    const gl = (this.app.renderer as unknown as { gl?: WebGLRenderingContext }).gl;
    if (gl) this.maxTextureSize = Math.min(4096, gl.getParameter(gl.MAX_TEXTURE_SIZE));
    this.host.appendChild(this.app.canvas);
    this.app.canvas.setAttribute('aria-label', 'Battlefield map');
    this.scene.addChild(this.terrain, ...layers);
    this.app.stage.addChild(this.scene);
    this.app.canvas.addEventListener('click', this.click);
    this.app.canvas.addEventListener('pointermove', this.hover);
    this.app.canvas.addEventListener('pointerdown', this.pointerDown);
    this.app.canvas.addEventListener('pointerup', this.pointerUp);
    this.app.canvas.addEventListener('pointercancel', this.pointerUp);
    this.app.canvas.addEventListener('wheel', this.wheel, { passive: false });
    this.app.canvas.addEventListener('contextmenu', this.contextMenu);
    this.observer = new ResizeObserver(() => this.resize());
    this.observer.observe(this.host);
    this.resize();
  }

  get ready(): boolean {
    return !!this.app.renderer && !this.destroyed;
  }

  get currentScenario(): Scenario | undefined {
    return this.scenario;
  }

  syncTerrain(scenario: Scenario, catalog: Catalog, overlay: boolean) {
    this.scenario = scenario;
    if (!this.ready) return;
    if (scenario.map !== this.mapRef || overlay !== this.overlayRef) {
      const dimensionsChanged =
        !this.mapRef ||
        this.mapRef.width !== scenario.map.width ||
        this.mapRef.height !== scenario.map.height;
      this.mapRef = scenario.map;
      this.overlayRef = overlay;
      if (dimensionsChanged) {
        this.camera = { x: scenario.map.width / 2, y: scenario.map.height / 2, zoom: 1 };
      }
      this.drawTerrain(scenario, catalog, overlay);
      this.resize();
    }
  }

  setCamera(camera: Camera) {
    if (
      camera.x === this.camera.x &&
      camera.y === this.camera.y &&
      camera.zoom === this.camera.zoom
    ) {
      return;
    }
    this.camera = { ...camera };
    this.resize();
  }

  fit() {
    if (!this.scenario) return;
    this.camera = { x: this.scenario.map.width / 2, y: this.scenario.map.height / 2, zoom: 1 };
    this.resize();
    this.callbacks.onCameraChange?.({ ...this.camera });
  }

  zoomBy(factor: number) {
    this.zoomAt(factor, this.host.clientWidth / 2, this.host.clientHeight / 2);
  }

  inView(x: number, y: number): boolean {
    const b = this.viewBounds;
    return x >= b.minX && x <= b.maxX && y >= b.minY && y <= b.maxY;
  }

  isPointNearViewport(point: Point): boolean {
    const x = point.x * this.scene.scale.x + this.scene.x;
    const y = point.y * this.scene.scale.y + this.scene.y;
    return (
      x >= -20 && y >= -20 && x <= this.host.clientWidth + 20 && y <= this.host.clientHeight + 20
    );
  }

  private drawTerrain(scenario: Scenario, catalog: Catalog, overlay: boolean) {
    const { map } = scenario;
    this.terrain.clear();
    const colors = new Map(catalog.surfaces.map((surface) => [surface.id, surface.color]));
    const min = Math.min(...map.elevations);
    const max = Math.max(...map.elevations);
    for (let y = 0; y < map.height; y++) {
      for (let x = 0; x < map.width; x++) {
        const surface = map.surfaces[y * map.width + x];
        this.terrain.rect(x, y, 1, 1).fill(colors.get(surface) ?? 0xaaa27f);
        if (overlay && max > min) {
          const a = map.elevations[y * (map.width + 1) + x];
          const b = map.elevations[y * (map.width + 1) + x + 1];
          const c = map.elevations[(y + 1) * (map.width + 1) + x];
          const d = map.elevations[(y + 1) * (map.width + 1) + x + 1];
          // Sample the same two linear triangles as the simulation for smooth visual shading.
          const subdivisions = map.width * map.height > 4096 ? 1 : 4;
          for (let sy = 0; sy < subdivisions; sy++) {
            for (let sx = 0; sx < subdivisions; sx++) {
              const u = (sx + 0.5) / subdivisions;
              const v = (sy + 0.5) / subdivisions;
              const elevation =
                u >= v ? a + (b - a) * u + (d - b) * v : a + (d - c) * u + (c - a) * v;
              this.terrain
                .rect(
                  x + sx / subdivisions,
                  y + sy / subdivisions,
                  1 / subdivisions,
                  1 / subdivisions,
                )
                .fill({ color: 0xffffff, alpha: (0.22 * (elevation - min)) / (max - min) });
            }
          }
        }
        if (surface === 'brush' || surface === 'woodland') {
          this.terrain
            .circle(x + 0.3, y + 0.3, 0.06)
            .circle(x + 0.7, y + 0.65, 0.08)
            .fill({ color: 0x263b27, alpha: 0.45 });
        } else if (surface === 'mud') {
          this.terrain
            .moveTo(x + 0.2, y + 0.7)
            .lineTo(x + 0.5, y + 0.4)
            .stroke({ color: 0x463c30, alpha: 0.35, width: 0.05 });
        }
      }
    }
    if (overlay) {
      // Intersect each constant-height plane with the map's NW-SE triangles.
      type Vertex = { x: number; y: number; z: number };
      const contour = (vertices: Vertex[], level: number) => {
        const points: Point[] = [];
        for (let i = 0; i < 3; i++) {
          const a = vertices[i];
          const b = vertices[(i + 1) % 3];
          if ((a.z < level && b.z >= level) || (b.z < level && a.z >= level)) {
            const t = (level - a.z) / (b.z - a.z);
            points.push({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t });
          }
        }
        if (points.length === 2)
          this.terrain.moveTo(points[0].x, points[0].y).lineTo(points[1].x, points[1].y);
      };
      for (let y = 0; y < map.height; y++) {
        for (let x = 0; x < map.width; x++) {
          const at = (vx: number, vy: number): Vertex => ({
            x: vx,
            y: vy,
            z: map.elevations[vy * (map.width + 1) + vx],
          });
          const a = at(x, y);
          const b = at(x + 1, y);
          const c = at(x, y + 1);
          const d = at(x + 1, y + 1);
          const lo = Math.ceil(Math.min(a.z, b.z, c.z, d.z));
          const hi = Math.floor(Math.max(a.z, b.z, c.z, d.z));
          // Keep contour work bounded for steep, large authored maps.
          const interval = Math.max(1, Math.ceil(((max - min) * map.width * map.height) / 100000));
          for (let level = Math.ceil(lo / interval) * interval; level <= hi; level += interval) {
            contour([a, b, d], level);
            contour([a, d, c], level);
          }
        }
      }
      this.terrain.stroke({ color: 0x343e30, alpha: 0.45, width: 0.05 });
      for (let x = 0; x <= map.width; x++) this.terrain.moveTo(x, 0).lineTo(x, map.height);
      for (let y = 0; y <= map.height; y++) this.terrain.moveTo(0, y).lineTo(map.width, y);
      this.terrain.stroke({ color: 0x404a38, alpha: 0.22, width: 0.025 });
    }
    for (const obstacle of map.obstacles) {
      const full = obstacle.heightMetres >= catalog.fullObstacleHeight;
      this.terrain
        .rect(obstacle.x + 0.04, obstacle.y + 0.04, obstacle.width - 0.08, obstacle.height - 0.08)
        .fill(full ? 0x555b4b : 0x91815f)
        .stroke({ color: 0x373e34, width: 0.07 });
      if (!full) {
        this.terrain
          .moveTo(obstacle.x + 0.1, obstacle.y + 0.5)
          .lineTo(obstacle.x + obstacle.width - 0.1, obstacle.y + 0.5)
          .stroke({ color: 0xe5d6b1, width: 0.08 });
      }
    }
    this.terrain.updateCacheTexture();
  }

  resize() {
    if (this.destroyed || !this.app.renderer || !this.scenario) return;
    const width = Math.max(1, this.host.clientWidth);
    const height = Math.max(1, this.host.clientHeight);
    this.app.renderer.resize(width, height);
    const scale =
      Math.min(width / this.scenario.map.width, height / this.scenario.map.height) *
      this.camera.zoom;
    const halfX = width / (2 * scale);
    const halfY = height / (2 * scale);
    this.camera.x =
      halfX >= this.scenario.map.width / 2
        ? this.scenario.map.width / 2
        : Math.max(halfX, Math.min(this.scenario.map.width - halfX, this.camera.x));
    this.camera.y =
      halfY >= this.scenario.map.height / 2
        ? this.scenario.map.height / 2
        : Math.max(halfY, Math.min(this.scenario.map.height - halfY, this.camera.y));
    this.scene.position.set(width / 2 - this.camera.x * scale, height / 2 - this.camera.y * scale);
    this.scene.scale.set(scale);
    const pad = 1.5;
    this.viewBounds = {
      minX: -this.scene.x / scale - pad,
      maxX: (width - this.scene.x) / scale + pad,
      minY: -this.scene.y / scale - pad,
      maxY: (height - this.scene.y) / scale + pad,
    };
    this.callbacks.onViewportChange?.();
    // The map is static between edits; cache it at the current display resolution.
    const resolution = Math.max(
      0.25,
      Math.min(
        Math.ceil(scale * this.app.renderer.resolution),
        this.maxTextureSize / Math.max(this.scenario.map.width, this.scenario.map.height),
      ),
    );
    if (resolution !== this.terrainResolution) {
      this.terrainResolution = resolution;
      this.terrain.cacheAsTexture({ resolution, antialias: true });
    }
  }

  private zoomAt(factor: number, x: number, y: number) {
    if (!this.scenario) return;
    const old = this.scene.scale.x;
    const worldX = (x - this.scene.x) / old;
    const worldY = (y - this.scene.y) / old;
    this.camera.zoom = Math.max(1, Math.min(16, this.camera.zoom * factor));
    const scale =
      Math.min(
        this.host.clientWidth / this.scenario.map.width,
        this.host.clientHeight / this.scenario.map.height,
      ) * this.camera.zoom;
    this.camera.x = worldX - (x - this.host.clientWidth / 2) / scale;
    this.camera.y = worldY - (y - this.host.clientHeight / 2) / scale;
    this.resize();
    this.callbacks.onCameraChange?.({ ...this.camera });
  }

  private coordinates(event: MouseEvent, clamp = false): Point | null {
    if (!this.scenario) return null;
    const rect = this.app.canvas.getBoundingClientRect();
    let x = (event.clientX - rect.left - this.scene.x) / this.scene.scale.x;
    let y = (event.clientY - rect.top - this.scene.y) / this.scene.scale.y;
    if (clamp) {
      x = Math.max(0, Math.min(this.scenario.map.width - 0.001, x));
      y = Math.max(0, Math.min(this.scenario.map.height - 0.001, y));
    }
    return x >= 0 && y >= 0 && x < this.scenario.map.width && y < this.scenario.map.height
      ? { x, y }
      : null;
  }

  private click = (event: MouseEvent) => {
    if (this.suppressClick) {
      this.suppressClick = false;
      return;
    }
    if (this.callbacks.isEditorActive?.() || event.button !== 0) return;
    const point = this.coordinates(event);
    if (point) this.callbacks.onClick(point.x, point.y);
  };

  private hover = (event: PointerEvent) => {
    if (this.pointer === event.pointerId) {
      if (this.panning) {
        this.camera.x -= (event.clientX - this.lastPointer.x) / this.scene.scale.x;
        this.camera.y -= (event.clientY - this.lastPointer.y) / this.scene.scale.y;
        this.resize();
        this.callbacks.onCameraChange?.({ ...this.camera });
      } else {
        const point = this.coordinates(event, true);
        if (point) this.callbacks.onGestureMove?.(point);
      }
      this.lastPointer = { x: event.clientX, y: event.clientY };
    }
    const point = this.coordinates(event);
    this.callbacks.onPointerWorld?.(point);
    if (point) this.callbacks.onHover(Math.floor(point.x), Math.floor(point.y));
  };

  private pointerDown = (event: PointerEvent) => {
    const point = this.coordinates(event);
    if (!point) return;
    this.panning = event.button === 1 || event.button === 2;
    if (!this.panning && (!this.callbacks.isEditorActive?.() || event.button !== 0)) return;
    event.preventDefault();
    this.pointer = event.pointerId;
    this.lastPointer = { x: event.clientX, y: event.clientY };
    this.app.canvas.setPointerCapture(event.pointerId);
    if (!this.panning) this.callbacks.onGestureStart?.(point);
  };

  private pointerUp = (event: PointerEvent) => {
    if (this.pointer !== event.pointerId) return;
    const point = this.coordinates(event, true);
    if (!this.panning && point) this.callbacks.onGestureEnd?.(point);
    if (this.panning) {
      this.suppressClick = true;
      setTimeout(() => {
        this.suppressClick = false;
      }, 0);
    }
    this.pointer = null;
    this.panning = false;
  };

  private wheel = (event: WheelEvent) => {
    event.preventDefault();
    const rect = this.app.canvas.getBoundingClientRect();
    this.zoomAt(
      event.deltaY < 0 ? 1.2 : 1 / 1.2,
      event.clientX - rect.left,
      event.clientY - rect.top,
    );
  };

  private contextMenu = (event: MouseEvent) => event.preventDefault();

  destroy(onBeforeAppDestroy?: () => void) {
    this.destroyed = true;
    this.observer?.disconnect();
    if (this.app.renderer) {
      onBeforeAppDestroy?.();
      this.app.canvas.removeEventListener('click', this.click);
      this.app.canvas.removeEventListener('pointermove', this.hover);
      this.app.canvas.removeEventListener('pointerdown', this.pointerDown);
      this.app.canvas.removeEventListener('pointerup', this.pointerUp);
      this.app.canvas.removeEventListener('pointercancel', this.pointerUp);
      this.app.canvas.removeEventListener('wheel', this.wheel);
      this.app.canvas.removeEventListener('contextmenu', this.contextMenu);
      this.app.destroy(true, { children: true });
    }
  }
}
