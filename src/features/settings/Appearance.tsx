import {useEffect, useState} from 'react';
import type {RuntimeSnapshot, ReaderAppearance, TextAlignment} from '../../bridge/types';
import {message, type Product} from '../library/Search';

export function Appearance({state,product}:{state:RuntimeSnapshot;product:Product}) {
 const [editing,setDraft]=useState<RuntimeSnapshot['appearance']|null>(null);
 const draft=editing??state.appearance;
 const persistedAppearance=JSON.stringify(state.appearance);
 useEffect(()=>setDraft(null),[persistedAppearance]);
 const [error,setError]=useState('');
 const [status,setStatus]=useState('');
 const [busy,setBusy]=useState(false);
 const change=(target:'main'|'floating',value:Partial<ReaderAppearance>)=>{
  setStatus('');setDraft(current=>{const base=current??state.appearance;return {...base,[target]:{...base[target],...value}};});
 };
 async function save(){
  setBusy(true);setError('');setStatus('');
  try{await product({kind:'appearance',value:draft});setStatus('Reading appearance saved.');}
  catch(e){setError(message(e));}finally{setBusy(false);}
 }
 async function reset(){
  setBusy(true);setError('');setStatus('');
  try{await product({kind:'reset_appearance'});setDraft(null);setStatus('Reading appearance reset.');}
  catch(e){setError(message(e));}finally{setBusy(false);}
 }
 return <section className="settings"><h2>Reading appearance</h2><p>Choose independent text styles for each reader, then save your changes.</p>
  {error&&<p role="alert">{error}</p>}{status&&<p role="status">{status}</p>}
  <form onSubmit={e=>{e.preventDefault();void save();}}><fieldset disabled={busy}>
   {(['main','floating'] as const).map(target=><div key={target}><h3>{target==='main'?'Main reader':'Floating reader'}</h3>
    <label>Text size ({target})<input name={`${target}-text-size`} type="number" required min={14} max={64} value={Number.isNaN(draft[target].textSize)?'':draft[target].textSize} onChange={e=>change(target,{textSize:e.target.valueAsNumber})}/></label>
    <label>Alignment ({target})<select name={`${target}-alignment`} value={draft[target].alignment} onChange={e=>change(target,{alignment:e.target.value as TextAlignment})}><option value="left">Left</option><option value="center">Center</option><option value="right">Right</option></select></label>
    <label><input type="checkbox" checked={draft[target].words} onChange={e=>change(target,{words:e.target.checked})}/>Emphasize supplied word timing ({target})</label>
   </div>)}
   <label>Floating background opacity · {Math.round(draft.opacity*100)}%<input name="opacity" type="range" min={0.2} max={1} step={0.05} value={draft.opacity} onChange={e=>setDraft({...draft,opacity:Number(e.target.value)})}/></label>
   <label><input type="checkbox" checked={draft.reducedMotion} onChange={e=>setDraft({...draft,reducedMotion:e.target.checked})}/>Reduce motion</label>
   <div className="control-row"><button type="submit">{busy?'Saving…':'Save appearance'}</button><button type="button" onClick={()=>void reset()}>Reset reading appearance</button></div>
  </fieldset></form>
  <h3>Window recovery</h3><p>Bring the floating reader back into view if it is off screen.</p><button disabled={busy} onClick={()=>{void product({kind:'reset_overlay'}).catch(e=>setError(message(e)));}}>Recover floating window</button>
 </section>;
}
