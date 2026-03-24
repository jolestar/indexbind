import fs from 'node:fs';
import path from 'node:path';
import { getTargetByKey } from './release-targets.mjs';

const targetKey = process.env.NATIVE_TARGET ?? process.argv[2];
if (!targetKey) {
  throw new Error('Usage: node scripts/prepare-native-package.mjs <target-key>');
}

const target = getTargetByKey(targetKey);
const version = process.env.RELEASE_VERSION ?? '0.1.0';
const sourceBinary = process.env.NATIVE_BINARY
  ? path.resolve(process.env.NATIVE_BINARY)
  : path.resolve('native', target.artifactName);
const outputDir = process.env.RELEASE_NATIVE_DIR
  ? path.resolve(process.env.RELEASE_NATIVE_DIR)
  : path.resolve('release', 'npm', target.packageName.replace('@', '').replace('/', '__'));

if (!fs.existsSync(sourceBinary)) {
  throw new Error(`Native binary not found: ${sourceBinary}`);
}

fs.rmSync(outputDir, { recursive: true, force: true });
fs.mkdirSync(outputDir, { recursive: true });
fs.copyFileSync(sourceBinary, path.join(outputDir, target.artifactName));
fs.copyFileSync(path.resolve('README.md'), path.join(outputDir, 'README.md'));
fs.copyFileSync(path.resolve('LICENSE'), path.join(outputDir, 'LICENSE'));

const packageJson = {
  name: target.packageName,
  version,
  description: `Prebuilt native addon for indexbind on ${target.key}.`,
  repository: {
    type: 'git',
    url: 'https://github.com/jolestar/indexbind.git',
  },
  homepage: 'https://github.com/jolestar/indexbind#readme',
  bugs: {
    url: 'https://github.com/jolestar/indexbind/issues',
  },
  os: [target.os],
  cpu: [target.arch],
  files: [target.artifactName, 'README.md', 'LICENSE'],
  license: 'MIT',
  publishConfig: {
    access: 'public',
  },
  main: target.artifactName,
};

fs.writeFileSync(
  path.join(outputDir, 'package.json'),
  `${JSON.stringify(packageJson, null, 2)}\n`,
);

console.log(`Prepared native package ${target.packageName} at ${outputDir}`);
