//! Demonstration test showing how parent context will look in scope_context
//! This test manually creates symbols with parent context to show the feature working

use codanna::Symbol;
use codanna::parsing::{ParserContext, ScopeType};
use codanna::symbol::ScopeContext;
use codanna::types::{FileId, Range, SymbolCounter, SymbolKind};

#[test]
fn demonstrate_parent_context_in_scope() {
    println!("\n=== PARENT CONTEXT DEMONSTRATION ===\n");

    // Create a parser context to simulate parsing
    let mut context = ParserContext::new();
    let mut counter = SymbolCounter::new();
    let file_id = FileId::new(1).unwrap();

    // Simulate entering a function called "processData"
    context.enter_scope(ScopeType::hoisting_function());
    context.set_current_function(Some("processData".to_string()));

    // Create a symbol that's local to this function
    let local_var = Symbol::new(
        counter.next_id(),
        "localVariable",
        SymbolKind::Variable,
        file_id,
        Range::new(10, 4, 10, 20),
    )
    .with_scope(context.current_scope_context());

    // Print what the scope looks like
    println!("Symbol: {}", local_var.name.as_ref());
    println!("Kind: {:?}", local_var.kind);

    match &local_var.scope_context {
        Some(ScopeContext::Local {
            hoisted,
            parent_name,
            parent_kind,
        }) => {
            println!("Scope: Local");
            println!("  - Hoisted: {hoisted}");
            println!("  - Parent Name: {parent_name:?}");
            println!("  - Parent Kind: {parent_kind:?}");

            // Verify the parent context is set correctly
            assert_eq!(
                parent_name.as_ref().map(|s| s.as_ref()),
                Some("processData")
            );
            assert_eq!(parent_kind, &Some(SymbolKind::Function));
        }
        _ => panic!("Expected Local scope"),
    }

    // Exit function and enter a class
    context.exit_scope();
    context.enter_scope(ScopeType::Class);
    context.set_current_class(Some("UserManager".to_string()));

    // Enter a method within the class
    context.enter_scope(ScopeType::Function { hoisting: false });
    context.set_current_function(Some("getUserById".to_string()));

    // Create a symbol local to the method
    let method_local = Symbol::new(
        counter.next_id(),
        "userId",
        SymbolKind::Variable,
        file_id,
        Range::new(25, 8, 25, 20),
    )
    .with_scope(context.current_scope_context());

    println!("\nSymbol: {}", method_local.name.as_ref());
    println!("Kind: {:?}", method_local.kind);

    match &method_local.scope_context {
        Some(ScopeContext::Local {
            hoisted,
            parent_name,
            parent_kind,
        }) => {
            println!("Scope: Local");
            println!("  - Hoisted: {hoisted}");
            println!("  - Parent Name: {parent_name:?}");
            println!("  - Parent Kind: {parent_kind:?}");

            // When inside a method, the parent is the function, not the class
            // (because we're tracking immediate parent)
            assert_eq!(
                parent_name.as_ref().map(|s| s.as_ref()),
                Some("getUserById")
            );
            assert_eq!(parent_kind, &Some(SymbolKind::Function));
        }
        _ => panic!("Expected Local scope"),
    }

    println!("\n=== Demonstrating JSON-like output ===\n");

    // Show what it would look like in JSON (using Debug format which is what gets serialized)
    println!("scope_context field in JSON would look like:");
    println!("{:#?}", local_var.scope_context);

    println!("\n=== DEMONSTRATION COMPLETE ===");
}

#[test]
fn demonstrate_nested_function_parent_context() {
    println!("\n=== NESTED FUNCTION PARENT CONTEXT ===\n");

    let mut context = ParserContext::new();
    let mut counter = SymbolCounter::new();
    let file_id = FileId::new(1).unwrap();

    // Simulate: function outer() { function inner() { let x = 1; } }

    // Enter outer function
    context.enter_scope(ScopeType::hoisting_function());
    context.set_current_function(Some("outerFunction".to_string()));

    // Create the inner function symbol itself
    let inner_func = Symbol::new(
        counter.next_id(),
        "innerFunction",
        SymbolKind::Function,
        file_id,
        Range::new(5, 4, 8, 5),
    )
    .with_scope(context.current_scope_context());

    println!("Inner function symbol:");
    match &inner_func.scope_context {
        Some(ScopeContext::Local {
            hoisted,
            parent_name,
            parent_kind,
        }) => {
            println!("  innerFunction is Local to: {parent_name:?} (a {parent_kind:?})");
            println!("  Hoisted: {hoisted}");

            assert_eq!(
                parent_name.as_ref().map(|s| s.as_ref()),
                Some("outerFunction")
            );
        }
        _ => panic!("Expected Local scope for inner function"),
    }

    // Now enter the inner function
    context.enter_scope(ScopeType::hoisting_function());
    context.set_current_function(Some("innerFunction".to_string()));

    // Create a variable inside the inner function
    let inner_var = Symbol::new(
        counter.next_id(),
        "innerVariable",
        SymbolKind::Variable,
        file_id,
        Range::new(6, 8, 6, 20),
    )
    .with_scope(context.current_scope_context());

    println!("\nVariable inside inner function:");
    match &inner_var.scope_context {
        Some(ScopeContext::Local {
            hoisted,
            parent_name,
            parent_kind,
        }) => {
            println!("  innerVariable is Local to: {parent_name:?} (a {parent_kind:?})");
            println!("  Hoisted: {hoisted}");

            // The variable's parent is the inner function
            assert_eq!(
                parent_name.as_ref().map(|s| s.as_ref()),
                Some("innerFunction")
            );
        }
        _ => panic!("Expected Local scope for inner variable"),
    }

    println!("\n=== NESTED CONTEXT DEMONSTRATION COMPLETE ===");
}

#[test]
fn demonstrate_serialized_format() {
    println!("\n=== SERIALIZED FORMAT DEMONSTRATION ===\n");

    // Create different scope contexts to show how they serialize
    let examples = vec![
        ("Module level function", ScopeContext::Module),
        (
            "Local variable in function 'main'",
            ScopeContext::Local {
                hoisted: false,
                parent_name: Some("main".to_string().into()),
                parent_kind: Some(SymbolKind::Function),
            },
        ),
        (
            "Hoisted function inside 'processData'",
            ScopeContext::Local {
                hoisted: true,
                parent_name: Some("processData".to_string().into()),
                parent_kind: Some(SymbolKind::Function),
            },
        ),
        ("Method in a class", ScopeContext::ClassMember),
        (
            "Variable in method 'save' of class 'User'",
            ScopeContext::Local {
                hoisted: false,
                parent_name: Some("save".to_string().into()),
                parent_kind: Some(SymbolKind::Function), // Methods are tracked as Functions in SymbolKind
            },
        ),
    ];

    for (description, scope) in examples {
        println!("{description}:");
        println!("  Debug format: {scope:?}");

        // This is what gets stored in Tantivy as a string
        let serialized = format!("{scope:?}");
        println!("  Serialized: {serialized}");
        println!();
    }

    println!("=== SERIALIZATION DEMONSTRATION COMPLETE ===");
}
