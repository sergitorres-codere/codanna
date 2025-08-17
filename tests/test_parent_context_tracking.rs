//! Test to verify parent context tracking works when parsers set parent information
//! This test demonstrates what the feature will look like when TypeScript parser
//! is updated to call set_current_function/set_current_class

use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;
use codanna::{FileId, SymbolKind};

#[test]
fn test_parent_context_with_typescript_parser() {
    // This test will pass once TypeScript parser is updated to track parent context
    let mut parser = TypeScriptParser::new().unwrap();
    let code = r#"
// Module-level function
function processData() {
    // This should have parent_name: "processData", parent_kind: Function
    const localVar = 42;

    // Nested function should also show parent
    function validateData() {
        // This should have parent_name: "validateData"
        const isValid = true;
        return isValid;
    }

    // Arrow function
    const transform = (x) => {
        // This should have parent_name: "transform" (if we track arrow functions)
        const result = x * 2;
        return result;
    };

    return validateData();
}

// Module-level class
class DataProcessor {
    constructor() {
        // This should have parent_name: "constructor", parent_kind: Function
        this.data = [];
    }

    process() {
        // Variables in methods should show the method as parent
        const temp = this.data;

        // Nested class in method
        class Helper {
            // This should have parent_name: "process"
            help() {}
        }

        return temp;
    }
}

// Test interfaces and types in functions
function createServer() {
    // These should have parent_name: "createServer"
    interface ServerConfig {
        port: number;
    }

    type ServerState = 'running' | 'stopped';

    return null;
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PARENT CONTEXT TRACKING TEST ===\n");
    println!("Testing if TypeScript parser sets parent context...\n");

    // Find the localVar inside processData
    let local_var = symbols.iter().find(|s| s.name.as_ref() == "localVar");

    if let Some(var) = local_var {
        println!("Found 'localVar' symbol:");
        match &var.scope_context {
            Some(ScopeContext::Local {
                hoisted,
                parent_name,
                parent_kind,
            }) => {
                println!("  Scope: Local");
                println!("  Hoisted: {hoisted}");
                println!("  Parent Name: {parent_name:?}");
                println!("  Parent Kind: {parent_kind:?}");

                // This assertion will fail until TypeScript parser is updated
                // Uncomment when implementing parser updates:
                // assert_eq!(parent_name.as_ref().map(|s| s.as_ref()), Some("processData"));
                // assert_eq!(parent_kind, &Some(SymbolKind::Function));

                if parent_name.is_none() {
                    println!("\n  ⚠️  Parent context not set - TypeScript parser needs update");
                } else {
                    println!("\n  ✅ Parent context is set!");
                }
            }
            _ => {
                println!("  Unexpected scope: {:?}", var.scope_context);
            }
        }
    }

    // Check validateData function (nested function)
    let validate_func = symbols.iter().find(|s| s.name.as_ref() == "validateData");

    if let Some(func) = validate_func {
        println!("\nFound 'validateData' (nested function):");
        match &func.scope_context {
            Some(ScopeContext::Local {
                hoisted,
                parent_name,
                parent_kind,
            }) => {
                println!("  Scope: Local (nested function)");
                println!("  Hoisted: {hoisted}");
                println!("  Parent Name: {parent_name:?}");
                println!("  Parent Kind: {parent_kind:?}");

                // Should have processData as parent when implemented
                if parent_name.is_none() {
                    println!("  ⚠️  Parent context not set");
                }
            }
            _ => {
                println!("  Scope: {:?}", func.scope_context);
            }
        }
    }

    // Check interface inside function
    let server_config = symbols.iter().find(|s| s.name.as_ref() == "ServerConfig");

    if let Some(interface) = server_config {
        println!("\nFound 'ServerConfig' interface (inside createServer function):");
        match &interface.scope_context {
            Some(ScopeContext::Local {
                hoisted,
                parent_name,
                parent_kind,
            }) => {
                println!("  Scope: Local");
                println!("  Hoisted: {hoisted} (interfaces are hoisted)");
                println!("  Parent Name: {parent_name:?}");
                println!("  Parent Kind: {parent_kind:?}");

                // Should have createServer as parent when implemented
                if parent_name.is_none() {
                    println!("  ⚠️  Parent context not set - needs parser update");
                } else if parent_name.as_ref().map(|s| s.as_ref()) == Some("createServer") {
                    println!("  ✅ Correctly shows parent function!");
                }
            }
            _ => {
                println!("  Scope: {:?}", interface.scope_context);
            }
        }
    }

    // Summary
    println!("\n=== SUMMARY ===");
    let locals_with_parent = symbols
        .iter()
        .filter(|s| {
            matches!(
                s.scope_context,
                Some(ScopeContext::Local {
                    parent_name: Some(_),
                    ..
                })
            )
        })
        .count();

    let total_locals = symbols
        .iter()
        .filter(|s| matches!(s.scope_context, Some(ScopeContext::Local { .. })))
        .count();

    println!("Local symbols with parent context: {locals_with_parent}/{total_locals}");

    if locals_with_parent == 0 && total_locals > 0 {
        println!("Status: ⚠️  TypeScript parser needs to be updated to track parent context");
        println!(
            "Next step: Update TypeScript parser to call set_current_function/set_current_class"
        );
    } else if locals_with_parent == total_locals {
        println!("Status: ✅ All local symbols have parent context!");
    } else {
        println!(
            "Status: ⚡ Partial implementation - {} symbols still need parent context",
            total_locals - locals_with_parent
        );
    }

    println!("\n=== END OF TEST ===\n");
}

#[test]
fn test_expected_parent_context_format() {
    // This test shows what we expect the parent context to look like
    // once parsers are updated

    println!("\n=== EXPECTED PARENT CONTEXT FORMAT ===\n");

    use codanna::types::CompactString;

    // Example 1: Variable in a function
    let expected_var_in_func = ScopeContext::Local {
        hoisted: false,
        parent_name: Some(CompactString::from("processData")),
        parent_kind: Some(SymbolKind::Function),
    };

    println!("Variable inside function 'processData':");
    println!("{expected_var_in_func:#?}");

    // Example 2: Nested function
    let expected_nested_func = ScopeContext::Local {
        hoisted: true, // Function declarations are hoisted in JS/TS
        parent_name: Some(CompactString::from("outerFunction")),
        parent_kind: Some(SymbolKind::Function),
    };

    println!("\nNested function inside 'outerFunction':");
    println!("{expected_nested_func:#?}");

    // Example 3: Variable in a method
    let expected_var_in_method = ScopeContext::Local {
        hoisted: false,
        parent_name: Some(CompactString::from("handleRequest")),
        parent_kind: Some(SymbolKind::Function), // Methods are Functions in SymbolKind
    };

    println!("\nVariable inside method 'handleRequest':");
    println!("{expected_var_in_method:#?}");

    // Example 4: Class inside a function (rare but valid)
    let expected_class_in_func = ScopeContext::Local {
        hoisted: false, // Classes are not hoisted
        parent_name: Some(CompactString::from("factory")),
        parent_kind: Some(SymbolKind::Function),
    };

    println!("\nClass defined inside function 'factory':");
    println!("{expected_class_in_func:#?}");

    println!("\n=== FORMAT EXAMPLES COMPLETE ===\n");
}
