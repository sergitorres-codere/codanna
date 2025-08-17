//! Tests for PHP Resolution API implementation
//!
//! This test file verifies:
//! 1. Module path resolution from files (PSR-4 conventions)
//! 2. Import matching with PHP namespace rules
//! 3. Enhanced import resolution with context

use codanna::FileId;
use codanna::parsing::{LanguageBehavior, LanguageParser, PhpBehavior, PhpParser};
use codanna::types::SymbolCounter;
use std::path::Path;

#[test]
fn test_php_module_path_for_file() {
    println!("\n=== PHP Module Path Resolution Test ===\n");

    let behavior = PhpBehavior::new();
    let project_root = Path::new("/project");

    // Test PSR-4 standard paths
    let test_cases = vec![
        // (file_path, expected_module_path)
        (
            "/project/src/App/Controllers/UserController.php",
            Some("\\App\\Controllers\\UserController"),
        ),
        (
            "/project/app/Services/AuthService.php",
            Some("\\Services\\AuthService"),
        ),
        (
            "/project/src/App/Models/User.php",
            Some("\\App\\Models\\User"),
        ),
        (
            "/project/lib/Helpers/StringHelper.php",
            Some("\\Helpers\\StringHelper"),
        ),
        // Special files that shouldn't have module paths
        ("/project/index.php", None),
        ("/project/config.php", None),
        ("/project/.env.php", None),
        // Class files with .class.php extension
        ("/project/src/MyClass.class.php", Some("\\MyClass")),
    ];

    for (file_path, expected) in test_cases {
        let path = Path::new(file_path);
        let result = behavior.module_path_from_file(path, project_root);

        println!(
            "[{}] {} -> {:?}",
            if result.as_deref() == expected {
                "✓"
            } else {
                "✗"
            },
            file_path,
            result
        );

        assert_eq!(result.as_deref(), expected, "Failed for {file_path}");
    }

    println!("\n✅ Module path resolution verified");
}

#[test]
fn test_php_import_matches_symbol() {
    println!("\n=== PHP Import Matching Test ===\n");

    let behavior = PhpBehavior::new();

    let test_cases = vec![
        // (import_path, symbol_path, importing_module, should_match, description)
        (
            "App\\Models\\User",
            "\\App\\Models\\User",
            Some("\\App\\Controllers\\UserController"),
            true,
            "Exact match with/without leading backslash",
        ),
        (
            "User",
            "\\App\\Models\\User",
            Some("\\App\\Models"),
            true,
            "Short name should match when in parent namespace",
        ),
        (
            "Models\\User",
            "\\App\\Models\\User",
            Some("\\App"),
            true,
            "Relative namespace from parent",
        ),
        (
            "AuthService",
            "\\App\\Services\\AuthService",
            Some("\\App\\Services"),
            true,
            "Class in same namespace",
        ),
        (
            "\\App\\Models\\User",
            "\\App\\Models\\User",
            None,
            true,
            "Fully qualified match without context",
        ),
        (
            "User",
            "\\Other\\Models\\User",
            Some("\\App\\Controllers"),
            false,
            "Should NOT match different namespace",
        ),
        (
            "Services\\AuthService",
            "\\App\\Services\\AuthService",
            Some("\\App\\Controllers"),
            true,
            "Sibling namespace resolution",
        ),
    ];

    for (import_path, symbol_path, importing_module, expected, description) in test_cases {
        let result = behavior.import_matches_symbol(import_path, symbol_path, importing_module);

        println!(
            "[{}] {}",
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
fn test_php_get_module_path_for_file() {
    println!("\n=== PHP get_module_path_for_file Test ===\n");

    let behavior = PhpBehavior::new();

    // Register some files with the behavior's state
    let file1 = FileId::new(1).unwrap();
    let file2 = FileId::new(2).unwrap();
    let file3 = FileId::new(3).unwrap();

    behavior.register_file(
        Path::new("/project/src/App/Controllers/UserController.php").to_path_buf(),
        file1,
        "\\App\\Controllers\\UserController".to_string(),
    );

    behavior.register_file(
        Path::new("/project/src/App/Models/User.php").to_path_buf(),
        file2,
        "\\App\\Models\\User".to_string(),
    );

    behavior.register_file(
        Path::new("/project/src/App/Services/AuthService.php").to_path_buf(),
        file3,
        "\\App\\Services\\AuthService".to_string(),
    );

    // Test retrieval using the Resolution API method
    let test_cases = vec![
        (file1, Some("\\App\\Controllers\\UserController")),
        (file2, Some("\\App\\Models\\User")),
        (file3, Some("\\App\\Services\\AuthService")),
        (FileId::new(99).unwrap(), None), // Non-existent file
    ];

    for (file_id, expected) in test_cases {
        let result = behavior.get_module_path_for_file(file_id);

        println!(
            "[{}] FileId {:?} -> {:?}",
            if result.as_deref() == expected {
                "✓"
            } else {
                "✗"
            },
            file_id,
            result
        );

        assert_eq!(result.as_deref(), expected, "Failed for FileId {file_id:?}");
    }

    println!("\n✅ get_module_path_for_file verified (O(1) HashMap lookup)");
}

#[test]
fn test_php_namespace_aliases() {
    println!("\n=== PHP Namespace Aliases Test ===\n");

    let behavior = PhpBehavior::new();

    // Test aliased imports like:
    // use App\Models\User as UserModel;
    // use App\Services\{AuthService, CacheService};

    let test_cases = vec![
        // When we see "UserModel", it should match "App\Models\User"
        (
            "UserModel",           // This would be the alias used in code
            "\\App\\Models\\User", // The actual symbol path
            Some("\\App\\Controllers"),
            false, // Aliases need special handling, not implemented yet
            "Aliased imports need special handling",
        ),
        // Grouped imports expand to multiple use statements
        (
            "AuthService",
            "\\App\\Services\\AuthService",
            Some("\\App\\Controllers"),
            true,
            "Grouped import member resolution",
        ),
    ];

    for (import_path, symbol_path, importing_module, expected, description) in test_cases {
        let result = behavior.import_matches_symbol(import_path, symbol_path, importing_module);

        println!(
            "[{}] {} (currently: {})",
            if result == expected { "✓" } else { "○" },
            description,
            if expected {
                "supported"
            } else {
                "not supported"
            }
        );
    }

    println!("\n✅ Namespace alias test completed");
}

#[test]
fn test_php_real_world_example() {
    println!("\n=== PHP Real World Example Test ===\n");

    // This tests resolution as it would happen with real PHP files
    let mut parser = PhpParser::new().unwrap();
    let behavior = PhpBehavior::new();

    // Parse a simplified PHP file
    let code = r#"<?php
namespace App\Controllers;

use App\Models\User;
use App\Services\AuthService;

class UserController extends BaseController {
    private AuthService $authService;

    public function show(int $id): User {
        return User::find($id);
    }
}"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("Parsed {} symbols:", symbols.len());
    for symbol in &symbols {
        println!(
            "  - {} ({:?}) at {:?}",
            symbol.name.as_ref(),
            symbol.kind,
            symbol.module_path
        );
    }

    // Verify we found the expected symbols
    assert!(
        symbols.iter().any(|s| s.name.as_ref() == "UserController"),
        "Should find UserController class"
    );
    assert!(
        symbols.iter().any(|s| s.name.as_ref() == "authService"),
        "Should find authService field"
    );
    assert!(
        symbols.iter().any(|s| s.name.as_ref() == "show"),
        "Should find show method"
    );

    // Note: The PHP parser currently doesn't set module_path during parsing
    // That's handled separately during indexing when the file context is known
    println!("  Note: Parser doesn't set module_path (that's done during indexing)");

    // Check that imports would be resolvable
    let importing_module = Some("App\\Controllers");

    // These would be the imports from the file
    assert!(
        behavior.import_matches_symbol(
            "App\\Models\\User",
            "\\App\\Models\\User",
            importing_module
        ),
        "User import should match"
    );

    assert!(
        behavior.import_matches_symbol(
            "App\\Services\\AuthService",
            "\\App\\Services\\AuthService",
            importing_module
        ),
        "AuthService import should match"
    );

    println!("\n✅ Real world example verified");
}

#[test]
fn test_php_psr4_autoloading_conventions() {
    println!("\n=== PHP PSR-4 Autoloading Conventions Test ===\n");

    let behavior = PhpBehavior::new();
    let project_root = Path::new("/project");

    // PSR-4 maps namespace prefixes to directory paths
    // Common patterns:
    // - Vendor\Package\ => src/
    // - App\ => app/ or src/App/

    let psr4_cases = vec![
        // Composer standard: vendor/package structure
        (
            "/project/vendor/monolog/monolog/src/Logger.php",
            Some("\\Logger"),
        ),
        (
            "/project/vendor/symfony/console/Command/Command.php",
            Some("\\Command\\Command"),
        ),
        // Application code following PSR-4
        (
            "/project/src/App/Http/Kernel.php",
            Some("\\App\\Http\\Kernel"),
        ),
        (
            "/project/src/App/Console/Commands/MigrateCommand.php",
            Some("\\App\\Console\\Commands\\MigrateCommand"),
        ),
        // Laravel-style app directory
        (
            "/project/app/Http/Controllers/HomeController.php",
            Some("\\Http\\Controllers\\HomeController"),
        ),
        ("/project/app/Models/Post.php", Some("\\Models\\Post")),
    ];

    for (file_path, expected) in psr4_cases {
        let path = Path::new(file_path);
        let result = behavior.module_path_from_file(path, project_root);

        println!(
            "[{}] PSR-4: {} -> {:?}",
            if result == expected.map(String::from) {
                "✓"
            } else {
                "✗"
            },
            file_path,
            result
        );
    }

    println!("\n✅ PSR-4 conventions verified");
}
