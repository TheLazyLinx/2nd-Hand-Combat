<script lang="ts">
  /** After-action battle results summary displaying winner/draw, reason, and per-faction counts. */
  import { factionColorHex, factionById, factionName } from '../lib/factions';
  import type { BattleResult, FactionDefinition } from '../lib/types';

  interface Props {
    result: BattleResult;
    factions: FactionDefinition[];
  }

  let { result, factions }: Props = $props();
</script>

<section class="result">
  <p class="eyebrow">AFTER ACTION REPORT</p>
  <h2>
    {result.winner === null ? 'Draw' : `${factionName(factions, result.winner)} wins`}
  </h2>
  <p>
    {result.reason === 'timeout'
      ? 'Time limit reached'
      : result.reason === 'stopped'
        ? 'Battle ended early'
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
