<script lang="ts">
  /**
   * Skirmish workspace orchestrating Worker communication (`BattleClient`), PixiJS `Battlefield`,
   * unit deployment/doctrine setup, live battle controls, and unit/terrain inspection.
   */
  import { onMount, tick } from 'svelte';
  import { BattleClient } from '../lib/client';
  import { Battlefield } from '../lib/battlefield';
  import { CompactSnapshotView, isCompactFrame, type LiveFrame } from '../lib/compact-snapshot';
  import {
    factionColorHex,
    factionName,
    previewUnitInventory,
    unitDefinition,
  } from '../lib/factions';
  import { listScenarios, type SavedScenario } from '../lib/scenario-library';
  import DoctrineEditor from './DoctrineEditor.svelte';
  import FactionPanel from './FactionPanel.svelte';
  import InventoryDisplay from './InventoryDisplay.svelte';
  import Results from './Results.svelte';
  import UnitPicker from './UnitPicker.svelte';
  import LiveUnitStatus from './skirmish/LiveUnitStatus.svelte';
  import SkirmishActionBar from './skirmish/SkirmishActionBar.svelte';
  import SkirmishSetupBar from './skirmish/SkirmishSetupBar.svelte';
  import type {
    BattleResult,
    Catalog,
    Doctrine,
    InventorySnapshot,
    Scenario,
    ScenarioAnalysis,
    ScenarioDocument,
    UnitPlacement,
    WorkerReply,
  } from '../lib/types';

  interface Props {
    showHp?: boolean;
    initialDocument?: ScenarioDocument | null;
    onBack: () => void;
    returnToMaker?: boolean;
  }

  let { showHp = true, initialDocument = null, onBack, returnToMaker = false }: Props = $props();

  let catalog = $state.raw<Catalog | null>(null);
  let saved = $state.raw<SavedScenario[]>([]);
  let analysis = $state.raw<ScenarioAnalysis | null>(null);
  let description = $state('');
  let scenario = $state.raw<Scenario | null>(null);
  let starting = $state.raw<Scenario | null>(null);
  let snapshot = $state.raw<LiveFrame | null>(null);
  let inspectedInventory = $state.raw<{ unitId: number; inventory: InventorySnapshot } | null>(
    null,
  );
  let behindRealTime = $state(false);
  let result = $state.raw<BattleResult | null>(null);
  let phase = $state<'loading' | 'setup' | 'running' | 'paused' | 'finished' | 'error'>('loading');
  let error = $state('');
  let selected = $state<number | null>(null);
  let placement = $state<{ faction: number; definitionId: string } | null>(null);
  let modalFaction = $state<number | null>(null);
  let busy = $state(false);
  let deployX = $state(0);
  let deployY = $state(0);
  let mapId = $state('');
  let startingMapId = $state('');
  let inspectX = $state(0);
  let inspectY = $state(0);
  let terrainOverlay = $state(true);
  let lastNumericTimeLimit = 600;

  let host: HTMLElement | undefined;
  let client = $state.raw<BattleClient | null>(null);
  let battlefield = $state.raw<Battlefield | null>(null);
  let analyzedScenario: Scenario | null = null;
  let analysisRevision = 0;
  let generation = 0;

  const copy = <T,>(value: T): T => structuredClone(value);

  function formatClock(seconds: number): string {
    const whole = Math.max(0, Math.round(seconds));
    const mins = Math.floor(whole / 60);
    const secs = whole % 60;
    return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  }

  const selectedUnit = $derived(scenario?.units.find((unit) => unit.id === selected) ?? null);
  const inspected = $derived(
    scenario &&
      analysis &&
      Number.isInteger(inspectX) &&
      Number.isInteger(inspectY) &&
      inspectX >= 0 &&
      inspectY >= 0 &&
      inspectX < scenario.map.width &&
      inspectY < scenario.map.height
      ? analysis.cells[inspectY * scenario.map.width + inspectX]
      : null,
  );
  const selectedState = $derived.by(() => {
    if (!snapshot || selected === null) return null;
    if (!isCompactFrame(snapshot)) {
      return snapshot.units.find((unit) => unit.id === selected) ?? null;
    }
    const inv =
      inspectedInventory?.unitId === selected
        ? inspectedInventory.inventory
        : catalog && scenario && selectedUnit
          ? previewUnitInventory(catalog, scenario, selectedUnit)
          : null;
    return inv ? snapshot.unitSnapshot(selected, inv) : null;
  });

  $effect(() => {
    if (scenario && client && scenario !== analyzedScenario) {
      void analyzeScenario(scenario);
    }
  });

  $effect(() => {
    if (battlefield && scenario && catalog) {
      battlefield.update(
        scenario,
        snapshot,
        selected,
        showHp,
        phase === 'running',
        catalog,
        terrainOverlay,
      );
    }
  });

  $effect(() => {
    if (client && phase !== 'setup' && phase !== 'loading' && phase !== 'error') {
      client.inspect(selected);
    }
  });

  async function analyzeScenario(value: Scenario) {
    analyzedScenario = value;
    const revision = ++analysisRevision;
    analysis = null;
    try {
      const next = await client!.analyze(value, revision);
      if (revision === analysisRevision) analysis = next;
    } catch (e) {
      if (revision === analysisRevision) error = e instanceof Error ? e.message : String(e);
    }
  }

  function receive(message: WorkerReply) {
    if (message.type === 'ready') {
      catalog = message.catalog;
      mapId = initialDocument ? 'editor' : (catalog.maps[0]?.id ?? '');
      description = initialDocument?.description ?? catalog.maps[0]?.description ?? '';
      const current = generation;
      listScenarios(catalog)
        .then((items) => {
          if (current === generation) saved = items;
        })
        .catch((e) => {
          if (current === generation) error = e instanceof Error ? e.message : String(e);
        });
      scenario = initialDocument
        ? copy(initialDocument.scenario)
        : {
            map: copy(catalog.maps[0].map),
            seed: catalog.defaultSeed,
            factions: copy(catalog.defaultFactions),
            units: [],
            loadouts: [],
            timeLimit: catalog.defaultTimeLimit,
          };
      lastNumericTimeLimit = scenario.timeLimit ?? catalog.defaultTimeLimit;
      snapshot = null;
      inspectedInventory = null;
      behindRealTime = false;
      result = null;
      selected = null;
      placement = null;
      phase = 'setup';
      error = '';
    } else if (message.type === 'snapshot') {
      snapshot = message.snapshot;
      if (message.snapshot.result) {
        result = message.snapshot.result;
        behindRealTime = false;
        phase = 'finished';
      }
    } else if (message.type === 'compactSnapshot') {
      snapshot = new CompactSnapshotView(
        message.tick,
        message.elapsed,
        message.unitCount,
        message.buffer,
        message.result,
      );
      if (message.inspectedInventory) {
        inspectedInventory = {
          unitId: message.inspectedInventory.unitId,
          inventory: message.inspectedInventory.inventory,
        };
      }
      if (message.result) {
        result = message.result;
        behindRealTime = false;
        phase = 'finished';
      }
    } else if (message.type === 'unitInventory') {
      inspectedInventory = {
        unitId: message.update.unitId,
        inventory: message.update.inventory,
      };
    } else if (message.type === 'performance') {
      behindRealTime = message.behindRealTime;
    } else if (message.type === 'state') {
      if (message.state !== 'running') behindRealTime = false;
      if (message.state === 'running' && document.hidden) {
        client?.pause();
        phase = 'paused';
      } else if (message.state === 'running' || message.state === 'paused') phase = message.state;
    } else if (message.type === 'result') {
      result = message.result;
      behindRealTime = false;
      phase = 'finished';
    } else if (message.type === 'error') {
      error = message.message;
      behindRealTime = false;
      phase = 'error';
    }
  }

  async function openSkirmish() {
    closeBattle();
    phase = 'loading';
    error = '';
    const current = ++generation;
    await tick();
    if (current !== generation || !host) return;
    try {
      const nextField = new Battlefield(host, mapClick, (x, y) => {
        inspectX = x;
        inspectY = y;
      });
      await nextField.init();
      if (current !== generation) {
        nextField.destroy();
        return;
      }
      battlefield = nextField;
      const nextClient = new BattleClient(receive);
      client = nextClient;
      nextClient.initialize();
    } catch (e) {
      if (current === generation) {
        error = e instanceof Error ? e.message : String(e);
        phase = 'error';
      }
    }
  }

  function closeBattle() {
    generation++;
    analysisRevision++;
    analyzedScenario = null;
    analysis = null;
    client?.dispose();
    client = null;
    battlefield?.destroy();
    battlefield = null;
    catalog = null;
    scenario = null;
    snapshot = null;
    inspectedInventory = null;
    behindRealTime = false;
    result = null;
    starting = null;
    selected = null;
    placement = null;
    modalFaction = null;
    busy = false;
  }

  function back() {
    closeBattle();
    onBack();
  }

  async function mapClick(x: number, y: number) {
    if (!scenario || !catalog || busy) return;
    if (phase !== 'setup') {
      if (isCompactFrame(snapshot)) {
        selected = snapshot.findNearest(x, y, 0.65);
      } else {
        const nearest = snapshot?.units.find(
          (unit) => Math.hypot(unit.position.x - x, unit.position.y - y) < 0.65,
        );
        selected = nearest?.id ?? null;
      }
      return;
    }
    if (placement !== null) {
      const { faction, definitionId } = placement;
      const candidate = copy(scenario);
      const ids = new Set(candidate.units.map((unit) => unit.id));
      let id = 1;
      while (ids.has(id)) id++;
      candidate.units.push({
        id,
        faction,
        definitionId,
        position: { x: Math.floor(x) + 0.5, y: Math.floor(y) + 0.5 },
        doctrine: copy(catalog.defaultDoctrine),
      });
      const token = generation;
      busy = true;
      error = '';
      try {
        const validation = await client!.validate(candidate, false);
        if (token !== generation) return;
        if (validation.valid) {
          scenario = candidate;
          selected = candidate.units[candidate.units.length - 1].id;
          placement = null;
        } else error = validation.issues.map((issue) => issue.message).join(' ');
      } catch (e) {
        if (token === generation) error = e instanceof Error ? e.message : String(e);
      } finally {
        if (token === generation) busy = false;
      }
    } else {
      const nearest = scenario.units.find(
        (unit) => Math.hypot(unit.position.x - x, unit.position.y - y) < 0.65,
      );
      selected = nearest?.id ?? null;
    }
  }

  function editUnit(id: number, patch: Partial<UnitPlacement>) {
    if (!scenario || phase !== 'setup' || busy) return;
    scenario = {
      ...scenario,
      units: scenario.units.map((unit) => (unit.id === id ? { ...unit, ...patch } : unit)),
    };
  }

  function doctrine(field: keyof Doctrine, value: Doctrine[keyof Doctrine]) {
    if (selectedUnit)
      editUnit(selectedUnit.id, { doctrine: { ...selectedUnit.doctrine, [field]: value } });
  }

  function changeTimeLimitMinutes(input: HTMLInputElement) {
    if (!scenario || !catalog || phase !== 'setup' || busy) return;
    const minutes = Number(input.value);
    const seconds = Math.round(minutes * 60);
    if (input.value.trim() === '' || !Number.isFinite(minutes) || seconds < 1 || seconds > 3600) {
      error = 'Time limit must be between 1 second and 60 minutes.';
      input.value =
        scenario.timeLimit === null ? '' : String(Number((scenario.timeLimit / 60).toFixed(2)));
      return;
    }
    error = '';
    lastNumericTimeLimit = seconds;
    scenario = { ...scenario, timeLimit: seconds };
  }

  function toggleNoTimeLimit(unlimited: boolean) {
    if (!scenario || !catalog || phase !== 'setup' || busy) return;
    error = '';
    if (unlimited) {
      if (scenario.timeLimit !== null) lastNumericTimeLimit = scenario.timeLimit;
      scenario = { ...scenario, timeLimit: null };
    } else {
      scenario = { ...scenario, timeLimit: lastNumericTimeLimit || catalog.defaultTimeLimit };
    }
  }

  async function changeMap(id: string) {
    if (!catalog || !scenario || !client || phase !== 'setup' || busy) return;
    const preset = catalog.maps.find((map) => map.id === id);
    const entry = saved.find((item) => `saved:${item.id}` === id);
    if (preset) {
      mapId = id;
      description = preset.description;
      scenario = {
        map: copy(preset.map),
        seed: catalog.defaultSeed,
        factions: copy(catalog.defaultFactions),
        units: [],
        loadouts: [],
        timeLimit: catalog.defaultTimeLimit,
      };
    } else if (entry) {
      busy = true;
      const token = generation;
      try {
        const candidate = copy(entry.document.scenario);
        const validation = await client.analyze(candidate, ++analysisRevision);
        if (token !== generation) return;
        if (!validation.structuralValid) {
          error = validation.issues.map((issue) => issue.message).join(' ');
          return;
        }
        scenario = candidate;
        lastNumericTimeLimit = candidate.timeLimit ?? catalog.defaultTimeLimit;
        mapId = id;
        description = entry.document.description;
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
        return;
      } finally {
        if (token === generation) busy = false;
      }
    } else return;
    selected = null;
    placement = null;
    inspectX = 0;
    inspectY = 0;
    error = '';
  }

  function removeUnit() {
    if (!scenario || selected === null || busy) return;
    scenario = { ...scenario, units: scenario.units.filter((unit) => unit.id !== selected) };
    selected = null;
  }

  async function start() {
    if (!scenario || !client || busy) return;
    const candidate = copy(scenario);
    const token = generation;
    busy = true;
    error = '';
    try {
      const validation = await client.validate(candidate, true);
      if (token !== generation) return;
      if (!validation.valid) {
        error = validation.issues.map((issue) => issue.message).join(' ');
        return;
      }
      starting = copy(candidate);
      startingMapId = mapId;
      snapshot = null;
      inspectedInventory = null;
      behindRealTime = false;
      result = null;
      selected = null;
      placement = null;
      client.start(candidate);
      if (document.hidden) {
        client.pause();
        phase = 'paused';
      } else phase = 'running';
    } catch (e) {
      if (token === generation) error = e instanceof Error ? e.message : String(e);
    } finally {
      if (token === generation) busy = false;
    }
  }

  function reset() {
    if (!client || !starting) return;
    client.reset();
    scenario = copy(starting);
    mapId = startingMapId;
    snapshot = null;
    inspectedInventory = null;
    behindRealTime = false;
    result = null;
    selected = null;
    placement = null;
    error = '';
    phase = 'setup';
  }

  function pause() {
    if (phase === 'running') {
      client?.pause();
      phase = 'paused';
    }
  }

  function resume() {
    if (phase === 'paused') {
      client?.resume();
      phase = 'running';
    }
  }

  function endBattle() {
    if (phase === 'running' || phase === 'paused') {
      client?.stop();
    }
  }

  onMount(() => {
    const visibility = () => {
      if (document.hidden) pause();
    };
    const keydown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        modalFaction = null;
        placement = null;
      }
    };
    document.addEventListener('visibilitychange', visibility);
    window.addEventListener('keydown', keydown);
    void openSkirmish();
    return () => {
      document.removeEventListener('visibilitychange', visibility);
      window.removeEventListener('keydown', keydown);
      closeBattle();
    };
  });
</script>

<main class="workspace">
  <header class="topbar">
    <button class="back" onclick={back}>
      &larr; {returnToMaker ? 'Return to MapMaker' : 'Main menu'}
    </button>
    <div class="brand">2ND HAND <span>COMBAT</span></div>
    <div class="top-status" data-testid="battle-status">
      <i></i>{phase === 'setup' ? 'SETUP' : phase.toUpperCase()}
    </div>
  </header>
  {#if phase === 'loading' || phase === 'error'}<div class="loading">
      <p class="eyebrow">FIELD OPERATIONS</p>
      <h2>{phase === 'loading' ? 'Preparing the field…' : 'Unable to prepare the field'}</h2>
      {#if error}<p role="alert">{error}</p>
        <button onclick={openSkirmish}>Retry</button>{/if}<button
        class="text-button"
        onclick={back}
      >
        Back to menu
      </button>
    </div>{/if}
  <div class:hidden={phase === 'loading' || phase === 'error'} class="battle-layout">
    <section class="field-column">
      <div class="field-heading">
        <div>
          <p class="eyebrow">OPERATION / SKIRMISH</p>
          <h1>Battlefield<span class="period">.</span></h1>
        </div>
        <div class="field-meta">
          {scenario?.map.width} × {scenario?.map.height} <span> / </span> SEED {scenario?.seed}
        </div>
      </div>
      <div class="map-frame">
        <div class="map-top">
          <span>TACTICAL MAP</span>{#if behindRealTime && phase === 'running'}<span
              class="lag-badge"
              role="status"
              data-testid="behind-realtime"
            >
              Simulation behind real time
            </span>{/if}<span data-testid="battle-clock">
            {phase === 'setup' ? 'DEPLOYMENT' : 'LIVE VIEW'} / {snapshot
              ? `${snapshot.elapsed.toFixed(1)}s`
              : '00:00'} / {scenario?.timeLimit === null
              ? 'NO LIMIT'
              : formatClock(scenario?.timeLimit ?? catalog?.defaultTimeLimit ?? 600)}
          </span>
        </div>
        <div
          class="battlefield"
          style={`aspect-ratio: ${scenario?.map.width ?? 32} / ${scenario?.map.height ?? 20}; max-height: 70vh`}
          data-testid="battlefield"
          data-map-width={scenario?.map.width ?? 32}
          data-map-height={scenario?.map.height ?? 20}
          bind:this={host}
        ></div>
        <div class="map-bottom">
          {#each scenario?.factions ?? [] as f (f.id)}<span>
              <i class="dot" style={`background:${factionColorHex(f.color)}`}
              ></i>{f.name.toUpperCase()}
            </span>{/each}<span><i class="square cover"></i>LOW OBSTACLE</span><span>
            <i class="square wall"></i>FULL OBSTACLE
          </span><span class="map-hint">
            {placement !== null && catalog
              ? `CLICK THE MAP TO PLACE ${factionName(scenario?.factions ?? [], placement.faction).toUpperCase()} ${unitDefinition(catalog, placement.definitionId).name.toUpperCase()} · ESC TO CANCEL`
              : phase === 'setup'
                ? 'SELECT A SOLDIER TO EDIT DOCTRINE'
                : 'AUTONOMOUS COMBAT'}
          </span>
        </div>
      </div>
      {#if catalog && scenario}
        <SkirmishSetupBar
          {catalog}
          {scenario}
          {saved}
          {mapId}
          {description}
          {phase}
          {busy}
          {terrainOverlay}
          {inspectX}
          {inspectY}
          {inspected}
          placementActive={placement !== null}
          {deployX}
          {deployY}
          onFit={() => battlefield?.fit()}
          onZoom={(factor) => battlefield?.zoomBy(factor)}
          onChangeMap={changeMap}
          onTimeLimitInput={changeTimeLimitMinutes}
          onToggleNoTimeLimit={toggleNoTimeLimit}
          onOverlayChange={(enabled) => (terrainOverlay = enabled)}
          onInspectChange={(x, y) => {
            inspectX = x;
            inspectY = y;
          }}
          onDeployChange={(x, y) => {
            deployX = x;
            deployY = y;
          }}
          onPlaceAt={(x, y) => void mapClick(x, y)}
        />
      {/if}
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <SkirmishActionBar
        {phase}
        {busy}
        canStart={!!analysis?.canStart}
        hasStarting={!!starting}
        onStart={start}
        onPause={pause}
        onResume={resume}
        onEndBattle={endBattle}
        onReset={reset}
      />
      {#if result}<Results {result} factions={scenario?.factions ?? []} />{/if}
    </section>
    <aside class="sidebar">
      <div class="sidebar-head">
        <p class="eyebrow">FORCE ROSTER</p>
        <h2>Deploy units</h2>
        <p>Build each side, then adjust individual doctrine.</p>
      </div>
      {#each scenario?.factions ?? [] as faction, side (faction.id)}
        <FactionPanel
          {faction}
          {side}
          {scenario}
          {snapshot}
          {catalog}
          {selected}
          editable={phase === 'setup'}
          {busy}
          onSelect={(id) => {
            selected = id;
            placement = null;
          }}
          onAdd={() => {
            modalFaction = faction.id;
            placement = null;
          }}
        />
      {/each}
      {#if catalog && scenario && selectedUnit && phase === 'setup'}
        <DoctrineEditor
          unit={selectedUnit}
          {catalog}
          {busy}
          onClose={() => (selected = null)}
          onChange={doctrine}
          onRemove={removeUnit}
        />
        <InventoryDisplay
          {catalog}
          inventory={previewUnitInventory(catalog, scenario, selectedUnit)}
        />
      {:else if catalog && selectedUnit && selectedState}
        <LiveUnitStatus {catalog} unit={selectedUnit} state={selectedState} />
      {/if}
    </aside>
  </div>
</main>
{#if modalFaction !== null && catalog}
  <UnitPicker
    name={factionName(scenario?.factions ?? [], modalFaction)}
    units={catalog.units}
    onClose={() => (modalFaction = null)}
    onChoose={(definitionId) => {
      placement = { faction: modalFaction!, definitionId };
      modalFaction = null;
      selected = null;
      error = '';
    }}
  />
{/if}
