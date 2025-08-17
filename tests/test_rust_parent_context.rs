//! Test for Rust parser parent context tracking

use codanna::FileId;
use codanna::Settings;
use codanna::parsing::Language;
use codanna::parsing::ParserFactory;
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;
use std::sync::Arc;

#[test]
fn test_rust_parent_context_tracking() {
    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings);

    let mut parser_with_behavior = factory
        .create_parser_with_behavior(Language::Rust)
        .expect("Failed to create Rust parser");
    let parser = &mut parser_with_behavior.parser;

    let code = r#"
// Module-level function with nested items
fn process_data() {
    let local_var = 42;

    // Closure (should have parent_name: "process_data")
    let transform = |x| {
        let result = x * 2;
        result
    };

    // Nested struct (should have parent_name: "process_data")
    struct DataProcessor {
        value: i32,
    }

    impl DataProcessor {
        fn process(&self) -> i32 {
            let processed = self.value * 3;
            processed
        }
    }

    transform(local_var)
}

// Module-level struct with methods
struct Calculator {
    base: i32,
}

impl Calculator {
    fn new(base: i32) -> Self {
        let instance = Self { base };
        instance
    }

    fn calculate(&self, x: i32) -> i32 {
        let result = self.base + x;

        // Inner closure in method
        let multiply = |y| {
            let product = y * 2;
            product
        };

        multiply(result)
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== RUST PARENT CONTEXT TEST ===\n");
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
                    "local_var" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("process_data"),
                            "local_var should have process_data as parent"
                        );
                    }
                    "transform" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("process_data"),
                            "transform closure should have process_data as parent"
                        );
                    }
                    "result" if symbol.range.start_line == 7 => {
                        // Result inside transform closure
                        assert!(
                            parent_name.is_some(),
                            "result in closure should have parent context"
                        );
                    }
                    "DataProcessor" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("process_data"),
                            "DataProcessor struct should have process_data as parent"
                        );
                    }
                    "processed" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("process"),
                            "processed variable should have process method as parent"
                        );
                    }
                    "instance" => {
                        assert_eq!(
                            parent_name.as_ref().map(|s| s.as_ref()),
                            Some("new"),
                            "instance variable should have new method as parent"
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
fn test_rust_deeply_nested_parent_context() {
    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings);

    let mut parser_with_behavior = factory
        .create_parser_with_behavior(Language::Rust)
        .expect("Failed to create Rust parser");
    let parser = &mut parser_with_behavior.parser;

    let code = r#"
fn outer() {
    let outer_var = 1;

    fn inner() {
        let inner_var = 2;

        fn deeply_nested() {
            let deep_var = 3;

            let closure = || {
                let closure_var = 4;
                closure_var
            };

            closure()
        }

        deeply_nested()
    }

    inner()
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== RUST DEEPLY NESTED TEST ===\n");

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
                "outer_var" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("outer"),
                        "outer_var should belong to outer function"
                    );
                }
                "inner_var" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("inner"),
                        "inner_var should belong to inner function"
                    );
                }
                "deep_var" => {
                    assert_eq!(
                        parent_name.as_ref().map(|s| s.as_ref()),
                        Some("deeply_nested"),
                        "deep_var should belong to deeply_nested function"
                    );
                }
                "closure_var" => {
                    assert!(
                        parent_name.is_some(),
                        "closure_var should have parent context"
                    );
                }
                _ => {}
            }
        }
    }

    println!("\n=== DEEPLY NESTED TEST COMPLETE ===\n");
}
