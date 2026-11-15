import AxeBuilder from '@axe-core/playwright';
import {expect,test,type Page} from '@playwright/test';
import {fixtureSnapshot} from '../../src/bridge/fixture';

async function designFixture(page:Page,mode='compact') {
 const state=fixtureSnapshot();state.capabilities.mode='native';state.trackingEnabled=true;
 state.overlayMode=mode as typeof state.overlayMode;
 state.reader={...state.reader,status:'ready',activeLine:1,positionAvailable:true,cached:true,
  version:{id:'design-version',recordingId:'fixture-recording',contentHash:'synthetic',origin:'provider',sourceName:'Synthetic provider',sourceUrl:null,confidence:'uncertain',timing:'supplied',providerId:'fixture',attribution:'Original synthetic lyrics for automated checks',retention:'cached',providerDurationMs:'120000',lines:['A little light across the sky','We follow as the clouds go by','One quiet note, another day','The words will find their way'].map((text,i)=>({id:String(i),text,startMs:String(i*2000),endMs:null,words:[]}))},
  versions:[{id:'design-version',source:'Synthetic provider',confidence:'uncertain',timing:'supplied',origin:'provider'}]};
 await page.route('**/src/bridge/fixture.ts',route=>route.fulfill({contentType:'text/javascript',body:`
 export function createFixture(){const state=${JSON.stringify(state)};const listeners=new Set();
 const publish=()=>{state.revision=String(BigInt(state.revision)+1n);for(const fn of listeners)fn(structuredClone(state));};
 return {snapshot:async()=>structuredClone(state),subscribe:async fn=>{listeners.add(fn);return()=>listeners.delete(fn)},action:async()=>{},
 product:async command=>{window.designCommands??=[];window.designCommands.push(command);
 if(command.kind==='appearance'){state.appearance=command.value;publish();}
 if(command.kind==='reset_appearance'){state.appearance=${JSON.stringify(fixtureSnapshot().appearance)};publish();}
 if(command.kind==='library')return {kind:'library',items:[],more:false};
 if(command.kind==='backup_poll')return {kind:'backup',state:{stage:'idle',token:null,message:'',summary:[]}};
 return {kind:'ok'};}};}` }));
}
async function accessible(page:Page){const result=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();expect(result.violations).toEqual([]);}
async function fits(page:Page){expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);}

test('desktop reading, settings drafts and keyboard focus remain usable',async({page})=>{
 const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));
 await designFixture(page);await page.goto('http://127.0.0.1:1420');
 await expect(page.getByRole('heading',{name:'A little light'})).toBeVisible();
 await page.keyboard.press('Tab');await expect(page.getByRole('link',{name:'Skip to content'})).toBeFocused();
 await page.keyboard.press('Enter');await expect(page.locator('#main-content')).toBeFocused();
 await fits(page);await accessible(page);
 const active=await page.locator('[aria-current=true]').boundingBox();expect(active!.y+active!.height).toBeLessThan(760);
 await page.getByRole('heading',{name:'Now playing'}).click();await page.screenshot({path:'artifacts/design-now.png',fullPage:true});
 await page.getByRole('button',{name:'Settings',exact:true}).click();await accessible(page);
 await page.getByRole('button',{name:'Appearance',exact:true}).click();
 const size=page.getByRole('spinbutton',{name:'Text size (main)',exact:true});
 await size.fill('');await size.fill('32');
 await expect(size).toHaveValue('32');
 expect(await page.evaluate(()=>((window as unknown as {designCommands?:{kind:string}[]}).designCommands??[]).filter(c=>c.kind==='appearance').length)).toBe(0);
 await page.getByRole('button',{name:'Save appearance',exact:true}).click();
 await expect(page.getByRole('status')).toHaveText('Reading appearance saved.');
 await page.getByRole('button',{name:'Reset reading appearance',exact:true}).click();
 await expect(size).toHaveValue('24');
 await size.fill('35');await page.getByRole('button',{name:'Reset reading appearance',exact:true}).click();await expect(size).toHaveValue('24');
 await accessible(page);await page.screenshot({path:'artifacts/design-settings.png',fullPage:true});
 await page.setViewportSize({width:620,height:480});await fits(page);
 await page.getByRole('button',{name:'Backups',exact:true}).click();await fits(page);await accessible(page);
 await page.screenshot({path:'artifacts/design-backups-narrow.png',fullPage:true});
 await page.getByRole('button',{name:'Library data',exact:true}).click();
 await page.getByRole('button',{name:'Clear downloaded lyrics',exact:true}).click();await accessible(page);
 await page.getByRole('button',{name:'Cancel',exact:true}).click();
 await page.getByRole('button',{name:'About',exact:true}).click();await accessible(page);
 await page.getByRole('button',{name:'Library',exact:true}).click();
 await page.getByRole('button',{name:'Find a song',exact:true}).click();await fits(page);
 await expect(page.getByLabel('Title',{exact:true})).toBeVisible();
 await accessible(page);await page.screenshot({path:'artifacts/design-search-narrow.png',fullPage:true});
 expect(errors).toEqual([]);
});
for(const mode of ['compact','expanded','minimal']) test(`floating ${mode} layout keeps named controls reachable`,async({page})=>{
 await page.setViewportSize(mode==='minimal'?{width:320,height:110}:mode==='compact'?{width:440,height:360}:{width:560,height:720});
 await designFixture(page,mode);await page.goto('http://127.0.0.1:1420/?overlay');
 await expect(page.getByRole('button',{name:'Hide',exact:true})).toBeVisible();await fits(page);
 if(mode!=='minimal')await expect(page.getByRole('button',{name:'Dock to left edge'})).toBeVisible();
 await page.getByRole('combobox',{name:'Reader mode'}).focus();
 expect(await page.getByRole('combobox',{name:'Reader mode'}).evaluate(e=>getComputedStyle(e).outlineStyle)).toBe('solid');
 await accessible(page);
 if(mode!=='minimal'){
  const active=page.locator('[aria-current=true]');const viewport=page.getByLabel('Lyric text',{exact:true});
  await expect.poll(async()=>{const row=await active.boundingBox();const box=await viewport.boundingBox();return !!row&&!!box&&row.y>=box.y-1&&row.y+row.height<=box.y+box.height+1;}).toBe(true);
 }
 await page.screenshot({path:`artifacts/design-floating-${mode}.png`,fullPage:true});
});
