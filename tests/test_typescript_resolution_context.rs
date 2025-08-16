//! Integration tests for TypeScript resolution context
//!
//! This test verifies that the TypeScript resolution context properly handles:
//! - Hoisting of functions and var declarations
//! - Block scoping of let/const
//! - Import resolution
//! - Type space vs value space

use codanna::parsing::ResolutionScope;
use codanna::parsing::typescript::TypeScriptResolutionContext;
use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::symbol::ScopeContext;
use codanna::types::{FileId, SymbolCounter};

#[test]
fn test_typescript_resolution_hoisting() {
    println!("\n=== TypeScript Resolution Context Hoisting Test ===\n");

    let mut context = TypeScriptResolutionContext::new(FileId::new(1).unwrap());

    // Add hoisted function (should go to hoisted_scope)
    context.add_symbol_with_context(
        "myFunction".to_string(),
        codanna::SymbolId::new(1).unwrap(),
        Some(&ScopeContext::Local { hoisted: true }),
    );

    // Add block-scoped const (should go to local_scope)
    context.add_symbol_with_context(
        "myConst".to_string(),
        codanna::SymbolId::new(2).unwrap(),
        Some(&ScopeContext::Local { hoisted: false }),
    );

    // Add module-level export
    context.add_symbol_with_context(
        "MyClass".to_string(),
        codanna::SymbolId::new(3).unwrap(),
        Some(&ScopeContext::Module),
    );

    // Test resolution order
    assert_eq!(
        context.resolve("myFunction"),
        Some(codanna::SymbolId::new(1).unwrap()),
        "Should resolve hoisted function"
    );

    assert_eq!(
        context.resolve("myConst"),
        Some(codanna::SymbolId::new(2).unwrap()),
        "Should resolve block-scoped const"
    );

    assert_eq!(
        context.resolve("MyClass"),
        Some(codanna::SymbolId::new(3).unwrap()),
        "Should resolve module-level class"
    );

    // Clear local scope
    context.clear_local_scope();

    // Hoisted should still be available (in real TypeScript, depends on scope)
    // But local block-scoped should be gone
    assert_eq!(
        context.resolve("myConst"),
        None,
        "Block-scoped const should be cleared"
    );

    println!("✅ Hoisting resolution verified");
}

#[test]
fn test_typescript_import_resolution() {
    println!("\n=== TypeScript Import Resolution Test ===\n");

    let mut context = TypeScriptResolutionContext::new(FileId::new(1).unwrap());

    // Add regular import
    context.add_import_symbol(
        "React".to_string(),
        codanna::SymbolId::new(10).unwrap(),
        false, // not type-only
    );

    // Add type-only import (should go to type_space)
    context.add_import_symbol(
        "Props".to_string(),
        codanna::SymbolId::new(11).unwrap(),
        true, // type-only
    );

    // Test resolution
    assert_eq!(
        context.resolve("React"),
        Some(codanna::SymbolId::new(10).unwrap()),
        "Should resolve regular import"
    );

    assert_eq!(
        context.resolve("Props"),
        Some(codanna::SymbolId::new(11).unwrap()),
        "Should resolve type-only import from type space"
    );

    println!("✅ Import resolution verified");
}

#[test]
fn test_typescript_parser_scope_extraction() {
    println!("\n=== TypeScript Parser Scope Extraction Test ===\n");

    let code = r#"
// Module-level constant
const MODULE_CONST = 42;

// Hoisted function declaration
function hoistedFunction() {
    // Block-scoped variable
    const blockVar = 10;
    
    // Nested hoisted function
    function nestedHoisted() {
        return blockVar;
    }
    
    // Arrow function (not hoisted)
    const arrowFunc = () => {
        console.log("arrow");
    };
    
    return nestedHoisted();
}

// Module-level class
export class ExportedClass {
    private field: number = 0;
    
    public method(): void {
        // Method-local variable
        const methodLocal = 20;
    }
}

// Module-level arrow function (not hoisted)
const moduleArrow = (x: number) => x * 2;
"#;

    // Parse the code
    let mut parser = TypeScriptParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("Parsed {} symbols", symbols.len());

    // Check that symbols have proper scope context
    for symbol in &symbols {
        let name = symbol.name.as_ref();
        let scope = &symbol.scope_context;
        println!("Symbol: {name} - Scope: {scope:?}");

        match symbol.name.as_ref() {
            "hoistedFunction" => {
                assert_eq!(
                    symbol.scope_context,
                    Some(ScopeContext::Module),
                    "Module-level function should have Module scope"
                );
            }
            "nestedHoisted" => {
                assert_eq!(
                    symbol.scope_context,
                    Some(ScopeContext::Local { hoisted: true }),
                    "Nested function should be hoisted"
                );
            }
            "arrowFunc" => {
                // Arrow functions are not hoisted
                if let Some(ScopeContext::Local { hoisted }) = &symbol.scope_context {
                    assert!(!hoisted, "Arrow function should not be hoisted");
                }
            }
            "MODULE_CONST" => {
                assert_eq!(
                    symbol.scope_context,
                    Some(ScopeContext::Module),
                    "Module-level const should have Module scope"
                );
            }
            _ => {}
        }
    }

    println!("✅ Parser scope extraction verified");
}

#[test]
fn test_typescript_resolution_with_symbols() {
    println!("\n=== TypeScript Resolution with Parsed Symbols Test ===\n");

    // Parse some TypeScript code
    let code = r#"
function globalHoisted() {
    return "hoisted";
}

const blockScoped = 42;

class MyClass {
    method() {
        function localHoisted() {
            const localBlock = 10;
            return localBlock;
        }
        
        const methodLocal = localHoisted();
        return methodLocal;
    }
}
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Create resolution context and add symbols with their proper scope
    let mut context = TypeScriptResolutionContext::new(file_id);

    for symbol in &symbols {
        let name = symbol.name.as_ref();
        let scope = &symbol.scope_context;
        println!("Adding symbol: {name} with scope {scope:?}");
        context.add_symbol_with_context(
            symbol.name.to_string(),
            symbol.id,
            symbol.scope_context.as_ref(),
        );
    }

    // Test resolution
    let symbols_in_scope = context.symbols_in_scope();
    println!("\nSymbols in scope:");
    for (name, _id, scope_level) in &symbols_in_scope {
        println!("  - {name} ({scope_level:?})");
    }

    // Verify specific symbols
    assert!(
        context.resolve("globalHoisted").is_some(),
        "Should resolve globalHoisted function"
    );

    assert!(
        context.resolve("blockScoped").is_some(),
        "Should resolve blockScoped const"
    );

    assert!(
        context.resolve("MyClass").is_some(),
        "Should resolve MyClass"
    );

    println!("✅ Resolution with parsed symbols verified");
}
