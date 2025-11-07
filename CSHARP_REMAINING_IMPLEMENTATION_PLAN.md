# C# Parser - Complete Remaining Implementation Plan

**Document Date:** 2025-11-07
**Session ID:** claude/csharp-linq-query-syntax-011CUs3A3tkdUYzZbbmUEgrx
**Last Updated:** 2025-11-07 (Session 2)

---

## 📊 Current Status Summary

### ✅ Completed Features (6/9 from original plan)

1. **Attributes/Annotations Extraction** ✅ (Previous session)
   - Full support with 12 tests
   - Commit: `feat(csharp): add comprehensive attribute/annotation extraction support`

2. **Pattern Matching Support** ✅ (Previous session)
   - Type patterns, switch expressions
   - 6 comprehensive tests
   - Commit: `feat(csharp): add pattern matching and LINQ query syntax support`

3. **Generic Constraints Tracking** ✅ (Session 1 - 2025-11-07)
   - Where clauses on types and methods
   - 6 comprehensive tests
   - Commit: `feat(csharp): add generic constraints and nullable reference type tracking`

4. **Nullable Reference Types** ✅ (Session 1 - 2025-11-07)
   - Nullable markers preserved in signatures
   - 8 comprehensive tests
   - Commit: `feat(csharp): add generic constraints and nullable reference type tracking`

5. **Using Directives / Imports** ✅ (Session 2 - 2025-11-07)
   - Basic using directives (using System;)
   - Static usings (using static System.Math;)
   - Global usings (global using System;)
   - Using aliases (using Json = System.Text.Json;)
   - 5 comprehensive tests
   - Commit: `feat(csharp): add enhanced using directives and file-scoped types support`

6. **File-scoped Types (C# 11)** ✅ (Session 2 - 2025-11-07)
   - File-scoped classes, interfaces, structs, records
   - Proper visibility mapping (Private)
   - Signature includes "file" modifier
   - 5 comprehensive tests
   - Commit: `feat(csharp): add enhanced using directives and file-scoped types support`

### ❌ Attempted But Failed

- **LINQ Query Syntax Support** ❌
  - Tree-sitter-c-sharp v0.23.1 claims support but doesn't generate expected AST nodes
  - Attempted in previous session, documented in SESSION_SUMMARY_2025-11-06.md
  - **Recommendation:** Skip until tree-sitter-c-sharp is updated

---

## 🎯 Remaining Features - Detailed Implementation Plan

### Priority 1: Medium Priority Features (Worth Implementing)

---

#### Feature 1: Primary Constructors (C# 12)

**Estimated Effort:** 2-3 hours
**Business Value:** Medium (C# 12 feature, growing adoption)
**Complexity:** Low-Medium

##### What to Implement

```csharp
// Basic primary constructor
public class Person(string firstName, string lastName, int age)
{
    // Parameters are available throughout the class
    public string FullName => $"{firstName} {lastName}";
    public bool IsAdult => age >= 18;
}

// With base class
public class Employee(string name, int id, decimal salary)
    : Person(name, "", 0)
{
    public decimal AnnualSalary => salary * 12;
}

// With records (enhanced)
public record struct Point(int X, int Y);
```

##### Implementation Steps

**Phase 1: Detect Primary Constructors (45 min)**
- Look for parameter lists directly on class/struct/record declarations
- Tree-sitter node: `class_declaration` with `parameter_list` child
- Extract parameter names and types

**Phase 2: Symbol Extraction (45 min)**
- Create constructor symbol with parameters
- Track parameters as available throughout class scope
- Signature: `Person(string firstName, string lastName, int age)`

**Phase 3: Parameter Availability Tracking (30 min)**
- Primary constructor parameters are fields/properties
- Add to scope context for the entire class
- Support method body references to parameters

**Phase 4: Testing (30-45 min)**
- Test basic primary constructor
- Test with inheritance
- Test with records
- Test parameter usage in methods/properties

##### Tree-sitter Investigation Needed

```bash
# Test if tree-sitter parses primary constructors
echo 'public class Person(string name, int age) {}' | tree-sitter parse --language c-sharp
```

Expected node structure:
```
class_declaration
├── identifier ("Person")
├── parameter_list ← PRIMARY CONSTRUCTOR
│   ├── parameter (string name)
│   └── parameter (int age)
└── declaration_list
```

##### Test Cases

```rust
#[test]
fn test_primary_constructor_basic() {
    let code = r#"
        public class Person(string firstName, string lastName)
        {
            public string FullName => $"{firstName} {lastName}";
        }
    "#;

    // Should extract:
    // - Person class
    // - Constructor with signature: Person(string firstName, string lastName)
    // - FullName property
}

#[test]
fn test_primary_constructor_with_base() {
    let code = r#"
        public class Employee(string name, int id) : Person(name)
        {
            public int EmployeeId => id;
        }
    "#;
    // Should track base class call
}
```

##### Files to Modify

- `src/parsing/csharp/parser.rs`:
  - Add `has_primary_constructor()` check in `process_class()`
  - Add `extract_primary_constructor_params()` helper
  - Update class signature to include primary constructor parameters

---

#### Feature 2: Enhanced Records Support

**Estimated Effort:** 3-4 hours
**Business Value:** Medium (records increasingly common in modern C#)
**Complexity:** Medium

##### What to Implement

```csharp
// Positional records (primary focus)
public record Point(int X, int Y);
public record Person(string FirstName, string LastName, int Age);

// Record structs
public record struct Temperature(double Celsius)
{
    public double Fahrenheit => Celsius * 9/5 + 32;
}

// With expressions (track immutable updates)
var original = new Point(1, 2);
var updated = original with { X = 10 };

// Inheritance
public record Employee(string Name, int Id) : Person(Name, "", 0);
```

##### Current State

- Basic record declarations already work (treated as classes)
- Need to enhance: positional parameters, with expressions, record structs

##### Implementation Steps

**Phase 1: Positional Record Parameters (1h)**
- Detect parameter_list on record_declaration
- Extract positional parameters as properties
- Generate property symbols for each parameter

**Phase 2: Record Structs (45 min)**
- Already handled as structs, verify signature includes "record struct"
- Test struct-specific behavior

**Phase 3: With Expressions (Optional) (1h)**
- Detect `with_expression` nodes
- Track as a special kind of object creation
- Low priority - mainly for completeness

**Phase 4: Testing (45-60 min)**
- Test positional records
- Test record structs
- Test inheritance
- Test with expressions (if implemented)

##### Tree-sitter Investigation

```bash
echo 'public record Point(int X, int Y);' | tree-sitter parse --language c-sharp
```

Expected structure:
```
record_declaration
├── identifier ("Point")
├── parameter_list ← POSITIONAL PARAMETERS
│   ├── parameter (int X)
│   └── parameter (int Y)
└── ;
```

##### Test Cases

```rust
#[test]
fn test_positional_record() {
    let code = r#"
        public record Point(int X, int Y);
    "#;

    // Should extract:
    // - Point record class
    // - X property (int)
    // - Y property (int)
}

#[test]
fn test_record_struct() {
    let code = r#"
        public record struct Temperature(double Celsius);
    "#;

    // Should extract as struct with property
}
```

---

#### Feature 3: File-scoped Types (C# 11)

**Estimated Effort:** 1-2 hours
**Business Value:** Low-Medium (niche feature but easy to implement)
**Complexity:** Low

##### What to Implement

```csharp
// File-scoped class (only visible in current file)
file class InternalHelper
{
    public void Utility() { }
}

// File-scoped interface
file interface IInternalService
{
    void Process();
}
```

##### Implementation Steps

**Phase 1: Detect File Modifier (30 min)**
- Check for "file" modifier on class/interface/struct declarations
- Tree-sitter: modifier node with "file" text

**Phase 2: Update Visibility (15 min)**
- Add new visibility level: `Visibility::File` (may need to check if exists)
- Or use `Visibility::Private` with metadata flag

**Phase 3: Extract File-scoped Types (30 min)**
- Process normally but mark with file-scoped visibility
- Include "file" in signature

**Phase 4: Testing (30 min)**
- Test file-scoped class
- Test file-scoped interface
- Test file-scoped struct

##### Tree-sitter Check

```bash
echo 'file class Helper {}' | tree-sitter parse --language c-sharp
```

##### Test Cases

```rust
#[test]
fn test_file_scoped_class() {
    let code = r#"
        file class InternalHelper
        {
            public void Process() { }
        }
    "#;

    // Should extract with file-level visibility
    // Signature should include "file class InternalHelper"
}
```

---

### Priority 2: Lower Priority Features (Optional Enhancements)

---

#### Feature 4: Extension Methods

**Estimated Effort:** 4-5 hours
**Business Value:** Medium (helps understand utility patterns)
**Complexity:** Medium-High

##### What to Implement

```csharp
public static class StringExtensions
{
    // Extension method (first param with 'this')
    public static bool IsEmpty(this string str)
    {
        return string.IsNullOrEmpty(str);
    }

    public static T Parse<T>(this string str) where T : IParseable<T>
    {
        return T.Parse(str);
    }
}

// Usage tracking
string name = "test";
bool empty = name.IsEmpty(); // Should track IsEmpty extends string
```

##### Implementation Steps

**Phase 1: Detect Extension Methods (1.5h)**
- Check for static methods in static classes
- First parameter must have `this` modifier
- Tree-sitter: Look for "this" modifier on first parameter

**Phase 2: Track Extended Type (1h)**
- Extract the type being extended (first parameter type)
- Store in symbol metadata: `extends_type: Option<String>`

**Phase 3: Track Extension Usage (1.5h)**
- When finding method calls, check if receiver type matches extensions
- Create relationships: string → IsEmpty (extends)

**Phase 4: Testing (1h)**
- Test detection of extension methods
- Test generic extension methods
- Test extension method calls

##### Data Structure Addition

May need to add to Symbol:
```rust
pub struct Symbol {
    // ... existing fields
    pub extends_type: Option<Rc<str>>, // For extension methods: "string", "IEnumerable<T>"
}
```

##### Test Cases

```rust
#[test]
fn test_extension_method_detection() {
    let code = r#"
        public static class Extensions
        {
            public static bool IsEmpty(this string str)
            {
                return string.IsNullOrEmpty(str);
            }
        }
    "#;

    // Should extract IsEmpty with:
    // - kind: Method
    // - extends_type: Some("string")
    // - signature includes "this string str"
}
```

---

#### Feature 5: Operator Overloading

**Estimated Effort:** 2-3 hours
**Business Value:** Low (niche feature)
**Complexity:** Low-Medium

##### What to Implement

```csharp
public class Vector
{
    public int X { get; set; }
    public int Y { get; set; }

    // Operator overloads
    public static Vector operator +(Vector a, Vector b)
    {
        return new Vector { X = a.X + b.X, Y = a.Y + b.Y };
    }

    public static bool operator ==(Vector a, Vector b)
    {
        return a.X == b.X && a.Y == b.Y;
    }

    public static bool operator !=(Vector a, Vector b)
    {
        return !(a == b);
    }
}
```

##### Implementation Steps

**Phase 1: Detect Operator Methods (1h)**
- Look for methods with "operator" keyword
- Extract operator symbol (+, -, ==, !=, etc.)
- Tree-sitter node: `operator_declaration`

**Phase 2: Symbol Extraction (45 min)**
- Create method symbol with special naming: "operator+"
- Include full signature with operator keyword
- Track as special kind of method

**Phase 3: Testing (45 min)**
- Test arithmetic operators (+, -, *, /)
- Test comparison operators (==, !=, <, >)
- Test conversion operators (implicit, explicit)

##### Test Cases

```rust
#[test]
fn test_operator_overload() {
    let code = r#"
        public class Vector
        {
            public static Vector operator +(Vector a, Vector b)
            {
                return new Vector();
            }
        }
    "#;

    // Should extract:
    // - Method named "operator+"
    // - Signature: "static Vector operator +(Vector a, Vector b)"
}
```

---

#### Feature 6: Async/Await Tracking

**Estimated Effort:** 2-3 hours
**Business Value:** Low (mostly syntactic)
**Complexity:** Low

##### What to Implement

```csharp
public class Service
{
    // Async method
    public async Task<string> GetDataAsync()
    {
        await Task.Delay(100);
        return "data";
    }

    // Async void (event handlers)
    public async void HandleClick(object sender, EventArgs e)
    {
        await ProcessAsync();
    }
}
```

##### Implementation Steps

**Phase 1: Detect Async Modifier (1h)**
- Check for "async" modifier on methods
- Track in signature

**Phase 2: Track Task Return Types (30 min)**
- Already captured in signatures
- Optionally: unwrap Task<T> to T for type tracking

**Phase 3: Track Await Expressions (Optional) (1h)**
- Detect `await_expression` nodes
- Track as special kind of call/expression

**Phase 4: Testing (30 min)**
- Test async methods with Task<T>
- Test async void
- Test await expressions

---

#### Feature 7: Improved Documentation Extraction

**Estimated Effort:** 3-4 hours
**Business Value:** Medium (better doc extraction)
**Complexity:** Medium

##### Current State

Basic XML doc comments already extracted (///)

##### What to Enhance

```csharp
/// <summary>
/// Calculates the sum of two numbers
/// </summary>
/// <param name="a">First number</param>
/// <param name="b">Second number</param>
/// <returns>The sum of a and b</returns>
/// <exception cref="OverflowException">Thrown when overflow occurs</exception>
public int Add(int a, int b)
{
    return a + b;
}
```

##### Implementation Steps

**Phase 1: Parse XML Tags (2h)**
- Extract text, parse as XML (or simple tag extraction)
- Extract: summary, param, returns, exception, remarks

**Phase 2: Structured Doc Storage (1h)**
- Create DocComment struct with fields
- Store parsed tags separately

**Phase 3: Testing (1h)**
- Test various XML doc tags
- Test multiline documentation

##### Data Structure

```rust
pub struct ParsedDocComment {
    pub summary: Option<String>,
    pub params: Vec<(String, String)>, // (param_name, description)
    pub returns: Option<String>,
    pub exceptions: Vec<(String, String)>, // (exception_type, description)
    pub remarks: Option<String>,
}
```

---

#### Feature 8: Complete Using Directives / Imports

**Estimated Effort:** 2-3 hours
**Business Value:** Low (basic version exists but commented as incomplete)
**Complexity:** Low-Medium

##### Current State

`find_imports()` exists but test is marked as `#[ignore]` with note: "find_imports implementation needs to be completed - currently returns empty"

Looking at code (line 1921-1935), the implementation actually exists and should work!

##### What's Needed

**Phase 1: Verify Current Implementation (30 min)**
- Un-ignore the test
- Run the test
- If it fails, debug why

**Phase 2: Fix Issues (1h)**
- The existing implementation looks correct
- May just need minor fixes

**Phase 3: Enhanced Import Tracking (1h)**
- Track static usings: `using static System.Math;`
- Track global usings (C# 10): `global using System;`
- Track alias usings: `using Json = System.Text.Json;`

**Phase 4: Testing (30 min)**
- Test basic using directives
- Test static usings
- Test global usings
- Test using aliases

##### Quick Fix (Highest ROI)

This is actually the easiest win - un-ignore the test and see if it works!

```rust
#[test]
// Remove this line: #[ignore = "find_imports implementation needs to be completed - currently returns empty"]
fn test_csharp_using_directive_extraction() {
    // ... existing test code
}
```

---

## 📋 Recommended Implementation Order

### ✅ Session 1 (Quick Wins - COMPLETED)

**Completed Features:**

1. ✅ **Using Directives / Imports** (~2 hours)
   - Un-ignored test and enhanced implementation
   - Added static, global, and alias using support
   - 5 comprehensive tests

2. ✅ **File-scoped Types** (~1-2 hours)
   - Added file modifier detection
   - Proper visibility mapping and signature tracking
   - 5 comprehensive tests

**Results:**
- Both features implemented successfully
- All tests passing
- Quality checks clean (cargo test, check, fmt, clippy)
- Commit: `feat(csharp): add enhanced using directives and file-scoped types support`

---

### 🔲 Session 2 (Modern C# Features - 5-7 hours) - NEXT RECOMMENDED

1. **Primary Constructors** (2-3 hours)
   - C# 12 feature
   - Growing adoption
   - Moderate complexity

2. **Enhanced Records Support** (3-4 hours)
   - Builds on primary constructors knowledge
   - Modern C# pattern
   - Widely used

---

### Session 3 (Utility Features - 6-8 hours)

1. **Extension Methods** (4-5 hours)
   - Very useful for understanding codebases
   - Medium complexity
   - High developer interest

2. **Operator Overloading** (2-3 hours)
   - Complements extension methods
   - Relatively simple
   - Completes "advanced method" support

---

### Session 4 (Polish & Enhancement - 5-7 hours)

1. **Improved Documentation Extraction** (3-4 hours)
   - Better doc support
   - Useful for all existing features

2. **Async/Await Tracking** (2-3 hours)
   - Mostly syntactic
   - Good to have
   - Low complexity

---

## 🎯 Priority Recommendations

### Must Implement (High Value)
1. ✅ Generic Constraints - DONE (Session 1)
2. ✅ Nullable Reference Types - DONE (Session 1)
3. ✅ Using Directives - DONE (Session 2)
4. 🔲 Primary Constructors - C# 12 growing adoption
5. 🔲 Enhanced Records - Modern C# pattern

### Should Implement (Medium Value)
6. ✅ File-scoped Types - DONE (Session 2)
7. 🔲 Extension Methods - Useful pattern understanding
8. 🔲 Improved Documentation - Benefits all features

### Nice to Have (Lower Priority)
9. 🔲 Operator Overloading - Niche but complete
10. 🔲 Async/Await - Mostly syntactic

---

## 📊 Total Effort Estimates

| Priority | Features | Status | Time Spent/Estimate |
|----------|----------|--------|---------------------|
| Must Implement | 5 features | 3/5 complete | ~7-9 hours spent, 4-7 hours remain |
| Should Implement | 3 features | 1/3 complete | ~2 hours spent, 7-11 hours remain |
| Nice to Have | 2 features | 0/2 complete | 4-6 hours remain |
| **TOTAL** | **10 features** | **4/10 complete** | **~9-11 hours spent, 15-24 hours remain** |

### Session Progress
- **Session 1**: Generic Constraints + Nullable Types (~4-5 hours)
- **Session 2**: Using Directives + File-scoped Types (~3-4 hours)
- **Total Progress**: 40% complete

---

## 🔍 Feature Verification Checklist

Before implementing each feature, verify tree-sitter support:

```bash
# Template for checking AST structure
echo '<C# code>' | tree-sitter parse --language c-sharp

# Example for primary constructors
echo 'public class Person(string name) {}' | tree-sitter parse --language c-sharp
```

If expected nodes don't exist (like LINQ query_expression), document and skip.

---

## ✅ Quality Standards (Every Commit)

Before any commit:

- ✅ `cargo test` - all tests pass
- ✅ `cargo check` - no compilation errors
- ✅ `cargo fmt` - formatted code
- ✅ `cargo clippy` - no warnings
- ✅ Clear commit message with examples
- ✅ **NO changes outside src/parsing/csharp/parser.rs without permission**

---

## 📝 Notes

- **LINQ Query Syntax**: Documented as not feasible with current tree-sitter-c-sharp version
- **Nullable Types**: Implemented but simpler than planned (relies on tree-sitter text extraction)
- **Test Coverage**: Aim for 5-8 tests per feature minimum

---

**Document Prepared By:** Claude (Session: claude/csharp-linq-query-syntax-011CUs3A3tkdUYzZbbmUEgrx)
**Last Updated:** 2025-11-07
**Status:** Ready for implementation
