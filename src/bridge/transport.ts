import type { RuntimeSnapshot } from './types';
import validate from './validate.generated.js';
export interface Transport {
  snapshot(): Promise<unknown>;
  subscribe(callback: (state: unknown) => void): Promise<() => void>;
}
export class SnapshotStore {
  current: RuntimeSnapshot | undefined;
  accept(value: unknown): boolean {
    if (!validate(value)) throw new Error('Singalong received an invalid state update.');
    if (this.current) {
      const generation = BigInt(value.generation), currentGeneration = BigInt(this.current.generation);
      if (generation < currentGeneration || (generation === currentGeneration && BigInt(value.revision) <= BigInt(this.current.revision))) return false;
    }
    this.current = structuredClone(value);
    return true;
  }
}
export async function connect(transport: Transport, receive: (value: RuntimeSnapshot) => void, failure: (error: Error) => void, signal: AbortSignal): Promise<() => void> {
  const store = new SnapshotStore();
  let stopped = false;
  let unsubscribe: (() => void) | undefined;
  const stop = () => {
    stopped = true;
    signal.removeEventListener('abort', stop);
    const cleanup = unsubscribe;
    unsubscribe = undefined;
    cleanup?.();
  };
  const accept = (value: unknown) => {
    if (stopped) return;
    try { if (store.accept(value)) receive(store.current!); }
    catch (error) { stop(); failure(error instanceof Error ? error : new Error('Invalid state update.')); }
  };
  if (signal.aborted) return stop;
  signal.addEventListener('abort', stop, { once: true });
  try {
    unsubscribe = await transport.subscribe(accept);
    if (stopped) { stop(); return stop; }
    accept(await transport.snapshot());
    return stop;
  } catch (error) {
    stop();
    throw error;
  }
}
