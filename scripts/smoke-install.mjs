import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const rootPackageDir = process.env.ROOT_PACKAGE_DIR;
const nativePackageDir = process.env.NATIVE_PACKAGE_DIR;
const artifactPath = process.env.ARTIFACT_PATH;
const expectedTopHit = process.env.EXPECTED_TOP_HIT ?? 'guides/rust.md';

if (!rootPackageDir || !nativePackageDir || !artifactPath) {
  throw new Error('ROOT_PACKAGE_DIR, NATIVE_PACKAGE_DIR, and ARTIFACT_PATH are required');
}

const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const nodeCommand = process.execPath;
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'indexbind-smoke-'));
const packDir = path.join(tempDir, 'packs');

fs.mkdirSync(packDir, { recursive: true });

const rootTarball = pack(rootPackageDir, packDir);
const nativeTarball = pack(nativePackageDir, packDir);

run(npmCommand, ['init', '-y'], tempDir);
run(npmCommand, ['install', rootTarball, nativeTarball], tempDir);

const docsDir = path.join(tempDir, 'docs');
fs.mkdirSync(docsDir, { recursive: true });
fs.writeFileSync(
  path.join(docsDir, 'rust.md'),
  '# Rust Guide\n\nRust retrieval guide for local search.\n',
);

const cliArtifactPath = path.join(tempDir, 'cli.sqlite');
const buildOutput = capture(
  npmCommand,
  ['exec', '--', 'indexbind', 'build', docsDir, cliArtifactPath, 'hashing'],
  tempDir,
);
const buildStats = JSON.parse(buildOutput);
if (buildStats.documentCount !== 1 || buildStats.chunkCount < 1) {
  throw new Error(`Unexpected build stats: ${buildOutput}`);
}

const inspectOutput = capture(
  npmCommand,
  ['exec', '--', 'indexbind', 'inspect', cliArtifactPath],
  tempDir,
);
const inspectInfo = JSON.parse(inspectOutput);
if (inspectInfo.documentCount !== 1) {
  throw new Error(`Unexpected inspect output: ${inspectOutput}`);
}

const vectorSearchOutput = capture(
  npmCommand,
  ['exec', '--', 'indexbind', 'search', cliArtifactPath, 'rust guide', '--mode', 'vector', '--top-k', '3'],
  tempDir,
);
const vectorSearchResult = JSON.parse(vectorSearchOutput);
if (
  vectorSearchResult.options?.mode !== 'vector' ||
  vectorSearchResult.hitCount !== 1 ||
  vectorSearchResult.hits[0]?.relativePath !== 'rust.md'
) {
  throw new Error(`Unexpected vector CLI search output: ${vectorSearchOutput}`);
}

const lexicalSearchOutput = capture(
  npmCommand,
  ['exec', '--', 'indexbind', 'search', cliArtifactPath, 'rust guide', '--mode', 'lexical', '--top-k', '3'],
  tempDir,
);
const lexicalSearchResult = JSON.parse(lexicalSearchOutput);
if (
  lexicalSearchResult.options?.mode !== 'lexical' ||
  lexicalSearchResult.hitCount !== 1 ||
  lexicalSearchResult.hits[0]?.relativePath !== 'rust.md'
) {
  throw new Error(`Unexpected lexical CLI search output: ${lexicalSearchOutput}`);
}

const helpResult = spawnSync(
  npmCommand,
  ['exec', '--', 'indexbind', 'search', '--help'],
  {
    cwd: tempDir,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: {
      ...process.env,
      NPM_CONFIG_LOGLEVEL: 'silent',
    },
    encoding: 'utf8',
  },
);

if (helpResult.status !== 0 || !`${helpResult.stdout}${helpResult.stderr}`.includes('indexbind search <artifact-file> <query>')) {
  throw new Error(`Unexpected CLI help output: ${helpResult.stdout}${helpResult.stderr}`);
}

const verifyScript = path.join(tempDir, 'verify.mjs');
fs.writeFileSync(
  verifyScript,
  `
import { openIndex } from 'indexbind';

const index = await openIndex(${JSON.stringify(artifactPath)});
const hits = await index.search('rust guide', {
  reranker: { candidatePoolSize: 25 },
});

if (!hits[0]) {
  throw new Error('No hits returned from smoke test query');
}

if (hits[0].relativePath !== ${JSON.stringify(expectedTopHit)}) {
  throw new Error(\`Expected top hit ${expectedTopHit}, received \${hits[0].relativePath}\`);
}

console.log(JSON.stringify({
  topHit: hits[0].relativePath,
  score: hits[0].score,
}, null, 2));
`,
);

run(nodeCommand, [verifyScript], tempDir);

function pack(packageDir, destination) {
  const result = spawnSync(
    npmCommand,
    ['pack', '.', '--pack-destination', destination],
    {
      cwd: packageDir,
      stdio: ['ignore', 'pipe', 'inherit'],
      env: process.env,
      encoding: 'utf8',
    },
  );

  if (result.status !== 0) {
    throw new Error(`Failed to pack ${packageDir}`);
  }

  const tarball = result.stdout
    .trim()
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .at(-1);

  if (!tarball) {
    throw new Error(`Could not determine tarball name for ${packageDir}`);
  }

  return path.join(destination, tarball);
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: 'inherit',
    env: process.env,
  });

  if (result.status !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(' ')}`);
  }
}

function capture(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: ['ignore', 'pipe', 'inherit'],
    env: {
      ...process.env,
      ...(command === npmCommand ? { NPM_CONFIG_LOGLEVEL: 'silent' } : {}),
    },
    encoding: 'utf8',
  });

  if (result.status !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(' ')}`);
  }

  return result.stdout.trim();
}
