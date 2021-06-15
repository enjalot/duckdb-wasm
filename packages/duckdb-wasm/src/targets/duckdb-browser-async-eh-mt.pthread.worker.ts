import * as pthread_api from '../bindings/duckdb_wasm_eh_mt.pthread';
import DuckDB from '../bindings/duckdb_wasm_eh_mt';
import { BROWSER_RUNTIME } from '../bindings/runtime_browser';

// Register the global DuckDB runtime
globalThis.DUCKDB_RUNTIME = {};
for (const func of Object.getOwnPropertyNames(BROWSER_RUNTIME)) {
    if (func == 'constructor') continue;
    globalThis.DUCKDB_RUNTIME[func] = Object.getOwnPropertyDescriptor(BROWSER_RUNTIME, func)!.value;
}

// We just override the load handler of the pthread wrapper to bundle DuckDB with esbuild.
globalThis.onmessage = (e: any) => {
    if (e.data.cmd === 'load') {
        const m = pthread_api.getModule();

        // Module and memory were sent from main thread
        m['wasmModule'] = e.data.wasmModule;
        m['wasmMemory'] = e.data.wasmMemory;
        m['buffer'] = m['wasmMemory'].buffer;
        m['ENVIRONMENT_IS_PTHREAD'] = true;

        DuckDB(m).then(function (instance) {
            pthread_api.setModule(instance);
        });
    } else {
        pthread_api.onmessage(e);
    }
};
