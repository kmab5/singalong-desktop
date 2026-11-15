import { useEffect, useRef, useState } from 'react';
import type { ReaderState } from '../../bridge/types';

export function Reader({ state, retry }: { state: ReaderState; retry?: () => void }) {
  const [following, setFollowing] = useState(true);
  const lines = useRef<HTMLDivElement>(null);
  const version = state.version;
  useEffect(() => setFollowing(true), [version?.id]);
  useEffect(() => {
    if (following && state.activeLine != null && lines.current) {
      const container=lines.current;
      const line=container.querySelector<HTMLElement>(`[data-line="${state.activeLine}"]`);
      if (line) container.scrollTop=line.offsetTop-container.offsetTop-container.clientHeight/2+line.clientHeight/2;
    }
  }, [following, state.activeLine, version?.id]);
  const timed = version?.timing === 'supplied';
  return <section className="reader live-reader" aria-label="Lyrics">
    {version ? <>
      <p className="reader-caption">{version.sourceName ?? (version.origin === 'personal' ? 'Personal lyrics' : 'Saved lyrics')}<span>{timed ? 'Supplied timing' : 'Unsynced'}{state.cached ? ' · Saved offline' : ''}</span></p>
      {version.confidence === 'uncertain' && <p className="match-notice">Possible match</p>}
      {!timed || !state.positionAvailable ? <p className="reader-help">{timed ? 'Playback position unavailable · read at your own pace' : 'Read at your own pace'}</p> : <div className="reader-follow"><span>{state.countdownSeconds ? `Next line in ${state.countdownSeconds}s` : following ? 'Following playback' : 'Manual scrolling'}</span>{!following && <button onClick={() => setFollowing(true)}>Resume following</button>}</div>}
      <div className="lyric-lines" ref={lines} tabIndex={0} aria-label="Lyric text" onWheel={() => setFollowing(false)} onPointerDown={() => setFollowing(false)} onKeyDown={event => { if (['ArrowDown', 'ArrowUp', 'PageDown', 'PageUp', 'Home', 'End', ' '].includes(event.key)) setFollowing(false); }}>
        {version.lines.map((line, index) => <p key={line.id} data-line={index} className={state.activeLine === index ? 'lyric-focus' : undefined} aria-current={state.activeLine === index ? 'true' : undefined}>{line.text || '\u00a0'}</p>)}
      </div>
      {version.attribution && <small className="lyric-attribution">{version.attribution}</small>}
      {state.message && <p role="status">{state.message}</p>}
    </> : <div role="status"><p>{state.message || 'Waiting for lyrics'}</p>{state.status === 'waiting' && <small>Lyrics are collected after ten seconds of playback, or when this reader opens.</small>}{retry && ['failed', 'not_found', 'insufficient_metadata'].includes(state.status) && <button onClick={retry}>Retry lyrics</button>}</div>}
  </section>;
}
