import { spawnSync } from 'node:child_process';
import { performance } from 'node:perf_hooks';
import { root } from './tooling.mjs';

const fast = process.argv.includes('--fast');

const steps = fast
  ? [
      'fmt:check',
      'lint:rust',
      'test:rust',
      'fmt:web:check',
      'lint:web',
      'test:boundaries',
      'check:privacy',
      'test:privacy',
      'build',
      'check',
      'test:browser:smoke',
    ]
  : [
      'fmt:check',
      'lint:rust',
      'test:rust',
      'deny:rust',
      'fmt:web:check',
      'lint:web',
      'test:boundaries',
      'check:privacy',
      'test:privacy',
      'build',
      'check',
      'test:browser',
    ];

const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const totalStart = performance.now();
const summary = [];

for (const step of steps) {
  const start = performance.now();
  console.log(`\n=== verify (${fast ? 'fast' : 'full'}): npm run ${step} ===`);
  const result = spawnSync(npm, ['run', step], {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
    windowsHide: true,
  });
  const seconds = (performance.now() - start) / 1000;
  if (result.error) {
    console.error(`Step "${step}" failed to launch: ${result.error.message}`);
    process.exit(1);
  }
  if (result.status !== 0) {
    console.error(
      `\nverify (${fast ? 'fast' : 'full'}) failed at "${step}" after ${seconds.toFixed(1)}s.`,
    );
    process.exit(result.status ?? 1);
  }
  summary.push({ step, seconds });
}

const totalSeconds = (performance.now() - totalStart) / 1000;
console.log(`\n=== verify (${fast ? 'fast' : 'full'}) passed in ${totalSeconds.toFixed(1)}s ===`);
for (const item of summary) {
  console.log(`  ${item.step.padEnd(20)} ${item.seconds.toFixed(1)}s`);
}
