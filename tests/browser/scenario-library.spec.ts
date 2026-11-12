import { test, expect, type Page } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { observeWorkers } from './helpers';

function fixture(name = 'Library round trip') {
  return {
    format: '2nd-hand-combat-scenario', schemaVersion: 1, name, description: 'Portable custom setup',
    scenario: {
      seed: 42, timeLimit: 90, factions: [0, 1],
      map: { width: 8, height: 8, surfaces: Array(64).fill('open'), elevations: Array(81).fill(0), obstacles: [{x: 3, y: 3, width: 2, height: 1, heightMetres: 0.9}], patrol: [{x: 2.5, y: 2.5}, {x: 5.5, y: 5.5}] },
      units: [0, 1].map(faction => ({id: faction + 1, faction, definitionId: 'infantry', position: {x: faction ? 6.5 : 1.5, y: faction ? 6.5 : 1.5}, doctrine: {engagementDistance: 6, fallbackHp: 35, positionPreference: 'cover', routePreference: 'protected'}})),
    },
  };
}
async function openMaker(page: Page) {
  await page.goto('/');
  await page.getByRole('button', {name: /02 Map\s?Maker/i}).click();
  await expect(page.getByLabel('Name', {exact: true})).toBeVisible();
  await expect(page.getByText(/Draft can be saved|Ready to test/)).toBeVisible();
}
async function importFile(page: Page, value: unknown) {
  await page.locator('input[type=file]').setInputFiles({name: 'scenario.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(value))});
  await expect(async () => {
    const guard = page.getByRole('dialog', {name: 'Unsaved changes'});
    if (await guard.isVisible()) await guard.getByRole('button', {name: 'Discard', exact: true}).click();
    await expect(page.getByLabel('Name', {exact: true})).toHaveValue((value as {name: string}).name);
  }).toPass({timeout: 10_000});
  await expect(page.getByText(/Ready to test|Draft can be saved/)).toBeVisible();
}
async function exported(page: Page, path: string) {
  const downloaded = page.waitForEvent('download');
  await page.getByRole('button', {name: 'Export', exact: true}).click();
  await (await downloaded).saveAs(path);
  return JSON.parse(await readFile(path, 'utf8'));
}

test.beforeEach(async ({page}) => observeWorkers(page));

test('portable scenarios preserve their authored fields and survive a browser reload', async ({page}, info) => {
  await openMaker(page);
  const document = fixture();
  await importFile(page, document);
  await page.getByRole('button', {name: 'Save', exact: true}).first().click();
  await expect(page.getByText('Unsaved changes', {exact: true})).toHaveCount(0);
  const downloadEvent = page.waitForEvent('download');
  await page.getByRole('button', {name: 'Export', exact: true}).click();
  const download = await downloadEvent;
  const path = info.outputPath('scenario.json');
  await download.saveAs(path);
  expect(JSON.parse(await readFile(path, 'utf8'))).toEqual(document);
  await page.reload();
  await page.getByRole('button', {name: /02 Map\s?Maker/i}).click();
  const entry = page.locator('.library-item').filter({hasText: document.name});
  await entry.getByRole('button', {name: 'Open', exact: true}).click();
  const guard = page.getByRole('dialog', {name: 'Unsaved changes'});
  if (await guard.isVisible()) await guard.getByRole('button', {name: 'Discard', exact: true}).click();
  await expect(page.getByLabel('Seed', {exact: true})).toHaveValue('42');
  await expect(page.getByLabel('Time limit', {exact: true})).toHaveValue('90');
  await expect(page.getByText('Ready to test', {exact: true})).toBeVisible();
});

test('unsupported and malformed imports leave the current draft intact', async ({page}) => {
  await openMaker(page);
  const originalName = await page.getByLabel('Name', {exact: true}).inputValue();
  await page.locator('input[type=file]').setInputFiles({name: 'future.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify({...fixture(), schemaVersion: 9}))});
  await expect(page.getByRole('alert')).toContainText(/version.*not supported/i);
  await expect(page.getByLabel('Name', {exact: true})).toHaveValue(originalName);
  await page.locator('input[type=file]').setInputFiles({name: 'broken.json', mimeType: 'application/json', buffer: Buffer.from('{ broken')});
  await expect(page.getByRole('alert')).toContainText(/not valid.*JSON/i);
  await expect(page.getByLabel('Name', {exact: true})).toHaveValue(originalName);
  const malformed=fixture(); malformed.scenario.map.elevations=[];
  await page.locator('input[type=file]').setInputFiles({name:'bad-terrain.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(malformed))});
  await expect(page.getByRole('alert')).toContainText(/Terrain arrays/);
  await expect(page.getByLabel('Name', {exact:true})).toHaveValue(originalName);
});

test('storage transaction failure keeps the unsaved draft exportable', async ({page}, info) => {
  await openMaker(page);
  await importFile(page, fixture('Export after storage failure'));
  await page.getByLabel('Description', {exact: true}).fill('Edits awaiting storage');
  await page.getByLabel('Name', {exact: true}).focus();
  await page.evaluate(() => {
    const original = IDBDatabase.prototype.transaction;
    IDBDatabase.prototype.transaction = function (...args: Parameters<IDBDatabase['transaction']>) {
      if (args[1] === 'readwrite') throw new DOMException('Simulated quota exceeded', 'QuotaExceededError');
      return original.apply(this, args);
    };
  });
  await page.getByRole('button', {name: 'Save', exact: true}).first().click();
  await expect(page.getByRole('alert')).toContainText(/quota/i);
  await expect(page.getByText('Unsaved changes', {exact: true})).toBeVisible();
  const downloaded = page.waitForEvent('download');
  await page.getByRole('button', {name: 'Export', exact: true}).click();
  const path = info.outputPath('recovered.json');
  await (await downloaded).saveAs(path);
  expect(JSON.parse(await readFile(path, 'utf8')).name).toBe('Export after storage failure');
});

test('shrinking clips authored objects and undo restores every scenario field', async ({page}, info) => {
  await openMaker(page);
  const original = fixture('Resize authored terrain');
  original.scenario.map.surfaces[9] = 'road';
  original.scenario.map.elevations[10] = -0.2;
  await importFile(page, original);
  await page.getByLabel('Width', {exact: true}).fill('4');
  await page.getByLabel('Height', {exact: true}).fill('4');
  await page.getByRole('button', {name: 'Preview resize'}).click();
  await expect(page.getByText(/1 soldiers and 1 patrol points removed; 1 obstacles/)).toBeVisible();
  await page.getByRole('button', {name: 'Apply resize'}).click();
  const resized = await exported(page, info.outputPath('resized.json'));
  expect(resized.scenario.map.surfaces).toHaveLength(16);
  expect(resized.scenario.map.surfaces[5]).toBe('road');
  expect(resized.scenario.map.elevations[6]).toBe(-0.2);
  expect(resized.scenario.map.obstacles).toEqual([{x:3,y:3,width:1,height:1,heightMetres:0.9}]);
  expect(resized.scenario.map.patrol).toEqual([{x:2.5,y:2.5}]);
  expect(resized.scenario.units).toEqual([original.scenario.units[0]]);
  await page.getByRole('button', {name:'Undo',exact:true}).click();
  expect(await exported(page, info.outputPath('restored.json'))).toEqual(original);
});

test('layout errors remain portable drafts and fixing the selected soldier enables play', async ({page}, info) => {
  await openMaker(page);
  const draft = fixture('Blocked soldier draft');
  draft.scenario.units[0].position = {x:3.5,y:3.5};
  await importFile(page, draft);
  await expect(page.getByRole('button',{name:'Test battle'})).toBeDisabled();
  expect(await exported(page, info.outputPath('draft.json'))).toEqual(draft);
  await page.getByRole('button',{name:'#1 North',exact:true}).click();
  await page.getByLabel('Selected X',{exact:true}).fill('1.5');
  await page.getByLabel('Selected Y',{exact:true}).fill('1.5');
  await page.getByLabel('Selected X',{exact:true}).focus();
  await expect(page.getByRole('button',{name:'Test battle'})).toBeEnabled();
});

test('surface and elevation tools update the real terrain analysis and undo as gestures', async ({page}, info) => {
  await openMaker(page);
  await page.getByRole('combobox',{name:/Surface/}).selectOption('road');
  await page.getByLabel('Edit column').fill('2');
  await page.getByLabel('Edit row').fill('2');
  await page.getByRole('button',{name:'Apply at coordinates'}).click();
  await page.getByRole('button',{name:'raise',exact:true}).click();
  await page.getByLabel('Elevation amount').fill('0.2');
  await page.getByRole('button',{name:'Apply at coordinates'}).click();
  let document = await exported(page,info.outputPath('painted.json'));
  expect(document.scenario.map.surfaces[2*32+2]).toBe('road');
  expect(document.scenario.map.elevations[3*33+3]).toBeCloseTo(0.2);
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  document=await exported(page,info.outputPath('undone.json'));
  expect(document.scenario.map.elevations.every((value:number)=>value===0)).toBe(true);
  expect(document.scenario.map.surfaces[2*32+2]).toBe('road');
});

test('a saved full scenario loads through Skirmish and reset restores its detached starting copy', async ({page}) => {
  await openMaker(page);
  const document=fixture('Skirmish library setup');
  await importFile(page,document);
  await page.getByRole('button',{name:'Main menu',exact:true}).click();
  await page.getByRole('button',{name:/01 Skirmish/}).click();
  await page.getByTestId('map-preset').selectOption({label:document.name});
  await expect(page.getByTestId('start-battle')).toBeEnabled();
  await page.getByTestId('start-battle').click();
  await expect(page.getByTestId('battle-status')).toHaveText('RUNNING');
  await page.getByTestId('pause-battle').click();
  await expect(page.getByTestId('battle-status')).toHaveText('PAUSED');
  await page.getByTestId('reset-battle').click();
  await expect(page.getByTestId('battle-status')).toHaveText('SETUP');
  await page.getByTestId('start-battle').click();
  const starts=await page.evaluate(()=>window.observedWorkers.at(-1)!.sent.filter(message=>message.type==='start'));
  expect(starts).toHaveLength(2);
  for(const start of starts) if(start.type==='start') expect(start.scenario).toEqual(document.scenario);
});

test('library duplication, rename and confirmed deletion affect only the chosen record', async ({page}) => {
  await openMaker(page);
  await importFile(page,fixture('Original library record'));
  const entries=page.locator('.library-item');
  await entries.filter({hasText:'Original library record'}).getByRole('button',{name:'Duplicate',exact:true}).click();
  await expect(entries).toHaveCount(2);
  const duplicate=entries.filter({hasText:'Original library record copy'});
  await duplicate.getByRole('button',{name:'Rename',exact:true}).click();
  const rename=page.getByRole('dialog',{name:'Rename scenario',exact:true});
  await rename.getByRole('textbox',{name:'Name',exact:true}).fill('Renamed copy');
  await rename.getByRole('button',{name:'Rename',exact:true}).click();
  const renamed=entries.filter({hasText:'Renamed copy'});
  await renamed.getByRole('button',{name:'Delete',exact:true}).click();
  const confirm=page.getByRole('dialog',{name:'Delete scenario',exact:true});
  await confirm.getByRole('button',{name:'Cancel',exact:true}).click();
  await expect(entries).toHaveCount(2);
  await renamed.getByRole('button',{name:'Delete',exact:true}).click();
  await confirm.getByRole('button',{name:'Delete',exact:true}).click();
  await expect(entries).toHaveCount(1);
  await expect(entries).toContainText('Original library record');
});

test('imported faction and unit IDs remain usable when authoring and deploying', async ({page}) => {
  await openMaker(page);
  const document=fixture('Stable imported IDs');
  document.scenario.factions=[10,20];
  document.scenario.units[0].faction=10; document.scenario.units[0].id=0xffffffff;
  document.scenario.units[1].faction=20;
  await importFile(page,document);
  await page.getByRole('button',{name:'soldier',exact:true}).click();
  await page.getByLabel('Edit column').fill('1');
  await page.getByLabel('Edit row').fill('5');
  await page.getByRole('button',{name:'Apply at coordinates'}).click();
  await expect(page.getByRole('button',{name:'#1 North',exact:true})).toBeVisible();
  await page.getByLabel('Selected faction',{exact:true}).selectOption('20');
  await expect(page.getByRole('button',{name:'#1 South',exact:true})).toBeVisible();
  await page.getByLabel('Selected faction',{exact:true}).selectOption('10');
  await page.getByRole('button',{name:'Test battle'}).click();
  await expect(page.getByTestId('faction-10')).toContainText('Infantry #1');
  await expect(page.getByTestId('faction-20')).toContainText('Infantry #2');
  await expect(page.getByTestId('start-battle')).toBeEnabled();
});

test('a delayed save cannot attach its library identity to a newly opened document', async ({page}) => {
  await openMaker(page);
  await page.getByLabel('Name',{exact:true}).fill('Delayed original');
  await page.getByLabel('Description',{exact:true}).focus();
  await expect(page.getByRole('button',{name:'Export',exact:true})).toBeEnabled();
  await page.evaluate(()=>{
    const original=IDBDatabase.prototype.transaction;
    IDBDatabase.prototype.transaction=function(...args:Parameters<IDBDatabase['transaction']>) {
      const tx=original.apply(this,args);
      if(args[1]==='readwrite') Object.defineProperty(tx,'oncomplete',{set(listener:(event:Event)=>void){
        tx.addEventListener('complete',event=>setTimeout(()=>listener.call(tx,event),1500));
      }});
      return tx;
    };
  });
  await page.getByRole('button',{name:'Save',exact:true}).first().click();
  await page.getByRole('button',{name:'New blank',exact:true}).click();
  await page.getByRole('dialog',{name:'Unsaved changes',exact:true}).getByRole('button',{name:'Discard',exact:true}).click();
  await page.getByLabel('Name',{exact:true}).fill('Independent new draft');
  await page.getByLabel('Description',{exact:true}).focus();
  await expect(page.locator('.library-item')).toHaveCount(1);
  await page.getByRole('button',{name:'Save',exact:true}).first().click();
  await expect(page.locator('.library-item')).toHaveCount(2);
  await expect(page.locator('.library-item').filter({hasText:'Delayed original'})).toBeVisible();
  await expect(page.locator('.library-item').filter({hasText:'Independent new draft'})).toBeVisible();
});

test('numeric obstacle properties author a rectangular solid footprint', async ({page},info)=>{
  await openMaker(page);
  await page.getByRole('button',{name:'obstacle',exact:true}).click();
  await page.getByLabel('Edit column').fill('4');
  await page.getByLabel('Edit row').fill('4');
  await page.getByRole('button',{name:'Apply at coordinates'}).click();
  await page.getByLabel('Obstacle width',{exact:true}).fill('3');
  await page.getByLabel('Obstacle depth',{exact:true}).fill('2');
  await page.getByLabel('Obstacle width',{exact:true}).focus();
  const document=await exported(page,info.outputPath('rectangular-obstacle.json'));
  expect(document.scenario.map.obstacles).toEqual([{x:4,y:4,width:3,height:2,heightMetres:0.9}]);
});

test('patrol points retain authored order and reordering is undoable',async({page},info)=>{
  await openMaker(page);
  await page.getByRole('button',{name:'patrol',exact:true}).click();
  for(const coordinate of [2,6]) {
    await page.getByLabel('Edit column').fill(String(coordinate));
    await page.getByLabel('Edit row').fill(String(coordinate));
    await page.getByRole('button',{name:'Apply at coordinates'}).click();
  }
  await page.getByRole('button',{name:'Move earlier',exact:true}).click();
  expect((await exported(page,info.outputPath('ordered-patrol.json'))).scenario.map.patrol).toEqual([{x:6.5,y:6.5},{x:2.5,y:2.5}]);
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  expect((await exported(page,info.outputPath('restored-patrol.json'))).scenario.map.patrol).toEqual([{x:2.5,y:2.5},{x:6.5,y:6.5}]);
});

test('a sparse pointer stroke paints continuously and undo removes the entire gesture',async({page},info)=>{
  await openMaker(page);
  await page.getByRole('combobox',{name:/Surface/}).selectOption('road');
  const canvas=page.locator('.maker canvas'),box=await canvas.boundingBox();
  if(!box) throw new Error('Missing editor canvas');
  const scale=Math.min(box.width/32,box.height/20);
  const origin={x:box.x+(box.width-scale*32)/2,y:box.y+(box.height-scale*20)/2};
  await page.mouse.move(origin.x+1.5*scale,origin.y+1.5*scale);
  await page.mouse.down();
  await page.mouse.move(origin.x+8.5*scale,origin.y+1.5*scale);
  await page.mouse.up();
  const document=await exported(page,info.outputPath('pointer-stroke.json'));
  expect(document.scenario.map.surfaces.slice(33,41)).toEqual(Array(8).fill('road'));
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  expect((await exported(page,info.outputPath('pointer-undone.json'))).scenario.map.surfaces.every((value:string)=>value==='open')).toBe(true);
});
