use crate::chunking::ChunkingOptions;
use crate::embedding::EmbeddingBackend;
use crate::types::SourceRoot;
use blake3::Hasher;

#[derive(Debug, Clone)]
pub struct BuildArtifactOptions {
    pub source_root: SourceRoot,
    pub embedding_backend: EmbeddingBackend,
    pub chunking: ChunkingOptions,
}

impl Default for BuildArtifactOptions {
    fn default() -> Self {
        Self {
            source_root: SourceRoot {
                id: "root".to_string(),
                original_path: ".".to_string(),
            },
            embedding_backend: EmbeddingBackend::default(),
            chunking: ChunkingOptions::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildStats {
    pub document_count: usize,
    pub chunk_count: usize,
}

pub(crate) fn build_doc_id(source_root_id: &str, relative_path: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(source_root_id.as_bytes());
    hasher.update(b":");
    hasher.update(relative_path.as_bytes());
    hasher.finalize().to_hex().to_string()
}

pub(crate) fn build_chunk_id(doc_id: &str, ordinal: usize) -> i64 {
    let mut hasher = Hasher::new();
    hasher.update(doc_id.as_bytes());
    hasher.update(b":");
    hasher.update(ordinal.to_string().as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
    i64::from_be_bytes(bytes) & i64::MAX
}
