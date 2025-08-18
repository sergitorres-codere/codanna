//! Tests for OutputManager unified output handling

use codanna::io::{EntityType, ExitCode, OutputFormat, OutputManager, UnifiedOutputBuilder};
use codanna::symbol::Symbol;
use codanna::types::{FileId, Range, SymbolId, SymbolKind};
use std::sync::{Arc, Mutex};

fn create_test_symbol() -> Symbol {
    Symbol::new(
        SymbolId::new(1).unwrap(),
        "test_function",
        SymbolKind::Function,
        FileId::new(1).unwrap(),
        Range::new(10, 0, 15, 0),
    )
}

#[test]
fn test_unified_output_json() {
    println!("\n=== TEST: Unified Output JSON Format ===");

    let output = Arc::new(Mutex::new(Vec::new()));
    let output_clone = output.clone();
    let stderr = Vec::new();

    struct OutputCapture {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl std::io::Write for OutputCapture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Json,
        Box::new(OutputCapture {
            buffer: output_clone,
        }),
        Box::new(stderr),
    );

    let symbols = vec![create_test_symbol()];
    println!("  Created {} test symbol(s)", symbols.len());

    let unified = UnifiedOutputBuilder::items(symbols, EntityType::Symbol).build();
    println!("  Built UnifiedOutput with status: {:?}", unified.status);
    println!("  Exit code should be: {:?}", unified.exit_code);

    let exit_code = manager.unified(unified).unwrap();
    println!("  Manager returned exit code: {exit_code:?}");

    // Check the JSON output
    let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
    println!("  JSON output length: {} bytes", output_str.len());

    // Pretty print first few lines of JSON for verification
    let lines: Vec<&str> = output_str.lines().take(10).collect();
    println!("  JSON sample (first 10 lines):");
    for line in &lines {
        println!("    {line}");
    }
    if output_str.lines().count() > 10 {
        println!("    ... ({} more lines)", output_str.lines().count() - 10);
    }

    // Verify it's valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&output_str).expect("Output should be valid JSON");
    println!("  JSON structure validated successfully");

    // Check key fields exist
    assert!(
        parsed.get("status").is_some(),
        "JSON should have 'status' field"
    );
    assert!(
        parsed.get("entity_type").is_some(),
        "JSON should have 'entity_type' field"
    );
    assert!(
        parsed.get("count").is_some(),
        "JSON should have 'count' field"
    );
    assert!(
        parsed.get("items").is_some(),
        "JSON should have 'items' field for Items variant"
    );

    println!("  All required JSON fields present");
    assert_eq!(exit_code, ExitCode::Success);
    println!("=== TEST PASSED ===\n");
}

#[test]
fn test_unified_output_text() {
    println!("\n=== TEST: Unified Output Text Format ===");

    let output = Arc::new(Mutex::new(Vec::new()));
    let output_clone = output.clone();
    let stderr = Vec::new();

    struct OutputCapture {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl std::io::Write for OutputCapture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Text,
        Box::new(OutputCapture {
            buffer: output_clone,
        }),
        Box::new(stderr),
    );

    let symbols = vec![create_test_symbol()];
    println!("  Created {} test symbol(s)", symbols.len());

    let unified = UnifiedOutputBuilder::items(symbols, EntityType::Symbol).build();
    println!("  Built UnifiedOutput with status: {:?}", unified.status);

    let exit_code = manager.unified(unified).unwrap();
    println!("  Manager returned exit code: {exit_code:?}");

    // Check the text output
    let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
    println!("  Text output: '{}'", output_str.trim());

    // Verify output contains expected content
    assert!(!output_str.is_empty(), "Text output should not be empty");
    assert!(
        output_str.contains("test_function"),
        "Output should contain the symbol name"
    );

    assert_eq!(exit_code, ExitCode::Success);
    println!("=== TEST PASSED ===\n");
}

#[test]
fn test_unified_output_empty() {
    println!("\n=== TEST: Unified Output Empty Collection ===");

    let output = Vec::new();
    let stderr = Arc::new(Mutex::new(Vec::new()));
    let stderr_clone = stderr.clone();

    struct StderrCapture {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl std::io::Write for StderrCapture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Text,
        Box::new(output),
        Box::new(StderrCapture {
            buffer: stderr_clone,
        }),
    );

    let symbols: Vec<Symbol> = vec![];
    println!("  Created empty symbol collection");

    let unified = UnifiedOutputBuilder::items(symbols, EntityType::Symbol).build();
    println!("  Built UnifiedOutput with status: {:?}", unified.status);
    println!("  Exit code should be: {:?}", unified.exit_code);

    let exit_code = manager.unified(unified).unwrap();
    println!("  Manager returned exit code: {exit_code:?}");
    assert_eq!(exit_code, ExitCode::NotFound);

    // Check that error message went to stderr
    let stderr_str = String::from_utf8(stderr.lock().unwrap().clone()).unwrap();
    println!("  Stderr content: '{}'", stderr_str.trim());

    // Verify the error message
    assert!(
        stderr_str.contains("not found"),
        "Expected 'not found' in stderr: {stderr_str}"
    );
    println!("  Error message correctly sent to stderr");
    println!("=== TEST PASSED ===\n");
}

#[test]
fn test_unified_output_with_guidance() {
    println!("\n=== TEST: Unified Output with AI Guidance ===");

    let output = Vec::new();
    let stderr = Arc::new(Mutex::new(Vec::new()));
    let stderr_clone = stderr.clone();

    struct StderrCapture {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl std::io::Write for StderrCapture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut manager = OutputManager::new_with_writers(
        OutputFormat::Text,
        Box::new(output),
        Box::new(StderrCapture {
            buffer: stderr_clone,
        }),
    );

    let symbols = vec![create_test_symbol()];
    println!("  Created {} test symbol(s)", symbols.len());

    let guidance_msg = "Consider using 'find_symbol' for more details";
    println!("  Adding guidance: '{guidance_msg}'");

    let unified = UnifiedOutputBuilder::items(symbols, EntityType::Symbol)
        .with_guidance(guidance_msg)
        .build();
    println!("  Built UnifiedOutput with guidance attached");

    let exit_code = manager.unified(unified).unwrap();
    println!("  Manager returned exit code: {exit_code:?}");
    assert_eq!(exit_code, ExitCode::Success);

    // Check that guidance went to stderr
    let stderr_str = String::from_utf8(stderr.lock().unwrap().clone()).unwrap();
    println!("  Stderr content: '{}'", stderr_str.trim());

    assert!(
        stderr_str.contains("Consider using 'find_symbol'"),
        "Guidance message should be in stderr"
    );
    println!("  Guidance message correctly sent to stderr");
    println!("=== TEST PASSED ===\n");
}
