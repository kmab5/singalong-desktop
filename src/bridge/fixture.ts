import type { RuntimeSnapshot } from './types';
import type { Transport } from './transport';

export function fixtureSnapshot(): RuntimeSnapshot {
  return {
    schema: 1, revision: '1', generation: '0', trackingEnabled: false,
    capabilities: { mode: 'fixture', platform: 'windows', trackingAvailable: false, libraryAvailable: false, reason: 'Synthetic local fixture. No players are being monitored.' },
    sessions: [{
      sessionId: 'fixture-session', playerId: { platform: 'windows', nativeId: 'singalong.synthetic' },
      playerName: 'Synthetic player', status: 'paused', positionMs: null, observedAtMs: '0', rate: null,
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
