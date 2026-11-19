<script lang="ts">
  /** Per-faction unit roster panel displaying live behavior/HP status and deployment button. */
  import { frameUnitSummary, type LiveFrame } from '../lib/compact-snapshot';
  import { behaviorLabel, factionColorHex, unitDefinition } from '../lib/factions';
  import type { Catalog, FactionDefinition, Scenario } from '../lib/types';

  interface Props {
    faction: FactionDefinition;
    side: number;
    scenario: Scenario | null;
    snapshot: LiveFrame | null;
    catalog: Catalog | null;
    selected: number | null;
    editable: boolean;
    busy: boolean;
    onSelect: (id: number) => void;
    onAdd: () => void;
  }

  let {
    faction,
    side,
    scenario,
    snapshot,
    catalog,
    selected,
    editable,
    busy,
    onSelect,
    onAdd,
  }: Props = $props();

  const roster = $derived(scenario?.units.filter((u) => u.faction === faction.id) ?? []);
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
    {@const live = frameUnitSummary(snapshot, unit.id)}
    <button
      class:active={selected === unit.id}
      class="roster-unit"
      disabled={busy}
      onclick={() => onSelect(unit.id)}
    >
      <span class="unit-icon">&#10022;</span>
      <span>
        <strong>{unitDef?.name ?? unit.definitionId} #{unit.id}</strong><small>
          {live ? behaviorLabel(live.behavior) : 'Ready for deployment'}
        </small>
      </span>
      <span class="unit-hp">
        {live?.hp ?? unitDef?.maxHp} HP
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
