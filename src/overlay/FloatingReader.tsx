import { useEffect, useState } from 'react';
import { connect, type Transport } from '../bridge/transport';
import type { RuntimeSnapshot } from '../bridge/types';
import { Reader } from '../features/now/Reader';

export function FloatingReader({ transport }: { transport: Transport }) {
  const [state, setState] = useState<RuntimeSnapshot>();
  const [error, setError] = useState('');
  useEffect(() => {
    const controller = new AbortController();
    void connect(transport, setState, error => setError(error.message), controller.signal).catch(() => setError('Reader connection unavailable'));
    return () => controller.abort();
  }, [transport]);
  const session = state?.sessions.find(s => s.sessionId === state.selectedSessionId);
  return <main className="floating-shell">
    <header><p className="eyebrow">SINGALONG</p><h1>{session?.recording?.title || 'Waiting for music'}</h1><p>{session?.recording?.artists.join(', ') || state?.statusMessage}</p></header>
    {error ? <p role="alert">{error}</p> : state && <Reader state={state.reader} />}
  </main>;
}
