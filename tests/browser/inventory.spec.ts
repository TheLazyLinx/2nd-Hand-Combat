import { test, expect } from '@playwright/test';
import { latestSnapshot, observeWorkers, openSkirmish, place } from './helpers';

test.beforeEach(async ({page}) => observeWorkers(page));

test('real Worker inventory consumes ammunition, freezes on pause and restores on reset', async ({page}, info) => {
  await openSkirmish(page);
  await place(page, 0, 3.5, 2.5);
  await expect(page.getByTestId('inventory-load')).toContainText('9.52 / 20.00 kg');
  await expect(page.getByTestId('inventory-load')).toContainText('6.50 / 15.00 L');
  await expect(page.getByTestId('inventory-hands')).toContainText('Left hand: Rifle');
  await expect(page.getByTestId('inventory-hands')).toContainText('Right hand: Rifle');
  await place(page, 1, 7.5, 2.5);
  await page.getByTestId('start-battle').click();
  await expect.poll(async () => (await latestSnapshot(page))?.units[0].shotsFired).toBeGreaterThan(0);
  await page.getByTestId('pause-battle').click();
  await expect(page.getByTestId('battle-status')).toHaveText('PAUSED');
  await page.getByTestId('faction-0').getByRole('button', {name: /Infantry #1/}).click();
  const paused = await latestSnapshot(page);
  const unit = paused!.units[0];
  expect(unit.inventory.entries.find(entry => entry.itemId === 'rifle-ammo')?.quantity).toBe(60 - unit.shotsFired);
  expect(unit.inventory.weightGrams).toBe(9520 - 12 * unit.shotsFired);
  expect(unit.inventory.stowedVolumeMl).toBe(6500 - 15 * unit.shotsFired);
  expect(unit.inventory.leftHand).toBe(unit.inventory.rightHand);
  expect(unit.inventory.entries.find(entry => entry.id === unit.inventory.leftHand)).toMatchObject({itemId: 'rifle', quantity: 1});
  expect(unit.sustainment.nutrition).toBeCloseTo(100 - paused!.elapsed * 0.5, 6);
  expect(unit.sustainment.hydration).toBeCloseTo(100 - paused!.elapsed, 6);
  await expect(page.getByTestId('inventory-items')).toContainText(`Rifle rounds: ${60 - unit.shotsFired}`);
  await expect(page.getByTestId('inventory-needs')).toContainText('Combined mobility');
  for (const width of [1440, 900]) {
    await page.setViewportSize({width, height: 1000});
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({path: info.outputPath(`inventory-${width}.png`), fullPage: true});
  }
  await page.waitForTimeout(350);
  expect(await latestSnapshot(page)).toEqual(paused);
  await page.getByTestId('reset-battle').click();
  await page.getByTestId('faction-0').getByRole('button', {name: /Infantry #1/}).click();
  await expect(page.getByTestId('inventory-load')).toContainText('9.52 / 20.00 kg');
  await expect(page.getByTestId('inventory-items')).toContainText('Rifle rounds: 60');
  await page.getByTestId('start-battle').click();
  await expect.poll(() => page.evaluate(() => window.observedWorkers.at(-1)!.received.some(message => message.type === 'snapshot' && message.battleId === 3 && message.snapshot.tick === 0))).toBe(true);
  const reset = await page.evaluate(() => {
    const message = window.observedWorkers.at(-1)!.received.find(message => message.type === 'snapshot' && message.battleId === 3 && message.snapshot.tick === 0);
    return message?.type === 'snapshot' ? message.snapshot.units[0] : null;
  });
  expect(reset!.inventory).toMatchObject({weightGrams: 9520, stowedVolumeMl: 6500});
  expect(reset!.inventory.uses.every(use => use.count === 0)).toBe(true);
  expect(reset!.sustainment).toMatchObject({nutrition: 100, hydration: 100, needsFactor: 1});
});

test('grenade doctrine and cluster threshold change the first real Worker attack', async ({page}) => {
  await openSkirmish(page);
  await place(page, 0, 3.5, 2.5);
  await place(page, 1, 7.5, 2.5);
  await place(page, 1, 7.5, 3.5);
  const selectFirst = () => page.getByTestId('faction-0').getByRole('button', {name: /Infantry #1/}).click();
  await selectFirst();
  await page.getByTestId('selected-unit').getByText('Grenades', {exact: true}).click();
  await page.getByTestId('grenade-policy').selectOption('never');

  async function firstAttack(battleId: number) {
    await page.getByTestId('start-battle').click();
    await expect.poll(() => page.evaluate(id => window.observedWorkers.at(-1)!.received.some(message => message.type === 'snapshot' && message.battleId === id && message.snapshot.tick > 0), battleId)).toBe(true);
    await page.getByTestId('pause-battle').click();
    await expect(page.getByTestId('battle-status')).toHaveText('PAUSED');
    return page.evaluate(id => {
      const message = window.observedWorkers.at(-1)!.received.find(message => message.type === 'snapshot' && message.battleId === id && message.snapshot.tick > 0);
      return message?.type === 'snapshot' ? message.snapshot.units[0] : null;
    }, battleId);
  }
  const never = await firstAttack(1);
  expect(never!.inventory.entries.find(entry => entry.itemId === 'frag-grenade')?.quantity).toBe(2);
  expect(never!.shotsFired).toBe(1);
  await page.getByTestId('reset-battle').click();
  await selectFirst();
  await page.getByTestId('selected-unit').getByText('Grenades', {exact: true}).click();
  await page.getByTestId('grenade-policy').selectOption('clusters');
  await page.getByTestId('grenade-enemies').fill('3');
  await page.getByTestId('grenade-distance').focus();
  const three = await firstAttack(3);
  expect(three!.inventory.entries.find(entry => entry.itemId === 'frag-grenade')?.quantity).toBe(2);
  expect(three!.shotsFired).toBe(1);
  await page.getByTestId('reset-battle').click();
  await selectFirst();
  await page.getByTestId('selected-unit').getByText('Grenades', {exact: true}).click();
  await page.getByTestId('grenade-enemies').fill('2');
  await page.getByTestId('grenade-distance').focus();
  const two = await firstAttack(5);
  expect(two!.inventory.entries.find(entry => entry.itemId === 'frag-grenade')?.quantity).toBe(1);
  expect(two!.shotsFired).toBe(0);
  expect(two!.inventory.leftHand).toBe(two!.inventory.rightHand);
});
