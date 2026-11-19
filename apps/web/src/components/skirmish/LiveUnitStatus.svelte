<script lang="ts">
  /** Live soldier terrain status and inventory/sustainment readout during an active battle. */
  import { unitDefinition } from '../../lib/factions';
  import type { Catalog, UnitPlacement, UnitSnapshot } from '../../lib/types';
  import InventoryDisplay from '../InventoryDisplay.svelte';

  interface Props {
    catalog: Catalog;
    unit: UnitPlacement;
    state: UnitSnapshot;
  }

  let { catalog, unit, state }: Props = $props();
</script>

<section class="editor" data-testid="unit-terrain-status">
  <h3>{unitDefinition(catalog, unit.definitionId).name} #{unit.id}</h3>
  <p>
    {state.terrain.surface} | elevation {state.terrain.elevation.toFixed(1)} m | Actual movement x{state.terrain.movementMultiplier.toFixed(
      2,
    )}
  </p>
  <p>
    {state.terrain.revealed ? 'Revealed' : 'Not revealed'} | {state.terrain.protection === null
      ? 'No current target'
      : `${Math.round(state.terrain.protection * 100)}% directional protection against target #${state.targetId}`}
  </p>
</section>
<InventoryDisplay {catalog} inventory={state.inventory} live={state} />
