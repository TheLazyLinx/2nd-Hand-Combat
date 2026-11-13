import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import test from 'node:test';
import { formatFindings, inspectText, scanProject, sensitiveFilename } from '../scripts/check-privacy.mjs';
import { formatSecretFindings } from '../scripts/report-secret-findings.mjs';

const personalEmail = ['person', 'personal-domain.dev'].join('@');
const windowsHome = ['C:', 'Users', 'someone', 'project'].join('\\');
const unixHome = ['', 'home', 'someone', 'project'].join('/');
const macHome = ['', 'Users', 'someone', 'project'].join('/');
const privateChat = ['https:', '', 'chatgpt.com', 'c', 'example-private-conversation'].join('/');

function removeFixture(directory) {
  const target = resolve(directory);
  assert.equal(dirname(target), resolve(tmpdir()));
  assert.match(basename(target), /^combat-privacy-/);
  rmSync(target, {recursive: true, force: true});
}

test('personal emails are reported by location without retaining the address', () => {
  const findings = inspectText('notes.md', `Heading\nContact ${personalEmail}`);
  assert.deepEqual(findings, [{path: 'notes.md', line: 2, rule: 'personal-email'}]);
  assert.ok(!JSON.stringify(findings).includes(personalEmail));
});

test('reserved example addresses and GitHub noreply addresses are permitted', () => {
  const examples = ['dev@example.com', 'dev@sub.example.org', 'dev@example.net', 'dev@example.test', 'dev@users.noreply.github.com'];
  assert.deepEqual(inspectText('examples.md', examples.join('\n')), []);
  assert.equal(inspectText('notes.md', ['dev', 'example.com.other-domain.dev'].join('@')).length, 1);
});

test('Windows, escaped Windows, macOS and Linux home paths are detected', () => {
  for (const path of [windowsHome, windowsHome.replaceAll('\\', '\\\\'), unixHome, macHome]) {
    assert.deepEqual(inspectText('notes.md', path).map(finding => finding.rule), ['personal-home-path']);
  }
  assert.deepEqual(inspectText('build.yml', '$HOME/project\n${USERPROFILE}/project'), []);
});

test('private chat links are blocked while public documentation remains permitted', () => {
  assert.deepEqual(inspectText('notes.md', privateChat), [{path: 'notes.md', line: 1, rule: 'private-chat-link'}]);
  assert.deepEqual(inspectText('notes.md', 'https://github.com/actions/checkout\nhttps://chatgpt.com'), []);
});

test('sensitive filenames are rejected even if empty or binary', () => {
  for (const path of ['.env', 'config/.env.local', 'private.pem', 'private.key', 'cert.p12', 'cert.pfx', 'vault.kdbx', '.ssh/id_rsa', '.ssh/id_ed25519', '.aws/credentials', 'credentials.json', 'service-account-prod.json']) {
    assert.equal(sensitiveFilename(path), true, path);
  }
  for (const path of ['.env.example', '.env.sample', '.env.template', '.env.production.example', '.ssh/id_ed25519.pub', 'credentials.md', 'package-lock.json']) {
    assert.equal(sensitiveFilename(path), false, path);
  }
});

test('Git enumeration scans tracked and new files but excludes ignored local files', () => {
  const directory = mkdtempSync(join(tmpdir(), 'combat-privacy-'));
  try {
    const git = args => {
      const result = spawnSync('git', args, {cwd: directory, encoding: 'utf8', windowsHide: true});
      assert.equal(result.status, 0, result.stderr);
    };
    git(['init', '--quiet']);
    writeFileSync(join(directory, '.gitignore'), 'ignored/\n.env\n');
    writeFileSync(join(directory, 'tracked.md'), 'Initial safe content');
    git(['add', '.gitignore', 'tracked.md']);
    writeFileSync(join(directory, 'tracked.md'), personalEmail);
    writeFileSync(join(directory, 'new.md'), privateChat);
    mkdirSync(join(directory, 'ignored'));
    writeFileSync(join(directory, 'ignored', 'local.md'), personalEmail);
    writeFileSync(join(directory, '.env'), 'Local-only configuration');
    assert.deepEqual(scanProject(directory).findings.map(finding => [finding.path, finding.rule]).sort(), [
      ['new.md', 'private-chat-link'], ['tracked.md', 'personal-email'],
    ]);
    git(['add', '--force', '.env']);
    assert.ok(scanProject(directory).findings.some(finding => finding.path === '.env' && finding.rule === 'sensitive-file'));
  } finally {
    removeFixture(directory);
  }
});

test('a missing Git repository is a failure rather than a successful empty scan', () => {
  const directory = mkdtempSync(join(tmpdir(), 'combat-privacy-not-git-'));
  try { assert.throws(() => scanProject(directory), /Cannot enumerate/); }
  finally { removeFixture(directory); }
});

test('finding output hides values and escapes filenames that could forge log lines', () => {
  const output = formatFindings(inspectText('notes\n::warning::forged.md', personalEmail));
  assert.ok(!output.includes(personalEmail));
  assert.ok(!output.includes('\n'));
  assert.match(output, /personal-email/);
});

test('secret reports print only locations and rules, never matched values or author metadata', () => {
  const output = formatSecretFindings([{
    File: 'settings.txt', StartLine: 3, RuleID: 'generic-api-key',
    Secret: 'sensitive-test-value', Match: 'sensitive-matched-line', Author: 'Private author', Email: personalEmail,
  }]);
  assert.equal(output, '- "settings.txt":3: "generic-api-key"');
  for (const hidden of ['sensitive-test-value', 'sensitive-matched-line', 'Private author', personalEmail]) assert.ok(!output.includes(hidden));
  assert.throws(() => formatSecretFindings({Secret: 'sensitive-test-value'}), /Expected an array/);
  assert.throws(() => formatSecretFindings([{File: 'settings.txt'}]), /Malformed/);
});
