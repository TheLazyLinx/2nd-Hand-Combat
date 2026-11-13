<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { BattleClient } from '../lib/client';
  import { Battlefield } from '../lib/battlefield';
  import { listScenarios, type SavedScenario } from '../lib/scenario-library';
  import FactionPanel from './FactionPanel.svelte';
  import DoctrineEditor from './DoctrineEditor.svelte';
  import InventoryDisplay from './InventoryDisplay.svelte';
  import UnitPicker from './UnitPicker.svelte';
  import Results from './Results.svelte';
  import type { BattleResult, Catalog, Doctrine, Scenario, Snapshot, UnitPlacement, WorkerReply } from '../lib/types';

  export let showHp = true;
  export let initialDocument: import('../lib/types').ScenarioDocument | null = null;
  export let onBack: () => void;
  export let returnToMaker = false;
  let catalog: Catalog | null = null;
  let saved: SavedScenario[] = [];
  let analysis: import('../lib/types').ScenarioAnalysis | null = null;
  let analyzedScenario: Scenario | null = null;
  let analysisRevision = 0;
  let description = '';
  let scenario: Scenario | null = null;
  let starting: Scenario | null = null;
  let snapshot: Snapshot | null = null;
  let result: BattleResult | null = null;
  let phase: 'loading' | 'setup' | 'running' | 'paused' | 'finished' | 'error' = 'loading';
  let error = '';
  let selected: number | null = null;
  let placement: number | null = null;
  let modalFaction: number | null = null;
  let busy = false;
  let deployX = 0;
  let deployY = 0;
  let mapId = '';
  let startingMapId = '';
  let inspectX = 0;
  let inspectY = 0;
  let terrainOverlay = true;

  let host: HTMLElement;
  let client: BattleClient | null = null;
  let battlefield: Battlefield | null = null;
  let generation = 0;
  const names = ['North', 'South'];
  const copy = <T,>(value: T): T => structuredClone(value);
  $: selectedUnit = scenario?.units.find(unit => unit.id === selected) ?? null;
  $: inspected = scenario && analysis && Number.isInteger(inspectX) && Number.isInteger(inspectY) && inspectX >= 0 && inspectY >= 0 && inspectX < scenario.map.width && inspectY < scenario.map.height ? analysis.cells[inspectY * scenario.map.width + inspectX] : null;
  $: if (scenario && client && scenario !== analyzedScenario) { analyzedScenario = scenario; analyzeScenario(scenario); }
  $: selectedState = snapshot?.units.find(unit => unit.id === selected) ?? null;
  $: if (battlefield && scenario && catalog) battlefield.update(scenario, snapshot, selected, showHp, catalog.unit.maxHp, phase === 'running', catalog, terrainOverlay);

  async function analyzeScenario(value: Scenario) {
    const revision = ++analysisRevision;
    analysis = null;
    try { const next = await client!.analyze(value, revision); if (revision === analysisRevision) analysis = next; }
    catch (e) { if (revision === analysisRevision) error = e instanceof Error ? e.message : String(e); }
  }
  function receive(message: WorkerReply) {
    if (message.type === 'ready') {
      catalog = message.catalog;
      mapId = initialDocument ? 'editor' : catalog.maps[0]?.id ?? '';
      description = initialDocument?.description ?? catalog.maps[0]?.description ?? '';
      const current = generation;
      listScenarios(catalog.defaultDoctrine).then(items => { if (current === generation) saved = items; }).catch(e => { if (current === generation) error = e instanceof Error ? e.message : String(e); });
      scenario = initialDocument ? copy(initialDocument.scenario) : { map: copy(catalog.map), seed: catalog.defaultSeed, factions: [0, 1], units: [], timeLimit: catalog.defaultTimeLimit };
      snapshot = null; result = null; selected = null; placement = null; phase = 'setup'; error = '';
    } else if (message.type === 'snapshot') {
      snapshot = message.snapshot;
      if (message.snapshot.result) { result = message.snapshot.result; phase = 'finished'; }
    } else if (message.type === 'state') {
      if (message.state === 'running' && document.hidden) { client?.pause(); phase = 'paused'; }
      else if (message.state === 'running' || message.state === 'paused') phase = message.state;
    } else if (message.type === 'result') { result = message.result; phase = 'finished'; }
    else if (message.type === 'error') { error = message.message; phase = 'error'; }
  }
  async function openSkirmish() {
    closeBattle(); phase = 'loading'; error = '';
    const current = ++generation;
    await tick();
    if (current !== generation || !host) return;
    try {
      battlefield = new Battlefield(host, mapClick, (x, y) => { inspectX = x; inspectY = y; });
      await battlefield.init();
      if (current !== generation) return;
      client = new BattleClient(receive);
      client.initialize();
    } catch (e) { if (current === generation) { error = e instanceof Error ? e.message : String(e); phase = 'error'; } }
  }
  function closeBattle() {
    generation++; analysisRevision++; analyzedScenario = null; analysis = null; client?.dispose(); client = null; battlefield?.destroy(); battlefield = null;
    catalog = null; scenario = null; snapshot = null; result = null; starting = null; selected = null; placement = null; modalFaction = null; busy = false;
  }
  function back() { closeBattle(); onBack(); }
  async function mapClick(x: number, y: number) {
    if (!scenario || !catalog || busy) return;
    if (phase !== 'setup') {
      const nearest = snapshot?.units.find(unit => Math.hypot(unit.position.x - x, unit.position.y - y) < 0.65);
      selected = nearest?.id ?? null; return;
    }
    if (placement !== null) {
      const faction = placement;
      const candidate = copy(scenario);
      const ids = new Set(candidate.units.map(unit => unit.id)); let id = 1; while (ids.has(id)) id++;
      candidate.units.push({ id, faction, definitionId: catalog.unit.id, position: {x: Math.floor(x) + 0.5, y: Math.floor(y) + 0.5}, doctrine: copy(catalog.defaultDoctrine) });
      const token = generation; busy = true; error = '';
      try {
        const validation = await client!.validate(candidate, false);
        if (token !== generation) return;
        if (validation.valid) { scenario = candidate; selected = candidate.units[candidate.units.length - 1].id; placement = null; }
        else error = validation.issues.map(issue => issue.message).join(' ');
      } catch (e) { if (token === generation) error = e instanceof Error ? e.message : String(e); }
      finally { if (token === generation) busy = false; }
    } else {
      const nearest = scenario.units.find(unit => Math.hypot(unit.position.x - x, unit.position.y - y) < 0.65);
      selected = nearest?.id ?? null;
    }
  }
  function editUnit(id: number, patch: Partial<UnitPlacement>) {
    if (!scenario || phase !== 'setup' || busy) return;
    scenario = {...scenario, units: scenario.units.map(unit => unit.id === id ? {...unit, ...patch} : unit)};
  }
  function doctrine(field: keyof Doctrine, value: Doctrine[keyof Doctrine]) {
    if (selectedUnit) editUnit(selectedUnit.id, {doctrine: {...selectedUnit.doctrine, [field]: value}});
  }
  async function changeMap(id: string) {
    if (!catalog || !scenario || !client || phase !== 'setup' || busy) return;
    const preset = catalog.maps.find(map => map.id === id);
    const entry = saved.find(item => `saved:${item.id}` === id);
    if (preset) {
      mapId = id; description = preset.description;
      scenario = {map: copy(preset.map), seed: catalog.defaultSeed, factions: [0, 1], units: [], timeLimit: catalog.defaultTimeLimit};
    } else if (entry) {
      busy = true;
      const token = generation;
      try {
        const candidate = copy(entry.document.scenario);
        const validation = await client.analyze(candidate, ++analysisRevision);
        if (token !== generation) return;
        if (!validation.structuralValid) { error = validation.issues.map(issue => issue.message).join(' '); return; }
        scenario = candidate; mapId = id; description = entry.document.description;
      } catch (e) { error = e instanceof Error ? e.message : String(e); return; }
      finally { if (token === generation) busy = false; }
    } else return;
    selected = null; placement = null; inspectX = 0; inspectY = 0; error = '';
  }
  function removeUnit() {
    if (!scenario || selected === null || busy) return;
    scenario = {...scenario, units: scenario.units.filter(unit => unit.id !== selected)}; selected = null;
  }
  async function start() {
    if (!scenario || !client || busy) return;
    const candidate = copy(scenario), token = generation; busy = true; error = '';
    try {
      const validation = await client.validate(candidate, true);
      if (token !== generation) return;
      if (!validation.valid) { error = validation.issues.map(issue => issue.message).join(' '); return; }
      starting = copy(candidate); startingMapId = mapId; snapshot = null; result = null; selected = null; placement = null;
      client.start(candidate);
      if (document.hidden) { client.pause(); phase = 'paused'; } else phase = 'running';
    } catch (e) { if (token === generation) error = e instanceof Error ? e.message : String(e); }
    finally { if (token === generation) busy = false; }
  }
  function reset() {
    if (!client || !starting) return;
    client.reset(); scenario = copy(starting); mapId = startingMapId; snapshot = null; result = null; selected = null; placement = null; error = ''; phase = 'setup';
  }
  function pause() { if (phase === 'running') { client?.pause(); phase = 'paused'; } }
  function resume() { if (phase === 'paused') { client?.resume(); phase = 'running'; } }
  onMount(() => {
    const visibility = () => { if (document.hidden) pause(); };
    const keydown = (event: KeyboardEvent) => { if (event.key === 'Escape') { modalFaction = null; placement = null; } };
    document.addEventListener('visibilitychange', visibility); window.addEventListener('keydown', keydown);
    openSkirmish();
    return () => { document.removeEventListener('visibilitychange', visibility); window.removeEventListener('keydown', keydown); closeBattle(); };
  });
</script>

<main class="workspace"><header class="topbar"><button class="back" onclick={back}>&larr; {returnToMaker ? 'Return to MapMaker' : 'Main menu'}</button><div class="brand">2ND HAND <span>COMBAT</span></div><div class="top-status" data-testid="battle-status"><i></i>{phase === 'setup' ? 'SETUP' : phase.toUpperCase()}</div></header>
    {#if phase === 'loading' || phase === 'error'}<div class="loading"><p class="eyebrow">FIELD OPERATIONS</p><h2>{phase === 'loading' ? 'Preparing the field…' : 'Unable to prepare the field'}</h2>{#if error}<p role="alert">{error}</p><button onclick={openSkirmish}>Retry</button>{/if}<button class="text-button" onclick={back}>Back to menu</button></div>{/if}
    <div class:hidden={phase === 'loading' || phase === 'error'} class="battle-layout">
      <section class="field-column"><div class="field-heading"><div><p class="eyebrow">OPERATION / SKIRMISH</p><h1>Battlefield<span class="period">.</span></h1></div><div class="field-meta">{scenario?.map.width} × {scenario?.map.height} <span> / </span> SEED {scenario?.seed}</div></div>
        <div class="map-frame"><div class="map-top"><span>TACTICAL MAP</span><span>{phase === 'setup' ? 'DEPLOYMENT' : 'LIVE VIEW'} / {snapshot ? `${snapshot.elapsed.toFixed(1)}s` : '00:00'}</span></div><div class="battlefield" style={`aspect-ratio: ${scenario?.map.width ?? 32} / ${scenario?.map.height ?? 20}; max-height: 70vh`} data-testid="battlefield" data-map-width={scenario?.map.width ?? 32} data-map-height={scenario?.map.height ?? 20} bind:this={host}></div><div class="map-bottom"><span><i class="dot north"></i>NORTH</span><span><i class="dot south"></i>SOUTH</span><span><i class="square cover"></i>LOW OBSTACLE</span><span><i class="square wall"></i>FULL OBSTACLE</span><span class="map-hint">{placement !== null ? `CLICK THE MAP TO PLACE ${names[scenario?.factions.indexOf(placement) ?? 0]?.toUpperCase() ?? 'FACTION'} INFANTRY · ESC TO CANCEL` : phase === 'setup' ? 'SELECT A SOLDIER TO EDIT DOCTRINE' : 'AUTONOMOUS COMBAT'}</span></div></div>
        {#if catalog && scenario}
          <div class="terrain-controls"><button onclick={() => battlefield?.fit()}>Fit map</button><button onclick={() => battlefield?.zoomBy(1.25)}>Zoom in</button><button onclick={() => battlefield?.zoomBy(0.8)}>Zoom out</button>
            <label>Map <select data-testid="map-preset" value={mapId} disabled={phase !== 'setup' || busy} onchange={(e) => changeMap(e.currentTarget.value)}>{#if mapId === 'editor'}<option value="editor">MapMaker test</option>{/if}<optgroup label="Built-in maps">{#each catalog.maps as map}<option value={map.id}>{map.name}</option>{/each}</optgroup><optgroup label="My scenarios">{#each saved as entry}<option value={`saved:${entry.id}`}>{entry.document.name}</option>{/each}</optgroup></select></label>
            <span>{description}</span><small>Selecting a map or scenario replaces the current setup.</small>
            <label><input type="checkbox" bind:checked={terrainOverlay}/> Terrain grid and elevation shading</label>
          </div>
          <div class="terrain-inspector" aria-label="Terrain inspector">
            <strong>Inspect terrain</strong><label>Column <input aria-label="Inspect column" type="number" min="0" max={scenario.map.width - 1} bind:value={inspectX}/></label><label>Row <input aria-label="Inspect row" type="number" min="0" max={scenario.map.height - 1} bind:value={inspectY}/></label>
            {#if inspected}<span data-testid="terrain-readout">{catalog.surfaces.find(surface => surface.id === inspected.surface)?.name}: elevation {inspected.elevation.toFixed(1)} m | movement x{inspected.movementMultiplier.toFixed(2)} | {inspected.traversable ? 'traversable' : 'blocked'}</span>{:else}<span>Choose a valid cell.</span>{/if}
          </div>
          <div class="terrain-legend" aria-label="Terrain legend">{#each catalog.surfaces as surface}<span><i style={`background:#${surface.color.toString(16).padStart(6, '0')}`}></i>{surface.name}</span>{/each}</div>
        {/if}
        {#if placement !== null}<div class="coordinate-entry"><span>Or enter a grid position:</span><label>Column <input type="number" min="0" max={(scenario?.map.width ?? 1) - 1} bind:value={deployX}/></label><label>Row <input type="number" min="0" max={(scenario?.map.height ?? 1) - 1} bind:value={deployY}/></label><button disabled={busy || !Number.isInteger(deployX) || !Number.isInteger(deployY) || deployX < 0 || deployY < 0 || deployX >= (scenario?.map.width ?? 0) || deployY >= (scenario?.map.height ?? 0)} onclick={() => mapClick(deployX + 0.5, deployY + 0.5)}>Place soldier</button></div>{/if}
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        <div class="action-bar"><div><strong>{phase === 'setup' ? 'Prepare your forces' : phase === 'finished' ? 'Battle complete' : phase === 'paused' ? 'Battle paused' : 'Battle in progress'}</strong><small>{phase === 'setup' ? 'Place at least one soldier on each side.' : phase === 'paused' ? 'Resume when you are ready.' : 'Doctrine stays fixed during battle.'}</small></div><div class="actions">{#if phase === 'setup'}<button class="primary" data-testid="start-battle" disabled={busy || !analysis?.canStart} onclick={start}>Start battle <span>&rarr;</span></button>{:else if phase === 'running'}<button class="primary" data-testid="pause-battle" onclick={pause}>Pause battle</button>{:else if phase === 'paused'}<button class="primary" data-testid="resume-battle" onclick={resume}>Resume battle</button>{/if}{#if starting && phase !== 'setup'}<button data-testid="reset-battle" onclick={reset}>Reset</button>{/if}</div></div>
        {#if result}<Results {result} {names} factions={scenario?.factions ?? [0, 1]} />{/if}
      </section>
      <aside class="sidebar"><div class="sidebar-head"><p class="eyebrow">FORCE ROSTER</p><h2>Deploy units</h2><p>Build each side, then adjust individual doctrine.</p></div>
        {#each scenario?.factions ?? [0, 1] as faction, side}
          <FactionPanel {faction} {side} name={names[side]} {scenario} {snapshot} {catalog} {selected}
            editable={phase === 'setup'} {busy}
            onSelect={(id) => { selected = id; placement = null; }}
            onAdd={() => { modalFaction = faction; placement = null; }} />
        {/each}
        {#if catalog && selectedUnit && phase === 'setup'}
          <DoctrineEditor unit={selectedUnit} {catalog} {busy} onClose={() => selected = null} onChange={doctrine} onRemove={removeUnit} />
          <InventoryDisplay {catalog} inventory={catalog.defaultInventory} />
        {:else if catalog && selectedUnit && selectedState}
          <section class="editor" data-testid="unit-terrain-status"><h3>Infantry #{selectedUnit.id}</h3><p>{selectedState.terrain.surface} | elevation {selectedState.terrain.elevation.toFixed(1)} m | Actual movement x{selectedState.terrain.movementMultiplier.toFixed(2)}</p><p>{selectedState.terrain.revealed ? 'Revealed' : 'Not revealed'} | {selectedState.terrain.protection === null ? 'No current target' : `${Math.round(selectedState.terrain.protection * 100)}% directional protection against target #${selectedState.targetId}`}</p></section>
          <InventoryDisplay {catalog} inventory={selectedState.inventory} live={selectedState} />
        {/if}
      </aside>
    </div>
  </main>
  {#if modalFaction !== null}
    <UnitPicker name={names[scenario?.factions.indexOf(modalFaction) ?? 0]} onClose={() => modalFaction = null}
      onChoose={() => { placement = modalFaction; modalFaction = null; selected = null; error = ''; }} />
  {/if}
