import { useEffect, useState } from 'react';
import { connect, type Transport } from '../bridge/transport';
import { Players } from '../features/settings/Players';
import { SessionPicker } from '../features/now/SessionPicker';
import type { RuntimeAction, RuntimeSnapshot } from '../bridge/types';

type Page = 'Now' | 'Library' | 'Settings';
export function App({ transport, scenario }: { transport: Transport; scenario?: (value: 'paused' | 'playing' | 'empty') => void }) {
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
  const session = snapshot?.sessions.find(session => session.sessionId === snapshot.selectedSessionId);
  const fixture = snapshot?.capabilities.mode === 'fixture';
  return <div className="shell">
    <aside className="sidebar">
      <div className="wordmark"><span className="brand-mark" aria-hidden="true">♫</span> singalong<span className="desktop-label">DESKTOP</span></div>
      <nav aria-label="Main navigation">{(['Now', 'Library', 'Settings'] as Page[]).map((item, index) => <button key={item} aria-current={page === item ? 'page' : undefined} onClick={() => setPage(item)}><span aria-hidden="true">{['♪', '▤', '⚙'][index]}</span>{item}</button>)}</nav>
      <p className="sidebar-note">Your words.<br />Your music.</p>
      <span className="build-label">Tracking development</span>
    </aside>
    <main className="workspace">
      <header><div><p className="eyebrow">SING ALONG, STAY IN THE MOMENT</p><h1>{page === 'Now' ? 'Now playing' : page}</h1></div><span className="status"><span aria-hidden="true" />{fixture ? 'Local fixture' : snapshot?.trackingEnabled ? 'Tracking on' : 'Tracking off'}</span></header>
      {actionError && <p role="alert">{actionError}</p>}
      {snapshot?.closeNotice && <section className="close-notice" role="alert"><h2>Singalong stays in the tray</h2><p>Tracking continues when this window closes. Use Quit Singalong in the tray to stop everything.</p><button disabled={busy} onClick={() => act({ kind: 'close_to_tray' })}>Got it — hide in tray</button></section>}
      {snapshot && !fixture && page === 'Now' && transport.action && <><div className="control-row"><button disabled={busy} onClick={() => act({ kind: 'tracking', enabled: !snapshot.trackingEnabled })}>{snapshot.trackingEnabled ? 'Stop tracking' : 'Start tracking'}</button><button onClick={() => setPage('Settings')}>Choose players</button></div><SessionPicker state={snapshot} act={act} busy={busy} /></>}
      {scenario && <section className="fixture-tools" aria-label="Fixture controls"><strong>Synthetic content · local testing only</strong><div>{(['paused', 'playing', 'empty'] as const).map(value => <button key={value} onClick={() => scenario(value)}>{value}</button>)}</div></section>}
      {error ? <section role="alert" className="empty"><h2>Connection unavailable</h2><p>{error}</p></section> : !snapshot ? <p role="status">Connecting to Singalong…</p> : page === 'Now' ? session ? <>
        <section className="track"><div className="record-art" aria-hidden="true"><span>♪</span></div><div><p className="eyebrow">{session.playerName} · {session.status}</p><h2>{session.recording?.title ?? 'Unknown track'}</h2><p>{session.recording?.artists.join(', ') || 'Artist unavailable'}</p></div></section>
        {fixture ? <section className="reader" aria-label="Synthetic lyrics"><p className="reader-caption">Original sample lyrics <span>Unsynced</span></p><p>A little light across the sky</p><p className="lyric-focus">We follow as the clouds go by</p><p>One quiet note, another day</p><p>The words will find their way</p><small>Playback position unavailable · read at your own pace</small></section> : <section className="reader"><p className="reader-caption">{session.positionMs === null ? 'Playback position unavailable · manual reading' : `Position ${Math.floor(Number(session.positionMs) / 1000)} s`}</p><p>Lyrics are coming next.</p><small>Live detection is connected. Provider lookup and the floating reader are the next development steps.</small></section>}
      </> : <section className="empty"><div className="empty-note" aria-hidden="true">♫</div><h2>A place for the words</h2><p>Singalong will follow music from the players you enable.</p><p className="muted">{snapshot.capabilities.reason}</p></section> : page === 'Library' ? <section className="empty"><h2>Your songs will live here</h2><p>Saved lyrics, favourites and offline reading are coming in a later development step.</p><button onClick={() => setPage('Now')}>Return to Now</button></section> : transport.action ? <Players state={snapshot} act={act} busy={busy} /> : <section className="settings"><h2>Playback and privacy</h2><dl><div><dt>Playback tracking</dt><dd>Off · not connected yet</dd></div><div><dt>Start at sign-in</dt><dd>Off</dd></div><div><dt>Automatic floating lyrics</dt><dd>Off</dd></div></dl><p>No players are being monitored. Tracking controls will appear when the native runtime is ready.</p></section>}
      <footer><span>{fixture ? 'No real players, network lookups or library writes' : 'Development build · lyrics lookup is not connected yet'}</span><span>Singalong Desktop</span></footer>
    </main>
  </div>;
}
