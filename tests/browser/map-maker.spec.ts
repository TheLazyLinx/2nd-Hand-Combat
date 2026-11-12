import { test, expect } from '@playwright/test';
import { observeWorkers } from './helpers';

test.beforeEach(async ({ page }) => {
  observeWorkers(page);
  await page.goto('/');
  await page.getByRole('button', { name: /02 Map\s?Maker/i }).click();
  await expect(page.getByText(/Draft can be saved|Ready to test/)).toBeVisible();
});

test('resize preserves the northwest corner and undo restores the map', async ({ page }) => {
  await page.getByLabel('Width', { exact: true }).fill('4');
  await page.getByLabel('Height', { exact: true }).fill('3');
  await page.getByRole('button', { name: 'Preview resize' }).click();
  await expect(page.getByText(/Northwest anchor/)).toBeVisible();
  await page.getByRole('button', { name: 'Apply resize' }).click();
  await expect(page.getByText('4 × 3', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(page.getByText('32 × 20', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Redo', exact: true }).click();
  await expect(page.getByText('4 × 3', { exact: true })).toBeVisible();
});

test('coordinate placements enable a test battle and return preserves the editor', async ({ page }) => {
  await page.getByRole('button', { name: 'soldier', exact: true }).click();
  await page.getByLabel('Edit column').fill('2');
  await page.getByLabel('Edit row').fill('2');
  await page.getByRole('button', { name: 'Apply at coordinates' }).click();
  await page.getByLabel('Faction', { exact: true }).selectOption('1');
  await page.getByLabel('Edit column').fill('28');
  await page.getByLabel('Edit row').fill('17');
  await page.getByRole('button', { name: 'Apply at coordinates' }).click();
  await expect(page.getByRole('button', { name: 'Test battle' })).toBeEnabled();
  await page.getByRole('button', { name: 'Test battle' }).click();
  await expect(page.getByTestId('battle-status')).toBeVisible();
  await page.getByRole('button', { name: /MapMaker/i }).click();
  await expect(page.getByRole('button', { name: '#1 North' })).toBeVisible();
  await expect(page.getByRole('button', { name: '#2 South' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Undo', exact: true })).toBeEnabled();
});

test('editor contours, legend and resize preview fit wide and narrow viewports', async ({page}, info) => {
  await page.getByRole('combobox',{name:/Copy built-in/}).selectOption('ridge');
  await page.getByRole('dialog',{name:'Unsaved changes',exact:true}).getByRole('button',{name:'Discard',exact:true}).click();
  await expect(page.getByLabel('Name',{exact:true})).toHaveValue('Ridge and Hollow');
  await expect(page.getByRole('button',{name:'Export',exact:true})).toBeEnabled();
  await expect(page.getByLabel('Terrain legend',{exact:true})).toContainText('Woodland');
  for (const width of [1440,900]) {
    await page.setViewportSize({width,height:1000});
    await expect.poll(()=>page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await page.screenshot({path:info.outputPath(`editor-ridge-${width}.png`),fullPage:true});
  }
  await page.getByLabel('Width',{exact:true}).fill('16');
  await page.getByRole('button',{name:'Preview resize',exact:true}).click();
  await page.screenshot({path:info.outputPath('resize-preview.png'),fullPage:true});
  await page.getByRole('button',{name:'Cancel resize',exact:true}).click();
  await page.getByRole('checkbox',{name:'Elevation overlay'}).uncheck();
  await expect(page.getByRole('checkbox',{name:'Elevation overlay'})).not.toBeChecked();
});

test('extreme valid terrain remains editable with bounded contour rendering', async ({page},info) => {
  const document=await page.evaluate(()=>{
    const ready=window.observedWorkers[0].received.find(message=>message.type==='ready');
    if(!ready || ready.type!=='ready') throw new Error('Catalog unavailable');
    const width=128,height=128;
    return {format:'2nd-hand-combat-scenario',schemaVersion:1,name:'Extreme cliffs',description:'',scenario:{seed:1,timeLimit:180,factions:[0,1],units:[],map:{width,height,surfaces:Array(width*height).fill('open'),elevations:Array.from({length:(width+1)*(height+1)},(_,i)=>(i%2 ? -100:100)),obstacles:[],patrol:[]}}};
  });
  await page.locator('input[type=file]').setInputFiles({name:'extreme.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(document))});
  await expect(async()=>{
    const guard=page.getByRole('dialog',{name:'Unsaved changes',exact:true});
    if(await guard.isVisible()) await guard.getByRole('button',{name:'Discard',exact:true}).click();
    await expect(page.getByLabel('Name',{exact:true})).toHaveValue('Extreme cliffs');
  }).toPass({timeout:15000});
  await expect(page.getByRole('button',{name:'Export',exact:true})).toBeEnabled();
  const gaps=await page.evaluate(()=>new Promise<number[]>(resolve=>{
    const gaps:number[]=[];let previous=0;const start=performance.now();
    function frame(now:number){if(previous) gaps.push(now-previous);previous=now;if(now-start<1500) requestAnimationFrame(frame);else resolve(gaps);}
    requestAnimationFrame(frame);
  }));
  gaps.sort((a,b)=>a-b);
  await info.attach('extreme-terrain-frames',{body:JSON.stringify({p95:gaps[Math.floor(gaps.length*.95)],max:Math.max(...gaps)}),contentType:'application/json'});
  expect(gaps[Math.floor(gaps.length*.95)]).toBeLessThan(100);
  expect(Math.max(...gaps)).toBeLessThan(750);
  await page.getByRole('button',{name:'Zoom in',exact:true}).click();
  await page.screenshot({path:info.outputPath('extreme-terrain.png')});
});
