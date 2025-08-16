//! Test TypeScript import matching logic
//! This verifies that import_matches_symbol works correctly for TypeScript

use codanna::parsing::LanguageBehavior;
use codanna::parsing::typescript::TypeScriptBehavior;

#[test]
fn test_typescript_import_matching() {
    println!("\n=== TypeScript Import Matching Test ===");

    let behavior = TypeScriptBehavior::new();

    // Test cases: (import_path, symbol_module_path, importing_module, expected_match, description)
    let test_cases = vec![
        // Exact matches
        ("react", "react", None, true, "Exact match without context"),
        ("lodash", "lodash", None, true, "Package exact match"),
        (
            "src.utils.helper",
            "src.utils.helper",
            None,
            true,
            "Module path exact match",
        ),
        // Relative imports with context
        (
            "./helper",
            "src.components.helper",
            Some("src.components"),
            true,
            "Same directory import",
        ),
        (
            "./utils/helper",
            "src.components.utils.helper",
            Some("src.components"),
            true,
            "Subdirectory import",
        ),
        (
            "../utils",
            "src.utils",
            Some("src.components"),
            true,
            "Parent directory import",
        ),
        (
            "../../lib/parser",
            "src.lib.parser",
            Some("src.components.button"),
            true,
            "Multiple parent levels",
        ),
        // Index file resolution
        (
            "./folder",
            "src.components.folder.index",
            Some("src.components"),
            true,
            "Index file resolution",
        ),
        (
            "../utils",
            "src.utils.index",
            Some("src.components"),
            true,
            "Parent with index",
        ),
        // Non-matches
        (
            "./helper",
            "src.utils.helper",
            Some("src.components"),
            false,
            "Wrong directory",
        ),
        (
            "../utils",
            "src.components.utils",
            Some("src.components"),
            false,
            "Wrong parent resolution",
        ),
        ("react", "react-dom", None, false, "Similar but not exact"),
        // Package imports (currently just exact match)
        (
            "@types/node",
            "@types/node",
            None,
            true,
            "Scoped package exact match",
        ),
        (
            "@app/utils",
            "@app/utils",
            None,
            true,
            "App alias exact match",
        ),
        // Without importing context (should only match exact)
        (
            "./helper",
            "src.components.helper",
            None,
            false,
            "Relative without context",
        ),
        (
            "../utils",
            "src.utils",
            None,
            false,
            "Parent without context",
        ),
    ];

    println!(
        "\n{:<30} | {:<35} | {:<25} | {:<8} | {:<8} | Description",
        "Import Path", "Symbol Module Path", "Importing Module", "Expected", "Result"
    );
    println!("{}", "-".repeat(140));

    let mut all_passed = true;
    for (import_path, symbol_module_path, importing_module, expected, description) in test_cases {
        let result =
            behavior.import_matches_symbol(import_path, symbol_module_path, importing_module);

        let passed = result == expected;
        if !passed {
            all_passed = false;
        }

        println!(
            "{:<30} | {:<35} | {:<25} | {:<8} | {:<8} | {}",
            import_path,
            symbol_module_path,
            importing_module.unwrap_or("None"),
            if expected { "true" } else { "false" },
            if passed { "✅ PASS" } else { "❌ FAIL" },
            description
        );
    }

    println!("\n=== SUMMARY ===");
    if all_passed {
        println!("✅ All tests passed!");
    } else {
        println!("❌ Some tests failed!");
    }

    assert!(all_passed, "Some import matching tests failed");
}

#[test]
fn test_typescript_path_resolution_edge_cases() {
    println!("\n=== TypeScript Path Resolution Edge Cases ===");

    let behavior = TypeScriptBehavior::new();

    // Test edge cases
    let edge_cases = vec![
        // Multiple '../' traversals
        (
            "../../../root",
            "src.root",
            Some("src.components.button.item"),
            true,
            "Three levels up",
        ),
        // Mixed path separators (normalized to dots)
        (
            "./utils",
            "src.components.utils",
            Some("src.components"),
            true,
            "Normalized path",
        ),
        // Empty importing module (root level)
        ("./helper", "helper", Some(""), true, "Root level import"),
        // Complex nested paths
        (
            "./a/b/c/d",
            "src.a.b.c.d",
            Some("src"),
            true,
            "Deep nesting",
        ),
        ("../../a/b", "src.a.b", Some("src.x.y"), true, "Up and down"),
    ];

    println!(
        "\n{:<30} | {:<35} | {:<25} | {:<8} | Description",
        "Import Path", "Symbol Module Path", "Importing Module", "Result"
    );
    println!("{}", "-".repeat(120));

    for (import_path, symbol_module_path, importing_module, expected, description) in edge_cases {
        let result =
            behavior.import_matches_symbol(import_path, symbol_module_path, importing_module);

        let passed = result == expected;

        println!(
            "{:<30} | {:<35} | {:<25} | {:<8} | {}",
            import_path,
            symbol_module_path,
            importing_module.unwrap_or("None"),
            if passed { "✅ PASS" } else { "❌ FAIL" },
            description
        );

        assert_eq!(result, expected, "Failed: {description}");
    }

    println!("\n✅ All edge cases handled correctly!");
}
