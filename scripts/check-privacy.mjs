import { spawnSync } from 'node:child_process';
import { lstatSync, readFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const email = /[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/gi;
const homePath = /\b[A-Z]:[\\/]+Users[\\/]+[^\\/\s"'<>|`]+|\/(?:Users|home)\/[^/\s"'<>|`]+/gi;
const privateChat = /https?:\/\/(?:chatgpt\.com|chat\.openai\.com)\/(?:c|g)\/[A-Z0-9/_-]+/gi;

function exampleEmail(address) {
  const domain = address.slice(address.lastIndexOf('@') + 1).toLowerCase();
  return /(?:^|\.)(?:example\.(?:com|org|net)|invalid|test)$/.test(domain)
    || domain === 'users.noreply.github.com';
}

export function sensitiveFilename(path) {
  const name = basename(path.replaceAll('\\', '/'));
  const template = /^\.env(?:\.[\w-]+)*\.(?:example|sample|template)$/i.test(name);
  return (/^\.env(?:\..+)?$/i.test(name) && !template)
    || /\.(?:pem|key|p12|pfx|kdbx)$/i.test(name)
    || /^id_(?:rsa|dsa|ecdsa|ed25519)(?:\.(?!pub$).+)?$/i.test(name)
    || /^(?:credentials(?:\.(?:json|ya?ml|ini|toml))?|client_secret[^/]*\.json|service-account[^/]*\.json)$/i.test(name);
}

/** Return locations and rule names only: matched personal information never leaves this function. */
export function inspectText(path, text) {
  const findings = [];
  const rules = [
    ['personal-email', email, match => !exampleEmail(match)],
    ['personal-home-path', homePath],
    ['private-chat-link', privateChat],
  ];
  for (const [index, line] of text.split(/\r?\n/).entries()) {
    for (const [rule, pattern, accept] of rules) {
      const matches = line.match(pattern) ?? [];
      if (matches.some(match => !accept || accept(match))) findings.push({path, line: index + 1, rule});
    }
  }
  return findings;
}

export function scanProject(directory = root) {
  const listed = spawnSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '-z'], {
    cwd: directory, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, windowsHide: true,
  });
  if (listed.error) throw listed.error;
  if (listed.status !== 0) throw new Error('Cannot enumerate project files with git ls-files.');
  const findings = [];
  const paths = [...new Set(listed.stdout.split('\0').filter(Boolean))];
  for (const path of paths) {
    const file = join(directory, path);
    let metadata;
    try { metadata = lstatSync(file); }
    catch (error) { if (error.code === 'ENOENT') continue; throw error; } // Unstaged deletion.
    if (sensitiveFilename(path)) findings.push({path, line: 1, rule: 'sensitive-file'});
    if (metadata.isSymbolicLink()) {
      findings.push({path, line: 1, rule: 'unscanned-symlink'});
      continue;
    }
    if (!metadata.isFile()) throw new Error(`Cannot scan project entry ${JSON.stringify(path)}.`);
    const bytes = readFileSync(file);
    if (!bytes.includes(0)) findings.push(...inspectText(path, bytes.toString('utf8')));
  }
  return {findings, files: paths.length};
}

export function formatFindings(findings) {
  return findings.map(({path, line, rule}) => `- ${JSON.stringify(path)}:${line}: ${rule}`).join('\n');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const {findings, files} = scanProject();
    if (findings.length) {
      console.error(`Privacy check found ${findings.length} issue(s). Matched values are hidden.\n${formatFindings(findings)}`);
      process.exitCode = 1;
    } else console.log(`Privacy check passed (${files} project files considered).`);
  } catch (error) {
    console.error(`Privacy check failed: ${error.message}`);
    process.exitCode = 2;
  }
}
