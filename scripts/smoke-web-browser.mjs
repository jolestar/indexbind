import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { chromium } from '@playwright/test';

const repoRoot = process.cwd();
const cargoCommand = 'cargo';
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'indexbind-web-browser-'));
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
    cargoCommand,
    [
      'run',
      '-p',
      'indexbind-build',
      '--',
      'build-bundle',
      fixtureDocs,
      testCase.bundleDir,
      testCase.backendArg,
    ],
    repoRoot,
  );
}

const server = createSmokeServer(repoRoot, tempDir);
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));

const browser = await chromium.launch({ headless: true });
try {
  const address = server.address();
  if (!address || typeof address === 'string') {
    throw new Error('Failed to resolve browser smoke server address');
  }

  const baseUrl = `http://127.0.0.1:${address.port}`;
  const page = await browser.newPage();
  page.on('console', (message) => {
    console.log(`[browser:${message.type()}] ${message.text()}`);
  });
  page.on('pageerror', (error) => {
    console.error(`[browser:error] ${error.stack ?? error.message}`);
  });

  for (const testCase of cases) {
    await page.goto(`${baseUrl}/?case=${encodeURIComponent(testCase.name)}`, {
      waitUntil: 'networkidle',
    });
    await page.waitForFunction(() => window.__indexbindResult !== undefined, undefined, {
      timeout: 30000,
    });
    const result = await page.evaluate(() => window.__indexbindResult);
    if (!result?.ok) {
      throw new Error(`[${testCase.name}] ${result?.error ?? 'browser smoke failed'}`);
    }
    if (result.topHit !== expectedTopHit) {
      throw new Error(
        `[${testCase.name}] expected top hit ${expectedTopHit}, got ${result.topHit}`,
      );
    }
    console.log(JSON.stringify({
      case: testCase.name,
      runtime: 'browser',
      topHit: result.topHit,
      score: result.score,
    }, null, 2));
  }
} finally {
  await browser.close();
  await new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
}

function ensureBuiltArtifacts() {
  const requiredFiles = [
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

function createSmokeServer(repoRoot, bundleRoot) {
  return http.createServer((request, response) => {
    const requestUrl = new URL(request.url, 'http://127.0.0.1');

    if (requestUrl.pathname === '/') {
      response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
      response.end(renderSmokePage(requestUrl.searchParams.get('case') ?? 'hashing'));
      return;
    }

    if (requestUrl.pathname.startsWith('/dist/')) {
      serveFile(response, path.join(repoRoot, requestUrl.pathname.slice(1)));
      return;
    }

    if (requestUrl.pathname.startsWith('/node_modules/')) {
      serveFile(response, path.join(repoRoot, requestUrl.pathname.slice(1)));
      return;
    }

    if (requestUrl.pathname.startsWith('/bundles/')) {
      const relativePath = requestUrl.pathname.slice('/bundles/'.length);
      serveFile(response, path.join(bundleRoot, relativePath));
      return;
    }

    response.writeHead(404).end('not found');
  });
}

function renderSmokePage(caseName) {
  const bundleUrl = `/bundles/${caseName}.bundle/`;
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>indexbind browser smoke</title>
    <script type="importmap">
      {
        "imports": {
          "@noble/hashes/blake3.js": "/node_modules/@noble/hashes/blake3.js"
        }
      }
    </script>
  </head>
  <body>
    <script type="module">
      import { openWebIndex } from '/dist/web.js';

      try {
        const index = await openWebIndex(${JSON.stringify(bundleUrl)});
        const hits = await index.search('rust guide');
        window.__indexbindResult = {
          ok: true,
          topHit: hits[0]?.relativePath,
          score: hits[0]?.score,
        };
      } catch (error) {
        window.__indexbindResult = {
          ok: false,
          error: error instanceof Error ? (error.stack ?? error.message) : String(error),
        };
      }
    </script>
  </body>
</html>`;
}

function serveFile(response, filePath) {
  const normalized = path.normalize(filePath);
  if (!fs.existsSync(normalized) || fs.statSync(normalized).isDirectory()) {
    response.writeHead(404).end('not found');
    return;
  }

  response.writeHead(200, {
    'content-type': contentTypeFor(normalized),
    'cache-control': 'no-store',
  });
  fs.createReadStream(normalized).pipe(response);
}

function contentTypeFor(filePath) {
  if (filePath.endsWith('.html')) return 'text/html; charset=utf-8';
  if (filePath.endsWith('.js')) return 'text/javascript; charset=utf-8';
  if (filePath.endsWith('.json')) return 'application/json';
  if (filePath.endsWith('.wasm')) return 'application/wasm';
  if (filePath.endsWith('.bin') || filePath.endsWith('.safetensors')) return 'application/octet-stream';
  return 'text/plain; charset=utf-8';
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
