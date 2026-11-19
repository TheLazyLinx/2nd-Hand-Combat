<script lang="ts">
  /** MapMaker tool palette, brush/coordinate controls, terrain inspector, and legend. */
  import { isBrushTool, type EditorTool } from '../../lib/editor';
  import type { Catalog, Scenario, Surface, TerrainCellInfo } from '../../lib/types';

  const TOOLS: readonly EditorTool[] = [
    'surface',
    'raise',
    'lower',
    'set',
    'smooth',
    'obstacle',
    'select',
    'patrol',
    'soldier',
  ];

  interface Props {
    catalog: Catalog;
    scenario: Scenario;
    tool: EditorTool;
    brushRadius: number;
    surface: Surface;
    amount: number;
    setValue: number;
    obstacleHeight: number;
    faction: number;
    x: number;
    y: number;
    terrainOverlay: boolean;
    inspectX: number;
    inspectY: number;
    inspected: TerrainCellInfo | null;
    onChangeTool: (tool: EditorTool) => void;
    onCoordinateAction: () => void;
  }

  let {
    catalog,
    scenario,
    tool,
    brushRadius = $bindable(),
    surface = $bindable(),
    amount = $bindable(),
    setValue = $bindable(),
    obstacleHeight = $bindable(),
    faction = $bindable(),
    x = $bindable(),
    y = $bindable(),
    terrainOverlay = $bindable(),
    inspectX = $bindable(),
    inspectY = $bindable(),
    inspected,
    onChangeTool,
    onCoordinateAction,
  }: Props = $props();
</script>

<div class="tools" aria-label="Editor tools">
  {#each TOOLS as item (item)}<button
      class:active={tool === item}
      onclick={() => onChangeTool(item)}
    >
      {item}
    </button>{/each}
</div>
<div class="controls">
  {#if isBrushTool(tool)}<label>
      Brush radius <input type="number" min="0" max="5" step="1" bind:value={brushRadius} />
    </label>{/if}
  {#if tool === 'surface'}<label>
      Surface <select bind:value={surface}>
        {#each catalog.surfaces as item (item.id)}<option value={item.id}>
            {item.name}
          </option>{/each}
      </select>
    </label>{/if}
  {#if tool === 'raise' || tool === 'lower'}<label>
      Elevation amount <input type="number" min="0.1" max="100" step="0.1" bind:value={amount} />
    </label>{/if}
  {#if tool === 'set'}<label>
      Set elevation <input
        type="number"
        min={catalog.editorRules.minElevation}
        max={catalog.editorRules.maxElevation}
        step="0.1"
        bind:value={setValue}
      />
    </label>{/if}
  {#if tool === 'obstacle'}<label>
      Obstacle height <select bind:value={obstacleHeight}>
        <option value={catalog.lowObstacleHeight}>Low</option><option
          value={catalog.fullObstacleHeight}
        >
          Full
        </option>
      </select>
    </label>{/if}
  {#if tool === 'soldier'}<label>
      Faction <select aria-label="Faction" bind:value={faction}>
        {#each scenario.factions as side (side.id)}<option value={side.id}>
            {side.name}
          </option>{/each}
      </select>
    </label>{/if}
  <label>
    Column <input
      aria-label="Edit column"
      type="number"
      min="0"
      max={scenario.map.width - 1}
      bind:value={x}
    />
  </label><label>
    Row <input
      aria-label="Edit row"
      type="number"
      min="0"
      max={scenario.map.height - 1}
      bind:value={y}
    />
  </label><button onclick={onCoordinateAction}>Apply at coordinates</button>
</div>
<div class="controls">
  <label>
    <input type="checkbox" bind:checked={terrainOverlay} /> Elevation overlay
  </label><span>Wheel to zoom; middle or right drag to pan.</span>
</div>
<div class="controls" aria-label="Terrain inspector">
  <label>
    Inspect column <input
      type="number"
      min="0"
      max={scenario.map.width - 1}
      bind:value={inspectX}
    />
  </label><label>
    Inspect row <input type="number" min="0" max={scenario.map.height - 1} bind:value={inspectY} />
  </label>{#if inspected}<span data-testid="editor-terrain-readout">
      {catalog.surfaces.find((item) => item.id === inspected.surface)?.name}: {inspected.elevation.toFixed(
        1,
      )} m | movement x{inspected.movementMultiplier.toFixed(2)} | {inspected.traversable
        ? 'traversable'
        : 'blocked'}
    </span>{/if}
</div>
<div class="controls" aria-label="Terrain legend">
  {#each catalog.surfaces as item (item.id)}<span>
      <i
        style={`display:inline-block;width:12px;height:12px;margin-right:4px;background:#${item.color.toString(16).padStart(6, '0')}`}
      ></i>{item.name}
    </span>{/each}<span>Amber: steep slope · Red: invalid placement</span>
</div>
