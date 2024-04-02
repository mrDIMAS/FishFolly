/* tslint:disable */
/* eslint-disable */
/**
*/
export function main(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly main: () => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly wasm_bindgen__convert__closures__invoke0_mut__ha285f2b48cefc64c: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures__invoke1_mut__h04809d32b7bc1b4b: (a: number, b: number, c: number) => void;
  readonly wasm_bindgen__convert__closures__invoke2_mut__h617336902c1d9a47: (a: number, b: number, c: number, d: number) => void;
  readonly wasm_bindgen__convert__closures__invoke0_mut__hbe92f595e93378b9: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures__invoke0_mut__hb593dd527393c6ef: (a: number, b: number) => void;
  readonly wasm_bindgen__convert__closures__invoke1_mut__h10d814b60b45b6aa: (a: number, b: number, c: number) => void;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h195f574f2a5cd6d5: (a: number, b: number, c: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
