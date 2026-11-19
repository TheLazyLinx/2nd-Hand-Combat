<script lang="ts">
  import { factionColorHex, unitDefinition } from '../lib/factions';
  import type { Catalog, FactionDefinition, Scenario, Snapshot } from '../lib/types';
  export let faction: FactionDefinition;
  export let side: number;
  export let scenario: Scenario | null;
  export let snapshot: Snapshot | null;
  export let catalog: Catalog | null;
  export let selected: number | null;
  export let editable: boolean;
  export let busy: boolean;
  export let onSelect: (id: number) => void;
  export let onAdd: () => void;

  $: roster = scenario?.units.filter((u) => u.faction === faction.id) ?? [];
</script>

<section class="faction" data-testid={`faction-${faction.id}`}>
  <div class="faction-title">
    <div>
      <i class="dot" style={`background:${factionColorHex(faction.color)}`}></i><strong>
        {faction.name}
      </strong><small>FACTION {String(side + 1).padStart(2, '0')}</small>
    </div>
    <span>
      {roster.length} / {catalog?.maxPerFaction ?? 8}
    </span>
  </div>
  {#each roster as unit (unit.id)}
    {@const unitDef = catalog ? unitDefinition(catalog, unit.definitionId) : null}
    <button
      class:active={selected === unit.id}
      class="roster-unit"
      disabled={busy}
      onclick={() => onSelect(unit.id)}
    >
      <span class="unit-icon">&#10022;</span>
      <span>
        <strong>{unitDef?.name ?? unit.definitionId} #{unit.id}</strong><small>
          {snapshot?.units.find((u) => u.id === unit.id)?.behavior ?? 'Ready for deployment'}
        </small>
      </span>
      <span class="unit-hp">
        {snapshot?.units.find((u) => u.id === unit.id)?.hp ?? unitDef?.maxHp} HP
      </span>
    </button>
  {/each}
  {#if editable}
    <button
      class="add"
      data-testid={`add-soldier-${faction.id}`}
      disabled={busy || roster.length >= (catalog?.maxPerFaction ?? 8)}
      onclick={onAdd}
    >
      + <span>Add soldier</span>
    </button>
  {/if}
</section>
