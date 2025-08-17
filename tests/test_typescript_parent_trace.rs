use codanna::FileId;
use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;

#[test]
fn trace_typescript_parent_context() {
    let mut parser = TypeScriptParser::new().unwrap();

    // Simpler test case focusing on the arrow function issue
    let code = r#"
function processData() {
    const transform = (x) => {
        const result = x * 2;
        return result;
    };
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== ARROW FUNCTION PARENT CONTEXT TEST ===\n");

    for symbol in &symbols {
        println!(
            "Symbol: {:20} | Kind: {:?} | Scope: {:?}",
            symbol.name.as_ref(),
            symbol.kind,
            symbol.scope_context
        );
    }

    // Check the transform arrow function
    let transform = symbols.iter().find(|s| s.name.as_ref() == "transform");
    if let Some(t) = transform {
        match &t.scope_context {
            Some(ScopeContext::Local {
                hoisted,
                parent_name,
                parent_kind,
            }) => {
                println!("\nTransform arrow function:");
                println!("  Hoisted: {hoisted}");
                println!("  Parent Name: {parent_name:?}");
                println!("  Parent Kind: {parent_kind:?}");

                if parent_name.is_none() {
                    println!("  ❌ MISSING PARENT CONTEXT");
                } else {
                    println!("  ✅ Has parent context");
                }
            }
            _ => println!("  Unexpected scope: {:?}", t.scope_context),
        }
    }
}
