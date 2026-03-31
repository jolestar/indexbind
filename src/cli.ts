#!/usr/bin/env node

import { openIndex, type DocumentHit, type SearchOptions } from './index.js';
import {
  benchmarkArtifact,
  buildCanonicalBundleFromDirectory,
  buildFromDirectory,
  exportArtifactFromBuildCache,
  exportCanonicalBundleFromBuildCache,
  inspectArtifact,
  updateBuildCacheFromDirectory,
} from './build.js';

type OutputMode = 'json' | 'text';

interface SearchEnvelope {
  query: string;
  options: SearchOptions;
  hitCount: number;
  hits: DocumentHit[];
}

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
      case 'search':
        await searchCommand(rest);
        break;
      default:
        await buildLegacyCommand(commandOrInput, rest);
        break;
    }
  } catch (error) {
    if (error instanceof Error) {
      console.error(error.message);
    } else {
      console.error(String(error));
    }
    process.exit(1);
  }
}

async function buildCommand(args: string[]): Promise<void> {
  if (wantsHelp(args)) {
    printUsage(buildUsage());
    return;
  }
  const { outputMode, args: filteredArgs } = extractOutputMode(args);
  const [inputDir, outputPath, backend] = filteredArgs;
  if (!inputDir || !outputPath || filteredArgs.length > 3) {
    throw new Error(usage());
  }

  const stats = await buildFromDirectory(inputDir, outputPath, parseBuildOptions(backend));
  emit(
    outputMode,
    stats,
    `built artifact with ${stats.documentCount} documents and ${stats.chunkCount} chunks`,
  );
}

async function buildBundleCommand(args: string[]): Promise<void> {
  if (wantsHelp(args)) {
    printUsage(buildBundleUsage());
    return;
  }
  const { outputMode, args: filteredArgs } = extractOutputMode(args);
  const [inputDir, outputDir, backend] = filteredArgs;
  if (!inputDir || !outputDir || filteredArgs.length > 3) {
    throw new Error(usage());
  }

  const stats = await buildCanonicalBundleFromDirectory(
    inputDir,
    outputDir,
    parseBuildOptions(backend),
  );
  emit(
    outputMode,
    stats,
    `built canonical artifact bundle with ${stats.documentCount} documents, ${stats.chunkCount} chunks, and ${stats.vectorDimensions}-dim vectors`,
  );
}

async function updateCacheCommand(args: string[]): Promise<void> {
  if (wantsHelp(args)) {
    printUsage(updateCacheUsage());
    return;
  }
  const { outputMode, args: filteredArgs } = extractOutputMode(args);
  const positional: string[] = [];
  let useGitDiff = false;
  let gitBase: string | undefined;

  for (let index = 0; index < filteredArgs.length; index += 1) {
    const value = filteredArgs[index];
    if (value === '--git-diff') {
      useGitDiff = true;
      continue;
    }
    if (value === '--git-base') {
      gitBase = filteredArgs[index + 1];
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

  emit(
    outputMode,
    stats,
    [
      `scanned documents: ${stats.scannedDocumentCount}`,
      `new: ${stats.newDocumentCount}`,
      `changed: ${stats.changedDocumentCount}`,
      `unchanged: ${stats.unchangedDocumentCount}`,
      `removed: ${stats.removedDocumentCount}`,
      `active documents: ${stats.activeDocumentCount}`,
      `active chunks: ${stats.activeChunkCount}`,
    ].join('\n'),
  );
}

async function exportArtifactCommand(args: string[]): Promise<void> {
  if (wantsHelp(args)) {
    printUsage(exportArtifactUsage());
    return;
  }
  const { outputMode, args: filteredArgs } = extractOutputMode(args);
  const [cachePath, outputPath] = filteredArgs;
  if (!cachePath || !outputPath || filteredArgs.length !== 2) {
    throw new Error(usage());
  }
  const stats = await exportArtifactFromBuildCache(cachePath, outputPath);
  emit(
    outputMode,
    stats,
    `exported artifact with ${stats.documentCount} documents and ${stats.chunkCount} chunks`,
  );
}

async function exportBundleCommand(args: string[]): Promise<void> {
  if (wantsHelp(args)) {
    printUsage(exportBundleUsage());
    return;
  }
  const { outputMode, args: filteredArgs } = extractOutputMode(args);
  const [cachePath, outputDir] = filteredArgs;
  if (!cachePath || !outputDir || filteredArgs.length !== 2) {
    throw new Error(usage());
  }
  const stats = await exportCanonicalBundleFromBuildCache(cachePath, outputDir);
  emit(
    outputMode,
    stats,
    `exported canonical artifact bundle with ${stats.documentCount} documents, ${stats.chunkCount} chunks, and ${stats.vectorDimensions}-dim vectors`,
  );
}

async function inspectCommand(args: string[]): Promise<void> {
  if (wantsHelp(args)) {
    printUsage(inspectUsage());
    return;
  }
  const { outputMode, args: filteredArgs } = extractOutputMode(args);
  const [artifactPath] = filteredArgs;
  if (!artifactPath || filteredArgs.length !== 1) {
    throw new Error('usage: indexbind inspect <artifact-file> [--text]');
  }
  const info = await inspectArtifact(artifactPath);
  emit(
    outputMode,
    info,
    [
      `schema version: ${info.schemaVersion}`,
      `built at: ${info.builtAt}`,
      `embedding backend: ${JSON.stringify(info.embeddingBackend)}`,
      `lexical tokenizer: ${info.lexicalTokenizer}`,
      `source root: ${JSON.stringify(info.sourceRoot)}`,
      `document count: ${info.documentCount}`,
      `chunk count: ${info.chunkCount}`,
    ].join('\n'),
  );
}

async function benchmarkCommand(args: string[]): Promise<void> {
  if (wantsHelp(args)) {
    printUsage(benchmarkUsage());
    return;
  }
  const { outputMode, args: filteredArgs } = extractOutputMode(args);
  const [artifactPath, queriesJsonPath] = filteredArgs;
  if (!artifactPath || !queriesJsonPath || filteredArgs.length !== 2) {
    throw new Error('usage: indexbind benchmark <artifact-file> <queries-json> [--text]');
  }
  const summary = await benchmarkArtifact(artifactPath, queriesJsonPath);
  emit(
    outputMode,
    summary,
    [
      `fixture: ${summary.fixture}`,
      `passed: ${summary.passed}/${summary.total}`,
      ...summary.results.map(
        (result) =>
          `${result.passed ? 'PASS' : 'FAIL'} ${result.name}: expected=${result.expectedTopHit} actual=${result.actualTopHit ?? 'null'}`,
      ),
    ].join('\n'),
  );
}

async function searchCommand(args: string[]): Promise<void> {
  if (wantsHelp(args)) {
    printUsage(searchUsage());
    return;
  }
  const { outputMode, args: filteredArgs } = extractOutputMode(args);
  const { artifactPath, query, options } = parseSearchCommandArgs(filteredArgs);
  const index = await openIndex(artifactPath, {
    modeProfile: options.mode === 'lexical' ? 'lexical' : 'hybrid',
  });
  const hits = await index.search(query, options);
  const envelope: SearchEnvelope = {
    query,
    options: normalizeSearchOptions(options),
    hitCount: hits.length,
    hits,
  };
  emit(outputMode, envelope, formatSearchText(envelope));
}

async function buildLegacyCommand(inputDir: string, args: string[]): Promise<void> {
  const { outputMode, args: filteredArgs } = extractOutputMode(args);
  const [outputPath, backend] = filteredArgs;
  if (!outputPath || filteredArgs.length > 2) {
    throw new Error(usage());
  }
  const stats = await buildFromDirectory(inputDir, outputPath, parseBuildOptions(backend));
  emit(
    outputMode,
    stats,
    `built artifact with ${stats.documentCount} documents and ${stats.chunkCount} chunks`,
  );
}

function parseSearchCommandArgs(args: string[]): {
  artifactPath: string;
  query: string;
  options: SearchOptions;
} {
  const positional: string[] = [];
  const metadata: Record<string, string> = {};
  let topK: number | undefined;
  let mode: 'hybrid' | 'vector' | 'lexical' | undefined;
  let minScore: number | undefined;
  let rerankerKind: 'embedding-v1' | 'heuristic-v1' | undefined;
  let candidatePoolSize: number | undefined;
  let relativePathPrefix: string | undefined;
  let metadataNumericMultiplier: string | undefined;

  for (let index = 0; index < args.length; index += 1) {
    const value = args[index];
    if (value === '--') {
      positional.push(...args.slice(index + 1));
      break;
    }
    switch (value) {
      case '--top-k':
        topK = parseIntegerFlag('--top-k', args[index + 1]);
        index += 1;
        break;
      case '--mode':
        mode = parseMode(args[index + 1]);
        index += 1;
        break;
      case '--hybrid':
        throw new Error(
          'The --hybrid flag has been removed. Use --mode hybrid, --mode vector, or --mode lexical instead.',
        );
      case '--min-score':
        minScore = parseFloatFlag('--min-score', args[index + 1]);
        index += 1;
        break;
      case '--reranker':
        rerankerKind = parseReranker(args[index + 1]);
        index += 1;
        break;
      case '--candidate-pool-size':
        candidatePoolSize = parseIntegerFlag('--candidate-pool-size', args[index + 1]);
        index += 1;
        break;
      case '--relative-path-prefix':
        relativePathPrefix = requireFlagValue('--relative-path-prefix', args[index + 1]);
        index += 1;
        break;
      case '--metadata': {
        const metadataArg = requireFlagValue('--metadata', args[index + 1]);
        const separator = metadataArg.indexOf('=');
        if (separator <= 0) {
          throw new Error('--metadata requires key=value');
        }
        metadata[metadataArg.slice(0, separator)] = metadataArg.slice(separator + 1);
        index += 1;
        break;
      }
      case '--score-adjust-metadata-multiplier':
        metadataNumericMultiplier = requireFlagValue(
          '--score-adjust-metadata-multiplier',
          args[index + 1],
        );
        index += 1;
        break;
      default:
        if (value.startsWith('--')) {
          throw new Error(searchUsage());
        }
        positional.push(value);
        break;
    }
  }

  const [artifactPath, query] = positional;
  if (!artifactPath || !query || positional.length !== 2) {
    throw new Error(searchUsage());
  }

  const options: SearchOptions = {
    topK,
    mode,
    minScore,
    relativePathPrefix,
  };

  if (Object.keys(metadata).length > 0) {
    options.metadata = metadata;
  }

  if (rerankerKind || candidatePoolSize !== undefined) {
    options.reranker = {
      kind: rerankerKind,
      candidatePoolSize,
    };
  }

  if (metadataNumericMultiplier) {
    options.scoreAdjustment = {
      metadataNumericMultiplier,
    };
  }

  return {
    artifactPath,
    query,
    options,
  };
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

function parseIntegerFlag(flag: string, value: string | undefined): number {
  const parsed = Number(requireFlagValue(flag, value));
  if (!Number.isInteger(parsed)) {
    throw new Error(`${flag} requires an integer`);
  }
  return parsed;
}

function parseFloatFlag(flag: string, value: string | undefined): number {
  const parsed = Number(requireFlagValue(flag, value));
  if (!Number.isFinite(parsed)) {
    throw new Error(`${flag} requires a finite number`);
  }
  return parsed;
}

function parseReranker(value: string | undefined): 'embedding-v1' | 'heuristic-v1' {
  const reranker = requireFlagValue('--reranker', value);
  if (reranker === 'embedding-v1' || reranker === 'heuristic-v1') {
    return reranker;
  }
  throw new Error(`unsupported reranker kind: ${reranker}`);
}

function parseMode(value: string | undefined): 'hybrid' | 'vector' | 'lexical' {
  const mode = requireFlagValue('--mode', value);
  if (mode === 'hybrid' || mode === 'vector' || mode === 'lexical') {
    return mode;
  }
  throw new Error(`unsupported retrieval mode: ${mode}`);
}

function requireFlagValue(flag: string, value: string | undefined): string {
  if (!value) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function extractOutputMode(args: string[]): { outputMode: OutputMode; args: string[] } {
  const filteredArgs: string[] = [];
  let outputMode: OutputMode = 'json';

  for (const value of args) {
    if (value === '--text') {
      outputMode = 'text';
      continue;
    }
    filteredArgs.push(value);
  }

  return { outputMode, args: filteredArgs };
}

function wantsHelp(args: string[]): boolean {
  for (const value of args) {
    if (value === '--') {
      return false;
    }
    if (value === '--help' || value === '-h') {
      return true;
    }
  }
  return false;
}

function emit(outputMode: OutputMode, jsonValue: unknown, textValue: string): void {
  if (outputMode === 'text') {
    console.log(textValue);
    return;
  }
  console.log(JSON.stringify(jsonValue, null, 2));
}

function normalizeSearchOptions(options: SearchOptions): SearchOptions {
  return {
    topK: options.topK ?? 10,
    mode: options.mode ?? 'hybrid',
    ...(options.minScore !== undefined ? { minScore: options.minScore } : {}),
    ...(options.reranker
      ? {
          reranker: {
            kind: options.reranker.kind ?? 'embedding-v1',
            candidatePoolSize: options.reranker.candidatePoolSize ?? 50,
          },
        }
      : {}),
    ...(options.relativePathPrefix ? { relativePathPrefix: options.relativePathPrefix } : {}),
    ...(options.metadata && Object.keys(options.metadata).length > 0
      ? { metadata: options.metadata }
      : {}),
    ...(options.scoreAdjustment?.metadataNumericMultiplier
      ? { scoreAdjustment: options.scoreAdjustment }
      : {}),
  };
}

function formatSearchText(result: SearchEnvelope): string {
  if (result.hits.length === 0) {
    return `query: ${result.query}\nhits: 0`;
  }

  const lines = [`query: ${result.query}`, `hits: ${result.hitCount}`];
  for (const [index, hit] of result.hits.entries()) {
    lines.push(`${index + 1}. [${hit.score.toFixed(4)}] ${hit.relativePath}`);
    if (hit.title) {
      lines.push(`   title: ${hit.title}`);
    }
    if (hit.bestMatch.excerpt) {
      lines.push(`   excerpt: ${hit.bestMatch.excerpt.replace(/\s+/g, ' ').trim()}`);
    }
  }
  return lines.join('\n');
}

function printUsage(text: string = usage()): void {
  console.log(text);
}

function usage(): string {
  return `usage:
  ${buildUsage()}
  ${buildBundleUsage()}
  ${updateCacheUsage()}
  ${exportArtifactUsage()}
  ${exportBundleUsage()}
  ${inspectUsage()}
  ${benchmarkUsage()}
  ${searchUsage()}

By default, commands print JSON. Add \`--text\` for human-friendly output.

For backward compatibility, \`indexbind <input-dir> <output-file> [hashing|<model-id>] [--text]\` still works.`;
}

function buildUsage(): string {
  return 'indexbind build <input-dir> <output-file> [hashing|<model-id>] [--text]';
}

function buildBundleUsage(): string {
  return 'indexbind build-bundle <input-dir> <output-dir> [hashing|<model-id>] [--text]';
}

function updateCacheUsage(): string {
  return 'indexbind update-cache <input-dir> <cache-file> [hashing|<model-id>] [--git-diff] [--git-base <rev>] [--text]';
}

function exportArtifactUsage(): string {
  return 'indexbind export-artifact <cache-file> <output-file> [--text]';
}

function exportBundleUsage(): string {
  return 'indexbind export-bundle <cache-file> <output-dir> [--text]';
}

function inspectUsage(): string {
  return 'indexbind inspect <artifact-file> [--text]';
}

function benchmarkUsage(): string {
  return 'indexbind benchmark <artifact-file> <queries-json> [--text]';
}

function searchUsage(): string {
  return 'usage: indexbind search <artifact-file> <query> [--top-k <n>] [--mode <hybrid|vector|lexical>] [--reranker <kind>] [--candidate-pool-size <n>] [--relative-path-prefix <prefix>] [--metadata key=value] [--score-adjust-metadata-multiplier <field>] [--min-score <float>] [--text]';
}

await main();
