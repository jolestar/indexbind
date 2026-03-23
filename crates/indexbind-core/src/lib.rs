mod artifact;
mod chunking;
mod embedding;
mod error;
mod retriever;
mod types;

pub use artifact::{build_artifact, BuildArtifactOptions, BuildStats};
pub use embedding::EmbeddingBackend;
pub use error::{IndexbindError, Result};
pub use retriever::{ArtifactInfo, RerankerKind, RerankerOptions, Retriever, SearchOptions};
pub use types::{
    BestMatch, DocumentHit, MetadataMap, NormalizedDocument, SourceRoot, StoredChunk, StoredDocument,
};
