//! Cross-language project configuration resolver (Sprint 0)
//!
//! Resolves project-level configuration files (tsconfig.json, pyproject.toml, go.mod, etc.)
//! to determine module resolution rules, import paths, and project-specific settings.
//!
//! This is distinct from `parsing::resolution` which handles symbol resolution within code.
//! - project_resolver: "What tsconfig.json applies to this file?"
//! - parsing::resolution: "What does the identifier 'foo' refer to in this scope?"

pub mod memo;
pub mod provider;
pub mod registry;
pub mod sha;

// Shared core types to be extended in later steps (TDD-driven)
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sha256Hash(pub String);

#[derive(Debug, thiserror::Error)]
pub enum ResolutionError {
    /// Error reading/writing cache files on disk
    #[error("cache io error at '{path}': {source}")]
    CacheIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Cache format is invalid or incompatible
    #[error("invalid cache: {details}")]
    InvalidCache { details: String },
}

impl ResolutionError {
    pub fn cache_io(path: PathBuf, source: std::io::Error) -> Self {
        Self::CacheIo { path, source }
    }
    pub fn invalid_cache(details: impl Into<String>) -> Self {
        Self::InvalidCache {
            details: details.into(),
        }
    }
    pub fn suggestion(&self) -> &'static str {
        match self {
            ResolutionError::CacheIo { .. } => {
                "Check permissions and disk space; delete the cache file to rebuild."
            }
            ResolutionError::InvalidCache { .. } => {
                "Delete the on-disk cache to rebuild; ensure codanna version matches cache format."
            }
        }
    }
    /// Stable code for programmatic handling in JSON responses
    pub fn status_code(&self) -> String {
        match self {
            ResolutionError::CacheIo { .. } => "RESOLUTION_CACHE_IO",
            ResolutionError::InvalidCache { .. } => "RESOLUTION_INVALID_CACHE",
        }
        .to_string()
    }
    /// Recovery suggestions list (mirrors project error conventions)
    pub fn recovery_suggestions(&self) -> Vec<&'static str> {
        match self {
            ResolutionError::CacheIo { .. } => vec![
                "Ensure the cache directory exists and is writable",
                "Check disk space and permissions",
                "Delete the on-disk cache to force a rebuild",
            ],
            ResolutionError::InvalidCache { .. } => vec![
                "Delete the on-disk cache to force a rebuild",
                "Verify codanna version compatibility with cache format",
            ],
        }
    }
}

pub type ResolutionResult<T> = Result<T, ResolutionError>;
