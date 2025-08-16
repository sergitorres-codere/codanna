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

use crate::parsing::resolution::{
    GenericInheritanceResolver, GenericResolutionContext, InheritanceResolver, ResolutionScope,
    ScopeLevel,
};
use crate::relationship::RelationKind;
use crate::storage::DocumentIndex;
use crate::{FileId, IndexError, IndexResult, Symbol, SymbolId, Visibility};
use std::path::{Path, PathBuf};
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

    /// Calculate the module path from a file path according to language conventions
    ///
    /// This method converts a file system path to a language-specific module path.
    /// Each language has different conventions for how file paths map to module/namespace paths.
    ///
    /// # Examples
    /// - Rust: `"src/foo/bar.rs"` → `"crate::foo::bar"`
    /// - Python: `"src/package/module.py"` → `"package.module"`
    /// - PHP: `"src/Namespace/Class.php"` → `"\\Namespace\\Class"`
    ///
    /// # Default Implementation
    /// Returns None by default. Languages should override this if they have
    /// specific module path conventions.
    fn module_path_from_file(&self, _file_path: &Path, _project_root: &Path) -> Option<String> {
        None
    }

    /// Resolve an import path to a symbol ID using language-specific conventions
    ///
    /// This method handles the language-specific logic for resolving import paths
    /// to actual symbols in the index. Each language has different import semantics
    /// and path formats.
    ///
    /// # Examples
    /// - Rust: `"crate::foo::Bar"` → looks for Bar in module crate::foo
    /// - Python: `"package.module.Class"` → looks for Class in package.module
    /// - PHP: `"\\App\\Controllers\\UserController"` → looks for UserController in \\App\\Controllers
    ///
    /// # Default Implementation
    /// The default implementation:
    /// 1. Splits the path using the language's module separator
    /// 2. Extracts the symbol name (last segment)
    /// 3. Searches for symbols with that name
    /// 4. Matches against the full module path
    fn resolve_import_path(
        &self,
        import_path: &str,
        document_index: &DocumentIndex,
    ) -> Option<SymbolId> {
        // Split the path using this language's separator
        let separator = self.module_separator();
        let segments: Vec<&str> = import_path.split(separator).collect();

        if segments.is_empty() {
            return None;
        }

        // The symbol name is the last segment
        let symbol_name = segments.last()?;

        // Find symbols with this name
        let candidates = document_index.find_symbols_by_name(symbol_name).ok()?;

        // Find the one with matching full module path
        for candidate in &candidates {
            if let Some(module_path) = &candidate.module_path {
                if module_path.as_ref() == import_path {
                    return Some(candidate.id);
                }
            }
        }

        None
    }

    // ========== New Resolution Methods (v0.4.1) ==========

    /// Create a language-specific resolution context
    ///
    /// Returns a resolution scope that implements the language's scoping rules.
    /// Default implementation returns a generic context that works for most languages.
    fn create_resolution_context(&self, file_id: FileId) -> Box<dyn ResolutionScope> {
        Box::new(GenericResolutionContext::new(file_id))
    }

    /// Create a language-specific inheritance resolver
    ///
    /// Returns an inheritance resolver that handles the language's inheritance model.
    /// Default implementation returns a generic resolver.
    fn create_inheritance_resolver(&self) -> Box<dyn InheritanceResolver> {
        Box::new(GenericInheritanceResolver::new())
    }

    /// Add an import to the language's import tracking
    ///
    /// Default implementation is a no-op. Languages should override to track imports.
    fn add_import(&self, _import: crate::indexing::Import) {
        // Default: no-op
    }

    /// Register a file with its module path
    ///
    /// Default implementation is a no-op. Languages should override to track files.
    fn register_file(&self, _path: PathBuf, _file_id: FileId, _module_path: String) {
        // Default: no-op
    }

    /// Resolve a symbol using language-specific resolution rules
    ///
    /// Default implementation delegates to the resolution context.
    fn resolve_symbol(
        &self,
        name: &str,
        context: &dyn ResolutionScope,
        _document_index: &DocumentIndex,
    ) -> Option<SymbolId> {
        context.resolve(name)
    }

    /// Add a trait/interface implementation
    ///
    /// Default implementation is a no-op. Languages with traits/interfaces should override.
    fn add_trait_impl(&self, _type_name: String, _trait_name: String, _file_id: FileId) {
        // Default: no-op for languages without traits
    }

    /// Add inherent methods for a type
    ///
    /// Default implementation is a no-op. Languages with inherent methods should override.
    fn add_inherent_methods(&self, _type_name: String, _methods: Vec<String>) {
        // Default: no-op for languages without inherent methods
    }

    /// Add methods that a trait/interface defines
    ///
    /// Default implementation is a no-op. Languages with traits/interfaces should override.
    fn add_trait_methods(&self, _trait_name: String, _methods: Vec<String>) {
        // Default: no-op
    }

    /// Resolve which trait/interface provides a method
    ///
    /// Returns the trait/interface name if the method comes from one, None if inherent.
    fn resolve_method_trait(&self, _type_name: &str, _method: &str) -> Option<&str> {
        None
    }

    /// Format a method call for this language
    ///
    /// Default uses the module separator (e.g., Type::method for Rust, Type.method for others)
    fn format_method_call(&self, receiver: &str, method: &str) -> String {
        format!("{}{}{}", receiver, self.module_separator(), method)
    }

    /// Get the inheritance relationship name for this language
    ///
    /// Returns "implements" for languages with interfaces, "extends" for inheritance.
    fn inheritance_relation_name(&self) -> &'static str {
        if self.supports_traits() {
            "implements"
        } else {
            "extends"
        }
    }

    /// Map language-specific relationship to generic RelationKind
    ///
    /// Allows languages to define how their concepts map to the generic relationship types.
    fn map_relationship(&self, language_specific: &str) -> RelationKind {
        match language_specific {
            "extends" => RelationKind::Extends,
            "implements" => RelationKind::Implements,
            "inherits" => RelationKind::Extends,
            "uses" => RelationKind::Uses,
            "calls" => RelationKind::Calls,
            "defines" => RelationKind::Defines,
            _ => RelationKind::References,
        }
    }

    /// Build a complete resolution context for a file
    ///
    /// This is the main entry point for resolution context creation.
    /// This language-agnostic implementation:
    /// 1. Adds imports tracked by the behavior (not ImportResolver)
    /// 2. Adds resolvable symbols from the current file
    /// 3. Adds visible symbols from other files
    ///
    /// Each language controls behavior through its overrides of:
    /// - `get_imports_for_file()` - what imports are available
    /// - `resolve_import()` - how imports resolve to symbols
    /// - `is_resolvable_symbol()` - what symbols can be resolved
    /// - `is_symbol_visible_from_file()` - cross-file visibility rules
    fn build_resolution_context(
        &self,
        file_id: FileId,
        document_index: &DocumentIndex,
    ) -> IndexResult<Box<dyn ResolutionScope>> {
        // Create language-specific resolution context
        let mut context = self.create_resolution_context(file_id);

        // 1. Add imported symbols (using behavior's tracked imports, not ImportResolver)
        let imports = self.get_imports_for_file(file_id);
        for import in imports {
            if let Some(symbol_id) = self.resolve_import(&import, document_index) {
                // Use alias if provided, otherwise use the last segment of the path
                let name = if let Some(alias) = &import.alias {
                    alias.clone()
                } else {
                    // Let the language determine the separator
                    import
                        .path
                        .split(self.module_separator())
                        .last()
                        .unwrap_or(&import.path)
                        .to_string()
                };

                // Add as imported symbol (higher priority than module symbols)
                context.add_symbol(name, symbol_id, ScopeLevel::Module);
            }
        }

        // 2. Add file's module-level symbols
        let file_symbols =
            document_index
                .find_symbols_by_file(file_id)
                .map_err(|e| IndexError::TantivyError {
                    operation: "find_symbols_by_file".to_string(),
                    cause: e.to_string(),
                })?;

        for symbol in file_symbols {
            if self.is_resolvable_symbol(&symbol) {
                context.add_symbol(symbol.name.to_string(), symbol.id, ScopeLevel::Module);
            }
        }

        // 3. Add visible symbols from other files (public/exported symbols)
        // Note: This is expensive, so we limit to a reasonable number
        let all_symbols =
            document_index
                .get_all_symbols(10000)
                .map_err(|e| IndexError::TantivyError {
                    operation: "get_all_symbols".to_string(),
                    cause: e.to_string(),
                })?;

        for symbol in all_symbols {
            // Skip symbols from the current file (already added above)
            if symbol.file_id == file_id {
                continue;
            }

            // Check if this symbol is visible from the current file
            if self.is_symbol_visible_from_file(&symbol, file_id) {
                // Add as global symbol (lower priority)
                context.add_symbol(symbol.name.to_string(), symbol.id, ScopeLevel::Global);
            }
        }

        Ok(context)
    }

    /// Check if a symbol should be resolvable (added to resolution context)
    ///
    /// Languages override this to filter which symbols are available for resolution.
    /// For example, local variables might not be resolvable from other scopes.
    ///
    /// Default implementation includes common top-level symbols.
    fn is_resolvable_symbol(&self, symbol: &Symbol) -> bool {
        use crate::SymbolKind;

        // Check scope_context first if available
        if let Some(scope_context) = symbol.scope_context {
            use crate::symbol::ScopeContext;
            match scope_context {
                ScopeContext::Module | ScopeContext::Global | ScopeContext::Package => true,
                ScopeContext::Local { .. } | ScopeContext::Parameter => false,
                ScopeContext::ClassMember => {
                    // Class members might be resolvable depending on visibility
                    matches!(symbol.visibility, Visibility::Public)
                }
            }
        } else {
            // Fallback to symbol kind for backward compatibility
            matches!(
                symbol.kind,
                SymbolKind::Function
                    | SymbolKind::Method
                    | SymbolKind::Struct
                    | SymbolKind::Trait
                    | SymbolKind::Interface
                    | SymbolKind::Class
                    | SymbolKind::TypeAlias
                    | SymbolKind::Enum
                    | SymbolKind::Constant
            )
        }
    }

    /// Check if a symbol is visible from another file
    ///
    /// Languages implement their visibility rules here.
    /// For example, Rust checks pub, Python might check __all__, etc.
    ///
    /// Default implementation checks basic visibility.
    fn is_symbol_visible_from_file(&self, symbol: &Symbol, from_file: FileId) -> bool {
        // Same file: always visible
        if symbol.file_id == from_file {
            return true;
        }

        // Different file: check visibility
        matches!(symbol.visibility, Visibility::Public)
    }

    /// Get imports for a file
    ///
    /// Returns the list of imports that were registered for this file.
    /// Languages should track imports when add_import() is called.
    ///
    /// Default implementation returns empty (no imports).
    fn get_imports_for_file(&self, _file_id: FileId) -> Vec<crate::indexing::Import> {
        Vec::new()
    }

    /// Resolve an import to a symbol ID
    ///
    /// Takes an import and resolves it to an actual symbol in the index.
    /// Languages implement their specific import resolution logic here.
    ///
    /// Default implementation tries basic name matching.
    fn resolve_import(
        &self,
        import: &crate::indexing::Import,
        document_index: &DocumentIndex,
    ) -> Option<SymbolId> {
        // Default: Try to resolve by exact name match
        // Languages should override with proper import resolution
        self.resolve_import_path(&import.path, document_index)
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
