import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

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
const vectorSearchResult = JSON.parse(
  captureCli(['search', artifactPath, 'rust guide', '--mode', 'vector', '--top-k', '1']),
);
if (vectorSearchResult.options?.mode !== 'vector') {
  throw new Error(`Expected vector mode search output, got ${JSON.stringify(vectorSearchResult)}`);
}

const lexicalSearchResult = JSON.parse(
  captureCli(['search', artifactPath, 'rust guide', '--mode', 'lexical', '--top-k', '1']),
);
if (lexicalSearchResult.options?.mode !== 'lexical') {
  throw new Error(`Expected lexical mode search output, got ${JSON.stringify(lexicalSearchResult)}`);
}

const literalQueryResult = JSON.parse(captureCli(['search', artifactPath, '--', '--help']));
if (literalQueryResult.query !== '--help') {
  throw new Error(`Expected literal query --help, got ${JSON.stringify(literalQueryResult)}`);
}

assertFailure(
  ['search', artifactPath, 'rust guide', '--hybrid', 'true'],
  'The --hybrid flag has been removed.',
);

if (lexicalSearchResult.query !== 'rust guide') {
  throw new Error(`Expected search query rust guide, got ${JSON.stringify(lexicalSearchResult)}`);
}

const { openIndex } = await import(pathToFileURL(path.join(repoRoot, 'dist/index.js')).href);
const index = await openIndex(artifactPath);
const lexicalHits = await index.search('rust guide', { mode: 'lexical' });
if (!lexicalHits[0] || lexicalHits[0].relativePath !== 'rust.md') {
  throw new Error(`Expected lexical mode API search hit, got ${JSON.stringify(lexicalHits)}`);
}
const apiHits = await index.search('rust guide', { mode: 'vector' });
if (!apiHits[0] || apiHits[0].relativePath !== 'rust.md') {
  throw new Error(`Expected vector mode API search hit, got ${JSON.stringify(apiHits)}`);
}

const lexicalProfileIndex = await openIndex(artifactPath, { modeProfile: 'lexical' });
const lexicalProfileHits = await lexicalProfileIndex.search('rust guide');
if (!lexicalProfileHits[0] || lexicalProfileHits[0].relativePath !== 'rust.md') {
  throw new Error(
    `Expected lexical profile API search hit, got ${JSON.stringify(lexicalProfileHits)}`,
  );
}
let sawLexicalProfileModeError = false;
try {
  await lexicalProfileIndex.search('rust guide', { mode: 'vector' });
} catch (error) {
  if (
    error instanceof Error &&
    error.message.includes('this index was opened with modeProfile: "lexical"')
  ) {
    sawLexicalProfileModeError = true;
  } else {
    throw error;
  }
}
if (!sawLexicalProfileModeError) {
  throw new Error('Expected lexical profile to reject vector mode');
}

let sawLegacyHybridError = false;
try {
  await index.search('rust guide', { hybrid: true });
} catch (error) {
  if (
    error instanceof Error &&
    error.message.includes('Search option "hybrid" has been removed.')
  ) {
    sawLegacyHybridError = true;
  } else {
    throw error;
  }
}
if (!sawLegacyHybridError) {
  throw new Error('Expected Node API to reject the legacy hybrid option');
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
