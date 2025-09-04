//! Mock TypeScript Provider for architecture validation
//!
//! This simulates what Sprint 1 will build, validating our architecture
//! supports the TypeScript tsconfig.json resolution use case.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use codanna::config::Settings;
use codanna::project_resolver::{
    ResolutionResult, Sha256Hash,
    memo::ResolutionMemo,
    provider::ProjectResolutionProvider,
    registry::{ResolutionProviderRegistry, SimpleProviderRegistry},
    sha::compute_file_sha,
};

/// Mock TypeScript provider that simulates tsconfig.json resolution
pub struct MockTypeScriptProvider {
    /// In-memory cache of computed SHAs for config files
    sha_cache: ResolutionMemo<HashMap<PathBuf, Sha256Hash>>,
}

impl Default for MockTypeScriptProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTypeScriptProvider {
    pub fn new() -> Self {
        Self {
            sha_cache: ResolutionMemo::new(),
        }
    }
}

impl ProjectResolutionProvider for MockTypeScriptProvider {
    fn language_id(&self) -> &'static str {
        "typescript"
    }

    fn is_enabled(&self, _settings: &Settings) -> bool {
        // In real impl: check settings.languages.typescript.enabled
        true
    }

    fn config_paths(&self, _settings: &Settings) -> Vec<PathBuf> {
        // In Sprint 1: Read from settings.languages.typescript.tsconfig_files
        vec![
            PathBuf::from("tsconfig.json"),
            PathBuf::from("packages/web/tsconfig.json"),
        ]
    }

    fn compute_shas(&self, configs: &[PathBuf]) -> ResolutionResult<HashMap<PathBuf, Sha256Hash>> {
        let mut shas = HashMap::with_capacity(configs.len());

        for config_path in configs {
            if config_path.exists() {
                let sha = compute_file_sha(config_path)?;
                shas.insert(config_path.to_path_buf(), sha);
            }
        }

        Ok(shas)
    }

    fn rebuild_cache(&self, settings: &Settings) -> ResolutionResult<()> {
        // Get config paths from settings
        let configs = self.config_paths(settings);

        // Compute SHAs for all configs
        let shas = self.compute_shas(&configs)?;

        // Store in memoization cache
        // In Sprint 1: This would also parse tsconfig and build path rules
        let cache_key = Sha256Hash("typescript_provider_v1".to_string());
        self.sha_cache.insert(cache_key, shas);

        Ok(())
    }

    fn select_affected_files(
        &self,
        _indexer: &codanna::indexing::SimpleIndexer,
        settings: &Settings,
    ) -> Vec<PathBuf> {
        // This is called by IndexWatcher when configs change
        // Returns files that need reindexing based on which configs changed

        // In Sprint 1: Will implement longest-prefix matching:
        // - tsconfig.json at root affects src/**/*.ts
        // - packages/web/tsconfig.json affects packages/web/**/*.ts

        // For testing: Return mock paths based on config paths
        let config_paths = self.config_paths(settings);
        let mut affected = Vec::new();

        for config in config_paths {
            if config == PathBuf::from("tsconfig.json") {
                affected.push(PathBuf::from("src/app.ts"));
                affected.push(PathBuf::from("src/index.tsx"));
            } else if config == PathBuf::from("packages/web/tsconfig.json") {
                affected.push(PathBuf::from("packages/web/src/main.ts"));
            }
        }

        affected
    }
}

#[test]
fn mock_typescript_provider_basics() {
    let provider = MockTypeScriptProvider::new();

    assert_eq!(provider.language_id(), "typescript");
    assert!(provider.is_enabled(&Settings::default()));
}

#[test]
fn mock_typescript_provider_config_paths() {
    let provider = MockTypeScriptProvider::new();
    let paths = provider.config_paths(&Settings::default());

    assert_eq!(paths.len(), 2, "Should have root and package tsconfig");
    assert!(paths.contains(&PathBuf::from("tsconfig.json")));
    assert!(paths.contains(&PathBuf::from("packages/web/tsconfig.json")));
}

#[test]
fn mock_typescript_provider_computes_shas() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("tsconfig.json");

    let config_content = r#"{
        "compilerOptions": {
            "baseUrl": ".",
            "paths": {
                "@app/*": ["src/app/*"],
                "@shared/*": ["src/shared/*"]
            }
        }
    }"#;

    let mut file = fs::File::create(&config_path).unwrap();
    file.write_all(config_content.as_bytes()).unwrap();

    let provider = MockTypeScriptProvider::new();
    let paths = vec![config_path.clone()];
    let result = provider.compute_shas(&paths);

    assert!(result.is_ok(), "Should compute SHA successfully");
    let shas = result.unwrap();
    assert_eq!(shas.len(), 1, "Should have one SHA");
    assert!(
        shas.contains_key(&config_path),
        "Should have SHA for config file"
    );
}

#[test]
fn mock_typescript_provider_rebuild_cache() {
    let provider = MockTypeScriptProvider::new();
    let settings = Settings::default();

    // This should not fail even without actual config files
    let result = provider.rebuild_cache(&settings);
    assert!(
        result.is_ok(),
        "Rebuild should handle missing configs gracefully"
    );
}

#[test]
fn end_to_end_provider_registry_integration() {
    // Create registry and add TypeScript provider
    let mut registry = SimpleProviderRegistry::new();
    registry.add(Arc::new(MockTypeScriptProvider::new()));

    // Verify provider is registered
    assert_eq!(registry.providers().len(), 1);
    assert_eq!(registry.providers()[0].language_id(), "typescript");

    // Get active providers based on settings
    let settings = Settings::default();
    let active = registry.active_providers(&settings);
    assert_eq!(active.len(), 1, "TypeScript should be active");

    // Simulate config change detection and rebuild
    for provider in active.iter() {
        let rebuild_result = provider.rebuild_cache(&settings);
        assert!(rebuild_result.is_ok(), "Provider cache rebuild should work");
    }
}

#[test]
fn sha_based_invalidation_detection() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("tsconfig.json");

    // Write initial config
    let initial_content = r#"{"compilerOptions": {"baseUrl": "."}}"#;
    fs::write(&config_path, initial_content).unwrap();

    let provider = MockTypeScriptProvider::new();

    // Compute initial SHA
    let paths = vec![config_path.clone()];
    let initial_shas = provider.compute_shas(&paths).unwrap();
    let initial_sha = initial_shas.get(&config_path).unwrap().clone();

    // Modify config
    let updated_content = r#"{"compilerOptions": {"baseUrl": "./src"}}"#;
    fs::write(&config_path, updated_content).unwrap();

    // Compute new SHA
    let updated_shas = provider.compute_shas(&paths).unwrap();
    let updated_sha = updated_shas.get(&config_path).unwrap();

    // SHAs should differ
    assert_ne!(
        initial_sha, *updated_sha,
        "SHA should change when config changes"
    );
}
