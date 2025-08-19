//! Go-specific language behavior implementation

use crate::parsing::LanguageBehavior;
use crate::parsing::behavior_state::{BehaviorState, StatefulBehavior};
use crate::parsing::resolution::{InheritanceResolver, ResolutionScope};
use crate::storage::DocumentIndex;
use crate::types::FileId;
use crate::{SymbolId, Visibility};
use std::path::{Path, PathBuf};
use tree_sitter::Language;

use super::resolution::{GoInheritanceResolver, GoResolutionContext};

/// Go language behavior implementation
#[derive(Clone)]
pub struct GoBehavior {
    state: BehaviorState,
}

impl GoBehavior {
    /// Create a new Go behavior instance
    pub fn new() -> Self {
        Self {
            state: BehaviorState::new(),
        }
    }
}

impl Default for GoBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl StatefulBehavior for GoBehavior {
    fn state(&self) -> &BehaviorState {
        &self.state
    }
}

impl LanguageBehavior for GoBehavior {
    fn format_module_path(&self, base_path: &str, _symbol_name: &str) -> String {
        // Go uses file paths as module paths, not including the symbol name
        // All symbols in the same file share the same module path for visibility
        base_path.to_string()
    }

    fn get_language(&self) -> Language {
        tree_sitter_go::LANGUAGE.into()
    }
    fn module_separator(&self) -> &'static str {
        "/"
    }

    fn module_path_from_file(&self, file_path: &Path, project_root: &Path) -> Option<String> {
        // Convert file path to Go package path relative to project root
        // e.g., pkg/utils/helpers.go -> pkg/utils

        // Get relative path from project root
        let relative_path = file_path
            .strip_prefix(project_root)
            .ok()
            .or_else(|| file_path.strip_prefix("./").ok())
            .unwrap_or(file_path);

        let path = relative_path.to_str()?;

        // Remove Go file extension and get directory
        let module_path = path
            .trim_start_matches("./")
            .trim_end_matches(".go");

        // Get directory path (Go packages are directories)
        let dir_path = if let Some(parent) = Path::new(module_path).parent() {
            parent.to_str().unwrap_or("")
        } else {
            "" // Root package
        };

        // Convert empty path to current directory marker
        if dir_path.is_empty() {
            Some(".".to_string())
        } else {
            Some(dir_path.to_string())
        }
    }

    fn parse_visibility(&self, signature: &str) -> Visibility {
        // Go uses capitalization for visibility
        // Extract the symbol name from the signature and check if it starts with uppercase
        
        // Try to extract the symbol name from different signature patterns
        let symbol_name = if let Some(func_start) = signature.find("func ") {
            // Function signature: "func FunctionName(" or "func (receiver) MethodName("
            let after_func = &signature[func_start + 5..];
            if let Some(receiver_end) = after_func.find(") ") {
                // Method with receiver: "func (r *Type) MethodName("
                let after_receiver = &after_func[receiver_end + 2..];
                after_receiver.split('(').next().unwrap_or("").trim()
            } else {
                // Regular function: "func FunctionName("
                after_func.split('(').next().unwrap_or("").trim()
            }
        } else if let Some(type_start) = signature.find("type ") {
            // Type signature: "type TypeName struct" or "type TypeName interface"
            let after_type = &signature[type_start + 5..];
            after_type.split_whitespace().next().unwrap_or("")
        } else if let Some(var_start) = signature.find("var ") {
            // Variable signature: "var VariableName type"
            let after_var = &signature[var_start + 4..];
            after_var.split_whitespace().next().unwrap_or("")
        } else if let Some(const_start) = signature.find("const ") {
            // Constant signature: "const ConstantName = value"
            let after_const = &signature[const_start + 6..];
            after_const.split_whitespace().next().unwrap_or("")
        } else {
            // Fallback: take the first word that looks like an identifier
            signature.split_whitespace()
                .find(|word| word.chars().next().map_or(false, |c| c.is_alphabetic()))
                .unwrap_or("")
        };

        // Go visibility: uppercase first letter = public, lowercase = private
        if let Some(first_char) = symbol_name.chars().next() {
            if first_char.is_uppercase() {
                Visibility::Public
            } else {
                Visibility::Private
            }
        } else {
            Visibility::Private
        }
    }

    fn supports_traits(&self) -> bool {
        true // Go has interfaces
    }

    fn supports_inherent_methods(&self) -> bool {
        true // Go has class methods
    }

    // Go-specific resolution overrides

    fn create_resolution_context(&self, file_id: FileId) -> Box<dyn ResolutionScope> {
        Box::new(GoResolutionContext::new(file_id))
    }

    fn create_inheritance_resolver(&self) -> Box<dyn InheritanceResolver> {
        Box::new(GoInheritanceResolver::new())
    }

    fn inheritance_relation_name(&self) -> &'static str {
        // Go uses both "extends" and "implements"
        // Default to "extends" as it's more general
        "extends"
    }

    fn map_relationship(&self, language_specific: &str) -> crate::relationship::RelationKind {
        use crate::relationship::RelationKind;

        match language_specific {
            "extends" => RelationKind::Extends,
            "implements" => RelationKind::Implements,
            "uses" => RelationKind::Uses,
            "calls" => RelationKind::Calls,
            "defines" => RelationKind::Defines,
            _ => RelationKind::References,
        }
    }

    // Override import tracking methods to use state

    fn register_file(&self, path: PathBuf, file_id: FileId, module_path: String) {
        self.register_file_with_state(path, file_id, module_path);
    }

    fn add_import(&self, import: crate::indexing::Import) {
        self.add_import_with_state(import);
    }

    fn get_imports_for_file(&self, file_id: FileId) -> Vec<crate::indexing::Import> {
        self.get_imports_from_state(file_id)
    }

    fn build_resolution_context(
        &self,
        file_id: FileId,
        document_index: &DocumentIndex,
    ) -> crate::error::IndexResult<Box<dyn ResolutionScope>> {
        use crate::error::IndexError;

        // Create Go-specific resolution context
        let mut context = GoResolutionContext::new(file_id);

        // 1. Add imported symbols (using behavior's tracked imports)
        let imports = self.get_imports_for_file(file_id);
        for import in imports {
            if let Some(symbol_id) = self.resolve_import(&import, document_index) {
                // Use alias if provided, otherwise use the last segment of the path
                let name = if let Some(alias) = &import.alias {
                    alias.clone()
                } else {
                    import
                        .path
                        .split(self.module_separator())
                        .last()
                        .unwrap_or(&import.path)
                        .to_string()
                };

                // Use the is_type_only field to determine where to place the import
                context.add_import_symbol(name, symbol_id, import.is_type_only);
            }
        }

        // 2. Add file's module-level symbols with proper scope context
        let file_symbols =
            document_index
                .find_symbols_by_file(file_id)
                .map_err(|e| IndexError::TantivyError {
                    operation: "find_symbols_by_file".to_string(),
                    cause: e.to_string(),
                })?;

        for symbol in file_symbols {
            if self.is_resolvable_symbol(&symbol) {
                // Use the new method that respects scope_context for hoisting
                context.add_symbol_with_context(
                    symbol.name.to_string(),
                    symbol.id,
                    symbol.scope_context.as_ref(),
                );
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
            // Only add if visible from this file
            if symbol.file_id != file_id && self.is_symbol_visible_from_file(&symbol, file_id) {
                // Global symbols go to global scope, others to module scope
                let scope_level = match symbol.visibility {
                    Visibility::Public => crate::parsing::ScopeLevel::Global,
                    _ => crate::parsing::ScopeLevel::Module,
                };

                context.add_symbol(symbol.name.to_string(), symbol.id, scope_level);
            }
        }

        Ok(Box::new(context))
    }

    // Go-specific: Support hoisting
    fn is_resolvable_symbol(&self, symbol: &crate::Symbol) -> bool {
        use crate::SymbolKind;
        use crate::symbol::ScopeContext;

        // Go hoists function declarations and class declarations
        // They can be used before their definition in the file
        let hoisted = matches!(
            symbol.kind,
            SymbolKind::Function | SymbolKind::Class | SymbolKind::Interface | SymbolKind::Enum
        );

        if hoisted {
            return true;
        }

        // Methods are always resolvable within their file
        if matches!(symbol.kind, SymbolKind::Method) {
            return true;
        }

        // Check scope_context for non-hoisted symbols
        if let Some(ref scope_context) = symbol.scope_context {
            match scope_context {
                ScopeContext::Module | ScopeContext::Global | ScopeContext::Package => true,
                ScopeContext::Local { .. } | ScopeContext::Parameter => false,
                ScopeContext::ClassMember => {
                    // Class members are resolvable if public or exported
                    matches!(symbol.visibility, Visibility::Public)
                }
            }
        } else {
            // Fallback for symbols without scope_context
            matches!(
                symbol.kind,
                SymbolKind::TypeAlias | SymbolKind::Constant | SymbolKind::Variable
            )
        }
    }

    // Go-specific: Handle ES module imports
    fn resolve_import(
        &self,
        import: &crate::indexing::Import,
        document_index: &DocumentIndex,
    ) -> Option<SymbolId> {
        // Go imports can be:
        // 1. Relative imports: ./foo, ../bar, ./utils/helper
        // 2. Absolute imports: @app/utils, lodash
        // 3. Named imports: import { foo } from './bar'
        // 4. Default imports: import foo from './bar'
        // 5. Namespace imports: import * as foo from './bar'

        // For now, use basic resolution
        // TODO: Implement full ES module resolution algorithm
        self.resolve_import_path(&import.path, document_index)
    }

    fn get_module_path_for_file(&self, file_id: FileId) -> Option<String> {
        // Use the BehaviorState to get module path (O(1) lookup)
        self.state.get_module_path(file_id)
    }

    fn import_matches_symbol(
        &self,
        import_path: &str,
        symbol_module_path: &str,
        importing_module: Option<&str>,
    ) -> bool {
        // Helper function to resolve relative path to absolute module path for Go
        fn resolve_relative_path(import_path: &str, importing_mod: &str) -> String {
            if import_path.starts_with("./") {
                // Same directory import
                let relative = import_path.trim_start_matches("./");

                if importing_mod.is_empty() || importing_mod == "." {
                    relative.to_string()
                } else {
                    format!("{importing_mod}/{relative}")
                }
            } else if import_path.starts_with("../") {
                // Parent directory import
                // Start with the importing module parts as owned strings
                let mut module_parts: Vec<String> =
                    importing_mod.split('/').map(|s| s.to_string()).collect();

                let mut path_remaining: &str = import_path;

                // Navigate up for each '../'
                while path_remaining.starts_with("../") {
                    if !module_parts.is_empty() {
                        module_parts.pop();
                    }
                    // If we've gone above the module root, we just continue
                    // This handles cases like ../../../some/path from a shallow module
                    path_remaining = &path_remaining[3..];
                }

                // Add the remaining path
                if !path_remaining.is_empty() {
                    module_parts.extend(
                        path_remaining
                            .split('/')
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string()),
                    );
                }

                module_parts.join("/")
            } else {
                // Not a relative path, return as-is
                import_path.to_string()
            }
        }

        // Case 1: Exact match (most common case, check first for performance)
        if import_path == symbol_module_path {
            return true;
        }

        // Case 2: Only do complex matching if we have the importing module context
        if let Some(importing_mod) = importing_module {
            // Go import resolution differs from Rust:
            // - Relative imports start with './' or '../'
            // - Absolute imports are package names or path aliases

            if import_path.starts_with("./") || import_path.starts_with("../") {
                // Resolve relative path to absolute module path
                let resolved = resolve_relative_path(import_path, importing_mod);

                // Check if it matches (with or without index)
                if matches_with_index(&resolved, symbol_module_path) {
                    return true;
                }
            }
            // else: bare module imports and scoped packages
            // These need exact match for now (TODO: implement proper resolution)
        }

        false
    }
}
