import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { execFileSync } from 'node:child_process';

const root = process.cwd();
const wasmTarget = 'wasm32-unknown-unknown';
const wasmOutDir = resolve(root, 'dist', 'wasm');
const wasmArtifact = resolve(
  root,
  'target',
  wasmTarget,
  'release',
  'indexbind_wasm.wasm',
);

mkdirSync(dirname(wasmArtifact), { recursive: true });
mkdirSync(wasmOutDir, { recursive: true });

execFileSync(
  'cargo',
  ['build', '-p', 'indexbind-wasm', '--target', wasmTarget, '--release'],
  { stdio: 'inherit' },
);

execFileSync(
  'wasm-bindgen',
  ['--target', 'web', '--out-dir', wasmOutDir, wasmArtifact],
  { stdio: 'inherit' },
);
