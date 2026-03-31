import wasmModule from './wasm/indexbind_wasm_bg.wasm';
import { initSync, WasmIndex } from './wasm/indexbind_wasm.js';
import { openWebIndexWithBindings } from './web-core.js';

export {
  WebIndex,
  type BestMatch,
  type DocumentHit,
  type JsonValue,
  type OpenWebIndexOptions,
  type RerankerOptions,
  type SearchOptions,
  type WebArtifactInfo,
} from './web-core.js';

let wasmInitialized = false;

export async function openWebIndex(
  base: string | URL,
  options: import('./web-core.js').OpenWebIndexOptions = {},
) {
  if (options.modeProfile !== 'lexical' && !wasmInitialized) {
    initSync({ module: wasmModule });
    wasmInitialized = true;
  }
  return openWebIndexWithBindings(base, WasmIndex, options);
}

export { openWebIndex as openCloudflareIndex };
