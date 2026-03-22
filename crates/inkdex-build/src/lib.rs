use anyhow::{anyhow, Result};
use inkdex_core::{
    build_artifact, BuildArtifactOptions, BuildStats, NormalizedDocument, SourceRoot,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn build_from_directory(
    input: &Path,
    output: &Path,
    mut options: BuildArtifactOptions,
) -> Result<BuildStats> {
    let source_root = input.canonicalize()?;
    options.source_root = SourceRoot {
        id: "root".to_string(),
        original_path: source_root.display().to_string(),
    };
    let documents = read_documents(&source_root)?;
    build_artifact(output, &documents, &options).map_err(Into::into)
}

fn read_documents(root: &Path) -> Result<Vec<NormalizedDocument>> {
    let mut documents = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() || !supported_extension(entry.path()) {
            continue;
        }
        let path = entry.path();
        let content = fs::read_to_string(path)?;
        let relative_path = relative_path(root, path)?;
        let title = extract_title(&content).or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string)
        });
        documents.push(NormalizedDocument {
            original_path: path.canonicalize()?.display().to_string(),
            relative_path,
            title,
            content,
            metadata: BTreeMap::new(),
        });
    }
    Ok(documents)
}

fn extract_title(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            Some(trimmed.trim_start_matches('#').trim().to_string())
        } else {
            None
        }
    })
}

fn relative_path(root: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| anyhow!("path is outside of source root: {}", path.display()))?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn supported_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md" | "mdx" | "txt")
    )
}

#[allow(dead_code)]
fn _debug_root(path: &PathBuf) -> String {
    path.display().to_string()
}
