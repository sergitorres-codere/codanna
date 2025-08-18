// Test for OutputManager broken pipe handling
// This should fail initially (TDD Red phase) then pass after implementation

use codanna::io::{OutputManager, OutputFormat, ExitCode};
use codanna::symbol::context::{SymbolContext, SymbolRelationships};
use codanna::symbol::Symbol;
use codanna::types::{FileId, Range, SymbolId, SymbolKind};
use std::io::{self, Write};

/// A writer that always returns broken pipe error
struct BrokenPipeWriter;

impl Write for BrokenPipeWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "pipe broken"))
    }
    
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "pipe broken"))
    }
}

/// Helper to create a test SymbolContext
fn create_test_context(name: &str) -> SymbolContext {
    let symbol = Symbol::new(
        SymbolId::new(1).unwrap(),
        name,
        SymbolKind::Function,
        FileId::new(1).unwrap(),
        Range::new(10, 0, 20, 0),
    );
    
    SymbolContext {
        symbol,
        file_path: "src/test.rs:11".to_string(),
        relationships: SymbolRelationships::default(),
    }
}

#[test]
fn test_output_manager_success_handles_broken_pipe() {
    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Json,
        Box::new(BrokenPipeWriter),
        Box::new(Vec::new()),
    );
    
    let context = create_test_context("test_function");
    
    // Should not panic on broken pipe
    let result = manager.success(context);
    
    // We expect it to either:
    // 1. Return Ok (ignoring the broken pipe) - this is what we want
    // 2. Return the broken pipe error - acceptable but not ideal
    match result {
        Ok(exit_code) => {
            // Ideal: broken pipe is silently ignored
            assert_eq!(exit_code, ExitCode::Success);
            println!("✓ Broken pipe handled gracefully (ignored)");
        }
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
            // Acceptable: error is returned but not panicking
            println!("✓ Broken pipe returned as error (no panic)");
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

#[test]
fn test_output_manager_not_found_handles_broken_pipe() {
    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Json,
        Box::new(BrokenPipeWriter),
        Box::new(Vec::new()),
    );
    
    // Test not_found method with broken pipe
    let result = manager.not_found("Symbol", "undefined_function");
    
    match result {
        Ok(exit_code) => {
            assert_eq!(exit_code, ExitCode::NotFound);
            println!("✓ not_found: Broken pipe handled gracefully");
        }
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
            println!("✓ not_found: Broken pipe returned as error");
        }
        Err(e) => {
            panic!("Unexpected error in not_found: {:?}", e);
        }
    }
}

#[test]
fn test_output_manager_collection_handles_broken_pipe() {
    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Json,
        Box::new(BrokenPipeWriter),
        Box::new(Vec::new()),
    );
    
    let contexts = vec![
        create_test_context("func1"),
        create_test_context("func2"),
    ];
    
    // Test collection method with broken pipe
    let result = manager.collection(contexts, "symbol");
    
    match result {
        Ok(_) => {
            println!("✓ collection: Broken pipe handled gracefully");
        }
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
            println!("✓ collection: Broken pipe returned as error");
        }
        Err(e) => {
            panic!("Unexpected error in collection: {:?}", e);
        }
    }
}

#[test]
fn test_output_manager_text_format_broken_pipe() {
    // Also test with Text format
    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Text,
        Box::new(BrokenPipeWriter),
        Box::new(Vec::new()),
    );
    
    let context = create_test_context("test_text");
    
    let result = manager.success(context);
    
    match result {
        Ok(_) => {
            println!("✓ Text format: Broken pipe handled gracefully");
        }
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
            println!("✓ Text format: Broken pipe returned as error");
        }
        Err(e) => {
            panic!("Unexpected error with text format: {:?}", e);
        }
    }
}

#[test]
fn test_stderr_broken_pipe() {
    // Test stderr output (used for errors and progress)
    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Text,
        Box::new(Vec::new()), // stdout is fine
        Box::new(BrokenPipeWriter), // stderr is broken
    );
    
    // progress() writes to stderr
    let result = manager.progress("Processing...");
    
    match result {
        Ok(()) => {
            println!("✓ stderr: Broken pipe handled gracefully");
        }
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
            println!("✓ stderr: Broken pipe returned as error");
        }
        Err(e) => {
            panic!("Unexpected error with stderr: {:?}", e);
        }
    }
}