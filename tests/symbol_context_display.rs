// Test for SymbolContext Display trait implementation
// This test should fail initially (TDD Red phase)

use codanna::symbol::Symbol;
use codanna::symbol::context::{SymbolContext, SymbolRelationships};
use codanna::types::{FileId, Range, SymbolId, SymbolKind};

/// Helper function to create a test symbol
fn create_test_symbol(name: &str) -> Symbol {
    Symbol::new(
        SymbolId::new(42).unwrap(),
        name,
        SymbolKind::Function,
        FileId::new(1).unwrap(),
        Range::new(100, 4, 120, 5),
    )
    .with_signature("fn test_function(a: &str) -> Result<(), Error>")
    .with_doc("Test function for Display trait")
}

/// Helper function to create a test SymbolContext
fn create_test_symbol_context() -> SymbolContext {
    let symbol = create_test_symbol("calculate_similarity");

    SymbolContext {
        symbol,
        file_path: "src/vector/similarity.rs:101".to_string(),
        relationships: SymbolRelationships::default(),
    }
}

#[test]
fn test_symbol_context_implements_display() {
    let context = create_test_symbol_context();

    // This should compile only if Display is implemented
    // Currently this will fail compilation (TDD Red phase)
    let display_str = format!("{context}");

    // Verify output contains key information
    assert!(display_str.contains("calculate_similarity"));
    assert!(display_str.contains("Function"));
    assert!(display_str.contains("src/vector/similarity.rs:101"));
}

#[test]
fn test_symbol_context_display_with_relationships() {
    let mut context = create_test_symbol_context();

    // Add some relationships
    let helper_symbol = create_test_symbol("helper_func");
    context.relationships.calls = Some(vec![(helper_symbol, Some("direct".to_string()))]);

    // Should work even with populated relationships
    let display_str = format!("{context}");
    assert!(!display_str.is_empty());

    // The display should still contain basic info
    assert!(display_str.contains("calculate_similarity"));
}

#[test]
fn test_symbol_context_display_various_kinds() {
    // Test with different symbol kinds
    let test_cases = vec![
        (SymbolKind::Function, "test_func"),
        (SymbolKind::Struct, "TestStruct"),
        (SymbolKind::Trait, "TestTrait"),
        (SymbolKind::Method, "test_method"),
        (SymbolKind::Class, "TestClass"),
    ];

    for (kind, name) in test_cases {
        let symbol = Symbol::new(
            SymbolId::new(1).unwrap(),
            name,
            kind,
            FileId::new(1).unwrap(),
            Range::new(10, 0, 20, 0),
        );

        let context = SymbolContext {
            symbol,
            file_path: "src/test.rs:11".to_string(),
            relationships: SymbolRelationships::default(),
        };

        let display_str = format!("{context}");
        assert!(display_str.contains(name));
        assert!(!display_str.is_empty());
    }
}
