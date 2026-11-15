import type { RuntimeSnapshot } from './types';
import type { Transport } from './transport';

export function fixtureSnapshot(): RuntimeSnapshot {
  return {
    schema: 1, revision: '1', generation: '0', trackingEnabled: false,
    appearance:{main:{textSize:24,alignment:'center',words:true},floating:{textSize:24,alignment:'center',words:true},opacity:0.95,reducedMotion:false},overlayMode:'compact',clickThrough:false,overlayCollapsed:false,overlayDocked:false,
    overlayVisible: false, offline: false, reader: { status: 'idle', version: null, activeLine: null, countdownSeconds: null, positionAvailable: false, cached: false, message: '', retryAfterMs: null,versions:[],delayMs:0,estimated:false,estimatedAvailable:false,wordEmphasis:[] },
    players: [], selection: { kind: 'automatic' }, selectedSessionId: 'fixture-session', statusMessage: '', startAtSignIn: false, autoOverlay: false, closeNotice: false,
    capabilities: { mode: 'fixture', platform: 'windows', trackingAvailable: false, libraryAvailable: false, reason: 'Synthetic local fixture. No players are being monitored.' },
    sessions: [{
      sessionId: 'fixture-session', playerId: { platform: 'windows', nativeId: 'singalong.synthetic' },
      playerName: 'Synthetic player', status: 'paused', positionMs: null, observedAtMs: '0', rate: null, streamPinAvailable: false,
      recording: { id: 'fixture-recording', title: 'A little light', artists: ['Singalong fixtures'], album: null, durationMs: null, versionLabel: null, fileName: null },
    }],
  };
}

export type FixtureTransport = Transport & { setScenario: (scenario: 'paused' | 'playing' | 'empty') => void };
export function createFixture(): FixtureTransport {
  let current = fixtureSnapshot();
  const listeners = new Set<(value: unknown) => void>();
  return {
    async snapshot() { return structuredClone(current); },
    async subscribe(callback) { listeners.add(callback); return () => { listeners.delete(callback); }; },
    setScenario(scenario) {
      const revision = (BigInt(current.revision) + 1n).toString();
      current = fixtureSnapshot();
      current.revision = revision;
      if (scenario === 'empty') current.sessions = [];
      else current.sessions[0].status = scenario;
      for (const callback of listeners) callback(structuredClone(current));
    },
  };
}
