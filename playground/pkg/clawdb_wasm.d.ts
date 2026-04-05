/* tslint:disable */
/* eslint-disable */

/**
 * Clear all data from memory
 */
export function clear(): string;

/**
 * Dump all records from a memory type
 */
export function dump(memory_type: string): string;

/**
 * Execute an AQL query and return JSON result
 */
export function execute(query: string): string;

/**
 * Initialize the WASM module
 */
export function init(): void;

/**
 * Parse an AQL query and return AST as JSON (for debugging)
 */
export function parse(query: string): string;

/**
 * Get statistics about stored data
 */
export function stats(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly clear: () => [number, number];
    readonly dump: (a: number, b: number) => [number, number];
    readonly execute: (a: number, b: number) => [number, number];
    readonly parse: (a: number, b: number) => [number, number];
    readonly stats: () => [number, number];
    readonly init: () => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
