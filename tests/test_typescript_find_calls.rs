//! Tests for TypeScript find_calls() implementation

use codanna::parsing::{LanguageParser, TypeScriptParser};

#[test]
fn test_typescript_find_calls_proof() {
    println!("\n=== TypeScript find_calls() Proof Test ===\n");

    let code = include_str!("../examples/typescript/comprehensive.ts");
    let mut parser = TypeScriptParser::new().expect("Failed to create parser");

    let calls = parser.find_calls(code);

    println!("Found {} function calls:", calls.len());

    // Print first 20 for debugging
    for (i, (caller, called, range)) in calls.iter().enumerate() {
        if i >= 20 {
            println!("  ... and {} more", calls.len() - 20);
            break;
        }
        println!(
            "  In function '{}': calls '{}' at line {}",
            caller, called, range.start_line
        );
    }

    // Verify some specific calls we expect to find in comprehensive.ts

    // In formatDate function (line 33), no calls but it uses toISOString method
    // In add function (line 38), just arithmetic

    // In fetch method (line 191-194), should call fetch and json
    assert!(
        calls
            .iter()
            .any(|(caller, called, _)| caller == &"fetch" && called == &"fetch"),
        "Should find fetch calling fetch global function"
    );

    // In createUser function (line 270-281), should call Date.now()
    // Note: Date.now() is a method call, not a function call

    // In sum function (line 284-286), uses reduce which is a method

    // In parse function (line 291-293), should call JSON.parse
    // Note: JSON.parse is also a method call

    // In fetchData async function (line 333-336), should call fetch
    assert!(
        calls
            .iter()
            .any(|(caller, called, _)| caller == &"fetchData" && called == &"fetch"),
        "Should find fetchData calling fetch"
    );

    // In Application.start method (line 478-480), console.log is a method call

    println!("\n--- Checking for false positives ---");

    // Should NOT find method calls like console.log, this.method(), etc.
    assert!(
        !calls.iter().any(|(_, called, _)| called == &"log"),
        "Should NOT find console.log (it's a method call)"
    );

    assert!(
        !calls.iter().any(|(_, called, _)| called == &"push"),
        "Should NOT find array.push (it's a method call)"
    );

    assert!(
        !calls.iter().any(|(_, called, _)| called == &"json"),
        "Should NOT find response.json (it's a method call)"
    );

    println!("\n✅ Function call extraction verified");
    println!("   Total calls found: {}", calls.len());
}

#[test]
fn test_typescript_find_calls_simple() {
    println!("\n=== TypeScript find_calls() Simple Test ===\n");

    let code = r#"
function outer() {
    inner();           // Function call
    helper(123);       // Function call with args
    
    const x = process(); // Function call in assignment
    
    // Nested calls
    transform(getData());
    
    // Should NOT find these (method calls):
    console.log("test");
    array.push(1);
    this.method();
    obj.property.deepMethod();
}

async function asyncExample() {
    await fetchData();  // Async function call
    await obj.method(); // Async method call (should NOT find)
}

const arrow = () => {
    arrowHelper();     // Call from arrow function
};
"#;

    let mut parser = TypeScriptParser::new().expect("Failed to create parser");
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
            .any(|(caller, called, _)| caller == &"outer" && called == &"getData"),
        "Should find outer calling getData"
    );

    assert!(
        calls
            .iter()
            .any(|(caller, called, _)| caller == &"asyncExample" && called == &"fetchData"),
        "Should find asyncExample calling fetchData"
    );

    // Arrow functions might not have names tracked correctly yet
    // This is a known limitation we can address later

    // Verify we DON'T find method calls
    assert!(
        !calls.iter().any(|(_, called, _)| called == &"log"),
        "Should NOT find console.log"
    );

    assert!(
        !calls.iter().any(|(_, called, _)| called == &"push"),
        "Should NOT find array.push"
    );

    assert!(
        !calls.iter().any(|(_, called, _)| called == &"method"),
        "Should NOT find this.method or obj.method"
    );

    println!("\n✅ Simple test passed");
}
