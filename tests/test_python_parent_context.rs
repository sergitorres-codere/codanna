//! Test for Python parser parent context tracking

use codanna::FileId;
use codanna::parsing::{LanguageParser, PythonParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;

#[test]
fn test_python_parent_context_tracking() {
    let mut parser = PythonParser::new().unwrap();

    let code = r#"
# Module-level function with nested items
def process_data():
    local_var = 42

    # Nested function (should have parent_name: "process_data")
    def transform():
        result = local_var * 2
        return result

    # Lambda (if extracted, should have parent context)
    compute = lambda x: x * 3

    # Nested class (should have parent_name: "process_data")
    class DataProcessor:
        def __init__(self):
            self.value = 0

        def process(self):
            processed = self.value * 3
            return processed

    return transform()

# Module-level class with methods
class Calculator:
    base = 10

    def __init__(self, base):
        self.base = base

    def calculate(self, x):
        result = self.base + x

        # Inner function in method
        def helper(y):
            product = y * 2
            return product

        return helper(result)

    @classmethod
    def create(cls, value):
        instance = cls(value)
        return instance
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PYTHON PARENT CONTEXT TEST ===\n");
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
                            Some("process_data"),
                            "transform function should have process_data as parent"
                        );
                    }
                    "DataProcessor" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("process_data"),
                            "DataProcessor class should have process_data as parent"
                        );
                    }
                    "helper" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("calculate"),
                            "helper function should have calculate method as parent"
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
fn test_python_deeply_nested_parent_context() {
    let mut parser = PythonParser::new().unwrap();

    let code = r#"
def outer():
    outer_var = 1

    def inner():
        inner_var = 2

        def deeply_nested():
            deep_var = 3

            def very_deep():
                very_deep_var = 4
                return very_deep_var

            return very_deep()

        return deeply_nested()

    return inner()

class OuterClass:
    def method1(self):
        class InnerClass:
            def method2(self):
                def nested_in_method():
                    return "nested"
                return nested_in_method()
        return InnerClass()
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PYTHON DEEPLY NESTED TEST ===\n");

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
                "deeply_nested" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("inner"),
                        "deeply_nested should belong to inner function"
                    );
                }
                "very_deep" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("deeply_nested"),
                        "very_deep should belong to deeply_nested function"
                    );
                }
                "InnerClass" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("method1"),
                        "InnerClass should belong to method1"
                    );
                }
                "nested_in_method" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("method2"),
                        "nested_in_method should belong to method2"
                    );
                }
                _ => {}
            }
        }
    }

    println!("\n=== DEEPLY NESTED TEST COMPLETE ===\n");
}
