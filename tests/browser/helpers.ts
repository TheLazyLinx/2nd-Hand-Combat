import { expect, type Page } from '@playwright/test';
import type { Snapshot, WorkerCommand, WorkerReply } from '../../apps/web/src/lib/types';

declare global {
  interface Window {
    observedWorkers: { worker: Worker; url: string; received: WorkerReply[]; times: number[]; sent: WorkerCommand[]; terminated: boolean }[];
  }
}

/** Observe actual browser Workers without replacing their execution or messages. */
export async function observeWorkers(page: Page) {
  await page.addInitScript(() => {
    window.observedWorkers = [];
    const RealWorker = window.Worker;
    window.Worker = class extends RealWorker {
      record: Window['observedWorkers'][number];
      constructor(url: string | URL, options?: WorkerOptions) {
        super(url, options);
        this.record = { worker: this, url: String(url), received: [], times: [], sent: [], terminated: false };
        window.observedWorkers.push(this.record);
        this.addEventListener('message', event => { this.record.received.push(event.data); this.record.times.push(performance.now()); });
      }
      override postMessage(message: WorkerCommand) {
        this.record.sent.push(structuredClone(message));
        super.postMessage(message);
      }
      override terminate() { this.record.terminated = true; super.terminate(); }
    };
  });
}

export async function openSkirmish(page: Page) {
  await page.goto('/');
  await page.getByRole('button', { name: /01 Skirmish/ }).click();
  await expect(page.getByTestId('battle-status')).toHaveText('SETUP');
  await expect(page.getByTestId('battlefield').locator('canvas')).toBeVisible();
}

export async function clickMap(page: Page, x: number, y: number) {
  const canvas = page.getByTestId('battlefield').locator('canvas');
  const box = await canvas.boundingBox();
  if (!box) throw new Error('Battlefield canvas has no visible bounds.');
  const dimensions = await page.getByTestId('battlefield').evaluate(element => ({width: Number(element.dataset.mapWidth), height: Number(element.dataset.mapHeight)}));
  const scale = Math.min(box.width / dimensions.width, box.height / dimensions.height);
  await canvas.click({ position: { x: (box.width - dimensions.width * scale) / 2 + scale * x, y: (box.height - dimensions.height * scale) / 2 + scale * y } });
}

export async function place(page: Page, faction: number, x: number, y: number) {
  await page.getByTestId(`add-soldier-${faction}`).click();
  await page.getByTestId('choose-infantry').click();
  await clickMap(page, x, y);
  await expect(page.getByTestId('selected-unit')).toBeVisible();
}

export async function latestSnapshot(page: Page): Promise<Snapshot | null> {
  return page.evaluate(() => {
    const messages = window.observedWorkers.at(-1)?.received ?? [];
    const latest = messages.findLast(message => message.type === 'snapshot');
    return latest?.type === 'snapshot' ? latest.snapshot : null;
  });
}
