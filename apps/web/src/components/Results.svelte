<script lang="ts">
  import type { BattleResult } from '../lib/types';
  export let result: BattleResult;
  export let names: string[];
  export let factions: number[];
</script>

<section class="result">
  <p class="eyebrow">AFTER ACTION REPORT</p>
  <h2>{result.winner === null ? 'Draw' : `${names[factions.indexOf(result.winner)] ?? `Faction ${result.winner}`} wins`}</h2>
  <p>{result.reason === 'timeout' ? 'Time limit reached' : result.reason === 'simultaneous_elimination' ? 'Both sides eliminated' : 'Opposing force eliminated'} &middot; {result.elapsed.toFixed(1)} seconds</p>
  <div class="result-grid">
    {#each result.factions as faction}
      <div><strong>{names[factions.indexOf(faction.faction)] ?? `Faction ${faction.faction}`}</strong><span>{faction.survivors} survivors / {faction.casualties} casualties</span></div>
    {/each}
  </div>
</section>
