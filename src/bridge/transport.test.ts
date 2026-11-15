import { describe, it, expect } from 'vitest';
import { connect, SnapshotStore, type Transport } from './transport';
import { fixtureSnapshot } from './fixture';

describe('shared state boundary', () => {
  it('rejects non-finite rates but accepts an unknown rate', () => {
    for (const rate of [NaN, Infinity, -Infinity]) {
      const value = fixtureSnapshot(); value.sessions[0].rate = rate;
      expect(() => new SnapshotStore().accept(value)).toThrow();
    }
    const value = fixtureSnapshot(); value.sessions[0].rate = null;
    expect(new SnapshotStore().accept(value)).toBe(true);
  });
  it('closes the connection on a malformed initial snapshot or event', async () => {
    for (const invalidAt of ['snapshot', 'event', 'registration']) {
      let callback!: (value: unknown) => void;
      let cleanup = 0, failures = 0, received = 0;
      const transport: Transport = {
        async subscribe(value) { callback = value; if (invalidAt === 'registration') value({}); return () => { cleanup++; }; },
        async snapshot() { return invalidAt === 'snapshot' ? {} : fixtureSnapshot(); },
      };
      const stop = await connect(transport, () => { received++; }, () => { failures++; }, new AbortController().signal);
      if (invalidAt === 'event') callback({});
      const before = received;
      callback({ ...fixtureSnapshot(), revision: '2' });
      expect(received).toBe(before);
      expect(failures).toBe(1);
      expect(cleanup).toBe(1);
      stop(); expect(cleanup).toBe(1);
    }
  });
  it('preserves nullable timing, Unicode and large counters', () => {
    const store = new SnapshotStore();
    const snapshot = fixtureSnapshot();
    snapshot.revision = '9007199254740993';
    snapshot.sessions[0].positionMs = null;
    snapshot.sessions[0].playerName = '播放器 🎵';
    expect(store.accept(snapshot)).toBe(true);
    expect(store.current?.sessions[0].positionMs).toBeNull();
    expect(store.current?.sessions[0].playerName).toBe('播放器 🎵');
    expect(store.accept({ ...snapshot, revision: '9007199254740992' })).toBe(false);
  });
  it('rejects stale generations, duplicate revisions, malformed counters and unknown fields', () => {
    const store = new SnapshotStore();
    store.accept({ ...fixtureSnapshot(), generation: '2', revision: '0' });
    expect(store.accept({ ...fixtureSnapshot(), generation: '1', revision: '999' })).toBe(false);
    expect(store.accept({ ...fixtureSnapshot(), generation: '2', revision: '0' })).toBe(false);
    for (const revision of ['01', '-1', '18446744073709551616', 123]) {
      expect(() => store.accept({ ...fixtureSnapshot(), revision })).toThrow();
    }
    expect(() => store.accept({ ...fixtureSnapshot(), unexpected: true })).toThrow();
  });
  it('subscribes before snapshot and retains a newer event arriving during startup', async () => {
    let subscribed = false;
    const received: string[] = [];
    const transport: Transport = {
      async subscribe(callback) { subscribed = true; callback({ ...fixtureSnapshot(), revision: '2' }); return () => {}; },
      async snapshot() { expect(subscribed).toBe(true); return { ...fixtureSnapshot(), revision: '1' }; },
    };
    const stop = await connect(transport, state => received.push(state.revision), () => {}, new AbortController().signal);
    expect(received).toEqual(['2']);
    stop();
  });
  it('cleans up a delayed subscription after unmount and ignores late data', async () => {
    const controller = new AbortController();
    let release!: (stop: () => void) => void;
    let cleanup = 0;
    let reads = 0;
    const received: unknown[] = [];
    const transport: Transport = {
      subscribe: () => new Promise(resolve => { release = resolve; }),
      async snapshot() { reads++; return fixtureSnapshot(); },
    };
    const pending = connect(transport, value => received.push(value), () => {}, controller.signal);
    controller.abort();
    release(() => { cleanup++; });
    const stop = await pending;
    stop();
    expect(cleanup).toBe(1);
    expect(reads).toBe(0);
    expect(received).toEqual([]);
  });
  it('releases the listener if startup snapshot fails', async () => {
    let cleanup = 0;
    const transport: Transport = {
      async subscribe() { return () => { cleanup++; }; },
      async snapshot() { throw new Error('disconnected'); },
    };
    await expect(connect(transport, () => {}, () => {}, new AbortController().signal)).rejects.toThrow('disconnected');
    expect(cleanup).toBe(1);
  });
});

it('validates camelCase pin fields emitted by Rust', () => {
  const state = fixtureSnapshot();
  state.selection = { kind: 'app_pin', playerId: state.sessions[0].playerId };
  expect(new SnapshotStore().accept(state)).toBe(true);
});
