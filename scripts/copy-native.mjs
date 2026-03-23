import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import path from 'node:path';

const isRelease = process.argv.includes('--release');
const profile = isRelease ? 'release' : 'debug';
const extension = process.platform === 'win32' ? 'dll' : process.platform === 'darwin' ? 'dylib' : 'so';
const libraryName = process.platform === 'win32' ? 'indexbind_node.dll' : `libindexbind_node.${extension}`;
const source = path.resolve('target', profile, libraryName);
const targetDir = path.resolve('native');
const target = path.join(targetDir, `indexbind.${process.platform}-${process.arch}.node`);

if (!existsSync(source)) {
  throw new Error(`Native library not found: ${source}`);
}

mkdirSync(targetDir, { recursive: true });
copyFileSync(source, target);
console.log(`Copied native addon to ${target}`);
