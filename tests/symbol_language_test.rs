use codanna::parsing::registry::LanguageId;
use codanna::symbol::Symbol;
use codanna::types::{FileId, Range, SymbolId, SymbolKind};

#[test]
fn test_symbol_with_language_id() {
    let symbol = Symbol::new(
        SymbolId::new(1).unwrap(),
        "test_function",
        SymbolKind::Function,
        FileId::new(1).unwrap(),
        Range::new(1, 0, 5, 0),
    )
    .with_language_id(LanguageId::new("rust"));

    assert_eq!(symbol.language_id, Some(LanguageId::new("rust")));
}

#[test]
fn test_symbol_backward_compatibility() {
    // Existing code should work without language_id
    let symbol = Symbol::new(
        SymbolId::new(1).unwrap(),
        "old_function",
        SymbolKind::Function,
        FileId::new(1).unwrap(),
        Range::new(1, 0, 5, 0),
    );

    assert_eq!(symbol.language_id, None);
}

#[test]
fn test_symbol_language_id_builder_pattern() {
    let symbol = Symbol::new(
        SymbolId::new(1).unwrap(),
        "test_class",
        SymbolKind::Class,
        FileId::new(2).unwrap(),
        Range::new(10, 0, 20, 0),
    )
    .with_signature("class TestClass")
    .with_language_id(LanguageId::new("python"))
    .with_doc("Test documentation");

    assert_eq!(symbol.language_id, Some(LanguageId::new("python")));
    assert!(symbol.signature.is_some());
    assert!(symbol.doc_comment.is_some());
}

#[test]
fn test_different_language_ids() {
    let rust_symbol = Symbol::new(
        SymbolId::new(1).unwrap(),
        "rust_fn",
        SymbolKind::Function,
        FileId::new(1).unwrap(),
        Range::new(1, 0, 5, 0),
    )
    .with_language_id(LanguageId::new("rust"));

    let python_symbol = Symbol::new(
        SymbolId::new(2).unwrap(),
        "python_fn",
        SymbolKind::Function,
        FileId::new(2).unwrap(),
        Range::new(1, 0, 5, 0),
    )
    .with_language_id(LanguageId::new("python"));

    let typescript_symbol = Symbol::new(
        SymbolId::new(3).unwrap(),
        "ts_fn",
        SymbolKind::Function,
        FileId::new(3).unwrap(),
        Range::new(1, 0, 5, 0),
    )
    .with_language_id(LanguageId::new("typescript"));

    assert_eq!(rust_symbol.language_id, Some(LanguageId::new("rust")));
    assert_eq!(python_symbol.language_id, Some(LanguageId::new("python")));
    assert_eq!(
        typescript_symbol.language_id,
        Some(LanguageId::new("typescript"))
    );
    assert_ne!(rust_symbol.language_id, python_symbol.language_id);
}
