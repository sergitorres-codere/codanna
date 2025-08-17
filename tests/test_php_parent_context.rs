//! Test for PHP parser parent context tracking

use codanna::FileId;
use codanna::parsing::{LanguageParser, PhpParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;

#[test]
fn test_php_parent_context_tracking() {
    let mut parser = PhpParser::new().unwrap();

    let code = r#"<?php

// Module-level function with nested items
function processData() {
    $localVar = 42;

    // Nested function (should have parent_name: "processData")
    function transform() {
        $result = $GLOBALS['localVar'] * 2;
        return $result;
    }

    // Anonymous function (closure) - if extracted
    $compute = function($x) {
        return $x * 3;
    };

    // Nested class (should have parent_name: "processData")
    class DataProcessor {
        private $value;

        public function __construct() {
            $this->value = 0;
        }

        public function process() {
            $processed = $this->value * 3;
            return $processed;
        }
    }

    return transform();
}

// Module-level class with methods
class Calculator {
    private $base;

    public function __construct($base) {
        $this->base = $base;
    }

    public function calculate($x) {
        $result = $this->base + $x;

        // Nested function in method (PHP allows this)
        function helper($y) {
            $product = $y * 2;
            return $product;
        }

        return helper($result);
    }

    public static function create($value) {
        $instance = new self($value);
        return $instance;
    }
}

// Module-level trait
trait HelperTrait {
    public function help() {
        function nestedInTrait() {
            return "nested";
        }
        return nestedInTrait();
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PHP PARENT CONTEXT TEST ===\n");
    println!("Total symbols found: {}", symbols.len());

    // Track how many local symbols have parent context
    let mut local_with_parent = 0;
    let mut local_without_parent = 0;

    for symbol in &symbols {
        match &symbol.scope_context {
            Some(ScopeContext::Local {
                hoisted,
                parent_name,
                parent_kind,
            }) => {
                println!(
                    "Local Symbol: {:20} | Parent: {:?} ({:?}) | Hoisted: {}",
                    symbol.name.as_ref(),
                    parent_name.as_ref().map(|s| s.as_ref()),
                    parent_kind,
                    hoisted
                );

                if parent_name.is_some() {
                    local_with_parent += 1;
                } else {
                    local_without_parent += 1;
                    println!("  ❌ MISSING PARENT CONTEXT for: {}", symbol.name.as_ref());
                }

                // Specific assertions
                match symbol.name.as_ref() {
                    "transform" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("processData"),
                            "transform function should have processData as parent"
                        );
                    }
                    "DataProcessor" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("processData"),
                            "DataProcessor class should have processData as parent"
                        );
                    }
                    "helper" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("calculate"),
                            "helper function should have calculate method as parent"
                        );
                    }
                    "nestedInTrait" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("help"),
                            "nestedInTrait function should have help method as parent"
                        );
                    }
                    _ => {}
                }
            }
            Some(ScopeContext::Module) => {
                println!(
                    "Module Symbol: {:20} | Kind: {:?}",
                    symbol.name.as_ref(),
                    symbol.kind
                );
            }
            Some(ScopeContext::ClassMember) => {
                println!(
                    "Class Member: {:20} | Kind: {:?}",
                    symbol.name.as_ref(),
                    symbol.kind
                );
            }
            _ => {}
        }
    }

    println!("\n--- Summary ---");
    println!("Local symbols with parent context: {local_with_parent}");
    println!("Local symbols WITHOUT parent context: {local_without_parent}");

    // All local symbols should have parent context
    assert_eq!(
        local_without_parent, 0,
        "All local symbols should have parent context"
    );

    println!("\n=== TEST COMPLETE ===\n");
}

#[test]
fn test_php_deeply_nested_parent_context() {
    let mut parser = PhpParser::new().unwrap();

    let code = r#"<?php

function outer() {
    $outerVar = 1;

    function inner() {
        $innerVar = 2;

        function deeplyNested() {
            $deepVar = 3;

            function veryDeep() {
                $veryDeepVar = 4;
                return $veryDeepVar;
            }

            return veryDeep();
        }

        return deeplyNested();
    }

    return inner();
}

class OuterClass {
    public function method1() {
        // PHP doesn't allow class definitions inside methods directly,
        // but allows anonymous classes (PHP 7+)
        $innerClass = new class {
            public function method2() {
                function nestedInMethod() {
                    return "nested";
                }
                return nestedInMethod();
            }
        };
        return $innerClass;
    }
}

namespace MyNamespace {
    function namespacedFunc() {
        function nestedInNamespace() {
            return "nested";
        }
        return nestedInNamespace();
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PHP DEEPLY NESTED TEST ===\n");

    for symbol in &symbols {
        if let Some(ScopeContext::Local {
            parent_name,
            parent_kind,
            ..
        }) = &symbol.scope_context
        {
            println!(
                "Symbol: {:15} | Parent: {:?} ({:?})",
                symbol.name.as_ref(),
                parent_name.as_ref().map(|s| s.as_ref()),
                parent_kind
            );

            // Verify specific parent relationships
            match symbol.name.as_ref() {
                "inner" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("outer"),
                        "inner should belong to outer function"
                    );
                }
                "deeplyNested" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("inner"),
                        "deeplyNested should belong to inner function"
                    );
                }
                "veryDeep" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("deeplyNested"),
                        "veryDeep should belong to deeplyNested function"
                    );
                }
                "nestedInMethod" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("method2"),
                        "nestedInMethod should belong to method2"
                    );
                }
                "nestedInNamespace" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("namespacedFunc"),
                        "nestedInNamespace should belong to namespacedFunc"
                    );
                }
                _ => {}
            }
        }
    }

    println!("\n=== DEEPLY NESTED TEST COMPLETE ===\n");
}
