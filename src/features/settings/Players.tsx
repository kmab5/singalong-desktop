import type { RuntimeSnapshot, RuntimeAction } from '../../bridge/types';
export type RunAction = (action: RuntimeAction) => void;
export function Players({ state, act, busy }: { state: RuntimeSnapshot; act: RunAction; busy: boolean }) {
  return <section className="settings"><h2>Playback and privacy</h2><p>Only enabled apps can be followed. New apps stay off until you enable them.</p>
    <div className="control-row"><button disabled={busy} onClick={() => act({ kind: 'discover' })}>Discover players</button><button disabled={busy} onClick={() => act({ kind: 'tracking', enabled: !state.trackingEnabled })}>{state.trackingEnabled ? 'Stop tracking' : 'Start tracking'}</button></div>
    <fieldset disabled={busy}><legend>Enabled players</legend>{state.players.length ? state.players.map(player => <label className="player-option" key={JSON.stringify(player.id)}><input type="checkbox" checked={player.enabled} onChange={event => act({ kind: 'enable_player', playerId: player.id, enabled: event.target.checked })} /><span>{player.name}<small>{player.available ? 'Available now' : 'Not currently available'}</small></span></label>) : <p>Play something in a media app, then discover its exposed session.</p>}</fieldset>
    <label className="player-option"><input disabled={busy} type="checkbox" checked={state.startAtSignIn} onChange={event => act({ kind: 'startup', enabled: event.target.checked })} /><span>Start Singalong when I sign in to Windows<small>Starts in the tray. Resumes tracking only if it was previously on.</small></span></label>
    <label className="player-option"><input disabled={busy} type="checkbox" checked={state.offline} onChange={event => act({ kind: 'offline', enabled: event.target.checked })} /><span>Offline mode<small>Read saved lyrics without contacting providers. Otherwise, lookup can use any connection.</small></span></label>
    <p>Closing this window keeps Singalong in the system tray. Use Quit Singalong in the tray to stop the application.</p>
    <label className="player-option"><input disabled={busy} type="checkbox" checked={state.autoOverlay} onChange={event => act({ kind: 'auto_overlay', enabled: event.target.checked })} /><span>Show floating lyrics automatically<small>Off by default. Manually hiding or minimizing the reader keeps it hidden until you show it again or re-enable this option.</small></span></label>
    <button disabled={busy} onClick={() => act({ kind: 'quit' })}>Quit Singalong</button>
  </section>;
}
