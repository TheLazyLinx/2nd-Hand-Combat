import { join } from 'node:path';
import { root, runTool } from './tooling.mjs';
runTool('wasm-pack', [
  'build', 'crates/sim-wasm', '--target', 'web',
  '--out-dir', join(root, 'apps/web/src/generated'),
  '--out-name', 'sim_wasm', '--no-pack',
  process.argv.includes('--dev') ? '--dev' : '--release',
]);
