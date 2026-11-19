<script lang="ts">
  /** Selected-entity property controls (soldier, obstacle, patrol) and live validation panel for MapMaker. */
  import { previewUnitInventory } from '../../lib/factions';
  import type {
    Catalog,
    Doctrine,
    Obstacle,
    Point,
    Scenario,
    ScenarioAnalysis,
    UnitPlacement,
  } from '../../lib/types';
  import DoctrineEditor from '../DoctrineEditor.svelte';
  import InventoryDisplay from '../InventoryDisplay.svelte';

  interface Props {
    catalog: Catalog;
    scenario: Scenario;
    selectedUnit: UnitPlacement | null;
    selectedObstacle: Obstacle | null;
    selectedPatrol: Point | null;
    selectedPatrolIndex: number | null;
    analysis: ScenarioAnalysis | null;
    checking: boolean;
    onMoveSelected: (axis: 'x' | 'y', value: number) => void;
    onChangeFaction: (faction: number) => void;
    onChangeDoctrine: (field: keyof Doctrine, value: Doctrine[keyof Doctrine]) => void;
    onClearSelection: () => void;
    onRemoveSelected: () => void;
    onSizeObstacle: (axis: 'width' | 'height', value: number) => void;
    onObstacleHeight: (heightMetres: number) => void;
    onReorderPatrol: (direction: number) => void;
  }

  let {
    catalog,
    scenario,
    selectedUnit,
    selectedObstacle,
    selectedPatrol,
    selectedPatrolIndex,
    analysis,
    checking,
    onMoveSelected,
    onChangeFaction,
    onChangeDoctrine,
    onClearSelection,
    onRemoveSelected,
    onSizeObstacle,
    onObstacleHeight,
    onReorderPatrol,
  }: Props = $props();
</script>

{#if selectedUnit || selectedPatrol || selectedObstacle}
  <div class="controls">
    <strong>Selected position</strong><label>
      X <input
        aria-label="Selected X"
        type="number"
        step="0.5"
        value={selectedUnit?.position.x ?? selectedPatrol?.x ?? selectedObstacle?.x}
        onchange={(e) => onMoveSelected('x', Number(e.currentTarget.value))}
      />
    </label><label>
      Y <input
        aria-label="Selected Y"
        type="number"
        step="0.5"
        value={selectedUnit?.position.y ?? selectedPatrol?.y ?? selectedObstacle?.y}
        onchange={(e) => onMoveSelected('y', Number(e.currentTarget.value))}
      />
    </label>
  </div>
{/if}
{#if selectedUnit}
  <div class="controls">
    <label>
      Selected faction <select
        aria-label="Selected faction"
        value={selectedUnit.faction}
        onchange={(e) => onChangeFaction(Number(e.currentTarget.value))}
      >
        {#each scenario.factions as side (side.id)}<option value={side.id}>
            {side.name}
          </option>{/each}
      </select>
    </label>
  </div>
  <DoctrineEditor
    unit={selectedUnit}
    {catalog}
    busy={false}
    onClose={onClearSelection}
    onChange={onChangeDoctrine}
    onRemove={onRemoveSelected}
  /><InventoryDisplay {catalog} inventory={previewUnitInventory(catalog, scenario, selectedUnit)} />
{/if}
{#if selectedObstacle}
  <div class="controls">
    <strong>Obstacle</strong><label>
      Width <input
        aria-label="Obstacle width"
        type="number"
        min="1"
        max={scenario.map.width - selectedObstacle.x}
        value={selectedObstacle.width}
        onchange={(e) => onSizeObstacle('width', Number(e.currentTarget.value))}
      />
    </label><label>
      Depth <input
        aria-label="Obstacle depth"
        type="number"
        min="1"
        max={scenario.map.height - selectedObstacle.y}
        value={selectedObstacle.height}
        onchange={(e) => onSizeObstacle('height', Number(e.currentTarget.value))}
      />
    </label><label>
      Height (m) <input
        type="number"
        min="0.1"
        max={catalog.editorRules.maxObstacleHeight}
        step="0.1"
        value={selectedObstacle.heightMetres}
        onchange={(e) => onObstacleHeight(Number(e.currentTarget.value))}
      />
    </label><button onclick={onRemoveSelected}>Delete obstacle</button>
  </div>
{/if}
{#if selectedPatrol && selectedPatrolIndex !== null}
  <div class="controls">
    <strong>Patrol point {selectedPatrolIndex + 1}</strong><button
      onclick={() => onReorderPatrol(-1)}
    >
      Move earlier
    </button><button onclick={() => onReorderPatrol(1)}>Move later</button><button
      onclick={onRemoveSelected}
    >
      Delete patrol point
    </button>
  </div>
{/if}
<section aria-label="Validation">
  <h2>Validation</h2>
  {#if checking}<p>Checking scenario…</p>{:else if analysis}<p>
      {analysis.canStart
        ? 'Ready to test'
        : analysis.structuralValid
          ? 'Draft can be saved; resolve issues to test.'
          : 'Fix structural issues before saving.'}
    </p>
    {#each analysis.issues as issue, index (index)}<p class:error={issue.severity === 'error'}>
        {issue.severity}: {issue.message}
      </p>{/each}{/if}
</section>
