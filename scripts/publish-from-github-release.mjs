import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const args = process.argv.slice(2);
const dryRun = args.includes('--dry-run');
const tagArg = args.find((arg) => !arg.startsWith('-'));
const tag = process.env.RELEASE_TAG ?? tagArg ?? 'v0.1.0';
const repo = 'jolestar/indexbind';
const releaseDir = fs.mkdtempSync(path.join(os.tmpdir(), 'indexbind-release-publish-'));

const packages = [
  `indexbind-native-darwin-arm64-${tag.slice(1)}.tgz`,
  `indexbind-native-darwin-x64-${tag.slice(1)}.tgz`,
  `indexbind-native-linux-x64-gnu-${tag.slice(1)}.tgz`,
  `indexbind-${tag.slice(1)}.tgz`,
];

run('gh', ['release', 'download', tag, '--repo', repo, '--dir', releaseDir, ...packages.flatMap((name) => ['--pattern', name])]);

for (const tarball of packages) {
  const tarballPath = path.join(releaseDir, tarball);
  if (!fs.existsSync(tarballPath)) {
    throw new Error(`Missing downloaded tarball: ${tarballPath}`);
  }

  const args = ['publish', tarballPath, '--access', 'public'];
  if (dryRun) {
    args.push('--dry-run');
  }

  run('npm', args);
}

console.log(`Published release assets from ${tag}${dryRun ? ' (dry-run)' : ''}`);

function run(command, args) {
  const executable = process.platform === 'win32' && command === 'npm' ? 'npm.cmd' : command;
  const result = spawnSync(executable, args, {
    stdio: 'inherit',
    env: process.env,
  });

  if (result.status !== 0) {
    throw new Error(`Command failed: ${executable} ${args.join(' ')}`);
  }
}
