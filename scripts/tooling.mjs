import { existsSync } from 'node:fs';
import { delimiter, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

export const root = fileURLToPath(new URL('../', import.meta.url));
export function toolEnvironment() {
  const env = { ...process.env };
  const localCargo = join(root, '.tools', 'cargo');
  const localRustup = join(root, '.tools', 'rustup');
  const paths = [join(root, '.tools', 'bin')];
  if (existsSync(join(localCargo, 'bin'))) {
    env.CARGO_HOME = localCargo;
    env.RUSTUP_HOME = localRustup;
    paths.push(join(localCargo, 'bin'));
  }
  // On Windows the environment key may be "Path" rather than "PATH".
  const pathKey = Object.keys(env).find(key => key.toLowerCase() === 'path') ?? 'PATH';
  env[pathKey] = [...paths, env[pathKey] ?? ''].join(delimiter);
  return env;
}
export function runTool(command, args) {
  const result = spawnSync(command, args, { cwd: root, env: toolEnvironment(), stdio: 'inherit', windowsHide: true });
  if (result.error) {
    console.error(`Cannot run ${command}: ${result.error.message}. See README.md for development prerequisites.`);
    process.exit(1);
  }
  if (result.status !== 0) process.exit(result.status ?? 1);
}
