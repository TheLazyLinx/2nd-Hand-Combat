import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Do not publish Gitleaks' Secret, Match, Author, Email, or other report fields. */
export function formatSecretFindings(report) {
  if (!Array.isArray(report)) throw new Error('Expected an array of Gitleaks findings.');
  return report.map(finding => {
    if (typeof finding.File !== 'string' || typeof finding.RuleID !== 'string' || !Number.isInteger(finding.StartLine)) {
      throw new Error('Malformed Gitleaks finding.');
    }
    return `- ${JSON.stringify(finding.File)}:${finding.StartLine}: ${JSON.stringify(finding.RuleID)}`;
  }).join('\n');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const report = JSON.parse(readFileSync(process.argv[2], 'utf8'));
    console.error(`Secret scan found ${report.length} issue(s). Matched values are hidden.\n${formatSecretFindings(report)}`);
  } catch {
    // JSON parser errors can include the original report text, so keep this diagnostic generic.
    console.error('Cannot read a valid Gitleaks report. Report contents were not printed.');
    process.exitCode = 2;
  }
}
