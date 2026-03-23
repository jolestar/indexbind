use anyhow::{anyhow, bail, Result};
use inkdex_build::build_from_directory;
use inkdex_core::{BuildArtifactOptions, EmbeddingBackend, Retriever};
use serde_json::json;
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let Some(command_or_input) = args.next() else {
        bail!("{}", usage());
    };

    match command_or_input.as_str() {
        "build" => build_command(args.collect()),
        "inspect" => inspect_command(args.collect()),
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
        .ok_or_else(|| anyhow!("usage: inkdex-build inspect <artifact-file>"))?;
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

fn usage() -> &'static str {
    "usage:\n  inkdex-build build <input-dir> <output-file> [hashing|<model-id>]\n  inkdex-build inspect <artifact-file>\n\nFor backward compatibility, `inkdex-build <input-dir> <output-file> [hashing|<model-id>]` still works."
}
