use codanna::FileId;
use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;

#[test]
fn test_arrow_function_complex() {
    let mut parser = TypeScriptParser::new().unwrap();

    // Exact code from the complex test for processData
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
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== COMPLEX ARROW FUNCTION TEST ===\n");

    // Print all symbols
    for symbol in &symbols {
        println!(
            "Symbol: {:20} | Kind: {:?} | Scope: {:?}",
            symbol.name.as_ref(),
            symbol.kind,
            symbol.scope_context
        );
    }

    // Check specifically the transform symbol
    let transform = symbols.iter().find(|s| s.name.as_ref() == "transform");
    if let Some(t) = transform {
        match &t.scope_context {
            Some(ScopeContext::Local {
                hoisted,
                parent_name,
                parent_kind,
            }) => {
                println!("\nTransform const (arrow function):");
                println!("  Hoisted: {hoisted}");
                println!(
                    "  Parent Name: {:?}",
                    parent_name.as_ref().map(|s| s.as_ref())
                );
                println!("  Parent Kind: {parent_kind:?}");

                assert_eq!(
                    parent_name.as_ref().map(|s| s.as_ref()),
                    Some("processData"),
                    "Transform should have processData as parent"
                );
            }
            _ => panic!("Transform should have Local scope"),
        }
    } else {
        panic!("Transform symbol not found!");
    }
}
