use anyhow::{anyhow, bail, Result};
use indexbind_build::build_from_directory;
use indexbind_core::{BuildArtifactOptions, EmbeddingBackend, Retriever, SearchOptions};
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let Some(command_or_input) = args.next() else {
        bail!("{}", usage());
    };

    match command_or_input.as_str() {
        "build" => build_command(args.collect()),
        "inspect" => inspect_command(args.collect()),
        "benchmark" => benchmark_command(args.collect()),
        input => build_command_with_input(input.to_string(), args.collect()),
    }
}

fn build_command(args: Vec<String>) -> Result<()> {
    let mut args = args.into_iter();
    let input = args.next().ok_or_else(|| anyhow!("{}", usage()))?;
    build_command_with_input(input, args.collect())
}

fn build_command_with_input(input: String, args: Vec<String>) -> Result<()> {
    let mut args = args.into_iter();
    let output = args.next().ok_or_else(|| anyhow!("{}", usage()))?;
    let backend = match args.next().as_deref() {
        Some("hashing") => EmbeddingBackend::Hashing { dimensions: 256 },
        Some(model) => EmbeddingBackend::Model2Vec {
            model: model.to_string(),
            batch_size: 512,
        },
        None => EmbeddingBackend::default(),
    };

    let stats = build_from_directory(
        &PathBuf::from(input),
        &PathBuf::from(output),
        BuildArtifactOptions {
            embedding_backend: backend,
            ..Default::default()
        },
    )?;

    println!(
        "built artifact with {} documents and {} chunks",
        stats.document_count, stats.chunk_count
    );
    Ok(())
}

fn inspect_command(args: Vec<String>) -> Result<()> {
    let mut args = args.into_iter();
    let artifact = args
        .next()
        .ok_or_else(|| anyhow!("usage: indexbind-build inspect <artifact-file>"))?;
    let retriever = Retriever::open(&PathBuf::from(artifact), None)?;
    let info = retriever.info();
    let payload = json!({
        "schemaVersion": info.schema_version,
        "builtAt": info.built_at,
        "embeddingBackend": info.embedding_backend,
        "sourceRoot": info.source_root,
        "documentCount": info.document_count,
        "chunkCount": info.chunk_count,
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn benchmark_command(args: Vec<String>) -> Result<()> {
    let mut args = args.into_iter();
    let artifact = args
        .next()
        .ok_or_else(|| anyhow!("usage: indexbind-build benchmark <artifact-file> <queries-json>"))?;
    let queries_path = args
        .next()
        .ok_or_else(|| anyhow!("usage: indexbind-build benchmark <artifact-file> <queries-json>"))?;
    let payload = fs::read_to_string(&queries_path)?;
    let fixture: BenchmarkFixture = serde_json::from_str(&payload)?;
    let mut retriever = Retriever::open(&PathBuf::from(artifact), None)?;

    let mut passed = 0usize;
    let mut results = Vec::new();
    for case in &fixture.queries {
        let hits = retriever.search(
            &case.query,
            SearchOptions {
                top_k: case.top_k.unwrap_or(5),
                ..SearchOptions::default()
            },
        )?;
        let top_hit = hits.first().map(|hit| hit.relative_path.clone());
        let success = top_hit.as_deref() == Some(case.expected_top_hit.as_str());
        if success {
            passed += 1;
        }
        results.push(json!({
            "name": case.name,
            "query": case.query,
            "expectedTopHit": case.expected_top_hit,
            "actualTopHit": top_hit,
            "passed": success,
        }));
    }

    let summary = json!({
        "fixture": fixture.name,
        "total": fixture.queries.len(),
        "passed": passed,
        "failed": fixture.queries.len().saturating_sub(passed),
        "results": results,
    });
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

#[derive(Debug, Deserialize)]
struct BenchmarkFixture {
    name: String,
    queries: Vec<BenchmarkQuery>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkQuery {
    name: String,
    query: String,
    expected_top_hit: String,
    top_k: Option<usize>,
}

fn usage() -> &'static str {
    "usage:\n  indexbind-build build <input-dir> <output-file> [hashing|<model-id>]\n  indexbind-build inspect <artifact-file>\n  indexbind-build benchmark <artifact-file> <queries-json>\n\nFor backward compatibility, `indexbind-build <input-dir> <output-file> [hashing|<model-id>]` still works."
}
