mod artifact;
mod canonical;
mod chunking;
mod embedding;
mod error;
mod retriever;
mod types;

pub use artifact::{build_artifact, BuildArtifactOptions, BuildStats};
pub use canonical::{
    build_canonical_artifact, CanonicalArtifactManifest, CanonicalBuildStats, CanonicalChunkRecord,
    CanonicalDocumentRecord, CanonicalPosting, CanonicalPostings,
};
pub use chunking::ChunkingOptions;
pub use embedding::EmbeddingBackend;
pub use error::{IndexbindError, Result};
pub use retriever::{ArtifactInfo, RerankerKind, RerankerOptions, Retriever, SearchOptions};
pub use types::{
    BestMatch, DocumentHit, MetadataMap, NormalizedDocument, SourceRoot, StoredChunk, StoredDocument,
};
