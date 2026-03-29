use anyhow::{anyhow, Result};
use indexbind_core::{
    build_artifact, build_canonical_artifact, BuildArtifactOptions, BuildStats,
    CanonicalBuildStats, NormalizedDocument, SourceRoot,
};
use serde_json::{Map, Value};
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

pub fn build_canonical_from_directory(
    input: &Path,
    output_dir: &Path,
    mut options: BuildArtifactOptions,
) -> Result<CanonicalBuildStats> {
    let source_root = input.canonicalize()?;
    options.source_root = SourceRoot {
        id: "root".to_string(),
        original_path: source_root.display().to_string(),
    };
    let documents = read_documents(&source_root)?;
    build_canonical_artifact(output_dir, &documents, &options).map_err(Into::into)
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
        let source = fs::read_to_string(path)?;
        let relative_path = relative_path(root, path)?;
        let file_name_title = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_string);
        let parsed = parse_document_source(&source, file_name_title);
        documents.push(NormalizedDocument {
            doc_id: None,
            source_path: Some(path.canonicalize()?.display().to_string()),
            relative_path,
            canonical_url: parsed.canonical_url,
            title: parsed.title,
            summary: parsed.summary,
            content: parsed.content,
            metadata: parsed.metadata,
        });
    }
    Ok(documents)
}

#[derive(Debug, PartialEq)]
struct ParsedDocumentSource {
    canonical_url: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    content: String,
    metadata: BTreeMap<String, Value>,
}

fn parse_document_source(source: &str, file_name_title: Option<String>) -> ParsedDocumentSource {
    let (frontmatter, content) = split_frontmatter(source)
        .and_then(|(frontmatter, body)| parse_frontmatter(frontmatter).map(|parsed| (parsed, body)))
        .map(|(frontmatter, body)| (frontmatter, body.to_string()))
        .unwrap_or_else(|| (ParsedFrontmatter::default(), source.to_string()));
    let title = frontmatter
        .title
        .clone()
        .or_else(|| extract_title(&content))
        .or(file_name_title);

    ParsedDocumentSource {
        canonical_url: frontmatter.canonical_url,
        title,
        summary: frontmatter.summary,
        content,
        metadata: frontmatter.metadata,
    }
}

#[derive(Debug, Default)]
struct ParsedFrontmatter {
    canonical_url: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    metadata: BTreeMap<String, Value>,
}

fn split_frontmatter(source: &str) -> Option<(&str, &str)> {
    let (rest, delimiter) = if let Some(rest) = source.strip_prefix("---\n") {
        (rest, "\n---\n")
    } else if let Some(rest) = source.strip_prefix("---\r\n") {
        (rest, "\r\n---\r\n")
    } else {
        return None;
    };
    let (frontmatter, body) = rest.split_once(delimiter)?;
    Some((frontmatter, body))
}

fn parse_frontmatter(frontmatter: &str) -> Option<ParsedFrontmatter> {
    let value = serde_yaml::from_str::<serde_yaml::Value>(frontmatter).ok()?;
    let Some(object) = yaml_mapping_to_json_map(value) else {
        return None;
    };

    let mut metadata = BTreeMap::new();
    let mut title = None;
    let mut summary = None;
    let mut canonical_url = None;
    for (key, value) in object {
        match key.as_str() {
            "title" => title = value.as_str().map(str::to_string),
            "summary" => summary = value.as_str().map(str::to_string),
            "canonical_url" | "canonicalUrl" => canonical_url = value.as_str().map(str::to_string),
            _ => {
                metadata.insert(key, value);
            }
        }
    }

    Some(ParsedFrontmatter {
        canonical_url,
        title,
        summary,
        metadata,
    })
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

fn yaml_mapping_to_json_map(value: serde_yaml::Value) -> Option<Map<String, Value>> {
    serde_json::to_value(value).ok()?.as_object().cloned()
}

#[allow(dead_code)]
fn _debug_root(path: &PathBuf) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::{parse_document_source, read_documents};
    use serde_json::json;
    use std::fs;

    #[test]
    fn frontmatter_overrides_body_heading_and_populates_metadata() {
        let source = r#"---
title: Frontmatter Title
summary: Short summary
canonical_url: /docs/getting-started
lang: rust
weight: 2
published: true
---

# Body Heading

Hello world.
"#;

        let parsed = parse_document_source(source, Some("fallback".to_string()));
        assert_eq!(parsed.title.as_deref(), Some("Frontmatter Title"));
        assert_eq!(parsed.summary.as_deref(), Some("Short summary"));
        assert_eq!(
            parsed.canonical_url.as_deref(),
            Some("/docs/getting-started")
        );
        assert_eq!(
            parsed.content.trim_start(),
            "# Body Heading\n\nHello world.\n"
        );
        assert_eq!(parsed.metadata.get("lang"), Some(&json!("rust")));
        assert_eq!(parsed.metadata.get("weight"), Some(&json!(2)));
        assert_eq!(parsed.metadata.get("published"), Some(&json!(true)));
        assert!(!parsed.metadata.contains_key("title"));
        assert!(!parsed.metadata.contains_key("summary"));
        assert!(!parsed.metadata.contains_key("canonical_url"));
    }

    #[test]
    fn body_heading_and_filename_remain_fallbacks() {
        let with_heading = parse_document_source("# Heading\n\nBody", Some("fallback".to_string()));
        assert_eq!(with_heading.title.as_deref(), Some("Heading"));

        let with_filename = parse_document_source("Body only", Some("fallback".to_string()));
        assert_eq!(with_filename.title.as_deref(), Some("fallback"));
    }

    #[test]
    fn canonical_url_alias_is_supported() {
        let source = r#"---
canonicalUrl: /docs/alias
---

Body
"#;
        let parsed = parse_document_source(source, None);
        assert_eq!(parsed.canonical_url.as_deref(), Some("/docs/alias"));
    }

    #[test]
    fn invalid_frontmatter_falls_back_to_body_content() {
        let source = "---\ninvalid: [\n---\n# Heading\n";
        let parsed = parse_document_source(source, None);
        assert_eq!(parsed.title.as_deref(), Some("Heading"));
        assert_eq!(parsed.content, source);
        assert!(parsed.metadata.is_empty());
    }

    #[test]
    fn windows_style_frontmatter_is_supported() {
        let source = "---\r\ntitle: Guide\r\nsummary: Windows\r\n---\r\n\r\nBody\r\n";
        let parsed = parse_document_source(source, None);
        assert_eq!(parsed.title.as_deref(), Some("Guide"));
        assert_eq!(parsed.summary.as_deref(), Some("Windows"));
        assert_eq!(parsed.content, "\r\nBody\r\n");
    }

    #[test]
    fn read_documents_parses_frontmatter_from_directory_files() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("guide.md");
        fs::write(
            &path,
            r#"---
title: Guide
summary: Directory summary
canonical_url: /docs/guide
section: docs
---

# Ignored Heading

Body
"#,
        )
        .unwrap();

        let documents = read_documents(tempdir.path()).unwrap();
        assert_eq!(documents.len(), 1);
        let document = &documents[0];
        assert_eq!(document.title.as_deref(), Some("Guide"));
        assert_eq!(document.summary.as_deref(), Some("Directory summary"));
        assert_eq!(document.canonical_url.as_deref(), Some("/docs/guide"));
        assert_eq!(document.metadata.get("section"), Some(&json!("docs")));
        assert_eq!(document.content.trim_start(), "# Ignored Heading\n\nBody\n");
    }
}
