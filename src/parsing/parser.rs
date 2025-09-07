//! Language parser trait
//!
//! This module defines the common interface that all language parsers
//! must implement to work with the indexing system.

use crate::parsing::method_call::MethodCall;
use crate::types::SymbolCounter;
use crate::{FileId, Range, Symbol};
use std::any::Any;
use std::collections::HashSet;
use tree_sitter::Node;

/// Common interface for all language parsers
pub trait LanguageParser: Send + Sync {
    /// Parse source code and extract symbols
    fn parse(
        &mut self,
        code: &str,
        file_id: FileId,
        symbol_counter: &mut SymbolCounter,
    ) -> Vec<Symbol>;

    /// Enable downcasting to concrete parser types
    fn as_any(&self) -> &dyn Any;

    /// Extract documentation comment for a node
    ///
    /// Each language has its own documentation conventions:
    /// - Rust: `///` and `/** */`
    /// - Python: Docstrings (first string literal)
    /// - JavaScript/TypeScript: JSDoc `/** */`
    fn extract_doc_comment(&self, node: &Node, code: &str) -> Option<String>;

    /// Find function/method calls in the code
    ///
    /// Returns tuples of (caller_name, callee_name, range)
    /// Zero-cost: Returns string slices into the source code
    fn find_calls<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)>;

    /// Find method calls with rich receiver information
    ///
    /// Default implementation converts from find_calls() for backward compatibility.
    /// Parsers can override this method to provide enhanced receiver tracking.
    ///
    /// # Returns
    ///
    /// A vector of MethodCall structs with structured receiver information
    fn find_method_calls(&mut self, code: &str) -> Vec<MethodCall> {
        self.find_calls(code)
            .into_iter()
            .map(|(caller, target, range)| MethodCall::from_legacy_format(caller, target, range))
            .collect()
    }

    /// Find trait/interface implementations
    ///
    /// Returns tuples of (type_name, trait_name, range)
    /// Zero-cost: Returns string slices into the source code
    fn find_implementations<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)>;

    /// Find inheritance relationships (extends for classes/interfaces)
    ///
    /// Returns tuples of (derived_type, base_type, range)
    /// Zero-cost: Returns string slices into the source code
    fn find_extends<'a>(&mut self, _code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        // Default implementation returns empty for languages without inheritance
        Vec::new()
    }

    /// Find type usage (in fields, parameters, returns)
    ///
    /// Returns tuples of (context_name, used_type, range)
    /// Zero-cost: Returns string slices into the source code
    fn find_uses<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)>;

    /// Find method definitions (in traits/interfaces or types)
    ///
    /// Returns tuples of (definer_name, method_name, range)
    /// Zero-cost: Returns string slices into the source code
    fn find_defines<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)>;

    /// Find import statements in the code
    ///
    /// Returns Import structs with path, alias, and glob information
    fn find_imports(&mut self, code: &str, file_id: FileId) -> Vec<crate::parsing::Import>;

    /// Get the language this parser handles
    fn language(&self) -> crate::parsing::Language;

    /// Extract variable bindings with their types
    /// Returns tuples of (variable_name, type_name, range)
    /// Zero-cost: Returns string slices into the source code
    fn find_variable_types<'a>(&mut self, _code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        // Default implementation returns empty - languages can override
        Vec::new()
    }

    /// Find inherent methods (methods defined directly on types)
    /// Returns tuples of (type_name, method_name, range)
    ///
    /// This is for methods defined directly on types (not through traits/interfaces).
    /// Default implementation returns empty - languages can override.
    ///
    /// Note: Returns owned strings to support complex type names that need construction
    /// (e.g., Rust's `Option<String>`, `Vec<T>`, etc.)
    fn find_inherent_methods(&mut self, _code: &str) -> Vec<(String, String, Range)> {
        Vec::new()
    }
}

/// Trait for creating language parsers
pub trait ParserFactory: Send + Sync {
    /// Create a new parser instance
    fn create(&self) -> Result<Box<dyn LanguageParser>, String>;
}

/// Information about a handled AST node
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HandledNode {
    pub name: String,
    pub id: u16,
}

/// Extension trait for tracking which AST node types a parser handles
///
/// This enables dynamic audit reporting by automatically tracking which
/// node types receive explicit handling during parsing. Eliminates the
/// need for manually maintaining static lists of implemented nodes.
pub trait NodeTracker {
    /// Get the set of node types this parser has encountered and handled
    fn get_handled_nodes(&self) -> &HashSet<HandledNode>;

    /// Register that we've handled a specific node type with its tree-sitter ID
    fn register_handled_node(&mut self, node_kind: &str, node_id: u16);
}

/// Default implementation of NodeTracker using a HashSet
///
/// Parsers can include this struct and delegate to it for zero-cost node tracking.
#[derive(Debug, Default)]
pub struct NodeTrackingState {
    handled_nodes: HashSet<HandledNode>,
}

impl NodeTrackingState {
    /// Create a new empty tracking state
    pub fn new() -> Self {
        Self {
            handled_nodes: HashSet::new(),
        }
    }
}

impl NodeTracker for NodeTrackingState {
    fn get_handled_nodes(&self) -> &HashSet<HandledNode> {
        &self.handled_nodes
    }

    #[inline]
    fn register_handled_node(&mut self, node_kind: &str, node_id: u16) {
        // Create node info - only allocates if we haven't seen this exact node before
        let node_info = HandledNode {
            name: node_kind.to_string(),
            id: node_id,
        };

        // HashSet::insert is efficient - only stores if not already present
        self.handled_nodes.insert(node_info);
    }
}
