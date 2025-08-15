//! Tests for TypeScript parser scope tracking

use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;
use codanna::{FileId, SymbolKind};

#[test]
fn verify_typescript_scope_with_debug() {
    let mut parser = TypeScriptParser::new().unwrap();
    let code = r#"
// Module-level constant
const MAX_SIZE = 100;

// Module-level function (hoisted)
function moduleFunction() {
    const localVar = 42;
    
    // Nested function
    function nestedFunction() {
        return localVar;
    }
    
    // Arrow function (not hoisted)
    const arrowFunc = () => {
        console.log("arrow");
    };
    
    return nestedFunction();
}

// Module-level class
class MyClass {
    private field: number = 0;
    
    constructor() {
        this.field = 1;
    }
    
    public method(): void {
        const methodLocal = 10;
        
        // Inner class
        class InnerClass {
            innerMethod() {}
        }
    }
    
    static staticMethod() {
        return "static";
    }
}

// Interface at module level
interface MyInterface {
    property: string;
    method(): void;
}

// Type alias
type MyType = string | number;

// Enum
enum Color {
    Red,
    Green,
    Blue
}

// Arrow function at module level
const moduleArrow = (x: number) => x * 2;
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== TYPESCRIPT SCOPE TRACKING VERIFICATION ===\n");
    println!("Total symbols found: {}", symbols.len());
    println!("\n--- Symbol Details ---");

    for symbol in &symbols {
        let scope_str = match &symbol.scope_context {
            Some(ScopeContext::Module) => "MODULE",
            Some(ScopeContext::Local { hoisted: false }) => "LOCAL (not hoisted)",
            Some(ScopeContext::Local { hoisted: true }) => "LOCAL (hoisted)",
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
    let module_func = symbols.iter().find(|s| s.name.as_ref() == "moduleFunction");
    if let Some(mf) = module_func {
        println!(
            "moduleFunction scope: {:?} (expected: Module)",
            mf.scope_context
        );
        assert_eq!(mf.scope_context, Some(ScopeContext::Module));
    }

    let nested_func = symbols.iter().find(|s| s.name.as_ref() == "nestedFunction");
    if let Some(nf) = nested_func {
        println!(
            "nestedFunction scope: {:?} (expected: Local with hoisting)",
            nf.scope_context
        );
        // Function declarations are hoisted even when nested
        assert_eq!(
            nf.scope_context,
            Some(ScopeContext::Local { hoisted: true })
        );
    }

    let my_class = symbols.iter().find(|s| s.name.as_ref() == "MyClass");
    if let Some(mc) = my_class {
        println!("MyClass scope: {:?} (expected: Module)", mc.scope_context);
        assert_eq!(mc.scope_context, Some(ScopeContext::Module));
    }

    let method = symbols.iter().find(|s| s.name.as_ref() == "method");
    if let Some(m) = method {
        println!(
            "method scope: {:?} (expected: ClassMember)",
            m.scope_context
        );
        assert_eq!(m.scope_context, Some(ScopeContext::ClassMember));
    }

    let inner_class = symbols.iter().find(|s| s.name.as_ref() == "InnerClass");
    if let Some(ic) = inner_class {
        println!(
            "InnerClass scope: {:?} (expected: ClassMember since in method)",
            ic.scope_context
        );
        // InnerClass is inside a method, which is inside a class
        assert_eq!(ic.scope_context, Some(ScopeContext::ClassMember));
    } else {
        println!("WARNING: InnerClass not found in symbols!");
        println!("All symbols found:");
        for symbol in &symbols {
            println!(
                "  - {} at line {}",
                symbol.name.as_ref(),
                symbol.range.start_line
            );
        }
        panic!("CRITICAL: InnerClass should be extracted but is missing!");
    }

    let interface = symbols.iter().find(|s| s.name.as_ref() == "MyInterface");
    if let Some(i) = interface {
        println!(
            "MyInterface scope: {:?} (expected: Module)",
            i.scope_context
        );
        assert_eq!(i.scope_context, Some(ScopeContext::Module));
    }

    println!("\n=== SCOPE VERIFICATION COMPLETE ===\n");
}

#[test]
fn test_typescript_hoisting_distinction() {
    let mut parser = TypeScriptParser::new().unwrap();
    let code = r#"
// Test hoisting differences
console.log(hoistedFunc()); // Works - function declarations are hoisted

function hoistedFunc() {
    return "I'm hoisted!";
}

// Arrow functions are NOT hoisted
// console.log(notHoisted()); // Would error!
const notHoisted = () => "I'm not hoisted";

class TestClass {
    // Methods are not hoisted within class
    method1() {
        this.method2(); // Can call other methods
    }
    
    method2() {
        return "method2";
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== TYPESCRIPT HOISTING TEST ===\n");

    for symbol in &symbols {
        if symbol.kind == SymbolKind::Function {
            println!(
                "Function: {} - Scope: {:?}",
                symbol.name.as_ref(),
                symbol.scope_context
            );
        }
    }

    // Verify hoisting behavior once we implement it properly
    // For now, just check that scope is set
    assert!(symbols.iter().all(|s| s.scope_context.is_some()));

    println!("\n=== HOISTING TEST COMPLETE ===\n");
}
