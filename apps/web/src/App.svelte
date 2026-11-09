<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { BattleClient } from './lib/client';
  import { Battlefield } from './lib/battlefield';
  import FactionPanel from './components/FactionPanel.svelte';
  import DoctrineEditor from './components/DoctrineEditor.svelte';
  import UnitPicker from './components/UnitPicker.svelte';
  import Results from './components/Results.svelte';
  import type { BattleResult, Catalog, Scenario, Snapshot, UnitPlacement, WorkerReply } from './lib/types';

  let screen: 'home' | 'options' | 'skirmish' = 'home';
  let catalog: Catalog | null = null;
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
  let showHp = true;
  let fullscreen = false;
  let host: HTMLElement;
  let client: BattleClient | null = null;
  let battlefield: Battlefield | null = null;
  let generation = 0;
  const names = ['North', 'South'];
  const copy = <T,>(value: T): T => structuredClone(value);
  $: selectedUnit = scenario?.units.find(unit => unit.id === selected) ?? null;
  $: if (battlefield && scenario) battlefield.update(scenario, snapshot, selected, showHp, catalog?.unit.maxHp ?? 100, phase === 'running');

  function receive(message: WorkerReply) {
    if (message.type === 'ready') {
      catalog = message.catalog;
      scenario = { map: copy(catalog.map), seed: catalog.defaultSeed, factions: [0, 1], units: [], timeLimit: catalog.defaultTimeLimit };
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
    closeBattle(); screen = 'skirmish'; phase = 'loading'; error = '';
    const current = ++generation;
    await tick();
    if (current !== generation || !host) return;
    try {
      battlefield = new Battlefield(host, mapClick);
      await battlefield.init();
      if (current !== generation) return;
      client = new BattleClient(receive);
      client.initialize();
    } catch (e) { if (current === generation) { error = e instanceof Error ? e.message : String(e); phase = 'error'; } }
  }
  function closeBattle() {
    generation++; client?.dispose(); client = null; battlefield?.destroy(); battlefield = null;
    catalog = null; scenario = null; snapshot = null; result = null; starting = null; selected = null; placement = null; modalFaction = null; busy = false;
  }
  function back() { closeBattle(); screen = 'home'; }
  async function mapClick(x: number, y: number) {
    if (!scenario || !catalog || phase !== 'setup' || busy) return;
    if (placement !== null) {
      const faction = placement;
      const candidate = copy(scenario);
      candidate.units.push({ id: Math.max(0, ...candidate.units.map(unit => unit.id)) + 1, faction, definitionId: catalog.unit.id, position: {x: Math.floor(x) + 0.5, y: Math.floor(y) + 0.5}, doctrine: copy(catalog.defaultDoctrine) });
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
  function doctrine(field: 'engagementDistance' | 'fallbackHp', value: number) {
    if (selectedUnit) editUnit(selectedUnit.id, {doctrine: {...selectedUnit.doctrine, [field]: value}});
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
      starting = copy(candidate); snapshot = null; result = null; selected = null; placement = null;
      client.start(candidate);
      if (document.hidden) { client.pause(); phase = 'paused'; } else phase = 'running';
    } catch (e) { if (token === generation) error = e instanceof Error ? e.message : String(e); }
    finally { if (token === generation) busy = false; }
  }
  function reset() {
    if (!client || !starting) return;
    client.reset(); scenario = copy(starting); snapshot = null; result = null; selected = null; placement = null; error = ''; phase = 'setup';
  }
  function pause() { if (phase === 'running') { client?.pause(); phase = 'paused'; } }
  function resume() { if (phase === 'paused') { client?.resume(); phase = 'running'; } }
  async function toggleFullscreen() {
    error = '';
    try { if (document.fullscreenElement) await document.exitFullscreen(); else await document.documentElement.requestFullscreen(); }
    catch (e) { error = `Fullscreen unavailable: ${e instanceof Error ? e.message : String(e)}`; }
  }
  onMount(() => {
    const visibility = () => { if (document.hidden) pause(); };
    const fullscreenChange = () => { fullscreen = !!document.fullscreenElement; };
    const keydown = (event: KeyboardEvent) => { if (event.key === 'Escape') { modalFaction = null; placement = null; } };
    document.addEventListener('visibilitychange', visibility); document.addEventListener('fullscreenchange', fullscreenChange); window.addEventListener('keydown', keydown);
    return () => { document.removeEventListener('visibilitychange', visibility); document.removeEventListener('fullscreenchange', fullscreenChange); window.removeEventListener('keydown', keydown); closeBattle(); };
  });
</script>

<svelte:head><title>2nd Hand Combat</title><meta name="description" content="A small tactical sandbox of autonomous infantry." /></svelte:head>
{#if screen === 'home'}
  <main class="landing">
    <div class="landing-top"><span>FIELD MANUAL / 01</span><span>INFANTRY SANDBOX</span></div>
    <div class="landing-content">
      <p class="eyebrow">A tactical sandbox</p><h1>2nd Hand<br/><em>Combat</em><span class="period">.</span></h1>
      <p class="intro">Set the doctrine. Place your soldiers. Watch the battle unfold.</p>
      <div class="menu"><button class="menu-main" onclick={openSkirmish}><span>01</span><strong>Skirmish</strong><b>&rarr;</b></button><button disabled><span>02</span><strong>Campaign</strong><small>COMING LATER</small></button><button disabled><span>03</span><strong>Puzzles</strong><small>COMING LATER</small></button><button onclick={() => { error = ''; screen = 'options'; }}><span>04</span><strong>Options</strong><b>&rarr;</b></button></div>
    </div><div class="landing-foot">A STUDY IN SMALL-UNIT TACTICS <span>&bull;</span> EST. 01</div>
  </main>
{:else if screen === 'options'}
  <main class="options-page"><header class="topbar"><button class="back" onclick={() => screen = 'home'}>&larr; Back</button><span>FIELD MANUAL / OPTIONS</span></header><section class="options-card"><p class="eyebrow">Preferences / 01</p><h1>Options<span class="period">.</span></h1><p>Make the battlefield your own.</p><div class="option-row"><div><strong>Fullscreen</strong><small>{fullscreen ? 'On' : 'Off'}</small></div><button onclick={toggleFullscreen}>{fullscreen ? 'Exit fullscreen' : 'Enter fullscreen'}</button></div><div class="option-row"><div><strong>Health indicators</strong><small>Show health above soldiers during battle</small></div><label class="check"><input type="checkbox" bind:checked={showHp}/> Show</label></div>{#if error}<p class="error" role="alert">{error}</p>{/if}</section></main>
{:else}
  <main class="workspace"><header class="topbar"><button class="back" onclick={back}>&larr; Main menu</button><div class="brand">2ND HAND <span>COMBAT</span></div><div class="top-status" data-testid="battle-status"><i></i>{phase === 'setup' ? 'SETUP' : phase.toUpperCase()}</div></header>
    {#if phase === 'loading' || phase === 'error'}<div class="loading"><p class="eyebrow">FIELD OPERATIONS</p><h2>{phase === 'loading' ? 'Preparing the field…' : 'Unable to prepare the field'}</h2>{#if error}<p role="alert">{error}</p><button onclick={openSkirmish}>Retry</button>{/if}<button class="text-button" onclick={back}>Back to menu</button></div>{/if}
    <div class:hidden={phase === 'loading' || phase === 'error'} class="battle-layout">
      <section class="field-column"><div class="field-heading"><div><p class="eyebrow">OPERATION / SKIRMISH</p><h1>Battlefield<span class="period">.</span></h1></div><div class="field-meta">32 × 20 <span> / </span> SEED {scenario?.seed}</div></div>
        <div class="map-frame"><div class="map-top"><span>TACTICAL MAP</span><span>{phase === 'setup' ? 'DEPLOYMENT' : 'LIVE VIEW'} / {snapshot ? `${snapshot.elapsed.toFixed(1)}s` : '00:00'}</span></div><div class="battlefield" data-testid="battlefield" bind:this={host}></div><div class="map-bottom"><span><i class="dot north"></i>NORTH</span><span><i class="dot south"></i>SOUTH</span><span><i class="square cover"></i>COVER</span><span><i class="square wall"></i>WALL</span><span class="map-hint">{placement !== null ? `CLICK THE MAP TO PLACE ${names[placement].toUpperCase()} INFANTRY · ESC TO CANCEL` : phase === 'setup' ? 'SELECT A SOLDIER TO EDIT DOCTRINE' : 'AUTONOMOUS COMBAT'}</span></div></div>
        {#if placement !== null}<div class="coordinate-entry"><span>Or enter a grid position:</span><label>Column <input type="number" min="0" max={(scenario?.map.width ?? 1) - 1} bind:value={deployX}/></label><label>Row <input type="number" min="0" max={(scenario?.map.height ?? 1) - 1} bind:value={deployY}/></label><button disabled={busy || !Number.isInteger(deployX) || !Number.isInteger(deployY) || deployX < 0 || deployY < 0 || deployX >= (scenario?.map.width ?? 0) || deployY >= (scenario?.map.height ?? 0)} onclick={() => mapClick(deployX + 0.5, deployY + 0.5)}>Place soldier</button></div>{/if}
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        <div class="action-bar"><div><strong>{phase === 'setup' ? 'Prepare your forces' : phase === 'finished' ? 'Battle complete' : phase === 'paused' ? 'Battle paused' : 'Battle in progress'}</strong><small>{phase === 'setup' ? 'Place at least one soldier on each side.' : phase === 'paused' ? 'Resume when you are ready.' : 'Doctrine stays fixed during battle.'}</small></div><div class="actions">{#if phase === 'setup'}<button class="primary" data-testid="start-battle" disabled={busy || !scenario?.units.some(u => u.faction === 0) || !scenario?.units.some(u => u.faction === 1)} onclick={start}>Start battle <span>&rarr;</span></button>{:else if phase === 'running'}<button class="primary" data-testid="pause-battle" onclick={pause}>Pause battle</button>{:else if phase === 'paused'}<button class="primary" data-testid="resume-battle" onclick={resume}>Resume battle</button>{/if}{#if starting && phase !== 'setup'}<button data-testid="reset-battle" onclick={reset}>Reset</button>{/if}</div></div>
        {#if result}<Results {result} {names} />{/if}
      </section>
      <aside class="sidebar"><div class="sidebar-head"><p class="eyebrow">FORCE ROSTER</p><h2>Deploy units</h2><p>Build each side, then adjust individual doctrine.</p></div>
        {#each [0, 1] as faction}
          <FactionPanel {faction} name={names[faction]} {scenario} {snapshot} {catalog} {selected}
            editable={phase === 'setup'} {busy}
            onSelect={(id) => { selected = id; placement = null; }}
            onAdd={() => { modalFaction = faction; placement = null; }} />
        {/each}
        {#if selectedUnit && phase === 'setup'}
          <DoctrineEditor unit={selectedUnit} {busy} onClose={() => selected = null} onChange={doctrine} onRemove={removeUnit} />
        {/if}
      </aside>
    </div>
  </main>
  {#if modalFaction !== null}
    <UnitPicker name={names[modalFaction]} onClose={() => modalFaction = null}
      onChoose={() => { placement = modalFaction; modalFaction = null; selected = null; error = ''; }} />
  {/if}
{/if}

