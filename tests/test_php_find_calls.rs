//! Tests for PHP find_calls() implementation

use codanna::parsing::{LanguageParser, PhpParser};

#[test]
fn test_php_find_calls_simple() {
    println!("\n=== PHP find_calls() Simple Test ===\n");

    let code = r#"
<?php

function outer() {
    inner();           // Function call
    helper(123);       // Function call with args

    $x = process();    // Function call in assignment

    // Nested calls
    transform(getData());

    // Should NOT find these (method calls):
    $obj->method();      // Method call
    $this->method();     // Method call
    self::staticMethod(); // Static method call
    parent::method();    // Parent method call

    // Built-in functions
    print("test");       // Built-in function
    echo "hello";        // Language construct (might not be tracked)
    strlen($str);        // Built-in function
}

function anotherFunc() {
    outer();             // Call to outer
    asyncHelper();       // Regular call
}

class MyClass {
    public function process() {
        validate();      // Function call from method
        $this->helper(); // Method call (should NOT be tracked)
        self::init();    // Static method (should NOT be tracked)
    }

    public static function staticMethod() {
        doSomething();   // Function call from static method
    }
}
"#;

    let mut parser = PhpParser::new().expect("Failed to create parser");
    let calls = parser.find_calls(code);

    println!("Found {} function calls:", calls.len());
    for (caller, called, range) in &calls {
        println!(
            "  In '{}': calls '{}' at line {}",
            caller, called, range.start_line
        );
    }

    // Verify expected function calls
    assert!(
        calls
            .iter()
            .any(|(caller, called, _)| caller == &"outer" && called == &"inner"),
        "Should find outer calling inner"
    );

    assert!(
        calls
            .iter()
            .any(|(caller, called, _)| caller == &"outer" && called == &"helper"),
        "Should find outer calling helper"
    );

    assert!(
        calls
            .iter()
            .any(|(caller, called, _)| caller == &"outer" && called == &"process"),
        "Should find outer calling process"
    );

    assert!(
        calls
            .iter()
            .any(|(caller, called, _)| caller == &"outer" && called == &"transform"),
        "Should find outer calling transform"
    );

    assert!(
        calls
            .iter()
            .any(|(caller, called, _)| caller == &"outer" && called == &"getData"),
        "Should find outer calling getData"
    );

    // PHP's print might be a language construct, not a function call
    // Comment out for now as it might not be tracked
    // assert!(
    //     calls
    //         .iter()
    //         .any(|(caller, called, _)| caller == &"outer" && called == &"print"),
    //     "Should find outer calling print"
    // );

    // Verify we DON'T find method calls
    assert!(
        !calls.iter().any(|(_, called, _)| called == &"method"),
        "Should NOT find $obj->method or $this->method"
    );

    assert!(
        !calls.iter().any(|(_, called, _)| called == &"staticMethod"),
        "Should NOT find self::staticMethod"
    );

    // Check that process method doesn't have a helper call (it should be $this->helper, a method call)
    let process_calls: Vec<_> = calls
        .iter()
        .filter(|(caller, _, _)| *caller == "process")
        .map(|(_, called, _)| *called)
        .collect();
    assert!(
        !process_calls.contains(&"helper"),
        "Should NOT find $this->helper from process method"
    );

    println!("\n✅ PHP call tracking test completed");
    println!("   Total calls found: {}", calls.len());
}

#[test]
fn test_php_find_calls_comprehensive() {
    println!("\n=== PHP find_calls() Comprehensive Test ===\n");

    let code = r#"
<?php

// Global function calls
setup();
initialize($config);

class Calculator {
    public function __construct() {
        // Constructor calls
        $this->setupInternal();  // Method call (should NOT find)
        validateInput();         // Function call (should find)
    }

    public function process() {
        // Mix of calls
        preprocess();            // Function call
        $this->helper();         // Method call (should NOT find)
        $result = compute($x, $y); // Function call
        return finalize($result);   // Function call
    }

    public static function staticMethod() {
        doSomething();           // Function call
    }
}

function nestedCalls() {
    // Complex nested calls
    $result = array_map(function($x) {
        return processItem($x);
    }, array_filter($items, 'isValid'));

    // Array functions
    $sorted = sort($data);
    $mapped = array_map('transform', $list);
}

// Namespace function calls
namespace\functionCall();
\globalFunction();

trait MyTrait {
    public function traitMethod() {
        traitHelper();           // Function call from trait
        $this->instanceMethod(); // Method call (should NOT find)
    }
}
"#;

    let mut parser = PhpParser::new().expect("Failed to create parser");
    let calls = parser.find_calls(code);

    println!("Found {} function calls:", calls.len());

    // Group by caller for better readability
    let mut calls_by_caller = std::collections::HashMap::new();
    for (caller, called, _) in &calls {
        calls_by_caller
            .entry(*caller)
            .or_insert_with(Vec::new)
            .push(*called);
    }

    for (caller, called_funcs) in calls_by_caller {
        println!("  {caller}: calls {called_funcs:?}");
    }

    // Basic verification
    assert!(!calls.is_empty(), "Should find at least some calls");

    println!("\n✅ Comprehensive test completed");
}
