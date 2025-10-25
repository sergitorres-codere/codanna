//! Project manifest - team contract stored at .codanna/manifest.json

use super::error::{ProfileError, ProfileResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectManifest {
    pub profile: String,
}

impl ProjectManifest {
    /// Create a new empty manifest
    pub fn new() -> Self {
        Self {
            profile: String::new(),
        }
    }

    /// Load manifest from disk
    pub fn load(path: &Path) -> ProfileResult<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }

    /// Save manifest to disk
    pub fn save(&self, path: &Path) -> ProfileResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load manifest or create new if doesn't exist
    pub fn load_or_create(path: &Path) -> ProfileResult<Self> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::new())
        }
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> ProfileResult<Self> {
        let manifest: Self = serde_json::from_str(json)?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> ProfileResult<()> {
        if self.profile.is_empty() {
            return Err(ProfileError::InvalidManifest {
                reason: "Profile name cannot be empty".to_string(),
            });
        }
        Ok(())
    }
}

impl Default for ProjectManifest {
    fn default() -> Self {
        Self::new()
    }
}
