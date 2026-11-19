import assert from 'node:assert/strict';
import { join } from 'node:path';
import test from 'node:test';
import { ESLint } from 'eslint';
import { root } from '../scripts/tooling.mjs';

const eslint = new ESLint({ cwd: root });

async function lintSnippet(relativePath, code) {
  const [result] = await eslint.lintText(code, { filePath: join(root, relativePath) });
  return result.messages;
}

test('Svelte components cannot import Worker internals or generated WASM bindings', async () => {
  const workerImport = await lintSnippet(
    'apps/web/src/components/Sample.svelte',
    `<script lang="ts">\n  import '../worker';\n</script>\n`,
  );
  assert.ok(
    workerImport.some((m) => m.ruleId === 'no-restricted-imports'),
    `expected no-restricted-imports for ../worker, got: ${JSON.stringify(workerImport)}`,
  );

  const wasmImport = await lintSnippet(
    'apps/web/src/components/Sample.svelte',
    `<script lang="ts">\n  import init from '../generated/wasm/sim_wasm';\n  void init;\n</script>\n`,
  );
  assert.ok(
    wasmImport.some((m) => m.ruleId === 'no-restricted-imports'),
    `expected no-restricted-imports for generated WASM, got: ${JSON.stringify(wasmImport)}`,
  );
});

test('presentation code cannot instantiate Worker directly outside BattleClient', async () => {
  const violation = await lintSnippet(
    'apps/web/src/components/Sample.svelte',
    `<script lang="ts">\n  const w = new Worker('worker.ts');\n  void w;\n</script>\n`,
  );
  assert.ok(
    violation.some((m) => m.ruleId === 'no-restricted-syntax'),
    `expected no-restricted-syntax for new Worker, got: ${JSON.stringify(violation)}`,
  );

  const allowed = await lintSnippet(
    'apps/web/src/lib/client.ts',
    `export const w = new Worker(new URL('../worker.ts', import.meta.url), { type: 'module' });\n`,
  );
  assert.deepEqual(
    allowed.filter((m) => m.ruleId === 'no-restricted-syntax'),
    [],
  );
});

test('renderer stays decoupled from BattleClient, Worker, and Svelte', async () => {
  const clientImport = await lintSnippet(
    'apps/web/src/lib/battlefield.ts',
    `import { BattleClient } from './client';\nvoid BattleClient;\n`,
  );
  assert.ok(
    clientImport.some((m) => m.ruleId === 'no-restricted-imports'),
    `expected no-restricted-imports for ./client, got: ${JSON.stringify(clientImport)}`,
  );

  const svelteImport = await lintSnippet(
    'apps/web/src/lib/battlefield.ts',
    `import { tick } from 'svelte';\nvoid tick;\n`,
  );
  assert.ok(
    svelteImport.some((m) => m.ruleId === 'no-restricted-imports'),
    `expected no-restricted-imports for svelte, got: ${JSON.stringify(svelteImport)}`,
  );
});

test('Worker stays decoupled from PixiJS, Svelte, and renderer modules', async () => {
  const pixiImport = await lintSnippet(
    'apps/web/src/worker.ts',
    `import { Application } from 'pixi.js';\nvoid Application;\n`,
  );
  assert.ok(
    pixiImport.some((m) => m.ruleId === 'no-restricted-imports'),
    `expected no-restricted-imports for pixi.js, got: ${JSON.stringify(pixiImport)}`,
  );
});
