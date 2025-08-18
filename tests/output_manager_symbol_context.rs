// Test for OutputManager::symbol_contexts() method
// TDD Red phase - write failing test first

use codanna::io::{ExitCode, OutputFormat, OutputManager};
use codanna::symbol::Symbol;
use codanna::symbol::context::{SymbolContext, SymbolRelationships};
use codanna::types::{FileId, Range, SymbolId, SymbolKind};
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Mock writer that wraps an Arc<Mutex<Vec<u8>>> for testing
struct MockWriter(Arc<Mutex<Vec<u8>>>);

impl Write for MockWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}
/// Helper to create a test SymbolContext
fn create_test_context(
    id: u32,
    name: &str,
    kind: SymbolKind,
    file: &str,
    line: u32,
) -> SymbolContext {
    let symbol = Symbol::new(
        SymbolId::new(id).unwrap(),
        name,
        kind,
        FileId::new(1).unwrap(),
        Range::new(line, 0, line + 10, 0),
    );

    SymbolContext {
        symbol,
        file_path: format!("{}:{}", file, line + 1), // line + 1 for 1-based indexing
        relationships: SymbolRelationships::default(),
    }
}

#[test]
#[ignore = "TDD Red phase - implementation pending"]
fn test_symbol_contexts_with_multiple_items() {
    let stdout = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let stderr = Vec::new();

    let stdout_clone = stdout.clone();
    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Json,
        Box::new(MockWriter(stdout_clone)),
        Box::new(stderr),
    );

    let contexts = vec![
        create_test_context(1, "main", SymbolKind::Function, "src/main.rs", 10),
        create_test_context(2, "calculate", SymbolKind::Function, "src/lib.rs", 20),
        create_test_context(3, "Parser", SymbolKind::Struct, "src/parser.rs", 30),
    ];

    let exit_code = manager.symbol_contexts(contexts, "symbols").unwrap();
    assert_eq!(exit_code, ExitCode::Success);

    // Verify JSON output contains all symbols
    let output = String::from_utf8(stdout.lock().unwrap().clone()).unwrap();
    assert!(output.contains("\"main\""));
    assert!(output.contains("\"calculate\""));
    assert!(output.contains("\"Parser\""));
    assert!(output.contains("\"success\": true"));
}

#[test]
fn test_symbol_contexts_empty_returns_not_found() {
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let stderr = Vec::new();

    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Json,
        Box::new(MockWriter(stdout.clone())),
        Box::new(stderr),
    );

    let contexts: Vec<SymbolContext> = vec![];

    let exit_code = manager.symbol_contexts(contexts, "symbols").unwrap();
    assert_eq!(exit_code, ExitCode::NotFound);
}

#[test]
fn test_symbol_contexts_text_format() {
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let stderr = Vec::new();

    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Text,
        Box::new(MockWriter(stdout.clone())),
        Box::new(stderr),
    );

    let contexts = vec![
        create_test_context(
            1,
            "process_file",
            SymbolKind::Function,
            "src/processor.rs",
            100,
        ),
        create_test_context(
            2,
            "validate_input",
            SymbolKind::Function,
            "src/validator.rs",
            50,
        ),
    ];

    let exit_code = manager.symbol_contexts(contexts, "functions").unwrap();
    assert_eq!(exit_code, ExitCode::Success);

    let output = String::from_utf8(stdout.lock().unwrap().clone()).unwrap();

    // Check text format output
    assert!(output.contains("Found 2 functions:"));
    assert!(output.contains("Function process_file at src/processor.rs:101"));
    assert!(output.contains("Function validate_input at src/validator.rs:51"));
}

#[test]
fn test_symbol_contexts_single_item() {
    let stdout = Vec::new();
    let stderr = Vec::new();

    let mut manager =
        OutputManager::new_with_writers(OutputFormat::Json, Box::new(stdout), Box::new(stderr));

    let contexts = vec![create_test_context(
        1,
        "singleton",
        SymbolKind::Class,
        "src/pattern.rs",
        42,
    )];

    let exit_code = manager.symbol_contexts(contexts, "class").unwrap();
    assert_eq!(exit_code, ExitCode::Success);
}

#[test]
fn test_symbol_contexts_preserves_all_fields() {
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let stderr = Vec::new();

    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Json,
        Box::new(MockWriter(stdout.clone())),
        Box::new(stderr),
    );

    let mut symbol = Symbol::new(
        SymbolId::new(1).unwrap(),
        "documented_function",
        SymbolKind::Function,
        FileId::new(1).unwrap(),
        Range::new(10, 0, 20, 0),
    );

    // Add documentation and signature
    symbol.doc_comment = Some("This function does something important".to_string().into());
    symbol.signature = Some(
        "fn documented_function(x: i32) -> String"
            .to_string()
            .into(),
    );

    let context = SymbolContext {
        symbol,
        file_path: "src/documented.rs:11".to_string(),
        relationships: SymbolRelationships::default(),
    };

    let exit_code = manager.symbol_contexts(vec![context], "function").unwrap();
    assert_eq!(exit_code, ExitCode::Success);

    let output = String::from_utf8(stdout.lock().unwrap().clone()).unwrap();

    // Verify all fields are preserved in JSON
    assert!(output.contains("\"documented_function\""));
    assert!(output.contains("\"doc_comment\""));
    assert!(output.contains("This function does something important"));
    assert!(output.contains("\"signature\""));
    assert!(output.contains("fn documented_function(x: i32) -> String"));
    assert!(output.contains("\"file_path\""));
    assert!(output.contains("src/documented.rs:11"));
}
