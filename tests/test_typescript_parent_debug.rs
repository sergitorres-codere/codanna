use codanna::FileId;
use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;

#[test]
fn debug_typescript_parent_context() {
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

    println!("\n=== ALL LOCAL SYMBOLS ===\n");

    for symbol in &symbols {
        if let Some(ScopeContext::Local {
            hoisted,
            parent_name,
            parent_kind,
        }) = &symbol.scope_context
        {
            println!(
                "Symbol: {:20} | Hoisted: {:5} | Parent: {:?} ({:?})",
                symbol.name.as_ref(),
                hoisted,
                parent_name
                    .as_ref()
                    .map(|s| s.as_ref())
                    .unwrap_or("MISSING"),
                parent_kind
            );
        }
    }

    // Check which symbol is missing parent context
    let missing_parent: Vec<_> = symbols
        .iter()
        .filter(|s| {
            matches!(
                s.scope_context,
                Some(ScopeContext::Local {
                    parent_name: None,
                    ..
                })
            )
        })
        .collect();

    if !missing_parent.is_empty() {
        println!("\n=== SYMBOLS MISSING PARENT CONTEXT ===");
        for symbol in missing_parent {
            println!("  - {} (kind: {:?})", symbol.name.as_ref(), symbol.kind);
        }
    }
}
