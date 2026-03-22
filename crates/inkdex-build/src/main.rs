use anyhow::{anyhow, Result};
use inkdex_build::build_from_directory;
use inkdex_core::{BuildArtifactOptions, EmbeddingBackend};
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let input = args.next().ok_or_else(|| {
        anyhow!("usage: inkdex-build <input-dir> <output-file> [hashing|fastembed]")
    })?;
    let output = args.next().ok_or_else(|| {
        anyhow!("usage: inkdex-build <input-dir> <output-file> [hashing|fastembed]")
    })?;
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
