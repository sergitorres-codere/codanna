//! Rust-specific resolution and inheritance implementation
//!
//! This module migrates the hardcoded Rust logic from:
//! - src/indexing/trait_resolver.rs → RustTraitResolver
//! - src/indexing/resolver.rs → RustResolutionContext
//! - src/indexing/resolution_context.rs → RustResolutionContext

use crate::parsing::{InheritanceResolver, ResolutionScope, ScopeLevel, ScopeType};
use crate::{FileId, SymbolId};
use std::collections::HashMap;

/// Rust-specific resolution context implementing proper scoping rules
///
/// Rust has a specific resolution order:
/// 1. Local scope (variables, parameters)
/// 2. Imported symbols (use statements)
/// 3. Module symbols (items in current module)
/// 4. Crate symbols (pub items from crate root)
pub struct RustResolutionContext {
    #[allow(dead_code)]
    file_id: FileId,  // Will be used in Sprint 4 for file-specific resolution
    
    /// Local variables and parameters in current scope
    local_scope: HashMap<String, SymbolId>,
    
    /// Symbols imported via use statements
    imported_symbols: HashMap<String, SymbolId>,
    
    /// Symbols defined at module level in current file
    module_symbols: HashMap<String, SymbolId>,
    
    /// Public symbols visible from the crate
    crate_symbols: HashMap<String, SymbolId>,
    
    /// Track nested scopes (functions, blocks, etc.)
    scope_stack: Vec<ScopeType>,
    
    /// Import tracking (path -> alias)
    imports: Vec<(String, Option<String>)>,
}

impl RustResolutionContext {
    pub fn new(file_id: FileId) -> Self {
        Self {
            file_id,
            local_scope: HashMap::new(),
            imported_symbols: HashMap::new(),
            module_symbols: HashMap::new(),
            crate_symbols: HashMap::new(),
            scope_stack: Vec::new(),
            imports: Vec::new(),
        }
    }
    
    /// Add an import (use statement)
    pub fn add_import(&mut self, path: String, alias: Option<String>) {
        self.imports.push((path, alias));
    }
}

impl ResolutionScope for RustResolutionContext {
    fn add_symbol(&mut self, name: String, symbol_id: SymbolId, scope_level: ScopeLevel) {
        match scope_level {
            ScopeLevel::Local => {
                self.local_scope.insert(name, symbol_id);
            }
            ScopeLevel::Module => {
                self.module_symbols.insert(name, symbol_id);
            }
            ScopeLevel::Package => {
                // In Rust, Package level maps to imported symbols
                self.imported_symbols.insert(name, symbol_id);
            }
            ScopeLevel::Global => {
                // In Rust, Global maps to crate-level symbols
                self.crate_symbols.insert(name, symbol_id);
            }
        }
    }
    
    fn resolve(&self, name: &str) -> Option<SymbolId> {
        // Rust resolution order: local → imported → module → crate
        
        // 1. Check local scope
        if let Some(&id) = self.local_scope.get(name) {
            return Some(id);
        }
        
        // 2. Check imported symbols
        if let Some(&id) = self.imported_symbols.get(name) {
            return Some(id);
        }
        
        // 3. Check module-level symbols
        if let Some(&id) = self.module_symbols.get(name) {
            return Some(id);
        }
        
        // 4. Check crate-level symbols
        if let Some(&id) = self.crate_symbols.get(name) {
            return Some(id);
        }
        
        // 5. Check if it's a path (contains ::)
        if name.contains("::") {
            // This would need DocumentIndex access to resolve
            // For now, return None (will be handled by behavior)
            return None;
        }
        
        None
    }
    
    fn clear_local_scope(&mut self) {
        self.local_scope.clear();
    }
    
    fn enter_scope(&mut self, scope_type: ScopeType) {
        self.scope_stack.push(scope_type);
        // Rust doesn't hoist, so entering a scope doesn't affect resolution
    }
    
    fn exit_scope(&mut self) {
        self.scope_stack.pop();
        // Clear locals when exiting function scope
        if matches!(
            self.scope_stack.last(),
            None | Some(ScopeType::Module | ScopeType::Global)
        ) {
            self.clear_local_scope();
        }
    }
    
    fn symbols_in_scope(&self) -> Vec<(String, SymbolId, ScopeLevel)> {
        let mut symbols = Vec::new();
        
        // Add all symbols with their appropriate scope levels
        for (name, &id) in &self.local_scope {
            symbols.push((name.clone(), id, ScopeLevel::Local));
        }
        for (name, &id) in &self.imported_symbols {
            symbols.push((name.clone(), id, ScopeLevel::Package));
        }
        for (name, &id) in &self.module_symbols {
            symbols.push((name.clone(), id, ScopeLevel::Module));
        }
        for (name, &id) in &self.crate_symbols {
            symbols.push((name.clone(), id, ScopeLevel::Global));
        }
        
        symbols
    }
}

/// Rust trait resolution system
///
/// This migrates the logic from TraitResolver, handling:
/// - Trait implementations
/// - Inherent methods
/// - Method resolution with Rust's preference rules
pub struct RustTraitResolver {
    /// Maps type names to traits they implement
    /// Key: "TypeName", Value: Vec<("TraitName", file_id)>
    type_to_traits: HashMap<String, Vec<(String, FileId)>>,
    
    /// Maps trait names to their methods
    /// Key: "TraitName", Value: Vec<"method_name">
    trait_methods: HashMap<String, Vec<String>>,
    
    /// Maps (type, method) pairs to the trait that defines the method
    /// Key: ("TypeName", "method_name"), Value: "TraitName"
    type_method_to_trait: HashMap<(String, String), String>,
    
    /// Tracks inherent methods on types (methods in impl blocks without traits)
    /// Key: "TypeName", Value: Vec<"method_name">
    inherent_methods: HashMap<String, Vec<String>>,
}

impl RustTraitResolver {
    pub fn new() -> Self {
        Self {
            type_to_traits: HashMap::new(),
            trait_methods: HashMap::new(),
            type_method_to_trait: HashMap::new(),
            inherent_methods: HashMap::new(),
        }
    }
    
    /// Check if a method is an inherent method on a type
    fn is_inherent_method(&self, type_name: &str, method_name: &str) -> bool {
        self.inherent_methods
            .get(type_name)
            .map(|methods| methods.iter().any(|m| m == method_name))
            .unwrap_or(false)
    }
}

impl InheritanceResolver for RustTraitResolver {
    fn add_inheritance(&mut self, child: String, parent: String, kind: &str) {
        if kind == "implements" {
            // In Rust, this is a trait implementation
            // We store with a dummy FileId for now (this will be fixed in Sprint 4)
            self.type_to_traits
                .entry(child)
                .or_default()
                .push((parent, FileId::new(1).unwrap()));
        }
        // Rust doesn't have class inheritance (extends), only trait implementations
    }
    
    fn resolve_method(&self, type_name: &str, method_name: &str) -> Option<String> {
        // Rust resolution order: inherent methods > trait methods
        
        // 1. Check if it's an inherent method (Rust prefers these)
        if self.is_inherent_method(type_name, method_name) {
            return Some(type_name.to_string());
        }
        
        // 2. Check direct trait method mapping
        if let Some(trait_name) = self
            .type_method_to_trait
            .get(&(type_name.to_string(), method_name.to_string()))
        {
            return Some(trait_name.clone());
        }
        
        // 3. Check if type implements any traits that have this method
        if let Some(traits) = self.type_to_traits.get(type_name) {
            for (trait_name, _) in traits {
                if let Some(methods) = self.trait_methods.get(trait_name) {
                    if methods.iter().any(|m| m == method_name) {
                        return Some(trait_name.clone());
                    }
                }
            }
        }
        
        None
    }
    
    fn get_inheritance_chain(&self, type_name: &str) -> Vec<String> {
        let mut chain = vec![type_name.to_string()];
        
        // Add all implemented traits
        if let Some(traits) = self.type_to_traits.get(type_name) {
            for (trait_name, _) in traits {
                if !chain.contains(trait_name) {
                    chain.push(trait_name.clone());
                }
            }
        }
        
        chain
    }
    
    fn is_subtype(&self, child: &str, parent: &str) -> bool {
        // In Rust, check if type implements trait
        if let Some(traits) = self.type_to_traits.get(child) {
            traits.iter().any(|(trait_name, _)| trait_name == parent)
        } else {
            false
        }
    }
    
    fn add_type_methods(&mut self, type_name: String, methods: Vec<String>) {
        // In Rust, this generic method should only be used for inherent methods
        // Traits should use the explicit add_trait_methods() method
        // This maintains the separation that Rust requires
        self.inherent_methods
            .entry(type_name)
            .or_default()
            .extend(methods);
    }
    
    fn get_all_methods(&self, type_name: &str) -> Vec<String> {
        let mut all_methods = Vec::new();
        
        // Add inherent methods
        if let Some(methods) = self.inherent_methods.get(type_name) {
            all_methods.extend(methods.clone());
        }
        
        // Add trait methods
        if let Some(traits) = self.type_to_traits.get(type_name) {
            for (trait_name, _) in traits {
                if let Some(methods) = self.trait_methods.get(trait_name) {
                    for method in methods {
                        if !all_methods.contains(method) {
                            all_methods.push(method.clone());
                        }
                    }
                }
            }
        }
        
        all_methods
    }
}

/// Extension methods for RustTraitResolver that match the original API
impl RustTraitResolver {
    /// Register that a type implements a trait (from original TraitResolver)
    pub fn add_trait_impl(&mut self, type_name: String, trait_name: String, file_id: FileId) {
        self.type_to_traits
            .entry(type_name)
            .or_default()
            .push((trait_name, file_id));
    }
    
    /// Register methods that a trait defines (from original TraitResolver)
    pub fn add_trait_methods(&mut self, trait_name: String, methods: Vec<String>) {
        self.trait_methods.insert(trait_name, methods);
    }
    
    /// Register inherent methods for a type (from original TraitResolver)
    pub fn add_inherent_methods(&mut self, type_name: String, methods: Vec<String>) {
        self.inherent_methods
            .entry(type_name)
            .or_default()
            .extend(methods);
    }
    
    /// Given a type and method name, find which trait it comes from (from original TraitResolver)
    /// Returns None if it's an inherent method or not found
    pub fn resolve_method_trait(&self, type_name: &str, method_name: &str) -> Option<&str> {
        // Skip if this is an inherent method (Rust prefers inherent methods)
        if self.is_inherent_method(type_name, method_name) {
            return None;
        }
        
        // First check direct mapping
        if let Some(trait_name) = self
            .type_method_to_trait
            .get(&(type_name.to_string(), method_name.to_string()))
        {
            return Some(trait_name);
        }
        
        // Then check if type implements any traits that have this method
        if let Some(traits) = self.type_to_traits.get(type_name) {
            let mut matching_traits = Vec::new();
            
            for (trait_name, _) in traits {
                if let Some(methods) = self.trait_methods.get(trait_name) {
                    if methods.contains(&method_name.to_string()) {
                        matching_traits.push(trait_name.as_str());
                    }
                }
            }
            
            // If multiple traits define the same method, return the first one
            // In real Rust this would be an error requiring disambiguation
            if !matching_traits.is_empty() {
                if matching_traits.len() > 1 {
                    eprintln!(
                        "WARNING: Ambiguous method '{}' on type '{}' - found in traits: {:?}",
                        method_name, type_name, matching_traits
                    );
                }
                return Some(matching_traits[0]);
            }
        }
        
        None
    }
}