//! Language-agnostic resolution traits for symbol and inheritance resolution
//!
//! This module provides the trait abstractions that allow each language to implement
//! its own resolution logic while keeping the indexer language-agnostic.

use super::context::ScopeType;
use crate::{FileId, SymbolId};
use std::collections::HashMap;

/// Scope levels that work across all languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScopeLevel {
    /// Function/block scope (local variables, parameters)
    Local,
    /// Module/file scope (module-level definitions)
    Module,
    /// Package/namespace scope (package-level visibility)
    Package,
    /// Global/project scope (public exports)
    Global,
}

/// Language-agnostic resolution scope
///
/// Each language implements this trait to provide its own scoping rules.
/// For example:
/// - Rust: local -> imports -> module -> crate
/// - Python: LEGB (Local, Enclosing, Global, Built-in)
/// - TypeScript: hoisting, namespaces, type vs value space
pub trait ResolutionScope: Send + Sync {
    /// Add a symbol to the scope at the specified level
    fn add_symbol(&mut self, name: String, symbol_id: SymbolId, scope_level: ScopeLevel);

    /// Resolve a symbol name according to language-specific rules
    fn resolve(&self, name: &str) -> Option<SymbolId>;

    /// Clear the local scope (e.g., when exiting a function)
    fn clear_local_scope(&mut self);

    /// Enter a new scope (e.g., entering a function or block)
    fn enter_scope(&mut self, scope_type: ScopeType);

    /// Exit the current scope
    fn exit_scope(&mut self);

    /// Get all symbols currently in scope (for debugging)
    fn symbols_in_scope(&self) -> Vec<(String, SymbolId, ScopeLevel)>;

    /// Get as Any for downcasting (needed for language-specific operations)
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Language-agnostic inheritance resolver
///
/// Each language implements this trait to handle its inheritance model:
/// - Rust: traits and inherent implementations
/// - TypeScript: interfaces and class extension
/// - Python: multiple inheritance with MRO
/// - PHP: traits and interfaces
pub trait InheritanceResolver: Send + Sync {
    /// Add an inheritance relationship
    fn add_inheritance(&mut self, child: String, parent: String, kind: &str);

    /// Resolve which parent provides a method
    fn resolve_method(&self, type_name: &str, method: &str) -> Option<String>;

    /// Get the inheritance chain for a type
    fn get_inheritance_chain(&self, type_name: &str) -> Vec<String>;

    /// Check if one type is a subtype of another
    fn is_subtype(&self, child: &str, parent: &str) -> bool;

    /// Add methods that a type defines
    fn add_type_methods(&mut self, type_name: String, methods: Vec<String>);

    /// Get all methods available on a type (including inherited)
    fn get_all_methods(&self, type_name: &str) -> Vec<String>;
}

/// Generic resolution context that wraps the existing ResolutionContext
///
/// This provides a default implementation that maintains backward compatibility
/// while allowing languages to override with their own logic.
pub struct GenericResolutionContext {
    #[allow(dead_code)]
    file_id: FileId, // Kept for future use when we need file-specific resolution
    symbols: HashMap<ScopeLevel, HashMap<String, SymbolId>>,
    scope_stack: Vec<ScopeType>,
}

impl GenericResolutionContext {
    /// Create a new generic resolution context
    pub fn new(file_id: FileId) -> Self {
        let mut symbols = HashMap::new();
        symbols.insert(ScopeLevel::Local, HashMap::new());
        symbols.insert(ScopeLevel::Module, HashMap::new());
        symbols.insert(ScopeLevel::Package, HashMap::new());
        symbols.insert(ScopeLevel::Global, HashMap::new());

        Self {
            file_id,
            symbols,
            scope_stack: vec![ScopeType::Global],
        }
    }

    /// Wrap an existing ResolutionContext (for migration)
    pub fn from_existing(file_id: FileId) -> Self {
        Self::new(file_id)
    }
}

impl ResolutionScope for GenericResolutionContext {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn add_symbol(&mut self, name: String, symbol_id: SymbolId, scope_level: ScopeLevel) {
        self.symbols
            .entry(scope_level)
            .or_default()
            .insert(name, symbol_id);
    }

    fn resolve(&self, name: &str) -> Option<SymbolId> {
        // Check scopes in order: Local -> Module -> Package -> Global
        for level in &[
            ScopeLevel::Local,
            ScopeLevel::Module,
            ScopeLevel::Package,
            ScopeLevel::Global,
        ] {
            if let Some(symbols) = self.symbols.get(level) {
                if let Some(&id) = symbols.get(name) {
                    return Some(id);
                }
            }
        }
        None
    }

    fn clear_local_scope(&mut self) {
        if let Some(local) = self.symbols.get_mut(&ScopeLevel::Local) {
            local.clear();
        }
    }

    fn enter_scope(&mut self, scope_type: ScopeType) {
        self.scope_stack.push(scope_type);
    }

    fn exit_scope(&mut self) {
        self.scope_stack.pop();
        // Clear local scope when exiting a function
        if matches!(self.scope_stack.last(), Some(ScopeType::Function { .. })) {
            self.clear_local_scope();
        }
    }

    fn symbols_in_scope(&self) -> Vec<(String, SymbolId, ScopeLevel)> {
        let mut result = Vec::new();
        for (&level, symbols) in &self.symbols {
            for (name, &id) in symbols {
                result.push((name.clone(), id, level));
            }
        }
        result
    }
}

/// Generic inheritance resolver that provides default implementation
///
/// This wraps existing trait resolution logic while allowing languages
/// to provide their own inheritance semantics.
pub struct GenericInheritanceResolver {
    /// Maps child to parent relationships
    inheritance: HashMap<String, Vec<(String, String)>>, // (parent, kind)
    /// Maps types to their methods
    type_methods: HashMap<String, Vec<String>>,
}

impl GenericInheritanceResolver {
    /// Create a new generic inheritance resolver
    pub fn new() -> Self {
        Self {
            inheritance: HashMap::new(),
            type_methods: HashMap::new(),
        }
    }
}

impl Default for GenericInheritanceResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl InheritanceResolver for GenericInheritanceResolver {
    fn add_inheritance(&mut self, child: String, parent: String, kind: &str) {
        self.inheritance
            .entry(child)
            .or_default()
            .push((parent, kind.to_string()));
    }

    fn resolve_method(&self, type_name: &str, method: &str) -> Option<String> {
        // First check if the type has the method directly
        if let Some(methods) = self.type_methods.get(type_name) {
            if methods.contains(&method.to_string()) {
                return Some(type_name.to_string());
            }
        }

        // Then check parent types
        if let Some(parents) = self.inheritance.get(type_name) {
            for (parent, _kind) in parents {
                if let Some(result) = self.resolve_method(parent, method) {
                    return Some(result);
                }
            }
        }

        None
    }

    fn get_inheritance_chain(&self, type_name: &str) -> Vec<String> {
        let mut chain = vec![type_name.to_string()];
        let mut visited = std::collections::HashSet::new();
        visited.insert(type_name.to_string());

        let mut to_visit = vec![type_name.to_string()];

        while let Some(current) = to_visit.pop() {
            if let Some(parents) = self.inheritance.get(&current) {
                for (parent, _kind) in parents {
                    if visited.insert(parent.clone()) {
                        chain.push(parent.clone());
                        to_visit.push(parent.clone());
                    }
                }
            }
        }

        chain
    }

    fn is_subtype(&self, child: &str, parent: &str) -> bool {
        if child == parent {
            return true;
        }

        let chain = self.get_inheritance_chain(child);
        chain.contains(&parent.to_string())
    }

    fn add_type_methods(&mut self, type_name: String, methods: Vec<String>) {
        self.type_methods.insert(type_name, methods);
    }

    fn get_all_methods(&self, type_name: &str) -> Vec<String> {
        let mut methods = Vec::new();
        let chain = self.get_inheritance_chain(type_name);

        for ancestor in chain {
            if let Some(type_methods) = self.type_methods.get(&ancestor) {
                for method in type_methods {
                    if !methods.contains(method) {
                        methods.push(method.clone());
                    }
                }
            }
        }

        methods
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_resolution_context() {
        let mut ctx = GenericResolutionContext::new(FileId::new(1).unwrap());

        // Add symbols at different scope levels
        ctx.add_symbol(
            "local_var".to_string(),
            SymbolId::new(1).unwrap(),
            ScopeLevel::Local,
        );
        ctx.add_symbol(
            "module_fn".to_string(),
            SymbolId::new(2).unwrap(),
            ScopeLevel::Module,
        );
        ctx.add_symbol(
            "global_type".to_string(),
            SymbolId::new(3).unwrap(),
            ScopeLevel::Global,
        );

        // Test resolution order
        assert_eq!(ctx.resolve("local_var"), Some(SymbolId::new(1).unwrap()));
        assert_eq!(ctx.resolve("module_fn"), Some(SymbolId::new(2).unwrap()));
        assert_eq!(ctx.resolve("global_type"), Some(SymbolId::new(3).unwrap()));
        assert_eq!(ctx.resolve("unknown"), None);

        // Test scope clearing
        ctx.clear_local_scope();
        assert_eq!(ctx.resolve("local_var"), None);
        assert_eq!(ctx.resolve("module_fn"), Some(SymbolId::new(2).unwrap()));
    }

    #[test]
    fn test_generic_inheritance_resolver() {
        let mut resolver = GenericInheritanceResolver::new();

        // Set up a simple inheritance hierarchy
        resolver.add_inheritance("Child".to_string(), "Parent".to_string(), "extends");
        resolver.add_inheritance("Parent".to_string(), "GrandParent".to_string(), "extends");

        // Add methods
        resolver.add_type_methods("GrandParent".to_string(), vec!["method1".to_string()]);
        resolver.add_type_methods("Parent".to_string(), vec!["method2".to_string()]);
        resolver.add_type_methods("Child".to_string(), vec!["method3".to_string()]);

        // Test method resolution
        assert_eq!(
            resolver.resolve_method("Child", "method3"),
            Some("Child".to_string())
        );
        assert_eq!(
            resolver.resolve_method("Child", "method2"),
            Some("Parent".to_string())
        );
        assert_eq!(
            resolver.resolve_method("Child", "method1"),
            Some("GrandParent".to_string())
        );

        // Test inheritance chain
        let chain = resolver.get_inheritance_chain("Child");
        assert!(chain.contains(&"Child".to_string()));
        assert!(chain.contains(&"Parent".to_string()));
        assert!(chain.contains(&"GrandParent".to_string()));

        // Test subtype checking
        assert!(resolver.is_subtype("Child", "Parent"));
        assert!(resolver.is_subtype("Child", "GrandParent"));
        assert!(!resolver.is_subtype("Parent", "Child"));
    }
}
