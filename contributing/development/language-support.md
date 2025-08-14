# Adding Language Support

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

## ABI-15 Node Exploration Tool

Before implementing a parser, use `tests/abi15_exploration.rs` to discover the correct node type names for your language. Tree-sitter node names often don't match language keywords.

### Running the Explorer

```bash
# Add your language test to tests/abi15_exploration.rs
cargo test explore_yourlang_abi15 -- --nocapture
```

### What It Reveals

The explorer shows:

- Actual node type names (e.g., `enum_item` not `enum` in Rust)
- Available field names for extracting data
- Node IDs for validation
- ABI version compatibility

### Example Discovery

```rust
#[test]
fn explore_rust_abi15_features() {
    let language: Language = tree_sitter_rust::LANGUAGE.into();

    // This revealed that Rust uses:
    // - "enum_item" not "enum"
    // - "type_item" not "type_alias"
    // - "const_item" not "const"
}
```

This tool prevents hours of debugging incorrect node names. Always explore first, then implement.

## Implementation Checklist

1. Create directory: `src/parsing/{language}/`
2. Implement the four required files:
   - `parser.rs` - Main parsing logic
   - `behavior.rs` - Language-specific behaviors
   - `definition.rs` - Registry definition with `register()` function
   - `mod.rs` - Module re-exports
3. Add to `src/parsing/registry.rs`: `super::{language}::register(registry);`
4. Update `src/parsing/mod.rs`: 
   - Add `pub mod {language};`
   - Add `pub use {language}::{LanguageParser, LanguageBehavior};`
5. Add to `Cargo.toml`: `tree-sitter-{language} = "0.x"`

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
