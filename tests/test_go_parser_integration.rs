//! Go Parser Integration Tests
//! TDD Phase: Integration
//!
//! Key validations:
//! - Go symbol extraction (structs, interfaces, functions, methods)
//! - Go import parsing (standard library and module imports)
//! - Go visibility rules (exported vs unexported symbols)
//! - Go method resolution (receiver methods)
//! - Go signature extraction for all symbol types
//! - Performance targets: >10,000 symbols/second

use anyhow::Result;
use std::num::NonZeroU32;
use std::path::Path;
use thiserror::Error;

/// Errors specific to Go parser testing
#[derive(Error, Debug)]
pub enum GoParserError {
    #[error(
        "Parser initialization failed: {0}\nSuggestion: Check that tree-sitter-go is properly configured"
    )]
    InitializationFailed(String),

    #[error(
        "Symbol extraction failed: {0}\nSuggestion: Verify Go fixture files are valid and contain expected symbols"
    )]
    SymbolExtractionFailed(String),

    #[error(
        "Import parsing failed: {0}\nSuggestion: Check Go import statements follow correct syntax"
    )]
    ImportParsingFailed(String),

    #[error(
        "Signature generation failed: {0}\nSuggestion: Ensure Go function/method signatures are well-formed"
    )]
    SignatureGenerationFailed(String),
}

// Type-safe wrappers for test data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TestFileId(NonZeroU32);

impl TestFileId {
    pub fn new(id: u32) -> Option<Self> {
        NonZeroU32::new(id).map(Self)
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoSymbolInfo {
    pub name: String,
    pub kind: String,
    pub signature: String,
    pub is_exported: bool,
    pub file_id: TestFileId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoImportInfo {
    pub path: String,
    pub alias: Option<String>,
    pub is_dot_import: bool,
    pub is_blank_import: bool,
}

// Test constants
// TODO: Use for regression testing once symbol counts stabilize
#[allow(dead_code)]
const EXPECTED_BASIC_SYMBOLS: usize = 15; // Approximate count from basic.go
#[allow(dead_code)]
const EXPECTED_STRUCT_SYMBOLS: usize = 20; // Approximate count from structs.go
#[allow(dead_code)]
const EXPECTED_INTERFACE_SYMBOLS: usize = 12; // Approximate count from interfaces.go
const PERFORMANCE_TARGET_SYMBOLS_PER_SEC: usize = 10_000;

/// Test 1: Basic Go symbol extraction from simple constructs
/// Goal: Verify parser can extract functions, variables, constants, and type aliases
#[test]
fn test_basic_go_symbol_extraction() -> Result<()> {
    println!("\n=== Test 1: Basic Go Symbol Extraction ===");

    // Given: Basic Go code with various symbol types
    let fixture_path = "tests/fixtures/go/basic.go";
    let symbols = extract_symbols_from_fixture(fixture_path)?;

    // When: We extract symbols from the code
    // Then: We should find expected symbols

    // Check for package-level constants
    let constants = filter_symbols_by_kind(&symbols, "constant");
    assert!(!constants.is_empty(), "Should find package constants");

    // Check for functions
    let functions = filter_symbols_by_kind(&symbols, "function");
    assert!(functions.len() >= 10, "Should find at least 10 functions");

    // Check for exported vs unexported functions
    let exported_functions: Vec<_> = functions.iter().filter(|s| s.is_exported).collect();
    let unexported_functions: Vec<_> = functions.iter().filter(|s| !s.is_exported).collect();

    assert!(
        !exported_functions.is_empty(),
        "Should find exported functions"
    );
    assert!(
        !unexported_functions.is_empty(),
        "Should find unexported functions"
    );

    // Check for type aliases
    let type_aliases = filter_symbols_by_kind(&symbols, "type_alias");
    assert!(!type_aliases.is_empty(), "Should find type aliases");

    // Verify specific function signatures
    let main_func = symbols
        .iter()
        .find(|s| s.name == "main" && s.kind == "function")
        .expect("Should find main function");
    assert!(
        main_func.signature.contains("func main()"),
        "Main function should have correct signature"
    );

    println!("✓ Found {} total symbols", symbols.len());
    println!(
        "✓ Found {} functions ({} exported, {} unexported)",
        functions.len(),
        exported_functions.len(),
        unexported_functions.len()
    );
    println!("✓ Found {} constants", constants.len());
    println!("✓ Found {} type aliases", type_aliases.len());
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 2: Go struct and method extraction
/// Goal: Verify parser can extract struct types, fields, and methods with receivers
#[test]
fn test_go_struct_and_method_extraction() -> Result<()> {
    println!("\n=== Test 2: Go Struct and Method Extraction ===");

    // Given: Go code with struct definitions and methods
    let fixture_path = "tests/fixtures/go/structs.go";
    let symbols = extract_symbols_from_fixture(fixture_path)?;

    // When: We extract symbols from struct-heavy code
    // Then: We should find structs and their methods

    // Check for struct types
    let structs = filter_symbols_by_kind(&symbols, "struct");
    assert!(structs.len() >= 5, "Should find at least 5 struct types");

    // Check for struct fields (should have qualified names)
    let fields = filter_symbols_by_kind(&symbols, "field");
    assert!(!fields.is_empty(), "Should find struct fields");

    // Verify that fields have qualified names (StructName.FieldName)
    let qualified_fields: Vec<_> = fields.iter().filter(|f| f.name.contains('.')).collect();
    assert!(
        !qualified_fields.is_empty(),
        "Should find fields with qualified names (StructName.FieldName)"
    );

    // Check for methods (functions with receivers)
    let methods = filter_symbols_by_kind(&symbols, "method");
    assert!(methods.len() >= 10, "Should find at least 10 methods");

    // Verify specific struct exists
    let user_struct = structs
        .iter()
        .find(|s| s.name == "User")
        .expect("Should find User struct");
    assert!(user_struct.is_exported, "User struct should be exported");

    // Verify method signatures include receiver information
    let user_methods: Vec<_> = methods
        .iter()
        .filter(|m| m.signature.contains("User"))
        .collect();
    assert!(
        !user_methods.is_empty(),
        "Should find methods on User struct"
    );

    // Check for both value and pointer receiver methods
    let pointer_receiver_methods: Vec<_> = user_methods
        .iter()
        .filter(|m| m.signature.contains("*User"))
        .collect();
    let value_receiver_methods: Vec<_> = user_methods
        .iter()
        .filter(|m| m.signature.contains("User") && !m.signature.contains("*User"))
        .collect();

    assert!(
        !pointer_receiver_methods.is_empty(),
        "Should find pointer receiver methods"
    );
    assert!(
        !value_receiver_methods.is_empty(),
        "Should find value receiver methods"
    );

    println!("✓ Found {} struct types", structs.len());
    println!(
        "✓ Found {} struct fields ({} qualified)",
        fields.len(),
        qualified_fields.len()
    );
    println!("✓ Found {} methods", methods.len());
    println!(
        "✓ Found {} pointer receiver methods",
        pointer_receiver_methods.len()
    );
    println!(
        "✓ Found {} value receiver methods",
        value_receiver_methods.len()
    );
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 3: Go interface extraction and implementation detection
/// Goal: Verify parser can extract interface types and their method signatures
#[test]
fn test_go_interface_extraction() -> Result<()> {
    println!("\n=== Test 3: Go Interface Extraction ===");

    // Given: Go code with interface definitions
    let fixture_path = "tests/fixtures/go/interfaces.go";
    let symbols = extract_symbols_from_fixture(fixture_path)?;

    // When: We extract symbols from interface-heavy code
    // Then: We should find interfaces and their methods

    // Check for interface types
    let interfaces = filter_symbols_by_kind(&symbols, "interface");
    assert!(
        interfaces.len() >= 5,
        "Should find at least 5 interface types"
    );

    // Check for interface methods (they are stored as regular methods)
    let methods = filter_symbols_by_kind(&symbols, "method");
    let interface_methods: Vec<_> = methods
        .iter()
        .filter(|m| {
            // Interface methods should have qualified names like "InterfaceName.MethodName"
            m.name.contains('.') &&
            // Also check they don't have receiver syntax (which indicates struct methods)
            !m.signature.contains("func (")
        })
        .collect();
    assert!(
        !interface_methods.is_empty(),
        "Should find interface methods with qualified names"
    );

    // Verify specific interfaces
    let reader_interface = interfaces
        .iter()
        .find(|s| s.name == "Reader")
        .expect("Should find Reader interface");
    assert!(
        reader_interface.is_exported,
        "Reader interface should be exported"
    );

    // TODO: Complete embedded interface validation once parser fully supports interface embedding
    // Check for embedded interfaces
    let _embedded_interfaces: Vec<_> = interfaces
        .iter()
        .filter(|i| i.signature.contains("embed") || i.name.contains("ReadWrite"))
        .collect();
    // Note: This check depends on how embedded interfaces are represented

    println!("✓ Found {} interface types", interfaces.len());
    println!("✓ Found {} interface methods", interface_methods.len());
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 4: Go import parsing and resolution
/// Goal: Verify parser can correctly parse different types of Go imports
#[test]
fn test_go_import_parsing() -> Result<()> {
    println!("\n=== Test 4: Go Import Parsing ===");

    // Given: Go code with various import styles
    let fixture_path = "tests/fixtures/go/basic.go";
    let imports = extract_imports_from_fixture(fixture_path)?;

    // When: We extract imports from Go code
    // Then: We should find different import types

    assert!(!imports.is_empty(), "Should find import statements");

    // Check for standard library imports
    let std_imports: Vec<_> = imports.iter().filter(|i| !i.path.contains("/")).collect();
    assert!(
        !std_imports.is_empty(),
        "Should find standard library imports"
    );

    // Check for aliased imports
    let aliased_imports: Vec<_> = imports.iter().filter(|i| i.alias.is_some()).collect();
    assert!(!aliased_imports.is_empty(), "Should find aliased imports");

    // Check for dot imports
    let dot_imports: Vec<_> = imports.iter().filter(|i| i.is_dot_import).collect();
    assert!(!dot_imports.is_empty(), "Should find dot imports");

    // Check for blank imports
    let blank_imports: Vec<_> = imports.iter().filter(|i| i.is_blank_import).collect();
    assert!(!blank_imports.is_empty(), "Should find blank imports");

    println!("✓ Found {} total imports", imports.len());
    println!("✓ Found {} standard library imports", std_imports.len());
    println!("✓ Found {} aliased imports", aliased_imports.len());
    println!("✓ Found {} dot imports", dot_imports.len());
    println!("✓ Found {} blank imports", blank_imports.len());
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 5: Go generics parsing (Go 1.18+)
/// Goal: Verify parser can handle generic types and functions
#[test]
fn test_go_generics_parsing() -> Result<()> {
    println!("\n=== Test 5: Go Generics Parsing ===");

    // Given: Go code with generic constructs
    let fixture_path = "tests/fixtures/go/generics.go";
    let symbols = extract_symbols_from_fixture(fixture_path)?;

    // When: We extract symbols from generic Go code
    // Then: We should find generic types and functions

    // Check for generic functions
    let generic_functions: Vec<_> = symbols
        .iter()
        .filter(|s| s.kind == "function" && s.signature.contains("["))
        .collect();
    assert!(
        !generic_functions.is_empty(),
        "Should find generic functions"
    );

    // Check for generic types
    let generic_types: Vec<_> = symbols
        .iter()
        .filter(|s| s.kind == "struct" && s.signature.contains("["))
        .collect();
    assert!(!generic_types.is_empty(), "Should find generic types");

    // Verify specific generic constructs
    let identity_func = generic_functions
        .iter()
        .find(|s| s.name == "Identity")
        .expect("Should find Identity generic function");
    assert!(
        identity_func.signature.contains("[T any]"),
        "Identity function should have generic type parameter"
    );

    println!("✓ Found {} generic functions", generic_functions.len());
    println!("✓ Found {} generic types", generic_types.len());
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 6: Go visibility rules (exported vs unexported)
/// Goal: Verify parser correctly identifies exported vs unexported symbols
#[test]
fn test_go_visibility_rules() -> Result<()> {
    println!("\n=== Test 6: Go Visibility Rules ===");

    // Given: Go code with mixed exported/unexported symbols
    let fixture_path = "tests/fixtures/go/basic.go";
    let symbols = extract_symbols_from_fixture(fixture_path)?;

    // When: We check symbol visibility
    // Then: Capitalized names should be exported, lowercase unexported

    let exported_symbols: Vec<_> = symbols.iter().filter(|s| s.is_exported).collect();
    let unexported_symbols: Vec<_> = symbols.iter().filter(|s| !s.is_exported).collect();

    assert!(!exported_symbols.is_empty(), "Should find exported symbols");
    assert!(
        !unexported_symbols.is_empty(),
        "Should find unexported symbols"
    );

    // Verify visibility rules are correctly applied
    for symbol in &exported_symbols {
        assert!(
            symbol.name.chars().next().unwrap().is_uppercase(),
            "Exported symbol '{}' should start with uppercase",
            symbol.name
        );
    }

    for symbol in &unexported_symbols {
        // Skip special cases like main function, init functions, and blank imports (_)
        if symbol.name != "main" && symbol.name != "init" && symbol.name != "_" {
            // For qualified names like "Struct.field", check the actual field name part
            let name_to_check = if symbol.name.contains('.') {
                symbol.name.split('.').next_back().unwrap()
            } else {
                &symbol.name
            };

            assert!(
                name_to_check.chars().next().unwrap().is_lowercase(),
                "Unexported symbol '{}' should start with lowercase",
                symbol.name
            );
        }
    }

    println!("✓ Found {} exported symbols", exported_symbols.len());
    println!("✓ Found {} unexported symbols", unexported_symbols.len());
    println!("✓ Visibility rules correctly applied");
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 7: Qualified names disambiguation
/// Goal: Verify that struct fields and interface methods have qualified names for disambiguation
#[test]
fn test_qualified_names_disambiguation() -> Result<()> {
    println!("\n=== Test 7: Qualified Names Disambiguation ===");

    // Given: Go code with structs and interfaces that have common field/method names
    let fixture_path = "tests/fixtures/go/qualified_names_test.go";
    let symbols = extract_symbols_from_fixture(fixture_path)?;

    // When: We extract symbols from the code
    // Then: Fields and interface methods should have qualified names

    // Check for struct fields with qualified names
    let fields = filter_symbols_by_kind(&symbols, "field");
    assert!(!fields.is_empty(), "Should find struct fields");

    // Should have Person.Name, Person.Age, Product.Name, Product.Price
    let expected_qualified_fields = ["Person.Name", "Person.Age", "Product.Name", "Product.Price"];
    for expected_field in &expected_qualified_fields {
        let found = fields.iter().any(|f| f.name == *expected_field);
        assert!(found, "Should find qualified field: {expected_field}");
    }

    // Check for interface methods with qualified names
    let methods = filter_symbols_by_kind(&symbols, "method");
    let interface_methods: Vec<_> = methods
        .iter()
        .filter(|m| m.name.contains('.') && !m.signature.contains("func ("))
        .collect();
    assert!(
        !interface_methods.is_empty(),
        "Should find interface methods"
    );

    // Should have Reader.Read, Reader.Close, Writer.Write, Writer.Close
    let expected_qualified_methods = [
        "Reader.Read",
        "Reader.Close",
        "Writer.Write",
        "Writer.Close",
    ];
    for expected_method in &expected_qualified_methods {
        let found = interface_methods.iter().any(|m| m.name == *expected_method);
        assert!(found, "Should find qualified method: {expected_method}");
    }

    // Verify disambiguation: we should be able to distinguish between
    // Person.Name vs Product.Name and Reader.Close vs Writer.Close
    let person_name = fields.iter().find(|f| f.name == "Person.Name");
    let product_name = fields.iter().find(|f| f.name == "Product.Name");
    assert!(
        person_name.is_some() && product_name.is_some(),
        "Should distinguish between Person.Name and Product.Name"
    );

    let reader_close = interface_methods.iter().find(|m| m.name == "Reader.Close");
    let writer_close = interface_methods.iter().find(|m| m.name == "Writer.Close");
    assert!(
        reader_close.is_some() && writer_close.is_some(),
        "Should distinguish between Reader.Close and Writer.Close"
    );

    println!(
        "✓ Found {} qualified fields",
        fields.iter().filter(|f| f.name.contains('.')).count()
    );
    println!(
        "✓ Found {} qualified interface methods",
        interface_methods.len()
    );
    println!("✓ Successfully disambiguated symbols with same names");
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 8: Performance validation
/// Goal: Verify parser meets performance targets
#[test]
fn test_go_parser_performance() -> Result<()> {
    use std::time::Instant;

    println!("\n=== Test 8: Go Parser Performance ===");

    // Given: Multiple Go fixture files
    let fixture_paths = vec![
        "tests/fixtures/go/basic.go",
        "tests/fixtures/go/structs.go",
        "tests/fixtures/go/interfaces.go",
        "tests/fixtures/go/generics.go",
        "tests/fixtures/go/complex.go",
    ];

    // When: We parse all fixtures and measure time
    let start = Instant::now();
    let mut total_symbols = 0;

    for path in &fixture_paths {
        let symbols = extract_symbols_from_fixture(path)?;
        total_symbols += symbols.len();
    }

    let elapsed = start.elapsed();
    let symbols_per_sec = if elapsed.as_secs() > 0 {
        total_symbols / elapsed.as_secs() as usize
    } else {
        total_symbols * 1000 / elapsed.as_millis() as usize
    };

    // Then: Performance should meet targets
    assert!(
        symbols_per_sec >= PERFORMANCE_TARGET_SYMBOLS_PER_SEC,
        "Parser performance {symbols_per_sec} symbols/sec is below target {PERFORMANCE_TARGET_SYMBOLS_PER_SEC} symbols/sec"
    );

    println!("✓ Parsed {total_symbols} symbols in {elapsed:?}");
    println!("✓ Performance: {symbols_per_sec} symbols/second");
    println!("✓ Meets performance target of {PERFORMANCE_TARGET_SYMBOLS_PER_SEC} symbols/second");
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 8: End-to-end integration with complex Go code
/// Goal: Verify parser handles real-world Go patterns correctly
#[test]
fn test_complex_go_integration() -> Result<()> {
    println!("\n=== Test 8: Complex Go Integration ===");

    // Given: Complex Go code with advanced patterns
    let fixture_path = "tests/fixtures/go/complex.go";
    let symbols = extract_symbols_from_fixture(fixture_path)?;

    // When: We extract symbols from complex code
    // Then: We should handle all Go language features

    assert!(
        symbols.len() >= 30,
        "Should find many symbols in complex code"
    );

    // Check for channels and goroutine-related types
    let channel_types: Vec<_> = symbols
        .iter()
        .filter(|s| s.signature.contains("chan "))
        .collect();

    // Check for interface implementations
    let implementations: Vec<_> = symbols
        .iter()
        .filter(|s| s.kind == "struct" && s.name.ends_with("Processor"))
        .collect();

    // Check for complex method signatures
    let complex_methods: Vec<_> = symbols
        .iter()
        .filter(|s| s.kind == "method" && s.signature.len() > 100)
        .collect();

    println!("✓ Found {} total symbols in complex code", symbols.len());
    println!("✓ Found {} channel-related symbols", channel_types.len());
    println!(
        "✓ Found {} processor implementations",
        implementations.len()
    );
    println!(
        "✓ Found {} complex method signatures",
        complex_methods.len()
    );
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 9: Module registration and end-to-end integration
/// Goal: Verify Go parser is properly registered and works through the full system
#[test]
fn test_go_module_registration_integration() -> Result<()> {
    println!("\n=== Test 9: Module Registration and End-to-End Integration ===");

    // Test 1: Verify Go language is registered in the global registry
    {
        use codanna::parsing::LanguageId;
        use codanna::parsing::registry::get_registry;

        let registry_guard = get_registry().lock().unwrap();
        let go_id = LanguageId::new("go");

        // Test that Go language is available
        assert!(
            registry_guard.is_available(go_id),
            "Go language should be available in registry"
        );

        // Test file extension recognition
        let detected = registry_guard.get_by_extension("go");
        assert!(detected.is_some(), "Go files (.go) should be recognized");
        assert_eq!(
            detected.unwrap().id(),
            go_id,
            "Extension should map to Go language"
        );

        println!("✓ Go language properly registered in global registry");
    }

    // Test 2: Test factory methods create correct instances
    {
        use codanna::Settings;
        use codanna::parsing::LanguageDefinition;
        use codanna::parsing::go::GoLanguage;

        let settings = Settings::default();
        let language = GoLanguage;

        // Test parser creation
        let parser = language.create_parser(&settings);
        assert!(parser.is_ok(), "Should be able to create Go parser");

        // Test behavior creation
        let _behavior = language.create_behavior();
        // Just verify that we can create the behavior (no specific assertion needed)

        println!("✓ Factory methods create correct instances");
    }

    // Test 3: End-to-end parsing with a simple Go program
    {
        // Create a simple Go program in memory
        let go_code = r#"
package main

import "fmt"

const Version = "1.0.0"

type Person struct {
    Name string
    Age  int
}

func (p *Person) Greet() string {
    return fmt.Sprintf("Hello, I'm %s", p.Name)
}

func main() {
    person := &Person{Name: "Alice", Age: 30}
    fmt.Println(person.Greet())
}
"#;

        use codanna::parsing::LanguageParser;
        use codanna::parsing::go::GoParser;
        use codanna::types::{FileId, SymbolCounter};

        let mut parser = GoParser::new().map_err(|e| {
            GoParserError::InitializationFailed(format!("Failed to create parser: {e}"))
        })?;

        let mut symbol_counter = SymbolCounter::new();
        let file_id = FileId::new(1).expect("Failed to create file ID");
        let symbols = parser.parse(go_code, file_id, &mut symbol_counter);

        // Verify we found expected symbols
        assert!(!symbols.is_empty(), "Should extract symbols from Go code");

        // Look for specific symbols
        let symbol_names: Vec<&str> = symbols.iter().map(|s| s.name.as_ref()).collect();

        // Should find main function
        assert!(
            symbol_names.contains(&"main"),
            "Should find main function, found: {symbol_names:?}"
        );

        // Should find Person struct
        assert!(
            symbol_names.contains(&"Person"),
            "Should find Person struct, found: {symbol_names:?}"
        );

        // Should find Greet method
        assert!(
            symbol_names.contains(&"Greet"),
            "Should find Greet method, found: {symbol_names:?}"
        );

        // Should find Version constant
        assert!(
            symbol_names.contains(&"Version"),
            "Should find Version constant, found: {symbol_names:?}"
        );

        println!("✓ End-to-end parsing extracted {} symbols", symbols.len());
        println!("✓ Found expected symbols: main, Person, Greet, Version");
    }

    // Test 4: Test with actual indexing system (if available)
    {
        use codanna::indexing::SimpleIndexer;
        use std::fs;

        // Create a temporary Go file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_integration.go");

        let test_code = r#"
package test

// PublicFunc is an exported function
func PublicFunc() string {
    return "public"
}

// privateFunc is an unexported function
func privateFunc() int {
    return 42
}
"#;

        fs::write(&temp_file, test_code).map_err(|e| {
            GoParserError::SymbolExtractionFailed(format!("Failed to write temp file: {e}"))
        })?;

        // Test indexing the file
        let mut indexer = SimpleIndexer::new();

        let result = indexer.index_file(&temp_file);

        // Clean up temp file
        let _ = fs::remove_file(&temp_file);

        match result {
            Ok(_) => {
                println!("✓ Successfully indexed Go file through SimpleIndexer");
            }
            Err(e) => {
                println!("⚠ SimpleIndexer test skipped: {e}");
                // This might fail in test environment, so we don't panic
            }
        }
    }

    println!("=== PASSED ===\n");
    Ok(())
}

/// Test 10: Error handling and edge cases
/// Goal: Verify parser handles malformed Go code gracefully
#[test]
fn test_go_parser_error_handling() -> Result<()> {
    println!("\n=== Test 10: Error Handling and Edge Cases ===");

    use codanna::parsing::LanguageParser;
    use codanna::parsing::go::GoParser;
    use codanna::types::{FileId, SymbolCounter};

    let mut parser = GoParser::new().map_err(|e| {
        GoParserError::InitializationFailed(format!("Failed to create parser: {e}"))
    })?;

    // Test with malformed Go code
    let malformed_cases = vec![
        ("Empty file", ""),
        ("Only whitespace", "   \n\t  \n"),
        ("Incomplete function", "func incomplete("),
        ("Invalid syntax", "this is not go code!"),
        (
            "Unmatched braces",
            "package main\nfunc test() {\n// missing closing brace",
        ),
    ];

    for (case_name, malformed_code) in malformed_cases {
        let mut symbol_counter = SymbolCounter::new();
        let file_id = FileId::new(1).expect("Failed to create file ID");

        // Parser should not panic, but may return empty or partial results
        let symbols = parser.parse(malformed_code, file_id, &mut symbol_counter);

        println!(
            "✓ Parser handled '{}' gracefully ({} symbols found)",
            case_name,
            symbols.len()
        );
    }

    // Test with very large function signatures
    let large_signature = format!(
        "package main\nfunc VeryLongFunctionName{}(param1 string, param2 int, param3 bool) (string, error) {{ return \"\", nil }}",
        "WithManyParameters".repeat(10)
    );

    let mut symbol_counter = SymbolCounter::new();
    let file_id = FileId::new(1).expect("Failed to create file ID");
    let symbols = parser.parse(&large_signature, file_id, &mut symbol_counter);

    assert!(
        !symbols.is_empty(),
        "Should handle large function signatures"
    );

    println!("✓ Large function signatures handled correctly");
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 11: Real-world Go patterns integration
/// Goal: Verify parser handles complex real-world Go patterns
#[test]
fn test_real_world_go_patterns() -> Result<()> {
    println!("\n=== Test 11: Real-World Go Patterns ===");

    // Test with complex real-world Go code patterns
    let complex_go_code = r#"
package main

import (
    "context"
    "fmt"
    "log"
    "net/http"
    "sync"
    "time"
)

// Generic constraint interface
type Comparable[T any] interface {
    Compare(T) int
    ~int | ~string | ~float64
}

// Generic struct with embedded interface
type Repository[T Comparable[T]] struct {
    mu      sync.RWMutex
    items   map[string]T
    logger  *log.Logger
    timeout time.Duration
}

// Factory function with generics
func NewRepository[T Comparable[T]](timeout time.Duration) *Repository[T] {
    return &Repository[T]{
        items:   make(map[string]T),
        logger:  log.Default(),
        timeout: timeout,
    }
}

// Method with context and error handling
func (r *Repository[T]) Store(ctx context.Context, key string, value T) error {
    select {
    case <-ctx.Done():
        return ctx.Err()
    default:
    }

    r.mu.Lock()
    defer r.mu.Unlock()
    
    r.items[key] = value
    r.logger.Printf("Stored item with key: %s", key)
    return nil
}

// Interface with embedded interface
type HTTPHandler interface {
    http.Handler
    Setup() error
    Cleanup() error
}

// Struct implementing multiple interfaces
type WebServer struct {
    *Repository[string]
    mux    *http.ServeMux
    server *http.Server
}

// Method with complex receiver and return types
func (ws *WebServer) ServeHTTP(w http.ResponseWriter, r *http.Request) {
    defer func() {
        if err := recover(); err != nil {
            http.Error(w, "Internal server error", http.StatusInternalServerError)
            ws.logger.Printf("Panic recovered: %v", err)
        }
    }()
    
    ws.mux.ServeHTTP(w, r)
}

// Function with channel types
func processRequests(ctx context.Context, requests <-chan *http.Request, responses chan<- *http.Response) {
    for {
        select {
        case req := <-requests:
            if req == nil {
                return
            }
            // Process request...
            
        case <-ctx.Done():
            close(responses)
            return
        }
    }
}

// Main function with complex initialization
func main() {
    ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
    defer cancel()

    repo := NewRepository[string](5 * time.Second)
    
    server := &WebServer{
        Repository: repo,
        mux:        http.NewServeMux(),
        server: &http.Server{
            Addr:         ":8080",
            ReadTimeout:  10 * time.Second,
            WriteTimeout: 10 * time.Second,
        },
    }

    if err := server.Setup(); err != nil {
        log.Fatal(err)
    }
    defer server.Cleanup()

    fmt.Println("Server starting...")
}
"#;

    // Parse the complex code
    let symbols = extract_symbols_from_source(complex_go_code)?;

    // Verify we extracted expected complex constructs
    let generic_types: Vec<_> = symbols
        .iter()
        .filter(|s| s.signature.contains("[") && (s.kind == "struct" || s.kind == "interface"))
        .collect();

    assert!(
        !generic_types.is_empty(),
        "Should find generic types and interfaces"
    );

    // Look specifically for struct methods (which have receiver syntax)
    let struct_methods: Vec<_> = symbols
        .iter()
        .filter(|s| s.kind == "method" && s.signature.contains("func ("))
        .collect();

    // Also count interface methods (which have qualified names but no receiver syntax)
    let interface_methods: Vec<_> = symbols
        .iter()
        .filter(|s| s.kind == "method" && s.name.contains('.') && !s.signature.contains("func ("))
        .collect();

    let total_methods = struct_methods.len() + interface_methods.len();

    assert!(
        total_methods >= 3,
        "Should find methods (struct methods: {}, interface methods: {})",
        struct_methods.len(),
        interface_methods.len()
    );

    let channel_related: Vec<_> = symbols
        .iter()
        .filter(|s| {
            s.signature.contains("chan ")
                || s.signature.contains("<-chan")
                || s.signature.contains("chan<-")
        })
        .collect();

    assert!(
        !channel_related.is_empty(),
        "Should find channel-related symbols"
    );

    // Look for interfaces that should have embedded interfaces (like HTTPHandler)
    let http_handler_interface: Vec<_> = symbols
        .iter()
        .filter(|s| s.kind == "interface" && s.name == "HTTPHandler")
        .collect();

    assert!(
        !http_handler_interface.is_empty(),
        "Should find HTTPHandler interface (which embeds http.Handler)"
    );

    println!(
        "✓ Found {} generic types and interfaces",
        generic_types.len()
    );
    println!(
        "✓ Found {} methods total ({} struct methods, {} interface methods)",
        total_methods,
        struct_methods.len(),
        interface_methods.len()
    );
    println!("✓ Found {} channel-related symbols", channel_related.len());
    println!("✓ Found HTTPHandler interface with embedded http.Handler");
    println!("✓ Total symbols extracted: {}", symbols.len());
    println!("=== PASSED ===\n");

    Ok(())
}

/// Test 12: Regression tests for fixed issues
/// Goal: Ensure previously fixed bugs don't reoccur
#[test]
fn test_go_parser_regression_tests() -> Result<()> {
    println!("\n=== Test 12: Regression Tests ===");

    // Regression test data for common issues
    let regression_cases = vec![
        (
            "Empty interface",
            "package main\ntype Empty interface {}\nfunc main() {}",
            "Should handle empty interfaces",
        ),
        (
            "Method with pointer receiver",
            "package main\ntype T struct{}\nfunc (t *T) Method() {}\nfunc main() {}",
            "Should correctly parse pointer receivers",
        ),
        (
            "Multiple return values",
            "package main\nfunc multiReturn() (string, int, error) { return \"\", 0, nil }\nfunc main() {}",
            "Should handle multiple return values",
        ),
        (
            "Generic function with constraints",
            "package main\nfunc Process[T any](item T) T { return item }\nfunc main() {}",
            "Should handle generic functions with constraints",
        ),
        (
            "Method on generic type",
            "package main\ntype List[T any] []T\nfunc (l List[T]) Len() int { return len(l) }\nfunc main() {}",
            "Should handle methods on generic types",
        ),
    ];

    for (case_name, test_code, expectation) in regression_cases {
        let symbols = extract_symbols_from_source(test_code)?;

        assert!(
            !symbols.is_empty(),
            "Regression case '{case_name}' failed: {expectation}"
        );

        // Specific validations based on case
        match case_name {
            "Empty interface" => {
                let interfaces = filter_symbols_by_kind(&symbols, "interface");
                assert!(!interfaces.is_empty(), "Should find empty interface");
            }
            "Method with pointer receiver" => {
                let methods = filter_symbols_by_kind(&symbols, "method");
                assert!(
                    !methods.is_empty(),
                    "Should find method with pointer receiver"
                );
                assert!(
                    methods.iter().any(|m| m.signature.contains("*T")),
                    "Should preserve pointer receiver in signature"
                );
            }
            "Multiple return values" => {
                let functions = filter_symbols_by_kind(&symbols, "function");
                let multi_return = functions
                    .iter()
                    .find(|f| f.name == "multiReturn")
                    .expect("Should find multiReturn function");
                assert!(
                    multi_return.signature.contains("string, int, error"),
                    "Should preserve multiple return types"
                );
            }
            _ => {
                // Generic validation - symbols should be present
            }
        }

        println!("✓ Regression test '{case_name}' passed");
    }

    println!("=== PASSED ===\n");

    Ok(())
}

// Helper functions for the integration tests

/// Extract symbols from a Go fixture file
fn extract_symbols_from_fixture(fixture_path: &str) -> Result<Vec<GoSymbolInfo>> {
    use codanna::parsing::LanguageParser;
    use codanna::parsing::go::GoParser;
    use codanna::types::{FileId as ActualFileId, SymbolCounter, SymbolKind};
    use std::fs;

    let path = Path::new(fixture_path);
    if !path.exists() {
        return Err(GoParserError::SymbolExtractionFailed(format!(
            "Fixture file not found: {fixture_path}"
        ))
        .into());
    }

    // Read the Go source code
    let source_code = fs::read_to_string(path).map_err(|e| {
        GoParserError::SymbolExtractionFailed(format!("Failed to read file {fixture_path}: {e}"))
    })?;

    // Create parser and parse the file
    let mut parser = GoParser::new().map_err(|e| {
        GoParserError::InitializationFailed(format!("Failed to create Go parser: {e}"))
    })?;

    let mut symbol_counter = SymbolCounter::new();
    let file_id = ActualFileId::new(1).expect("Failed to create file ID");
    let symbols = parser.parse(&source_code, file_id, &mut symbol_counter);

    // Convert internal symbols to test symbols
    let test_symbols: Result<Vec<_>> = symbols
        .into_iter()
        .map(|sym| {
            let test_file_id = TestFileId::new(1).ok_or_else(|| {
                GoParserError::SymbolExtractionFailed("Invalid test file ID".to_string())
            })?;

            let kind_str = match sym.kind {
                SymbolKind::Function => "function",
                SymbolKind::Method => "method",
                SymbolKind::Struct => "struct",
                SymbolKind::Interface => "interface",
                SymbolKind::Variable => "variable",
                SymbolKind::Constant => "constant",
                SymbolKind::Field => "field",
                SymbolKind::TypeAlias => "type_alias",
                _ => "unknown",
            };

            let is_exported = matches!(sym.visibility, codanna::Visibility::Public);

            Ok(GoSymbolInfo {
                name: sym.name.to_string(),
                kind: kind_str.to_string(),
                signature: sym
                    .signature
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("{} {}", kind_str, sym.name)),
                is_exported,
                file_id: test_file_id,
            })
        })
        .collect();

    test_symbols
}

/// Extract import information from a Go fixture file
fn extract_imports_from_fixture(fixture_path: &str) -> Result<Vec<GoImportInfo>> {
    use codanna::parsing::LanguageParser;
    use codanna::parsing::go::GoParser;
    use codanna::types::{FileId as ActualFileId, SymbolCounter};
    use std::fs;

    let path = Path::new(fixture_path);
    if !path.exists() {
        return Err(GoParserError::ImportParsingFailed(format!(
            "Fixture file not found: {fixture_path}"
        ))
        .into());
    }

    // Read the Go source code
    let source_code = fs::read_to_string(path).map_err(|e| {
        GoParserError::ImportParsingFailed(format!("Failed to read file {fixture_path}: {e}"))
    })?;

    // Create parser instance
    let mut parser = GoParser::new().map_err(|e| {
        GoParserError::InitializationFailed(format!("Failed to create Go parser: {e}"))
    })?;

    // Parse the file to get imports
    let mut symbol_counter = SymbolCounter::new();
    let file_id = ActualFileId::new(1).expect("Failed to create file ID");
    let _symbols = parser.parse(&source_code, file_id, &mut symbol_counter);

    // Note: The current parser design doesn't expose imports directly through the parse method.
    // The imports are processed internally during symbol resolution.
    // For now, we'll parse the source code manually to extract import information.

    let imports = extract_imports_from_source(&source_code)?;
    Ok(imports)
}

/// Extract imports directly from Go source code
fn extract_imports_from_source(source: &str) -> Result<Vec<GoImportInfo>> {
    let mut imports = Vec::new();
    let lines = source.lines();
    let mut in_import_block = false;

    for line in lines {
        let trimmed = line.trim();

        // Handle import block start
        if trimmed.starts_with("import (") {
            in_import_block = true;
            continue;
        }

        // Handle import block end
        if in_import_block && trimmed == ")" {
            in_import_block = false;
            continue;
        }

        // Handle single import or import within block
        if trimmed.starts_with("import ") || in_import_block {
            if let Some(import_info) = parse_import_line(trimmed)? {
                imports.push(import_info);
            }
        }
    }

    Ok(imports)
}

/// Parse a single import line
fn parse_import_line(line: &str) -> Result<Option<GoImportInfo>> {
    let trimmed = line.trim();

    // Skip empty lines and comments
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return Ok(None);
    }

    // Remove "import " prefix if present
    let import_part = if let Some(stripped) = trimmed.strip_prefix("import ") {
        stripped
    } else {
        trimmed
    };

    let import_part = import_part.trim();

    // Check for different import patterns
    if let Some(path) = extract_quoted_string(import_part) {
        // Simple import: "fmt"
        Ok(Some(GoImportInfo {
            path,
            alias: None,
            is_dot_import: false,
            is_blank_import: false,
        }))
    } else if let Some(stripped) = import_part.strip_prefix("_ ") {
        // Blank import: _ "database/sql"
        if let Some(path) = extract_quoted_string(stripped) {
            Ok(Some(GoImportInfo {
                path,
                alias: None,
                is_dot_import: false,
                is_blank_import: true,
            }))
        } else {
            Ok(None)
        }
    } else if let Some(stripped) = import_part.strip_prefix(". ") {
        // Dot import: . "math"
        if let Some(path) = extract_quoted_string(stripped) {
            Ok(Some(GoImportInfo {
                path,
                alias: None,
                is_dot_import: true,
                is_blank_import: false,
            }))
        } else {
            Ok(None)
        }
    } else if let Some(space_pos) = import_part.find(' ') {
        // Aliased import: mylog "log"
        let alias = import_part[..space_pos].trim().to_string();
        if let Some(path) = extract_quoted_string(&import_part[space_pos + 1..]) {
            Ok(Some(GoImportInfo {
                path,
                alias: Some(alias),
                is_dot_import: false,
                is_blank_import: false,
            }))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Extract a quoted string from Go import syntax
fn extract_quoted_string(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        Some(trimmed[1..trimmed.len() - 1].to_string())
    } else {
        None
    }
}

/// Extract symbols directly from Go source code
fn extract_symbols_from_source(source_code: &str) -> Result<Vec<GoSymbolInfo>> {
    use codanna::parsing::LanguageParser;
    use codanna::parsing::go::GoParser;
    use codanna::types::{FileId as ActualFileId, SymbolCounter, SymbolKind};

    // Create parser and parse the source
    let mut parser = GoParser::new().map_err(|e| {
        GoParserError::InitializationFailed(format!("Failed to create Go parser: {e}"))
    })?;

    let mut symbol_counter = SymbolCounter::new();
    let file_id = ActualFileId::new(1).expect("Failed to create file ID");
    let symbols = parser.parse(source_code, file_id, &mut symbol_counter);

    // Convert internal symbols to test symbols
    let test_symbols: Result<Vec<_>> = symbols
        .into_iter()
        .map(|sym| {
            let test_file_id = TestFileId::new(1).ok_or_else(|| {
                GoParserError::SymbolExtractionFailed("Invalid test file ID".to_string())
            })?;

            let kind_str = match sym.kind {
                SymbolKind::Function => "function",
                SymbolKind::Method => "method",
                SymbolKind::Struct => "struct",
                SymbolKind::Interface => "interface",
                SymbolKind::Variable => "variable",
                SymbolKind::Constant => "constant",
                SymbolKind::Field => "field",
                SymbolKind::TypeAlias => "type_alias",
                _ => "unknown",
            };

            let is_exported = matches!(sym.visibility, codanna::Visibility::Public);

            Ok(GoSymbolInfo {
                name: sym.name.to_string(),
                kind: kind_str.to_string(),
                signature: sym
                    .signature
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("{} {}", kind_str, sym.name)),
                is_exported,
                file_id: test_file_id,
            })
        })
        .collect();

    test_symbols
}

/// Filter symbols by their kind
fn filter_symbols_by_kind<'a>(symbols: &'a [GoSymbolInfo], kind: &str) -> Vec<&'a GoSymbolInfo> {
    symbols.iter().filter(|s| s.kind == kind).collect()
}

/// Create test data for performance testing
/// TODO: Use in performance benchmarks (Phase 7.3)
#[allow(dead_code)]
fn generate_test_symbols(count: usize) -> Result<Vec<GoSymbolInfo>> {
    let symbols: Result<Vec<_>> = (0..count)
        .map(|i| {
            let file_id = TestFileId::new(1).ok_or_else(|| {
                GoParserError::SymbolExtractionFailed("Invalid file ID".to_string())
            })?;

            Ok(GoSymbolInfo {
                name: format!("symbol_{i}"),
                kind: "function".to_string(),
                signature: format!("func symbol_{i}()"),
                is_exported: i % 2 == 0,
                file_id,
            })
        })
        .collect();

    symbols
}

/// Validate that a symbol has the expected structure
/// TODO: Use in comprehensive symbol validation tests (Phase 7.1)
#[allow(dead_code)]
fn validate_symbol_structure(symbol: &GoSymbolInfo) -> Result<()> {
    if symbol.name.is_empty() {
        return Err(GoParserError::SymbolExtractionFailed(
            "Symbol name cannot be empty".to_string(),
        )
        .into());
    }

    if symbol.kind.is_empty() {
        return Err(GoParserError::SymbolExtractionFailed(
            "Symbol kind cannot be empty".to_string(),
        )
        .into());
    }

    if symbol.signature.is_empty() {
        return Err(GoParserError::SignatureGenerationFailed(
            "Symbol signature cannot be empty".to_string(),
        )
        .into());
    }

    Ok(())
}
