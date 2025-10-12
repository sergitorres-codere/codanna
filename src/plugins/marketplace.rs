//! Marketplace manifest handling for plugin discovery

use super::error::{PluginError, PluginResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Marketplace manifest structure
/// Located at .claude-plugin/marketplace.json in marketplace repos
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketplaceManifest {
    /// Marketplace name
    pub name: String,

    /// Owner information
    pub owner: MarketplaceOwner,

    /// List of available plugins
    pub plugins: Vec<MarketplacePlugin>,

    /// Optional marketplace description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional marketplace URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Marketplace owner information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketplaceOwner {
    /// Owner name (user or organization)
    pub name: String,

    /// Optional email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Optional website
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Plugin entry in marketplace
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketplacePlugin {
    /// Plugin name (must be unique in marketplace)
    pub name: String,

    /// Path to plugin directory relative to marketplace root
    pub source: String,

    /// Plugin description
    pub description: String,

    /// Optional version constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Optional tags for categorization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl MarketplaceManifest {
    /// Load marketplace manifest from JSON file
    pub fn from_file(path: &Path) -> PluginResult<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }

    /// Parse marketplace manifest from JSON string
    pub fn from_json(json: &str) -> PluginResult<Self> {
        let manifest: Self = serde_json::from_str(json)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate manifest structure
    pub fn validate(&self) -> PluginResult<()> {
        if self.name.is_empty() {
            return Err(PluginError::InvalidMarketplaceManifest {
                reason: "Marketplace name cannot be empty".to_string(),
            });
        }

        if self.owner.name.is_empty() {
            return Err(PluginError::InvalidMarketplaceManifest {
                reason: "Owner name cannot be empty".to_string(),
            });
        }

        if self.plugins.is_empty() {
            return Err(PluginError::InvalidMarketplaceManifest {
                reason: "Marketplace must contain at least one plugin".to_string(),
            });
        }

        // Check for duplicate plugin names
        let mut seen = std::collections::HashSet::new();
        for plugin in &self.plugins {
            if !seen.insert(&plugin.name) {
                return Err(PluginError::InvalidMarketplaceManifest {
                    reason: format!("Duplicate plugin name: {}", plugin.name),
                });
            }

            // Validate plugin entry
            plugin.validate()?;
        }

        Ok(())
    }

    /// Find a plugin by name
    pub fn find_plugin(&self, name: &str) -> Option<&MarketplacePlugin> {
        self.plugins.iter().find(|p| p.name == name)
    }
}

impl MarketplacePlugin {
    /// Validate plugin entry
    pub fn validate(&self) -> PluginResult<()> {
        if self.name.is_empty() {
            return Err(PluginError::InvalidMarketplaceManifest {
                reason: "Plugin name cannot be empty".to_string(),
            });
        }

        if self.source.is_empty() {
            return Err(PluginError::InvalidMarketplaceManifest {
                reason: format!("Plugin '{}' source path cannot be empty", self.name),
            });
        }

        if self.description.is_empty() {
            return Err(PluginError::InvalidMarketplaceManifest {
                reason: format!("Plugin '{}' description cannot be empty", self.name),
            });
        }

        // Validate source path doesn't escape marketplace
        if self.source.contains("..") {
            return Err(PluginError::InvalidMarketplaceManifest {
                reason: format!("Plugin '{}' source path cannot contain '..'", self.name),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_marketplace() {
        let json = r#"{
            "name": "test-marketplace",
            "owner": {
                "name": "Test User"
            },
            "plugins": [
                {
                    "name": "test-plugin",
                    "source": "./plugins/test",
                    "description": "A test plugin"
                }
            ]
        }"#;

        let manifest = MarketplaceManifest::from_json(json).unwrap();
        assert_eq!(manifest.name, "test-marketplace");
        assert_eq!(manifest.plugins.len(), 1);
    }

    #[test]
    fn test_reject_empty_marketplace() {
        let json = r#"{
            "name": "test-marketplace",
            "owner": {
                "name": "Test User"
            },
            "plugins": []
        }"#;

        let result = MarketplaceManifest::from_json(json);
        assert!(matches!(
            result,
            Err(PluginError::InvalidMarketplaceManifest { .. })
        ));
    }

    #[test]
    fn test_reject_duplicate_plugins() {
        let json = r#"{
            "name": "test-marketplace",
            "owner": {
                "name": "Test User"
            },
            "plugins": [
                {
                    "name": "duplicate",
                    "source": "./plugin1",
                    "description": "Plugin 1"
                },
                {
                    "name": "duplicate",
                    "source": "./plugin2",
                    "description": "Plugin 2"
                }
            ]
        }"#;

        let result = MarketplaceManifest::from_json(json);
        assert!(matches!(
            result,
            Err(PluginError::InvalidMarketplaceManifest { .. })
        ));
    }

    #[test]
    fn test_find_plugin() {
        let json = r#"{
            "name": "test-marketplace",
            "owner": {
                "name": "Test User"
            },
            "plugins": [
                {
                    "name": "plugin-a",
                    "source": "./a",
                    "description": "Plugin A"
                },
                {
                    "name": "plugin-b",
                    "source": "./b",
                    "description": "Plugin B"
                }
            ]
        }"#;

        let manifest = MarketplaceManifest::from_json(json).unwrap();
        assert!(manifest.find_plugin("plugin-a").is_some());
        assert!(manifest.find_plugin("plugin-b").is_some());
        assert!(manifest.find_plugin("plugin-c").is_none());
    }
}
