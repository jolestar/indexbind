import fs from 'node:fs';
import path from 'node:path';
import { OPTIONAL_DEPENDENCIES, ROOT_PACKAGE_NAME } from './release-targets.mjs';

const root = process.cwd();
const packageJson = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const version = process.env.RELEASE_VERSION ?? packageJson.version;
const outputDir = process.env.RELEASE_ROOT_DIR
  ? path.resolve(process.env.RELEASE_ROOT_DIR)
  : path.resolve('release', 'npm', ROOT_PACKAGE_NAME);

fs.rmSync(outputDir, { recursive: true, force: true });
fs.mkdirSync(path.join(outputDir, 'dist', 'wasm'), { recursive: true });
fs.mkdirSync(path.join(outputDir, 'dist', 'wasm-bundler'), { recursive: true });

for (const relativePath of [
  'dist/index.js',
  'dist/index.d.ts',
  'dist/native.js',
  'dist/native.d.ts',
  'dist/build.js',
  'dist/build.d.ts',
  'dist/web.js',
  'dist/web.d.ts',
  'dist/cloudflare.js',
  'dist/cloudflare.d.ts',
  'dist/wasm/indexbind_wasm.js',
  'dist/wasm/indexbind_wasm.d.ts',
  'dist/wasm/indexbind_wasm_bg.wasm',
  'dist/wasm/indexbind_wasm_bg.wasm.d.ts',
  'dist/wasm-bundler/indexbind_wasm.js',
  'dist/wasm-bundler/indexbind_wasm.d.ts',
  'dist/wasm-bundler/indexbind_wasm_bg.js',
  'dist/wasm-bundler/indexbind_wasm_bg.wasm',
  'dist/wasm-bundler/indexbind_wasm_bg.wasm.d.ts',
  'README.md',
  'LICENSE',
  'CHANGELOG.md',
]) {
  fs.copyFileSync(path.join(root, relativePath), path.join(outputDir, relativePath));
}

const publishPackageJson = {
  name: packageJson.name,
  version,
  description: packageJson.description,
  type: packageJson.type,
  main: packageJson.main,
  types: packageJson.types,
  exports: packageJson.exports,
  repository: packageJson.repository,
  homepage: packageJson.homepage,
  bugs: packageJson.bugs,
  license: packageJson.license,
  engines: packageJson.engines,
  publishConfig: packageJson.publishConfig,
  dependencies: packageJson.dependencies,
  optionalDependencies: Object.fromEntries(
    Object.keys(OPTIONAL_DEPENDENCIES).map((name) => [name, version]),
  ),
};

fs.writeFileSync(
  path.join(outputDir, 'package.json'),
  `${JSON.stringify(publishPackageJson, null, 2)}\n`,
);

console.log(`Prepared root package at ${outputDir}`);
