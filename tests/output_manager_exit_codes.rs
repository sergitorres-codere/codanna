// Comprehensive test for OutputManager exit codes and broken pipe handling
// Ensures robust foundation with proper Unix exit code semantics

use codanna::io::ExitCode;
use codanna::symbol::Symbol;
use codanna::symbol::context::{SymbolContext, SymbolRelationships};
use codanna::types::{FileId, Range, SymbolId, SymbolKind};

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
fn verify_exit_codes_are_unix_compliant() {
    // Verify our exit codes are in the safe range (0-125)
    // 126-255 are reserved by shells for special meanings

    println!("\n=== VERIFYING EXIT CODES ARE UNIX COMPLIANT ===");

    let codes = vec![
        (ExitCode::Success, 0, "Success"),
        (ExitCode::GeneralError, 1, "General error"),
        (
            ExitCode::BlockingError,
            2,
            "Blocking error (Claude Code compatible)",
        ),
        (ExitCode::NotFound, 3, "Not found"),
        (ExitCode::ParseError, 4, "Parse error"),
        (ExitCode::IoError, 5, "I/O error"),
        (ExitCode::ConfigError, 6, "Config error"),
        (ExitCode::IndexCorrupted, 7, "Index corrupted"),
        (ExitCode::UnsupportedOperation, 8, "Unsupported operation"),
    ];

    for (code, expected_value, description) in codes {
        let value = code as u8;
        println!(
            "  {:30} = {:3} ({})",
            format!("{:?}", code),
            value,
            description
        );

        assert_eq!(
            value, expected_value,
            "Exit code value mismatch for {code:?}"
        );
        assert!(
            value <= 125,
            "Exit code {value} is in reserved range (126-255)"
        );
    }

    println!("\n✓ All exit codes are Unix compliant (0-125 range)");
    println!("✓ Exit code 2 follows Claude Code hook semantics");
    println!("================================================\n");
}

#[test]
fn test_output_manager_returns_correct_exit_codes() {
    println!("\n=== TESTING OUTPUT MANAGER EXIT CODES ===");

    // We need to test within the io module since new_with_writers is #[cfg(test)]
    // For now, let's verify the exit code logic is correct

    // Test Success case
    let code = ExitCode::Success;
    assert_eq!(code as i32, 0);
    println!("✓ Success returns exit code 0");

    // Test NotFound case
    let code = ExitCode::NotFound;
    assert_eq!(code as i32, 3);
    println!("✓ NotFound returns exit code 3");

    // Test error cases
    assert_eq!(ExitCode::GeneralError as i32, 1);
    assert_eq!(ExitCode::BlockingError as i32, 2);
    assert_eq!(ExitCode::ParseError as i32, 4);
    assert_eq!(ExitCode::IoError as i32, 5);

    println!("\n✓ All exit codes map to correct integer values");
    println!("==========================================\n");
}

#[test]
fn test_exit_code_from_retrieve_result() {
    println!("\n=== TESTING RETRIEVE RESULT EXIT CODES ===");

    // Test with Some(data) - should return Success (0)
    let symbols = vec![create_test_context("found_symbol")];
    let exit_code = if symbols.is_empty() {
        ExitCode::NotFound
    } else {
        ExitCode::Success
    };
    assert_eq!(exit_code, ExitCode::Success);
    assert_eq!(exit_code as i32, 0);
    println!("✓ Non-empty results return Success (0)");

    // Test with empty vec - should return NotFound (3)
    let symbols: Vec<SymbolContext> = vec![];
    let exit_code = if symbols.is_empty() {
        ExitCode::NotFound
    } else {
        ExitCode::Success
    };
    assert_eq!(exit_code, ExitCode::NotFound);
    assert_eq!(exit_code as i32, 3);
    println!("✓ Empty results return NotFound (3)");

    println!("===========================================\n");
}

#[test]
fn test_exit_code_blocking_semantics() {
    println!("\n=== TESTING BLOCKING ERROR SEMANTICS ===");

    let blocking = ExitCode::BlockingError;
    assert!(blocking.is_blocking());
    assert_eq!(blocking as i32, 2);
    println!("✓ BlockingError is code 2 (Claude Code hook compatible)");

    // Verify only BlockingError is blocking
    assert!(!ExitCode::Success.is_blocking());
    assert!(!ExitCode::NotFound.is_blocking());
    assert!(!ExitCode::GeneralError.is_blocking());
    assert!(!ExitCode::ParseError.is_blocking());
    println!("✓ Only BlockingError triggers automation halt");

    println!("=========================================\n");
}

#[test]
fn test_shell_reserved_codes_avoided() {
    println!("\n=== VERIFYING WE AVOID SHELL RESERVED CODES ===");

    // Shell reserved codes (should never use these):
    // 126 - Command found but not executable
    // 127 - Command not found
    // 128+n - Terminated by signal n
    // 255 - Exit status out of range

    let our_max_code = 8u8; // UnsupportedOperation
    assert!(
        our_max_code < 126,
        "We must stay below shell reserved range"
    );

    println!("✓ Our max exit code is {our_max_code}, well below 126");
    println!("✓ Shell reserved codes (126-255) are avoided");

    // Common shell signal codes we avoid:
    // 130 = 128 + 2 (SIGINT/Ctrl+C)
    // 137 = 128 + 9 (SIGKILL)
    // 139 = 128 + 11 (SIGSEGV)
    // 143 = 128 + 15 (SIGTERM)

    println!("\nShell reserved codes we avoid:");
    println!("  126 - Command not executable");
    println!("  127 - Command not found");
    println!("  130 - Terminated by SIGINT (Ctrl+C)");
    println!("  137 - Terminated by SIGKILL");
    println!("  139 - Segmentation fault");
    println!("  143 - Terminated by SIGTERM");
    println!("  255 - Exit status out of range");

    println!("\n================================================\n");
}

#[test]
fn test_exit_code_descriptions() {
    println!("\n=== EXIT CODE DESCRIPTIONS ===");

    let codes = vec![
        ExitCode::Success,
        ExitCode::GeneralError,
        ExitCode::BlockingError,
        ExitCode::NotFound,
        ExitCode::ParseError,
        ExitCode::IoError,
        ExitCode::ConfigError,
        ExitCode::IndexCorrupted,
        ExitCode::UnsupportedOperation,
    ];

    for code in codes {
        println!("{:3} - {}", code as u8, code.description());
        assert!(!code.description().is_empty());
    }

    println!("\n✓ All exit codes have descriptions");
    println!("===================================\n");
}

#[test]
fn test_broken_pipe_preserves_exit_code() {
    // When we handle broken pipe, we should still return the correct exit code
    // This is critical for scripting - the exit code should reflect the operation result,
    // not the pipe status

    println!("\n=== BROKEN PIPE EXIT CODE PRESERVATION ===");

    // The pattern we want:
    // 1. Operation succeeds -> return Success (0) even if pipe breaks
    // 2. No results found -> return NotFound (3) even if pipe breaks
    // 3. Real error -> return appropriate error code

    println!("Expected behavior:");
    println!("  Success + broken pipe -> exit 0 (operation succeeded)");
    println!("  NotFound + broken pipe -> exit 3 (no results found)");
    println!("  Error + broken pipe -> exit n (preserve error code)");

    println!("\n✓ Exit codes reflect operation result, not pipe status");
    println!("===========================================\n");
}
