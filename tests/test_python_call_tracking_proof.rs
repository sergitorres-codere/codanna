//! Proof that Python parser correctly tracks function calls vs method calls after fix

use codanna::parsing::{LanguageParser, PythonParser};

#[test]
fn test_python_call_tracking_proof() {
    println!("\n=== PROOF: Python Call Tracking Fixed ===\n");
    println!("This test proves that after fixing extract_call_target,");
    println!("Python parser correctly distinguishes function calls from method calls.\n");

    let code = r#"
class Calculator:
    def __init__(self):
        self.result = 0

    def add(self, x, y):
        return x + y

def process_data():
    # Function calls - SHOULD be tracked by find_calls
    validate_input("test")      # ✅ Function call
    result = compute(5, 10)     # ✅ Function call
    print(result)               # ✅ Built-in function

    # Method calls - should NOT be tracked by find_calls
    calc = Calculator()         # Constructor (currently not tracked)
    calc.add(1, 2)             # ❌ Instance method
    self.helper()              # ❌ Self method
    obj.method()               # ❌ Object method
    list.append(item)          # ❌ List method
    string.upper()             # ❌ String method
    dict.get("key")            # ❌ Dict method

    # Chained method calls - should NOT be tracked
    response.json().get("data") # ❌ Chained methods

    # Module/class method calls - should NOT be tracked
    math.sqrt(16)              # ❌ Module method
    os.path.join("a", "b")     # ❌ Nested module method
    MyClass.static_method()    # ❌ Static method

    # More function calls
    transform(data)            # ✅ Function call
    nested(inner())           # ✅ Both nested and inner are function calls

def another_function():
    process_data()            # ✅ Function call
    helper()                  # ✅ Function call
"#;

    let mut parser = PythonParser::new().expect("Failed to create parser");

    println!("Testing find_calls (should ONLY find function calls):");
    println!("{}", "=".repeat(50));

    let calls = parser.find_calls(code);

    println!("Found {} function calls:", calls.len());
    for (caller, called, range) in &calls {
        println!(
            "  ✅ In '{}': calls '{}' at line {}",
            caller, called, range.start_line
        );
    }

    // Verify we found the right function calls
    let function_calls: Vec<&str> = calls.iter().map(|(_, called, _)| *called).collect();

    println!("\n🎯 Verification:");
    println!("{}", "-".repeat(50));

    // These SHOULD be found
    assert!(
        function_calls.contains(&"validate_input"),
        "Should find validate_input function call"
    );
    assert!(
        function_calls.contains(&"compute"),
        "Should find compute function call"
    );
    assert!(
        function_calls.contains(&"print"),
        "Should find print built-in function"
    );
    assert!(
        function_calls.contains(&"transform"),
        "Should find transform function call"
    );
    assert!(
        function_calls.contains(&"nested"),
        "Should find nested function call"
    );
    assert!(
        function_calls.contains(&"inner"),
        "Should find inner function call"
    );
    assert!(
        function_calls.contains(&"process_data"),
        "Should find process_data function call"
    );
    assert!(
        function_calls.contains(&"helper"),
        "Should find helper function call"
    );

    // These should NOT be found (method calls)
    assert!(
        !function_calls.contains(&"add"),
        "Should NOT find calc.add() method call"
    );
    assert!(
        !function_calls.contains(&"append"),
        "Should NOT find list.append() method call"
    );
    assert!(
        !function_calls.contains(&"upper"),
        "Should NOT find string.upper() method call"
    );
    assert!(
        !function_calls.contains(&"get"),
        "Should NOT find dict.get() method call"
    );
    assert!(
        !function_calls.contains(&"json"),
        "Should NOT find response.json() method call"
    );
    assert!(
        !function_calls.contains(&"sqrt"),
        "Should NOT find math.sqrt() method call"
    );
    assert!(
        !function_calls.contains(&"join"),
        "Should NOT find os.path.join() method call"
    );
    assert!(
        !function_calls.contains(&"static_method"),
        "Should NOT find MyClass.static_method() call"
    );
    assert!(
        !function_calls.contains(&"method"),
        "Should NOT find obj.method() call"
    );

    println!("✅ All function calls correctly identified");
    println!("✅ All method calls correctly excluded");

    println!("\n{}", "=".repeat(50));
    println!("🎉 PROOF COMPLETE: Python parser is fixed!");
    println!("{}", "=".repeat(50));
}

#[test]
fn test_python_method_calls_still_work() {
    println!("\n=== PROOF: find_method_calls still works ===\n");

    let code = r#"
def example():
    # These should be found by find_method_calls
    obj.method()
    self.helper()
    list.append(1)
    string.upper()
    response.json()
"#;

    let mut parser = PythonParser::new().expect("Failed to create parser");
    let method_calls = parser.find_method_calls(code);

    println!("Found {} method calls:", method_calls.len());
    assert!(
        !method_calls.is_empty(),
        "find_method_calls should still find method calls"
    );

    println!("✅ find_method_calls is still functional\n");
}
