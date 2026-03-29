#!/usr/bin/env node

import {
  benchmarkArtifact,
  buildCanonicalBundleFromDirectory,
  buildFromDirectory,
  exportArtifactFromBuildCache,
  exportCanonicalBundleFromBuildCache,
  inspectArtifact,
  updateBuildCacheFromDirectory,
} from './build.js';

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const [commandOrInput, ...rest] = args;

  if (!commandOrInput || commandOrInput === '--help' || commandOrInput === '-h') {
    printUsage();
    process.exit(commandOrInput ? 0 : 1);
  }

  try {
    switch (commandOrInput) {
      case 'build':
        await buildCommand(rest);
        break;
      case 'build-bundle':
        await buildBundleCommand(rest);
        break;
      case 'update-cache':
        await updateCacheCommand(rest);
        break;
      case 'export-artifact':
        await exportArtifactCommand(rest);
        break;
      case 'export-bundle':
        await exportBundleCommand(rest);
        break;
      case 'inspect':
        await inspectCommand(rest);
        break;
      case 'benchmark':
        await benchmarkCommand(rest);
        break;
      default:
        await buildLegacyCommand(commandOrInput, rest);
        break;
    }
  } catch (error) {
    if (error instanceof Error && error.message.startsWith('usage:')) {
      console.error(error.message);
    } else if (error instanceof Error) {
      console.error(error.message);
    } else {
      console.error(String(error));
    }
    process.exit(1);
  }
}

async function buildCommand(args: string[]): Promise<void> {
  const [inputDir, outputPath, backend] = args;
  if (!inputDir || !outputPath || args.length > 3) {
    throw new Error(usage());
  }

  const stats = await buildFromDirectory(inputDir, outputPath, parseBuildOptions(backend));
  console.log(`built artifact with ${stats.documentCount} documents and ${stats.chunkCount} chunks`);
}

async function buildBundleCommand(args: string[]): Promise<void> {
  const [inputDir, outputDir, backend] = args;
  if (!inputDir || !outputDir || args.length > 3) {
    throw new Error(usage());
  }

  const stats = await buildCanonicalBundleFromDirectory(
    inputDir,
    outputDir,
    parseBuildOptions(backend),
  );
  console.log(
    `built canonical artifact bundle with ${stats.documentCount} documents, ${stats.chunkCount} chunks, and ${stats.vectorDimensions}-dim vectors`,
  );
}

async function updateCacheCommand(args: string[]): Promise<void> {
  const positional: string[] = [];
  let useGitDiff = false;
  let gitBase: string | undefined;

  for (let index = 0; index < args.length; index += 1) {
    const value = args[index];
    if (value === '--git-diff') {
      useGitDiff = true;
      continue;
    }
    if (value === '--git-base') {
      gitBase = args[index + 1];
      if (!gitBase) {
        throw new Error('--git-base requires a revision');
      }
      useGitDiff = true;
      index += 1;
      continue;
    }
    if (value.startsWith('--')) {
      throw new Error(usage());
    }
    positional.push(value);
  }

  if (positional.length < 2 || positional.length > 3) {
    throw new Error(usage());
  }

  const [inputDir, cachePath, backend] = positional;
  if (!inputDir || !cachePath) {
    throw new Error(usage());
  }

  const stats = await updateBuildCacheFromDirectory(
    inputDir,
    cachePath,
    parseBuildOptions(backend),
    useGitDiff ? { mode: 'git-diff', baseRevision: gitBase } : { mode: 'full-scan' },
  );
  console.log(JSON.stringify(stats, null, 2));
}

async function exportArtifactCommand(args: string[]): Promise<void> {
  const [cachePath, outputPath] = args;
  if (!cachePath || !outputPath || args.length !== 2) {
    throw new Error(usage());
  }
  const stats = await exportArtifactFromBuildCache(cachePath, outputPath);
  console.log(`exported artifact with ${stats.documentCount} documents and ${stats.chunkCount} chunks`);
}

async function exportBundleCommand(args: string[]): Promise<void> {
  const [cachePath, outputDir] = args;
  if (!cachePath || !outputDir || args.length !== 2) {
    throw new Error(usage());
  }
  const stats = await exportCanonicalBundleFromBuildCache(cachePath, outputDir);
  console.log(
    `exported canonical artifact bundle with ${stats.documentCount} documents, ${stats.chunkCount} chunks, and ${stats.vectorDimensions}-dim vectors`,
  );
}

async function inspectCommand(args: string[]): Promise<void> {
  const [artifactPath] = args;
  if (!artifactPath || args.length !== 1) {
    throw new Error('usage: indexbind inspect <artifact-file>');
  }
  console.log(JSON.stringify(await inspectArtifact(artifactPath), null, 2));
}

async function benchmarkCommand(args: string[]): Promise<void> {
  const [artifactPath, queriesJsonPath] = args;
  if (!artifactPath || !queriesJsonPath || args.length !== 2) {
    throw new Error('usage: indexbind benchmark <artifact-file> <queries-json>');
  }
  console.log(JSON.stringify(await benchmarkArtifact(artifactPath, queriesJsonPath), null, 2));
}

async function buildLegacyCommand(inputDir: string, args: string[]): Promise<void> {
  const [outputPath, backend] = args;
  if (!outputPath || args.length > 2) {
    throw new Error(usage());
  }
  const stats = await buildFromDirectory(inputDir, outputPath, parseBuildOptions(backend));
  console.log(`built artifact with ${stats.documentCount} documents and ${stats.chunkCount} chunks`);
}

function parseBuildOptions(backend: string | undefined) {
  if (!backend) {
    return {};
  }
  if (backend === 'hashing') {
    return { embeddingBackend: 'hashing' as const };
  }
  return { embeddingBackend: 'model2vec' as const, model: backend };
}

function printUsage(): void {
  console.log(usage());
}

function usage(): string {
  return `usage:
  indexbind build <input-dir> <output-file> [hashing|<model-id>]
  indexbind build-bundle <input-dir> <output-dir> [hashing|<model-id>]
  indexbind update-cache <input-dir> <cache-file> [hashing|<model-id>] [--git-diff] [--git-base <rev>]
  indexbind export-artifact <cache-file> <output-file>
  indexbind export-bundle <cache-file> <output-dir>
  indexbind inspect <artifact-file>
  indexbind benchmark <artifact-file> <queries-json>

For backward compatibility, \`indexbind <input-dir> <output-file> [hashing|<model-id>]\` still works.`;
}

await main();
