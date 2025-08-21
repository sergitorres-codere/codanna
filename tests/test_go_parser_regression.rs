//! Go Parser Regression Tests
//!
//! This test suite focuses on regression testing to ensure that changes to the Go parser
//! don't break existing functionality. It includes:
//! - Symbol count baseline verification
//! - Parser stability tests with known inputs
//! - Performance regression checks
//! - Edge case handling consistency

use anyhow::{Context, Result};
use codanna::parsing::LanguageParser;
use codanna::parsing::go::GoParser;
use codanna::types::{FileId, SymbolCounter};
use std::fs;

/// Regression test baselines - these should remain stable unless parser is intentionally changed
const EXPECTED_BASIC_SYMBOLS: usize = 48;
const EXPECTED_STRUCT_SYMBOLS: usize = 88;
const EXPECTED_INTERFACE_SYMBOLS: usize = 163;
const EXPECTED_GENERICS_SYMBOLS: usize = 146;
const EXPECTED_COMPLEX_SYMBOLS: usize = 197;

/// Performance baseline - parser should maintain >10k symbols/second
const PERFORMANCE_BASELINE_SYMBOLS_PER_SEC: usize = 10_000;

/// Regression test for basic Go symbol extraction stability
#[test]
fn test_basic_fixture_symbol_count_regression() -> Result<()> {
    let fixture_path = "tests/fixtures/go/basic.go";
    let symbol_count = count_symbols_in_fixture(fixture_path)?;

    assert_eq!(
        symbol_count, EXPECTED_BASIC_SYMBOLS,
        "REGRESSION: basic.go symbol count changed from {EXPECTED_BASIC_SYMBOLS} to {symbol_count}. This may indicate unintentional parser changes."
    );

    println!("✓ Basic fixture regression test passed ({symbol_count} symbols)");
    Ok(())
}

/// Regression test for struct parsing stability
#[test]
fn test_struct_fixture_symbol_count_regression() -> Result<()> {
    let fixture_path = "tests/fixtures/go/structs.go";
    let symbol_count = count_symbols_in_fixture(fixture_path)?;

    assert_eq!(
        symbol_count, EXPECTED_STRUCT_SYMBOLS,
        "REGRESSION: structs.go symbol count changed from {EXPECTED_STRUCT_SYMBOLS} to {symbol_count}. This may indicate unintentional parser changes."
    );

    println!("✓ Struct fixture regression test passed ({symbol_count} symbols)");
    Ok(())
}

/// Regression test for interface parsing stability
#[test]
fn test_interface_fixture_symbol_count_regression() -> Result<()> {
    let fixture_path = "tests/fixtures/go/interfaces.go";
    let symbol_count = count_symbols_in_fixture(fixture_path)?;

    assert_eq!(
        symbol_count, EXPECTED_INTERFACE_SYMBOLS,
        "REGRESSION: interfaces.go symbol count changed from {EXPECTED_INTERFACE_SYMBOLS} to {symbol_count}. This may indicate unintentional parser changes."
    );

    println!("✓ Interface fixture regression test passed ({symbol_count} symbols)");
    Ok(())
}

/// Regression test for generics parsing stability (Go 1.18+)
#[test]
fn test_generics_fixture_symbol_count_regression() -> Result<()> {
    let fixture_path = "tests/fixtures/go/generics.go";
    let symbol_count = count_symbols_in_fixture(fixture_path)?;

    assert_eq!(
        symbol_count, EXPECTED_GENERICS_SYMBOLS,
        "REGRESSION: generics.go symbol count changed from {EXPECTED_GENERICS_SYMBOLS} to {symbol_count}. This may indicate unintentional parser changes."
    );

    println!("✓ Generics fixture regression test passed ({symbol_count} symbols)");
    Ok(())
}

/// Regression test for complex Go code parsing stability
#[test]
fn test_complex_fixture_symbol_count_regression() -> Result<()> {
    let fixture_path = "tests/fixtures/go/complex.go";
    let symbol_count = count_symbols_in_fixture(fixture_path)?;

    assert_eq!(
        symbol_count, EXPECTED_COMPLEX_SYMBOLS,
        "REGRESSION: complex.go symbol count changed from {EXPECTED_COMPLEX_SYMBOLS} to {symbol_count}. This may indicate unintentional parser changes."
    );

    println!("✓ Complex fixture regression test passed ({symbol_count} symbols)");
    Ok(())
}

/// Regression test for parser performance
#[test]
fn test_parser_performance_regression() -> Result<()> {
    let fixture_path = "tests/fixtures/go/complex.go";
    let start = std::time::Instant::now();

    // Parse the same file multiple times to get measurable timing
    let iterations = 100;
    let mut total_symbols = 0;

    for _ in 0..iterations {
        total_symbols += count_symbols_in_fixture(fixture_path)?;
    }

    let elapsed = start.elapsed();
    let symbols_per_second = (total_symbols as f64 / elapsed.as_secs_f64()) as usize;

    assert!(
        symbols_per_second >= PERFORMANCE_BASELINE_SYMBOLS_PER_SEC,
        "REGRESSION: Parser performance degraded to {symbols_per_second} symbols/sec, below baseline of {PERFORMANCE_BASELINE_SYMBOLS_PER_SEC} symbols/sec"
    );

    println!("✓ Performance regression test passed ({symbols_per_second} symbols/sec)");
    Ok(())
}

/// Regression test for parser stability with edge cases
#[test]
fn test_edge_cases_stability_regression() -> Result<()> {
    let edge_cases = vec![
        ("empty file", "package main\n", 0), // Empty files are expected to have 0 symbols
        ("only package", "package main", 0), // Package declaration alone doesn't create symbols
        ("minimal function", "package main\nfunc main() {}", 1),
        (
            "struct with no fields",
            "package main\ntype Empty struct {}",
            1,
        ),
        (
            "interface with no methods",
            "package main\ntype Empty interface {}",
            1,
        ),
        ("single constant", "package main\nconst X = 1", 1),
        ("single variable", "package main\nvar x int", 1),
    ];

    for (name, code, expected_min) in edge_cases {
        let symbol_count = count_symbols_in_source(code)?;

        // Edge cases should parse successfully and return expected minimum symbols
        assert!(
            symbol_count >= expected_min,
            "REGRESSION: Edge case '{name}' extracted {symbol_count} symbols, expected at least {expected_min}"
        );

        println!("✓ Edge case '{name}' parsed successfully ({symbol_count} symbols)");
    }

    Ok(())
}

/// Regression test for specific Go language features that have caused issues
#[test]
fn test_language_features_regression() -> Result<()> {
    let test_cases = vec![
        (
            "qualified field names",
            r#"
                package main
                type Person struct {
                    Name string
                    Age int
                }
            "#,
        ),
        (
            "interface with embedded interface",
            r#"
                package main
                import "io"
                type ReadWriteCloser interface {
                    io.Reader
                    io.Writer
                    Close() error
                }
            "#,
        ),
        (
            "methods with receivers",
            r#"
                package main
                type Person struct { Name string }
                func (p *Person) GetName() string { return p.Name }
                func (p Person) SetName(name string) { p.Name = name }
            "#,
        ),
        (
            "generic functions and types",
            r#"
                package main
                func GenericFunc[T any](item T) T { return item }
                type GenericStruct[T comparable] struct { Value T }
            "#,
        ),
    ];

    for (feature_name, code) in test_cases {
        let symbol_count = count_symbols_in_source(code)
            .with_context(|| format!("Failed to parse {feature_name}"))?;

        assert!(
            symbol_count >= 2, // At least package + the main feature symbol
            "REGRESSION: {feature_name} parsing degraded (only {symbol_count} symbols found)"
        );

        println!("✓ {feature_name} regression test passed ({symbol_count} symbols)");
    }

    Ok(())
}

/// Test that all fixture files can be parsed without errors
#[test]
fn test_all_fixtures_parseable_regression() -> Result<()> {
    let fixture_files = vec![
        "tests/fixtures/go/basic.go",
        "tests/fixtures/go/structs.go",
        "tests/fixtures/go/interfaces.go",
        "tests/fixtures/go/generics.go",
        "tests/fixtures/go/complex.go",
        "tests/fixtures/go/scoping.go",
        "tests/fixtures/go/imports.go",
    ];

    for fixture_path in fixture_files {
        let symbol_count = count_symbols_in_fixture(fixture_path)
            .with_context(|| format!("Failed to parse fixture: {fixture_path}"))?;

        assert!(
            symbol_count > 0,
            "REGRESSION: Fixture {fixture_path} parsed but extracted no symbols"
        );

        println!("✓ Fixture {fixture_path} parsed successfully ({symbol_count} symbols)");
    }

    Ok(())
}

// Helper functions

/// Count symbols in a fixture file
fn count_symbols_in_fixture(fixture_path: &str) -> Result<usize> {
    let source = fs::read_to_string(fixture_path)
        .with_context(|| format!("Failed to read fixture file: {fixture_path}"))?;
    count_symbols_in_source(&source)
}

/// Count symbols in Go source code
fn count_symbols_in_source(source_code: &str) -> Result<usize> {
    let mut parser =
        GoParser::new().map_err(|e| anyhow::anyhow!("Failed to create Go parser: {}", e))?;

    let mut symbol_counter = SymbolCounter::new();
    let file_id = FileId::new(1).context("Failed to create file ID")?;

    let symbols = parser.parse(source_code, file_id, &mut symbol_counter);
    Ok(symbols.len())
}
