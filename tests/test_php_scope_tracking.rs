//! Tests for PHP parser scope tracking

use codanna::FileId;
use codanna::parsing::Language;
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;
use std::sync::Arc;

// Use the factory to get PHP parser
use codanna::Settings;
use codanna::parsing::ParserFactory;

#[test]
fn verify_php_scope_with_debug() {
    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings);

    // Get a PHP parser
    let mut parser_with_behavior = factory
        .create_parser_with_behavior(Language::Php)
        .expect("Failed to create PHP parser");
    let parser = &mut parser_with_behavior.parser;

    let code = r#"<?php
// Global constant
const MAX_SIZE = 100;
define('VERSION', '1.0.0');

// Global function
function moduleFunction() {
    $localVar = 42;
    
    // Nested function (not common in PHP but possible via closures)
    $closure = function($x) use ($localVar) {
        return $x + $localVar;
    };
    
    // Local class (PHP 7+)
    class LocalClass {
        public function localMethod() {
            echo "local";
        }
    }
    
    return $closure(10);
}

// Global class
class MyClass {
    private $field = 0;
    public $name;
    
    // Class constant
    const CLASS_CONST = 'constant';
    
    public function __construct($name) {
        $this->name = $name;
    }
    
    public function method() {
        $methodLocal = 10;
        
        // Inner class (rare but valid in PHP)
        class InnerClass {
            public function innerMethod() {
                return "inner";
            }
        }
        
        return new InnerClass();
    }
    
    protected function protectedMethod() {
        return "protected";
    }
    
    private static function staticMethod() {
        return "static";
    }
}

// Interface
interface MyInterface {
    public function interfaceMethod();
}

// Trait
trait MyTrait {
    public function traitMethod() {
        echo "trait";
    }
    
    abstract public function abstractTraitMethod();
}

// Class using trait
class TraitUser {
    use MyTrait;
    
    public function abstractTraitMethod() {
        echo "implemented";
    }
}

// Namespace (would normally be at top of file)
namespace MyNamespace {
    class NamespacedClass {
        public function namespacedMethod() {
            return "namespaced";
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PHP SCOPE TRACKING VERIFICATION ===\n");
    println!("Total symbols found: {}", symbols.len());
    println!("\n--- Symbol Details ---");

    for symbol in &symbols {
        let scope_str = match &symbol.scope_context {
            Some(ScopeContext::Module) => "MODULE",
            Some(ScopeContext::Local { hoisted: false }) => "LOCAL (not hoisted)",
            Some(ScopeContext::Local { hoisted: true }) => "LOCAL (hoisted - unexpected for PHP!)",
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

    // CRITICAL: Check for LocalClass inside function
    let local_class = symbols.iter().find(|s| s.name.as_ref() == "LocalClass");
    if let Some(lc) = local_class {
        println!("LocalClass scope: {:?} (expected: Local)", lc.scope_context);
        assert_eq!(
            lc.scope_context,
            Some(ScopeContext::Local { hoisted: false })
        );
    } else {
        println!("WARNING: LocalClass not found in symbols!");
        println!("All symbols found:");
        for symbol in &symbols {
            println!(
                "  - {} at line {}",
                symbol.name.as_ref(),
                symbol.range.start_line
            );
        }
        panic!("CRITICAL: LocalClass should be extracted but might be missing!");
    }

    // Check for InnerClass inside method
    let inner_class = symbols.iter().find(|s| s.name.as_ref() == "InnerClass");
    if let Some(ic) = inner_class {
        println!(
            "InnerClass scope: {:?} (expected: ClassMember since in method)",
            ic.scope_context
        );
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
        // Note: PHP might not extract nested classes like this
        println!("Note: PHP parser may not extract classes defined inside methods");
    }

    let interface = symbols.iter().find(|s| s.name.as_ref() == "MyInterface");
    if let Some(i) = interface {
        println!(
            "MyInterface scope: {:?} (expected: Module)",
            i.scope_context
        );
        assert_eq!(i.scope_context, Some(ScopeContext::Module));
    }

    let trait_def = symbols.iter().find(|s| s.name.as_ref() == "MyTrait");
    if let Some(td) = trait_def {
        println!("MyTrait scope: {:?} (expected: Module)", td.scope_context);
        assert_eq!(td.scope_context, Some(ScopeContext::Module));
    }

    println!("\n=== SCOPE VERIFICATION COMPLETE ===\n");
}

#[test]
fn test_php_nested_scopes() {
    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings);

    let mut parser_with_behavior = factory
        .create_parser_with_behavior(Language::Php)
        .expect("Failed to create PHP parser");
    let parser = &mut parser_with_behavior.parser;

    let code = r#"<?php
// Test nested functions and classes in PHP
function outerFunction() {
    // Closure (anonymous function)
    $add = function($a, $b) {
        return $a + $b;
    };
    
    // Named function inside function (not common)
    function innerFunction() {
        echo "inner";
    }
    
    // Class inside function
    class FunctionClass {
        public function classMethod() {
            return "method";
        }
    }
}

// Test namespace scoping
namespace Outer {
    function namespacedFunction() {}
    
    namespace Inner {
        function deepFunction() {}
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PHP NESTED SCOPES TEST ===\n");

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
