# Language Parser Interface

The entire relationship extraction system is language-agnostic. The architecture is highly modular, allowing easy addition of new languages while leveraging the sophisticated analysis infrastructure.

## Complete Registration Checklist

**WARNING**: Missing any of these registration points will cause your language to fail in subtle ways. Use this checklist to verify ALL points are covered:

- [ ] Language enum variant added (`src/parsing/language.rs`)
- [ ] Language methods updated (`from_extension`, `extensions`, `config_key`, `name`)
- [ ] Parser implementation created (`src/parsing/{language}.rs`)
- [ ] Parser factory - create_parser (`src/parsing/factory.rs`)
- [ ] Parser factory - enabled_languages list (`src/parsing/factory.rs`)
- [ ] File walker registration (`src/indexing/walker.rs`)
- [ ] CLI benchmark support (`src/main.rs`) 
- [ ] Configuration defaults (`src/config.rs` - `default_languages`)
- [ ] Configuration template (`src/config.rs` - `init_config_file`)

## Current Implementation Status

- **Rust**: ✅ Fully implemented with production-ready features
- **Python**: 🏗️ Infrastructure ready, parser not yet implemented
- **JavaScript**: 🏗️ Infrastructure ready, parser not yet implemented  
- **TypeScript**: 🏗️ Infrastructure ready, parser not yet implemented

## Current API

Each language implements the `LanguageParser` trait as defined in `src/parsing/parser.rs`:

```rust
/// Common interface for all language parsers
pub trait LanguageParser: Send + Sync {
    /// Parse source code and extract symbols
    fn parse(&mut self, code: &str, file_id: FileId, symbol_counter: &mut u32) -> Vec<Symbol>;

    /// Enable downcasting to concrete parser types
    fn as_any(&self) -> &dyn Any;

    /// Extract documentation comment for a node
    fn extract_doc_comment(&self, node: &Node, code: &str) -> Option<String>;

    /// Find function/method calls in the code (legacy method)
    /// Returns borrowed strings to avoid allocations
    fn find_calls<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)>;

    /// Find method calls with rich receiver information (enhanced method)
    /// Default implementation converts from find_calls()
    fn find_method_calls(&mut self, code: &str) -> Vec<MethodCall> {
        // Default implementation converts from legacy find_calls()
    }

    /// Find trait/interface implementations
    fn find_implementations<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)>;

    /// Find type usage (in fields, parameters, returns)
    fn find_uses<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)>;

    /// Find method definitions
    fn find_defines<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)>;

    /// Find import statements
    fn find_imports(&mut self, code: &str, file_id: FileId) -> Vec<crate::indexing::Import>;

    /// Get the language this parser handles
    fn language(&self) -> crate::parsing::Language;

    /// Extract variable bindings with their types (optional)
    fn find_variable_types<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        Vec::new() // Default empty implementation
    }

    /// Enable mutable downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

## Complete Implementation Steps

### Step 1: Add Language Enum Variant
**File**: `src/parsing/language.rs` (lines 10-15)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,  // ADD YOUR LANGUAGE HERE
}
```

### Step 2: Update Language Methods
**File**: `src/parsing/language.rs`
Update ALL match statements in:
- `from_extension()` (lines 19-27) - Map file extensions
- `extensions()` (lines 37-44) - Return default extensions
- `config_key()` (lines 47-54) - Configuration key for settings.toml
- `name()` (lines 57-64) - Display name

### Step 3: Create Parser Implementation
**File**: `src/parsing/{language}.rs`
- Define error types with `thiserror`
- Implement `LanguageParser` trait
- Follow zero-cost abstractions (see guidelines below)

### Step 4: Register in Parser Factory
**File**: `src/parsing/factory.rs`

Add to `create_parser` method (lines 46-71):
```rust
Language::Go => {
    let parser = GoParser::new().map_err(|e| IndexError::General(e.to_string()))?;
    Ok(Box::new(parser))
}
```

### Step 5: Update Factory Enabled Languages List
**File**: `src/parsing/factory.rs` (lines 89-98)

⚠️ **CRITICAL**: Add to hardcoded list in `enabled_languages`:
```rust
vec![
    Language::Rust,
    Language::Python,
    Language::JavaScript,
    Language::TypeScript,
    Language::Go,  // ADD THIS - WITHOUT IT, PARSER WON'T BE AVAILABLE
]
```

### Step 6: Register in File Walker ⚠️ CRITICAL
**File**: `src/indexing/walker.rs` (lines 85-90)

**WARNING**: Missing this step means your language files won't be discovered during indexing!

```rust
fn get_enabled_languages(&self) -> Vec<Language> {
    vec![
        Language::Rust,
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
        Language::Go,  // ADD THIS - WITHOUT IT, FILES WON'T BE INDEXED
    ]
    .into_iter()
    .filter(|&lang| {
        self.settings
            .languages
            .get(lang.config_key())
            .map(|config| config.enabled)
            .unwrap_or(false)
    })
    .collect()
}
```

### Step 7: Add CLI Benchmark Support
**File**: `src/main.rs` (lines 3000-3013)

Add to `run_benchmark_command`:
```rust
match language.to_lowercase().as_str() {
    "rust" => benchmark_rust_parser(custom_file),
    "python" => benchmark_python_parser(custom_file),
    "go" => benchmark_go_parser(custom_file),  // ADD THIS
    "all" => {
        benchmark_rust_parser(None);
        println!();
        benchmark_python_parser(None);
        benchmark_go_parser(None);  // ADD THIS
    }
    _ => {
        eprintln!("Unknown language: {language}");
        eprintln!("Available languages: rust, python, go, all");  // UPDATE THIS
        std::process::exit(1);
    }
}
```

### Step 8: Update Configuration
**File**: `src/config.rs`

Add to `default_languages` function (lines 283-296):
```rust
langs.insert(
    "go".to_string(),
    LanguageConfig {
        enabled: false,
        extensions: vec!["go".to_string()],
        parser_options: HashMap::new(),
    },
);
```

### Step 9: Update Configuration Template
**File**: `src/config.rs` (line 496)

Update the comment in `init_config_file`:
```toml
# Currently supported: Rust, Python, Go  # UPDATE THIS COMMENT
```

## Common Pitfalls and Their Symptoms

| Missing Registration | Symptom | Impact |
|---------------------|---------|--------|
| File Walker (walker.rs) | `codanna index` silently skips your language files | CRITICAL - No indexing |
| Factory enabled_languages | Parser created but never used | CRITICAL - No parsing |
| CLI Benchmark | `codanna benchmark <lang>` shows "Unknown language" | Confusing for testing |
| Configuration template | Users don't know language is supported | Poor UX |

## Verification Steps

After implementing your language, verify it works:

```bash
# 1. Check language is recognized
codanna init
grep "your_language" .codanna/settings.toml

# 2. Enable your language
# Edit .codanna/settings.toml: set enabled = true

# 3. Test indexing discovers files
codanna index . --dry-run | grep "your_extension"

# 4. Test actual indexing
codanna index test_file.ext --progress

# 5. Test benchmark
codanna benchmark your_language

# 6. Test retrieval
codanna retrieve symbol YourSymbol
```

## Key Implementation Notes

**Zero-Cost Abstractions**: The current API follows the project's development guidelines by:

1. **No Allocations**: Methods like `find_calls()` return borrowed `&str` references from source code
2. **Lifetime Parameters**: Relationship methods use `<'a>` to avoid string allocations
3. **Optional Features**: Methods like `find_variable_types()` have default empty implementations
4. **Enhanced Method Calls**: The `MethodCall` struct provides rich metadata for better analysis

**Enhanced Method Call System**: The `find_method_calls()` method returns `MethodCall` structs with:
- Caller and method names
- Receiver information (self, instance, or static)
- Call type metadata (instance vs. static method)
- Source location ranges

**For new language implementations**: Follow the Rust parser as the reference implementation, ensuring zero-cost abstractions where possible.

## Critical Implementation Requirements

**MANDATORY**: Every language parser MUST follow these guidelines:

1. **Error Handling**: Define proper error types (see Python progress doc)
   ```rust
   #[derive(Error, Debug)]
   pub enum GoParseError {
       #[error("Failed to initialize parser: {reason}\nSuggestion: Check tree-sitter-go version in Cargo.toml")]
       ParserInitFailed { reason: String },
   }
   ```

2. **Zero-Cost Abstractions**: Return borrowed data from source
   ```rust
   fn find_calls<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
       // Return slices directly from code using &code[node.byte_range()]
   }
   ```

3. **Type Safety**: Use newtypes for domain concepts
   ```rust
   pub struct NodeId(NonZeroU32);  // NOT raw u32
   ```

4. **Function Design**: Decompose complex operations into focused, composable helper methods
5. **Performance**: Parser must extract >10,000 symbols/second (AST parsing only, not including Tantivy indexing)

### Note on Current Implementation

The Rust parser in `src/parsing/rust.rs` currently uses `String` allocations in some methods due to API stability requirements. New language implementations should follow the zero-cost principles where possible, preparing for future API improvements.

## Testing Your Parser

Create comprehensive tests in `src/parsing/{language}.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function() {
        let mut parser = YourParser::new().unwrap();
        let code = "your language function syntax";
        let file_id = FileId::new(1).unwrap();

        let mut counter = 1u32;
        let symbols = parser.parse(code, file_id, &mut counter);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name.as_ref(), "function_name");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_find_calls() {
        let mut parser = YourParser::new().unwrap();
        let code = "your language call syntax";
        
        let calls = parser.find_calls(code);
        assert!(!calls.is_empty());
    }

    #[test]
    fn test_language_fully_registered() {
        // This test verifies the language is properly registered
        // Note: Some registration points can't be tested from unit tests
        // and require manual verification or integration tests
        
        // 1. Language enum and methods (language.rs)
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert_eq!(Language::Go.config_key(), "go");
        assert_eq!(Language::Go.name(), "Go");
        assert!(Language::Go.extensions().contains(&"go"));
        
        // 2. Parser implements trait correctly
        let mut parser = GoParser::new().unwrap();
        assert_eq!(parser.language(), Language::Go);
        
        // 3. Test actual parsing works
        let code = "package main\nfunc main() {}";
        let file_id = FileId::new(1).unwrap();
        let mut counter = 1u32;
        let symbols = parser.parse(code, file_id, &mut counter);
        assert!(!symbols.is_empty(), "Parser should extract symbols");
        
        // IMPORTANT: The following MUST be verified manually as they're in
        // private modules or require full system integration:
        //
        // ✓ factory.rs: create_parser() has "Language::Go =>" case
        // ✓ factory.rs: enabled_languages() includes Language::Go in vec!
        // ✓ walker.rs: get_enabled_languages() includes Language::Go in vec!
        // ✓ main.rs: run_benchmark_command() has "go" => case
        // ✓ config.rs: default_languages() has "go" entry
        // ✓ config.rs: init_config_file() comment mentions Go
    }
    
    #[test]
    fn verify_language_registration_checklist() {
        // This test helps catch common registration mistakes at compile time
        // If this doesn't compile, you forgot a registration step!
        
        // These should all compile if Language::Go exists
        let _lang = Language::Go;
        let _config_key = Language::Go.config_key();
        let _extensions = Language::Go.extensions();
        
        // This should compile if parser exists
        let parser_result = GoParser::new();
        assert!(parser_result.is_ok(), "GoParser::new() should succeed");
        
        // Manual verification commands to run:
        println!("Run these commands to verify full integration:");
        println!("1. cargo build --release");
        println!("2. ./target/release/codanna init");
        println!("3. grep 'go' .codanna/settings.toml");
        println!("4. ./target/release/codanna benchmark go");
        println!("5. Create test.go and run: ./target/release/codanna index test.go --dry-run");
    }
}
```

## Configuration and Settings

To enable a language, add it to `.codanna/settings.toml`:

```toml
[languages.go]
enabled = true
extensions = ["go"]  # Optional: override default extensions

[languages.python]
enabled = true
extensions = ["py", "pyi"]

[languages.javascript]
enabled = false  # Disabled by default until parser is implemented
```

The system only creates parsers for enabled languages, preventing overhead from unused language support.

## Dependencies

The following tree-sitter dependencies are already included in `Cargo.toml`:
- `tree-sitter-python`
- `tree-sitter-javascript`
- `tree-sitter-typescript`
- `tree-sitter-rust`
- `tree-sitter-go`
- `tree-sitter-java`

For new languages, add the appropriate tree-sitter dependency:
```bash
cargo add tree-sitter-{language}
```

This will automatically add the latest compatible version to your `Cargo.toml`.

## Language-Specific Patterns

### Python Specifics

1. **Docstrings**: First string literal in function/class body
2. **Multiple Inheritance**: `class Dog(Animal, Trainable):`
3. **Import Variations**: 
   - `import foo`
   - `from foo import bar`
   - `from foo import bar as baz`
   - `from foo import *`
4. **Method Calls**: `self.method()`, `obj.method()`, `super().method()`

### TypeScript Specifics (Future)

1. **JSDoc**: `/** */` comments
2. **Multiple Inheritance**: `class Dog extends Animal implements Trainable`
3. **Import Variations**: 
   - `import foo from 'foo'`
   - `import { bar } from 'foo'`
   - `import * as foo from 'foo'`
4. **Optional Chaining**: `obj?.method?.()`

### Go Specifics (Future)

1. **Doc Comments**: `//` comments before declarations
2. **Implicit Interfaces**: No explicit implements
3. **Import Variations**: 
   - `import "foo"`
   - `import . "foo"`
   - `import bar "foo"`
4. **Method Expressions**: `Type.Method`

## Key Integration Points

The language parsers integrate with:

1. **ResolutionContext**: For scope-based symbol resolution
2. **TraitResolver**: For inheritance/implementation tracking  
3. **ImportResolver**: For cross-file symbol resolution
4. **DocumentIndex**: For storage and retrieval with semantic search
5. **MethodCall System**: For rich method call metadata tracking
6. **MCP Server**: For AI assistant integration

All of these systems are language-agnostic and work identically regardless of the source language.

## Current Architecture Benefits

1. **Performance**: Zero-cost abstractions with borrowed strings
2. **Modularity**: Easy to add new languages without changing core systems
3. **Type Safety**: Strong typing with `SymbolId`, `FileId`, and proper error handling
4. **Extensibility**: Optional methods allow gradual feature addition
5. **AI Integration**: Full MCP support for code intelligence queries

## Implementation Examples

For complete, guideline-compliant implementation examples, see:
- **Python Reference**: `/docs/examples/python-parser-reference.md`
- **Progress Tracking**: `/docs/features/python-language-support-progress.md`

The reference implementation demonstrates:
- Proper error handling with `thiserror`
- Zero-cost abstractions with lifetime parameters
- Function decomposition (30-line limit)
- Type safety with newtypes
- Performance optimizations