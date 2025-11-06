# C# Parser - Remaining Feature Implementation Guide

This document provides detailed implementation guidance for the remaining medium-priority C# parser features.

## Session Summary (2025-11-06)

### ✅ Completed in Previous Sessions
- **Attributes/Annotations Extraction** - Full support with 12 tests
- **Pattern Matching Support** - Type patterns, switch expressions, 6 tests

### ❌ Attempted But Skipped
- **LINQ Query Syntax** - tree-sitter-c-sharp claims support but nodes not generated in practice

### 📝 Ready for Implementation
- Generic Constraints Tracking (3-4 hours)
- Nullable Reference Types (4-5 hours)

---

## Feature 1: Generic Constraints Tracking

### Business Value: Medium
Generic constraints are common in C# codebases and provide important type relationship information.

### Overview
Track `where` clauses on generic types and methods to understand type constraints.

### Examples

```csharp
// Class-level constraints
public class Repository<T> where T : IEntity, new()
{
    // Signature should include: "class Repository<T> where T : IEntity, new()"
}

// Multiple type parameters
public class Cache<TKey, TValue>
    where TKey : notnull
    where TValue : class, ISerializable
{
    // Signature: "class Cache<TKey, TValue> where TKey : notnull where TValue : class, ISerializable"
}

// Method-level constraints
public void Add<U>(U item)
    where U : T, IComparable<U>
{
    // Signature: "void Add<U>(U item) where U : IComparable<U>"
}
```

### Tree-Sitter Node Structure

According to tree-sitter-c-sharp grammar:

```
class_declaration
├── identifier (class name)
├── type_parameter_list (optional)
│   └── type_parameter+
├── type_parameter_constraints_clause* (THIS IS WHAT WE NEED)
│   ├── identifier (type parameter name)
│   └── type_constraint+
│       ├── class_constraint ("class")
│       ├── struct_constraint ("struct")
│       ├── constructor_constraint ("new()")
│       ├── type_constraint (interface/base type)
│       └── notnull_constraint ("notnull")
└── ...
```

### Implementation Steps

#### Step 1: Add Constraint Extraction Helper (30 min)

Add this method to `CSharpParser`:

```rust
/// Extract generic constraints from a node (class or method)
///
/// Looks for type_parameter_constraints_clause children and formats them
/// into a where clause string.
///
/// Returns: " where T : IEntity, new()" or empty string if no constraints
fn extract_generic_constraints(&self, node: Node, code: &str) -> String {
    let mut constraints = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter_constraints_clause" {
            // Structure: type_parameter_constraints_clause has:
            // - identifier (the type parameter, e.g., "T")
            // - one or more type_constraint nodes

            let mut type_param = None;
            let mut constraint_parts = Vec::new();

            let mut constraint_cursor = child.walk();
            for constraint_child in child.children(&mut constraint_cursor) {
                match constraint_child.kind() {
                    "identifier" if type_param.is_none() => {
                        type_param = Some(&code[constraint_child.byte_range()]);
                    }
                    "type_constraint" | "class_constraint" | "struct_constraint"
                    | "constructor_constraint" | "notnull_constraint" => {
                        constraint_parts.push(&code[constraint_child.byte_range()]);
                    }
                    _ => {}
                }
            }

            if let Some(param) = type_param {
                if !constraint_parts.is_empty() {
                    constraints.push(format!(
                        "{} : {}",
                        param,
                        constraint_parts.join(", ")
                    ));
                }
            }
        }
    }

    if constraints.is_empty() {
        String::new()
    } else {
        format!(" where {}", constraints.join(" where "))
    }
}
```

#### Step 2: Update Class Signature Extraction (15 min)

Modify `extract_class_signature()` method (around line 1500):

```rust
fn extract_class_signature(&self, node: Node, code: &str) -> String {
    let mut parts = Vec::new();

    // ... existing code to extract modifiers, keyword, name, type_parameters ...

    // NEW: Add generic constraints
    let constraints = self.extract_generic_constraints(node, code);

    let signature = format!(
        "{}{}{}",
        parts.join(" "),
        base_list,
        constraints  // <-- ADD THIS
    );

    signature
}
```

#### Step 3: Update Method Signature Extraction (15 min)

Modify `extract_method_signature()` method (around line 1600):

```rust
fn extract_method_signature(&self, node: Node, code: &str) -> String {
    // ... existing code to extract return type, name, parameters ...

    // NEW: Add generic constraints
    let constraints = self.extract_generic_constraints(node, code);

    format!(
        "{}{}{}{}",
        return_type,
        name,
        params,
        constraints  // <-- ADD THIS
    );
}
```

#### Step 4: Write Tests (2-2.5 hours)

Add to test module:

```rust
#[test]
fn test_generic_class_with_single_constraint() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Repository<T> where T : IEntity
        {
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let class_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "Repository")
        .expect("Should find Repository class");

    assert!(
        class_symbol.signature.as_ref().unwrap().contains("where T : IEntity"),
        "Signature should include constraint clause. Got: {}",
        class_symbol.signature.as_ref().unwrap()
    );
}

#[test]
fn test_generic_class_with_multiple_constraints() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Repository<T> where T : IEntity, new()
        {
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let class_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "Repository")
        .expect("Should find Repository class");

    let sig = class_symbol.signature.as_ref().unwrap();
    assert!(sig.contains("where T : IEntity"));
    assert!(sig.contains("new()"));
}

#[test]
fn test_generic_class_with_multiple_type_parameters() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Cache<TKey, TValue>
            where TKey : notnull
            where TValue : class, ISerializable
        {
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let class_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "Cache")
        .expect("Should find Cache class");

    let sig = class_symbol.signature.as_ref().unwrap();
    assert!(sig.contains("where TKey : notnull"));
    assert!(sig.contains("where TValue : class, ISerializable"));
}

#[test]
fn test_generic_method_with_constraints() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Repository<T>
        {
            public void Add<U>(U item) where U : T, IComparable<U>
            {
            }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let method_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "Add")
        .expect("Should find Add method");

    let sig = method_symbol.signature.as_ref().unwrap();
    assert!(sig.contains("where U : T, IComparable<U>"));
}

#[test]
fn test_struct_with_constraints() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public struct ValueWrapper<T> where T : struct
        {
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let struct_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "ValueWrapper")
        .expect("Should find ValueWrapper struct");

    assert!(struct_symbol.signature.as_ref().unwrap().contains("where T : struct"));
}

#[test]
fn test_interface_with_constraints() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public interface IRepository<T> where T : IEntity
        {
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let interface_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "IRepository")
        .expect("Should find IRepository interface");

    assert!(interface_symbol.signature.as_ref().unwrap().contains("where T : IEntity"));
}
```

### Key Considerations

1. **Constraint Order**: Preserve the order of constraints as written in source
2. **Complex Types**: Handle generic constraints like `IComparable<T>`
3. **Special Constraints**: Support `class`, `struct`, `new()`, `notnull`, `unmanaged`
4. **Multiple Parameters**: Each type parameter can have its own where clause

---

## Feature 2: Nullable Reference Types (C# 8.0+)

### Business Value: Medium
Modern C# codebases increasingly use nullable reference types for null safety.

### Overview
Track the `?` suffix on reference types to indicate nullability.

### Examples

```csharp
#nullable enable

public class User
{
    public string Name { get; set; }      // Non-nullable
    public string? Email { get; set; }    // Nullable - signature should show "string?"

    public string? GetEmail(string? input)  // Both param and return are nullable
    {
        return input?.Trim();
    }
}
```

### Tree-Sitter Node Structure

```
property_declaration
├── modifier* (public, etc.)
├── nullable_type  <-- THIS IS KEY
│   ├── predefined_type ("string")
│   └── "?" token
├── identifier (property name)
└── ...

method_declaration
├── nullable_type (return type)
│   ├── predefined_type
│   └── "?"
├── identifier (method name)
└── parameter_list
    └── parameter
        ├── nullable_type
        │   ├── predefined_type
        │   └── "?"
        └── identifier
```

### Implementation Steps

#### Step 1: Add Nullable Type Helper (20 min)

```rust
/// Extract type name including nullable marker
///
/// Returns "string?" if the type is nullable, "string" otherwise
fn extract_type_with_nullable(&self, type_node: Node, code: &str) -> String {
    match type_node.kind() {
        "nullable_type" => {
            // nullable_type wraps the actual type and adds "?"
            // Get the inner type and append "?"
            let mut cursor = type_node.walk();
            for child in type_node.children(&mut cursor) {
                if child.kind() != "?" {
                    return format!("{}?", &code[child.byte_range()]);
                }
            }
            // Fallback: just get the whole text
            code[type_node.byte_range()].to_string()
        }
        "predefined_type" | "identifier_name" | "generic_name" | "qualified_name" => {
            // Non-nullable type
            code[type_node.byte_range()].to_string()
        }
        _ => {
            // Unknown type node, get text as-is
            code[type_node.byte_range()].to_string()
        }
    }
}
```

#### Step 2: Update Property Signature (15 min)

Modify `extract_property_signature()`:

```rust
fn extract_property_signature(&self, node: Node, code: &str) -> String {
    let mut type_str = "".to_string();
    let mut name = "";

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "nullable_type" | "predefined_type" | "identifier_name"
            | "generic_name" | "qualified_name" => {
                if type_str.is_empty() {
                    // Use new helper instead of just byte_range
                    type_str = self.extract_type_with_nullable(child, code);
                }
            }
            "identifier" => {
                name = &code[child.byte_range()];
            }
            _ => {}
        }
    }

    format!("{} {}", type_str, name)
}
```

#### Step 3: Update Method Parameter Extraction (20 min)

Modify `extract_parameter_types()`:

```rust
fn extract_parameter_types(&self, params_node: Node, code: &str) -> String {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.children(&mut cursor) {
        if child.kind() == "parameter" {
            let mut param_type = String::new();
            let mut param_name = String::new();

            let mut param_cursor = child.walk();
            for param_child in child.children(&mut param_cursor) {
                match param_child.kind() {
                    "nullable_type" | "predefined_type" | "identifier_name"
                    | "generic_name" | "qualified_name" => {
                        if param_type.is_empty() {
                            // Use new helper
                            param_type = self.extract_type_with_nullable(param_child, code);
                        }
                    }
                    "identifier" => {
                        param_name = code[param_child.byte_range()].to_string();
                    }
                    _ => {}
                }
            }

            if !param_type.is_empty() {
                params.push(format!("{} {}", param_type, param_name));
            }
        }
    }

    format!("({})", params.join(", "))
}
```

#### Step 4: Update Method Return Type (15 min)

Modify `extract_method_signature()`:

```rust
fn extract_method_signature(&self, node: Node, code: &str) -> String {
    let mut return_type = String::new();

    // ... existing code ...

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "nullable_type" | "predefined_type" | "identifier_name"
            | "generic_name" | "qualified_name" | "void_keyword" => {
                if return_type.is_empty() {
                    // Use new helper
                    return_type = self.extract_type_with_nullable(child, code);
                }
            }
            // ... rest of existing code ...
        }
    }

    // ... rest of method ...
}
```

#### Step 5: Write Tests (2-2.5 hours)

```rust
#[test]
fn test_nullable_property() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class User
        {
            public string? Email { get; set; }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let prop_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "Email")
        .expect("Should find Email property");

    assert!(
        prop_symbol.signature.as_ref().unwrap().contains("string?"),
        "Property signature should show nullable type. Got: {}",
        prop_symbol.signature.as_ref().unwrap()
    );
}

#[test]
fn test_non_nullable_property() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class User
        {
            public string Name { get; set; }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let prop_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "Name")
        .expect("Should find Name property");

    let sig = prop_symbol.signature.as_ref().unwrap();
    assert!(sig.contains("string"));
    assert!(!sig.contains("string?"), "Should NOT be nullable");
}

#[test]
fn test_nullable_method_return_type() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class UserService
        {
            public string? FindEmail(int userId)
            {
                return null;
            }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let method_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "FindEmail")
        .expect("Should find FindEmail method");

    assert!(
        method_symbol.signature.as_ref().unwrap().contains("string?"),
        "Method return type should be nullable. Got: {}",
        method_symbol.signature.as_ref().unwrap()
    );
}

#[test]
fn test_nullable_method_parameters() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class UserService
        {
            public string? ProcessEmail(string? input)
            {
                return input?.Trim();
            }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let method_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "ProcessEmail")
        .expect("Should find ProcessEmail method");

    let sig = method_symbol.signature.as_ref().unwrap();
    // Both return type and parameter should be nullable
    assert!(sig.contains("string? ProcessEmail"));
    assert!(sig.contains("string? input"));
}

#[test]
fn test_nullable_generic_types() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class DataService
        {
            public List<string>? GetNames()
            {
                return null;
            }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let method_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "GetNames")
        .expect("Should find GetNames method");

    assert!(
        method_symbol.signature.as_ref().unwrap().contains("List<string>?"),
        "Generic type should be nullable. Got: {}",
        method_symbol.signature.as_ref().unwrap()
    );
}

#[test]
fn test_mixed_nullable_and_non_nullable_parameters() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class UserService
        {
            public void Update(int id, string name, string? email)
            {
            }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let method_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "Update")
        .expect("Should find Update method");

    let sig = method_symbol.signature.as_ref().unwrap();
    // id is int (value type, can't be nullable in this context)
    assert!(sig.contains("int id"));
    // name is non-nullable reference type
    assert!(sig.contains("string name"));
    // email is nullable reference type
    assert!(sig.contains("string? email"));
}

#[test]
fn test_nullable_field() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class User
        {
            private string? _cachedEmail;
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let field_symbol = symbols.iter()
        .find(|s| s.name.as_ref() == "_cachedEmail")
        .expect("Should find _cachedEmail field");

    assert!(
        field_symbol.signature.as_ref().unwrap().contains("string?"),
        "Field signature should show nullable type"
    );
}
```

### Key Considerations

1. **Value Types**: Only reference types can be nullable (except `Nullable<T>` / `T?` for value types)
2. **Generic Types**: Support `List<string>?` (nullable collection of non-nullable strings)
3. **Nested Nullability**: `List<string?>?` (nullable collection of nullable strings)
4. **Array Types**: `string[]?` (nullable array) vs `string?[]` (array of nullable strings)

---

## Testing Strategy

### Before Running Tests

```bash
# Format code
cargo fmt

# Check for errors
cargo check

# Check for warnings
cargo clippy
```

### Running Tests

```bash
# Run all C# parser tests
cargo test csharp --lib

# Run specific test
cargo test test_generic_class_with_single_constraint --lib

# Run with output
cargo test test_nullable_property --lib -- --nocapture
```

### Test Coverage Goals

- **Generic Constraints**: 6-7 tests covering all constraint types
- **Nullable References**: 8-9 tests covering all nullable scenarios
- **Total**: ~15 new tests

---

## Implementation Time Estimates

### Generic Constraints (3-4 hours)
- Helper method: 30 min
- Update class signatures: 15 min
- Update method signatures: 15 min
- Write tests: 2-2.5 hours
- Debug and fixes: 30-60 min

### Nullable Reference Types (4-5 hours)
- Helper method: 20 min
- Update property signatures: 15 min
- Update parameter extraction: 20 min
- Update method signatures: 15 min
- Write tests: 2-2.5 hours
- Debug and fixes: 1-1.5 hours

### Total: 7-9 hours for both features

---

## Commit Message Template

```
feat(csharp): add generic constraints and nullable reference type tracking

Implements two medium-priority features for C# parser:

1. Generic Constraints Tracking
   - Extracts where clauses from type and method declarations
   - Supports all constraint types: class, struct, new(), interfaces
   - Includes constraints in symbol signatures

2. Nullable Reference Types (C# 8.0+)
   - Detects nullable_type nodes in AST
   - Preserves '?' marker in type signatures
   - Supports nullable properties, parameters, return types, and fields

Examples:
- class Repository<T> where T : IEntity, new()
- public string? GetEmail(string? input)

Tests: 15 comprehensive tests covering all scenarios
- 7 tests for generic constraints
- 8 tests for nullable reference types

All tests passing. Code formatted with cargo fmt and checked with clippy.
```

---

## Known Limitations

### Generic Constraints
- Does not validate constraint correctness (parser's job is extraction, not validation)
- Complex constraint expressions may need refinement based on real-world codebases

### Nullable Reference Types
- Does not track `#nullable enable/disable` directives (would require state tracking)
- Assumes nullable annotations are explicit in code
- Does not infer nullability from context

---

## Future Enhancements (Lower Priority)

After implementing these two features, consider:

1. **Primary Constructors** (C# 12) - 2-3 hours
2. **Enhanced Records Support** - 3-4 hours
3. **File-scoped Types** (C# 11) - 1-2 hours
4. **Extension Methods Detection** - 4-5 hours
5. **Operator Overloading Tracking** - 2-3 hours

---

## Session Notes

- LINQ query syntax was attempted but tree-sitter-c-sharp doesn't generate the expected nodes
- Token usage reached 114k/200k when documenting these features
- Pragmatic approach: document thoroughly for next session rather than rush incomplete implementation
- Key lesson: Verify tree-sitter node generation with simple tests before full implementation

---

*Document created: 2025-11-06*
*For implementation questions, refer to tree-sitter-c-sharp grammar: https://github.com/tree-sitter/tree-sitter-c-sharp*
