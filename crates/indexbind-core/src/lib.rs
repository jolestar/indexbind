#[cfg(not(target_arch = "wasm32"))]
mod artifact;
mod build;
mod canonical;
mod chunking;
mod embedding;
mod error;
#[cfg(not(target_arch = "wasm32"))]
mod retriever;
mod types;

#[cfg(not(target_arch = "wasm32"))]
pub use artifact::build_artifact;
pub use build::{BuildArtifactOptions, BuildStats};
pub use canonical::{
    build_canonical_artifact, CanonicalArtifactManifest, CanonicalBuildStats, CanonicalChunkRecord,
    CanonicalDocumentRecord, CanonicalPosting, CanonicalPostings,
};
pub use chunking::ChunkingOptions;
pub use embedding::EmbeddingBackend;
pub use error::{IndexbindError, Result};
#[cfg(not(target_arch = "wasm32"))]
pub use retriever::{ArtifactInfo, RerankerKind, RerankerOptions, Retriever, SearchOptions};
pub use types::{
    BestMatch, DocumentHit, MetadataMap, NormalizedDocument, SourceRoot, StoredChunk, StoredDocument,
};
