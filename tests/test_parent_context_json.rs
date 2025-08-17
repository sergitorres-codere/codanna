//! Test showing how parent context appears in JSON output

use codanna::Symbol;
use codanna::symbol::ScopeContext;
use codanna::types::{FileId, Range, SymbolCounter, SymbolKind};

#[test]
fn test_parent_context_json_serialization() {
    println!("\n=== PARENT CONTEXT JSON OUTPUT ===\n");

    let mut counter = SymbolCounter::new();
    let file_id = FileId::new(1).unwrap();

    // Create a symbol with parent context
    let symbol = Symbol::new(
        counter.next_id(),
        "localVariable",
        SymbolKind::Variable,
        file_id,
        Range::new(10, 4, 10, 20),
    )
    .with_scope(ScopeContext::Local {
        hoisted: false,
        parent_name: Some("processData".to_string().into()),
        parent_kind: Some(SymbolKind::Function),
    });

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&symbol).unwrap();

    println!("Symbol serialized to JSON:");
    println!("{json}");

    // Parse it back to verify
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    println!("\nScope context field:");
    if let Some(scope) = parsed.get("scope_context") {
        println!("{}", serde_json::to_string_pretty(scope).unwrap());
    }

    // Verify the structure
    assert!(parsed.get("scope_context").is_some());
    let scope = &parsed["scope_context"]["Local"];
    assert_eq!(scope["hoisted"], false);
    assert_eq!(scope["parent_name"], "processData");
    assert_eq!(scope["parent_kind"], "Function");

    println!("\n=== JSON SERIALIZATION TEST COMPLETE ===");
}

#[test]
fn test_different_scope_types_json() {
    println!("\n=== DIFFERENT SCOPE TYPES IN JSON ===\n");

    let examples = vec![
        ("Module scope", ScopeContext::Module),
        ("Class member", ScopeContext::ClassMember),
        ("Parameter", ScopeContext::Parameter),
        (
            "Local with parent",
            ScopeContext::Local {
                hoisted: true,
                parent_name: Some("handleRequest".to_string().into()),
                parent_kind: Some(SymbolKind::Function),
            },
        ),
        (
            "Local without parent (old format)",
            ScopeContext::Local {
                hoisted: false,
                parent_name: None,
                parent_kind: None,
            },
        ),
    ];

    for (desc, scope) in examples {
        println!("{desc}:");
        let json = serde_json::to_string(&scope).unwrap();
        println!("  JSON: {json}");

        // Pretty print for the complex one
        if desc.contains("with parent") {
            let pretty = serde_json::to_string_pretty(&scope).unwrap();
            println!(
                "  Pretty:\n{}",
                pretty
                    .lines()
                    .map(|l| format!("    {l}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        println!();
    }

    println!("=== JSON SCOPE TYPES COMPLETE ===");
}
