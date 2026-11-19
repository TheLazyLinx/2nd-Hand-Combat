import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import prettier from 'prettier';
import { root, toolEnvironment } from './tooling.mjs';

const outputPath = join(root, 'apps', 'web', 'src', 'generated', 'sim-types.ts');
const checkOnly = process.argv.includes('--check');

const result = spawnSync(
  'cargo',
  ['run', '-q', '-p', 'sim-core', '--features', 'ts-export', '--bin', 'export-types'],
  {
    cwd: root,
    env: toolEnvironment(),
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    windowsHide: true,
  },
);

if (result.error) {
  console.error(`Cannot run cargo to export TypeScript types: ${result.error.message}`);
  process.exit(1);
}
if (result.status !== 0) {
  process.stderr.write(result.stderr || '');
  process.exit(result.status ?? 1);
}

const config = (await prettier.resolveConfig(outputPath)) ?? {};
const formatted = await prettier.format(result.stdout, {
  ...config,
  filepath: outputPath,
});
const normalize = (text) => text.replaceAll('\r\n', '\n');

if (checkOnly) {
  if (!existsSync(outputPath)) {
    console.error('Missing apps/web/src/generated/sim-types.ts. Run: npm run gen:types');
    process.exit(1);
  }
  const existing = readFileSync(outputPath, 'utf8');
  if (normalize(existing) !== normalize(formatted)) {
    console.error(
      'Generated TypeScript types (apps/web/src/generated/sim-types.ts) are out of date with crates/sim-core. Run: npm run gen:types',
    );
    process.exit(1);
  }
  console.log('Generated TypeScript types are up to date.');
} else {
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, formatted, 'utf8');
  console.log('Wrote apps/web/src/generated/sim-types.ts');
}
