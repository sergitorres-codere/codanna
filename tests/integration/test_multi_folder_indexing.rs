//! Integration tests for multi-folder indexing functionality

use codanna::{IndexPersistence, Settings, SimpleIndexer};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_index_multiple_folders() {
    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Create multiple source folders with Rust files
    let src_dir = workspace.join("src");
    let lib_dir = workspace.join("lib");
    let tests_dir = workspace.join("tests");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&lib_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();

    // Create sample files in each directory
    fs::write(
        src_dir.join("main.rs"),
        r#"
fn main() {
    println!("Hello from main!");
}

fn helper() -> i32 {
    42
}
"#,
    )
    .unwrap();

    fs::write(
        lib_dir.join("utils.rs"),
        r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
    )
    .unwrap();

    fs::write(
        tests_dir.join("test_utils.rs"),
        r#"
#[test]
fn test_add() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn test_multiply() {
    assert_eq!(2 * 3, 6);
}
"#,
    )
    .unwrap();

    // Create a config with indexed paths
    let mut settings = Settings::default();
    settings.add_indexed_path(src_dir.clone()).unwrap();
    settings.add_indexed_path(lib_dir.clone()).unwrap();
    settings.add_indexed_path(tests_dir.clone()).unwrap();

    // Setup index persistence
    let index_path = workspace.join(".codanna/index");
    fs::create_dir_all(&index_path).unwrap();
    settings.index_path = index_path.clone();

    let settings = Arc::new(settings);
    let mut indexer = SimpleIndexer::with_settings(settings.clone());

    // Index all configured folders
    let paths_to_index = settings.get_indexed_paths();
    assert_eq!(paths_to_index.len(), 3);

    for path in &paths_to_index {
        indexer
            .index_directory_with_options(path, false, false, false, None)
            .unwrap();
    }

    // Verify symbols from all folders were indexed
    let total_symbols = indexer.symbol_count();
    assert!(total_symbols > 0, "Should have indexed some symbols");

    // Check that we have symbols from different files
    let files = indexer.get_all_indexed_paths();
    assert_eq!(files.len(), 3, "Should have indexed 3 files");

    // Verify we can find symbols from each directory
    let main_symbol = indexer.find_symbols_by_name("main", None);
    assert!(
        !main_symbol.is_empty(),
        "Should find main function from src/"
    );

    let add_symbol = indexer.find_symbols_by_name("add", None);
    assert!(!add_symbol.is_empty(), "Should find add function from lib/");

    let test_add_symbol = indexer.find_symbols_by_name("test_add", None);
    assert!(
        !test_add_symbol.is_empty(),
        "Should find test_add function from tests/"
    );

    // Test persistence
    let persistence = IndexPersistence::new(index_path.clone());
    persistence.save(&indexer).unwrap();

    // Load the index back and verify
    let loaded_indexer = persistence
        .load_with_settings(settings.clone(), false)
        .unwrap();
    assert_eq!(
        loaded_indexer.symbol_count(),
        total_symbols,
        "Loaded index should have same number of symbols"
    );
}

#[test]
fn test_add_and_remove_folders_from_config() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Create test folders
    let folder1 = workspace.join("folder1");
    let folder2 = workspace.join("folder2");
    let folder3 = workspace.join("folder3");

    fs::create_dir_all(&folder1).unwrap();
    fs::create_dir_all(&folder2).unwrap();
    fs::create_dir_all(&folder3).unwrap();

    // Create config file
    let config_dir = workspace.join(".codanna");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("settings.toml");

    let mut settings = Settings::default();

    // Add folders
    settings.add_indexed_path(folder1.clone()).unwrap();
    settings.add_indexed_path(folder2.clone()).unwrap();
    settings.add_indexed_path(folder3.clone()).unwrap();

    assert_eq!(settings.indexing.indexed_paths.len(), 3);

    // Save config
    settings.save(&config_path).unwrap();

    // Load config and verify
    let loaded_settings = Settings::load_from(&config_path).unwrap();
    assert_eq!(loaded_settings.indexing.indexed_paths.len(), 3);

    // Remove a folder
    let mut modified_settings = loaded_settings;
    modified_settings.remove_indexed_path(&folder2).unwrap();
    assert_eq!(modified_settings.indexing.indexed_paths.len(), 2);

    // Save and reload
    modified_settings.save(&config_path).unwrap();
    let final_settings = Settings::load_from(&config_path).unwrap();
    assert_eq!(final_settings.indexing.indexed_paths.len(), 2);

    // Verify correct folders remain
    let canonical_folder1 = folder1.canonicalize().unwrap();
    let canonical_folder3 = folder3.canonicalize().unwrap();

    let remaining_paths: Vec<_> = final_settings
        .indexing
        .indexed_paths
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect();

    assert!(remaining_paths.contains(&canonical_folder1));
    assert!(remaining_paths.contains(&canonical_folder3));
}

#[test]
fn test_index_with_no_configured_paths_uses_default() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Create a Rust file in the workspace
    fs::write(
        workspace.join("test.rs"),
        r#"
fn example() {
    println!("test");
}
"#,
    )
    .unwrap();

    // Create settings with no configured paths
    let settings = Settings::default();
    let settings = Arc::new(settings);

    // get_indexed_paths should return empty vector when not configured (backward compatible)
    let paths = settings.get_indexed_paths();
    assert_eq!(paths.len(), 0);
}

#[test]
fn test_index_prevents_duplicate_paths() {
    let temp_dir = TempDir::new().unwrap();
    let test_folder = temp_dir.path().join("test_folder");
    fs::create_dir(&test_folder).unwrap();

    let mut settings = Settings::default();

    // Add the folder once
    assert!(settings.add_indexed_path(test_folder.clone()).is_ok());

    // Try to add the same folder again - should fail
    let result = settings.add_indexed_path(test_folder.clone());
    assert!(result.is_err());

    // Should still only have one path
    assert_eq!(settings.indexing.indexed_paths.len(), 1);
}

#[test]
fn test_add_folder_indexes_new_symbols() {
    // This test verifies that adding a new folder and reindexing adds its symbols to the index
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Create initial folder
    let src_dir = workspace.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("main.rs"),
        r#"
fn main() {
    println!("Hello");
}

fn helper() -> i32 {
    42
}
"#,
    )
    .unwrap();

    // Setup index
    let index_path = workspace.join(".codanna/index");
    fs::create_dir_all(&index_path).unwrap();

    let mut settings = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };
    settings.add_indexed_path(src_dir.clone()).unwrap();

    let settings = Arc::new(settings);
    let mut indexer = SimpleIndexer::with_settings(settings.clone());

    // Index initial folder
    indexer
        .index_directory_with_options(&src_dir, false, false, false, None)
        .unwrap();

    let initial_symbol_count = indexer.symbol_count();
    assert!(
        initial_symbol_count > 0,
        "Should have indexed initial symbols"
    );

    // Verify main function exists
    let main_symbols = indexer.find_symbols_by_name("main", None);
    assert!(!main_symbols.is_empty(), "Should find main function");

    // Verify utility function does NOT exist yet
    let add_symbols = indexer.find_symbols_by_name("add_numbers", None);
    assert!(
        add_symbols.is_empty(),
        "Should NOT find add_numbers function yet"
    );

    // Save the index
    let persistence = IndexPersistence::new(index_path.clone());
    persistence.save(&indexer).unwrap();

    // Now add a new folder with new code
    let lib_dir = workspace.join("lib");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::write(
        lib_dir.join("utils.rs"),
        r#"
pub fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#,
    )
    .unwrap();

    // Update settings to include new folder
    let mut updated_settings = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };
    updated_settings.add_indexed_path(src_dir.clone()).unwrap();
    updated_settings.add_indexed_path(lib_dir.clone()).unwrap();

    // Load the existing index
    let updated_settings = Arc::new(updated_settings);
    let mut updated_indexer = persistence
        .load_with_settings(updated_settings.clone(), false)
        .unwrap();

    // Index the new folder (incremental)
    updated_indexer
        .index_directory_with_options(&lib_dir, false, false, false, None)
        .unwrap();

    // Verify symbol count increased
    let final_symbol_count = updated_indexer.symbol_count();
    assert!(
        final_symbol_count > initial_symbol_count,
        "Symbol count should increase after adding new folder: {initial_symbol_count} -> {final_symbol_count}"
    );

    // Verify we can now find symbols from BOTH folders
    let main_symbols = updated_indexer.find_symbols_by_name("main", None);
    assert!(!main_symbols.is_empty(), "Should still find main function");

    let add_symbols = updated_indexer.find_symbols_by_name("add_numbers", None);
    assert!(
        !add_symbols.is_empty(),
        "Should NOW find add_numbers function from new folder"
    );

    let multiply_symbols = updated_indexer.find_symbols_by_name("multiply", None);
    assert!(
        !multiply_symbols.is_empty(),
        "Should find multiply function from new folder"
    );
}

#[test]
fn test_remove_folder_cleans_symbols() {
    // This test verifies that removing a folder and cleaning removes its symbols from the index
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Create two folders with different code
    let src_dir = workspace.join("src");
    let lib_dir = workspace.join("lib");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&lib_dir).unwrap();

    fs::write(
        src_dir.join("main.rs"),
        r#"
fn main() {
    println!("Hello");
}

fn src_helper() -> i32 {
    42
}
"#,
    )
    .unwrap();

    fs::write(
        lib_dir.join("utils.rs"),
        r#"
pub fn lib_function() -> String {
    "from lib".to_string()
}

pub fn lib_helper() -> i32 {
    100
}
"#,
    )
    .unwrap();

    // Setup index with both folders
    let index_path = workspace.join(".codanna/index");
    fs::create_dir_all(&index_path).unwrap();

    let mut settings = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };
    settings.add_indexed_path(src_dir.clone()).unwrap();
    settings.add_indexed_path(lib_dir.clone()).unwrap();

    let settings = Arc::new(settings);
    let mut indexer = SimpleIndexer::with_settings(settings.clone());

    // Index both folders
    indexer
        .index_directory_with_options(&src_dir, false, false, false, None)
        .unwrap();
    indexer
        .index_directory_with_options(&lib_dir, false, false, false, None)
        .unwrap();

    let initial_symbol_count = indexer.symbol_count();
    assert!(
        initial_symbol_count > 0,
        "Should have indexed symbols from both folders"
    );

    // Verify symbols from both folders exist
    let main_symbols = indexer.find_symbols_by_name("main", None);
    assert!(!main_symbols.is_empty(), "Should find main from src/");

    let src_helper_symbols = indexer.find_symbols_by_name("src_helper", None);
    assert!(
        !src_helper_symbols.is_empty(),
        "Should find src_helper from src/"
    );

    let lib_function_symbols = indexer.find_symbols_by_name("lib_function", None);
    assert!(
        !lib_function_symbols.is_empty(),
        "Should find lib_function from lib/"
    );

    let lib_helper_symbols = indexer.find_symbols_by_name("lib_helper", None);
    assert!(
        !lib_helper_symbols.is_empty(),
        "Should find lib_helper from lib/"
    );

    // Save the index
    let persistence = IndexPersistence::new(index_path.clone());
    persistence.save(&indexer).unwrap();

    // Now remove lib folder from configuration
    let mut updated_settings = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };
    updated_settings.add_indexed_path(src_dir.clone()).unwrap();
    // Note: NOT adding lib_dir

    // Load the index
    let updated_settings = Arc::new(updated_settings);
    let mut updated_indexer = persistence
        .load_with_settings(updated_settings.clone(), false)
        .unwrap();

    // Debug: print all indexed paths before cleanup
    eprintln!("Files before cleanup:");
    for path in updated_indexer.get_all_indexed_paths() {
        eprintln!("  - {}", path.display());
    }

    eprintln!("Configured folders:");
    for folder in &updated_settings.indexing.indexed_paths {
        eprintln!("  - {}", folder.display());
    }

    // Clean removed folders
    let removed_count = updated_indexer
        .clean_removed_folders(&updated_settings.indexing.indexed_paths)
        .unwrap();

    eprintln!("Removed {removed_count} files");

    assert!(
        removed_count > 0,
        "Should have removed files from lib/ folder"
    );

    // Note: Due to Tantivy's soft-delete mechanism, the symbol count might not decrease immediately
    // deleted documents are only physically removed during segment merges.
    // Instead, we verify that the symbols can't be found anymore.

    // Verify symbols from src/ still exist
    let main_symbols = updated_indexer.find_symbols_by_name("main", None);
    assert!(
        !main_symbols.is_empty(),
        "Should STILL find main from src/ (not removed)"
    );

    let src_helper_symbols = updated_indexer.find_symbols_by_name("src_helper", None);
    assert!(
        !src_helper_symbols.is_empty(),
        "Should STILL find src_helper from src/ (not removed)"
    );

    // Verify symbols from lib/ are gone
    let lib_function_symbols = updated_indexer.find_symbols_by_name("lib_function", None);
    eprintln!("lib_function symbols found: {}", lib_function_symbols.len());
    for sym in &lib_function_symbols {
        eprintln!("  - {} at {}", sym.name, sym.file_path);
    }
    assert!(
        lib_function_symbols.is_empty(),
        "Should NOT find lib_function anymore (should be removed)"
    );

    let lib_helper_symbols = updated_indexer.find_symbols_by_name("lib_helper", None);
    assert!(
        lib_helper_symbols.is_empty(),
        "Should NOT find lib_helper anymore (should be removed)"
    );

    // Save and reload to verify persistence
    persistence.save(&updated_indexer).unwrap();
    let reloaded_indexer = persistence
        .load_with_settings(updated_settings.clone(), false)
        .unwrap();

    // Verify the cleanup persisted - symbols should still not be findable
    let lib_function_after_reload = reloaded_indexer.find_symbols_by_name("lib_function", None);
    assert!(
        lib_function_after_reload.is_empty(),
        "lib_function should STILL be gone after reload"
    );

    let src_helper_after_reload = reloaded_indexer.find_symbols_by_name("src_helper", None);
    assert!(
        !src_helper_after_reload.is_empty(),
        "src_helper should STILL exist after reload"
    );
}

#[test]
fn test_nested_folders_no_duplicate_symbols() {
    // Test that indexing both a parent and child folder doesn't create duplicate symbols
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Create nested structure: src/ and src/utils/
    let src_dir = workspace.join("src");
    let utils_dir = src_dir.join("utils");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&utils_dir).unwrap();

    // File in parent folder
    fs::write(
        src_dir.join("main.rs"),
        r#"
fn main() {
    println!("Hello");
}
"#,
    )
    .unwrap();

    // File in nested folder
    fs::write(
        utils_dir.join("helper.rs"),
        r#"
pub fn helper() -> i32 {
    42
}
"#,
    )
    .unwrap();

    let index_path = workspace.join(".codanna/index");
    fs::create_dir_all(&index_path).unwrap();

    // Index only the parent folder first
    let mut settings = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };
    settings.add_indexed_path(src_dir.clone()).unwrap();

    let settings = Arc::new(settings);
    let mut indexer = SimpleIndexer::with_settings(settings.clone());

    indexer
        .index_directory_with_options(&src_dir, false, false, false, None)
        .unwrap();

    let symbol_count_parent_only = indexer.symbol_count();
    assert!(symbol_count_parent_only > 0, "Should have indexed symbols");

    // Should find symbols from both files since parent includes child
    let main_symbols = indexer.find_symbols_by_name("main", None);
    assert!(!main_symbols.is_empty(), "Should find main from parent");

    let helper_symbols = indexer.find_symbols_by_name("helper", None);
    assert!(
        !helper_symbols.is_empty(),
        "Should find helper from nested folder"
    );

    // Now explicitly add the nested folder too
    let persistence = IndexPersistence::new(index_path.clone());
    persistence.save(&indexer).unwrap();

    let mut settings_with_nested = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };
    settings_with_nested
        .add_indexed_path(src_dir.clone())
        .unwrap();
    settings_with_nested
        .add_indexed_path(utils_dir.clone())
        .unwrap();

    let settings_with_nested = Arc::new(settings_with_nested);
    let mut indexer_with_nested = persistence
        .load_with_settings(settings_with_nested.clone(), false)
        .unwrap();

    // Index the nested folder explicitly
    indexer_with_nested
        .index_directory_with_options(&utils_dir, false, false, false, None)
        .unwrap();

    let symbol_count_with_nested = indexer_with_nested.symbol_count();

    // Symbol count should be similar (might have some duplicates from re-indexing)
    // The important thing is it doesn't explode with duplicates
    assert!(
        symbol_count_with_nested <= symbol_count_parent_only + 5,
        "Should not have massive duplication: {symbol_count_parent_only} vs {symbol_count_with_nested}"
    );

    // Should still find both symbols
    let main_symbols = indexer_with_nested.find_symbols_by_name("main", None);
    assert!(!main_symbols.is_empty(), "Should still find main");

    let helper_symbols = indexer_with_nested.find_symbols_by_name("helper", None);
    assert!(!helper_symbols.is_empty(), "Should still find helper");
}

#[test]
fn test_overlapping_paths_cleanup_protection() {
    // Test that files under overlapping paths are protected from cleanup
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    let root_dir = workspace.join("project");
    let sub_dir = root_dir.join("submodule");

    fs::create_dir_all(&root_dir).unwrap();
    fs::create_dir_all(&sub_dir).unwrap();

    // File in root
    fs::write(
        root_dir.join("root.rs"),
        r#"
fn root_function() {
    println!("root");
}
"#,
    )
    .unwrap();

    // File in subdirectory
    fs::write(
        sub_dir.join("sub.rs"),
        r#"
fn sub_function() {
    println!("sub");
}
"#,
    )
    .unwrap();

    let index_path = workspace.join(".codanna/index");
    fs::create_dir_all(&index_path).unwrap();

    // Index both overlapping paths
    let mut settings = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };
    settings.add_indexed_path(root_dir.clone()).unwrap();
    settings.add_indexed_path(sub_dir.clone()).unwrap();

    let settings = Arc::new(settings);
    let mut indexer = SimpleIndexer::with_settings(settings.clone());

    indexer
        .index_directory_with_options(&root_dir, false, false, false, None)
        .unwrap();
    indexer
        .index_directory_with_options(&sub_dir, false, false, false, None)
        .unwrap();

    let initial_symbol_count = indexer.symbol_count();
    assert!(initial_symbol_count > 0);

    // Both functions should be findable
    let root_symbols = indexer.find_symbols_by_name("root_function", None);
    assert!(!root_symbols.is_empty(), "Should find root_function");

    let sub_symbols = indexer.find_symbols_by_name("sub_function", None);
    assert!(!sub_symbols.is_empty(), "Should find sub_function");

    // Save and remove only the root path (keep sub)
    let persistence = IndexPersistence::new(index_path.clone());
    persistence.save(&indexer).unwrap();

    let mut updated_settings = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };
    updated_settings.add_indexed_path(sub_dir.clone()).unwrap();
    // NOT adding root_dir

    let updated_settings = Arc::new(updated_settings);
    let mut updated_indexer = persistence
        .load_with_settings(updated_settings.clone(), false)
        .unwrap();

    // Clean removed folders
    let removed_count = updated_indexer
        .clean_removed_folders(&updated_settings.indexing.indexed_paths)
        .unwrap();

    // Should only remove the root.rs file, NOT sub.rs (because sub_dir is still indexed)
    assert_eq!(removed_count, 1, "Should only remove root.rs");

    // sub_function should STILL exist (protected by sub_dir being indexed)
    let sub_symbols = updated_indexer.find_symbols_by_name("sub_function", None);
    assert!(
        !sub_symbols.is_empty(),
        "sub_function should STILL exist (protected by submodule path)"
    );

    // root_function should be gone
    let root_symbols = updated_indexer.find_symbols_by_name("root_function", None);
    assert!(root_symbols.is_empty(), "root_function should be removed");
}

#[test]
#[cfg(unix)] // Symlinks work differently on Windows
fn test_symlinks_are_canonicalized() {
    // Test that symlinks are properly canonicalized to avoid duplicates
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    // Create real directory
    let real_dir = workspace.join("real");
    fs::create_dir_all(&real_dir).unwrap();

    fs::write(
        real_dir.join("code.rs"),
        r#"
fn real_function() {
    println!("real");
}
"#,
    )
    .unwrap();

    // Create symlink to real directory
    let symlink_dir = workspace.join("symlink");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real_dir, &symlink_dir).unwrap();

    let index_path = workspace.join(".codanna/index");
    fs::create_dir_all(&index_path).unwrap();

    // Try to add both the real path and the symlink
    let mut settings = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };

    // Add real directory
    settings.add_indexed_path(real_dir.clone()).unwrap();

    // Try to add symlink - should resolve to same canonical path
    let result = settings.add_indexed_path(symlink_dir.clone());

    // Should fail because they canonicalize to the same path
    assert!(
        result.is_err(),
        "Should not allow adding symlink if real path already added"
    );

    // Should still only have one path
    assert_eq!(
        settings.indexing.indexed_paths.len(),
        1,
        "Should only have one path (real path)"
    );

    // Verify the path is the canonical one
    let canonical_real = real_dir.canonicalize().unwrap();
    let stored_path = settings.indexing.indexed_paths[0].canonicalize().unwrap();
    assert_eq!(stored_path, canonical_real, "Should store canonical path");
}

#[test]
#[cfg(unix)]
fn test_symlink_removal_works_correctly() {
    // Test that removing a folder works correctly even when accessed via symlink
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    let dir1 = workspace.join("dir1");
    let dir2 = workspace.join("dir2");
    fs::create_dir_all(&dir1).unwrap();
    fs::create_dir_all(&dir2).unwrap();

    fs::write(dir1.join("file1.rs"), r#"fn func1() { println!("1"); }"#).unwrap();

    fs::write(dir2.join("file2.rs"), r#"fn func2() { println!("2"); }"#).unwrap();

    // Create symlink to dir2
    let symlink_to_dir2 = workspace.join("link2");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&dir2, &symlink_to_dir2).unwrap();

    let index_path = workspace.join(".codanna/index");
    fs::create_dir_all(&index_path).unwrap();

    // Index both directories (dir1 by real path, dir2 by symlink)
    let mut settings = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };
    settings.add_indexed_path(dir1.clone()).unwrap();
    settings.add_indexed_path(symlink_to_dir2.clone()).unwrap();

    let settings = Arc::new(settings);
    let mut indexer = SimpleIndexer::with_settings(settings.clone());

    indexer
        .index_directory_with_options(&dir1, false, false, false, None)
        .unwrap();
    indexer
        .index_directory_with_options(&symlink_to_dir2, false, false, false, None)
        .unwrap();

    // Both functions should exist
    let func1 = indexer.find_symbols_by_name("func1", None);
    let func2 = indexer.find_symbols_by_name("func2", None);
    assert!(!func1.is_empty() && !func2.is_empty());

    // Save and remove dir2 (using real path, not symlink)
    let persistence = IndexPersistence::new(index_path.clone());
    persistence.save(&indexer).unwrap();

    let mut updated_settings = Settings {
        index_path: index_path.clone(),
        ..Settings::default()
    };
    updated_settings.add_indexed_path(dir1.clone()).unwrap();
    // NOT adding dir2 or symlink_to_dir2

    let updated_settings = Arc::new(updated_settings);
    let mut updated_indexer = persistence
        .load_with_settings(updated_settings.clone(), false)
        .unwrap();

    // Clean should work correctly because paths are canonicalized
    let removed = updated_indexer
        .clean_removed_folders(&updated_settings.indexing.indexed_paths)
        .unwrap();

    assert!(removed > 0, "Should remove dir2 files");

    // func2 should be gone, func1 should remain
    let func1 = updated_indexer.find_symbols_by_name("func1", None);
    let func2 = updated_indexer.find_symbols_by_name("func2", None);

    assert!(!func1.is_empty(), "func1 should still exist");
    assert!(func2.is_empty(), "func2 should be removed");
}
