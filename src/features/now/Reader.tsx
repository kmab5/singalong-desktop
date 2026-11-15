import { useEffect, useRef, useState, type CSSProperties } from 'react';
import type { ReaderState, ReaderAppearance } from '../../bridge/types';
import { message, type Product } from '../library/Search';

export function Reader({ state, retry, product, appearance }: { state: ReaderState; retry?: () => void; product?:Product; appearance?:ReaderAppearance }) {
  const [error,setError]=useState('');const [busy,setBusy]=useState(false);const [align,setAlign]=useState(false);
  async function act(command: Parameters<Product>[0]) {if(!product||busy)return;setBusy(true);setError('');try{await product(command);}catch(e){setError(message(e));}finally{setBusy(false);}}

  const [following, setFollowing] = useState(true);
  const lines = useRef<HTMLDivElement>(null);
  const version = state.version;
  useEffect(() => setFollowing(true), [version?.id]);
  useEffect(() => {
    if (following && state.activeLine != null && lines.current) {
      const container=lines.current;
      const line=container.querySelector<HTMLElement>(`[data-line="${state.activeLine}"]`);
      if (line) {
        const follow = () => {
          const row = line.getBoundingClientRect();
          const viewport = container.getBoundingClientRect();
          container.scrollTop += row.top - viewport.top - container.clientHeight / 2 + row.height / 2;
        };
        follow();
        const observer = new ResizeObserver(follow);
        observer.observe(container);
        return () => observer.disconnect();
      }
    }
  }, [following, state.activeLine, version?.id]);
  const timed = version?.timing === 'supplied' || (state.estimated && state.estimatedAvailable);
  return <section className="reader live-reader" aria-label="Lyrics" style={{'--lyric-size':`${appearance?.textSize??24}px`,textAlign:appearance?.alignment??'center'} as CSSProperties}>
    {version ? <>
      <p className="reader-caption">{version.sourceName ?? (version.origin === 'personal' ? 'Personal lyrics' : 'Saved lyrics')}<span>{state.estimated && state.estimatedAvailable ? 'Estimated timing' : version.timing === 'supplied' ? 'Supplied timing' : 'Unsynced'}{state.cached ? ' · Saved offline' : ''}</span></p>
      {version.confidence === 'uncertain' && <p className="match-notice">Possible match</p>}
      {!timed || !state.positionAvailable ? <p className="reader-help">{timed ? 'Playback position unavailable · read at your own pace' : 'Read at your own pace'}</p> : <div className="reader-follow"><span>{state.countdownSeconds ? `Next line in ${state.countdownSeconds}s` : following ? 'Following playback' : 'Manual scrolling'}</span>{!following && <button onClick={() => setFollowing(true)}>Resume following</button>}</div>}
      {error&&<p role="alert">{error}</p>}
      <div className="lyric-lines" ref={lines} tabIndex={0} aria-label="Lyric text" onWheel={() => setFollowing(false)} onPointerDown={() => setFollowing(false)} onKeyDown={event => { if (['ArrowDown', 'ArrowUp', 'PageDown', 'PageUp', 'Home', 'End', ' '].includes(event.key)) setFollowing(false); }}>
        {version.lines.map((line, index) => <p key={line.id} dir="auto" data-line={index} className={state.activeLine === index ? 'lyric-focus' : undefined} aria-current={state.activeLine === index ? 'true' : undefined}>{align?<button disabled={busy} onClick={()=>{setAlign(false);void act({kind:'reader',action:{kind:'align',versionId:version.id,line:index}});}}>{line.text || '\u00a0'}</button>:state.activeLine===index&&appearance?.words!==false&&line.words.length>0&&state.wordEmphasis.length===line.words.length?<span aria-label={line.text}>{line.words.map((word,i)=><span aria-hidden="true" key={i} className="lyric-word" style={{opacity:0.45+0.55*(state.wordEmphasis[i]??0)}}>{word.text}</span>)}</span>:line.text || '\u00a0'}</p>)}
      </div>
      {product&&<details className="reader-controls"><summary>Version and timing</summary><label>Lyric version<select disabled={busy} value={version.id} onChange={e=>void act({kind:'reader',action:{kind:'select',versionId:e.target.value}})}>{state.versions.map(v=><option key={v.id} value={v.id}>{v.source} · {v.timing==='supplied'?'Timed':'Unsynced'}{v.confidence==='uncertain'?' · Possible match':''}{v.origin==='personal'?' · Personal':''}</option>)}</select></label><div className="control-row"><button disabled={busy||state.delayMs<=-600000} onClick={()=>void act({kind:'reader',action:{kind:'adjust',versionId:version.id,delayMs:Math.max(-600000,state.delayMs-100),estimated:state.estimated}})}>Earlier −0.1s</button><label>Offset (seconds)<input key={version.id+':'+state.delayMs} type="number" min={-600} max={600} step={0.1} defaultValue={state.delayMs/1000} disabled={busy} onBlur={e=>{const value=Number(e.target.value);if(Number.isFinite(value)&&value>=-600&&value<=600&&Math.round(value*1000)!==state.delayMs)void act({kind:'reader',action:{kind:'adjust',versionId:version.id,delayMs:Math.round(value*1000),estimated:state.estimated}});}}/></label><button disabled={busy||state.delayMs>=600000} onClick={()=>void act({kind:'reader',action:{kind:'adjust',versionId:version.id,delayMs:Math.min(600000,state.delayMs+100),estimated:state.estimated}})}>Later +0.1s</button><button disabled={busy||state.delayMs===0} onClick={()=>void act({kind:'reader',action:{kind:'adjust',versionId:version.id,delayMs:0,estimated:state.estimated}})}>Reset offset</button></div><p className="reader-help">Positive offset displays lyrics later. Changes apply to this version only.</p>{version.timing==='unsynced'&&<label><input type="checkbox" disabled={busy||(!state.estimatedAvailable&&!state.estimated)} checked={state.estimated} onChange={e=>void act({kind:'reader',action:{kind:'adjust',versionId:version.id,delayMs:state.delayMs,estimated:e.target.checked}})}/>Use estimated timing {state.estimatedAvailable?'':'(needs live position and duration)'}</label>}<button disabled={!state.positionAvailable||!timed} aria-pressed={align} onClick={()=>setAlign(v=>!v)}>{align?'Cancel alignment':'Align a line to playback'}</button>{align&&<p>Choose the line being sung now.</p>}</details>}
      {version.attribution && <small className="lyric-attribution">{version.attribution}</small>}
      {state.message && <p role="status">{state.message}</p>}
    </> : <div role="status"><p>{state.message || 'Waiting for lyrics'}</p>{state.status === 'waiting' && <small>Lyrics are collected after ten seconds of playback, or when this reader opens.</small>}{retry && ['failed', 'not_found', 'insufficient_metadata'].includes(state.status) && <button onClick={retry}>Retry lyrics</button>}</div>}
  </section>;
}
