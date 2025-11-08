# C# Parser Implementation - Detailed Action Plan

**Branch:** `claude/comprehensive-csharp-parser-final`
**Status:** 9/10 original features complete ✅
**Grade:** A- (Excellent, nearly production-ready)

---

## 🎯 Quick Summary

The C# parser is **already the most comprehensive parser in Codanna** with:
- ✅ 5,598 lines of code (74% larger than TypeScript)
- ✅ 159 functions (137% more than TypeScript)
- ✅ 53 passing tests (1,212 lines of test code)
- ✅ All major C# features supported
- ✅ Advanced features: Operator overloading, extension methods, async/await, attributes API, pattern matching API

**Main Gaps:**
1. User-facing documentation (3 hours)
2. Structured XML doc parsing (4-5 hours)
3. Benchmark suite (3-4 hours)
4. Cross-file resolution tests (4-5 hours)

---

## 📋 Phased Implementation Plan

### PHASE 1: Polish & Documentation (8-12 hours) 🔥 **START HERE**

**Goal:** Make existing implementation production-ready

#### Task 1.1: User Documentation (3 hours)
**Priority:** 🔴 CRITICAL
**File:** Create `docs/parsers/csharp.md`

**Subtasks:**
- [ ] Write overview section
  - What the C# parser does
  - Supported C# versions
  - Key features
- [ ] Add "Getting Started" section
  - Installation
  - Basic usage example
  - Configuration options
- [ ] Document public APIs
  - `parse()` method
  - `find_attributes()` method
  - `find_patterns()` method
  - `find_calls()`, `find_implementations()`, `find_imports()`
- [ ] Add usage examples
  ```rust
  // Example 1: Basic parsing
  // Example 2: Finding attributes
  // Example 3: Pattern matching
  // Example 4: Call graph extraction
  ```
- [ ] List known limitations
  - LINQ query syntax not supported
  - XML docs are raw text (for now)
- [ ] Add troubleshooting section
- [ ] Link to rustdoc for detailed API reference

**Acceptance Criteria:**
- New users can get started in <5 minutes
- All public APIs are documented with examples
- Limitations are clearly stated

---

#### Task 1.2: Architecture Documentation (2 hours)
**Priority:** 🟡 HIGH
**File:** Create `contributing/parsers/csharp/ARCHITECTURE.md`

**Subtasks:**
- [ ] Explain high-level design
  - Module structure (parser, behavior, resolution, definition)
  - AST traversal strategy
  - Scope tracking mechanism
- [ ] Document key design decisions
  - Why extension methods use `[ext:Type]` naming
  - How type inference works (5 strategies)
  - Pattern matching implementation
  - Attribute extraction approach
- [ ] Explain scope context system
  - How caller context is maintained
  - Why it's critical for call graphs
  - Edge cases and solutions
- [ ] Document extensibility points
  - How to add new symbol types
  - How to add new relationship types
  - Where to hook into parsing pipeline
- [ ] Add diagrams (if helpful)
  - AST traversal flow
  - Scope stack visualization
  - Relationship resolution pipeline

**Acceptance Criteria:**
- Contributors understand the design philosophy
- Easy to locate code for specific features
- Clear guidance on adding new features

---

#### Task 1.3: Testing Guide (1 hour)
**Priority:** 🟡 HIGH
**File:** Create `contributing/parsers/csharp/TESTING_GUIDE.md`

**Subtasks:**
- [ ] Explain test organization
  - Unit tests in parser.rs
  - Integration tests in tests/parsers/csharp/
  - Example files in examples/csharp/
- [ ] Document test categories
  - Symbol extraction tests
  - Relationship tests
  - Feature-specific tests (operator overloading, extension methods, etc.)
  - Edge case tests
- [ ] Provide "how to add a test" guide
  - When to add unit vs integration test
  - How to structure test code
  - How to use test helpers
- [ ] Add examples
  ```rust
  // Example: Testing a new feature
  // Example: Testing error handling
  // Example: Testing relationships
  ```
- [ ] Explain test naming conventions
- [ ] Document how to run tests
  ```bash
  cargo test --lib csharp
  cargo test --test parsers_tests csharp
  ```

**Acceptance Criteria:**
- New contributors know how to add tests
- Test organization is clear
- Examples cover common scenarios

---

#### Task 1.4: Error Handling Tests (2-3 hours)
**Priority:** 🟡 HIGH
**File:** Create `tests/parsers/csharp/test_error_handling.rs`

**Subtasks:**
- [ ] Test malformed C# code
  ```rust
  #[test]
  fn test_missing_braces() {
      let code = "class Foo { void Bar() /* missing closing brace */";
      // Should not panic, should return partial results
  }
  ```
- [ ] Test incomplete syntax
  - Unclosed generics: `List<`
  - Missing semicolons
  - Incomplete method signatures
- [ ] Test invalid constructs
  - Multiple visibility modifiers
  - Invalid generic constraints
  - Malformed attributes
- [ ] Test edge cases
  - Empty files
  - Files with only comments
  - Files with only whitespace
  - Very long identifiers (1000+ chars)
  - Deeply nested namespaces (20+ levels)
  - Very long generic lists
- [ ] Verify graceful degradation
  - Parser doesn't panic
  - Returns as many symbols as possible
  - Logs warnings for unparseable sections

**Acceptance Criteria:**
- Parser never panics on malformed code
- At least 10 error handling tests
- Edge cases are documented

---

#### Task 1.5: Code Comments Cleanup (1-2 hours)
**Priority:** 🟢 MEDIUM
**Files:** `src/parsing/csharp/parser.rs`, other module files

**Subtasks:**
- [ ] Review all public functions
  - Ensure they have doc comments
  - Add examples where helpful
  - Document parameters and return values
- [ ] Review complex private functions
  - Add inline comments explaining logic
  - Document non-obvious algorithms
- [ ] Update module-level documentation
  - Ensure `mod.rs` is up-to-date
  - Add "see also" references
- [ ] Add examples to tricky functions
  ```rust
  /// Extract type from initializer expression
  ///
  /// # Examples
  ///
  /// ```
  /// var user = new User();  // Returns "User"
  /// var result = GetUser(); // Returns "User" (heuristic)
  /// ```
  fn try_infer_type_from_initializer(...)
  ```
- [ ] Fix any rustdoc warnings
  ```bash
  cargo doc --package codanna --no-deps
  ```

**Acceptance Criteria:**
- All public functions have doc comments
- Complex logic is explained
- No rustdoc warnings

---

### PHASE 2: API Enhancements (10-14 hours)

**Goal:** Expose powerful APIs for advanced use cases

#### Task 2.1: Structured XML Documentation (4-5 hours)
**Priority:** 🟡 HIGH
**Files:** `src/types.rs`, `src/parsing/csharp/parser.rs`, `tests/parsers/csharp/test_parser.rs`

**Subtasks:**
- [ ] Define `ParsedDocComment` struct
  ```rust
  // In src/types.rs
  #[derive(Debug, Clone, PartialEq)]
  pub struct ParsedDocComment {
      pub summary: Option<String>,
      pub remarks: Option<String>,
      pub params: Vec<ParamDoc>,
      pub returns: Option<String>,
      pub exceptions: Vec<ExceptionDoc>,
      pub examples: Vec<String>,
      pub see_also: Vec<String>,
      pub raw_xml: String,  // Keep original for fallback
  }

  pub struct ParamDoc {
      pub name: String,
      pub description: String,
  }

  pub struct ExceptionDoc {
      pub exception_type: String,
      pub description: String,
  }
  ```
- [ ] Implement XML parser
  - Use simple regex or lightweight XML parser
  - Extract `<summary>`, `<param>`, `<returns>`, `<exception>`, `<remarks>`, `<example>`, `<see>`
  - Handle multiline tags
  - Handle nested tags
- [ ] Update Symbol struct
  ```rust
  // Change from:
  pub doc_comment: Option<String>
  // To:
  pub doc_comment: Option<ParsedDocComment>
  ```
- [ ] Update parser extraction logic
  - Parse XML in `extract_doc_comment()`
  - Fall back to raw text if parsing fails
- [ ] Add comprehensive tests
  ```rust
  #[test]
  fn test_xml_doc_with_params() {
      let code = r#"
      /// <summary>
      /// Adds two numbers
      /// </summary>
      /// <param name="a">First number</param>
      /// <param name="b">Second number</param>
      /// <returns>The sum</returns>
      public int Add(int a, int b) { return a + b; }
      "#;
      // Verify parsed doc has summary, 2 params, returns
  }
  ```
- [ ] Update documentation
  - Document new struct in rustdoc
  - Add examples of querying parsed docs
  - Note fallback behavior for malformed XML

**Acceptance Criteria:**
- XML doc tags are parsed into structured data
- At least 6-8 tests for different XML formats
- Backward compatible (raw XML preserved)
- Documentation updated

---

#### Task 2.2: Generic Type Information API (3-4 hours)
**Priority:** 🟢 MEDIUM
**Files:** `src/parsing/csharp/parser.rs`, `tests/parsers/csharp/test_parser.rs`

**Subtasks:**
- [ ] Define public structs
  ```rust
  #[derive(Debug, Clone, PartialEq)]
  pub struct GenericTypeInfo {
      pub type_name: String,
      pub type_parameters: Vec<TypeParameter>,
      pub range: Range,
  }

  #[derive(Debug, Clone, PartialEq)]
  pub struct TypeParameter {
      pub name: String,
      pub constraints: Vec<String>,
  }
  ```
- [ ] Add public API method
  ```rust
  impl CSharpParser {
      /// Find all generic type definitions in the code
      ///
      /// # Examples
      ///
      /// ```
      /// let code = "class Foo<T> where T : IDisposable { }";
      /// let generics = parser.find_generic_types(code);
      /// assert_eq!(generics[0].type_parameters[0].name, "T");
      /// assert_eq!(generics[0].type_parameters[0].constraints[0], "IDisposable");
      /// ```
      pub fn find_generic_types(&mut self, code: &str) -> Vec<GenericTypeInfo> {
          // Traverse AST, find type_parameter_list nodes
          // Extract constraints from where clauses
      }
  }
  ```
- [ ] Implement extraction logic
  - Reuse existing constraint extraction code
  - Handle multiple where clauses
  - Handle multiple constraints per parameter
- [ ] Add tests
  ```rust
  #[test]
  fn test_generic_class_with_constraints() { }

  #[test]
  fn test_generic_method_with_multiple_params() { }

  #[test]
  fn test_nested_generic_types() { }
  ```
- [ ] Document API
  - Add to `docs/parsers/csharp.md`
  - Add rustdoc examples

**Acceptance Criteria:**
- Public API works for classes, interfaces, methods
- Constraints are correctly extracted
- At least 5-6 tests
- Documented with examples

---

#### Task 2.3: Enhanced Attribute API (2-3 hours)
**Priority:** 🟢 MEDIUM
**Files:** `src/parsing/csharp/parser.rs`, `tests/parsers/csharp/test_parser.rs`

**Subtasks:**
- [ ] Add filtering options
  ```rust
  impl CSharpParser {
      /// Find attributes with optional filtering
      pub fn find_attributes_filtered(
          &mut self,
          code: &str,
          target_kind: Option<SymbolKind>,  // Only attributes on methods, classes, etc.
          attribute_name: Option<&str>,     // Only specific attribute names
      ) -> Vec<AttributeInfo> {
          // Filter results based on criteria
      }

      /// Find all attributes on a specific symbol
      pub fn find_attributes_for_symbol(
          &mut self,
          code: &str,
          symbol_name: &str,
      ) -> Vec<AttributeInfo> {
          // Find attributes attached to specific symbol
      }
  }
  ```
- [ ] Add convenience methods
  ```rust
  /// Check if a symbol has a specific attribute
  pub fn has_attribute(&mut self, code: &str, symbol: &str, attr: &str) -> bool {
      self.find_attributes_for_symbol(code, symbol)
          .iter()
          .any(|a| a.name == attr)
  }
  ```
- [ ] Add tests for new filtering
  ```rust
  #[test]
  fn test_filter_attributes_by_kind() {
      // Find only attributes on methods
  }

  #[test]
  fn test_find_specific_attribute_name() {
      // Find all [HttpGet] attributes
  }
  ```
- [ ] Update documentation
  - Add filtering examples
  - Document use cases

**Acceptance Criteria:**
- Filtering works correctly
- At least 3-4 new tests
- API is intuitive and well-documented

---

### PHASE 3: Performance & Scale (6-9 hours)

**Goal:** Ensure parser performs well on large codebases

#### Task 3.1: Benchmark Suite (3-4 hours)
**Priority:** 🟡 HIGH
**File:** Create `benches/csharp_parser_bench.rs`

**Subtasks:**
- [ ] Set up benchmark infrastructure
  ```toml
  # In Cargo.toml
  [[bench]]
  name = "csharp_parser_bench"
  harness = false
  ```
- [ ] Create benchmark file
  ```rust
  use criterion::{black_box, criterion_group, criterion_main, Criterion};
  use codanna::parsing::csharp::CSharpParser;
  use codanna::parsing::LanguageParser;

  fn bench_parse_comprehensive(c: &mut Criterion) {
      let code = std::fs::read_to_string("examples/csharp/comprehensive.cs").unwrap();
      c.bench_function("parse comprehensive.cs", |b| {
          b.iter(|| {
              let mut parser = CSharpParser::new().unwrap();
              let mut counter = SymbolCounter::new();
              black_box(parser.parse(&code, FileId::new(1).unwrap(), &mut counter))
          })
      });
  }

  criterion_group!(benches, bench_parse_comprehensive);
  criterion_main!(benches);
  ```
- [ ] Add benchmarks for:
  - Parsing comprehensive.cs (636 lines)
  - Parsing a large file (create 5000+ line example)
  - Finding all calls
  - Finding all interface implementations
  - Finding all attributes
  - Finding all patterns
- [ ] Run baseline benchmarks
  ```bash
  cargo bench --bench csharp_parser_bench
  ```
- [ ] Document results
  - Create `contributing/parsers/csharp/PERFORMANCE.md`
  - Include baseline numbers
  - Add performance expectations

**Acceptance Criteria:**
- Benchmark suite runs successfully
- Baseline performance documented
- Can detect regressions
- Benchmarks cover key operations

---

#### Task 3.2: Performance Optimization (3-5 hours)
**Priority:** 🟢 MEDIUM
**Files:** `src/parsing/csharp/parser.rs`

**Subtasks:**
- [ ] Profile parser
  ```bash
  cargo flamegraph --bench csharp_parser_bench
  ```
- [ ] Identify bottlenecks
  - String allocations
  - Redundant AST traversals
  - Expensive regex operations
  - Unnecessary cloning
- [ ] Optimize hot paths
  - Use string slices instead of String where possible
  - Cache repeated AST queries
  - Reduce allocations in tight loops
- [ ] Re-run benchmarks
  - Compare before/after
  - Document improvements
- [ ] Add performance tests
  ```rust
  #[test]
  fn test_large_file_performance() {
      let code = /* 10000 lines */;
      let start = Instant::now();
      parser.parse(code, file_id, &mut counter);
      let duration = start.elapsed();
      assert!(duration < Duration::from_millis(500), "Too slow!");
  }
  ```

**Acceptance Criteria:**
- Performance is measured and documented
- Any obvious bottlenecks are addressed
- Parser can handle large files (10k+ lines)
- Performance tests prevent regressions

---

### PHASE 4: Advanced Features (12-16 hours)

**Goal:** Support advanced C# patterns

#### Task 4.1: Cross-File Resolution Tests (4-5 hours)
**Priority:** 🟡 HIGH
**Files:** Create `tests/parsers/csharp/test_cross_file_resolution.rs`, `examples/csharp/multi_file/`

**Subtasks:**
- [ ] Create multi-file example project
  ```
  examples/csharp/multi_file/
    ├── Extensions/StringExtensions.cs
    ├── Interfaces/IDataService.cs
    ├── Models/User.cs
    ├── Services/UserService.cs (implements IDataService)
    └── Program.cs (uses all of the above)
  ```
- [ ] Write tests for:
  - Extension method defined in one file, used in another
  - Interface in one file, implementation in another
  - Partial classes across multiple files
  - Using directive resolution across files
  - Base class in one file, derived class in another
- [ ] Test scenario: Index entire project
  ```rust
  #[test]
  fn test_extension_method_cross_file() {
      // Parse StringExtensions.cs
      let ext_symbols = parser.parse(ext_code, ...);

      // Parse Program.cs which uses extension
      let prog_symbols = parser.parse(prog_code, ...);

      // Verify extension method is tracked
      let calls = parser.find_calls(prog_code);
      assert!(calls.iter().any(|(_, to, _)| to.contains("IsEmpty")));
  }
  ```
- [ ] Document cross-file behavior
  - How to handle multi-file projects
  - Limitations (if any)
  - Best practices

**Acceptance Criteria:**
- Multi-file project example exists
- At least 5-6 cross-file tests
- Documentation explains multi-file handling
- All tests pass

---

#### Task 4.2: Lambda Expression Tracking (5-6 hours) [OPTIONAL]
**Priority:** 🟢 LOW
**Files:** `src/parsing/csharp/parser.rs`, `tests/parsers/csharp/test_parser.rs`

**Subtasks:**
- [ ] Define lambda representation
  ```rust
  // Add to SymbolKind enum
  Lambda,

  // Or create specialized struct
  pub struct LambdaInfo {
      pub parameters: Vec<String>,
      pub captures: Vec<String>,  // Captured variables
      pub body_range: Range,
      pub range: Range,
  }
  ```
- [ ] Extract lambda expressions
  - Find `lambda_expression` nodes
  - Extract parameters
  - Identify captured variables
  - Track as symbols or relationships
- [ ] Add public API
  ```rust
  impl CSharpParser {
      pub fn find_lambdas(&mut self, code: &str) -> Vec<LambdaInfo> {
          // Extract all lambda expressions
      }
  }
  ```
- [ ] Add comprehensive tests
  ```rust
  #[test]
  fn test_simple_lambda() {
      let code = "x => x * 2";
      // Verify lambda found
  }

  #[test]
  fn test_lambda_with_captures() {
      let code = r#"
      int multiplier = 2;
      var func = x => x * multiplier;  // Captures multiplier
      "#;
      // Verify capture tracked
  }
  ```
- [ ] Document API

**Acceptance Criteria:**
- Lambdas are extracted and tracked
- Captures are identified
- Public API available
- 8-10 tests
- Documentation complete

---

#### Task 4.3: Advanced Pattern Tests (3-4 hours)
**Priority:** 🟢 LOW
**Files:** `tests/parsers/csharp/test_parser.rs`

**Subtasks:**
- [ ] Test record patterns (C# 10+)
  ```rust
  #[test]
  fn test_record_pattern_matching() {
      let code = r#"
      if (obj is Point { X: 0, Y: var y }) {
          Console.WriteLine(y);
      }
      "#;
      // Verify pattern extraction
  }
  ```
- [ ] Test tuple deconstruction
  ```rust
  #[test]
  fn test_tuple_deconstruction() {
      let code = "var (x, y) = GetCoordinates();";
      // Verify x and y are tracked as variables
  }
  ```
- [ ] Test switch expressions (C# 8)
  ```rust
  #[test]
  fn test_switch_expression_patterns() {
      let code = r#"
      var result = value switch {
          < 0 => "negative",
          0 => "zero",
          > 0 => "positive"
      };
      "#;
      // Verify patterns found
  }
  ```
- [ ] Test nested patterns
  ```rust
  #[test]
  fn test_deeply_nested_patterns() {
      let code = r#"
      if (obj is Container { Items: { Length: > 0 } items }) { }
      "#;
      // Verify nested pattern extraction
  }
  ```
- [ ] Document pattern support
  - What patterns are fully supported
  - What patterns are partially supported
  - Known limitations

**Acceptance Criteria:**
- All major pattern types tested
- Edge cases covered
- At least 8-10 new tests
- Pattern support documented

---

## 📊 Summary Checklist

### Quick Wins (Do These First!)

- [ ] **User Documentation** (3h) - `docs/parsers/csharp.md`
- [ ] **Error Handling Tests** (2-3h) - `tests/parsers/csharp/test_error_handling.rs`
- [ ] **Testing Guide** (1h) - `contributing/parsers/csharp/TESTING_GUIDE.md`

**Total: 6-7 hours** → Brings parser to production-ready state

---

### High-Value Features

- [ ] **Structured XML Docs** (4-5h) - Parse XML tags
- [ ] **Benchmark Suite** (3-4h) - Performance testing
- [ ] **Architecture Docs** (2h) - Design documentation

**Total: 9-11 hours** → Makes parser best-in-class

---

### Advanced Features (Nice-to-Have)

- [ ] **Cross-File Tests** (4-5h)
- [ ] **Generic Type API** (3-4h)
- [ ] **Enhanced Attribute API** (2-3h)
- [ ] **Lambda Tracking** (5-6h) [OPTIONAL]
- [ ] **Advanced Pattern Tests** (3-4h)

**Total: 17-22 hours** → Complete feature parity with any C# tool

---

## 🎯 Recommended Approach

### **Option A: Minimum Viable (6-7 hours)**
1. User Documentation (3h)
2. Error Handling Tests (2-3h)
3. Testing Guide (1h)

→ **Result:** Production-ready parser

---

### **Option B: High Quality (15-18 hours)**
1. All of Option A (6-7h)
2. Structured XML Docs (4-5h)
3. Architecture Docs (2h)
4. Benchmark Suite (3-4h)

→ **Result:** Best-in-class parser

---

### **Option C: Complete (36-51 hours)**
1. All phases (1-4)
2. Every feature implemented
3. Comprehensive documentation

→ **Result:** Industry-leading C# parser

---

## ✅ Success Metrics

After completing the recommended tasks, the C# parser will:

1. ✅ **Be production-ready** - Robust error handling, comprehensive tests
2. ✅ **Have excellent docs** - Users and contributors can get started quickly
3. ✅ **Outperform competitors** - Benchmarked and optimized
4. ✅ **Support advanced use cases** - Structured XML docs, generic type queries
5. ✅ **Be maintainable** - Clear architecture, testing guide, code comments

**Current Grade:** A- (Excellent)
**After Phase 1:** A (Production Ready)
**After Phase 2:** A+ (Best in Class)

---

## 📚 Related Documents

- **Full Analysis:** `CSHARP_COMPREHENSIVE_ANALYSIS.md`
- **Current Roadmap:** `CSHARP_PARSER_REMAINING_FEATURES.md`
- **Audit Report:** `contributing/parsers/csharp/AUDIT_REPORT.md`
- **Grammar Analysis:** `contributing/parsers/csharp/GRAMMAR_ANALYSIS.md`

---

**Last Updated:** 2025-11-08
**Status:** Ready for implementation
