# C# Parser Testing Guide

Complete guide for writing tests for the C# parser.

---

## Table of Contents

1. [Test Organization](#test-organization)
2. [Test Categories](#test-categories)
3. [Writing Tests](#writing-tests)
4. [Running Tests](#running-tests)
5. [Best Practices](#best-practices)
6. [Common Patterns](#common-patterns)
7. [Examples](#examples)

---

## Test Organization

### Test Locations

```
tests/parsers/csharp/
└── test_parser.rs          # Main test file (1,212 lines, 53 tests)

src/parsing/csharp/
└── parser.rs               # Additional unit tests (inline #[test] functions)

examples/csharp/
├── comprehensive.cs        # Main test file (636 lines)
├── RelationshipTest.cs     # Relationship testing (488 lines)
└── file_scoped_namespace.cs # Specific feature test (31 lines)
```

### Test File Structure

```rust
// tests/parsers/csharp/test_parser.rs

use codanna::parsing::LanguageParser;
use codanna::parsing::csharp::CSharpParser;
use codanna::types::{FileId, SymbolCounter};

// Test groups:
// 1. Basic symbol extraction
// 2. Relationship tracking
// 3. Advanced features (operators, extensions, async, patterns)
// 4. Edge cases

#[test]
fn test_feature_name() {
    // Test implementation
}
```

---

## Test Categories

### 1. Symbol Extraction Tests

Test that symbols are correctly extracted from C# code.

**What to test:**
- Classes, interfaces, structs, records, enums
- Methods, constructors, properties, fields, events
- Delegates, indexers, operators
- Nested types, partial classes

**Example:**
```rust
#[test]
fn test_class_extraction() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class UserService {
            public void ValidateUser(string username) { }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Assertions
    assert!(symbols.iter().any(|s| s.name == "UserService"));
    assert!(symbols.iter().any(|s| s.name == "ValidateUser"));
}
```

### 2. Relationship Tracking Tests

Test that relationships between symbols are correctly tracked.

**What to test:**
- Method calls (with proper caller context)
- Interface implementations
- Using directives
- Variable type bindings

**Example:**
```rust
#[test]
fn test_method_call_tracking() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Calculator {
            private int Add(int a, int b) { return a + b; }

            public int Calculate() {
                return Add(5, 10);
            }
        }
    "#;

    let calls = parser.find_calls(code);

    // Should find Calculate → Add
    assert!(
        calls.iter().any(|(from, to, _)| *from == "Calculate" && *to == "Add"),
        "Should detect Calculate -> Add call"
    );
}
```

### 3. Feature-Specific Tests

Test specific C# features.

**Categories:**
- Operator overloading (arithmetic, comparison, unary)
- Extension methods (basic, generics, complex types)
- Async/await (signatures, interface, expressions)
- Pattern matching (declaration, type, discard, property)
- Attributes (basic, with arguments, named arguments)
- XML documentation comments

**Example:**
```rust
#[test]
fn test_extension_method_detection() {
    let code = r#"
        public static class StringExtensions {
            public static bool IsEmpty(this string str) {
                return string.IsNullOrEmpty(str);
            }
        }
    "#;

    let symbols = parser.parse(code, file_id, &mut counter);

    // Extension methods marked with [ext:Type]
    let is_empty = symbols.iter()
        .find(|s| s.name.starts_with("IsEmpty") && s.name.contains("[ext:"));

    assert!(is_empty.is_some());
    assert!(is_empty.unwrap().name.contains("[ext:string]"));
}
```

### 4. Edge Case Tests

Test unusual or problematic scenarios.

**What to test:**
- Empty files
- Files with only comments
- Malformed code
- Deeply nested structures
- Very long identifiers
- Unicode identifiers
- Incomplete syntax

**Example:**
```rust
#[test]
fn test_empty_file() {
    let code = "";
    let symbols = parser.parse(code, file_id, &mut counter);
    assert!(symbols.is_empty());
}

#[test]
fn test_malformed_code() {
    let code = "class Foo { void Bar() /* missing closing brace */";
    let symbols = parser.parse(code, file_id, &mut counter);
    // Should not panic, should return partial results
    assert!(!symbols.is_empty() || symbols.is_empty()); // Either is valid
}
```

---

## Writing Tests

### Test Structure Template

```rust
#[test]
fn test_your_feature() {
    // 1. Setup
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        // Your C# code here
    "#;

    // 2. Execute
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // 3. Assert
    assert!(symbols.iter().any(|s| s.name == "ExpectedSymbol"));

    // 4. Detailed assertions (optional)
    let symbol = symbols.iter().find(|s| s.name == "ExpectedSymbol").unwrap();
    assert_eq!(symbol.kind, SymbolKind::Class);
    assert!(symbol.signature.is_some());
}
```

### When to Add Unit vs Integration Tests

**Unit Tests** (inline in `parser.rs`):
- Testing private helper functions
- Testing specific logic in isolation
- Quick smoke tests

```rust
// In src/parsing/csharp/parser.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_type_name() {
        // Test internal function
    }
}
```

**Integration Tests** (in `tests/parsers/csharp/`):
- Testing public API
- Testing end-to-end parsing
- Testing complex scenarios
- Testing relationships
- Testing feature interactions

```rust
// In tests/parsers/csharp/test_parser.rs
#[test]
fn test_complete_feature() {
    // Test full parsing workflow
}
```

---

## Running Tests

### Run All C# Parser Tests

```bash
# Run all tests (but will hit network error for other features)
cargo test --lib csharp

# Run specific test
cargo test --lib test_csharp_extension_methods_basic

# Run integration tests
cargo test --test parsers_tests csharp

# Run with output
cargo test --lib csharp -- --nocapture

# Run ignored tests
cargo test --lib csharp -- --ignored

# Run specific test file
cargo test --test parsers_tests --features default -- csharp --nocapture
```

### Run Formatting and Linting

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy --all-targets --all-features

# Run all checks
cargo test --lib csharp && cargo clippy && cargo fmt --check
```

---

## Best Practices

### 1. Use Descriptive Test Names

```rust
// ✅ Good
#[test]
fn test_extension_method_with_generic_type_parameter() { }

// ❌ Bad
#[test]
fn test_ext1() { }
```

### 2. Test One Thing Per Test

```rust
// ✅ Good - focused test
#[test]
fn test_async_method_has_async_in_signature() {
    let code = "public async Task GetDataAsync() { }";
    let symbols = parser.parse(code, file_id, &mut counter);
    let method = symbols.iter().find(|s| s.name == "GetDataAsync").unwrap();
    assert!(method.signature.unwrap().contains("async"));
}

// ❌ Bad - testing multiple things
#[test]
fn test_async_stuff() {
    // Tests async signature, async return type, async calls, etc.
    // Hard to debug when it fails
}
```

### 3. Include Descriptive Failure Messages

```rust
// ✅ Good
assert!(
    symbols.iter().any(|s| s.name == "ExpectedClass"),
    "Should find ExpectedClass. Found symbols: {:?}",
    symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
);

// ❌ Bad
assert!(symbols.iter().any(|s| s.name == "ExpectedClass"));
```

### 4. Use Example Files for Complex Scenarios

```rust
#[test]
fn test_comprehensive_parsing() {
    let code = std::fs::read_to_string("examples/csharp/comprehensive.cs").unwrap();
    let symbols = parser.parse(&code, file_id, &mut counter);

    // Test various aspects
    assert!(symbols.len() > 50);  // Should extract many symbols
    assert!(symbols.iter().any(|s| s.kind == SymbolKind::Class));
    assert!(symbols.iter().any(|s| s.kind == SymbolKind::Method));
}
```

### 5. Test Both Positive and Negative Cases

```rust
#[test]
fn test_extension_method_requires_static_class() {
    let code = r#"
        // ❌ Not an extension - instance method
        public class NonStaticClass {
            public void NotExtension(this string str) { }
        }

        // ✅ Extension method - static class + static method + this param
        public static class ProperExtensions {
            public static void RealExtension(this string str) { }
        }
    "#;

    let symbols = parser.parse(code, file_id, &mut counter);

    // NotExtension should NOT be marked as extension
    let not_ext = symbols.iter().find(|s| s.name == "NotExtension").unwrap();
    assert!(!not_ext.name.contains("[ext:"));

    // RealExtension SHOULD be marked
    let real_ext = symbols.iter()
        .find(|s| s.name.starts_with("RealExtension") && s.name.contains("[ext:"))
        .unwrap();
    assert!(real_ext.name.contains("[ext:string]"));
}
```

---

## Common Patterns

### Pattern 1: Find Symbol by Name

```rust
let symbol = symbols.iter()
    .find(|s| s.name == "SymbolName")
    .expect("Should find SymbolName");
```

### Pattern 2: Check Symbol Kind

```rust
assert_eq!(symbol.kind, SymbolKind::Class);
assert_eq!(symbol.kind, SymbolKind::Method);
```

### Pattern 3: Verify Documentation

```rust
if let Some(doc) = &symbol.doc_comment {
    assert!(doc.contains("<summary>"));
    assert!(doc.contains("expected text"));
}
```

### Pattern 4: Check Signature

```rust
let sig = symbol.signature.as_ref().unwrap();
assert!(sig.contains("public"));
assert!(sig.contains("async"));
assert!(sig.contains("Task<string>"));
```

### Pattern 5: Verify Relationship

```rust
// Method calls
assert!(
    calls.iter().any(|(from, to, _)| *from == "Caller" && *to == "Callee")
);

// Implementations
assert!(
    implementations.iter().any(|(class, interface, _)|
        *class == "MyClass" && *interface == "IMyInterface"
    )
);
```

### Pattern 6: Count Symbols

```rust
let method_count = symbols.iter()
    .filter(|s| s.kind == SymbolKind::Method)
    .count();
assert_eq!(method_count, 5);
```

---

## Examples

### Example 1: Testing a New Feature (Operator Overloading)

```rust
#[test]
fn test_csharp_operator_overloading_arithmetic() {
    let code = r#"
        public class Vector {
            public static Vector operator +(Vector a, Vector b) {
                return new Vector { X = a.X + b.X, Y = a.Y + b.Y };
            }

            public static Vector operator -(Vector a, Vector b) {
                return new Vector { X = a.X - b.X, Y = a.Y - b.Y };
            }
        }
    "#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Check operator+ exists
    let plus_op = symbols.iter().find(|s| &*s.name == "operator+");
    assert!(
        plus_op.is_some(),
        "Should detect operator+ overload. Found: {:?}",
        symbols.iter().map(|s| &*s.name).collect::<Vec<_>>()
    );

    // Verify signature
    let sig = plus_op.unwrap().signature.as_ref().unwrap();
    assert!(sig.contains("operator"));
    assert!(sig.contains("+"));

    // Check operator- exists
    assert!(symbols.iter().any(|s| &*s.name == "operator-"));
}
```

### Example 2: Testing Relationships (Method Calls)

```rust
#[test]
fn test_method_calls_with_proper_context() {
    let code = r#"
        public class Service {
            public void Process() {
                Validate();
                Transform();
                Save();
            }

            private void Validate() { }
            private void Transform() { }
            private void Save() { }
        }
    "#;

    let mut parser = CSharpParser::new().unwrap();
    let calls = parser.find_calls(code);

    // All three calls should have "Process" as caller
    assert!(calls.iter().any(|(from, to, _)|
        *from == "Process" && *to == "Validate"
    ));
    assert!(calls.iter().any(|(from, to, _)|
        *from == "Process" && *to == "Transform"
    ));
    assert!(calls.iter().any(|(from, to, _)|
        *from == "Process" && *to == "Save"
    ));
}
```

### Example 3: Testing Documentation Extraction

```rust
#[test]
fn test_multiline_xml_documentation() {
    let code = r#"
        /// <summary>
        /// This is a multi-line
        /// XML documentation comment
        /// </summary>
        /// <param name="username">The username</param>
        /// <returns>True if valid</returns>
        public bool ValidateUser(string username) {
            return true;
        }
    "#;

    let mut parser = CSharpParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let method = symbols.iter().find(|s| &*s.name == "ValidateUser").unwrap();
    let doc = method.doc_comment.as_ref().unwrap();

    // Verify all XML tags are captured
    assert!(doc.contains("<summary>"));
    assert!(doc.contains("multi-line"));
    assert!(doc.contains("</summary>"));
    assert!(doc.contains("<param"));
    assert!(doc.contains("username"));
    assert!(doc.contains("<returns>"));
}
```

### Example 4: Testing Edge Cases

```rust
#[test]
fn test_deeply_nested_namespaces() {
    let code = r#"
        namespace Level1.Level2.Level3.Level4 {
            public class DeepClass { }
        }
    "#;

    let symbols = parser.parse(code, file_id, &mut counter);
    let class = symbols.iter().find(|s| &*s.name == "DeepClass").unwrap();

    assert_eq!(class.module_path.as_deref(), Some("Level1.Level2.Level3.Level4"));
}

#[test]
fn test_unicode_identifiers() {
    let code = r#"
        public class Café {
            public void Naïve() { }
        }
    "#;

    let symbols = parser.parse(code, file_id, &mut counter);
    assert!(symbols.iter().any(|s| &*s.name == "Café"));
    assert!(symbols.iter().any(|s| &*s.name == "Naïve"));
}
```

---

## Adding New Tests

### Checklist for New Tests

- [ ] Descriptive test name
- [ ] Clear test structure (setup, execute, assert)
- [ ] Helpful failure messages
- [ ] Tests positive and negative cases
- [ ] Includes doc comments if testing complex feature
- [ ] Runs successfully: `cargo test --lib test_name`
- [ ] Formatted: `cargo fmt`
- [ ] No clippy warnings: `cargo clippy`

### Template for New Test

```rust
/// Test that [feature] is correctly [action]
///
/// This test verifies that the parser [detailed description].
///
/// Test cases:
/// - [Case 1 description]
/// - [Case 2 description]
#[test]
fn test_your_new_feature() {
    // Setup
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        // Your C# code example
    "#;

    // Execute
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Assert - positive cases
    assert!(
        symbols.iter().any(|s| s.name == "Expected"),
        "Should find Expected symbol. Found: {:?}",
        symbols.iter().map(|s| &*s.name).collect::<Vec<_>>()
    );

    // Assert - negative cases
    assert!(
        !symbols.iter().any(|s| s.name == "ShouldNotExist"),
        "Should NOT find ShouldNotExist"
    );

    // Assert - detailed checks
    let symbol = symbols.iter().find(|s| s.name == "Expected").unwrap();
    assert_eq!(symbol.kind, SymbolKind::YourKind);
    if let Some(sig) = &symbol.signature {
        assert!(sig.contains("expected text"));
    }
}
```

---

## Debugging Failed Tests

### Steps to Debug

1. **Read the failure message**
   ```
   Should find ExpectedClass. Found symbols: ["ActualClass1", "ActualClass2"]
   ```

2. **Run test with output**
   ```bash
   cargo test --lib test_name -- --nocapture
   ```

3. **Add debug prints**
   ```rust
   println!("Symbols: {:#?}", symbols);
   println!("Calls: {:#?}", calls);
   ```

4. **Inspect AST structure**
   ```rust
   #[test]
   #[ignore]
   fn debug_ast_structure() {
       let code = "your problematic code";
       let tree = parser.parser.parse(code, None).unwrap();
       print_tree(&tree.root_node(), code, 0);
   }

   fn print_tree(node: &Node, code: &str, depth: usize) {
       let indent = "  ".repeat(depth);
       println!("{indent}{}: {}", node.kind(), &code[node.byte_range()]);
       for child in node.children(&mut node.walk()) {
           print_tree(&child, code, depth + 1);
       }
   }
   ```

5. **Check tree-sitter grammar**
   - Refer to [node-types.json](../node-types.json)
   - Verify expected AST structure

---

## Related Documentation

- [Architecture Guide](./ARCHITECTURE.md) - Understand parser design
- [User Documentation](../../../docs/parsers/csharp.md) - Public API reference
- [Action Plan](../../../CSHARP_ACTION_PLAN.md) - Implementation roadmap

---

**Questions?**
- Open an issue: [GitHub Issues](https://github.com/bartolli/codanna/issues)
- Ask in discussions: [GitHub Discussions](https://github.com/bartolli/codanna/discussions)

---

**Last Updated:** 2025-11-08
**Test Count:** 53 passing tests
**Coverage:** Excellent (all major features tested)
