// Verification test to ensure Display output matches expectations
// This test will print actual output to verify no false positives

use codanna::symbol::context::{SymbolContext, SymbolRelationships};
use codanna::symbol::Symbol;
use codanna::types::{FileId, Range, SymbolId, SymbolKind};

#[test]
fn verify_display_output_format() {
    // Create a test symbol with known values
    let symbol = Symbol::new(
        SymbolId::new(42).unwrap(),
        "calculate_similarity",
        SymbolKind::Function,
        FileId::new(1).unwrap(),
        Range::new(100, 4, 120, 5),  // Line 100, but we expect 101 in output
    )
    .with_signature("fn calculate_similarity(a: &[f32], b: &[f32]) -> f32")
    .with_doc("Calculate cosine similarity between two vectors");

    // Create context with file_path that already includes line number
    let context = SymbolContext {
        symbol,
        file_path: "src/vector/similarity.rs:101".to_string(), // Note: line number already included!
        relationships: SymbolRelationships::default(),
    };

    // Get the Display output
    let display_output = format!("{}", context);
    
    // Print for manual verification
    println!("\n=== DISPLAY OUTPUT VERIFICATION ===");
    println!("Actual output: '{}'", display_output);
    
    // Expected format: "Function calculate_similarity at src/vector/similarity.rs:101"
    let expected_format = "Function calculate_similarity at src/vector/similarity.rs:101";
    println!("Expected format: '{}'", expected_format);
    
    // Verify exact match
    assert_eq!(display_output, expected_format, 
        "\nDisplay output doesn't match expected format!\nGot: '{}'\nExpected: '{}'",
        display_output, expected_format);
    
    // Additional checks to ensure no duplicate line numbers
    assert!(!display_output.contains(":101:"), "Found duplicate line number separator!");
    
    // Count occurrences of ":101"
    let line_number_count = display_output.matches(":101").count();
    assert_eq!(line_number_count, 1, "Line number should appear exactly once, found {} times", line_number_count);
    
    println!("✓ Display output format is correct!");
    println!("=================================\n");
}

#[test]
fn verify_different_symbol_kinds() {
    let test_cases = vec![
        (SymbolKind::Function, "test_func", "Function test_func at src/test.rs:11"),
        (SymbolKind::Struct, "TestStruct", "Struct TestStruct at src/test.rs:11"),
        (SymbolKind::Trait, "TestTrait", "Trait TestTrait at src/test.rs:11"),
        (SymbolKind::Method, "test_method", "Method test_method at src/test.rs:11"),
        (SymbolKind::Class, "TestClass", "Class TestClass at src/test.rs:11"),
        (SymbolKind::Interface, "TestInterface", "Interface TestInterface at src/test.rs:11"),
        (SymbolKind::TypeAlias, "TestType", "TypeAlias TestType at src/test.rs:11"),
        (SymbolKind::Enum, "TestEnum", "Enum TestEnum at src/test.rs:11"),
    ];
    
    println!("\n=== TESTING DIFFERENT SYMBOL KINDS ===");
    
    for (kind, name, expected) in test_cases {
        let symbol = Symbol::new(
            SymbolId::new(1).unwrap(),
            name,
            kind,
            FileId::new(1).unwrap(),
            Range::new(10, 0, 20, 0),  // Line 10 in range
        );
        
        let context = SymbolContext {
            symbol,
            file_path: "src/test.rs:11".to_string(),  // Line 11 in file_path (already adjusted)
            relationships: SymbolRelationships::default(),
        };
        
        let display_output = format!("{}", context);
        println!("{:?}: '{}'", kind, display_output);
        
        assert_eq!(display_output, expected,
            "\nMismatch for {:?}!\nGot: '{}'\nExpected: '{}'",
            kind, display_output, expected);
    }
    
    println!("✓ All symbol kinds format correctly!");
    println!("=====================================\n");
}

#[test]
fn verify_no_duplicate_line_numbers_in_path() {
    // Test with various file paths that already include line numbers
    let test_paths = vec![
        ("src/main.rs:42", "main", "Function main at src/main.rs:42"),
        ("lib/parser.rs:100", "parse", "Function parse at lib/parser.rs:100"),
        ("tests/test.rs:1", "test_fn", "Function test_fn at tests/test.rs:1"),
        ("src/very/deep/path/file.rs:9999", "deep_func", "Function deep_func at src/very/deep/path/file.rs:9999"),
    ];
    
    println!("\n=== VERIFYING NO DUPLICATE LINE NUMBERS ===");
    
    for (path, name, expected) in test_paths {
        let symbol = Symbol::new(
            SymbolId::new(1).unwrap(),
            name,
            SymbolKind::Function,
            FileId::new(1).unwrap(),
            Range::new(50, 0, 60, 0),  // Different line in range (should be ignored)
        );
        
        let context = SymbolContext {
            symbol,
            file_path: path.to_string(),
            relationships: SymbolRelationships::default(),
        };
        
        let display_output = format!("{}", context);
        println!("Path '{}': '{}'", path, display_output);
        
        // Ensure we don't have patterns like ":42:51" (double line numbers)
        assert!(!display_output.contains("::"), "Found double colon (possible duplicate line numbers)");
        
        // Count colons - should be exactly 1 for the line number
        let colon_count = display_output.chars().filter(|c| *c == ':').count();
        assert_eq!(colon_count, 1, "Should have exactly 1 colon for line number, found {}", colon_count);
        
        assert_eq!(display_output, expected,
            "\nUnexpected output for path '{}'", path);
    }
    
    println!("✓ No duplicate line numbers found!");
    println!("=========================================\n");
}

#[test]
fn verify_format_methods_consistency() {
    let symbol = Symbol::new(
        SymbolId::new(1).unwrap(),
        "test_function",
        SymbolKind::Function,
        FileId::new(1).unwrap(),
        Range::new(99, 0, 110, 0),  // Line 99 (0-indexed)
    );
    
    let context = SymbolContext {
        symbol,
        file_path: "src/test.rs:100".to_string(),  // Line 100 (1-indexed, already adjusted)
        relationships: SymbolRelationships::default(),
    };
    
    println!("\n=== VERIFYING FORMAT METHODS CONSISTENCY ===");
    
    // Test Display trait
    let display_output = format!("{}", context);
    println!("Display output: '{}'", display_output);
    
    // Test format_location_with_type() directly
    let format_with_type = context.format_location_with_type();
    println!("format_location_with_type(): '{}'", format_with_type);
    
    // They should be identical since Display delegates to format_location_with_type
    assert_eq!(display_output, format_with_type, 
        "Display and format_location_with_type should produce identical output!");
    
    // Test format_location() - should be similar but without the kind
    let format_location = context.format_location();
    println!("format_location(): '{}'", format_location);
    assert_eq!(format_location, "test_function at src/test.rs:100",
        "format_location should not include the kind");
    
    // Test format_full() - should include the header with correct format
    let format_full = context.format_full("");
    println!("format_full() first line: '{}'", format_full.lines().next().unwrap_or(""));
    
    let expected_header = "test_function (Function) at src/test.rs:100";
    assert!(format_full.starts_with(expected_header),
        "format_full should start with the correct header.\nExpected start: '{}'\nActual: '{}'",
        expected_header, format_full.lines().next().unwrap_or(""));
    
    println!("✓ All format methods are consistent!");
    println!("==========================================\n");
}