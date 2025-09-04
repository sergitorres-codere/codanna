//! Consolidated tests for project_resolver module
//!
//! Tests provider trait, registry, SHA computation, memoization, and activation

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use codanna::config::Settings;
use codanna::project_resolver::{
    ResolutionError, ResolutionResult, Sha256Hash,
    memo::ResolutionMemo,
    provider::ProjectResolutionProvider,
    registry::{ResolutionProviderRegistry, SimpleProviderRegistry},
    sha::{compute_file_sha, compute_sha256},
};

// ============================================================================
// Mock Provider for Testing
// ============================================================================

#[derive(Default)]
struct MockProvider {
    id: &'static str,
    enabled: bool,
}

impl ProjectResolutionProvider for MockProvider {
    fn language_id(&self) -> &'static str {
        self.id
    }
    fn is_enabled(&self, _settings: &Settings) -> bool {
        self.enabled
    }
    fn config_paths(&self, _settings: &Settings) -> Vec<PathBuf> {
        vec![]
    }
    fn compute_shas(&self, _configs: &[PathBuf]) -> ResolutionResult<HashMap<PathBuf, Sha256Hash>> {
        Ok(Default::default())
    }
    fn rebuild_cache(&self, _settings: &Settings) -> ResolutionResult<()> {
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

// ============================================================================
// Registry Tests
// ============================================================================

#[test]
fn registry_holds_providers_and_exposes_borrowed_slice() {
    let mut registry = SimpleProviderRegistry::new();

    let p1 = Arc::new(MockProvider {
        id: "typescript",
        enabled: true,
    });
    let p2 = Arc::new(MockProvider {
        id: "python",
        enabled: false,
    });

    registry.add(p1.clone());
    registry.add(p2.clone());

    let slice = registry.providers();
    assert_eq!(slice.len(), 2);
    assert_eq!(slice[0].language_id(), "typescript");
    assert_eq!(slice[1].language_id(), "python");
}

#[test]
fn provider_enabled_flag_is_reported() {
    let settings = Settings::default();
    let enabled = MockProvider {
        id: "ts",
        enabled: true,
    };
    let disabled = MockProvider {
        id: "py",
        enabled: false,
    };

    assert!(enabled.is_enabled(&settings));
    assert!(!disabled.is_enabled(&settings));
}

#[test]
fn registry_filters_active_providers() {
    println!("\n=== Testing: Registry should filter providers by enabled status ===");

    let mut registry = SimpleProviderRegistry::new();
    registry.add(Arc::new(MockProvider {
        id: "typescript",
        enabled: true,
    }));
    registry.add(Arc::new(MockProvider {
        id: "python",
        enabled: false,
    }));
    registry.add(Arc::new(MockProvider {
        id: "go",
        enabled: true,
    }));

    println!("Setup: Added 3 providers");
    println!("  - typescript: enabled=true");
    println!("  - python: enabled=false");
    println!("  - go: enabled=true");

    let settings = Settings::default();
    let active = registry.active_providers(&settings);

    println!("\nExpected: 2 active providers (typescript, go)");
    println!("Actual: {} active providers", active.len());

    let active_ids: Vec<&str> = active.iter().map(|p| p.language_id()).collect();
    println!("Active IDs: {active_ids:?}");

    // Proof of filtering
    println!("\nVerifying filter results:");
    println!(
        "  typescript in active? Expected: true, Got: {}",
        active_ids.contains(&"typescript")
    );
    println!(
        "  python in active? Expected: false, Got: {}",
        active_ids.contains(&"python")
    );
    println!(
        "  go in active? Expected: true, Got: {}",
        active_ids.contains(&"go")
    );

    assert_eq!(active.len(), 2, "Should have exactly 2 active providers");
    assert!(
        active_ids.contains(&"typescript"),
        "TypeScript should be active"
    );
    assert!(active_ids.contains(&"go"), "Go should be active");
    assert!(
        !active_ids.contains(&"python"),
        "Python should NOT be active"
    );
}

// ============================================================================
// SHA-256 Computation Tests
// ============================================================================

#[test]
fn sha256_computation_is_deterministic() {
    let content = r#"{"compilerOptions": {"baseUrl": "."}}"#;

    let hash1 = compute_sha256(content);
    let hash2 = compute_sha256(content);

    println!("Testing: Same content should produce same SHA");
    println!("Content: {content}");
    println!("Hash 1: {}", hash1.0);
    println!("Hash 2: {}", hash2.0);
    println!("Expected: hash1 == hash2");
    println!("Actual: {} == {}: {}", hash1.0, hash2.0, hash1 == hash2);

    assert_eq!(
        hash1, hash2,
        "FAILED: Hashes should be identical for same content"
    );
}

#[test]
fn sha256_differs_for_different_content() {
    let content1 = "content A";
    let content2 = "content B";

    let hash1 = compute_sha256(content1);
    let hash2 = compute_sha256(content2);

    println!("Testing: Different content should produce different SHA");
    println!("Content 1: '{content1}'");
    println!("Content 2: '{content2}'");
    println!("Hash 1: {}", hash1.0);
    println!("Hash 2: {}", hash2.0);
    println!("Expected: hash1 != hash2");
    println!("Actual: {} != {}: {}", hash1.0, hash2.0, hash1 != hash2);

    assert_ne!(
        hash1, hash2,
        "FAILED: Different content must produce different hashes"
    );
}

#[test]
fn compute_file_sha_handles_missing_file() {
    let missing = PathBuf::from("/definitely/does/not/exist/config.json");
    let result = compute_file_sha(&missing);
    assert!(result.is_err());
}

#[test]
fn sha256hash_is_type_safe_and_hashable() {
    let mut m: HashMap<PathBuf, Sha256Hash> = HashMap::new();
    m.insert(PathBuf::from("a"), Sha256Hash("abc".into()));
    assert_eq!(m.get(&PathBuf::from("a")).unwrap().0, "abc");
}

// ============================================================================
// Memoization Tests
// ============================================================================

#[test]
fn memo_insert_and_get_returns_same_value() {
    let memo: ResolutionMemo<Vec<PathBuf>> = ResolutionMemo::new();
    let key = Sha256Hash("abc123".to_string());
    let value = vec![PathBuf::from("src/lib.ts")];

    memo.insert(key.clone(), value.clone());
    let got = memo.get(&key).expect("value present");
    assert_eq!(&*got, &value);
}

#[test]
fn memo_clear_removes_all_entries() {
    let memo: ResolutionMemo<String> = ResolutionMemo::new();
    memo.insert(Sha256Hash("k1".into()), "v1".into());
    memo.insert(Sha256Hash("k2".into()), "v2".into());

    memo.clear();
    assert!(memo.get(&Sha256Hash("k1".into())).is_none());
    assert!(memo.get(&Sha256Hash("k2".into())).is_none());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn resolution_error_provides_suggestions() {
    let e1 = ResolutionError::cache_io(PathBuf::from("/tmp/file"), std::io::Error::other("failed"));
    assert!(e1.to_string().contains("cache io error"));
    assert!(!e1.suggestion().is_empty());

    let e2 = ResolutionError::invalid_cache("bad schema");
    assert!(e2.to_string().contains("invalid cache"));
    assert!(e2.suggestion().to_lowercase().contains("delete"));
}
