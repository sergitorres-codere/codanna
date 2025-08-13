# Adding Language Support

The codebase uses a modular language registry system where languages self-register. This architecture makes adding new languages straightforward without modifying core systems.

## Current Implementation Status

- **Rust**: ✅ Fully implemented with production features
- **Python**: ✅ Full implementation with classes and functions
- **PHP**: ✅ Full implementation with namespaces and traits
- **JavaScript**: 📋 Planned for v0.4.1
- **TypeScript**: 📋 Planned for v0.4.1
- **Go**: 📋 Planned for v0.4.2
- **C#**: 📋 Planned for v0.4.3
- **Java**: 📋 Planned for v0.4.4
- **C/C++**: 📋 Planned for v0.4.5

## Architecture Overview

The language system uses a **self-registering registry pattern** where each language:
1. Defines its own module with parser and behavior implementations
2. Implements the `LanguageDefinition` trait
3. Automatically registers itself at startup via `LazyLock`

No manual registration in multiple files required!

## Implementation Guide

### Step 1: Create Language Definition Module

Create a new file `src/parsing/{language}_definition.rs`:

```rust
use std::sync::Arc;
use super::{
    LanguageBehavior, LanguageDefinition, LanguageId, LanguageParser,
    YourLanguageBehavior, YourLanguageParser,
};
use crate::{IndexError, IndexResult, Settings};

/// Your language definition
pub struct YourLanguage;

impl YourLanguage {
    /// Language identifier constant
    pub const ID: LanguageId = LanguageId::new("yourlang");
}

impl LanguageDefinition for YourLanguage {
    fn id(&self) -> LanguageId {
        Self::ID
    }

    fn name(&self) -> &'static str {
        "YourLanguage"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ext", "ext2"]  // Your file extensions
    }

    fn create_parser(&self, _settings: &Settings) -> IndexResult<Box<dyn LanguageParser>> {
        let parser = YourLanguageParser::new()
            .map_err(|e| IndexError::General(e.to_string()))?;
        Ok(Box::new(parser))
    }

    fn create_behavior(&self) -> Box<dyn LanguageBehavior> {
        Box::new(YourLanguageBehavior::new())
    }

    fn default_enabled(&self) -> bool {
        false  // Set to true if language should be enabled by default
    }

    fn is_enabled(&self, settings: &Settings) -> bool {
        settings
            .languages
            .get(self.id().as_str())
            .map(|config| config.enabled)
            .unwrap_or(self.default_enabled())
    }
}

/// Register with the global registry
pub(super) fn register(registry: &mut super::LanguageRegistry) {
    registry.register(Arc::new(YourLanguage));
}
```

### Step 2: Create Language Parser

Create `src/parsing/{language}_parser.rs` implementing the `LanguageParser` trait:

```rust
use tree_sitter::{Parser, Node};
use crate::{Symbol, SymbolKind, FileId, Range};
use super::{LanguageParser, SymbolCounter};

#[derive(Debug)]
pub struct YourLanguageParser {
    parser: Parser,
}

impl YourLanguageParser {
    pub fn new() -> Result<Self, YourParseError> {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_yourlang::language())
            .map_err(|e| YourParseError::ParserInitFailed {
                reason: e.to_string()
            })?;
        Ok(Self { parser })
    }
}

impl LanguageParser for YourLanguageParser {
    fn parse(&mut self, code: &str, file_id: FileId, counter: &mut SymbolCounter) 
        -> Vec<Symbol> 
    {
        // Parse code and extract symbols
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };
        
        let mut symbols = Vec::new();
        // Walk tree and extract symbols...
        symbols
    }

    fn language(&self) -> super::Language {
        super::Language::YourLanguage  // If using legacy enum
    }

    // Implement other required methods...
}
```

### Step 3: Create Language Behavior

Create `src/parsing/{language}_behavior.rs` implementing the `LanguageBehavior` trait:

```rust
use super::LanguageBehavior;
use crate::{Symbol, Visibility};
use tree_sitter::Language;

#[derive(Debug, Clone)]
pub struct YourLanguageBehavior {
    language: Language,  // Tree-sitter language for ABI-15 validation
}

impl YourLanguageBehavior {
    pub fn new() -> Self {
        Self {
            language: tree_sitter_yourlang::language(),
        }
    }
}

impl LanguageBehavior for YourLanguageBehavior {
    fn module_separator(&self) -> &'static str {
        "."  // Common choices: "::" (Rust/C++), "." (Python/Java), "\\" (PHP)
    }

    fn format_module_path(&self, base_path: &str, symbol_name: &str) -> String {
        // Combine base path with symbol name using language conventions
        format!("{}{}{}", base_path, self.module_separator(), symbol_name)
    }

    fn parse_visibility(&self, signature: &str) -> Visibility {
        // Parse visibility from function/class signatures
        // Examples for different languages:
        
        // For explicit keyword languages (Java, C#, PHP):
        if signature.contains("public") {
            Visibility::Public
        } else if signature.contains("protected") {
            Visibility::Protected  
        } else if signature.contains("private") {
            Visibility::Private
        } else {
            Visibility::Module  // Default visibility
        }
        
        // For Python (naming conventions):
        // if symbol_name.starts_with("__") { Visibility::Private }
        // else if symbol_name.starts_with("_") { Visibility::Module }
        // else { Visibility::Public }
        
        // For Go (capitalization):
        // if symbol_name.chars().next().map_or(false, |c| c.is_uppercase()) {
        //     Visibility::Public
        // } else {
        //     Visibility::Private
        // }
    }

    fn supports_traits(&self) -> bool {
        false  // true for Rust, C#, Java (interfaces), PHP (traits)
    }

    fn supports_inherent_methods(&self) -> bool {
        false  // true for Rust, C++ (methods on structs/classes)
    }

    fn get_language(&self) -> Language {
        self.language.clone()
    }

    // Optional: Override configure_symbol for custom behavior
    fn configure_symbol(&self, symbol: &mut Symbol, module_path: Option<&str>) {
        // Default implementation handles module path and visibility
        // Override if you need custom processing
        
        // Apply module path formatting
        if let Some(path) = module_path {
            let full_path = self.format_module_path(path, &symbol.name);
            symbol.module_path = Some(full_path.into());
        }

        // Apply visibility parsing
        if let Some(ref sig) = symbol.signature {
            symbol.visibility = self.parse_visibility(sig);
        }
        
        // Add language-specific processing here if needed
    }
}
```

## Understanding LanguageBehavior

The `LanguageBehavior` trait encapsulates all language-specific conventions and rules that were previously hardcoded in the indexer. This is a key part of making the system truly language-agnostic.

### Core Responsibilities

1. **Module Path Formatting**
   - How to combine module/namespace paths with symbol names
   - Language-specific separators (`::`; `.`, `\`, `/`)
   - Package vs module vs namespace conventions

2. **Visibility Parsing**
   - Extract visibility from signatures or naming conventions
   - Map to universal visibility levels (Public, Protected, Module, Private)
   - Handle language-specific visibility rules

3. **Language Capabilities**
   - Does the language support traits/interfaces?
   - Does it have inherent methods (methods directly on types)?
   - What kinds of inheritance does it support?

4. **Tree-sitter Integration**
   - Provide the language grammar for validation
   - Validate node kinds using ABI-15
   - Enable grammar-aware processing

### Language-Specific Examples

#### Rust Behavior
```rust
fn module_separator(&self) -> &'static str { "::" }
fn supports_traits(&self) -> bool { true }
fn supports_inherent_methods(&self) -> bool { true }
fn parse_visibility(&self, sig: &str) -> Visibility {
    if sig.starts_with("pub ") { Visibility::Public }
    else if sig.starts_with("pub(crate)") { Visibility::Crate }
    else { Visibility::Private }
}
```

#### Python Behavior
```rust
fn module_separator(&self) -> &'static str { "." }
fn parse_visibility(&self, _sig: &str) -> Visibility {
    // Python uses naming conventions, not keywords
    // This would need the symbol name, not signature
    Visibility::Public  // Default for Python
}
```

#### PHP Behavior
```rust
fn module_separator(&self) -> &'static str { "\\" }
fn supports_traits(&self) -> bool { true }  // PHP has traits
fn parse_visibility(&self, sig: &str) -> Visibility {
    // PHP has explicit visibility keywords
    if sig.contains("public") { Visibility::Public }
    else if sig.contains("protected") { Visibility::Protected }
    else if sig.contains("private") { Visibility::Private }
    else { Visibility::Public }  // Default in PHP
}
```

### Step 4: Register in Registry Module

Update `src/parsing/registry.rs` to include your language:

```rust
// In the register_languages() function:
fn register_languages(registry: &mut LanguageRegistry) {
    rust_definition::register(registry);
    python_definition::register(registry);
    php_definition::register(registry);
    yourlang_definition::register(registry);  // ADD THIS
}
```

### Step 5: Update Module Exports

Update `src/parsing/mod.rs`:

```rust
// Add modules
mod yourlang_definition;
mod yourlang_parser;
mod yourlang_behavior;

// Export types
pub use yourlang_parser::YourLanguageParser;
pub use yourlang_behavior::YourLanguageBehavior;
```

### Step 6: Add Tree-sitter Dependency

In `Cargo.toml`:

```toml
[dependencies]
tree-sitter-yourlang = "0.20"  # Use appropriate version
```

## That's It!

The language will now:
- ✅ Automatically appear in `codanna init` generated config
- ✅ Be available for indexing when enabled
- ✅ Work with all CLI commands
- ✅ Support all MCP tools
- ✅ Integrate with semantic search

No need to manually update:
- ❌ ~~factory.rs~~ (registry handles it)
- ❌ ~~walker.rs~~ (registry provides extensions)
- ❌ ~~config.rs~~ (registry provides defaults)
- ❌ ~~main.rs~~ (benchmark can query registry)
- ❌ ~~language.rs enum~~ (being phased out)

## Verification

After implementing your language:

```bash
# 1. Build the project
cargo build --release

# 2. Initialize config (your language should appear)
./target/release/codanna init
grep "yourlang" .codanna/settings.toml

# 3. Enable your language in settings.toml
# [languages.yourlang]
# enabled = true

# 4. Test indexing
echo "your language code" > test.ext
./target/release/codanna index test.ext --progress

# 5. Test retrieval
./target/release/codanna retrieve symbol YourSymbol

# 6. Test benchmark (if implemented)
./target/release/codanna benchmark yourlang
```

## Implementation Requirements

### Parser Performance
- **Target**: >10,000 symbols/second
- **Measure**: AST parsing only (not Tantivy indexing)
- **Test**: Use `codanna benchmark <language>`

### Memory Efficiency
- Use borrowed strings (`&str`) where possible
- Return slices from source code: `&code[node.byte_range()]`
- Use `SymbolCounter` for ID generation (not raw `u32`)

### Error Handling
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum YourParseError {
    #[error("Failed to initialize parser: {reason}\nSuggestion: Check tree-sitter-yourlang version")]
    ParserInitFailed { reason: String },
    
    #[error("Invalid syntax at line {line}\nSuggestion: Check language version compatibility")]
    InvalidSyntax { line: usize },
}
```

### Required Trait Methods

The `LanguageParser` trait requires:

```rust
pub trait LanguageParser: Send + Sync {
    // Core parsing
    fn parse(&mut self, code: &str, file_id: FileId, counter: &mut SymbolCounter) -> Vec<Symbol>;
    
    // Language identification
    fn language(&self) -> Language;
    
    // Documentation extraction
    fn extract_doc_comment(&self, node: &Node, code: &str) -> Option<String>;
    
    // Relationship extraction
    fn find_calls(&mut self, code: &str) -> Vec<SimpleCall>;
    fn find_implementations<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)>;
    fn find_imports(&mut self, code: &str, file_id: FileId) -> Vec<Import>;
    
    // Optional methods (can return empty)
    fn find_inherent_methods(&mut self, code: &str) -> Vec<(String, String, Range)> {
        Vec::new()
    }
    
    fn find_uses<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        Vec::new()
    }
    
    fn find_defines<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        Vec::new()
    }
}
```

## Language-Specific Patterns

### Python
- **Docstrings**: First string literal after function/class definition
- **Module paths**: Use `.` separator
- **Visibility**: Based on naming conventions (`_private`, `__private`)

### PHP
- **Doc blocks**: `/** */` comments before declarations
- **Namespaces**: Use `\` separator
- **Visibility**: Explicit keywords (`public`, `private`, `protected`)

### JavaScript/TypeScript (Future)
- **JSDoc**: `/** */` comments
- **Module paths**: Use `.` or `/` for imports
- **Visibility**: Module exports determine visibility

### Go (Future)
- **Doc comments**: `//` comments before declarations
- **Package paths**: Use `/` separator
- **Visibility**: Capitalization determines export

## Testing

Create comprehensive tests in your parser module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_function() {
        let mut parser = YourLanguageParser::new().unwrap();
        let code = "function example() { }";
        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        
        let symbols = parser.parse(code, file_id, &mut counter);
        
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name.as_ref(), "example");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
    }
    
    #[test]
    fn test_language_registration() {
        use crate::parsing::get_registry;
        
        let registry = get_registry();
        let registry = registry.lock().unwrap();
        
        // Verify language is registered
        assert!(registry.is_available(LanguageId::new("yourlang")));
        
        // Verify extensions are registered
        assert!(registry.get_by_extension("ext").is_some());
    }
}
```

## Benefits of the New Architecture

1. **Self-contained**: Each language is a complete module
2. **No scattered changes**: All language code in one place
3. **Automatic registration**: Languages register themselves at startup
4. **Dynamic configuration**: Settings generated from registry
5. **Easy testing**: Each language module can be tested independently
6. **Type safe**: `LanguageId` newtype prevents string errors
7. **Extensible**: Easy to add new capabilities via trait methods

## Migration from Old System

If updating an existing language from the old manual registration system:

1. Create a `{language}_definition.rs` file with `LanguageDefinition` impl
2. Move parser logic to implement new trait structure
3. Create `{language}_behavior.rs` with language-specific rules
4. Remove manual registrations from:
   - `factory.rs` (create_parser, enabled_languages)
   - `walker.rs` (get_enabled_languages)
   - `config.rs` (default_languages, init_config_file)
   - `main.rs` (benchmark command)
5. Update imports in `mod.rs`
6. Test that language still works

## Support

For examples of complete implementations, see:
- `src/parsing/rust_definition.rs` - Most complete implementation
- `src/parsing/python_definition.rs` - Dynamic language example
- `src/parsing/php_definition.rs` - Namespace-based language example

The registry system handles all the complexity of language discovery, configuration, and integration, allowing you to focus on implementing the parser logic itself.