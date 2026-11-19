<script lang="ts">
  /** Displays starting or live soldier inventory, equipped hands, load factors, and sustainment. */
  import type { Catalog, InventorySnapshot, UnitSnapshot } from '../lib/types';

  interface Props {
    catalog: Catalog;
    inventory: InventorySnapshot;
    live?: UnitSnapshot | null;
  }

  let { catalog, inventory, live = null }: Props = $props();

  function itemName(itemId: string): string {
    return catalog.items.find((item) => item.id === itemId)?.name ?? itemId;
  }
  function quantity(itemId: string): number {
    return inventory.entries
      .filter((entry) => entry.itemId === itemId)
      .reduce((sum, entry) => sum + entry.quantity, 0);
  }
  function handName(id: number | null): string {
    if (id === null) return 'Empty';
    const entry = inventory.entries.find((item) => item.id === id);
    return entry ? itemName(entry.itemId) : `Unknown item #${id}`;
  }
  const listedItems = $derived(Array.from(new Set(inventory.entries.map((entry) => entry.itemId))));
  const equippedAmmoId = $derived.by(() => {
    for (const handId of [inventory.leftHand, inventory.rightHand]) {
      if (handId === null) continue;
      const entry = inventory.entries.find((item) => item.id === handId);
      if (!entry) continue;
      const itemDef = catalog.items.find((item) => item.id === entry.itemId);
      if (!itemDef?.weaponId) continue;
      const weapon = catalog.weapons.find((w) => w.id === itemDef.weaponId);
      if (weapon) return weapon.ammoItemId;
    }
    return null;
  });
</script>

<section class="editor inventory" data-testid="inventory">
  <h3>{live ? 'Inventory' : 'Starting inventory'}</h3>
  <p>{live ? 'Automatic use; loadout fixed' : 'Starting loadout'}</p>
  <ul data-testid="inventory-items">
    {#each listedItems as itemId (itemId)}<li>
        {itemName(itemId)}: {quantity(itemId)}
      </li>{:else}<li>Empty</li>{/each}
  </ul>
  <p data-testid="inventory-hands">
    Left hand: {handName(inventory.leftHand)}<br />Right hand: {handName(inventory.rightHand)}
  </p>
  <p data-testid="inventory-load">
    Weight: {(inventory.weightGrams / 1000).toFixed(2)} / {(
      inventory.maxWeightGrams / 1000
    ).toFixed(2)} kg<br />Stowed volume: {(inventory.stowedVolumeMl / 1000).toFixed(2)} / {(
      inventory.maxStowedVolumeMl / 1000
    ).toFixed(2)} L<br />Load factor: {inventory.loadFactor.toFixed(2)}
  </p>
  {#if live}
    <p data-testid="inventory-needs">
      Nutrition: {live.sustainment.nutrition.toFixed(1)}%<br />Hydration: {live.sustainment.hydration.toFixed(
        1,
      )}%<br />Needs factor: {live.sustainment.needsFactor.toFixed(2)}<br />Combined mobility: {live.mobilityFactor.toFixed(
        2,
      )}
    </p>
    {#if equippedAmmoId !== null && quantity(equippedAmmoId) === 0}<p>Out of ammunition</p>{/if}
    {#if live.sustainment.nutritionShortage}<p>Nutrition shortage</p>{/if}
    {#if live.sustainment.hydrationShortage}<p>Hydration shortage</p>{/if}
    <p>Item uses</p>
    <ul data-testid="inventory-uses">
      {#each inventory.uses as use (use.itemId)}<li>
          {itemName(use.itemId)}: {use.count}
        </li>{:else}<li>None</li>{/each}
    </ul>
  {/if}
</section>
