import fs from 'node:fs';
import path from 'node:path';
import { ROOT_PACKAGE_NAME } from './release-targets.mjs';

const packageDir = process.env.RELEASE_ROOT_DIR
  ? path.resolve(process.env.RELEASE_ROOT_DIR)
  : path.resolve('release', 'npm', ROOT_PACKAGE_NAME);

const requiredFiles = [
  'package.json',
  'README.md',
  'LICENSE',
  'CHANGELOG.md',
  'dist/index.js',
  'dist/index.d.ts',
  'dist/build.js',
  'dist/build.d.ts',
  'dist/native.js',
  'dist/native.d.ts',
  'dist/web.js',
  'dist/web.d.ts',
  'dist/web-core.js',
  'dist/web-core.d.ts',
  'dist/cloudflare.js',
  'dist/cloudflare.d.ts',
  'dist/cloudflare/worker.mjs',
  'dist/wasm/indexbind_wasm.js',
  'dist/wasm/indexbind_wasm.d.ts',
  'dist/wasm/indexbind_wasm_bg.wasm',
  'dist/wasm/indexbind_wasm_bg.wasm.d.ts',
  'dist/wasm-bundler/indexbind_wasm.js',
  'dist/wasm-bundler/indexbind_wasm.d.ts',
  'dist/wasm-bundler/indexbind_wasm_bg.js',
  'dist/wasm-bundler/indexbind_wasm_bg.wasm',
  'dist/wasm-bundler/indexbind_wasm_bg.wasm.d.ts',
];

for (const relativePath of requiredFiles) {
  const absolutePath = path.join(packageDir, relativePath);
  if (!fs.existsSync(absolutePath)) {
    throw new Error(`Prepared root package is missing required file: ${relativePath}`);
  }
}

for (const relativePath of collectFiles(path.join(packageDir, 'dist'))) {
  if (!relativePath.endsWith('.js') && !relativePath.endsWith('.d.ts') && !relativePath.endsWith('.mjs')) {
    continue;
  }

  const absolutePath = path.join(packageDir, relativePath);
  const content = fs.readFileSync(absolutePath, 'utf8');

  for (const specifier of collectRelativeSpecifiers(content)) {
    const target = path.resolve(path.dirname(absolutePath), specifier);
    if (!fs.existsSync(target)) {
      throw new Error(
        `Prepared root package import target is missing: ${relativePath} -> ${specifier}`,
      );
    }
  }
}

console.log(`Verified prepared root package at ${packageDir}`);

function collectFiles(directory) {
  const entries = fs.readdirSync(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const absolutePath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectFiles(absolutePath));
      continue;
    }
    files.push(path.relative(packageDir, absolutePath));
  }

  return files;
}

function collectRelativeSpecifiers(content) {
  const specifiers = new Set();
  const patterns = [
    /\bfrom\s+['"](\.{1,2}\/[^'"]+)['"]/g,
    /\bimport\s*\(\s*['"](\.{1,2}\/[^'"]+)['"]\s*\)/g,
  ];

  for (const pattern of patterns) {
    for (const match of content.matchAll(pattern)) {
      specifiers.add(match[1]);
    }
  }

  return [...specifiers];
}
