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
      map: { width: 32, height: 20, walls: [], cover: [], patrol: [] },
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
