//! TDD test for settings integration with project resolver
//!
//! Tests that providers can read real config paths from settings
//! and compute SHAs for actual files.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use codanna::config::Settings;
use codanna::project_resolver::{
    ResolutionResult, Sha256Hash, provider::ProjectResolutionProvider,
    registry::SimpleProviderRegistry, sha::compute_file_sha,
};

/// Test provider that reads actual config paths from settings
struct SettingsAwareTypeScriptProvider;

impl ProjectResolutionProvider for SettingsAwareTypeScriptProvider {
    fn language_id(&self) -> &'static str {
        "typescript"
    }

    fn is_enabled(&self, settings: &Settings) -> bool {
        // Check if TypeScript is enabled in settings
        settings
            .languages
            .get("typescript")
            .map(|config| config.enabled)
            .unwrap_or(true) // Default to enabled if not configured
    }

    fn config_paths(&self, settings: &Settings) -> Vec<PathBuf> {
        // Read from consolidated settings.languages["typescript"].config_files
        settings
            .languages
            .get("typescript")
            .map(|config| config.config_files.clone())
            .unwrap_or_default()
    }

    fn compute_shas(&self, configs: &[PathBuf]) -> ResolutionResult<HashMap<PathBuf, Sha256Hash>> {
        let mut shas = HashMap::with_capacity(configs.len()); // Pre-allocate
        for path in configs {
            if path.exists() {
                println!("Computing SHA for existing file: {}", path.display());
                let sha = compute_file_sha(path)?;
                println!("  SHA: {}", sha.0);
                shas.insert(path.to_path_buf(), sha); // Only allocate when storing
            } else {
                println!("Skipping non-existent file: {}", path.display());
            }
        }
        Ok(shas)
    }

    fn rebuild_cache(&self, settings: &Settings) -> ResolutionResult<()> {
        let paths = self.config_paths(settings);
        let _shas = self.compute_shas(&paths)?;
        // In real implementation, would store these
        Ok(())
    }

    fn select_affected_files(
        &self,
        _indexer: &codanna::indexing::SimpleIndexer,
        _settings: &Settings,
    ) -> Vec<PathBuf> {
        vec![]
    }
}

#[test]
fn provider_reads_config_paths_from_settings() {
    println!("\n=== Testing: Provider should read config paths from settings ===");

    // Load actual settings from .codanna/settings.toml if it exists
    let settings = if let Ok(loaded) = Settings::load() {
        println!("Loaded settings from .codanna/settings.toml");
        loaded
    } else {
        println!("Using default settings (no .codanna/settings.toml found)");
        Settings::default()
    };

    let provider = SettingsAwareTypeScriptProvider;
    let paths = provider.config_paths(&settings);

    println!("Expected: Provider reads paths from settings");
    println!("Got paths: {paths:?}");

    // If settings are loaded and configured, we should have paths
    if settings.languages.contains_key("typescript") {
        let ts_config = &settings.languages["typescript"];
        println!(
            "TypeScript config_files in settings: {:?}",
            ts_config.config_files
        );
    }
}

#[test]
fn provider_computes_shas_for_real_files() {
    use std::fs;

    println!("\n=== Testing: Provider computes SHAs for real config files ===");

    // Create a real test tsconfig file
    let test_dir = tempfile::tempdir().unwrap();
    let tsconfig_path = test_dir.path().join("tsconfig.json");

    let tsconfig_content = r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@app/*": ["src/app/*"],
      "@lib/*": ["src/lib/*"]
    }
  }
}"#;

    fs::write(&tsconfig_path, tsconfig_content).unwrap();
    println!("Created test tsconfig at: {}", tsconfig_path.display());

    // Provider should compute SHA for this real file
    let provider = SettingsAwareTypeScriptProvider;
    let shas = provider
        .compute_shas(&[tsconfig_path.to_path_buf()])
        .unwrap();

    println!("Computed {} SHA(s)", shas.len());
    assert_eq!(shas.len(), 1, "Should compute SHA for one file");

    let sha = shas.get(&tsconfig_path).unwrap();
    println!("SHA for test tsconfig: {}", sha.0);

    // Verify SHA is deterministic
    let shas2 = provider
        .compute_shas(&[tsconfig_path.to_path_buf()])
        .unwrap();
    let sha2 = shas2.get(&tsconfig_path).unwrap();

    println!("Recomputed SHA: {}", sha2.0);
    assert_eq!(sha, sha2, "SHA should be deterministic");
}

#[test]
fn settings_integration_end_to_end() {
    println!("\n=== Testing: End-to-end settings → provider → SHA flow ===");

    // Load real settings with configured TypeScript paths
    let settings = Settings::load().expect("Should load settings from .codanna/settings.toml");

    // Create provider and verify it reads config paths
    let provider = Arc::new(SettingsAwareTypeScriptProvider);

    // Get config paths from settings
    let config_paths = provider.config_paths(&settings);
    println!("Config paths from settings: {config_paths:?}");

    // Verify logic handles empty config_files correctly
    if config_paths.is_empty() {
        println!("Config paths empty - this is valid when config_files = [] in settings");
    } else {
        println!("Found {} config path(s)", config_paths.len());
    }

    // Compute SHAs for the actual files
    if !config_paths.is_empty() {
        let shas = provider.compute_shas(&config_paths).unwrap_or_default();
        println!("Computed SHAs for {} config files", shas.len());

        for (path, sha) in &shas {
            println!("  {}: {}", path.display(), sha.0);
        }

        // If tsconfig.json exists, verify we got its SHA
        let tsconfig_path = PathBuf::from("tsconfig.json");
        if tsconfig_path.exists() {
            assert!(
                shas.contains_key(&tsconfig_path),
                "Should compute SHA for existing tsconfig.json"
            );
            println!("✓ Successfully computed SHA for tsconfig.json");
        }
    }

    // Test registry integration
    let mut registry = SimpleProviderRegistry::new();
    registry.add(provider);

    let active = registry.active_providers(&settings);
    assert_eq!(active.len(), 1, "Should have TypeScript provider active");

    // Test cache rebuild
    let result = active[0].rebuild_cache(&settings);
    assert!(result.is_ok(), "Should rebuild cache successfully");
    println!("✓ Cache rebuild successful");
}
