import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const repoRoot = process.cwd();
const nodeCommand = process.execPath;
const cliPath = path.join(repoRoot, 'dist/cli.js');

ensureBuiltArtifacts();

assertHelp(['--help'], 'usage:');
assertHelp(['build', '--help'], 'indexbind build <input-dir> <output-file>');
assertHelp(['build-bundle', '--help'], 'indexbind build-bundle <input-dir> <output-dir>');
assertHelp(['update-cache', '--help'], 'indexbind update-cache <input-dir> <cache-file>');
assertHelp(['export-artifact', '--help'], 'indexbind export-artifact <cache-file> <output-file>');
assertHelp(['export-bundle', '--help'], 'indexbind export-bundle <cache-file> <output-dir>');
assertHelp(['inspect', '--help'], 'indexbind inspect <artifact-file>');
assertHelp(['benchmark', '--help'], 'indexbind benchmark <artifact-file> <queries-json>');
assertHelp(['search', '--help'], 'indexbind search <artifact-file> <query>');

const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'indexbind-cli-smoke-'));
const docsDir = path.join(tempDir, 'docs');
const artifactPath = path.join(tempDir, 'index.sqlite');
fs.mkdirSync(docsDir, { recursive: true });
fs.writeFileSync(path.join(docsDir, 'rust.md'), '# Rust Guide\n\nRust retrieval guide for local search.\n');

runCli(['build', docsDir, artifactPath, 'hashing']);
const searchResult = JSON.parse(captureCli(['search', artifactPath, '--', '--help']));
if (searchResult.query !== '--help') {
  throw new Error(`Expected literal query --help, got ${JSON.stringify(searchResult)}`);
}

assertFailure([], 'usage:');

console.log('CLI help smoke passed');

function ensureBuiltArtifacts() {
  if (!fs.existsSync(cliPath)) {
    throw new Error(`Missing built CLI: ${cliPath}. Run npm run build first.`);
  }
}

function assertHelp(args, expectedText) {
  const result = spawnSync(nodeCommand, [cliPath, ...args], {
    cwd: repoRoot,
    stdio: ['ignore', 'pipe', 'pipe'],
    encoding: 'utf8',
  });

  if (result.status !== 0) {
    throw new Error(`Expected help to exit 0 for ${args.join(' ') || '<no-args>'}`);
  }
  const output = `${result.stdout}${result.stderr}`;
  if (!output.includes(expectedText)) {
    throw new Error(`Expected help output for ${args.join(' ')} to include ${expectedText}`);
  }
}

function runCli(args) {
  const result = spawnSync(nodeCommand, [cliPath, ...args], {
    cwd: repoRoot,
    stdio: 'inherit',
    encoding: 'utf8',
  });

  if (result.status !== 0) {
    throw new Error(`Command failed: ${args.join(' ')}`);
  }
}

function captureCli(args) {
  const result = spawnSync(nodeCommand, [cliPath, ...args], {
    cwd: repoRoot,
    stdio: ['ignore', 'pipe', 'inherit'],
    encoding: 'utf8',
  });

  if (result.status !== 0) {
    throw new Error(`Command failed: ${args.join(' ')}`);
  }

  return result.stdout.trim();
}

function assertFailure(args, expectedText) {
  const result = spawnSync(nodeCommand, [cliPath, ...args], {
    cwd: repoRoot,
    stdio: ['ignore', 'pipe', 'pipe'],
    encoding: 'utf8',
  });

  if (result.status === 0) {
    throw new Error(`Expected failure for ${args.join(' ') || '<no-args>'}`);
  }
  const output = `${result.stdout}${result.stderr}`;
  if (!output.includes(expectedText)) {
    throw new Error(`Expected failure output for ${args.join(' ') || '<no-args>'} to include ${expectedText}`);
  }
}
