//! Verification test for TypeScript import extraction
//! This test provides PROOF that imports are correctly extracted
//! by parsing the actual examples/typescript/import_test.ts file

use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::types::FileId;
use std::fs;

#[test]
fn verify_typescript_imports_with_proof() {
    println!("\n=== TYPESCRIPT IMPORT EXTRACTION VERIFICATION ===");
    println!("Parsing: examples/typescript/import_test.ts");
    println!("This test provides PROOF of what imports are extracted\n");

    // Read the actual test file
    let code =
        fs::read_to_string("examples/typescript/import_test.ts").expect("Failed to read test file");

    let mut parser = TypeScriptParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();

    // Extract imports
    let imports = parser.find_imports(&code, file_id);

    println!("FOUND {} IMPORTS:\n", imports.len());
    println!("{:<50} | {:<20} | {:<10}", "PATH", "ALIAS", "IS_GLOB");
    println!("{}", "-".repeat(85));

    for (i, import) in imports.iter().enumerate() {
        println!(
            "{:2}. {:<47} | {:<20} | {:<10}",
            i + 1,
            import.path,
            import.alias.as_deref().unwrap_or("None"),
            if import.is_glob { "true" } else { "false" }
        );
    }

    println!("\n=== VERIFICATION AGAINST EXPECTATIONS ===\n");

    // Define our expectations based on the ACTUAL test file content
    let expectations = vec![
        // Named imports (per specifier)
        (
            "react",
            Some("Component"),
            false,
            "Named imports from react - Component",
        ),
        (
            "react",
            Some("useState"),
            false,
            "Named imports from react - useState",
        ),
        (
            "./utils/helper",
            Some("H"),
            false,
            "Named imports with alias from helpers - H",
        ),
        (
            "./utils/helper",
            Some("util"),
            false,
            "Named imports with alias from helpers - util",
        ),
        // Default imports
        ("react", Some("React"), false, "Default import React"),
        (
            "./components/Button",
            Some("Button"),
            false,
            "Default import Button",
        ),
        // Namespace imports
        ("lodash", Some("lodash"), true, "Namespace import lodash"),
        ("../utils", Some("utils"), true, "Namespace import utils"),
        // Mixed imports (default + named) -> two entries
        ("./mixed", Some("namedExport"), false, "Mixed named export"),
        (
            "./mixed",
            Some("DefaultExport"),
            false,
            "Mixed default export",
        ),
        // Type-only named imports (per specifier)
        ("./types", Some("Props"), false, "Type-only Props"),
        ("./types", Some("State"), false, "Type-only State"),
        // Mixed type and value imports
        (
            "./config",
            Some("Config"),
            false,
            "Mixed type import Config",
        ),
        (
            "./config",
            Some("createConfig"),
            false,
            "Mixed value import createConfig",
        ),
        // Side-effect imports
        ("./styles.css", None, false, "Side-effect import"),
        ("polyfill", None, false, "Side-effect import (no extension)"),
        // Re-exports
        ("react", None, false, "Re-export named"),
        ("./utils", None, true, "Re-export all"),
        ("./Button", None, false, "Re-export default as named"),
        ("./utils/helper", None, false, "Re-export with rename"),
        ("./types", None, false, "Type re-export"),
        // Path variations (per specifier)
        ("./sibling", Some("something"), false, "Sibling import"),
        (
            "../parent",
            Some("parent"),
            false,
            "Parent directory import",
        ),
        (
            "../../deep/module",
            Some("deep"),
            false,
            "Deep parent import",
        ),
        (
            "./folder",
            Some("indexed"),
            false,
            "Folder import (implies index)",
        ),
        // Scoped packages (per specifier)
        (
            "@types/express",
            Some("Request"),
            false,
            "Scoped package import",
        ),
        ("@app/services", Some("service"), false, "App-scoped import"),
    ];

    println!(
        "{:<5} | {:<25} | {:<15} | {:<10} | {:<10} | Description",
        "Test", "Path", "Alias", "IsGlob", "Result"
    );
    println!("{}", "-".repeat(100));

    let mut all_passed = true;
    for (i, (expected_path, expected_alias, expected_glob, description)) in
        expectations.iter().enumerate()
    {
        let found = imports.iter().find(|imp| {
            imp.path == *expected_path
                && imp.alias.as_deref() == *expected_alias
                && imp.is_glob == *expected_glob
        });

        let passed = found.is_some();
        if !passed {
            all_passed = false;
        }

        println!(
            "{:4}. | {:<25} | {:<15} | {:<10} | {:<10} | {}",
            i + 1,
            expected_path,
            expected_alias.unwrap_or("None"),
            if *expected_glob { "true" } else { "false" },
            if passed { "✅ PASS" } else { "❌ FAIL" },
            description
        );

        if !passed {
            // Show what we actually found with this path
            let matching_path = imports
                .iter()
                .filter(|imp| imp.path == *expected_path)
                .collect::<Vec<_>>();
            if !matching_path.is_empty() {
                println!("      FOUND WITH PATH '{expected_path}': ");
                for imp in matching_path {
                    println!("        - alias={:?}, is_glob={}", imp.alias, imp.is_glob);
                }
            } else {
                println!("      NOT FOUND: No import with path '{expected_path}'");
            }
        }
    }

    println!("\n=== SUMMARY ===");
    println!("Expected imports: {}", expectations.len());
    println!("Found imports: {}", imports.len());
    println!(
        "All tests passed: {}",
        if all_passed { "✅ YES" } else { "❌ NO" }
    );

    // Show any unexpected imports
    println!("\n=== UNEXPECTED IMPORTS (if any) ===");
    let mut has_unexpected = false;
    for import in &imports {
        let is_expected = expectations.iter().any(|(path, alias, glob, _)| {
            import.path == *path && import.alias.as_deref() == *alias && import.is_glob == *glob
        });

        if !is_expected {
            has_unexpected = true;
            println!(
                "  - path='{}', alias={:?}, is_glob={}",
                import.path, import.alias, import.is_glob
            );
        }
    }

    if !has_unexpected {
        println!("  None - all imports were expected!");
    }

    assert!(all_passed, "Some import expectations were not met!");
    assert_eq!(
        imports.len(),
        expectations.len(),
        "Number of imports doesn't match expectations"
    );

    println!("\n✅ ALL VERIFICATIONS PASSED!");
}
