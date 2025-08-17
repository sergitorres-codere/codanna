//! Tests for TypeScript find_method_calls() implementation

use codanna::parsing::{LanguageParser, TypeScriptParser};

#[test]
fn test_typescript_find_method_calls_simple() {
    println!("\n=== TypeScript find_method_calls() Simple Test ===\n");

    let code = r#"
class UserService {
    validate() {
        this.checkAuth();           // Method call on this
        this.logger.info("test");   // Chained method call
    }

    async process(data: any) {
        data.transform();           // Method call on parameter
        await data.save();          // Async method call

        // Chained calls
        data
            .filter(x => x > 0)     // Method call
            .map(x => x * 2)        // Chained method
            .reduce((a, b) => a + b); // Another chained
    }
}

function standalone() {
    const arr = [1, 2, 3];
    arr.map(x => x * 2);           // Array method

    console.log("test");            // Console method

    // Optional chaining
    obj?.method?.();                // Optional method call

    // Regular function calls (should NOT find)
    regularFunction();
    helperFunction(123);
}
"#;

    let mut parser = TypeScriptParser::new().expect("Failed to create parser");
    let method_calls = parser.find_method_calls(code);

    println!("Found {} method calls:", method_calls.len());
    for call in &method_calls {
        println!(
            "  In '{}': calls '{}.{}' at line {}",
            call.caller,
            call.receiver.as_ref().unwrap_or(&"<none>".to_string()),
            call.method_name,
            call.range.start_line
        );
    }

    // Verify expected method calls
    assert!(
        method_calls.iter().any(|c| c.caller == "validate"
            && c.method_name == "checkAuth"
            && c.receiver.as_deref() == Some("this")),
        "Should find this.checkAuth in validate"
    );

    assert!(
        method_calls.iter().any(|c| c.caller == "process"
            && c.method_name == "transform"
            && c.receiver.as_deref() == Some("data")),
        "Should find data.transform in process"
    );

    assert!(
        method_calls.iter().any(|c| c.caller == "process"
            && c.method_name == "save"
            && c.receiver.as_deref() == Some("data")),
        "Should find data.save in process"
    );

    assert!(
        method_calls.iter().any(|c| c.caller == "standalone"
            && c.method_name == "map"
            && c.receiver.as_deref() == Some("arr")),
        "Should find arr.map in standalone"
    );

    assert!(
        method_calls.iter().any(|c| c.caller == "standalone"
            && c.method_name == "log"
            && c.receiver.as_deref() == Some("console")),
        "Should find console.log in standalone"
    );

    // Check we're not finding regular function calls
    assert!(
        !method_calls
            .iter()
            .any(|c| c.method_name == "regularFunction"),
        "Should NOT find regularFunction"
    );

    assert!(
        !method_calls
            .iter()
            .any(|c| c.method_name == "helperFunction"),
        "Should NOT find helperFunction"
    );

    println!("\n✅ Method call extraction verified");
}

#[test]
fn test_typescript_method_calls_comprehensive() {
    println!("\n=== TypeScript find_method_calls() Comprehensive Test ===\n");

    let code = include_str!("../examples/typescript/comprehensive.ts");
    let mut parser = TypeScriptParser::new().expect("Failed to create parser");

    let method_calls = parser.find_method_calls(code);

    println!(
        "Found {} method calls in comprehensive.ts:",
        method_calls.len()
    );

    // Print first 15 for debugging
    for (i, call) in method_calls.iter().enumerate() {
        if i >= 15 {
            println!("  ... and {} more", method_calls.len() - 15);
            break;
        }
        println!(
            "  In '{}': calls '{}.{}' at line {}",
            call.caller,
            call.receiver.as_ref().unwrap_or(&"<none>".to_string()),
            call.method_name,
            call.range.start_line
        );
    }

    // Verify some specific method calls we expect in comprehensive.ts

    // In formatDate function (line 33), should find date.toISOString()
    assert!(
        method_calls
            .iter()
            .any(|c| c.caller == "formatDate" && c.method_name == "toISOString"),
        "Should find date.toISOString in formatDate"
    );

    // In fetch method (line 192), should find response.json()
    assert!(
        method_calls
            .iter()
            .any(|c| c.caller == "fetch" && c.method_name == "json"),
        "Should find response.json in fetch"
    );

    // In add method (line 211), should find this.items.push()
    assert!(
        method_calls
            .iter()
            .any(|c| c.caller == "add" && c.method_name == "push"),
        "Should find items.push in add"
    );

    println!("\n✅ Comprehensive test passed");
    println!("   Total method calls found: {}", method_calls.len());
}
