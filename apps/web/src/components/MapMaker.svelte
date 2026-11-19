<script lang="ts">
  /**
   * MapMaker orchestrator component: manages editor session, undo/redo, Worker terrain analysis,
   * IndexedDB scenario library persistence, canvas gestures, and delegates UI panels to maker/*.svelte.
   */
  import { onMount } from 'svelte';
  import '../maker.css';
  import { Battlefield } from '../lib/battlefield';
  import { BattleClient } from '../lib/client';
  import {
    applyBrushStroke,
    blankDocument,
    copy,
    createSession,
    dragSelection,
    fingerprint,
    isBrushTool,
    moveSelectedAxis,
    pickSelection,
    placeObstacle,
    placeSoldier,
    resizeObstacleAxis,
    resizeScenario,
    type EditorSession,
    type EditorTool,
  } from '../lib/editor';
  import { nextFactionDefinition, parseColorHex } from '../lib/factions';
  import {
    MAX_DOCUMENT_BYTES,
    deleteScenario,
    downloadScenario,
    listScenarios,
    parseScenarioDocument,
    saveScenario,
    type SavedScenario,
  } from '../lib/scenario-library';
  import type {
    Catalog,
    Doctrine,
    PatrolDirection,
    Point,
    ScenarioAnalysis,
    ScenarioDocument,
    Surface,
    WorkerReply,
  } from '../lib/types';
  import MakerDialogs from './maker/MakerDialogs.svelte';
  import MakerSelectionPanel from './maker/MakerSelectionPanel.svelte';
  import MakerSidebar from './maker/MakerSidebar.svelte';
  import MakerToolbar from './maker/MakerToolbar.svelte';

  interface Props {
    session: EditorSession | null;
    onSession: (session: EditorSession) => void;
    onTest: (document: ScenarioDocument) => void;
    onBack: () => void;
  }

  let { session: initialSession, onSession, onTest, onBack }: Props = $props();

  let session = $state.raw<EditorSession | null>(null);
  let catalog = $state.raw<Catalog | null>(null);
  let analysis = $state.raw<ScenarioAnalysis | null>(null);
  let client = $state.raw<BattleClient | null>(null);
  let battlefield = $state.raw<Battlefield | null>(null);
  let saved = $state.raw<SavedScenario[]>([]);
  let guardAction = $state.raw<(() => void) | null>(null);

  let host: HTMLElement;
  let error = $state('');
  let analysisError = $state('');
  let pending = $state(false);
  let revision = $state(0);
  let analyzedRevision = $state(-1);
  let surface = $state<Surface>('open');
  let amount = $state(0.5);
  let setValue = $state(0);
  let obstacleHeight = $state(0.9);
  let faction = $state(0);
  let brushRadius = $state(0);
  let terrainOverlay = $state(true);
  let inspectX = $state(0);
  let inspectY = $state(0);
  let x = $state(0);
  let y = $state(0);
  let width = $state(32);
  let height = $state(20);
  let newWidth = $state(32);
  let newHeight = $state(20);
  let resizePreview = $state(false);
  let deleteId = $state<string | null>(null);
  let renameId = $state<string | null>(null);
  let renameValue = $state('');

  let disposed = false;
  let lastTimeLimit = 600;
  let documentGeneration = 0;
  let lastPoint: Point | null = null;
  let gestureBefore: ScenarioDocument | null = null;
  let gestureStart: Point | null = null;
  let visited = new Set<string>();

  const doc = $derived(session?.document ?? null);
  const scenario = $derived(doc?.scenario ?? null);
  const dirty = $derived(!!session && fingerprint(session.document) !== session.savedSnapshot);
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
  const selectedUnit = $derived(
    session?.selection?.kind === 'unit'
      ? (scenario?.units.find((u) => u.id === session?.selection?.index) ?? null)
      : null,
  );
  const selectedObstacle = $derived(
    session?.selection?.kind === 'obstacle'
      ? (scenario?.map.obstacles[session.selection.index] ?? null)
      : null,
  );
  const selectedPatrolIndex = $derived(
    session?.selection?.kind === 'patrol' ? session.selection.index : null,
  );
  const selectedPatrol = $derived(
    selectedPatrolIndex !== null ? (scenario?.map.patrol[selectedPatrolIndex] ?? null) : null,
  );

  $effect(() => {
    if (scenario && !scenario.factions.some((f) => f.id === faction)) {
      faction = scenario.factions[0]?.id ?? 0;
    }
  });

  $effect(() => {
    if (!battlefield || !scenario || !catalog || !session) return;
    battlefield.update(
      scenario,
      null,
      selectedUnit?.id ?? null,
      false,
      false,
      catalog,
      terrainOverlay,
    );
    battlefield.setEditor({
      onStart: startGesture,
      onMove: moveGesture,
      onEnd: endGesture,
      camera: session.camera ?? undefined,
      onCameraChange: (camera) => {
        if (session) {
          session.camera = camera;
          publish();
        }
      },
      issues: analysis?.issues ?? [],
      steepEdges: analysis?.steepEdges ?? [],
      brushRadius,
      brushAtVertices: session.tool !== 'surface',
      resizePreview:
        resizePreview &&
        Number.isInteger(width) &&
        Number.isInteger(height) &&
        width > 0 &&
        height > 0
          ? { width, height }
          : undefined,
      selection: session.selection,
    });
  });

  function publish() {
    if (!session) return;
    session.settings = {
      surface,
      amount,
      setValue,
      obstacleHeight,
      faction,
      brushRadius,
      terrainOverlay,
    };
    onSession(session);
  }

  function changed(before: ScenarioDocument) {
    if (!session || fingerprint(before) === fingerprint(session.document)) return;
    session.undo = [...session.undo.slice(-99), before];
    session.redo = [];
    session = { ...session };
    publish();
    requestAnalysis();
  }

  function edit(fn: (d: ScenarioDocument) => void) {
    if (!session) return;
    const before = copy(session.document);
    session.document = copy(session.document);
    fn(session.document);
    changed(before);
  }

  function undo() {
    if (!session?.undo.length) return;
    const previous = session.undo[session.undo.length - 1];
    session = {
      ...session,
      document: copy(previous),
      undo: session.undo.slice(0, -1),
      redo: [...session.redo, copy(session.document)],
      selection: null,
    };
    publish();
    requestAnalysis();
  }

  function redo() {
    if (!session?.redo.length) return;
    const next = session.redo[session.redo.length - 1];
    session = {
      ...session,
      document: copy(next),
      redo: session.redo.slice(0, -1),
      undo: [...session.undo.slice(-99), copy(session.document)],
      selection: null,
    };
    publish();
    requestAnalysis();
  }

  function requestAnalysis() {
    revision++;
    analysis = null;
    if (!pending) void analyze();
  }

  async function analyze() {
    if (!session || !client || !catalog || disposed) return;
    const current = revision;
    const candidate = copy(session.document.scenario);
    pending = true;
    try {
      const result = await client.analyze(candidate, current);
      if (!disposed && current === revision) {
        analysis = result;
        analyzedRevision = current;
        analysisError = '';
      }
    } catch (e) {
      if (!disposed && current === revision) {
        analysisError = e instanceof Error ? e.message : String(e);
      }
    } finally {
      pending = false;
      if (!disposed && current !== revision) void analyze();
    }
  }

  function receive(message: WorkerReply) {
    if (message.type === 'ready') {
      catalog = message.catalog;
      if (!session) session = createSession(blankDocument(catalog));
      lastTimeLimit = session.document.scenario.timeLimit ?? catalog.defaultTimeLimit;
      width = session.document.scenario.map.width;
      height = session.document.scenario.map.height;
      publish();
      requestAnalysis();
      void refresh();
    } else if (message.type === 'error') {
      error = message.message;
    }
  }

  async function refresh() {
    if (!catalog || disposed) return;
    try {
      const entries = await listScenarios(catalog);
      if (!disposed) saved = entries;
    } catch (e) {
      if (!disposed) error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    let active = true;
    if (initialSession) {
      session = initialSession;
      if (initialSession.settings) {
        ({ surface, amount, setValue, obstacleHeight, faction, brushRadius, terrainOverlay } =
          initialSession.settings);
      }
    }
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (dirty) {
        event.preventDefault();
        event.returnValue = '';
      }
    };
    const keydown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
        return;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'z') {
        event.preventDefault();
        if (event.shiftKey) redo();
        else undo();
      } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'y') {
        event.preventDefault();
        redo();
      }
    };
    window.addEventListener('keydown', keydown);
    window.addEventListener('beforeunload', beforeUnload);
    void (async () => {
      try {
        battlefield = new Battlefield(
          host,
          () => {},
          (cx, cy) => {
            inspectX = cx;
            inspectY = cy;
          },
        );
        await battlefield.init();
        if (!active) return;
        client = new BattleClient(receive);
        client.initialize();
      } catch (e) {
        if (active) error = e instanceof Error ? e.message : String(e);
      }
    })();
    return () => {
      window.removeEventListener('keydown', keydown);
      window.removeEventListener('beforeunload', beforeUnload);
      active = false;
      disposed = true;
      client?.dispose();
      battlefield?.destroy();
    };
  });

  function exportDocument() {
    if (!doc) return;
    try {
      downloadScenario(doc);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function guard(action: () => void) {
    if (dirty) guardAction = action;
    else action();
  }

  async function save(asNew = false): Promise<boolean> {
    if (!session || !analysis || analyzedRevision !== revision || !analysis.structuralValid) {
      error = 'Wait for validation and fix structural errors before saving.';
      return false;
    }
    try {
      const generation = documentGeneration;
      const candidate = copy(session.document);
      const result = await saveScenario(
        candidate,
        asNew ? undefined : (session.savedId ?? undefined),
      );
      if (disposed || generation !== documentGeneration || !session) {
        await refresh();
        return false;
      }
      session = { ...session, savedId: result.id, savedSnapshot: fingerprint(candidate) };
      publish();
      await refresh();
      return true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      return false;
    }
  }

  async function saveAndContinue() {
    if (await save()) {
      if (session && fingerprint(session.document) !== session.savedSnapshot) {
        error = 'The scenario changed while saving. Save again before leaving.';
        return;
      }
      const action = guardAction;
      guardAction = null;
      action?.();
    }
  }

  async function duplicate(item: SavedScenario) {
    try {
      const document = copy(item.document);
      document.name = `${document.name} copy`.slice(0, 100);
      await saveScenario(document);
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function confirmRename() {
    const item = saved.find((entry) => entry.id === renameId);
    if (!item) return;
    try {
      const document = copy(item.document);
      document.name = renameValue;
      await saveScenario(document, item.id);
      if (session?.savedId === item.id && fingerprint(session.document) === session.savedSnapshot) {
        session = { ...session, document: copy(document), savedSnapshot: fingerprint(document) };
        publish();
      }
      renameId = null;
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function confirmDelete() {
    if (!deleteId) return;
    try {
      await deleteScenario(deleteId);
      if (session?.savedId === deleteId) {
        session = { ...session, savedId: null, savedSnapshot: null };
        publish();
      }
      deleteId = null;
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function load(document: ScenarioDocument, id: string | null = null) {
    documentGeneration++;
    error = '';
    session = createSession(document, id);
    width = document.scenario.map.width;
    height = document.scenario.map.height;
    publish();
    requestAnalysis();
    battlefield?.fit();
  }

  async function openSaved(item: SavedScenario) {
    try {
      if (!client) throw new Error('Simulation is not ready.');
      const result = await client.analyze(item.document.scenario, revision);
      if (!result.structuralValid)
        throw new Error(result.issues.map((issue) => issue.message).join(' '));
      load(item.document, item.id);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function importSaved(document: ScenarioDocument) {
    load(document);
    const generation = documentGeneration;
    try {
      const result = await saveScenario(document);
      if (!disposed && generation === documentGeneration && session) {
        session = { ...session, savedId: result.id, savedSnapshot: fingerprint(document) };
        publish();
      }
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function imported(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    try {
      if (file.size > MAX_DOCUMENT_BYTES)
        throw new Error('Scenario files must be no larger than 8 MiB.');
      if (!client || !catalog) throw new Error('Simulation is not ready.');
      const document = parseScenarioDocument(await file.text(), catalog);
      const result = await client.analyze(document.scenario, revision);
      if (!result.structuralValid)
        throw new Error(result.issues.map((issue) => issue.message).join(' '));
      guard(() => {
        void importSaved(document);
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function changeTool(tool: EditorTool) {
    if (!session) return;
    session = { ...session, tool, selection: null };
    publish();
  }

  function brush(point: Point) {
    if (!session || !scenario || !catalog) return;
    const nextDoc = copy(session.document);
    const applied = applyBrushStroke(
      nextDoc.scenario.map,
      session.tool,
      point,
      { surface, amount, setValue, brushRadius },
      catalog,
      visited,
    );
    if (!applied) return;
    session.document = nextDoc;
    session = { ...session };
  }

  function startGesture(point: Point) {
    if (!session || !scenario || !catalog) return;
    gestureBefore = copy(session.document);
    session.document = copy(session.document);
    const current = session.document.scenario;
    gestureStart = point;
    lastPoint = point;
    visited = new Set();
    if (isBrushTool(session.tool)) {
      brush(point);
    } else if (session.tool === 'select') {
      session = { ...session, selection: pickSelection(current, point) };
      publish();
    } else if (session.tool === 'patrol') {
      current.map.patrol.push({ x: Math.floor(point.x) + 0.5, y: Math.floor(point.y) + 0.5 });
      session.selection = { kind: 'patrol', index: current.map.patrol.length - 1 };
    } else if (session.tool === 'soldier') {
      const id = placeSoldier(current, faction, point, catalog);
      if (id !== null) session.selection = { kind: 'unit', index: id };
    }
    session = { ...session };
  }

  function moveGesture(point: Point) {
    if (!gestureBefore || !session || !isBrushTool(session.tool)) return;
    const from = lastPoint ?? point;
    const steps = Math.max(1, Math.ceil(Math.hypot(point.x - from.x, point.y - from.y) * 2));
    for (let i = 1; i <= steps; i++) {
      brush({
        x: from.x + ((point.x - from.x) * i) / steps,
        y: from.y + ((point.y - from.y) * i) / steps,
      });
    }
    lastPoint = point;
  }

  function endGesture(point: Point) {
    if (!gestureBefore || !gestureStart || !session) return;
    const current = session.document.scenario;
    if (session.tool === 'obstacle') {
      const index = placeObstacle(current, gestureStart, point, obstacleHeight);
      session.selection = { kind: 'obstacle', index };
    } else if (
      session.tool === 'select' &&
      session.selection &&
      Math.hypot(point.x - gestureStart.x, point.y - gestureStart.y) > 0.1
    ) {
      dragSelection(current, session.selection, point);
    }
    const before = gestureBefore;
    gestureBefore = null;
    gestureStart = null;
    changed(before);
  }

  function coordinateAction() {
    if (
      !scenario ||
      !Number.isInteger(x) ||
      !Number.isInteger(y) ||
      x < 0 ||
      y < 0 ||
      x >= scenario.map.width ||
      y >= scenario.map.height
    )
      return;
    const point = { x: Number(x) + 0.5, y: Number(y) + 0.5 };
    startGesture(point);
    endGesture(point);
  }

  function removeSelected() {
    if (!session?.selection) return;
    const s = session.selection;
    edit((d) => {
      if (s.kind === 'unit') d.scenario.units = d.scenario.units.filter((u) => u.id !== s.index);
      else if (s.kind === 'obstacle') d.scenario.map.obstacles.splice(s.index, 1);
      else d.scenario.map.patrol.splice(s.index, 1);
    });
    session = { ...session, selection: null };
    publish();
  }

  function reorderPatrol(direction: number) {
    if (!session?.selection || session.selection.kind !== 'patrol' || !scenario) return;
    const from = session.selection.index;
    const to = from + direction;
    if (to < 0 || to >= scenario.map.patrol.length) return;
    edit((d) => {
      const p = d.scenario.map.patrol.splice(from, 1)[0];
      d.scenario.map.patrol.splice(to, 0, p);
    });
    session = { ...session, selection: { kind: 'patrol', index: to } };
    publish();
  }

  function applyResize() {
    if (
      !scenario ||
      !catalog ||
      !Number.isInteger(width) ||
      !Number.isInteger(height) ||
      width < catalog.editorRules.minDimension ||
      height < catalog.editorRules.minDimension ||
      width > catalog.editorRules.maxDimension ||
      height > catalog.editorRules.maxDimension
    )
      return;
    edit((d) => {
      d.scenario = resizeScenario(d.scenario, width, height);
    });
    if (session) {
      session = { ...session, selection: null };
      publish();
    }
    resizePreview = false;
  }

  function changeNumber(field: 'seed' | 'timeLimit', input: HTMLInputElement) {
    if (!scenario) return;
    const value = Number(input.value);
    const valid =
      input.value.trim() !== '' &&
      Number.isFinite(value) &&
      (field === 'seed'
        ? Number.isSafeInteger(value) && value >= 0 && value <= 0xffffffff
        : value >= 1 && value <= 3600);
    if (!valid) {
      error =
        field === 'seed'
          ? 'Seed must be a whole number from 0 to 4294967295.'
          : 'Time limit must be between 1 and 3600 seconds.';
      input.value = scenario[field] === null ? '' : String(scenario[field]);
      return;
    }
    if (field === 'timeLimit') lastTimeLimit = value;
    edit((d) => {
      d.scenario[field] = value;
    });
  }

  function toggleNoTimeLimit(unlimited: boolean) {
    if (!scenario || !catalog) return;
    if (unlimited && scenario.timeLimit !== null) lastTimeLimit = scenario.timeLimit;
    const fallback = lastTimeLimit || catalog.defaultTimeLimit;
    edit((d) => {
      d.scenario.timeLimit = unlimited ? null : fallback;
    });
  }

  function moveSelected(axis: 'x' | 'y', value: number) {
    if (!session?.selection || !scenario) return;
    const selection = session.selection;
    edit((d) => {
      moveSelectedAxis(d.scenario, selection, axis, value);
    });
  }

  function changeDoctrine(field: keyof Doctrine, value: Doctrine[keyof Doctrine]) {
    if (!selectedUnit) return;
    const id = selectedUnit.id;
    edit((d) => {
      const u = d.scenario.units.find((unit) => unit.id === id);
      if (u) u.doctrine = { ...u.doctrine, [field]: value };
    });
  }

  function changeFaction(value: number) {
    if (
      !selectedUnit ||
      !scenario ||
      !catalog ||
      !scenario.factions.some((f) => f.id === value) ||
      selectedUnit.faction === value
    )
      return;
    if (scenario.units.filter((unit) => unit.faction === value).length >= catalog.maxPerFaction) {
      error = 'That faction already has the maximum number of soldiers.';
      return;
    }
    const id = selectedUnit.id;
    edit((d) => {
      const unit = d.scenario.units.find((entry) => entry.id === id);
      if (unit) unit.faction = value;
    });
  }

  function removeFaction(id: number) {
    if (!scenario || scenario.factions.length <= 2) return;
    edit((d) => {
      d.scenario.factions = d.scenario.factions.filter((f) => f.id !== id);
      d.scenario.units = d.scenario.units.filter((u) => u.faction !== id);
    });
    if (selectedUnit?.faction === id && session) {
      session = { ...session, selection: null };
      publish();
    }
  }

  function renameFaction(id: number, input: HTMLInputElement) {
    if (!scenario) return;
    const name = input.value.trim();
    const existing = scenario.factions.find((f) => f.id === id);
    if (!existing) return;
    if (!name || name.length > 64) {
      error = 'Faction names must be non-empty and at most 64 characters.';
      input.value = existing.name;
      return;
    }
    edit((d) => {
      const target = d.scenario.factions.find((f) => f.id === id);
      if (target) target.name = name;
    });
  }

  function sizeObstacle(axis: 'width' | 'height', value: number) {
    if (!selectedObstacle || session?.selection?.kind !== 'obstacle' || !scenario) return;
    const index = session.selection.index;
    edit((d) => {
      resizeObstacleAxis(d.scenario, index, axis, value);
    });
  }

  function changeObstacleHeight(value: number) {
    if (
      !catalog ||
      session?.selection?.kind !== 'obstacle' ||
      !Number.isFinite(value) ||
      value <= 0 ||
      value > catalog.editorRules.maxObstacleHeight
    )
      return;
    const index = session.selection.index;
    edit((d) => {
      d.scenario.map.obstacles[index].heightMetres = value;
    });
  }

  function copyBuiltIn(mapId: string) {
    if (!catalog) return;
    const preset = catalog.maps.find((m) => m.id === mapId);
    if (!preset) return;
    const activeCatalog = catalog;
    guard(() => {
      const d = blankDocument(activeCatalog);
      d.name = preset.name;
      d.description = preset.description;
      d.scenario.map = copy(preset.map);
      load(d);
    });
  }
</script>

<main class="maker">
  <header>
    <button onclick={() => guard(onBack)}>Main menu</button>
    <h1>MapMaker</h1>
    <span>{dirty ? 'Unsaved changes' : 'Saved'}</span><button
      disabled={!session?.undo.length}
      onclick={undo}
    >
      Undo
    </button><button disabled={!session?.redo.length} onclick={redo}>Redo</button><button
      onclick={() => save()}
    >
      Save
    </button><button onclick={() => save(true)}>Save as copy</button><button
      disabled={!analysis?.structuralValid || analyzedRevision !== revision}
      onclick={exportDocument}
    >
      Export
    </button><button
      disabled={!analysis?.canStart || analyzedRevision !== revision}
      onclick={() => {
        if (doc) {
          publish();
          onTest(copy(doc));
        }
      }}
    >
      Test battle
    </button>
  </header>
  {#if error}<p role="alert" class="error">{error}</p>{/if}{#if analysisError}<p
      role="alert"
      class="error"
    >
      {analysisError}
    </p>{/if}
  <div class="layout">
    <section class="map-area">
      {#if catalog && session && scenario && doc}
        <div class="map-tools">
          <button onclick={() => battlefield?.fit()}>Fit map</button><button
            onclick={() => battlefield?.zoomBy(1.25)}
          >
            Zoom in
          </button><button onclick={() => battlefield?.zoomBy(0.8)}>Zoom out</button><span>
            {scenario.map.width} × {scenario.map.height}
          </span>
        </div>
      {/if}
      <div class="map" bind:this={host} aria-label="Map editor"></div>
      {#if catalog && session && scenario && doc}
        <MakerToolbar
          {catalog}
          {scenario}
          tool={session.tool}
          bind:brushRadius
          bind:surface
          bind:amount
          bind:setValue
          bind:obstacleHeight
          bind:faction
          bind:x
          bind:y
          bind:terrainOverlay
          bind:inspectX
          bind:inspectY
          {inspected}
          onChangeTool={changeTool}
          onCoordinateAction={coordinateAction}
        />
        <MakerSelectionPanel
          {catalog}
          {scenario}
          {selectedUnit}
          {selectedObstacle}
          {selectedPatrol}
          {selectedPatrolIndex}
          {analysis}
          checking={pending || analyzedRevision !== revision}
          onMoveSelected={moveSelected}
          onChangeFaction={changeFaction}
          onChangeDoctrine={changeDoctrine}
          onClearSelection={() => {
            if (session) session = { ...session, selection: null };
          }}
          onRemoveSelected={removeSelected}
          onSizeObstacle={sizeObstacle}
          onObstacleHeight={changeObstacleHeight}
          onReorderPatrol={reorderPatrol}
        />
      {/if}
    </section>
    {#if catalog && session && scenario && doc}
      <MakerSidebar
        {catalog}
        {doc}
        {scenario}
        {saved}
        bind:width
        bind:height
        bind:newWidth
        bind:newHeight
        bind:resizePreview
        onNameChange={(value) =>
          edit((d) => {
            d.name = value;
          })}
        onDescriptionChange={(value) =>
          edit((d) => {
            d.description = value;
          })}
        onNumberChange={changeNumber}
        onToggleNoTimeLimit={toggleNoTimeLimit}
        onRecolorFaction={(id, hex) =>
          edit((d) => {
            const target = d.scenario.factions.find((f) => f.id === id);
            if (target) target.color = parseColorHex(hex);
          })}
        onRenameFaction={renameFaction}
        onPatrolDirection={(id, direction: PatrolDirection) =>
          edit((d) => {
            const target = d.scenario.factions.find((f) => f.id === id);
            if (target) target.patrolDirection = direction;
          })}
        onRemoveFaction={removeFaction}
        onAddFaction={() =>
          edit((d) => {
            d.scenario.factions.push(nextFactionDefinition(d.scenario.factions));
          })}
        onApplyResize={applyResize}
        onSelectUnit={(id) => {
          if (!session) return;
          session = { ...session, tool: 'select', selection: { kind: 'unit', index: id } };
          publish();
        }}
        onSelectPatrol={(index) => {
          if (!session) return;
          session = { ...session, tool: 'select', selection: { kind: 'patrol', index } };
          publish();
        }}
        onNewBlank={() =>
          catalog && guard(() => load(blankDocument(catalog!, newWidth, newHeight)))}
        onCopyBuiltIn={copyBuiltIn}
        onImport={imported}
        onOpenSaved={(item) =>
          guard(() => {
            void openSaved(item);
          })}
        onRequestRename={(item) => {
          renameId = item.id;
          renameValue = item.document.name;
        }}
        onDuplicate={duplicate}
        onRequestDelete={(id) => (deleteId = id)}
      />
    {:else}<p>Preparing MapMaker...</p>{/if}
  </div>
  <MakerDialogs
    showRename={renameId !== null}
    bind:renameValue
    showDelete={deleteId !== null}
    showUnsavedGuard={guardAction !== null}
    onConfirmRename={confirmRename}
    onCancelRename={() => (renameId = null)}
    onConfirmDelete={confirmDelete}
    onCancelDelete={() => (deleteId = null)}
    onSaveAndContinue={saveAndContinue}
    onDiscardAndContinue={() => {
      const action = guardAction;
      guardAction = null;
      action?.();
    }}
    onCancelGuard={() => (guardAction = null)}
  />
</main>
