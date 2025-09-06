//! Parser audit integration tests
//!
//! Tests that use the actual parsers to generate coverage reports
//! showing which nodes are implemented vs available in the grammar.

use codanna::parsing::go::audit::GoParserAudit;
use codanna::parsing::php::audit::PhpParserAudit;
use codanna::parsing::python::audit::PythonParserAudit;
use codanna::parsing::rust::audit::RustParserAudit;
use codanna::parsing::typescript::audit::TypeScriptParserAudit;
use std::fs;

#[test]
fn audit_go_parser() {
    println!("=== Go Parser Audit ===\n");

    // Audit the comprehensive example
    let audit = match GoParserAudit::audit_file("examples/go/comprehensive.go") {
        Ok(audit) => audit,
        Err(e) => {
            eprintln!("Failed to audit Go parser: {e}");
            panic!("Audit failed");
        }
    };

    // Generate report
    let report = audit.generate_report();
    println!("{report}");

    // Save to documentation
    fs::write("contributing/parsers/go/AUDIT_REPORT.md", &report)
        .expect("Failed to write audit report");

    // Basic assertions
    assert!(
        !audit.grammar_nodes.is_empty(),
        "Should discover grammar nodes"
    );
    assert!(
        !audit.extracted_symbol_kinds.is_empty(),
        "Should extract symbols"
    );

    // Check key nodes are in grammar
    assert!(audit.grammar_nodes.contains_key("function_declaration"));
    assert!(audit.grammar_nodes.contains_key("struct_type"));
    assert!(audit.grammar_nodes.contains_key("interface_type"));
}

#[test]
fn audit_php_parser() {
    println!("=== PHP Parser Audit ===\n");

    // Audit the comprehensive example
    let audit = match PhpParserAudit::audit_file("examples/php/comprehensive.php") {
        Ok(audit) => audit,
        Err(e) => {
            eprintln!("Failed to audit PHP parser: {e}");
            panic!("Audit failed");
        }
    };

    // Generate report
    let report = audit.generate_report();
    println!("{report}");

    // Save to documentation
    fs::write("contributing/parsers/php/AUDIT_REPORT.md", &report)
        .expect("Failed to write PHP audit report");

    // Basic assertions
    assert!(
        !audit.grammar_nodes.is_empty(),
        "Should discover grammar nodes"
    );
    assert!(
        !audit.extracted_symbol_kinds.is_empty(),
        "Should extract symbols"
    );

    // Check key nodes are in grammar
    assert!(audit.grammar_nodes.contains_key("class_declaration"));
    assert!(audit.grammar_nodes.contains_key("function_definition"));
    assert!(audit.grammar_nodes.contains_key("method_declaration"));
}

#[test]
fn audit_rust_parser() {
    println!("=== Rust Parser Audit ===\n");

    // Audit the comprehensive example
    let audit = match RustParserAudit::audit_file("examples/rust/comprehensive.rs") {
        Ok(audit) => audit,
        Err(e) => {
            eprintln!("Failed to audit Rust parser: {e}");
            panic!("Audit failed");
        }
    };

    // Generate report
    let report = audit.generate_report();
    println!("{report}");

    // Save to documentation
    fs::write("contributing/parsers/rust/AUDIT_REPORT.md", &report)
        .expect("Failed to write Rust audit report");

    // Basic assertions
    assert!(
        !audit.grammar_nodes.is_empty(),
        "Should discover grammar nodes"
    );
    assert!(
        !audit.extracted_symbol_kinds.is_empty(),
        "Should extract symbols"
    );

    // Check key nodes are in grammar
    assert!(audit.grammar_nodes.contains_key("struct_item"));
    assert!(audit.grammar_nodes.contains_key("impl_item"));
    assert!(audit.grammar_nodes.contains_key("function_item"));
}

#[test]
fn audit_typescript_parser() {
    println!("=== TypeScript Parser Audit ===\n");

    // Audit the comprehensive example
    let audit = match TypeScriptParserAudit::audit_file("examples/typescript/comprehensive.ts") {
        Ok(audit) => audit,
        Err(e) => {
            eprintln!("Failed to audit TypeScript parser: {e}");
            panic!("Audit failed");
        }
    };

    // Generate report
    let report = audit.generate_report();
    println!("{report}");

    // Save to documentation
    fs::write("contributing/parsers/typescript/AUDIT_REPORT.md", &report)
        .expect("Failed to write TypeScript audit report");

    // Basic assertions
    assert!(
        !audit.grammar_nodes.is_empty(),
        "Should discover grammar nodes"
    );
    assert!(
        !audit.extracted_symbol_kinds.is_empty(),
        "Should extract symbols"
    );

    // Check key nodes are in grammar
    assert!(audit.grammar_nodes.contains_key("class_declaration"));
    assert!(audit.grammar_nodes.contains_key("interface_declaration"));
    assert!(audit.grammar_nodes.contains_key("function_declaration"));
}

#[test]
fn audit_python_parser() {
    println!("=== Python Parser Audit ===\n");

    // Audit the comprehensive example
    let audit = match PythonParserAudit::audit_file("examples/python/comprehensive.py") {
        Ok(audit) => audit,
        Err(e) => {
            eprintln!("Failed to audit Python parser: {e}");
            panic!("Audit failed");
        }
    };

    // Generate report
    let report = audit.generate_report();
    println!("{report}");

    // Save to documentation
    fs::write("contributing/parsers/python/AUDIT_REPORT.md", &report)
        .expect("Failed to write Python audit report");

    // Basic assertions
    assert!(
        !audit.grammar_nodes.is_empty(),
        "Should discover grammar nodes"
    );
    assert!(
        !audit.extracted_symbol_kinds.is_empty(),
        "Should extract symbols"
    );

    // Check key nodes are in grammar
    assert!(audit.grammar_nodes.contains_key("class_definition"));
    assert!(audit.grammar_nodes.contains_key("function_definition"));
    assert!(audit.grammar_nodes.contains_key("assignment"));
}
