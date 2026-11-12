<script lang="ts">
  import type { Catalog, Scenario, Snapshot } from '../lib/types';
  export let faction: number;
  export let side: number;
  export let name: string;
  export let scenario: Scenario | null;
  export let snapshot: Snapshot | null;
  export let catalog: Catalog | null;
  export let selected: number | null;
  export let editable: boolean;
  export let busy: boolean;
  export let onSelect: (id: number) => void;
  export let onAdd: () => void;
</script>

<section class="faction" data-testid={`faction-${faction}`}>
  <div class="faction-title">
    <div><i class:north={side === 0} class:south={side === 1} class="dot"></i><strong>{name}</strong><small>FACTION 0{side + 1}</small></div>
    <span>{scenario?.units.filter(u => u.faction === faction).length ?? 0} / {catalog?.maxPerFaction ?? 8}</span>
  </div>
  {#each scenario?.units.filter(u => u.faction === faction) ?? [] as unit}
    <button class:active={selected === unit.id} class="roster-unit" disabled={busy} onclick={() => onSelect(unit.id)}>
      <span class="unit-icon">&#10022;</span>
      <span><strong>Infantry #{unit.id}</strong><small>{snapshot?.units.find(u => u.id === unit.id)?.behavior ?? 'Ready for deployment'}</small></span>
      <span class="unit-hp">{snapshot?.units.find(u => u.id === unit.id)?.hp ?? catalog?.unit.maxHp} HP</span>
    </button>
  {/each}
  {#if editable}
    <button class="add" data-testid={`add-soldier-${faction}`} disabled={busy || (scenario?.units.filter(u => u.faction === faction).length ?? 0) >= (catalog?.maxPerFaction ?? 8)} onclick={onAdd}>+ <span>Add soldier</span></button>
  {/if}
</section>
