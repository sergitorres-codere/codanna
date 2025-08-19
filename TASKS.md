# Go Parser Implementation Tasks

This document provides a systematic checklist for converting the current TypeScript-based Go parser into a proper Go language parser implementation.

## Executive Summary

### Current State
- ❌ Go parser is essentially a copy of TypeScript parser with minimal changes
- ❌ Uses TypeScript AST nodes and language features throughout
- ❌ Incorrect file extensions (`.ts`/`.tsx` instead of `.go`)
- ❌ Wrong import system (ES6 modules vs Go packages)
- ❌ Incorrect visibility model (TypeScript public/private vs Go capitalization)
- ✅ Basic infrastructure is in place (tree-sitter-go dependency, registration)

### Goal
Transform the current TypeScript-based implementation into a complete Go language parser that:
- Extracts Go symbols (structs, interfaces, functions, methods, variables)
- Handles Go package system and imports correctly
- Implements Go-specific language behaviors
- Follows the established parser patterns from Rust/Python implementations
- Achieves >10,000 symbols/second performance target

### Task Priority Levels
- 🔴 **Critical**: Breaks basic functionality
- 🟡 **High**: Required for complete Go support
- 🟢 **Medium**: Optimization and edge cases

---

## Phase 1: Pre-Implementation Setup

### 1.1 ABI-15 Node Discovery 🔴
- [x] Create comprehensive Go ABI-15 exploration test in `tests/abi15_exploration.rs`
- [x] Test all Go language constructs:
  - [x] Package declarations
  - [x] Import statements
  - [x] Function declarations (regular and method receivers)
  - [x] Struct type declarations
  - [x] Interface type declarations
  - [x] Variable/constant declarations
  - [x] Type aliases
  - [x] Generic types and constraints
- [x] Document findings in `contributing/parsers/go/NODE_MAPPING.md`
- [x] Run: `cargo test explore_go_abi15_comprehensive -- --nocapture > contributing/parsers/go/node_discovery.txt`

### 1.2 Test Infrastructure 🟡
- [ ] Create Go test fixtures in `tests/fixtures/go/`
- [ ] Add comprehensive Go code examples covering all language features
- [ ] Create integration test for Go parser in `tests/integration_go.rs`

---

## Phase 2: Parser Implementation (`src/parsing/go/parser.rs`)

### 2.1 Core Structure Replacement 🔴

#### Replace TypeScript Node Types
Current TypeScript nodes → Go equivalents:
- [ ] `class_declaration` → `type_declaration` (for structs)
- [ ] `interface_declaration` → `interface_type` 
- [ ] `function_declaration` → `function_declaration`
- [ ] `method_definition` → `method_declaration` (with receiver)
- [ ] `export_statement` → Remove (Go doesn't have exports)
- [ ] `import_statement` → `import_declaration`
- [ ] `variable_declaration` → `var_declaration`
- [ ] `type_alias_declaration` → `type_declaration` (with alias)

#### Update Symbol Extraction Methods
- [ ] **`parse()` method (lines 1-100)**:
  - [ ] Remove TypeScript-specific symbol extraction
  - [ ] Add Go package clause parsing
  - [ ] Add Go struct type parsing
  - [ ] Add Go interface type parsing  
  - [ ] Add Go function parsing with receivers
  - [ ] Add Go variable/constant parsing
  - [ ] Add Go type alias parsing

### 2.2 Import System Overhaul 🔴

#### Replace ES6 Import Logic
- [ ] **`find_imports()` method**:
  - [ ] Remove ES6 `import`/`export` parsing
  - [ ] Add Go `import` declaration parsing
  - [ ] Handle Go import paths (`"fmt"`, `"github.com/user/repo"`)
  - [ ] Handle Go import aliases (`import f "fmt"`)
  - [ ] Handle Go dot imports (`import . "fmt"`)
  - [ ] Handle Go blank imports (`import _ "database/sql"`)

#### Import Structure Updates
- [ ] Update `Import` struct usage:
  - [ ] Set `is_type_only: false` (Go doesn't have type-only imports)
  - [ ] Handle Go package paths correctly
  - [ ] Map Go import aliases properly

### 2.3 Symbol Extraction Rework 🔴

#### Struct and Interface Parsing
- [ ] **`extract_struct_symbols()`** (new method):
  - [ ] Parse struct type declarations
  - [ ] Extract struct fields with types
  - [ ] Handle embedded structs
  - [ ] Extract struct methods (functions with receivers)
  - [ ] Generate proper signatures: `type Person struct { Name string; Age int }`

- [ ] **`extract_interface_symbols()`** (new method):
  - [ ] Parse interface type declarations
  - [ ] Extract interface methods
  - [ ] Handle embedded interfaces
  - [ ] Generate proper signatures: `type Writer interface { Write([]byte) (int, error) }`

#### Function and Method Parsing
- [ ] **`extract_function_symbols()`** (update):
  - [ ] Parse regular functions
  - [ ] Parse methods with receivers: `func (r *Receiver) Method() {}`
  - [ ] Handle function parameters and return types
  - [ ] Extract generic type parameters
  - [ ] Generate complete signatures without function body

#### Variable and Constant Parsing
- [ ] **`extract_variable_symbols()`** (new method):
  - [ ] Parse `var` declarations
  - [ ] Parse `const` declarations
  - [ ] Handle grouped declarations: `var ( name string; age int )`
  - [ ] Extract variable types

### 2.4 Call Extraction Updates 🟡

#### Function Calls
- [ ] **`find_calls()` method**:
  - [ ] Remove TypeScript arrow function calls
  - [ ] Add Go function call parsing
  - [ ] Handle method calls with receivers
  - [ ] Handle package-qualified calls: `fmt.Println()`

#### Method Calls  
- [ ] **`find_method_calls()` method**:
  - [ ] Update for Go method syntax
  - [ ] Handle pointer receiver calls
  - [ ] Handle interface method calls
  - [ ] Support chained method calls

### 2.5 Implementation Detection 🟡

#### Interface Implementations
- [ ] **`find_implementations()` method**:
  - [ ] Remove TypeScript class inheritance
  - [ ] Add Go interface implementation detection
  - [ ] Check if struct implements interface (implicit)
  - [ ] Handle embedded interfaces

### 2.6 Signature Extraction 🟡

#### Go-Specific Signatures
- [ ] **`extract_signature()` method**:
  - [ ] Functions: `func name(params) (returns)`
  - [ ] Methods: `func (receiver) name(params) (returns)`
  - [ ] Structs: `type Name struct { fields }`
  - [ ] Interfaces: `type Name interface { methods }`
  - [ ] Variables: `var name type` or `const name = value`

### 2.7 Documentation Comments 🟢
- [ ] **`extract_doc_comment()` method**:
  - [ ] Support Go-style doc comments (`// Comment`)
  - [ ] Handle multi-line doc comments
  - [ ] Associate comments with symbols correctly

### 2.8 Test Updates 🟡
- [ ] Replace all TypeScript test code with Go examples
- [ ] Add tests for all Go language features
- [ ] Verify performance benchmarks with Go code

---

## Phase 3: Behavior Implementation (`src/parsing/go/behavior.rs`)

### 3.1 Module Path Formatting 🔴
- [ ] **`format_module_path()` method**:
  - [ ] Update from TypeScript module paths to Go package paths
  - [ ] Handle Go package imports: `github.com/user/repo/package`
  - [ ] Support standard library packages: `fmt`, `strings`, etc.

- [ ] **`module_separator()` method**:
  - [ ] Change from `"."` to appropriate Go separator (likely `"/"` for packages)

### 3.2 Visibility Rules 🔴  
- [ ] **`parse_visibility()` method**:
  - [ ] Remove TypeScript `public`/`private`/`protected` keywords
  - [ ] Implement Go capitalization-based visibility:
    - [ ] Uppercase first letter = public/exported
    - [ ] Lowercase first letter = private/unexported
  - [ ] Apply to all symbol types (functions, structs, fields, methods)

### 3.3 Language Capabilities 🟡
- [ ] **Update capability flags**:
  - [ ] `supports_traits()` → `false` (Go has interfaces, not traits)
  - [ ] `supports_inherent_methods()` → `true` (Go has methods on types)
  - [ ] Add `supports_interfaces()` → `true`
  - [ ] Add `supports_embedded_types()` → `true`

### 3.4 Symbol Resolution 🟡
- [ ] **`resolve_symbol()` method**:
  - [ ] Implement Go package-based symbol resolution
  - [ ] Handle local package symbols
  - [ ] Handle imported package symbols
  - [ ] Support Go module resolution

- [ ] **`is_resolvable_symbol()` method**:
  - [ ] Update for Go symbol types
  - [ ] Handle exported vs unexported symbols

### 3.5 Symbol Configuration 🟡
- [ ] **`configure_symbol()` method**:
  - [ ] Set appropriate Go module paths
  - [ ] Configure Go-specific symbol properties

### 3.6 Test Updates 🟡  
- [ ] Replace TypeScript test cases with Go examples
- [ ] Test Go package resolution
- [ ] Test visibility parsing with Go naming conventions

---

## Phase 4: Definition Updates (`src/parsing/go/definition.rs`)

### 4.1 Basic Metadata 🔴
- [ ] **`extensions()` method**:
  - [ ] Change from `&["ts", "tsx"]` to `&["go"]`

- [ ] **Language identification**:
  - [ ] Verify `id()` returns `LanguageId::Go`
  - [ ] Verify `name()` returns `"Go"`

### 4.2 AST Node Definitions 🟡
- [ ] **Update Go node type mappings**:
  - [ ] Document all Go Tree-sitter node types
  - [ ] Map to appropriate SymbolKind values
  - [ ] Handle Go-specific constructs

### 4.3 Symbol Classifications 🟡
- [ ] **`SymbolKind` mappings**:
  - [ ] Struct → `SymbolKind::Struct`
  - [ ] Interface → `SymbolKind::Interface`  
  - [ ] Function → `SymbolKind::Function`
  - [ ] Method → `SymbolKind::Method`
  - [ ] Variable → `SymbolKind::Variable`
  - [ ] Constant → `SymbolKind::Constant`
  - [ ] Type alias → `SymbolKind::Type`

### 4.4 Factory Methods 🟡
- [ ] **`create_parser()` and `create_behavior()` methods**:
  - [ ] Verify they create Go-specific instances
  - [ ] Remove any TypeScript-specific configuration

### 4.5 Test Updates 🟢
- [ ] Add Go-specific definition tests
- [ ] Test file extension recognition
- [ ] Test factory method behavior

---

## Phase 5: Resolution Implementation (`src/parsing/go/resolution.rs`)

### 5.1 Package Resolution 🟡
- [ ] **Implement Go package system**:
  - [ ] Resolve local package symbols
  - [ ] Resolve imported package symbols
  - [ ] Handle Go module paths
  - [ ] Support standard library packages

### 5.2 Import Resolution 🟡
- [ ] **Go import path resolution**:
  - [ ] Handle relative imports
  - [ ] Handle absolute module paths
  - [ ] Support vendor directories
  - [ ] Handle Go module system (`go.mod`)

### 5.3 Type System Integration 🟢
- [ ] **Go type resolution**:
  - [ ] Resolve user-defined types
  - [ ] Handle built-in types
  - [ ] Support generic type parameters (Go 1.18+)
  - [ ] Resolve interface implementations

### 5.4 Scope Management 🟡
- [ ] **Go-specific scoping rules**:
  - [ ] Package-level scope
  - [ ] Function-level scope
  - [ ] Block-level scope
  - [ ] Method receiver scope

---

## Phase 6: Module Integration (`src/parsing/go/mod.rs`)

### 6.1 Documentation 🟢
- [ ] **Update module documentation**:
  - [ ] Change from "TypeScript" to "Go" 
  - [ ] Update feature descriptions
  - [ ] Document Go-specific capabilities

### 6.2 Re-exports 🟢
- [ ] **Verify all re-exports**:
  - [ ] `pub use parser::GoParser;`
  - [ ] `pub use behavior::GoBehavior;`
  - [ ] `pub use definition::GoLanguage;`
  - [ ] `pub(crate) use definition::register;`

### 6.3 Integration Tests 🟡
- [ ] Add comprehensive integration tests
- [ ] Test module registration
- [ ] Test end-to-end functionality

---

## Phase 7: Testing and Validation

### 7.1 Unit Tests 🔴
- [ ] **Parser tests**:
  - [ ] Test all Go symbol extraction
  - [ ] Test import parsing
  - [ ] Test signature generation
  - [ ] Test error handling

- [ ] **Behavior tests**:
  - [ ] Test visibility parsing
  - [ ] Test module path formatting
  - [ ] Test symbol resolution

### 7.2 Integration Tests 🟡
- [ ] **End-to-end tests**:
  - [ ] Test complete Go project indexing
  - [ ] Test cross-package symbol resolution
  - [ ] Test performance with large Go codebases

### 7.3 Performance Validation 🟡
- [ ] **Benchmark tests**:
  - [ ] Verify >10,000 symbols/second target
  - [ ] Memory usage within acceptable limits
  - [ ] Compare with other language parsers

### 7.4 Regression Tests 🟢
- [ ] **Ensure no breakage**:
  - [ ] All existing language parsers still work
  - [ ] Go parser properly registered
  - [ ] MCP server recognizes Go files

---

## Phase 8: Documentation and Cleanup

### 8.1 Code Documentation 🟢
- [ ] Add comprehensive code comments
- [ ] Document Go-specific implementation decisions
- [ ] Update README if needed

### 8.2 Final Cleanup 🟢
- [ ] Remove all TypeScript references from comments
- [ ] Remove unused imports and code
- [ ] Run `cargo fmt` and `cargo clippy`
- [ ] Verify no TypeScript artifacts remain

### 8.3 Performance Verification 🟡
- [ ] Run `codanna benchmark go`
- [ ] Ensure performance targets met
- [ ] Document any performance characteristics

---

## Validation Checklist

### ✅ Completion Criteria
- [ ] All TypeScript code removed/converted
- [ ] Go parser handles all major Go language features
- [ ] Performance targets achieved (>10k symbols/sec)
- [ ] All tests passing
- [ ] Integration with MCP server working
- [ ] No regression in other language parsers

### 🔍 Quality Gates  
- [ ] `cargo test` passes all tests
- [ ] `cargo clippy` reports no warnings
- [ ] `cargo fmt --check` reports no formatting issues
- [ ] `./contributing/scripts/full-test.sh` passes completely

---

## Dependencies Between Tasks

**Critical Path:**
1. ABI-15 Node Discovery → All parser implementation
2. Parser core structure → All other parser methods
3. Import system → Symbol resolution
4. Symbol extraction → Behavior implementation
5. Behavior implementation → Resolution implementation

**Parallel Work Possible:**
- Definition updates can happen alongside parser work
- Test creation can happen alongside implementation
- Documentation can happen alongside implementation

---

## Estimated Timeline

- **Phase 1**: 1-2 days (node discovery and setup)
- **Phase 2**: 3-4 days (core parser implementation) 
- **Phase 3**: 1-2 days (behavior implementation)
- **Phase 4**: 0.5 days (definition updates)
- **Phase 5**: 1-2 days (resolution implementation)
- **Phase 6**: 0.5 days (module integration)
- **Phase 7**: 1-2 days (testing and validation)
- **Phase 8**: 0.5 days (documentation and cleanup)

**Total Estimated Time: 8-13 days**

---

*This document should be updated as tasks are completed. Each checkbox represents a specific, testable deliverable.*