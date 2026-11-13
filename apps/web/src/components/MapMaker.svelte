<script lang="ts">
  import { onMount } from 'svelte';
  import { BattleClient } from '../lib/client';
  import { Battlefield } from '../lib/battlefield';
  import DoctrineEditor from './DoctrineEditor.svelte';
  import InventoryDisplay from './InventoryDisplay.svelte';
  import { blankDocument, copy, createSession, fingerprint, paint, resizeScenario, type EditorSession, type EditorTool } from '../lib/editor';
  import { listScenarios, saveScenario, deleteScenario, parseScenarioDocument, downloadScenario, MAX_DOCUMENT_BYTES, type SavedScenario } from '../lib/scenario-library';
  import type { Catalog, Doctrine, Point, ScenarioAnalysis, ScenarioDocument, Surface, WorkerReply } from '../lib/types';

  export let session: EditorSession | null;
  export let onSession: (session: EditorSession) => void;
  export let onTest: (document: ScenarioDocument) => void;
  export let onBack: () => void;

  let catalog: Catalog | null = null, analysis: ScenarioAnalysis | null = null;
  let client: BattleClient | null = null, battlefield: Battlefield | null = null, host: HTMLElement;
  let saved: SavedScenario[] = [], error = '', analysisError = '', pending = false, revision = 0, analyzedRevision = -1, disposed = false;
  let surface: Surface = 'open', amount = 0.5, setValue = 0, obstacleHeight = 0.9, faction = 0, brushRadius = 0;
  let terrainOverlay = true, inspectX = 0, inspectY = 0;
  let lastPoint: Point | null = null;
  let x = 0, y = 0, width = 32, height = 20, newWidth = 32, newHeight = 20, resizePreview = false, importInput: HTMLInputElement;
  let gestureBefore: ScenarioDocument | null = null, gestureStart: Point | null = null, visited = new Set<string>();
  let guardAction: (() => void) | null = null;
  let documentGeneration = 0;
  let deleteId: string | null = null, renameId: string | null = null, renameValue = '';
  $: doc = session?.document ?? null;
  $: scenario = doc?.scenario ?? null;
  $: inspected = scenario && analysis && Number.isInteger(inspectX) && Number.isInteger(inspectY) && inspectX >= 0 && inspectY >= 0 && inspectX < scenario.map.width && inspectY < scenario.map.height ? analysis.cells[inspectY * scenario.map.width + inspectX] : null;
  $: dirty = !!session && fingerprint(session.document) !== session.savedSnapshot;
  $: selectedUnit = session?.selection?.kind === 'unit' ? scenario?.units.find(u => u.id === session?.selection?.index) ?? null : null;
  $: selectedObstacle = session?.selection?.kind === 'obstacle' ? scenario?.map.obstacles[session.selection.index] ?? null : null;
  $: selectedPatrol = session?.selection?.kind === 'patrol' ? scenario?.map.patrol[session.selection.index] ?? null : null;
  $: if (battlefield && scenario && catalog && session) {
    battlefield.update(scenario, null, selectedUnit?.id ?? null, false, catalog.unit.maxHp, false, catalog, terrainOverlay);
    battlefield.setEditor({ onStart: startGesture, onMove: moveGesture, onEnd: endGesture, camera: session.camera ?? undefined, onCameraChange: camera => { if (session) { session.camera = camera; publish(); } }, issues: analysis?.issues ?? [], steepEdges: analysis?.steepEdges ?? [], brushRadius, brushAtVertices: session.tool !== 'surface', resizePreview: resizePreview && Number.isInteger(width) && Number.isInteger(height) && width > 0 && height > 0 ? {width, height} : undefined, selection: session.selection });
  }
  function publish() { if (session) { session.settings = {surface, amount, setValue, obstacleHeight, faction, brushRadius, terrainOverlay}; onSession(session); } }
  function changed(before: ScenarioDocument) {
    if (!session || fingerprint(before) === fingerprint(session.document)) return;
    session.undo = [...session.undo.slice(-99), before]; session.redo = []; session = {...session}; publish(); requestAnalysis();
  }
  function edit(fn: (doc: ScenarioDocument) => void) { if (!session) return; const before = copy(session.document); session.document = copy(session.document); fn(session.document); changed(before); }
  function undo() { if (!session?.undo.length) return; const previous = session.undo[session.undo.length - 1]; session = {...session, document: copy(previous), undo: session.undo.slice(0,-1), redo: [...session.redo, copy(session.document)], selection: null}; publish(); requestAnalysis(); }
  function redo() { if (!session?.redo.length) return; const next = session.redo[session.redo.length - 1]; session = {...session, document: copy(next), redo: session.redo.slice(0,-1), undo: [...session.undo.slice(-99), copy(session.document)], selection: null}; publish(); requestAnalysis(); }
  function requestAnalysis() { revision++; analysis = null; if (!pending) void analyze(); }
  async function analyze() {
    if (!session || !client || !catalog || disposed) return;
    const current = revision, candidate = copy(session.document.scenario); pending = true;
    try { const result = await client.analyze(candidate, current); if (!disposed && current === revision) { analysis = result; analyzedRevision = current; analysisError = '';  } }
    catch (e) { if (!disposed && current === revision) analysisError = e instanceof Error ? e.message : String(e); }
    finally { pending = false; if (!disposed && current !== revision) void analyze(); }
  }
  function receive(message: WorkerReply) {
    if (message.type === 'ready') { catalog = message.catalog; if (!session) session = createSession(blankDocument(catalog)); width = session.document.scenario.map.width; height = session.document.scenario.map.height; publish(); requestAnalysis(); void refresh(); }
    else if (message.type === 'error') error = message.message;
  }
  async function refresh() { if (!catalog || disposed) return; try { const entries = await listScenarios(catalog.defaultDoctrine); if (!disposed) saved = entries; } catch(e) { if (!disposed) error = e instanceof Error ? e.message : String(e); } }
  onMount(() => {
    let active = true;
    if (session?.settings) ({surface, amount, setValue, obstacleHeight, faction, brushRadius, terrainOverlay} = session.settings);
    const beforeUnload = (event: BeforeUnloadEvent) => { if (dirty) { event.preventDefault(); event.returnValue = ''; } };
    const keydown = (event: KeyboardEvent) => { const target=event.target as HTMLElement; if (target.isContentEditable || ['INPUT','TEXTAREA','SELECT'].includes(target.tagName)) return; if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z') { event.preventDefault(); if(event.shiftKey) redo(); else undo(); } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'y') { event.preventDefault(); redo(); } };
    window.addEventListener('keydown',keydown);
    window.addEventListener('beforeunload', beforeUnload);
    void (async () => { try { battlefield = new Battlefield(host, () => {}, (x, y) => { inspectX = x; inspectY = y; }); await battlefield.init(); if (!active) return; client = new BattleClient(receive); client.initialize(); } catch(e) { if (active) error = e instanceof Error ? e.message : String(e); } })();
    return () => { window.removeEventListener('keydown',keydown); window.removeEventListener('beforeunload',beforeUnload); active = false; disposed = true; client?.dispose(); battlefield?.destroy(); };
  });
  function exportDocument() { if (!doc) return; try { downloadScenario(doc); } catch(e) { error=e instanceof Error ? e.message : String(e); } }
  function guard(action: () => void) { if (dirty) guardAction = action; else action(); }
  async function save(asNew = false): Promise<boolean> {
    if (!session || !analysis || analyzedRevision !== revision || !analysis.structuralValid) { error = 'Wait for validation and fix structural errors before saving.'; return false; }
    try { const generation=documentGeneration, candidate = copy(session.document); const result = await saveScenario(candidate, asNew ? undefined : session.savedId ?? undefined); if (disposed || generation !== documentGeneration || !session) { await refresh(); return false; } session = {...session, savedId: result.id, savedSnapshot: fingerprint(candidate)}; publish(); await refresh(); return true; }
    catch(e) { error = e instanceof Error ? e.message : String(e); return false; }
  }
  async function saveAndContinue() { if (await save()) { if (session && fingerprint(session.document)!==session.savedSnapshot) { error='The scenario changed while saving. Save again before leaving.'; return; } const action = guardAction; guardAction = null; action?.(); } }
  async function duplicate(item: SavedScenario) { try { const document=copy(item.document); document.name = `${document.name} copy`.slice(0,100); await saveScenario(document); await refresh(); } catch(e) { error=e instanceof Error ? e.message : String(e); } }
  async function confirmRename() { const item=saved.find(entry=>entry.id===renameId); if(!item) return; try { const document=copy(item.document); document.name=renameValue; await saveScenario(document,item.id); if(session?.savedId===item.id && fingerprint(session.document)===session.savedSnapshot) {session={...session,document:copy(document),savedSnapshot:fingerprint(document)}; publish();} renameId=null; await refresh(); } catch(e) {error=e instanceof Error ? e.message : String(e);} }
  async function confirmDelete() { if (!deleteId) return; try { await deleteScenario(deleteId); if(session?.savedId===deleteId) { session={...session,savedId:null,savedSnapshot:null}; publish(); } deleteId=null; await refresh(); } catch(e) {error=e instanceof Error ? e.message : String(e);} }
  function load(document: ScenarioDocument, id: string | null = null) { documentGeneration++; error = ''; session = createSession(document, id); width = document.scenario.map.width; height = document.scenario.map.height; publish(); requestAnalysis(); battlefield?.fit(); }
  async function openSaved(item: SavedScenario) { try { if(!client) throw new Error('Simulation is not ready.'); const result=await client.analyze(item.document.scenario, revision); if(!result.structuralValid) throw new Error(result.issues.map(issue=>issue.message).join(' ')); load(item.document,item.id); } catch(e) { error=e instanceof Error ? e.message : String(e); } }
  async function importSaved(document: ScenarioDocument) {
    load(document);
    const generation = documentGeneration;
    try {
      const result = await saveScenario(document);
      if (!disposed && generation === documentGeneration && session) { session = {...session, savedId: result.id, savedSnapshot: fingerprint(document)}; publish(); }
      await refresh();
    } catch(e) { error=e instanceof Error ? e.message : String(e); }
  }
  async function imported(event: Event) {
    const input = event.currentTarget as HTMLInputElement, file = input.files?.[0]; input.value = ''; if (!file) return;
    try { if (file.size > MAX_DOCUMENT_BYTES) throw new Error('Scenario files must be no larger than 8 MiB.'); if (!client || !catalog) throw new Error('Simulation is not ready.'); const document = parseScenarioDocument(await file.text(), catalog.defaultDoctrine); const result = await client.analyze(document.scenario, revision); if (!result.structuralValid) throw new Error(result.issues.map(issue => issue.message).join(' ')); guard(() => { void importSaved(document); }); }
    catch(e) { error = e instanceof Error ? e.message : String(e); }
  }
  function changeTool(tool: EditorTool) { if (!session) return; session = {...session, tool, selection: null}; publish(); }
  function pointKey(point: Point) { return session?.tool === 'surface' ? `${Math.floor(point.x)},${Math.floor(point.y)}` : `${Math.round(point.x)},${Math.round(point.y)}`; }
  function brush(point: Point) {
    if (!session || !scenario || !catalog || !Number.isInteger(brushRadius) || brushRadius < 0 || brushRadius > 5 || !Number.isFinite(amount) || amount <= 0 || !Number.isFinite(setValue)) return;
    session.document = copy(session.document);
    const map = session.document.scenario.map;
    for (let dy = -brushRadius; dy <= brushRadius; dy++) for (let dx = -brushRadius; dx <= brushRadius; dx++) {
      const p = {x: point.x + dx, y: point.y + dy};
      if (p.x < 0 || p.y < 0 || p.x >= map.width || p.y >= map.height) continue;
      const key = pointKey(p);
      if (visited.has(key)) continue;
      visited.add(key);
      paint(map, session.tool, p, surface, amount, setValue, catalog.editorRules.minElevation, catalog.editorRules.maxElevation);
    }
    session = {...session};
  }
  function startGesture(point: Point) {
    if (!session || !scenario || !catalog) return;
    gestureBefore = copy(session.document); session.document = copy(session.document); scenario = session.document.scenario; gestureStart = point; lastPoint = point; visited = new Set();
    if (['surface','raise','lower','set','smooth'].includes(session.tool)) brush(point);
    else if (session.tool === 'select') {
      const unit = scenario.units.find(u => Math.hypot(u.position.x-point.x,u.position.y-point.y)<0.7);
      const patrol = scenario.map.patrol.findIndex(p => Math.hypot(p.x-point.x,p.y-point.y)<0.5);
      const obstacle = scenario.map.obstacles.findIndex(o => point.x >= o.x && point.x < o.x+o.width && point.y >= o.y && point.y < o.y+o.height);
      session = {...session, selection: unit ? {kind:'unit',index:unit.id} : patrol >= 0 ? {kind:'patrol',index:patrol} : obstacle >= 0 ? {kind:'obstacle',index:obstacle} : null}; publish();
    } else if (session.tool === 'patrol') { scenario.map.patrol.push({x: Math.floor(point.x)+0.5,y: Math.floor(point.y)+0.5}); session.selection = {kind:'patrol',index:scenario.map.patrol.length-1}; }
    else if (session.tool === 'soldier' && scenario.units.filter(u => u.faction === scenario!.factions[faction]).length < catalog.maxPerFaction) { const ids = new Set(scenario.units.map(u=>u.id)); let id=1; while(ids.has(id)) id++; scenario.units.push({id,faction:scenario.factions[faction],definitionId:catalog.unit.id,position:{x:Math.floor(point.x)+0.5,y:Math.floor(point.y)+0.5},doctrine:copy(catalog.defaultDoctrine)}); session.selection = {kind:'unit',index:id}; }
    session = {...session};
  }
  function moveGesture(point: Point) { if (!gestureBefore || !session) return; if (['surface','raise','lower','set','smooth'].includes(session.tool)) { const from=lastPoint ?? point, steps=Math.max(1,Math.ceil(Math.hypot(point.x-from.x,point.y-from.y)*2)); for(let i=1;i<=steps;i++) brush({x:from.x+(point.x-from.x)*i/steps,y:from.y+(point.y-from.y)*i/steps}); lastPoint=point; } }
  function endGesture(point: Point) {
    if (!gestureBefore || !gestureStart || !session || !scenario) return;
    if (session.tool === 'obstacle') { const x1=Math.floor(Math.min(gestureStart.x,point.x)), y1=Math.floor(Math.min(gestureStart.y,point.y)), x2=Math.floor(Math.max(gestureStart.x,point.x)), y2=Math.floor(Math.max(gestureStart.y,point.y)); scenario.map.obstacles.push({x:x1,y:y1,width:x2-x1+1,height:y2-y1+1,heightMetres:obstacleHeight}); session.selection={kind:'obstacle',index:scenario.map.obstacles.length-1}; }
    else if (session.tool === 'select' && session.selection && Math.hypot(point.x-gestureStart.x,point.y-gestureStart.y)>0.1) { const p={x:Math.floor(point.x)+0.5,y:Math.floor(point.y)+0.5}, s=session.selection; if(s.kind==='unit') {const unit=scenario.units.find(u=>u.id===s.index); if(unit) unit.position=p;} else if(s.kind==='patrol') scenario.map.patrol[s.index]=p; else {const obstacle=scenario.map.obstacles[s.index]; if(obstacle) { obstacle.x=Math.max(0,Math.min(scenario.map.width-obstacle.width,Math.floor(point.x))); obstacle.y=Math.max(0,Math.min(scenario.map.height-obstacle.height,Math.floor(point.y))); }} }
    const before=gestureBefore; gestureBefore=null; gestureStart=null; changed(before);
  }
  function coordinateAction() { const point={x:Number(x)+0.5,y:Number(y)+0.5}; if (!scenario || !Number.isInteger(x) || !Number.isInteger(y) || x<0 || y<0 || x>=scenario.map.width || y>=scenario.map.height) return; startGesture(point); endGesture(point); }
  function removeSelected() { if (!session?.selection) return; const s=session.selection; edit(d=>{if(s.kind==='unit') d.scenario.units=d.scenario.units.filter(u=>u.id!==s.index); else if(s.kind==='obstacle') d.scenario.map.obstacles.splice(s.index,1); else d.scenario.map.patrol.splice(s.index,1);}); session.selection=null; session={...session}; publish(); }
  function reorder(direction: number) { if (!session?.selection || session.selection.kind !== 'patrol' || !scenario) return; const from=session.selection.index, to=from+direction; if(to<0 || to>=scenario.map.patrol.length) return; edit(d=>{ const p=d.scenario.map.patrol.splice(from,1)[0]; d.scenario.map.patrol.splice(to,0,p); }); session.selection={kind:'patrol',index:to}; session={...session}; publish(); }
  function applyResize() { if (!scenario || !Number.isInteger(width) || !Number.isInteger(height) || width<catalog!.editorRules.minDimension || height<catalog!.editorRules.minDimension || width>catalog!.editorRules.maxDimension || height>catalog!.editorRules.maxDimension) return; edit(d=>{d.scenario=resizeScenario(d.scenario,width,height);}); if(session) session.selection=null; resizePreview=false; }
  function changeNumber(field: 'seed' | 'timeLimit', input: HTMLInputElement) { if (!scenario) return; const value=Number(input.value); const valid=input.value.trim()!=='' && Number.isFinite(value) && (field==='seed' ? Number.isSafeInteger(value) && value>=0 && value<=0xffffffff : value>=1 && value<=3600); if(!valid) { error=field==='seed' ? 'Seed must be a whole number from 0 to 4294967295.' : 'Time limit must be between 1 and 3600 seconds.'; input.value=String(scenario[field]); return; } edit(d=>{d.scenario[field]=value;}); }
  function moveSelected(axis: 'x' | 'y', value: number) { if (!session?.selection || !scenario || !Number.isFinite(value)) return; const selection=session.selection; const limit=axis==='x' ? scenario.map.width : scenario.map.height; if(value<0 || value>=limit) return; edit(d=>{ if(selection.kind==='unit') { const unit=d.scenario.units.find(u=>u.id===selection.index); if(unit) unit.position[axis]=value; } else if(selection.kind==='patrol') d.scenario.map.patrol[selection.index][axis]=value; else { const obstacle=d.scenario.map.obstacles[selection.index]; const size=axis==='x' ? obstacle.width : obstacle.height; if(Number.isInteger(value) && value+size<=limit) obstacle[axis]=value; } }); }
  function changeDoctrine(field: keyof Doctrine, value: Doctrine[keyof Doctrine]) { if (!selectedUnit) return; const id=selectedUnit.id; edit(d=>{ const u=d.scenario.units.find(u=>u.id===id); if(u) u.doctrine={...u.doctrine,[field]:value}; }); }
  function changeFaction(value: number) {
    if (!selectedUnit || !scenario || !catalog || !scenario.factions.includes(value) || selectedUnit.faction === value) return;
    if (scenario.units.filter(unit=>unit.faction===value).length >= catalog.maxPerFaction) { error='That faction already has the maximum number of soldiers.'; return; }
    const id=selectedUnit.id;
    edit(d=>{const unit=d.scenario.units.find(unit=>unit.id===id); if(unit) unit.faction=value;});
  }
  function sizeObstacle(axis: 'width' | 'height', value: number) {
    if (!selectedObstacle || session?.selection?.kind !== 'obstacle' || !scenario || !Number.isInteger(value) || value < 1) return;
    const limit = axis === 'width' ? scenario.map.width - selectedObstacle.x : scenario.map.height - selectedObstacle.y;
    if (value > limit) return;
    const index = session.selection.index;
    edit(d=>{d.scenario.map.obstacles[index][axis]=value;});
  }
</script>

<main class="maker">
  <header><button onclick={() => guard(onBack)}>Main menu</button><h1>MapMaker</h1><span>{dirty ? 'Unsaved changes' : 'Saved'}</span><button disabled={!session?.undo.length} onclick={undo}>Undo</button><button disabled={!session?.redo.length} onclick={redo}>Redo</button><button onclick={() => save()}>Save</button><button onclick={() => save(true)}>Save as copy</button><button disabled={!analysis?.structuralValid || analyzedRevision !== revision} onclick={exportDocument}>Export</button><button disabled={!analysis?.canStart || analyzedRevision !== revision} onclick={() => { if(doc) { publish(); onTest(copy(doc)); } }}>Test battle</button></header>
  {#if error}<p role="alert" class="error">{error}</p>{/if}{#if analysisError}<p role="alert" class="error">{analysisError}</p>{/if}
  <div class="layout"><section class="map-area">
  {#if catalog && session && scenario && doc}
    <div class="map-tools"><button onclick={() => battlefield?.fit()}>Fit map</button><button onclick={() => battlefield?.zoomBy(1.25)}>Zoom in</button><button onclick={() => battlefield?.zoomBy(0.8)}>Zoom out</button><span>{scenario.map.width} × {scenario.map.height}</span></div>
    {/if}<div class="map" bind:this={host} aria-label="Map editor"></div>{#if catalog && session && scenario && doc}
    <div class="tools" aria-label="Editor tools">{#each ['surface','raise','lower','set','smooth','obstacle','select','patrol','soldier'] as tool}<button class:active={session.tool === tool} onclick={() => changeTool(tool as EditorTool)}>{tool}</button>{/each}</div>
    <div class="controls">
      {#if ['surface','raise','lower','set','smooth'].includes(session.tool)}<label>Brush radius <input type="number" min="0" max="5" step="1" bind:value={brushRadius}/></label>{/if}
      {#if session.tool === 'surface'}<label>Surface <select bind:value={surface}>{#each catalog.surfaces as item}<option value={item.id}>{item.name}</option>{/each}</select></label>{/if}
      {#if session.tool === 'raise' || session.tool === 'lower'}<label>Elevation amount <input type="number" min="0.1" max="100" step="0.1" bind:value={amount}/></label>{/if}
      {#if session.tool === 'set'}<label>Set elevation <input type="number" min={catalog.editorRules.minElevation} max={catalog.editorRules.maxElevation} step="0.1" bind:value={setValue}/></label>{/if}
      {#if session.tool === 'obstacle'}<label>Obstacle height <select bind:value={obstacleHeight}><option value={catalog.lowObstacleHeight}>Low</option><option value={catalog.fullObstacleHeight}>Full</option></select></label>{/if}
      {#if session.tool === 'soldier'}<label>Faction <select aria-label="Faction" bind:value={faction}><option value={0}>North</option><option value={1}>South</option></select></label>{/if}
      <label>Column <input aria-label="Edit column" type="number" min="0" max={scenario.map.width-1} bind:value={x}/></label><label>Row <input aria-label="Edit row" type="number" min="0" max={scenario.map.height-1} bind:value={y}/></label><button onclick={coordinateAction}>Apply at coordinates</button>
    </div>
    <div class="controls"><label><input type="checkbox" bind:checked={terrainOverlay}/> Elevation overlay</label><span>Wheel to zoom; middle or right drag to pan.</span></div>
    <div class="controls" aria-label="Terrain inspector"><label>Inspect column <input type="number" min="0" max={scenario.map.width-1} bind:value={inspectX}/></label><label>Inspect row <input type="number" min="0" max={scenario.map.height-1} bind:value={inspectY}/></label>{#if inspected}<span data-testid="editor-terrain-readout">{catalog.surfaces.find(item=>item.id===inspected.surface)?.name}: {inspected.elevation.toFixed(1)} m | movement x{inspected.movementMultiplier.toFixed(2)} | {inspected.traversable ? 'traversable' : 'blocked'}</span>{/if}</div>
    <div class="controls" aria-label="Terrain legend">{#each catalog.surfaces as item}<span><i style={`display:inline-block;width:12px;height:12px;margin-right:4px;background:#${item.color.toString(16).padStart(6,'0')}`}></i>{item.name}</span>{/each}<span>Amber: steep slope · Red: invalid placement</span></div>
    {#if selectedUnit || selectedPatrol || selectedObstacle}
      <div class="controls"><strong>Selected position</strong><label>X <input aria-label="Selected X" type="number" step="0.5" value={selectedUnit?.position.x ?? selectedPatrol?.x ?? selectedObstacle?.x} onchange={(e) => moveSelected('x',Number(e.currentTarget.value))}/></label><label>Y <input aria-label="Selected Y" type="number" step="0.5" value={selectedUnit?.position.y ?? selectedPatrol?.y ?? selectedObstacle?.y} onchange={(e) => moveSelected('y',Number(e.currentTarget.value))}/></label></div>
    {/if}
    {#if selectedUnit}<div class="controls"><label>Selected faction <select aria-label="Selected faction" value={selectedUnit.faction} onchange={(e)=>changeFaction(Number(e.currentTarget.value))}>{#each scenario.factions as side, index}<option value={side}>{index===0 ? 'North' : 'South'}</option>{/each}</select></label></div><DoctrineEditor unit={selectedUnit} {catalog} busy={false} onClose={() => {if(session) session={...session,selection:null};}} onChange={changeDoctrine} onRemove={removeSelected}/><InventoryDisplay {catalog} inventory={catalog.defaultInventory}/>{/if}
    {#if selectedObstacle}<div class="controls"><strong>Obstacle</strong><label>Width <input aria-label="Obstacle width" type="number" min="1" max={scenario.map.width-selectedObstacle.x} value={selectedObstacle.width} onchange={(e)=>sizeObstacle('width',Number(e.currentTarget.value))}/></label><label>Depth <input aria-label="Obstacle depth" type="number" min="1" max={scenario.map.height-selectedObstacle.y} value={selectedObstacle.height} onchange={(e)=>sizeObstacle('height',Number(e.currentTarget.value))}/></label><label>Height (m) <input type="number" min="0.1" max={catalog.editorRules.maxObstacleHeight} step="0.1" value={selectedObstacle.heightMetres} onchange={(e) => {const value=Number(e.currentTarget.value), index=session!.selection!.index; if(Number.isFinite(value) && value>0 && value<=catalog!.editorRules.maxObstacleHeight) edit(d=>{d.scenario.map.obstacles[index].heightMetres=value;});}}/></label><button onclick={removeSelected}>Delete obstacle</button></div>{/if}
    {#if selectedPatrol}<div class="controls"><strong>Patrol point {session.selection!.index+1}</strong><button onclick={() => reorder(-1)}>Move earlier</button><button onclick={() => reorder(1)}>Move later</button><button onclick={removeSelected}>Delete patrol point</button></div>{/if}
    <section aria-label="Validation"><h2>Validation</h2>{#if pending || analyzedRevision !== revision}<p>Checking scenario…</p>{:else if analysis}<p>{analysis.canStart ? 'Ready to test' : analysis.structuralValid ? 'Draft can be saved; resolve issues to test.' : 'Fix structural issues before saving.'}</p>{#each analysis.issues as issue}<p class:error={issue.severity === 'error'}>{issue.severity}: {issue.message}</p>{/each}{/if}</section>
  {/if}</section>{#if catalog && session && scenario && doc}<aside>
    <section><h2>Scenario</h2><label>Name <input maxlength="100" value={doc.name} onchange={(e) => edit(d=>{d.name=e.currentTarget.value;})}/></label><label>Description <textarea maxlength="2000" value={doc.description} onchange={(e) => edit(d=>{d.description=e.currentTarget.value;})}></textarea></label><label>Seed <input type="number" min="0" max="4294967295" value={scenario.seed} onchange={(e) => changeNumber('seed',e.currentTarget)}/></label><label>Time limit <input type="number" min="1" value={scenario.timeLimit} onchange={(e) => changeNumber('timeLimit',e.currentTarget)}/></label></section>
    <section><h2>Map size</h2><label>Width <input type="number" min={catalog.editorRules.minDimension} max={catalog.editorRules.maxDimension} bind:value={width}/></label><label>Height <input type="number" min={catalog.editorRules.minDimension} max={catalog.editorRules.maxDimension} bind:value={height}/></label><button onclick={() => resizePreview=true}>Preview resize</button>{#if resizePreview}<p>Northwest anchor. {scenario.units.filter(u=>u.position.x>=width||u.position.y>=height).length} soldiers and {scenario.map.patrol.filter(p=>p.x>=width||p.y>=height).length} patrol points removed; {scenario.map.obstacles.filter(o=>o.x+o.width>width||o.y+o.height>height).length} obstacles clipped or removed.</p><button onclick={applyResize}>Apply resize</button><button onclick={() => resizePreview=false}>Cancel resize</button>{/if}</section>
    <section><h2>Soldiers</h2>{#each scenario.units as unit}<button onclick={() => {session={...session!,tool:'select',selection:{kind:'unit',index:unit.id}}; publish();}}>#{unit.id} {unit.faction === scenario.factions[0] ? 'North' : 'South'}</button>{/each}<h2>Patrol</h2>{#each scenario.map.patrol as point, index}<button onclick={() => {session={...session!,tool:'select',selection:{kind:'patrol',index}}; publish();}}>Point {index+1}: {point.x}, {point.y}</button>{/each}</section>
    <section><h2>Library</h2><label>New map width <input type="number" min={catalog.editorRules.minDimension} max={catalog.editorRules.maxDimension} bind:value={newWidth}/></label><label>New map height <input type="number" min={catalog.editorRules.minDimension} max={catalog.editorRules.maxDimension} bind:value={newHeight}/></label><button disabled={!Number.isInteger(newWidth) || !Number.isInteger(newHeight) || newWidth < catalog.editorRules.minDimension || newHeight < catalog.editorRules.minDimension || newWidth > catalog.editorRules.maxDimension || newHeight > catalog.editorRules.maxDimension} onclick={() => guard(() => load(blankDocument(catalog!,newWidth,newHeight)))}>New blank</button><label>Copy built-in <select onchange={(e) => {const preset=catalog!.maps.find(m=>m.id===e.currentTarget.value); if(preset) guard(()=>{const d=blankDocument(catalog!); d.name=preset.name; d.description=preset.description; d.scenario.map=copy(preset.map); load(d);}); e.currentTarget.value='';}}><option value="">Choose map</option>{#each catalog.maps as preset}<option value={preset.id}>{preset.name}</option>{/each}</select></label><button onclick={() => importInput.click()}>Import</button><input bind:this={importInput} type="file" accept=".json,application/json" hidden onchange={imported}/>{#each saved as item}<div class="library-item"><strong>{item.document.name}</strong><button onclick={() => guard(() => { void openSaved(item); })}>Open</button><button onclick={() => {renameId=item.id; renameValue=item.document.name;}}>Rename</button><button onclick={() => duplicate(item)}>Duplicate</button><button onclick={() => deleteId=item.id}>Delete</button></div>{/each}</section>
  </aside>{:else}<p>Preparing MapMaker...</p>{/if}</div>
  {#if renameId}<div class="modal" role="dialog" aria-modal="true" aria-label="Rename scenario"><div><h2>Rename scenario</h2><label>Name <input bind:value={renameValue} maxlength="100"/></label><button onclick={confirmRename}>Rename</button><button onclick={() => renameId=null}>Cancel</button></div></div>{/if}
  {#if deleteId}<div class="modal" role="dialog" aria-modal="true" aria-label="Delete scenario"><div><h2>Delete scenario?</h2><p>This removes the saved copy from the library.</p><button onclick={confirmDelete}>Delete</button><button onclick={() => deleteId=null}>Cancel</button></div></div>{/if}
  {#if guardAction}<div class="modal" role="dialog" aria-modal="true" aria-label="Unsaved changes"><div><h2>Unsaved changes</h2><p>Save this scenario before leaving?</p><button onclick={saveAndContinue}>Save</button><button onclick={() => {const action=guardAction; guardAction=null; action?.();}}>Discard</button><button onclick={() => guardAction=null}>Cancel</button></div></div>{/if}
</main>
<style>
  .maker{padding:1rem;min-height:100vh;background:#e9e4d4;color:#293126}
  header,.map-tools,.tools,.controls{display:flex;align-items:center;gap:.5rem;flex-wrap:wrap}
  header h1{margin:0 auto 0 0;font-size:32px}h2{font-size:23px;margin-bottom:.5rem}
  button{border:1px solid #a5aa98;background:#f5f2e8;color:#364033;padding:7px 10px;font-size:12px;font-weight:600}
  button:not(:disabled):hover{background:#e2e4d5}.tools button{text-transform:capitalize}
  .layout{display:grid;grid-template-columns:minmax(0,1fr) 300px;gap:1rem;margin-top:1rem}
  .map{height:min(60vh,650px);min-height:320px;background:#aaa;border:6px solid #424b3b;overflow:hidden}
  .map-area section,aside section{background:#f8f5ea;padding:1rem;margin-top:1rem}
  aside label{display:block;margin:.6rem 0;font-size:13px}aside label input,aside textarea{display:block;margin-top:.3rem}
  input,textarea,select{max-width:100%;padding:.4rem;border:1px solid #b8bcaa;background:#faf8ef;color:#30382d;font:inherit}
  textarea{width:100%;min-height:4rem}input[type=number]{width:6rem}.controls{font-size:12px}
  .controls label{display:flex;align-items:center;gap:.35rem}.tools,.controls{margin-top:.6rem}
  .active{outline:2px solid #315f62;background:#dde5d9}.error{color:#a33}
  .library-item{padding:.7rem 0;border-top:1px solid #b8bcaa}.library-item strong{display:block;margin-bottom:.4rem}
  .modal{position:fixed;inset:0;width:auto;padding:1rem;background:#202b22b8;box-shadow:none;display:grid;place-items:center;z-index:20}
  .modal>div{width:min(100%,420px);background:#faf7ea;padding:2rem;box-shadow:0 15px 60px #17211755}
  .modal label{display:block;margin:1rem 0}.modal button{margin:.5rem .3rem 0 0}
  @media(max-width:850px){.layout{grid-template-columns:1fr}}
</style>
