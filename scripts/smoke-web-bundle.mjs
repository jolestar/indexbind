import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawnSync } from 'node:child_process';

const repoRoot = process.cwd();
const nodeCommand = process.execPath;
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'indexbind-web-bundle-'));
const fixtureDocs = path.join(repoRoot, 'fixtures/benchmark/basic/docs');
const expectedTopHit = 'guides/rust.md';

ensureBuiltArtifacts();

const cases = [
  {
    name: 'hashing',
    backendArg: 'hashing',
    bundleDir: path.join(tempDir, 'hashing.bundle'),
  },
  {
    name: 'model2vec',
    backendArg: 'minishlab/potion-base-2M',
    bundleDir: path.join(tempDir, 'model2vec.bundle'),
  },
];

for (const testCase of cases) {
  run(
    nodeCommand,
    [
      path.join(repoRoot, 'dist/cli.js'),
      'build-bundle',
      fixtureDocs,
      testCase.bundleDir,
      testCase.backendArg,
    ],
    repoRoot,
  );

  run(
    nodeCommand,
    [
      '--input-type=module',
      '-e',
      `
import { openWebIndex } from ${JSON.stringify(pathToFileUrl(path.join(repoRoot, 'dist/web.js')))};

const index = await openWebIndex(${JSON.stringify(testCase.bundleDir)});
const hits = await index.search('rust guide');

if (!hits[0]) {
  throw new Error(${JSON.stringify(`[${testCase.name}] expected at least one hit`)});
}

if (hits[0].relativePath !== ${JSON.stringify(expectedTopHit)}) {
  throw new Error(
    ${JSON.stringify(`[${testCase.name}] expected top hit ${expectedTopHit}, got `)} + hits[0].relativePath,
  );
}

console.log(JSON.stringify({
  case: ${JSON.stringify(testCase.name)},
  topHit: hits[0].relativePath,
  score: hits[0].score,
}, null, 2));
`,
    ],
    repoRoot,
  );

  run(
    nodeCommand,
    [
      '--input-type=module',
      '-e',
      `
import fs from 'node:fs/promises';
import path from 'node:path';
import { openWebIndex } from ${JSON.stringify(pathToFileUrl(path.join(repoRoot, 'dist/web.js')))};

const bundleDir = ${JSON.stringify(testCase.bundleDir)};
const requestedPaths = [];
const index = await openWebIndex('https://bundle.invalid/index.bundle/', {
  modeProfile: 'lexical',
  fetch: async (input) => {
    const url = new URL(typeof input === 'string' ? input : input instanceof URL ? input.href : input.url);
    requestedPaths.push(url.pathname);
    const relativePath = url.pathname.replace(/^\\/index\\.bundle\\//, '');
    const filePath = path.join(bundleDir, relativePath);
    const body = await fs.readFile(filePath);
    return new Response(body);
  },
});
const hits = await index.search('rust guide');
if (!hits[0] || hits[0].relativePath !== ${JSON.stringify(expectedTopHit)}) {
  throw new Error(${JSON.stringify(`[${testCase.name}] expected lexical profile top hit ${expectedTopHit}`)});
}
if (requestedPaths.some((value) => value.endsWith('/vectors.bin') || value.includes('/model/'))) {
  throw new Error(${JSON.stringify(`[${testCase.name}] lexical profile should not load vectors or model files: `)} + JSON.stringify(requestedPaths));
}
console.log(JSON.stringify({
  case: ${JSON.stringify(`${testCase.name}-lexical-profile`)},
  topHit: hits[0].relativePath,
  requests: requestedPaths,
}, null, 2));
`,
    ],
    repoRoot,
  );
}

function ensureBuiltArtifacts() {
  const requiredFiles = [
    path.join(repoRoot, 'dist/cli.js'),
    path.join(repoRoot, 'dist/web.js'),
    path.join(repoRoot, 'dist/wasm/indexbind_wasm.js'),
    path.join(repoRoot, 'dist/wasm/indexbind_wasm_bg.wasm'),
  ];

  for (const file of requiredFiles) {
    if (!fs.existsSync(file)) {
      throw new Error(`Missing built artifact: ${file}. Run npm run build first.`);
    }
  }
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

function pathToFileUrl(filePath) {
  return pathToFileURL(filePath).href;
}
