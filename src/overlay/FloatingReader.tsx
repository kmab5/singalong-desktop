import { useEffect, useState, useCallback } from 'react';
import { connect, type Transport } from '../bridge/transport';
import type { RuntimeSnapshot, ProductCommand, OverlayMode } from '../bridge/types';
import { Reader } from '../features/now/Reader';

export function FloatingReader({ transport }: { transport: Transport }) {
  const [state, setState] = useState<RuntimeSnapshot>();
  const [error, setError] = useState('');
  useEffect(() => {
    const controller = new AbortController();
    void connect(transport, setState, error => setError(error.message), controller.signal).catch(() => setError('Reader connection unavailable'));
    return () => controller.abort();
  }, [transport]);
  const product=useCallback((command:ProductCommand)=>transport.product?transport.product(command):Promise.reject(new Error('Controls unavailable')),[transport]);
  const act=(command:ProductCommand)=>{void product(command).catch(()=>setError('The floating reader action could not finish.'));};
  const session = state?.sessions.find(s => s.sessionId === state.selectedSessionId);
  if(state?.overlayDocked)return <main className="edge-handle"><button onClick={()=>act({kind:'overlay',mode:'compact'})} aria-label="Expand floating lyrics">♫<span>Expand</span></button>{error&&<span role="alert">{error}</span>}</main>;
  const minimal=state?.overlayCollapsed||state?.overlayMode==='minimal';
  return <main className={`floating-shell ${minimal?'minimal':state?.overlayMode??'compact'} ${state?.appearance.reducedMotion?'reduced-motion':''}`} style={{backgroundColor:`rgba(11,12,18,${state?.appearance.opacity??0.95})`}}>
    {state&&<div className="floating-toolbar"><label className="sr-only" htmlFor="reader-mode">Reader mode</label><select id="reader-mode" value={minimal?'minimal':state.overlayMode} onChange={e=>act({kind:'overlay',mode:e.target.value as OverlayMode})}><option value="compact">Compact</option><option value="expanded">Expanded</option><option value="minimal">Minimal</option></select>{minimal?<button onClick={()=>act({kind:'overlay',mode:'compact'})}>Expand</button>:<><button title="Dock to left edge" onClick={()=>act({kind:'dock',side:'left'})}>⇤</button><button title="Dock to right edge" onClick={()=>act({kind:'dock',side:'right'})}>⇥</button><button aria-pressed={state.clickThrough} onClick={()=>act({kind:'click_through',enabled:!state.clickThrough})}>Click-through</button></>}<button onClick={()=>act({kind:'hide_overlay'})}>Hide</button></div>}

    <header><p className="eyebrow">SINGALONG</p><h1>{session?.recording?.title || 'Waiting for music'}</h1><p>{session?.recording?.artists.join(', ') || state?.statusMessage}</p></header>
    {error ? <p role="alert">{error}</p> : state && (minimal?<><p className="minimal-line">{state.reader.activeLine!=null?state.reader.version?.lines[state.reader.activeLine]?.text:state.reader.version?'Open the reader to read at your own pace':state.reader.message}</p><small className="minimal-status">{state.reader.version?.confidence==='uncertain'?'Possible match · ':''}{state.reader.estimated&&state.reader.estimatedAvailable?'Estimated timing':state.reader.version?.timing==='supplied'?'Supplied timing':'Unsynced'}{!state.reader.positionAvailable?' · Position unavailable':''}</small></>:<Reader state={state.reader} appearance={state.appearance.floating} product={product}/>)}
  </main>;
}
