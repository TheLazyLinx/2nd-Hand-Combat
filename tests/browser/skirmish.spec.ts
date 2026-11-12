import { test, expect } from '@playwright/test';
import { clickMap, latestSnapshot, observeWorkers, openSkirmish, place } from './helpers';

test.beforeEach(async ({ page }) => observeWorkers(page));

test('splash exposes the supported modes and session options', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: /Campaign/ })).toBeDisabled();
  await expect(page.getByRole('button', { name: /Puzzles/ })).toBeDisabled();
  await page.getByRole('button', { name: /Options/ }).click();
  await page.getByRole('checkbox', { name: 'Show' }).uncheck();
  await page.getByRole('button', { name: /Back/ }).click();
  await page.getByRole('button', { name: /Options/ }).click();
  await expect(page.getByRole('checkbox', { name: 'Show' })).not.toBeChecked();
  await page.getByRole('button', { name: 'Enter fullscreen' }).click();
  await expect.poll(() => page.evaluate(() => !!document.fullscreenElement)).toBe(true);
  await page.getByRole('button', { name: 'Exit fullscreen' }).click();
  await expect.poll(() => page.evaluate(() => !!document.fullscreenElement)).toBe(false);
});

test('setup validates walls and overlap through Rust and keeps per-soldier doctrine', async ({ page }) => {
  await openSkirmish(page);
  await expect(page.getByTestId('start-battle')).toBeDisabled();
  await page.getByTestId('add-soldier-0').click();
  await page.getByTestId('choose-infantry').click();
  await clickMap(page, 15.5, 5.5);
  await expect(page.getByRole('alert')).toContainText(/open ground/);
  await clickMap(page, 3.5, 2.5);
  await expect(page.getByTestId('selected-unit')).toBeVisible();
  await page.getByTestId('engagement-distance').fill('5');
  await page.getByTestId('fallback-hp').fill('65');
  await page.getByTestId('position-preference').selectOption('cover');
  await page.getByTestId('route-preference').selectOption('protected');
  await page.getByTestId('add-soldier-1').click();
  await page.getByTestId('choose-infantry').click();
  await clickMap(page, 3.5, 2.5);
  await expect(page.getByRole('alert')).toContainText(/overlap/);
  await clickMap(page, 10.5, 2.5);
  await expect(page.getByTestId('engagement-distance')).toHaveValue('8');
  await expect(page.getByTestId('start-battle')).toBeEnabled();
  await page.getByTestId('faction-0').getByRole('button', { name: /Infantry #1/ }).click();
  await expect(page.getByTestId('engagement-distance')).toHaveValue('5');
  await expect(page.getByTestId('fallback-hp')).toHaveValue('65');
  await expect(page.getByTestId('position-preference')).toHaveValue('cover');
  await expect(page.getByTestId('route-preference')).toHaveValue('protected');
});

test('real WASM moves and damages units; pause freezes time and reset restores setup', async ({ page }) => {
  await openSkirmish(page);
  await place(page, 0, 2.5, 2.5);
  await page.getByTestId('engagement-distance').fill('4');
  await place(page, 1, 12.5, 2.5);
  await page.getByTestId('start-battle').click();
  await expect.poll(async () => (await latestSnapshot(page))?.units[0].position.x).toBeGreaterThan(2.5);
  await expect.poll(async () => (await latestSnapshot(page))?.units.some(unit => unit.hp < 100), { timeout: 12_000 }).toBe(true);
  await page.getByTestId('pause-battle').click();
  await expect(page.getByTestId('battle-status')).toHaveText('PAUSED');
  const paused = await latestSnapshot(page);
  await page.waitForTimeout(350); // Assert an actual frozen interval, not merely the pause label.
  expect(await latestSnapshot(page)).toEqual(paused);
  await page.getByTestId('resume-battle').click();
  await expect.poll(async () => (await latestSnapshot(page))?.tick).toBeGreaterThan(paused!.tick);
  await page.getByTestId('reset-battle').click();
  await expect(page.getByTestId('battle-status')).toHaveText('SETUP');
  await page.getByTestId('faction-0').getByRole('button', { name: /Infantry #1/ }).click();
  await expect(page.getByTestId('engagement-distance')).toHaveValue('4');
  await expect(page.getByTestId('faction-0')).toContainText('100 HP');
  // An old result delivered after reset must not replace the current setup.
  await page.evaluate(() => {
    window.observedWorkers.at(-1)!.worker.dispatchEvent(new MessageEvent('message', {
      data: { type: 'result', battleId: 1, result: { winner: 0, reason: 'elimination', elapsed: 1, factions: [] } },
    }));
  });
  await expect(page.getByTestId('battle-status')).toHaveText('SETUP');
  await page.getByTestId('start-battle').click();
  await expect.poll(() => page.evaluate(() => {
    const starts = window.observedWorkers.at(-1)!.sent.filter(message => message.type === 'start');
    return starts.length === 2 && JSON.stringify(starts[0].scenario) === JSON.stringify(starts[1].scenario);
  })).toBe(true);
});

test('changing doctrine changes the first firing position in the same setup', async ({ page }) => {
  await openSkirmish(page);
  await place(page, 0, 2.5, 2.5);
  await page.getByTestId('engagement-distance').fill('12');
  await place(page, 1, 12.5, 2.5);
  await page.getByTestId('start-battle').click();
  await expect.poll(async () => (await latestSnapshot(page))?.units[0].shotsFired).toBeGreaterThan(0);
  const firstLongRangeX = await page.evaluate(() => {
    const message = window.observedWorkers.at(-1)!.received.find(m => m.type === 'snapshot' && m.snapshot.units[0].shotsFired > 0);
    return message?.type === 'snapshot' ? message.snapshot.units[0].position.x : NaN;
  });
  await page.getByTestId('reset-battle').click();
  await page.getByTestId('faction-0').getByRole('button', { name: /Infantry #1/ }).click();
  await page.getByTestId('engagement-distance').fill('2');
  await page.getByTestId('start-battle').click();
  await expect.poll(() => page.evaluate(() => window.observedWorkers.at(-1)!.received.some(m => m.type === 'snapshot' && m.battleId === 3 && m.snapshot.units[0].shotsFired > 0)), { timeout: 12_000 }).toBe(true);
  const firstShortRangeX = await page.evaluate(() => {
    const message = window.observedWorkers.at(-1)!.received.find(m => m.type === 'snapshot' && m.battleId === 3 && m.snapshot.units[0].shotsFired > 0);
    return message?.type === 'snapshot' ? message.snapshot.units[0].position.x : NaN;
  });
  expect(firstShortRangeX).toBeGreaterThan(firstLongRangeX + 1);
});

test('an elimination result is displayed and can be reset', async ({ page }) => {
  await openSkirmish(page);
  await place(page, 0, 2.5, 2.5);
  await page.getByTestId('fallback-hp').fill('0');
  await place(page, 1, 7.5, 2.5);
  await page.getByTestId('fallback-hp').fill('0');
  await page.getByTestId('start-battle').click();
  await expect(page.getByTestId('battle-status')).toHaveText('FINISHED', { timeout: 20_000 });
  await expect(page.getByText('AFTER ACTION REPORT')).toBeVisible();
  await expect(page.getByText(/survivors \/ .*casualties/).first()).toBeVisible();
  await page.getByTestId('reset-battle').click();
  await expect(page.getByTestId('battle-status')).toHaveText('SETUP');
});

test('leaving and reopening terminates the old Worker and starts empty', async ({ page }) => {
  await openSkirmish(page);
  await place(page, 0, 2.5, 2.5);
  await page.getByRole('button', { name: /Main menu/ }).click();
  await page.getByRole('button', { name: /01 Skirmish/ }).click();
  await expect(page.getByTestId('battle-status')).toHaveText('SETUP');
  await expect(page.getByTestId('faction-0')).toContainText('0 / 8');
  expect(await page.evaluate(() => window.observedWorkers.map(worker => worker.terminated))).toEqual([true, false]);
});

test('WASM initialization failure is visible and retry recovers', async ({ page }) => {
  await page.route('**/*.wasm', route => route.abort());
  await page.goto('/');
  await page.getByRole('button', { name: /01 Skirmish/ }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
  await page.unroute('**/*.wasm');
  await page.getByRole('button', { name: 'Retry' }).click();
  await expect(page.getByTestId('battle-status')).toHaveText('SETUP');
});

test('the real production Worker validates input and ends at simulation time limit', async ({ page }) => {
  await openSkirmish(page);
  const messages = await page.evaluate(async () => {
    const { worker, received } = window.observedWorkers.at(-1)!;
    const ready = received.find(message => message.type === 'ready');
    if (!ready || ready.type !== 'ready') throw new Error('No catalog from Rust');
    const scenario = {
      map: { width: 32, height: 20, surfaces: Array(640).fill('open'), elevations: Array(693).fill(0), obstacles: [], patrol: [] },
      seed: 1, factions: [0, 1], timeLimit: 1,
      units: [0, 1].map(faction => ({ id: faction + 1, faction, definitionId: 'infantry', position: { x: faction ? 30.5 : 1.5, y: 1.5 }, doctrine: { ...ready.catalog.defaultDoctrine } })),
    };
    return await new Promise<Window['observedWorkers'][number]['received']>((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('Worker timeout test did not finish')), 5000);
      worker.addEventListener('message', function handler(event) {
        if (event.data.type === 'result') {
          clearTimeout(timer); worker.removeEventListener('message', handler); resolve(received);
        }
      });
      worker.postMessage({ type: 'validate', requestId: 999, forStart: true, scenario: { ...scenario, units: [] } });
      worker.postMessage({ type: 'start', battleId: 100, scenario });
    });
  });
  const validation = messages.find(m => m.type === 'validation' && m.requestId === 999);
  expect(validation?.type === 'validation' && validation.validation.valid).toBe(false);
  const result = messages.find(m => m.type === 'result');
  expect(result?.type === 'result' && result.result).toMatchObject({ winner: null, reason: 'timeout', elapsed: 1 });
  expect(messages.filter(m => m.type === 'result')).toHaveLength(1);
});


test('map selection clears deployment and exposes authoritative terrain inspection', async ({ page }) => {
  await openSkirmish(page);
  await expect(page.getByTestId('map-preset')).toHaveValue('training');
  await place(page, 0, 2.5, 2.5);
  await page.getByTestId('map-preset').selectOption('woodland');
  await expect(page.getByTestId('faction-0')).toContainText('0 / 8');
  await page.getByLabel('Inspect column').fill('6');
  await page.getByLabel('Inspect row').fill('3');
  await expect(page.getByTestId('terrain-readout')).toContainText('Woodland');
  await expect(page.getByTestId('terrain-readout')).toContainText('movement x0.70');
  await place(page, 0, 2.5, 2.5);
  await page.getByTestId('route-preference').selectOption('protected');
  await place(page, 1, 12.5, 2.5);
  await page.getByTestId('start-battle').click();
  await expect.poll(async () => (await latestSnapshot(page))?.tick).toBeGreaterThan(2);
  await page.getByTestId('faction-0').getByRole('button', {name: /Infantry #1/}).click();
  await expect(page.getByTestId('unit-terrain-status')).toContainText(/revealed/i);
  await page.getByTestId('reset-battle').click();
  await expect(page.getByTestId('map-preset')).toHaveValue('woodland');
  await page.getByTestId('faction-0').getByRole('button', {name: /Infantry #1/}).click();
  await expect(page.getByTestId('route-preference')).toHaveValue('protected');
});

test('each preset sustains sixteen units without prolonged worker or frame stalls', async ({ page }, testInfo) => {
  test.setTimeout(240_000);
  await page.setViewportSize({width: 1440, height: 900});
  await openSkirmish(page);
  for (const id of ['training', 'ridge', 'woodland']) {
    await page.getByTestId('map-preset').selectOption(id);
    for (let i = 0; i < 16; i++) await place(page, i < 8 ? 0 : 1, i < 8 ? 9.5 : 22.5, 2.5 + (i % 8) * 2);
    await page.screenshot({path: testInfo.outputPath(`${id}-wide.png`), fullPage: true});
    if (id === 'ridge') {
      await page.setViewportSize({width: 900, height: 900});
      await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
      await page.screenshot({path: testInfo.outputPath(`${id}-narrow.png`), fullPage: true});
      await page.setViewportSize({width: 1440, height: 900});
    }
    await page.evaluate(() => {
      const state = {previous: performance.now(), gaps: [] as number[], active: true};
      (window as unknown as {frameProbe: typeof state}).frameProbe = state;
      const frame = (now: number) => {state.gaps.push(now - state.previous); state.previous = now; if (state.active) requestAnimationFrame(frame);};
      requestAnimationFrame(frame);
    });
    await page.getByTestId('start-battle').click();
    const started = Date.now();
    const samples: {wallMs: number; tick: number}[] = [];
    for (let second = 0; second < 8; second++) {
      await page.waitForTimeout(1000);
      samples.push({wallMs: Date.now() - started, tick: (await latestSnapshot(page))?.tick ?? 0});
    }
    const frames = await page.evaluate(() => {
      const probe = (window as unknown as {frameProbe: {active: boolean; gaps: number[]}}).frameProbe;
      probe.active = false;
      const sorted = probe.gaps.sort((a, b) => a - b);
      return {max: sorted.at(-1) ?? Infinity, p95: sorted[Math.floor(sorted.length * 0.95)] ?? Infinity};
    });
    const errors = await page.evaluate(() => window.observedWorkers.at(-1)!.received.filter(m => m.type === 'error'));
    const shots = (await latestSnapshot(page))?.units.reduce((total, unit) => total + unit.shotsFired, 0) ?? 0;
    const last = samples.at(-1)!;
    await testInfo.attach(`${id}-performance`, {body: JSON.stringify({samples, frames, errors, shots}, null, 2), contentType: 'application/json'});
    console.log(`${id}: tick ${last.tick}, wall ${last.wallMs} ms, frame p95 ${frames.p95.toFixed(1)} ms, max ${frames.max.toFixed(1)} ms`);
    expect(errors, `${id} worker errors`).toEqual([]);
    expect(shots, `${id} combat activity`).toBeGreaterThan(0);
    expect(last.tick, `${id} worker progression`).toBeGreaterThan(last.wallMs / 50 * 0.8);
    expect(frames.p95, `${id} p95 frame gap`).toBeLessThan(100);
    expect(frames.max, `${id} maximum frame gap`).toBeLessThan(750);
    await page.getByTestId('reset-battle').click();
  }
});

test('real Workers apply terrain movement and sight and reject malformed terrain', async ({page}) => {
  test.setTimeout(30_000);
  await openSkirmish(page);
  const observations = await page.evaluate(async () => {
    const original = window.observedWorkers.at(-1)!;
    const ready = original.received.find(m => m.type === 'ready');
    if (!ready || ready.type !== 'ready') throw new Error('Missing catalog');
    const catalog = ready.catalog;
    const makeMap = (surface: string) => ({width: 32, height: 20, surfaces: Array(640).fill(surface), elevations: Array(693).fill(0), obstacles: [] as {x:number; y:number; width:number; height:number; heightMetres:number}[], patrol: [{x: 8.5, y: 5.5}]});
    const makeScenario = (map: ReturnType<typeof makeMap>, near: boolean) => ({map, seed: 1, factions: [0, 1], timeLimit: 180, units: [0, 1].map(faction => ({id: faction + 1, faction, definitionId: catalog.unit.id, position: {x: faction ? (near ? 7.5 : 28.5) : 3.5, y: faction && !near ? 18.5 : 5.5}, doctrine: {...catalog.defaultDoctrine}}))});
    const road = makeMap('road'), mud = makeMap('mud'), clear = makeMap('open'), wall = makeMap('open'), ridge = makeMap('open');
    wall.obstacles.push({x: 5, y: 5, width: 1, height: 1, heightMetres: catalog.fullObstacleHeight});
    for (let y = 0; y <= 20; y++) ridge.elevations[y * 33 + 5] = 4;
    const scenarios = [makeScenario(road, false), makeScenario(mud, false), makeScenario(clear, true), makeScenario(wall, true), makeScenario(ridge, true)];
    const workers = scenarios.map(() => new Worker(original.url, {type: 'module'}));
    try {
      const run = (worker: Worker, scenario: unknown, index: number) => new Promise<{x: number; target: number | null}>((resolve, reject) => {
        const timeout = setTimeout(() => reject(new Error(`Terrain worker ${index} timed out`)), 7000);
        worker.addEventListener('message', event => {
          const message = event.data;
          if (message.type === 'ready') worker.postMessage({type:'start', battleId: 1, scenario});
          else if (message.type === 'error') {clearTimeout(timeout); reject(new Error(message.message));}
          else if (message.type === 'snapshot' && message.snapshot.tick >= (index < 2 ? 30 : 2)) {
            clearTimeout(timeout); resolve({x: message.snapshot.units[0].position.x, target: message.snapshot.units[0].targetId});
          }
        });
        worker.postMessage({type:'initialize'});
      });
      const values = await Promise.all(workers.map((worker, index) => run(worker, scenarios[index], index)));
      const invalid = makeScenario(makeMap('open'), false);
      invalid.map.elevations.pop();
      const badSurface = makeScenario(makeMap('unknown'), false);
      const errors = await Promise.all([invalid, badSurface].map((scenario, index) => new Promise<string>((resolve, reject) => {
        const requestId = 900 + index;
        const timeout = setTimeout(() => reject(new Error('Validation timeout')), 5000);
        const handler = (event: MessageEvent) => {
          if (event.data.requestId !== requestId) return;
          clearTimeout(timeout); original.worker.removeEventListener('message', handler);
          resolve(event.data.type === 'validation' ? String(event.data.validation.valid) : event.data.message);
        };
        original.worker.addEventListener('message', handler);
        original.worker.postMessage({type:'validate', requestId, scenario, forStart: true});
      })));
      return {values, errors};
    } finally { workers.forEach(worker => worker.terminate()); }
  });
  expect(observations.values[0].x).toBeGreaterThan(observations.values[1].x + 0.2);
  expect(observations.values[2].target).toBe(2);
  expect(observations.values[3].target).toBeNull();
  expect(observations.values[4].target).toBeNull();
  expect(observations.errors[0]).toBe('false');
  expect(observations.errors[1]).toMatch(/unknown variant/i);
});


test('protected routes sustain sixteen units on each preset', async ({page}, testInfo) => {
  test.setTimeout(90_000);
  await openSkirmish(page);
  const measurements = await page.evaluate(async () => {
    const {worker, received} = window.observedWorkers.at(-1)!;
    const ready = received.find(message => message.type === 'ready');
    if (!ready || ready.type !== 'ready') throw new Error('Missing catalog');
    const catalog = ready.catalog;
    const measurements: {id: string; wallMs: number; tick: number; finished: boolean; errors: string[]}[] = [];
    for (const [index, preset] of catalog.maps.entries()) {
      const battleId = 100 + 2 * index;
      const scenario = {
        map: preset.map, seed: 1, factions: [0, 1], timeLimit: 180,
        units: Array.from({length: 16}, (_, i) => ({
          id: i + 1, faction: i < 8 ? 0 : 1, definitionId: catalog.unit.id,
          position: {x: i < 8 ? 9.5 : 22.5, y: 2.5 + (i % 8) * 2},
          doctrine: {...catalog.defaultDoctrine, routePreference: 'protected' as const},
        })),
      };
      const started = performance.now();
      worker.postMessage({type: 'start', battleId, scenario});
      await new Promise<void>(resolve => {
        const timer = setTimeout(done, 8000);
        function done() { clearTimeout(timer); worker.removeEventListener('message', onMessage); resolve(); }
        function onMessage(event: MessageEvent) {
          if (event.data.battleId === battleId && (event.data.type === 'result' || event.data.type === 'error')) done();
        }
        worker.addEventListener('message', onMessage);
      });
      const wallMs = performance.now() - started;
      const latest = received.findLast(message => message.type === 'snapshot' && message.battleId === battleId);
      const errors = received.filter(message => message.type === 'error' && message.battleId === battleId).map(message => message.type === 'error' ? message.message : '');
      measurements.push({id: preset.id, wallMs, tick: latest?.type === 'snapshot' ? latest.snapshot.tick : 0, finished: received.some(message => message.type === 'result' && message.battleId === battleId), errors});
      worker.postMessage({type: 'reset', battleId: battleId + 1});
    }
    return measurements;
  });
  await testInfo.attach('protected-route-performance', {body: JSON.stringify(measurements, null, 2), contentType: 'application/json'});
  for (const result of measurements) {
    console.log(`${result.id} protected: tick ${result.tick}, wall ${result.wallMs.toFixed(0)} ms, finished ${result.finished}`);
    expect(result.errors, `${result.id} protected worker errors`).toEqual([]);
    if (!result.finished) expect(result.tick, `${result.id} protected worker progression`).toBeGreaterThan(result.wallMs / 50 * 0.8);
  }
});


test('preset layouts fit wide and narrow viewports', async ({page}, testInfo) => {
  test.setTimeout(90_000);
  await page.setViewportSize({width: 1440, height: 900});
  await openSkirmish(page);
  for (const id of ['training', 'ridge', 'woodland']) {
    await page.getByTestId('map-preset').selectOption(id);
    await place(page, 0, 2.5, 2.5);
    for (const width of [1440, 900]) {
      await page.setViewportSize({width, height: 900});
      await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
      await page.screenshot({path: testInfo.outputPath(`${id}-${width}.png`), fullPage: true});
    }
  }
});
