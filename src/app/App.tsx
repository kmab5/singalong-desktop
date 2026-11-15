import { useEffect, useState, useCallback } from 'react';
import { connect, type Transport } from '../bridge/transport';
import { Players } from '../features/settings/Players';
import {Backup} from '../features/settings/Backup';
import {About} from '../features/settings/About';
import {Appearance} from '../features/settings/Appearance';
import {LibraryData} from '../features/settings/LibraryData';
import {Search} from '../features/library/Search';
import type {LibraryDetail} from '../bridge/types';
import { Library } from '../features/library/Library';
import type { ProductCommand } from '../bridge/types';
import { Reader } from '../features/now/Reader';
import { SessionPicker } from '../features/now/SessionPicker';
import type { RuntimeAction, RuntimeSnapshot } from '../bridge/types';

type Page = 'Now' | 'Library' | 'Settings';
export function App({ transport, scenario }: { transport: Transport; scenario?: (value: 'paused' | 'playing' | 'empty') => void }) {
  const product=useCallback((command:ProductCommand)=>{if(!transport.product)return Promise.reject(new Error('This feature requires the native app.'));return transport.product(command);},[transport]);
  const [correction,setCorrection]=useState<LibraryDetail>();
  const [page, setPage] = useState<Page>('Now');
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot>();
  const [error, setError] = useState('');
  const [actionError, setActionError] = useState('');
  const [busy, setBusy] = useState(false);
  const act = (action: RuntimeAction) => { if (!transport.action || busy) return; setBusy(true); setActionError(''); void transport.action(action).catch((error: unknown) => setActionError(typeof error === 'object' && error && 'message' in error ? String(error.message) : 'The action could not finish. Try again.')).finally(() => setBusy(false)); };
  useEffect(() => {
    const controller = new AbortController();
    void connect(transport, value => { setSnapshot(value); setError(''); }, error => setError(error.message), controller.signal)
      .catch(() => { if (!controller.signal.aborted) setError('Could not connect to Singalong. Close and reopen the window to retry.'); });
    return () => controller.abort();
  }, [transport]);
  useEffect(() => {
    if (snapshot?.capabilities.mode !== 'native' || !transport.action) return;
    void transport.action({ kind: 'reader_active', enabled: page === 'Now' }).catch(() => {});
  }, [page, snapshot?.capabilities.mode, transport]);
  const session = snapshot?.sessions.find(session => session.sessionId === snapshot.selectedSessionId);
  const fixture = snapshot?.capabilities.mode === 'fixture';
  return <div className={`shell ${snapshot?.appearance.reducedMotion?'reduced-motion':''}`}>
    <aside className="sidebar">
      <div className="wordmark"><span className="brand-mark" aria-hidden="true">♫</span> singalong<span className="desktop-label">DESKTOP</span></div>
      <nav aria-label="Main navigation">{(['Now', 'Library', 'Settings'] as Page[]).map((item, index) => <button key={item} aria-current={page === item ? 'page' : undefined} onClick={() => setPage(item)}><span aria-hidden="true">{['♪', '▤', '⚙'][index]}</span>{item}</button>)}</nav>
      <p className="sidebar-note">Your words.<br />Your music.</p>
      <span className="build-label">Lyrics development</span>
    </aside>
    <main className="workspace">
      <header><div><p className="eyebrow">SING ALONG, STAY IN THE MOMENT</p><h1>{page === 'Now' ? 'Now playing' : page}</h1></div><span className="status"><span aria-hidden="true" />{fixture ? 'Local fixture' : snapshot?.trackingEnabled ? 'Tracking on' : 'Tracking off'}</span></header>
      {actionError && <p role="alert">{actionError}</p>}
      {snapshot?.closeNotice && <section className="close-notice" role="alert"><h2>Singalong stays in the tray</h2><p>Tracking continues when this window closes. Use Quit Singalong in the tray to stop everything.</p><button disabled={busy} onClick={() => act({ kind: 'close_to_tray' })}>Got it — hide in tray</button></section>}
      {snapshot && !fixture && page === 'Now' && transport.action && <><div className="control-row"><button disabled={busy} onClick={() => act({ kind: 'tracking', enabled: !snapshot.trackingEnabled })}>{snapshot.trackingEnabled ? 'Stop tracking' : 'Start tracking'}</button><button onClick={() => setPage('Settings')}>Choose players</button><button disabled={busy} onClick={() => act({ kind: 'show_overlay', visible: !snapshot.overlayVisible })}>{snapshot.overlayVisible ? 'Hide floating lyrics' : 'Show floating lyrics'}</button></div><SessionPicker state={snapshot} act={act} busy={busy} /></>}
      {scenario && <section className="fixture-tools" aria-label="Fixture controls"><strong>Synthetic content · local testing only</strong><div>{(['paused', 'playing', 'empty'] as const).map(value => <button key={value} onClick={() => scenario(value)}>{value}</button>)}</div></section>}
      {error ? <section role="alert" className="empty"><h2>Connection unavailable</h2><p>{error}</p></section> : !snapshot ? <p role="status">Connecting to Singalong…</p> : page === 'Now' ? session ? <>
        <section className="track"><div className="record-art" aria-hidden="true"><span>♪</span></div><div><p className="eyebrow">{session.playerName} · {session.status}</p><h2>{session.recording?.title ?? 'Unknown track'}</h2><p>{session.recording?.artists.join(', ') || 'Artist unavailable'}</p></div></section>
        {!fixture&&transport.product&&<><button onClick={()=>{void product({kind:'current_detail'}).then(reply=>{if(reply.kind==='detail')setCorrection(reply.detail);}).catch(()=>setActionError('Could not open lyric correction.'));}}>Wrong lyrics</button>{correction&&<><button onClick={()=>setCorrection(undefined)}>Close correction</button><Search product={product} target={correction} saved={()=>setCorrection(undefined)}/></>}</>}
        {fixture ? <section className="reader" aria-label="Synthetic lyrics"><p className="reader-caption">Original sample lyrics <span>Unsynced</span></p><p>A little light across the sky</p><p className="lyric-focus">We follow as the clouds go by</p><p>One quiet note, another day</p><p>The words will find their way</p><small>Playback position unavailable · read at your own pace</small></section> : <Reader state={snapshot.reader} product={transport.product?product:undefined} appearance={snapshot.appearance.main} retry={() => act({ kind: 'retry_lyrics' })} />}
      </> : <section className="empty"><div className="empty-note" aria-hidden="true">♫</div><h2>A place for the words</h2><p>Singalong will follow music from the players you enable.</p><p className="muted">{snapshot.capabilities.reason}</p></section> : page === 'Library' ? transport.product ? <Library product={product} state={snapshot} returnToNow={()=>setPage('Now')}/> : <p>Library is available in the native app.</p> : transport.action ? <><Players state={snapshot} act={act} busy={busy} />{transport.product&&<><Appearance state={snapshot} product={product}/><LibraryData product={product}/><Backup product={product}/><About product={product}/></>}</> : <section className="settings"><h2>Playback and privacy</h2><dl><div><dt>Playback tracking</dt><dd>Off · not connected yet</dd></div><div><dt>Start at sign-in</dt><dd>Off</dd></div><div><dt>Automatic floating lyrics</dt><dd>Off</dd></div></dl><p>No players are being monitored. Tracking controls will appear when the native runtime is ready.</p></section>}
      <footer><span>{fixture ? 'No real players, network lookups or library writes' : 'Development build · Windows lyrics companion'}</span><span>Singalong Desktop</span></footer>
    </main>
  </div>;
}
