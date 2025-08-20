//! Quick Regression Tests for Go Parser Integration
//!
//! These tests ensure the Go parser integration doesn't break existing functionality.
//! They're designed to run quickly and catch critical regressions.

use codanna::Settings;
use codanna::parsing::registry::{LanguageId, get_registry};
use codanna::parsing::{Language, ParserFactory};
use std::sync::Arc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Test that all expected languages are properly registered
#[test]
fn test_all_languages_registered() -> Result<()> {
    println!("Testing language registration...");

    let registry_guard = get_registry().lock().unwrap();
    let expected_languages = vec![
        (LanguageId::new("rust"), "rs"),
        (LanguageId::new("python"), "py"),
        (LanguageId::new("typescript"), "ts"),
        (LanguageId::new("php"), "php"),
        (LanguageId::new("go"), "go"),
    ];

    for (lang_id, extension) in expected_languages {
        // Test language is available
        assert!(
            registry_guard.is_available(lang_id),
            "Language {lang_id:?} should be available"
        );

        // Test extension mapping
        assert!(
            registry_guard.get_by_extension(extension).is_some(),
            "Extension '{extension}' should be mapped to a language"
        );

        println!("  ✓ {lang_id} ({extension}) is properly registered");
    }

    println!("✅ All languages are properly registered");
    Ok(())
}

/// Test that all languages can create parsers
#[test]
fn test_all_parsers_creatable() -> Result<()> {
    println!("Testing parser creation...");

    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings.clone());

    let languages = vec![
        Language::Rust,
        Language::Python,
        Language::TypeScript,
        Language::Php,
        Language::Go,
    ];

    for language in languages {
        let result = factory.create_parser_with_behavior(language);
        assert!(
            result.is_ok(),
            "Should be able to create {} parser: {:?}",
            language,
            result.err()
        );
        println!("  ✓ {language} parser created successfully");
    }

    println!("✅ All parsers can be created");
    Ok(())
}

/// Test that Go parser is specifically working
#[test]
fn test_go_parser_functional() -> Result<()> {
    println!("Testing Go parser functionality...");

    // Test 1: Go is registered
    let registry_guard = get_registry().lock().unwrap();
    let go_id = LanguageId::new("go");
    assert!(registry_guard.is_available(go_id), "Go should be available");
    println!("  ✓ Go is registered");

    // Test 2: Can create Go parser
    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings);
    let go_parser_result = factory.create_parser_with_behavior(Language::Go);
    assert!(go_parser_result.is_ok(), "Should create Go parser");
    println!("  ✓ Go parser can be created");

    // Test 3: Go extension is mapped
    assert!(
        registry_guard.get_by_extension("go").is_some(),
        "Should recognize .go files"
    );
    println!("  ✓ .go extension is mapped");

    // Test 4: Go parser can handle basic parsing (minimal test)
    let mut parser = go_parser_result.unwrap().parser;
    let simple_go_code = "package main\nfunc main() {}";
    let file_id = codanna::types::FileId::new(1).unwrap();
    let mut counter = codanna::types::SymbolCounter::new();

    // This should not panic, even if it returns empty results
    let _symbols = parser.parse(simple_go_code, file_id, &mut counter);
    println!("  ✓ Go parser handles basic code without panicking");

    println!("✅ Go parser is functional");
    Ok(())
}

/// Test MCP-related functionality
#[test]
fn test_mcp_integration() -> Result<()> {
    println!("Testing MCP integration...");

    let registry_guard = get_registry().lock().unwrap();
    let settings = Settings::default();

    // Test that Go is in enabled extensions
    let enabled_extensions: Vec<&str> = registry_guard.enabled_extensions(&settings).collect();

    assert!(
        enabled_extensions.contains(&"go"),
        "Go extension should be enabled: {enabled_extensions:?}"
    );
    println!("  ✓ Go extension is in enabled list");

    // Test registry integrity
    let all_count = registry_guard.iter_all().count();
    assert!(all_count >= 5, "Should have at least 5 languages");
    println!("  ✓ Registry has {all_count} languages");

    println!("✅ MCP integration works");
    Ok(())
}

/// Comprehensive quick test that runs all checks
#[test]
fn test_comprehensive_regression() -> Result<()> {
    println!("\n🧪 Running Comprehensive Quick Regression Test");
    println!("{}", "=".repeat(50));

    test_all_languages_registered()?;
    test_all_parsers_creatable()?;
    test_go_parser_functional()?;
    test_mcp_integration()?;

    println!("\n🎉 All regression checks passed!");
    println!("   Go parser integration is working correctly");
    println!("   and has not broken existing functionality.");
    println!("{}", "=".repeat(50));

    Ok(())
}
