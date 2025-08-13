# Adding Language Support

Languages self-register via `LazyLock`. No manual registration needed.

## Architecture

Each language needs:

1. `{language}_definition.rs` - implements `LanguageDefinition`
2. `{language}_parser.rs` - implements `LanguageParser`
3. `{language}_behavior.rs` - implements `LanguageBehavior`

## File Structure

```
src/parsing/
├── {language}_definition.rs  # LanguageDefinition trait
├── {language}_parser.rs      # LanguageParser trait
├── {language}_behavior.rs    # LanguageBehavior trait
└── registry.rs               # Add registration call
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

1. Add to `src/parsing/registry.rs`: `yourlang_definition::register(registry)`
2. Update `src/parsing/mod.rs`: Add module declarations
3. Add to `Cargo.toml`: `tree-sitter-yourlang = "0.20"`

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

- `src/parsing/rust_definition.rs` - Traits and inherent methods
- `src/parsing/python_definition.rs` - Naming convention visibility
- `src/parsing/php_definition.rs` - Namespace handling
