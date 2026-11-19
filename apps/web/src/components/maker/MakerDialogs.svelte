<script lang="ts">
  /** Confirmation and rename modal dialogs for MapMaker library and unsaved-changes guards. */

  interface Props {
    showRename: boolean;
    renameValue: string;
    showDelete: boolean;
    showUnsavedGuard: boolean;
    onConfirmRename: () => void;
    onCancelRename: () => void;
    onConfirmDelete: () => void;
    onCancelDelete: () => void;
    onSaveAndContinue: () => void;
    onDiscardAndContinue: () => void;
    onCancelGuard: () => void;
  }

  let {
    showRename,
    renameValue = $bindable(),
    showDelete,
    showUnsavedGuard,
    onConfirmRename,
    onCancelRename,
    onConfirmDelete,
    onCancelDelete,
    onSaveAndContinue,
    onDiscardAndContinue,
    onCancelGuard,
  }: Props = $props();
</script>

{#if showRename}
  <div class="modal" role="dialog" aria-modal="true" aria-label="Rename scenario">
    <div>
      <h2>Rename scenario</h2>
      <label>Name <input bind:value={renameValue} maxlength="100" /></label><button
        onclick={onConfirmRename}
      >
        Rename
      </button><button onclick={onCancelRename}>Cancel</button>
    </div>
  </div>
{/if}
{#if showDelete}
  <div class="modal" role="dialog" aria-modal="true" aria-label="Delete scenario">
    <div>
      <h2>Delete scenario?</h2>
      <p>This removes the saved copy from the library.</p>
      <button onclick={onConfirmDelete}>Delete</button><button onclick={onCancelDelete}>
        Cancel
      </button>
    </div>
  </div>
{/if}
{#if showUnsavedGuard}
  <div class="modal" role="dialog" aria-modal="true" aria-label="Unsaved changes">
    <div>
      <h2>Unsaved changes</h2>
      <p>Save this scenario before leaving?</p>
      <button onclick={onSaveAndContinue}>Save</button><button onclick={onDiscardAndContinue}>
        Discard
      </button><button onclick={onCancelGuard}>Cancel</button>
    </div>
  </div>
{/if}
