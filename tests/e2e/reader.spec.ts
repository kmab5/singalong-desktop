import { test, expect } from '@playwright/test';
import { fixtureSnapshot } from '../../src/bridge/fixture';

test('shared reader labels possible matches, follows supplied lines and supports manual scrolling', async ({ page }) => {
  const state=fixtureSnapshot(); state.capabilities.mode='native';state.trackingEnabled=true;
  state.reader={...state.reader,status:'ready',activeLine:0,countdownSeconds:null,positionAvailable:true,cached:true,message:'',retryAfterMs:null,version:{id:'original-version',recordingId:'fixture-recording',contentHash:'fixture-hash',origin:'provider',sourceName:'Fixture provider',sourceUrl:null,confidence:'uncertain',timing:'supplied',providerId:'fixture',attribution:'Original synthetic fixture',retention:'cached',providerDurationMs:'120000',lines:[{id:'0',text:'Original first line',startMs:'1000',endMs:null,words:[]},{id:'1',text:'Original second line',startMs:'2000',endMs:null,words:[]}]}};
  await page.route('**/src/bridge/fixture.ts',route=>route.fulfill({contentType:'text/javascript',body:`
    export function createFixture(){let state=${JSON.stringify(state)};const listeners=new Set();
      window.readerUpdate=patch=>{Object.assign(state.reader,patch);state.revision=String(BigInt(state.revision)+1n);for(const fn of listeners)fn(structuredClone(state));};
      return {snapshot:async()=>structuredClone(state),subscribe:async fn=>{listeners.add(fn);return()=>listeners.delete(fn)},setScenario:()=>{},action:async()=>{}};}
  `}));
  await page.goto('http://127.0.0.1:1420');
  const reader=page.getByRole('region',{name:'Lyrics',exact:true});
  await expect(reader.getByText('Possible match',{exact:true})).toBeVisible();
  await expect(reader.getByText(/Supplied timing.*Saved offline/)).toBeVisible();
  await expect(reader.locator('[aria-current=true]')).toHaveText('Original first line');
  await reader.getByLabel('Lyric text',{exact:true}).press('PageDown');
  await expect(reader.getByRole('button',{name:'Resume following'})).toBeVisible();
  await page.evaluate(()=> (window as unknown as {readerUpdate:(value:unknown)=>void}).readerUpdate({activeLine:1}));
  await expect(reader.locator('[aria-current=true]')).toHaveText('Original second line');
  await reader.getByRole('button',{name:'Resume following'}).click();
  await expect(reader.getByText('Following playback',{exact:true})).toBeVisible();
  await page.evaluate(()=> (window as unknown as {readerUpdate:(value:unknown)=>void}).readerUpdate({activeLine:null,positionAvailable:false}));
  await expect(reader.getByText('Playback position unavailable · read at your own pace')).toBeVisible();
});
