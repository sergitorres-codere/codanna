//! Language-specific behavior abstraction
//!
//! This module provides the LanguageBehavior trait which encapsulates
//! all language-specific logic that was previously hardcoded in SimpleIndexer.
//! Each language implements this trait to define its specific conventions.
//!
//! # Architecture
//!
//! The LanguageBehavior trait is part of a larger refactoring to achieve true
//! language modularity in the codanna indexing system. It works in conjunction
//! with:
//!
//! - `LanguageParser`: Handles AST parsing for each language
//! - `ParserFactory`: Creates parser-behavior pairs
//! - `SimpleIndexer`: Uses behaviors to process symbols without language-specific code
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use codanna::parsing::{ParserFactory, Language};
//!
//! let factory = ParserFactory::new(settings);
//! let pair = factory.create_parser_with_behavior(Language::Rust)?;
//!
//! // Parse code with the parser
//! let symbols = pair.parser.parse(path, content, &mut counter)?;
//!
//! // Process symbols with the behavior
//! for mut symbol in symbols {
//!     pair.behavior.configure_symbol(&mut symbol, Some("crate::module"));
//! }
//! ```
//!
//! # Implementing a New Language
//!
//! To add support for a new language:
//!
//! 1. Create a parser implementing `LanguageParser`
//! 2. Create a behavior implementing `LanguageBehavior`
//! 3. Register both in `ParserFactory`
//! 4. (Future) Register in the language registry for auto-discovery

use crate::{Symbol, Visibility};
use tree_sitter::Language;

/// Trait for language-specific behavior and configuration
///
/// This trait extracts all language-specific logic from the indexer,
/// making the system truly language-agnostic. Each language parser
/// is paired with a behavior implementation that knows how to:
/// - Format module paths according to language conventions
/// - Parse visibility from signatures
/// - Validate node types using tree-sitter metadata
///
/// # Design Principles
///
/// 1. **Zero allocation where possible**: Methods return static strings or reuse inputs
/// 2. **Language agnostic core**: The indexer should never check language types
/// 3. **Extensible**: New languages can be added without modifying existing code
/// 4. **Type safe**: Use tree-sitter's ABI-15 for compile-time validation
pub trait LanguageBehavior: Send + Sync {
    /// Format a module path according to language conventions
    /// 
    /// # Examples
    /// - Rust: `"crate::module::submodule"`
    /// - Python: `"module.submodule"`
    /// - PHP: `"\\Namespace\\Subnamespace"`
    fn format_module_path(&self, base_path: &str, symbol_name: &str) -> String;
    
    /// Parse visibility from a symbol's signature
    ///
    /// # Examples
    /// - Rust: `"pub fn foo()"` -> Public
    /// - Python: `"def _foo()"` -> Module (single underscore)
    /// - PHP: `"private function foo()"` -> Private
    fn parse_visibility(&self, signature: &str) -> Visibility;
    
    /// Get the module separator for this language
    ///
    /// # Examples
    /// - Rust: `"::"`
    /// - Python: `"."`
    /// - PHP: `"\\"`
    fn module_separator(&self) -> &'static str;
    
    /// Check if this language supports trait/interface concepts
    fn supports_traits(&self) -> bool {
        false
    }
    
    /// Check if this language supports inherent methods
    /// (methods defined directly on types, not through traits)
    fn supports_inherent_methods(&self) -> bool {
        false
    }
    
    /// Get the tree-sitter Language for ABI-15 metadata access
    fn get_language(&self) -> Language;
    
    /// Get the Language enum type for this behavior
    /// TODO: This is temporary - will be removed in Stage 3.4
    fn get_language_type(&self) -> crate::parsing::Language;
    
    /// Validate that a node kind exists in this language's grammar
    /// Uses ABI-15 to check if the node type is valid
    fn validate_node_kind(&self, node_kind: &str) -> bool {
        self.get_language().id_for_node_kind(node_kind, true) != 0
    }
    
    /// Get the ABI version of the language grammar
    fn get_abi_version(&self) -> usize {
        self.get_language().abi_version()
    }
    
    /// Configure a symbol with language-specific rules
    /// 
    /// This is the main entry point for applying language-specific
    /// configuration to a symbol during indexing.
    fn configure_symbol(&self, symbol: &mut Symbol, module_path: Option<&str>) {
        // Apply module path formatting
        if let Some(path) = module_path {
            let full_path = self.format_module_path(path, &symbol.name);
            symbol.module_path = Some(full_path.into());
        }
        
        // Apply visibility parsing
        if let Some(ref sig) = symbol.signature {
            symbol.visibility = self.parse_visibility(sig);
        }
    }
}

/// Language metadata from ABI-15
#[derive(Debug, Clone)]
pub struct LanguageMetadata {
    pub abi_version: usize,
    pub node_kind_count: usize,
    pub field_count: usize,
}

impl LanguageMetadata {
    /// Create metadata from a tree-sitter Language
    pub fn from_language(language: Language) -> Self {
        Self {
            abi_version: language.abi_version(),
            node_kind_count: language.node_kind_count(),
            field_count: language.field_count(),
        }
    }
}