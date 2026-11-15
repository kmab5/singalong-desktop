import validateProduct from './validate-product.generated.js';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Transport } from './transport';

export function createNative(): Transport {
  return {
    product: async command => {const reply=await invoke<unknown>('product_command', { command });if(!validateProduct(reply))throw new Error('Singalong received an invalid command response.');return reply;},
    action: action => invoke('runtime_action', { action }),
    snapshot: () => invoke('get_snapshot'),
    subscribe: callback => listen<unknown>('runtime-state', event => callback(event.payload)),
  };
}
