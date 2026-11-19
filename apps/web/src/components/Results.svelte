<script lang="ts">
  import { factionColorHex, factionById, factionName } from '../lib/factions';
  import type { BattleResult, FactionDefinition } from '../lib/types';
  export let result: BattleResult;
  export let factions: FactionDefinition[];
</script>

<section class="result">
  <p class="eyebrow">AFTER ACTION REPORT</p>
  <h2>
    {result.winner === null ? 'Draw' : `${factionName(factions, result.winner)} wins`}
  </h2>
  <p>
    {result.reason === 'timeout'
      ? 'Time limit reached'
      : result.reason === 'simultaneous_elimination'
        ? 'All sides eliminated'
        : 'Opposing forces eliminated'} &middot; {result.elapsed.toFixed(1)} seconds
  </p>
  <div class="result-grid">
    {#each result.factions as entry (entry.faction)}
      {@const def = factionById(factions, entry.faction)}
      <div>
        <strong>
          {#if def}<i class="dot" style={`background:${factionColorHex(def.color)}`}></i>{/if}
          {factionName(factions, entry.faction)}
        </strong><span>{entry.survivors} survivors / {entry.casualties} casualties</span>
      </div>
    {/each}
  </div>
</section>
