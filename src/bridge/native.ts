import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Transport } from './transport';

export function createNative(): Transport {
  return {
    snapshot: () => invoke('get_snapshot'),
    subscribe: callback => listen<unknown>('runtime-state', event => callback(event.payload)),
  };
}
