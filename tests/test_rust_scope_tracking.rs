//! Tests for Rust parser scope tracking

use codanna::FileId;
use codanna::parsing::Language;
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;
use std::sync::Arc;

// Import the RustParser directly (need to make it public in the lib)
// For now, we'll use the factory
use codanna::Settings;
use codanna::parsing::ParserFactory;

#[test]
fn verify_rust_scope_with_debug() {
    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings);

    // Get a Rust parser
    let mut parser_with_behavior = factory
        .create_parser_with_behavior(Language::Rust)
        .expect("Failed to create Rust parser");
    let parser = &mut parser_with_behavior.parser;

    let code = r#"
// Module-level constant
const MAX_SIZE: usize = 100;

// Module-level static
static mut COUNTER: u32 = 0;

// Module-level function
fn module_function() {
    let local_var = 42;

    // Nested function (closure)
    let closure = |x| {
        x + local_var
    };

    // Nested type
    struct InnerStruct {
        field: i32,
    }

    closure(10);
}

// Module-level struct
struct MyStruct {
    field: i32,
    name: String,
}

// Impl block for struct
impl MyStruct {
    // Associated function (static method)
    fn new(name: String) -> Self {
        Self {
            field: 0,
            name,
        }
    }

    // Instance method
    fn method(&self) -> i32 {
        self.field
    }

    // Mutable method
    fn mut_method(&mut self) {
        self.field += 1;
    }
}

// Module-level trait
trait MyTrait {
    fn trait_method(&self);
    fn default_method(&self) {
        println!("default");
    }
}

// Trait implementation
impl MyTrait for MyStruct {
    fn trait_method(&self) {
        println!("impl");
    }
}

// Module-level enum
enum MyEnum {
    Variant1,
    Variant2(i32),
    Variant3 { x: i32, y: i32 },
}

// Type alias
type MyType = Vec<String>;

// Module
mod submodule {
    pub fn submodule_function() {
        println!("in submodule");
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== RUST SCOPE TRACKING VERIFICATION ===\n");
    println!("Total symbols found: {}", symbols.len());
    println!("\n--- Symbol Details ---");

    for symbol in &symbols {
        let scope_str = match &symbol.scope_context {
            Some(ScopeContext::Module) => "MODULE",
            Some(ScopeContext::Local { hoisted: false, .. }) => "LOCAL (not hoisted)",
            Some(ScopeContext::Local { hoisted: true, .. }) => {
                "LOCAL (hoisted - unexpected for Rust!)"
            }
            Some(ScopeContext::ClassMember) => "CLASS_MEMBER",
            Some(ScopeContext::Parameter) => "PARAMETER",
            Some(ScopeContext::Package) => "PACKAGE",
            Some(ScopeContext::Global) => "GLOBAL",
            None => "NONE (ERROR: scope not set!)",
        };

        println!(
            "Symbol: {:20} | Kind: {:12} | Scope: {:20} | Line: {}",
            symbol.name.as_ref(),
            format!("{:?}", symbol.kind),
            scope_str,
            symbol.range.start_line
        );
    }

    println!("\n--- Scope Verification ---");

    // Check specific symbols
    let module_func = symbols
        .iter()
        .find(|s| s.name.as_ref() == "module_function");
    if let Some(mf) = module_func {
        println!(
            "module_function scope: {:?} (expected: Module)",
            mf.scope_context
        );
        assert_eq!(mf.scope_context, Some(ScopeContext::Module));
    }

    let my_struct = symbols.iter().find(|s| s.name.as_ref() == "MyStruct");
    if let Some(ms) = my_struct {
        println!("MyStruct scope: {:?} (expected: Module)", ms.scope_context);
        assert_eq!(ms.scope_context, Some(ScopeContext::Module));
    }

    let new_method = symbols.iter().find(|s| s.name.as_ref() == "new");
    if let Some(nm) = new_method {
        println!(
            "new (associated fn) scope: {:?} (expected: ClassMember)",
            nm.scope_context
        );
        assert_eq!(nm.scope_context, Some(ScopeContext::ClassMember));
    }

    let method = symbols.iter().find(|s| s.name.as_ref() == "method");
    if let Some(m) = method {
        println!(
            "method scope: {:?} (expected: ClassMember)",
            m.scope_context
        );
        assert_eq!(m.scope_context, Some(ScopeContext::ClassMember));
    }

    let trait_def = symbols.iter().find(|s| s.name.as_ref() == "MyTrait");
    if let Some(td) = trait_def {
        println!("MyTrait scope: {:?} (expected: Module)", td.scope_context);
        assert_eq!(td.scope_context, Some(ScopeContext::Module));
    }

    let trait_method = symbols.iter().find(|s| s.name.as_ref() == "trait_method");
    if let Some(tm) = trait_method {
        println!(
            "trait_method scope: {:?} (expected: ClassMember)",
            tm.scope_context
        );
        // Trait methods are like class members
        assert_eq!(tm.scope_context, Some(ScopeContext::ClassMember));
    }

    let my_enum = symbols.iter().find(|s| s.name.as_ref() == "MyEnum");
    if let Some(me) = my_enum {
        println!("MyEnum scope: {:?} (expected: Module)", me.scope_context);
        assert_eq!(me.scope_context, Some(ScopeContext::Module));
    }

    // CRITICAL: Check for InnerStruct that should be inside module_function
    let inner_struct = symbols.iter().find(|s| s.name.as_ref() == "InnerStruct");
    if let Some(is) = inner_struct {
        println!(
            "InnerStruct scope: {:?} (expected: Local with parent context)",
            is.scope_context
        );
        // Rust parser now correctly tracks parent context!
        assert_eq!(
            is.scope_context,
            Some(ScopeContext::Local {
                hoisted: false,
                parent_name: Some("module_function".into()),
                parent_kind: Some(codanna::SymbolKind::Function)
            })
        );
    } else {
        println!("WARNING: InnerStruct not found in symbols!");
        println!("All symbols found:");
        for symbol in &symbols {
            println!(
                "  - {} at line {}",
                symbol.name.as_ref(),
                symbol.range.start_line
            );
        }
        panic!("CRITICAL: InnerStruct should be extracted but might be missing!");
    }

    // Check for submodule (should be extracted as a module)
    let submodule = symbols.iter().find(|s| s.name.as_ref() == "submodule");
    if let Some(sm) = submodule {
        println!("submodule scope: {:?} (expected: Module)", sm.scope_context);
        // Note: submodule might not be extracted by current parser
    } else {
        println!("Note: submodule not extracted (Rust parser may not extract mod items)")
    }

    println!("\n=== SCOPE VERIFICATION COMPLETE ===\n");
}

#[test]
fn test_rust_nested_scopes() {
    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings);

    let mut parser_with_behavior = factory
        .create_parser_with_behavior(Language::Rust)
        .expect("Failed to create Rust parser");
    let parser = &mut parser_with_behavior.parser;

    let code = r#"
// Test nested functions and types in Rust
fn outer_function() {
    // Inner struct (not commonly used but valid)
    struct InnerStruct {
        value: i32,
    }

    impl InnerStruct {
        fn inner_method(&self) -> i32 {
            self.value
        }
    }

    // Closure
    let add = |a, b| a + b;

    // Nested function via closure
    let nested = || {
        println!("nested");
    };
}

// Test module nesting
mod outer_mod {
    pub fn outer_mod_fn() {}

    pub mod inner_mod {
        pub fn inner_mod_fn() {}
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== RUST NESTED SCOPES TEST ===\n");

    for symbol in &symbols {
        println!(
            "Symbol: {} - Scope: {:?}",
            symbol.name.as_ref(),
            symbol.scope_context
        );
    }

    // Verify all symbols have scope context
    assert!(symbols.iter().all(|s| s.scope_context.is_some()));

    println!("\n=== NESTED SCOPES TEST COMPLETE ===\n");
}

#[test]
fn test_rust_qualified_function_calls() {
    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings);

    let mut parser_with_behavior = factory
        .create_parser_with_behavior(Language::Rust)
        .expect("Failed to create Rust parser");
    let parser = &mut parser_with_behavior.parser;

    let code = r#"
struct Foo;
struct Bar;

impl Foo {
    pub fn new() -> Self {
        Foo
    }
}

impl Bar {
    pub fn new() -> Self {
        Bar
    }
}

fn test_calls() {
    // These should be tracked as different qualified calls
    let f = Foo::new();  // Should be "Foo::new"
    let b = Bar::new();  // Should be "Bar::new"

    // Standard library calls
    let s = String::new();  // Should be "String::new"
    let v = Vec::<i32>::new();  // Should be "Vec::new" or "Vec::<i32>::new"
}
"#;

    // Get the function calls
    let calls = parser.find_calls(code);

    println!("\n=== RUST QUALIFIED FUNCTION CALLS TEST ===\n");
    println!("Found {} function calls:", calls.len());

    for (caller, target, range) in &calls {
        println!(
            "  {} -> '{}' at line {}",
            caller,
            target,
            range.start_line + 1
        );
    }

    // Verify we found the calls
    assert!(calls.len() >= 4, "Should find at least 4 function calls");

    // Check that qualified names are preserved
    let foo_new = calls
        .iter()
        .find(|(_, target, _)| target.contains("Foo") && target.contains("new"));
    let bar_new = calls
        .iter()
        .find(|(_, target, _)| target.contains("Bar") && target.contains("new"));
    let string_new = calls
        .iter()
        .find(|(_, target, _)| target.contains("String") && target.contains("new"));

    // The current implementation should preserve "Foo::new", "Bar::new", etc.
    assert!(foo_new.is_some(), "Should find Foo::new call");
    assert!(bar_new.is_some(), "Should find Bar::new call");
    assert!(string_new.is_some(), "Should find String::new call");

    // Verify the full qualified names are preserved
    if let Some((_, target, _)) = foo_new {
        println!("Foo::new tracked as: '{target}'");
        assert_eq!(
            *target, "Foo::new",
            "Should preserve full qualified name 'Foo::new'"
        );
    }

    if let Some((_, target, _)) = bar_new {
        println!("Bar::new tracked as: '{target}'");
        assert_eq!(
            *target, "Bar::new",
            "Should preserve full qualified name 'Bar::new'"
        );
    }

    if let Some((_, target, _)) = string_new {
        println!("String::new tracked as: '{target}'");
        assert_eq!(
            *target, "String::new",
            "Should preserve full qualified name 'String::new'"
        );
    }

    println!("\n=== QUALIFIED FUNCTION CALLS TEST COMPLETE ===\n");
}
