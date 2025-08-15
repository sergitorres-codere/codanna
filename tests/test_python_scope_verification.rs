//! Verification tests for Python parser scope tracking with debug output

use codanna::parsing::{LanguageParser, PythonParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;
use codanna::{FileId, SymbolKind};

#[test]
fn verify_python_scope_with_debug_output() {
    let mut parser = PythonParser::new().unwrap();
    let code = r#"
# Module-level constant
MAX_SIZE = 100

def module_func():
    """Module function"""
    local_var = 42
    
    def nested_func():
        """Nested function"""
        nested_var = 10
        return nested_var
    
    return nested_func()

class MyClass:
    """Module class"""
    CLASS_ATTR = "shared"
    
    def __init__(self):
        """Constructor"""
        self.instance_var = 0
    
    def method(self):
        """Instance method"""
        method_local = 5
        
        class InnerClass:
            """Class inside method"""
            pass
            
        return method_local
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PYTHON SCOPE TRACKING VERIFICATION ===\n");
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

    // Verify module-level symbols
    let max_size = symbols
        .iter()
        .find(|s| s.name.as_ref() == "MAX_SIZE")
        .unwrap();
    println!(
        "MAX_SIZE scope: {:?} (expected: Module)",
        max_size.scope_context
    );
    assert_eq!(max_size.scope_context, Some(ScopeContext::Module));

    let module_func = symbols
        .iter()
        .find(|s| s.name.as_ref() == "module_func")
        .unwrap();
    println!(
        "module_func scope: {:?} (expected: Module)",
        module_func.scope_context
    );
    assert_eq!(module_func.scope_context, Some(ScopeContext::Module));

    // Verify nested function (should be Local)
    let nested_func = symbols
        .iter()
        .find(|s| s.name.as_ref() == "nested_func")
        .unwrap();
    println!(
        "nested_func scope: {:?} (expected: Local)",
        nested_func.scope_context
    );
    assert_eq!(
        nested_func.scope_context,
        Some(ScopeContext::Local { hoisted: false })
    );

    // Verify class (should be Module)
    let my_class = symbols
        .iter()
        .find(|s| s.name.as_ref() == "MyClass")
        .unwrap();
    println!(
        "MyClass scope: {:?} (expected: Module)",
        my_class.scope_context
    );
    assert_eq!(my_class.scope_context, Some(ScopeContext::Module));

    // Verify methods (should be ClassMember)
    let init_method = symbols
        .iter()
        .find(|s| s.name.as_ref() == "__init__")
        .unwrap();
    println!(
        "__init__ scope: {:?} (expected: ClassMember)",
        init_method.scope_context
    );
    assert_eq!(init_method.scope_context, Some(ScopeContext::ClassMember));

    let method = symbols
        .iter()
        .find(|s| s.name.as_ref() == "method")
        .unwrap();
    println!(
        "method scope: {:?} (expected: ClassMember)",
        method.scope_context
    );
    assert_eq!(method.scope_context, Some(ScopeContext::ClassMember));

    // Verify inner class (should be Local since it's inside a method)
    let inner_class = symbols
        .iter()
        .find(|s| s.name.as_ref() == "InnerClass")
        .unwrap();
    println!(
        "InnerClass scope: {:?} (expected: Local)",
        inner_class.scope_context
    );
    assert_eq!(
        inner_class.scope_context,
        Some(ScopeContext::Local { hoisted: false })
    );

    println!("\n=== ALL SCOPE VERIFICATIONS PASSED ===\n");
}

#[test]
fn verify_python_variable_scopes() {
    let mut parser = PythonParser::new().unwrap();
    let code = r#"
# Global constant
GLOBAL_CONST = 42

# Global variable
global_var = "module level"

def outer():
    outer_local = 10
    OUTER_CONST = 100  # Should be local constant
    
    def inner():
        inner_var = 20
        return inner_var + outer_local
    
    return inner

class TestClass:
    class_var = "shared"
    
    def method(self):
        method_var = 30
        return method_var
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PYTHON VARIABLE SCOPE VERIFICATION ===\n");

    // Look for all variables and constants
    for symbol in &symbols {
        if symbol.kind == SymbolKind::Variable || symbol.kind == SymbolKind::Constant {
            let scope_str = match &symbol.scope_context {
                Some(ScopeContext::Module) => "MODULE",
                Some(ScopeContext::Local { .. }) => "LOCAL",
                Some(ScopeContext::ClassMember) => "CLASS_MEMBER",
                _ => "OTHER",
            };

            println!(
                "Variable/Constant: {:15} | Kind: {:10} | Scope: {:15}",
                symbol.name.as_ref(),
                if symbol.kind == SymbolKind::Constant {
                    "CONSTANT"
                } else {
                    "VARIABLE"
                },
                scope_str
            );
        }
    }

    // Specific verifications
    let global_const = symbols.iter().find(|s| s.name.as_ref() == "GLOBAL_CONST");
    if let Some(gc) = global_const {
        println!(
            "\nGLOBAL_CONST: kind={:?}, scope={:?}",
            gc.kind, gc.scope_context
        );
        assert_eq!(gc.kind, SymbolKind::Constant);
        assert_eq!(gc.scope_context, Some(ScopeContext::Module));
    }

    let global_var = symbols.iter().find(|s| s.name.as_ref() == "global_var");
    if let Some(gv) = global_var {
        println!(
            "global_var: kind={:?}, scope={:?}",
            gv.kind, gv.scope_context
        );
        assert_eq!(gv.kind, SymbolKind::Variable);
        assert_eq!(gv.scope_context, Some(ScopeContext::Module));
    }

    println!("\n=== VARIABLE SCOPE VERIFICATION COMPLETE ===\n");
}

#[test]
fn verify_edge_cases() {
    let mut parser = PythonParser::new().unwrap();
    let code = r#"
def func1():
    def func2():
        def func3():
            def func4():
                "Deeply nested"
                pass
            return func4
        return func3
    return func2

class A:
    class B:
        class C:
            "Nested classes"
            pass
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== EDGE CASE VERIFICATION ===\n");
    println!("Testing deeply nested functions and classes:\n");

    for symbol in &symbols {
        let indent = match &symbol.scope_context {
            Some(ScopeContext::Module) => "",
            Some(ScopeContext::Local { .. }) => "  ",
            Some(ScopeContext::ClassMember) => "    ",
            _ => "      ",
        };

        println!(
            "{}{:10} [{:?}] - Scope: {:?}",
            indent,
            symbol.name.as_ref(),
            symbol.kind,
            symbol.scope_context
        );
    }

    // Verify the deepest function
    if let Some(func4) = symbols.iter().find(|s| s.name.as_ref() == "func4") {
        println!(
            "\nDeepest function (func4) scope: {:?}",
            func4.scope_context
        );
        assert_eq!(
            func4.scope_context,
            Some(ScopeContext::Local { hoisted: false })
        );
    }

    println!("\n=== EDGE CASE VERIFICATION COMPLETE ===\n");
}
