<script lang="ts">
  /** Skirmish map controls, time-limit controls, terrain inspector/legend, and coordinate deploy. */
  import type { SavedScenario } from '../../lib/scenario-library';
  import type { Catalog, Scenario, TerrainCellInfo } from '../../lib/types';

  interface Props {
    catalog: Catalog;
    scenario: Scenario;
    saved: SavedScenario[];
    mapId: string;
    description: string;
    phase: 'loading' | 'setup' | 'running' | 'paused' | 'finished' | 'error';
    busy: boolean;
    terrainOverlay: boolean;
    inspectX: number;
    inspectY: number;
    inspected: TerrainCellInfo | null;
    placementActive: boolean;
    deployX: number;
    deployY: number;
    onFit: () => void;
    onZoom: (factor: number) => void;
    onChangeMap: (id: string) => void;
    onTimeLimitInput: (input: HTMLInputElement) => void;
    onToggleNoTimeLimit: (unlimited: boolean) => void;
    onOverlayChange: (enabled: boolean) => void;
    onInspectChange: (x: number, y: number) => void;
    onDeployChange: (x: number, y: number) => void;
    onPlaceAt: (x: number, y: number) => void;
  }

  let {
    catalog,
    scenario,
    saved,
    mapId,
    description,
    phase,
    busy,
    terrainOverlay,
    inspectX,
    inspectY,
    inspected,
    placementActive,
    deployX,
    deployY,
    onFit,
    onZoom,
    onChangeMap,
    onTimeLimitInput,
    onToggleNoTimeLimit,
    onOverlayChange,
    onInspectChange,
    onDeployChange,
    onPlaceAt,
  }: Props = $props();
</script>

<div class="terrain-controls">
  <button onclick={onFit}>Fit map</button><button onclick={() => onZoom(1.25)}>
    Zoom in
  </button><button onclick={() => onZoom(0.8)}> Zoom out </button>
  <label>
    Map <select
      data-testid="map-preset"
      value={mapId}
      disabled={phase !== 'setup' || busy}
      onchange={(e) => onChangeMap(e.currentTarget.value)}
    >
      {#if mapId === 'editor'}<option value="editor">MapMaker test</option>{/if}<optgroup
        label="Built-in maps"
      >
        {#each catalog.maps as map (map.id)}<option value={map.id}>
            {map.name}
          </option>{/each}
      </optgroup><optgroup label="My scenarios">
        {#each saved as entry (entry.id)}<option value={`saved:${entry.id}`}>
            {entry.document.name}
          </option>{/each}
      </optgroup>
    </select>
  </label>
  <span>{description}</span><small> Selecting a map or scenario replaces the current setup. </small>
  <label>
    Time limit (min) <input
      data-testid="time-limit-minutes"
      type="number"
      min="0.05"
      max="60"
      step="any"
      disabled={phase !== 'setup' || busy || scenario.timeLimit === null}
      value={scenario.timeLimit === null ? '' : Number((scenario.timeLimit / 60).toFixed(2))}
      onchange={(e) => onTimeLimitInput(e.currentTarget)}
    />
  </label><label>
    <input
      data-testid="no-time-limit"
      type="checkbox"
      disabled={phase !== 'setup' || busy}
      checked={scenario.timeLimit === null}
      onchange={(e) => onToggleNoTimeLimit(e.currentTarget.checked)}
    /> No time limit
  </label>
  <label>
    <input
      type="checkbox"
      checked={terrainOverlay}
      onchange={(e) => onOverlayChange(e.currentTarget.checked)}
    /> Terrain grid and elevation shading
  </label>
</div>
<div class="terrain-inspector" aria-label="Terrain inspector">
  <strong>Inspect terrain</strong><label>
    Column <input
      aria-label="Inspect column"
      type="number"
      min="0"
      max={scenario.map.width - 1}
      value={inspectX}
      oninput={(e) => onInspectChange(Number(e.currentTarget.value), inspectY)}
    />
  </label><label>
    Row <input
      aria-label="Inspect row"
      type="number"
      min="0"
      max={scenario.map.height - 1}
      value={inspectY}
      oninput={(e) => onInspectChange(inspectX, Number(e.currentTarget.value))}
    />
  </label>
  {#if inspected}<span data-testid="terrain-readout">
      {catalog.surfaces.find((surface) => surface.id === inspected.surface)?.name}: elevation
      {inspected.elevation.toFixed(1)} m | movement x{inspected.movementMultiplier.toFixed(2)} |
      {inspected.traversable ? 'traversable' : 'blocked'}
    </span>{:else}<span>Choose a valid cell.</span>{/if}
</div>
<div class="terrain-legend" aria-label="Terrain legend">
  {#each catalog.surfaces as surface (surface.id)}<span>
      <i style={`background:#${surface.color.toString(16).padStart(6, '0')}`}></i>{surface.name}
    </span>{/each}
</div>
{#if placementActive}<div class="coordinate-entry">
    <span>Or enter a grid position:</span><label>
      Column <input
        type="number"
        min="0"
        max={scenario.map.width - 1}
        value={deployX}
        oninput={(e) => onDeployChange(Number(e.currentTarget.value), deployY)}
      />
    </label><label>
      Row <input
        type="number"
        min="0"
        max={scenario.map.height - 1}
        value={deployY}
        oninput={(e) => onDeployChange(deployX, Number(e.currentTarget.value))}
      />
    </label><button
      disabled={busy ||
        !Number.isInteger(deployX) ||
        !Number.isInteger(deployY) ||
        deployX < 0 ||
        deployY < 0 ||
        deployX >= scenario.map.width ||
        deployY >= scenario.map.height}
      onclick={() => onPlaceAt(deployX + 0.5, deployY + 0.5)}
    >
      Place soldier
    </button>
  </div>{/if}
