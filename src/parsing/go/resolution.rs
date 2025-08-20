//! Go-specific resolution and inheritance implementation
//!
//! This module implements Go's scoping and resolution rules:
//! - Package-level scope (functions, types, variables, constants)
//! - Function/method scope (parameters and local variables)
//! - Block scope (variables declared in blocks)
//! - Imported package symbols
//! - Interface implementation tracking (implicit in Go)

use crate::parsing::{InheritanceResolver, ResolutionScope, ScopeLevel, ScopeType};
use crate::storage::DocumentIndex;
use crate::{FileId, SymbolId};
use std::collections::HashMap;

/// Go-specific resolution context handling Go's scoping rules
///
/// Go has the following scoping rules:
/// 1. Package scope - functions, types, variables, constants at package level
/// 2. Function scope - parameters and local variables within functions
/// 3. Block scope - variables declared within blocks (if, for, etc.)
/// 4. Imported symbols - symbols from imported packages
pub struct GoResolutionContext {
    /// Local scope (function parameters, local variables, block variables)
    local_scope: HashMap<String, SymbolId>,

    /// Package-level symbols (functions, types, variables, constants)
    package_symbols: HashMap<String, SymbolId>,

    /// Imported symbols from other packages
    imported_symbols: HashMap<String, SymbolId>,

    /// Track nested scopes (blocks, functions, etc.)
    scope_stack: Vec<ScopeType>,

    /// Import tracking (path -> alias)
    imports: Vec<(String, Option<String>)>,
}

impl GoResolutionContext {
    pub fn new(_file_id: FileId) -> Self {
        Self {
            local_scope: HashMap::new(),
            package_symbols: HashMap::new(),
            imported_symbols: HashMap::new(),
            scope_stack: Vec::new(),
            imports: Vec::new(),
        }
    }

    /// Add an import (import statement)
    pub fn add_import(&mut self, path: String, alias: Option<String>) {
        self.imports.push((path, alias));
    }

    /// Add an imported symbol to the context
    ///
    /// This is called when an import is resolved to add the symbol to the imported symbols.
    /// In Go, all imports are available in both type and value contexts.
    pub fn add_import_symbol(&mut self, name: String, symbol_id: SymbolId, _is_type_only: bool) {
        // In Go, all symbols are available in both type and value contexts
        self.imported_symbols.insert(name, symbol_id);
    }

    /// Add a symbol with proper scope context
    ///
    /// This method uses the symbol's scope_context to determine proper placement.
    /// In Go: package-level symbols, function parameters, and local variables.
    pub fn add_symbol_with_context(
        &mut self,
        name: String,
        symbol_id: SymbolId,
        scope_context: Option<&crate::symbol::ScopeContext>,
    ) {
        use crate::symbol::ScopeContext;

        match scope_context {
            Some(ScopeContext::Local { .. }) => {
                // Local variables and function parameters
                self.local_scope.insert(name, symbol_id);
            }
            Some(ScopeContext::ClassMember) => {
                // Struct/interface members - treat as local within the type
                self.local_scope.insert(name, symbol_id);
            }
            Some(ScopeContext::Parameter) => {
                // Function parameters are local
                self.local_scope.insert(name, symbol_id);
            }
            Some(ScopeContext::Module) | Some(ScopeContext::Global) => {
                // Package-level declarations (functions, types, variables, constants)
                self.package_symbols.insert(name, symbol_id);
            }
            Some(ScopeContext::Package) => {
                // Imported symbols
                self.imported_symbols.insert(name, symbol_id);
            }
            None => {
                // Default to package scope for Go (most symbols are package-level)
                self.package_symbols.insert(name, symbol_id);
            }
        }
    }

    /// Resolve local package symbols (symbols in the same package)
    ///
    /// This method resolves symbols that are declared in other files
    /// within the same Go package.
    pub fn resolve_local_package_symbols(
        &self,
        symbol_name: &str,
        document_index: &DocumentIndex,
    ) -> Option<SymbolId> {
        // In Go, all symbols in the same package are accessible
        // Look for symbols with matching name and package path
        if let Ok(candidates) = document_index.find_symbols_by_name(symbol_name) {
            for candidate in candidates {
                // TODO: Compare module paths for same-package symbol resolution (Phase 5.1)
                // Check if symbol is in the same package (same module path)
                if let Some(ref _module_path) = candidate.module_path {
                    // For now, consider symbols with same module path as local package
                    // This is a simplified approach - full Go module resolution would be more complex
                    if candidate.visibility == crate::Visibility::Public {
                        return Some(candidate.id);
                    }
                }
            }
        }
        None
    }

    /// Resolve imported package symbols (symbols from imported packages)
    ///
    /// This method resolves symbols that come from explicitly imported packages.
    /// It handles various Go import patterns:
    /// - Standard library: "fmt", "strings", "net/http"
    /// - External modules: "github.com/user/repo/package"
    /// - Local modules: "myproject/internal/utils"
    pub fn resolve_imported_package_symbols(
        &self,
        package_name: &str,
        symbol_name: &str,
        document_index: &DocumentIndex,
    ) -> Option<SymbolId> {
        // Look through imports to find the package
        for (import_path, alias) in &self.imports {
            let effective_name = alias.as_deref().unwrap_or_else(|| {
                // If no alias, use the last component of the import path
                import_path.split('/').next_back().unwrap_or(import_path)
            });

            if effective_name == package_name {
                // Found the matching import, now resolve the symbol
                return self.resolve_symbol_in_package(import_path, symbol_name, document_index);
            }
        }
        None
    }

    /// Handle Go module paths and go.mod resolution
    ///
    /// This method implements basic Go module resolution logic.
    /// In a full implementation, this would parse go.mod files and
    /// resolve module dependencies properly.
    pub fn handle_go_module_paths(
        &self,
        module_path: &str,
        _document_index: &DocumentIndex, // TODO: Use for go.mod parsing and module version resolution (Phase 5.2)
    ) -> Option<String> {
        // Simplified Go module resolution
        // In practice, this would:
        // 1. Check if the path is a standard library package
        // 2. Check go.mod for module replacements
        // 3. Resolve module versions and dependencies

        if self.is_standard_library_package(module_path) {
            // Standard library packages are always available
            Some(module_path.to_string())
        } else {
            // For now, assume the module path is valid
            // A full implementation would check go.mod, go.sum, and module cache
            Some(module_path.to_string())
        }
    }

    /// Check if a package is part of the Go standard library
    ///
    /// This method identifies Go standard library packages that don't
    /// need explicit module resolution.
    pub fn is_standard_library_package(&self, package_path: &str) -> bool {
        // Common Go standard library packages
        // In practice, this would be a more comprehensive list or
        // determined by checking the Go installation
        const STDLIB_PACKAGES: &[&str] = &[
            "fmt",
            "strings",
            "strconv",
            "io",
            "os",
            "time",
            "context",
            "encoding/json",
            "net/http",
            "net/url",
            "path/filepath",
            "regexp",
            "sort",
            "sync",
            "errors",
            "log",
            "math",
            "bytes",
            "bufio",
            "crypto",
            "database/sql",
            "reflect",
            "runtime",
        ];

        STDLIB_PACKAGES
            .iter()
            .any(|&pkg| package_path == pkg || package_path.starts_with(&format!("{pkg}/")))
    }

    /// Resolve a symbol within a specific package
    ///
    /// Helper method to find symbols that belong to a given package path.
    fn resolve_symbol_in_package(
        &self,
        package_path: &str,
        symbol_name: &str,
        document_index: &DocumentIndex,
    ) -> Option<SymbolId> {
        if let Ok(candidates) = document_index.find_symbols_by_name(symbol_name) {
            for candidate in candidates {
                if let Some(ref module_path) = candidate.module_path {
                    let module_str: &str = module_path.as_ref();

                    // Check for exact match or last component match
                    if (module_str == package_path
                        || module_str.split('/').next_back()
                            == Some(package_path.split('/').next_back().unwrap_or(package_path)))
                        && candidate.visibility == crate::Visibility::Public
                    {
                        return Some(candidate.id);
                    }
                }
            }
        }
        None
    }
}

impl ResolutionScope for GoResolutionContext {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn add_symbol(&mut self, name: String, symbol_id: SymbolId, scope_level: ScopeLevel) {
        match scope_level {
            ScopeLevel::Local => {
                // Local scope: function parameters, local variables, block variables
                self.local_scope.insert(name, symbol_id);
            }
            ScopeLevel::Module | ScopeLevel::Global => {
                // Package-level symbols: functions, types, variables, constants
                self.package_symbols.insert(name, symbol_id);
            }
            ScopeLevel::Package => {
                // Imported symbols from other packages
                self.imported_symbols.insert(name, symbol_id);
            }
        }
    }

    fn resolve(&self, name: &str) -> Option<SymbolId> {
        // Go resolution order:
        // 1. Local scope (function parameters, local variables, block variables)
        // 2. Package scope (functions, types, variables, constants)
        // 3. Imported symbols (from other packages)

        // 1. Check local scope first (most specific)
        if let Some(&id) = self.local_scope.get(name) {
            return Some(id);
        }

        // 2. Check package-level symbols
        if let Some(&id) = self.package_symbols.get(name) {
            return Some(id);
        }

        // 3. Check imported symbols
        self.imported_symbols.get(name).copied()
    }

    fn clear_local_scope(&mut self) {
        // Clear local variables and parameters when exiting scope
        self.local_scope.clear();
    }

    fn enter_scope(&mut self, scope_type: ScopeType) {
        self.scope_stack.push(scope_type);
        // No special handling needed for Go scope entry
    }

    fn exit_scope(&mut self) {
        self.scope_stack.pop();
        // Clear local scope when exiting function scope
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
        for (name, &id) in &self.package_symbols {
            symbols.push((name.clone(), id, ScopeLevel::Module));
        }
        for (name, &id) in &self.imported_symbols {
            symbols.push((name.clone(), id, ScopeLevel::Package));
        }

        symbols
    }
}

/// Go interface implementation resolution system
///
/// In Go, interface implementation is implicit - any type that has all the methods
/// of an interface automatically implements that interface. This resolver tracks:
/// - Type method definitions
/// - Interface method requirements
/// - Implicit interface implementations
pub struct GoInheritanceResolver {
    /// Maps struct names to interfaces they implement (implicit)
    /// Key: "StructName", Value: Vec<"InterfaceName">
    struct_implements: HashMap<String, Vec<String>>,

    /// Maps interface names to interfaces they embed
    /// Key: "InterfaceName", Value: Vec<"EmbeddedInterfaceName">
    interface_embeds: HashMap<String, Vec<String>>,

    /// Tracks methods on types (structs and interfaces)
    /// Key: "TypeName", Value: Vec<"method_name">
    type_methods: HashMap<String, Vec<String>>,
}

impl Default for GoInheritanceResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GoInheritanceResolver {
    pub fn new() -> Self {
        Self {
            struct_implements: HashMap::new(),
            interface_embeds: HashMap::new(),
            type_methods: HashMap::new(),
        }
    }

    /// Check if a type is an interface (heuristic)
    ///
    /// In Go, this should ideally be determined by the parser, but for now
    /// we use heuristics based on naming conventions and tracking.
    /// TODO: Use for interface detection in Phase 5.3 (Type System Integration) when resolving interface implementations
    #[allow(dead_code)]
    fn is_interface(&self, type_name: &str) -> bool {
        self.interface_embeds.contains_key(type_name)
            || type_name.starts_with("I")  // Common Go interface naming convention
            || !self.struct_implements.contains_key(type_name)
    }
}

impl InheritanceResolver for GoInheritanceResolver {
    fn add_inheritance(&mut self, child: String, parent: String, kind: &str) {
        match kind {
            "embeds" => {
                // Interface embeds another interface
                self.interface_embeds.entry(child).or_default().push(parent);
            }
            "implements" => {
                // Struct implements interface (implicit in Go)
                self.struct_implements
                    .entry(child)
                    .or_default()
                    .push(parent);
            }
            _ => {
                // Go doesn't have explicit inheritance like "extends"
                // Handle as interface embedding by default
                self.interface_embeds.entry(child).or_default().push(parent);
            }
        }
    }

    fn resolve_method(&self, type_name: &str, method_name: &str) -> Option<String> {
        // Check if the type has this method directly
        if let Some(methods) = self.type_methods.get(type_name) {
            if methods.iter().any(|m| m == method_name) {
                return Some(type_name.to_string());
            }
        }

        // For structs: check implemented interfaces
        if let Some(interfaces) = self.struct_implements.get(type_name) {
            for interface in interfaces {
                if let Some(methods) = self.type_methods.get(interface) {
                    if methods.iter().any(|m| m == method_name) {
                        return Some(interface.clone());
                    }
                }
                // Recursively check embedded interfaces
                if let Some(resolved) = self.resolve_method(interface, method_name) {
                    return Some(resolved);
                }
            }
        }

        // For interfaces: check embedded interfaces
        if let Some(embedded) = self.interface_embeds.get(type_name) {
            for embedded_interface in embedded {
                if let Some(resolved) = self.resolve_method(embedded_interface, method_name) {
                    return Some(resolved);
                }
            }
        }

        None
    }

    fn get_inheritance_chain(&self, type_name: &str) -> Vec<String> {
        let mut chain = vec![type_name.to_string()];
        let mut visited = std::collections::HashSet::new();
        visited.insert(type_name.to_string());

        // For structs: add implemented interfaces
        if let Some(interfaces) = self.struct_implements.get(type_name) {
            for interface in interfaces {
                if visited.insert(interface.clone()) {
                    chain.push(interface.clone());
                    // Get interface's embedded interfaces
                    for embedded in self.get_inheritance_chain(interface) {
                        if visited.insert(embedded.clone()) {
                            chain.push(embedded);
                        }
                    }
                }
            }
        }

        // For interfaces: add embedded interfaces
        if let Some(embedded) = self.interface_embeds.get(type_name) {
            for embedded_interface in embedded {
                if visited.insert(embedded_interface.clone()) {
                    chain.push(embedded_interface.clone());
                    // Recursively get embedded interface's chain
                    for ancestor in self.get_inheritance_chain(embedded_interface) {
                        if visited.insert(ancestor.clone()) {
                            chain.push(ancestor);
                        }
                    }
                }
            }
        }

        chain
    }

    fn is_subtype(&self, child: &str, parent: &str) -> bool {
        // Check if struct implements interface
        if let Some(interfaces) = self.struct_implements.get(child) {
            if interfaces.contains(&parent.to_string()) {
                return true;
            }
            // Check if any implemented interface embeds parent
            for interface in interfaces {
                if self.is_subtype(interface, parent) {
                    return true;
                }
            }
        }

        // Check interface embedding
        if let Some(embedded) = self.interface_embeds.get(child) {
            if embedded.contains(&parent.to_string()) {
                return true;
            }
            // Recursive check for embedded interfaces
            for embedded_interface in embedded {
                if self.is_subtype(embedded_interface, parent) {
                    return true;
                }
            }
        }

        false
    }

    fn add_type_methods(&mut self, type_name: String, methods: Vec<String>) {
        self.type_methods
            .entry(type_name)
            .or_default()
            .extend(methods);
    }

    fn get_all_methods(&self, type_name: &str) -> Vec<String> {
        let mut all_methods = Vec::new();
        let mut visited = std::collections::HashSet::new();

        // Helper to collect methods recursively
        fn collect_methods(
            resolver: &GoInheritanceResolver,
            type_name: &str,
            all_methods: &mut Vec<String>,
            visited: &mut std::collections::HashSet<String>,
        ) {
            if !visited.insert(type_name.to_string()) {
                return;
            }

            // Add this type's methods
            if let Some(methods) = resolver.type_methods.get(type_name) {
                for method in methods {
                    if !all_methods.contains(method) {
                        all_methods.push(method.clone());
                    }
                }
            }

            // For structs: check implemented interfaces
            if let Some(interfaces) = resolver.struct_implements.get(type_name) {
                for interface in interfaces {
                    collect_methods(resolver, interface, all_methods, visited);
                }
            }

            // For interfaces: check embedded interfaces
            if let Some(embedded) = resolver.interface_embeds.get(type_name) {
                for embedded_interface in embedded {
                    collect_methods(resolver, embedded_interface, all_methods, visited);
                }
            }
        }

        collect_methods(self, type_name, &mut all_methods, &mut visited);
        all_methods
    }
}

/// Extension methods for GoInheritanceResolver for Go-specific operations
impl GoInheritanceResolver {
    /// Register that a struct implements an interface (implicit in Go)
    pub fn add_struct_implements(&mut self, struct_name: String, interface_name: String) {
        self.struct_implements
            .entry(struct_name)
            .or_default()
            .push(interface_name);
    }

    /// Register that an interface embeds other interfaces
    pub fn add_interface_embeds(&mut self, interface_name: String, embedded: Vec<String>) {
        self.interface_embeds.insert(interface_name, embedded);
    }

    /// Get all interfaces that a struct implements (directly and indirectly)
    pub fn get_all_interfaces(&self, struct_name: &str) -> Vec<String> {
        let mut interfaces = Vec::new();
        let mut visited = std::collections::HashSet::new();

        // Get directly implemented interfaces
        if let Some(direct) = self.struct_implements.get(struct_name) {
            for interface in direct {
                if visited.insert(interface.clone()) {
                    interfaces.push(interface.clone());
                    // Get embedded interfaces
                    for embedded in self.get_inheritance_chain(interface) {
                        if visited.insert(embedded.clone()) {
                            interfaces.push(embedded);
                        }
                    }
                }
            }
        }

        interfaces
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_package_scope() {
        let mut context = GoResolutionContext::new(FileId::new(1).unwrap());

        // Add package-level function
        context.add_symbol(
            "MyFunction".to_string(),
            SymbolId::new(1).unwrap(),
            ScopeLevel::Module,
        );

        // Function should be resolvable at package scope
        assert_eq!(
            context.resolve("MyFunction"),
            Some(SymbolId::new(1).unwrap())
        );
    }

    #[test]
    fn test_local_scope() {
        let mut context = GoResolutionContext::new(FileId::new(1).unwrap());

        // Add local variable
        context.add_symbol(
            "localVar".to_string(),
            SymbolId::new(1).unwrap(),
            ScopeLevel::Local,
        );

        // Should be resolvable
        assert_eq!(context.resolve("localVar"), Some(SymbolId::new(1).unwrap()));

        // Clear local scope
        context.clear_local_scope();

        // Should no longer be resolvable
        assert_eq!(context.resolve("localVar"), None);
    }

    #[test]
    fn test_struct_implements_interface() {
        let mut resolver = GoInheritanceResolver::new();

        // Struct implements interface (implicit in Go)
        resolver.add_inheritance(
            "MyStruct".to_string(),
            "MyInterface".to_string(),
            "implements",
        );

        // Check subtype relationship
        assert!(resolver.is_subtype("MyStruct", "MyInterface"));

        // Check inheritance chain
        let chain = resolver.get_inheritance_chain("MyStruct");
        assert!(chain.contains(&"MyInterface".to_string()));
    }

    #[test]
    fn test_interface_embedding() {
        let mut resolver = GoInheritanceResolver::new();

        // Interface embeds multiple interfaces
        resolver.add_interface_embeds(
            "CompositeInterface".to_string(),
            vec!["Reader".to_string(), "Writer".to_string()],
        );

        // Check inheritance chain
        let chain = resolver.get_inheritance_chain("CompositeInterface");
        assert!(chain.contains(&"Reader".to_string()));
        assert!(chain.contains(&"Writer".to_string()));

        // Check subtype relationships
        assert!(resolver.is_subtype("CompositeInterface", "Reader"));
        assert!(resolver.is_subtype("CompositeInterface", "Writer"));
    }

    #[test]
    fn test_go_resolution_context_package_resolution() {
        let mut context = GoResolutionContext::new(FileId::new(1).unwrap());

        // Test adding imports
        context.add_import("fmt".to_string(), None);
        context.add_import(
            "github.com/user/repo/utils".to_string(),
            Some("utils".to_string()),
        );

        assert_eq!(context.imports.len(), 2);
        assert_eq!(context.imports[0], ("fmt".to_string(), None));
        assert_eq!(
            context.imports[1],
            (
                "github.com/user/repo/utils".to_string(),
                Some("utils".to_string())
            )
        );
    }

    #[test]
    fn test_standard_library_detection() {
        let context = GoResolutionContext::new(FileId::new(1).unwrap());

        // Test common standard library packages
        assert!(context.is_standard_library_package("fmt"));
        assert!(context.is_standard_library_package("strings"));
        assert!(context.is_standard_library_package("net/http"));
        assert!(context.is_standard_library_package("encoding/json"));

        // Test non-standard library packages
        assert!(!context.is_standard_library_package("github.com/user/repo"));
        assert!(!context.is_standard_library_package("myproject/internal/utils"));
        assert!(!context.is_standard_library_package("unknown_package"));
    }

    #[test]
    fn test_go_module_path_handling() {
        let context = GoResolutionContext::new(FileId::new(1).unwrap());

        // Create a minimal DocumentIndex for testing
        let temp_dir = std::env::temp_dir().join("codanna_test_go_resolution");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let document_index = DocumentIndex::new(&temp_dir).unwrap();

        // Test standard library path handling
        let stdlib_result = context.handle_go_module_paths("fmt", &document_index);
        assert_eq!(stdlib_result, Some("fmt".to_string()));

        let stdlib_subpackage = context.handle_go_module_paths("net/http", &document_index);
        assert_eq!(stdlib_subpackage, Some("net/http".to_string()));

        // Test external module path handling
        let external_result =
            context.handle_go_module_paths("github.com/user/repo", &document_index);
        assert_eq!(external_result, Some("github.com/user/repo".to_string()));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap_or(());
    }

    #[test]
    fn test_resolve_imported_package_symbols() {
        let mut context = GoResolutionContext::new(FileId::new(1).unwrap());

        // Set up imports
        context.add_import("fmt".to_string(), None);
        context.add_import(
            "github.com/user/repo/utils".to_string(),
            Some("utils".to_string()),
        );

        // Create a minimal DocumentIndex for testing
        let temp_dir = std::env::temp_dir().join("codanna_test_go_resolution_2");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let document_index = DocumentIndex::new(&temp_dir).unwrap();

        // The actual resolution would require symbols in the index
        // For now, test that the method handles the imports correctly
        let result = context.resolve_imported_package_symbols("fmt", "Println", &document_index);
        // Result will be None since we don't have actual symbols, but it should not panic
        assert!(result.is_none());

        let result = context.resolve_imported_package_symbols("utils", "Helper", &document_index);
        assert!(result.is_none());

        // Test non-existent package
        let result =
            context.resolve_imported_package_symbols("nonexistent", "Symbol", &document_index);
        assert!(result.is_none());

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).unwrap_or(());
    }

    #[test]
    fn test_resolution_scope_with_package_context() {
        let mut context = GoResolutionContext::new(FileId::new(1).unwrap());

        // Test adding symbols at different scope levels
        context.add_symbol(
            "LocalVar".to_string(),
            SymbolId::new(1).unwrap(),
            crate::parsing::ScopeLevel::Local,
        );

        context.add_symbol(
            "PackageFunc".to_string(),
            SymbolId::new(2).unwrap(),
            crate::parsing::ScopeLevel::Module,
        );

        context.add_symbol(
            "ImportedSymbol".to_string(),
            SymbolId::new(3).unwrap(),
            crate::parsing::ScopeLevel::Package,
        );

        // Test resolution order: local -> package -> imported
        assert_eq!(context.resolve("LocalVar"), Some(SymbolId::new(1).unwrap()));
        assert_eq!(
            context.resolve("PackageFunc"),
            Some(SymbolId::new(2).unwrap())
        );
        assert_eq!(
            context.resolve("ImportedSymbol"),
            Some(SymbolId::new(3).unwrap())
        );
        assert_eq!(context.resolve("NonExistent"), None);

        // Test that local scope has higher priority
        context.add_symbol(
            "ConflictingName".to_string(),
            SymbolId::new(4).unwrap(),
            crate::parsing::ScopeLevel::Local,
        );
        context.add_symbol(
            "ConflictingName".to_string(),
            SymbolId::new(5).unwrap(),
            crate::parsing::ScopeLevel::Module,
        );

        assert_eq!(
            context.resolve("ConflictingName"),
            Some(SymbolId::new(4).unwrap())
        );
    }
}
