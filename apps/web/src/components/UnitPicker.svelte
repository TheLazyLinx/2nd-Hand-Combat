<script lang="ts">
  import { onMount } from 'svelte';
  import type { UnitDefinition } from '../lib/types';
  export let name: string;
  export let units: UnitDefinition[];
  export let onClose: () => void;
  export let onChoose: (definitionId: string) => void;
  let dialog: HTMLDialogElement;
  onMount(() => dialog.showModal());
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="modal-title"
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
  onclick={(event) => {
    if (event.target === dialog) onClose();
  }}
>
  <div class="modal">
    <div class="editor-title">
      <p class="eyebrow">{name.toUpperCase()} / DEPLOYMENT</p>
      <button class="text-button" onclick={onClose} aria-label="Close">&times;</button>
    </div>
    <h2 id="modal-title">Choose a soldier</h2>
    <p>Select a unit, then click an open spot on the battlefield.</p>
    {#each units as unit (unit.id)}
      <button
        class="unit-choice"
        data-testid={`choose-${unit.id}`}
        onclick={() => onChoose(unit.id)}
      >
        <span class="unit-icon">&#10022;</span><span>
          <strong>{unit.name}</strong><small>
            {unit.maxHp} HP &middot; speed {unit.speed} m/s
          </small>
        </span><b>&rarr;</b>
      </button>
    {/each}
  </div>
</dialog>

<style>
  dialog {
    border: 0;
    padding: 0;
    width: min(calc(100% - 40px), 420px);
    background: transparent;
  }
  dialog::backdrop {
    background: #202b22b8;
  }
</style>
