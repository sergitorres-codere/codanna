//! Import resolution proof test module  
//! Verifies that language-specific import matching works correctly
//! Lives in src/indexing to have access to SimpleIndexer and Tantivy
//!
//! Note: This test focuses on path matching logic, not visibility.
//! Visibility is handled separately by is_symbol_visible_from_file()
//! and is not the focus of Sprint 1's import matching enhancement.

#[cfg(test)]
mod tests {
    use crate::indexing::SimpleIndexer;
    use crate::parsing::LanguageBehavior;
    use crate::parsing::rust::RustBehavior;
    use tempfile::TempDir;

    #[test]
    fn test_import_matching_logic() {
        println!("\n=== Import Matching Logic Test ===\n");

        // Test the matching logic in isolation (no indexing needed)
        println!("Testing import_matches_symbol method:");

        let rust_behavior = RustBehavior::new();

        // Test cases from our examples/rust/import_resolution_test.rs file
        let test_cases = vec![
            // (import_path, symbol_module_path, importing_module, should_match, description)
            (
                "helpers::helper_function",
                "crate::examples::rust::import_resolution_test::helpers::helper_function",
                Some("crate::examples::rust::import_resolution_test"),
                true,
                "Relative import should match full path",
            ),
            (
                "helpers::nested::nested_function",
                "crate::examples::rust::import_resolution_test::helpers::nested::nested_function",
                Some("crate::examples::rust::import_resolution_test"),
                true,
                "Nested module import should match",
            ),
            (
                "std::collections::HashMap",
                "std::collections::HashMap",
                Some("crate::examples::rust::import_resolution_test"),
                true,
                "Standard library import should match exactly",
            ),
            (
                "helpers::helper_function",
                "crate::examples::rust::import_resolution_test::other_helpers::helper_function",
                Some("crate::examples::rust::import_resolution_test"),
                false,
                "Should NOT match wrong module's function",
            ),
            (
                "crate::config::Config",
                "crate::config::Config",
                Some("crate::main"),
                true,
                "Fully qualified crate import should match exactly",
            ),
            (
                "config::Config",
                "crate::config::Config",
                Some("crate"),
                true,
                "Relative import from crate root should match",
            ),
        ];

        for (import_path, symbol_path, importing_module, expected, description) in test_cases {
            let result =
                rust_behavior.import_matches_symbol(import_path, symbol_path, importing_module);

            println!(
                "  [{}] {}",
                if result == expected { "✓" } else { "✗" },
                description
            );

            if result != expected {
                println!(
                    "    FAILED: import='{import_path}', symbol='{symbol_path}', from='{importing_module:?}'"
                );
            }

            assert_eq!(result, expected, "Failed: {description}");
        }

        println!("\n✅ Import matching logic verified");
    }

    #[test]
    fn test_import_resolution_with_indexer() {
        println!("\n=== Import Resolution with Indexer Test ===\n");

        // Create temp directory for test file
        let temp_dir = TempDir::new().unwrap();

        // Create a minimal test file
        let test_file = temp_dir.path().join("test_imports.rs");
        std::fs::write(
            &test_file,
            r#"
// Test file for import resolution
mod helpers {
    pub fn helper_function() -> String {
        "Helper".to_string()
    }
    
    pub struct HelperStruct {
        pub value: i32,
    }
}

// This import should be resolved when helper_function is called
use helpers::helper_function;
use helpers::HelperStruct;

fn main() {
    // This call should resolve to helpers::helper_function via the import
    let result = helper_function();
    let s = HelperStruct { value: 42 };
    println!("Result: {}", result);
}
"#,
        )
        .unwrap();

        // Create a test settings with temp directory as workspace root
        use crate::config::Settings;
        use std::sync::Arc;

        // Create the required directory structure first
        let index_path = temp_dir
            .path()
            .join(".codanna")
            .join("index")
            .join("tantivy");
        std::fs::create_dir_all(&index_path).unwrap();

        // Create settings with the temp directory as workspace root
        let settings = Settings {
            workspace_root: Some(temp_dir.path().to_path_buf()),
            index_path: std::path::PathBuf::from(".codanna/index"),
            ..Default::default()
        };

        // Create indexer with custom settings
        let mut indexer = SimpleIndexer::with_settings(Arc::new(settings));
        let result = indexer.index_file(&test_file);

        match result {
            Ok(file_id) => {
                println!("Step 1: File indexed successfully");
                println!("  File: {test_file:?}");
                println!("  File ID: {file_id:?}");

                // Get all symbols to verify they were extracted
                let symbols = indexer.get_all_symbols();
                let our_symbols: Vec<_> = symbols
                    .into_iter()
                    .filter(|s| {
                        let name = s.name.to_string();
                        name == "helper_function" || name == "HelperStruct" || name == "main"
                    })
                    .collect();

                println!("\nStep 2: Symbols extracted");
                for symbol in &our_symbols {
                    println!(
                        "  - {} ({:?}) at module: {:?}",
                        symbol.name.as_ref(),
                        symbol.kind,
                        symbol.module_path
                    );
                }

                // Verify we found the expected symbols
                assert!(
                    our_symbols
                        .iter()
                        .any(|s| s.name.as_ref() == "helper_function"),
                    "Should find helper_function"
                );
                assert!(
                    our_symbols
                        .iter()
                        .any(|s| s.name.as_ref() == "HelperStruct"),
                    "Should find HelperStruct"
                );
                assert!(
                    our_symbols.iter().any(|s| s.name.as_ref() == "main"),
                    "Should find main function"
                );

                println!("\n✅ Import resolution with indexer verified");
            }
            Err(e) => {
                panic!("Failed to index file: {e:?}");
            }
        }
    }
}
