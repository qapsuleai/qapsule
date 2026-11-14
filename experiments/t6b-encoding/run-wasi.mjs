// Runs a wasm32-wasip1 binary under Node's WASI (preview1). Usage: node run-wasi.mjs file.wasm
import { readFile } from 'node:fs/promises';
import { WASI } from 'node:wasi';
import { argv, env } from 'node:process';

const wasi = new WASI({ version: 'preview1', args: [argv[2]], env: {}, preopens: {} });
const mod = await WebAssembly.compile(await readFile(argv[2]));
const inst = await WebAssembly.instantiate(mod, wasi.getImportObject());
wasi.start(inst);
