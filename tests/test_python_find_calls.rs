//! Tests for Python find_calls() implementation

use codanna::parsing::{LanguageParser, PythonParser};

#[test]
fn test_python_find_calls_simple() {
    println!("\n=== Python find_calls() Simple Test ===\n");

    let code = r#"
def outer():
    inner()           # Function call
    helper(123)       # Function call with args

    x = process()     # Function call in assignment

    # Nested calls
    transform(get_data())

    # Should NOT find these (method calls):
    print("test")     # Built-in, but still a function call
    obj.method()      # Method call
    self.method()     # Method call
    list.append(1)    # Method call

def another_func():
    outer()           # Call to outer
    async_helper()    # Regular call

async def async_example():
    await fetch_data()  # Async function call
    await obj.method()  # Async method call (should NOT find)

# Lambda/anonymous functions
process_items = lambda items: filter_func(items)
"#;

    let mut parser = PythonParser::new().expect("Failed to create parser");
    let calls = parser.find_calls(code);

    println!("Found {} function calls:", calls.len());
    for (caller, called, range) in &calls {
        println!(
            "  In '{}': calls '{}' at line {}",
            caller, called, range.start_line
        );
    }

    // Verify expected calls
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
            .any(|(caller, called, _)| caller == &"outer" && called == &"get_data"),
        "Should find outer calling get_data"
    );

    assert!(
        calls
            .iter()
            .any(|(caller, called, _)| caller == &"another_func" && called == &"outer"),
        "Should find another_func calling outer"
    );

    // Python's print is a function, not a method
    assert!(
        calls
            .iter()
            .any(|(caller, called, _)| caller == &"outer" && called == &"print"),
        "Should find outer calling print (it's a function in Python)"
    );

    // Verify we DON'T find method calls
    assert!(
        !calls.iter().any(|(_, called, _)| called == &"method"),
        "Should NOT find obj.method or self.method"
    );

    assert!(
        !calls.iter().any(|(_, called, _)| called == &"append"),
        "Should NOT find list.append"
    );

    println!("\n✅ Python call tracking test completed");
    println!("   Total calls found: {}", calls.len());
}

#[test]
fn test_python_find_calls_comprehensive() {
    println!("\n=== Python find_calls() Comprehensive Test ===\n");

    let code = r#"
# Global function calls
setup()
initialize(config)

class MyClass:
    def __init__(self):
        # Constructor calls
        self.setup_internal()  # Method call (should NOT find)
        validate_input()       # Function call (should find)

    def process(self):
        # Mix of calls
        preprocess()           # Function call
        self.helper()          # Method call (should NOT find)
        result = compute(x, y) # Function call
        return finalize(result) # Function call

    @staticmethod
    def static_method():
        do_something()         # Function call

    @classmethod
    def class_method(cls):
        cls.factory()          # Method call (should NOT find)
        create_instance()      # Function call

def nested_calls():
    # Complex nested calls
    result = map(lambda x: process_item(x), filter(is_valid, get_items()))

    # List comprehension with calls
    values = [transform(x) for x in get_list() if validate(x)]

    # Dict comprehension
    mapping = {key_func(x): value_func(x) for x in source()}

# Decorators (function calls at module level)
@decorator_func(param)
@another_decorator
def decorated():
    pass

# Generator with calls
def generator():
    for item in fetch_items():
        yield process(item)

# Context manager
def with_context():
    with open_resource() as resource:
        use_resource(resource)
"#;

    let mut parser = PythonParser::new().expect("Failed to create parser");
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
