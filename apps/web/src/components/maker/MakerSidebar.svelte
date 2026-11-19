<script lang="ts">
  /** MapMaker sidebar: scenario metadata, factions, map resize, entity lists, and saved library. */
  import { factionColorHex, factionName } from '../../lib/factions';
  import type { SavedScenario } from '../../lib/scenario-library';
  import type { Catalog, PatrolDirection, Scenario, ScenarioDocument } from '../../lib/types';

  interface Props {
    catalog: Catalog;
    doc: ScenarioDocument;
    scenario: Scenario;
    saved: SavedScenario[];
    width: number;
    height: number;
    newWidth: number;
    newHeight: number;
    resizePreview: boolean;
    onNameChange: (value: string) => void;
    onDescriptionChange: (value: string) => void;
    onNumberChange: (field: 'seed' | 'timeLimit', input: HTMLInputElement) => void;
    onToggleNoTimeLimit: (unlimited: boolean) => void;
    onRecolorFaction: (id: number, hex: string) => void;
    onRenameFaction: (id: number, input: HTMLInputElement) => void;
    onPatrolDirection: (id: number, direction: PatrolDirection) => void;
    onRemoveFaction: (id: number) => void;
    onAddFaction: () => void;
    onApplyResize: () => void;
    onSelectUnit: (unitId: number) => void;
    onSelectPatrol: (index: number) => void;
    onNewBlank: () => void;
    onCopyBuiltIn: (mapId: string) => void;
    onImport: (event: Event) => void;
    onOpenSaved: (item: SavedScenario) => void;
    onRequestRename: (item: SavedScenario) => void;
    onDuplicate: (item: SavedScenario) => void;
    onRequestDelete: (id: string) => void;
  }

  let {
    catalog,
    doc,
    scenario,
    saved,
    width = $bindable(),
    height = $bindable(),
    newWidth = $bindable(),
    newHeight = $bindable(),
    resizePreview = $bindable(),
    onNameChange,
    onDescriptionChange,
    onNumberChange,
    onToggleNoTimeLimit,
    onRecolorFaction,
    onRenameFaction,
    onPatrolDirection,
    onRemoveFaction,
    onAddFaction,
    onApplyResize,
    onSelectUnit,
    onSelectPatrol,
    onNewBlank,
    onCopyBuiltIn,
    onImport,
    onOpenSaved,
    onRequestRename,
    onDuplicate,
    onRequestDelete,
  }: Props = $props();

  let importInput: HTMLInputElement;
</script>

<aside>
  <section>
    <h2>Scenario</h2>
    <label>
      Name <input
        maxlength="100"
        value={doc.name}
        onchange={(e) => onNameChange(e.currentTarget.value)}
      />
    </label><label>
      Description <textarea
        maxlength="2000"
        value={doc.description}
        onchange={(e) => onDescriptionChange(e.currentTarget.value)}></textarea>
    </label><label>
      Seed <input
        type="number"
        min="0"
        max="4294967295"
        value={scenario.seed}
        onchange={(e) => onNumberChange('seed', e.currentTarget)}
      />
    </label><label>
      Time limit <input
        type="number"
        min="1"
        disabled={scenario.timeLimit === null}
        value={scenario.timeLimit ?? ''}
        onchange={(e) => onNumberChange('timeLimit', e.currentTarget)}
      />
    </label><label class="checkbox-row">
      <input
        data-testid="maker-no-time-limit"
        type="checkbox"
        checked={scenario.timeLimit === null}
        onchange={(e) => onToggleNoTimeLimit(e.currentTarget.checked)}
      /> No time limit
    </label>
  </section>
  <section aria-label="Factions">
    <h2>Factions</h2>
    {#each scenario.factions as side (side.id)}<div
        class="faction-row"
        data-testid={`editor-faction-${side.id}`}
      >
        <input
          aria-label={`Faction ${side.name} color`}
          type="color"
          value={factionColorHex(side.color)}
          onchange={(e) => onRecolorFaction(side.id, e.currentTarget.value)}
        /><input
          aria-label={`Faction ${side.id} name`}
          maxlength="64"
          value={side.name}
          onchange={(e) => onRenameFaction(side.id, e.currentTarget)}
        /><select
          aria-label={`Faction ${side.name} patrol direction`}
          value={side.patrolDirection}
          onchange={(e) => onPatrolDirection(side.id, e.currentTarget.value as PatrolDirection)}
        >
          <option value="forward">Forward</option><option value="reverse">Reverse</option>
        </select><button
          disabled={scenario.factions.length <= 2}
          onclick={() => onRemoveFaction(side.id)}
        >
          Remove
        </button>
      </div>{/each}
    <button onclick={onAddFaction}>Add faction</button>
  </section>
  <section>
    <h2>Map size</h2>
    <label>
      Width <input
        type="number"
        min={catalog.editorRules.minDimension}
        max={catalog.editorRules.maxDimension}
        bind:value={width}
      />
    </label><label>
      Height <input
        type="number"
        min={catalog.editorRules.minDimension}
        max={catalog.editorRules.maxDimension}
        bind:value={height}
      />
    </label><button onclick={() => (resizePreview = true)}>
      Preview resize
    </button>{#if resizePreview}<p>
        Northwest anchor. {scenario.units.filter(
          (u) => u.position.x >= width || u.position.y >= height,
        ).length} soldiers and {scenario.map.patrol.filter((p) => p.x >= width || p.y >= height)
          .length} patrol points removed; {scenario.map.obstacles.filter(
          (o) => o.x + o.width > width || o.y + o.height > height,
        ).length} obstacles clipped or removed.
      </p>
      <button onclick={onApplyResize}>Apply resize</button><button
        onclick={() => (resizePreview = false)}
      >
        Cancel resize
      </button>{/if}
  </section>
  <section>
    <h2>Soldiers</h2>
    {#each scenario.units as unit (unit.id)}<button onclick={() => onSelectUnit(unit.id)}>
        #{unit.id}
        {factionName(scenario.factions, unit.faction)}
      </button>{/each}
    <h2>Patrol</h2>
    {#each scenario.map.patrol as point, index (index)}<button
        onclick={() => onSelectPatrol(index)}
      >
        Point {index + 1}: {point.x}, {point.y}
      </button>{/each}
  </section>
  <section>
    <h2>Library</h2>
    <label>
      New map width <input
        type="number"
        min={catalog.editorRules.minDimension}
        max={catalog.editorRules.maxDimension}
        bind:value={newWidth}
      />
    </label><label>
      New map height <input
        type="number"
        min={catalog.editorRules.minDimension}
        max={catalog.editorRules.maxDimension}
        bind:value={newHeight}
      />
    </label><button
      disabled={!Number.isInteger(newWidth) ||
        !Number.isInteger(newHeight) ||
        newWidth < catalog.editorRules.minDimension ||
        newHeight < catalog.editorRules.minDimension ||
        newWidth > catalog.editorRules.maxDimension ||
        newHeight > catalog.editorRules.maxDimension}
      onclick={onNewBlank}
    >
      New blank
    </button><label>
      Copy built-in <select
        onchange={(e) => {
          onCopyBuiltIn(e.currentTarget.value);
          e.currentTarget.value = '';
        }}
      >
        <option value="">Choose map</option>{#each catalog.maps as preset (preset.id)}<option
            value={preset.id}
          >
            {preset.name}
          </option>{/each}
      </select>
    </label><button onclick={() => importInput.click()}>Import</button><input
      bind:this={importInput}
      type="file"
      accept=".json,application/json"
      hidden
      onchange={onImport}
    />{#each saved as item (item.id)}<div class="library-item">
        <strong>{item.document.name}</strong><button onclick={() => onOpenSaved(item)}>
          Open
        </button><button onclick={() => onRequestRename(item)}>Rename</button><button
          onclick={() => onDuplicate(item)}
        >
          Duplicate
        </button><button onclick={() => onRequestDelete(item.id)}>Delete</button>
      </div>{/each}
  </section>
</aside>
