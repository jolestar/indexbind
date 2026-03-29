import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { unstable_startWorker } from 'wrangler';

const repoRoot = process.cwd();
const cargoCommand = 'cargo';
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'indexbind-cf-worker-'));
const fixtureDocs = path.join(repoRoot, 'fixtures/benchmark/basic/docs');
const expectedTopHit = 'guides/rust.md';
const localTempRoot = fs.mkdtempSync(path.join(repoRoot, '.tmp-indexbind-cf-worker-'));

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

const requestModes = [
  {
    name: 'direct-http',
    headers(testCase, baseUrl) {
      return {
        'x-bundle-base-url': `${baseUrl}/${path.basename(testCase.bundleDir)}/`,
      };
    },
  },
  {
    name: 'virtual-bundle',
    headers(testCase, baseUrl) {
      return {
        'x-bundle-base-url': `https://indexbind-smoke.invalid/${path.basename(testCase.bundleDir)}/`,
        'x-bundle-backing-base-url': `${baseUrl}/${path.basename(testCase.bundleDir)}/`,
      };
    },
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

let worker;
try {
  const address = server.address();
  if (!address || typeof address === 'string') {
    throw new Error('Failed to resolve Cloudflare worker smoke server address');
  }

  const baseUrl = `http://127.0.0.1:${address.port}`;
  const workerDir = createWorkerFixture(repoRoot, localTempRoot);
  worker = await unstable_startWorker({
    config: path.join(workerDir, 'wrangler.jsonc'),
    entrypoint: path.join(workerDir, 'index.ts'),
    compatibilityDate: '2026-03-24',
    dev: {
      remote: false,
      inspector: false,
      persist: false,
    },
  });
  await worker.ready;

  for (const testCase of cases) {
    for (const requestMode of requestModes) {
      const response = await withTimeout(
        worker.fetch(
          `https://indexbind-smoke.example/search?case=${encodeURIComponent(
            testCase.name,
          )}&mode=${encodeURIComponent(requestMode.name)}`,
          {
            headers: requestMode.headers(testCase, baseUrl),
          },
        ),
        20000,
        `Cloudflare worker request timed out for case ${testCase.name} (${requestMode.name})`,
      );
      const result = await response.json();

      if (!response.ok) {
        throw new Error(`[${testCase.name}/${requestMode.name}] ${result.error ?? response.statusText}`);
      }
      if (!result.topHit) {
        throw new Error(`[${testCase.name}/${requestMode.name}] expected at least one hit`);
      }
      if (result.topHit !== expectedTopHit) {
        throw new Error(
          `[${testCase.name}/${requestMode.name}] expected top hit ${expectedTopHit}, got ${result.topHit}`,
        );
      }

      console.log(
        JSON.stringify(
          {
            case: testCase.name,
            mode: requestMode.name,
            runtime: 'cloudflare-worker',
            topHit: result.topHit,
            score: result.score,
          },
          null,
          2,
        ),
      );
    }
  }
} finally {
  if (worker) {
    await worker.dispose();
  }
  await new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
  fs.rmSync(localTempRoot, { recursive: true, force: true });
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

function createWorkerFixture(repoRoot, tempRoot) {
  const workerDir = path.join(tempRoot, 'worker');
  fs.mkdirSync(workerDir, { recursive: true });

  const relativeWebModule = path
    .relative(workerDir, path.join(repoRoot, 'dist/cloudflare.js'))
    .replaceAll(path.sep, '/');
  const webModuleImport = relativeWebModule.startsWith('.')
    ? relativeWebModule
    : `./${relativeWebModule}`;

  fs.writeFileSync(
    path.join(workerDir, 'index.ts'),
    `import { openWebIndex } from ${JSON.stringify(webModuleImport)};

export default {
  async fetch(request: Request) {
    try {
      const bundleBaseUrl = request.headers.get('x-bundle-base-url');
      const backingBaseUrl = request.headers.get('x-bundle-backing-base-url');
      if (!bundleBaseUrl) {
        return Response.json({ error: 'missing bundle base url header' }, { status: 400 });
      }

      const index = backingBaseUrl
        ? await openVirtualBundleIndex(bundleBaseUrl, backingBaseUrl)
        : await openWebIndex(bundleBaseUrl);
      const hits = await index.search('rust guide');
      return Response.json({
        topHit: hits[0]?.relativePath,
        score: hits[0]?.score,
        count: hits.length,
      });
    } catch (error) {
      return Response.json(
        {
          error: error instanceof Error ? (error.stack ?? error.message) : String(error),
        },
        { status: 500 },
      );
    }
  },
};

async function openVirtualBundleIndex(bundleBaseUrl: string, backingBaseUrl: string) {
  const customFetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    const requestUrl =
      typeof input === 'string'
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    if (requestUrl.startsWith(bundleBaseUrl)) {
      const relativePath = requestUrl.slice(bundleBaseUrl.length);
      const rewrittenUrl = new URL(relativePath, backingBaseUrl);
      if (typeof input === 'string' || input instanceof URL) {
        return fetch(rewrittenUrl, init);
      }

      return fetch(new Request(rewrittenUrl, input), init);
    }

    return fetch(input as RequestInfo, init);
  };
  return await openWebIndex(new URL(bundleBaseUrl), { fetch: customFetch });
}
`,
  );

  fs.writeFileSync(
    path.join(workerDir, 'wrangler.jsonc'),
    JSON.stringify(
      {
        name: 'indexbind-smoke-worker',
        main: './index.ts',
        compatibility_date: '2026-03-24',
      },
      null,
      2,
    ),
  );

  return workerDir;
}

function createSmokeServer(repoRoot, bundleRoot) {
  return http.createServer((request, response) => {
    const requestUrl = new URL(request.url, 'http://127.0.0.1');

    if (requestUrl.pathname.startsWith('/dist/')) {
      serveFile(response, path.join(repoRoot, requestUrl.pathname.slice(1)));
      return;
    }

    if (requestUrl.pathname.startsWith('/node_modules/')) {
      serveFile(response, path.join(repoRoot, requestUrl.pathname.slice(1)));
      return;
    }

    if (requestUrl.pathname.startsWith('/')) {
      const relativePath = requestUrl.pathname.slice(1);
      serveFile(response, path.join(bundleRoot, relativePath));
      return;
    }

    response.writeHead(404).end('not found');
  });
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

function withTimeout(promise, timeoutMs, message) {
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      setTimeout(() => reject(new Error(message)), timeoutMs);
    }),
  ]);
}
