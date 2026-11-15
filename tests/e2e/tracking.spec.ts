import { test, expect } from '@playwright/test';
import { fixtureSnapshot } from '../../src/bridge/fixture';
test('tracking controls send explicit choices and keep native lyrics honest', async ({ page }) => {
  const initial=fixtureSnapshot();
  initial.capabilities.mode='native';initial.capabilities.trackingAvailable=true;
  const id=initial.sessions[0].playerId;
  initial.players=[{id,name:'Synthetic player',enabled:false,available:true}];
  initial.sessions=[];initial.selectedSessionId=null;
  await page.route('**/src/bridge/fixture.ts', route=>route.fulfill({contentType:'text/javascript',body:`
    export function createFixture() {
      let state=${JSON.stringify(initial)};const listeners=new Set();
      return {snapshot:async()=>structuredClone(state),subscribe:async fn=>{listeners.add(fn);return()=>listeners.delete(fn)},setScenario:()=>{},
        action:async action=>{
          window.fixtureActions ??=[];window.fixtureActions.push(action);
          if(action.kind==='enable_player') state.players[0].enabled=action.enabled;
          if(action.kind==='tracking') state.trackingEnabled=action.enabled;
          if(action.kind==='select') {state.selection=action.mode;state.statusMessage=action.mode.kind==='automatic'?'':'Waiting for pinned player';}
          state.revision=String(BigInt(state.revision)+1n);for(const fn of listeners)fn(structuredClone(state));
        }};
    }`}));
  await page.goto('http://127.0.0.1:1420');
  await page.getByRole('button',{name:'Settings',exact:true}).click();
  const player=page.getByRole('checkbox',{name:/Synthetic player/});
  await expect(player).not.toBeChecked();await player.check();await expect(player).toBeChecked();
  await page.getByRole('button',{name:'Start tracking',exact:true}).click();
  await expect(page.getByRole('button',{name:'Stop tracking',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Now',exact:true}).click();
  await page.locator('.session-picker summary').click();
  await page.getByRole('button',{name:'Pin Synthetic player app'}).click();
  await expect(page.getByRole('status')).toHaveText('Waiting for pinned player');
  await page.getByRole('button',{name:'Return to automatic'}).click();
  await expect(page.getByRole('button',{name:'Return to automatic'})).toBeDisabled();
  await expect(page.getByRole('region',{name:'Synthetic lyrics'})).toHaveCount(0);
  await expect(page.getByText('We follow as the clouds go by')).toHaveCount(0);
});
