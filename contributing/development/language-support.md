# Adding Language Support

> ⚠️ **IMPORTANT NOTICE - Breaking Changes Coming in v0.4.1** ⚠️
> 
> We are currently refactoring the language behavior system to be truly language-agnostic.
> This will make adding new languages significantly easier and more maintainable.
> 
> **If you're planning to add a new language:**
> - Please wait for the v0.4.1 release (expected August 17, 2025, TypeScript included)
> - The new architecture will eliminate many manual steps
> - Language-specific resolution logic will be self-contained
> 
> **Current limitations being addressed:**
> - Hardcoded Rust-specific resolution logic in `SimpleIndexer`
> - Language-specific traits/interfaces handled incorrectly
> - Module path resolution assumes Rust conventions

Languages self-register via the registry system. Each language lives in its own subdirectory for clean organization.

## Architecture

Each language needs:

1. `definition.rs` - implements `LanguageDefinition`
2. `parser.rs` - implements `LanguageParser`
3. `behavior.rs` - implements `LanguageBehavior`
4. `mod.rs` - module re-exports

## File Structure

```
src/parsing/
├── {language}/
│   ├── mod.rs        # Module re-exports
│   ├── parser.rs     # LanguageParser trait
│   ├── behavior.rs   # LanguageBehavior trait
│   └── definition.rs # LanguageDefinition trait
├── registry.rs       # Add registration call
└── [shared infrastructure files]
```

## Key Trait Signatures

### LanguageDefinition

```rust
fn id(&self) -> LanguageId
fn extensions(&self) -> &'static [&'static str]
fn create_parser(&self, settings: &Settings) -> IndexResult<Box<dyn LanguageParser>>
fn create_behavior(&self) -> Box<dyn LanguageBehavior>
```

### LanguageParser

```rust
fn parse(&mut self, code: &str, file_id: FileId, counter: &mut SymbolCounter) -> Vec<Symbol>
fn find_calls(&mut self, code: &str) -> Vec<SimpleCall>
fn find_implementations(&mut self, code: &str) -> Vec<(&str, &str, Range)>
fn find_imports(&mut self, code: &str, file_id: FileId) -> Vec<Import>
```

### LanguageBehavior

```rust
fn module_separator(&self) -> &'static str
fn module_path_from_file(&self, file_path: &Path) -> Option<String>
fn parse_visibility(&self, signature: &str) -> Visibility
fn supports_traits(&self) -> bool
fn supports_inherent_methods(&self) -> bool
```

## Step 1: ABI-15 Node Discovery (Required)

Create a comprehensive ABI-15 exploration test before implementing any parser functionality. Tree-sitter node names frequently differ from language keywords (e.g., `abstract_class_declaration` not `abstract class`), making node discovery essential for correct implementation.

### Creating the Node Discovery Test

1. Add a comprehensive test to `tests/abi15_exploration.rs` that covers all language constructs
2. Document findings in `docs/enhancements/{language}/NODE_MAPPING.md`
3. Reference the node mapping document during implementation

### Running the Explorer

```bash
# Run your comprehensive language test
cargo test explore_{language}_abi15_comprehensive -- --nocapture > docs/enhancements/{language}/node_discovery.txt
```

### What to Discover

The explorer reveals:

- Exact node type names for all language constructs
- Field names for data extraction
- Node IDs for validation
- Parent-child relationships
- ABI version compatibility

### Example Discovery Output

```rust
#[test]
fn explore_typescript_abi15_comprehensive() {
    // Discovered that TypeScript uses:
    // - "class_declaration" for regular classes
    // - "abstract_class_declaration" for abstract classes (not "class_declaration" with modifier)
    // - "interface_declaration" for interfaces
    // - "type_alias_declaration" for type aliases
}
```

### Node Categories to Test

Every language test should explore:
- Class/struct declarations (including abstract, sealed, etc.)
- Interface/trait declarations
- Function/method declarations
- Variable/constant declarations
- Type definitions
- Import/export statements
- Module/namespace declarations
- Language-specific constructs (decorators, attributes, etc.)

## Implementation Checklist

1. Run ABI-15 node discovery test and document findings
2. Create directory: `src/parsing/{language}/`
3. Implement the four required files:
   - `parser.rs` - Main parsing logic
   - `behavior.rs` - Language-specific behaviors
   - `definition.rs` - Registry definition with `register()` function
   - `mod.rs` - Module re-exports
4. Add to `src/parsing/registry.rs`: `super::{language}::register(registry);`
5. Update `src/parsing/mod.rs`: 
   - Add `pub mod {language};`
   - Add `pub use {language}::{LanguageParser, LanguageBehavior};`
6. Add to `Cargo.toml`: `tree-sitter-{language} = "0.x"`

## Quick Verification

```bash
cargo build --release
./target/release/codanna init  # Should see your language
./target/release/codanna index test.ext --progress
./target/release/codanna retrieve symbol YourSymbol
```

## Performance Requirements

- **Target**: >10,000 symbols/second
- **Memory**: Use `&str` and `&code[node.byte_range()]`
- **IDs**: Use `SymbolCounter`, not raw `u32`

## Example Implementations

- `src/parsing/rust/` - Full Rust implementation with traits and inherent methods
- `src/parsing/python/` - Python with naming convention visibility
- `src/parsing/php/` - PHP with namespace handling

## mod.rs Template

```rust
//! {Language} language parser implementation

pub mod behavior;
pub mod definition;
pub mod parser;

pub use behavior::{Language}Behavior;
pub use parser::{Language}Parser;
pub use definition::{Language}Language;

// Re-export for registry registration
pub(crate) use definition::register;
```
