//! Rust-specific language behavior implementation

use super::resolution::{RustResolutionContext, RustTraitResolver};
use crate::FileId;
use crate::Visibility;
use crate::parsing::{InheritanceResolver, LanguageBehavior, ResolutionScope};
use std::path::Path;
use tree_sitter::Language;

/// Rust language behavior implementation
#[derive(Clone)]
pub struct RustBehavior {
    language: Language,
}

impl RustBehavior {
    /// Create a new Rust behavior instance
    pub fn new() -> Self {
        Self {
            language: tree_sitter_rust::LANGUAGE.into(),
        }
    }
}

impl Default for RustBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageBehavior for RustBehavior {
    fn format_module_path(&self, base_path: &str, symbol_name: &str) -> String {
        format!("{base_path}::{symbol_name}")
    }

    fn parse_visibility(&self, signature: &str) -> Visibility {
        if signature.contains("pub(crate)") {
            Visibility::Crate
        } else if signature.contains("pub(super)") {
            Visibility::Module
        } else if signature.contains("pub ") || signature.starts_with("pub ") {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }

    fn module_separator(&self) -> &'static str {
        "::"
    }

    fn supports_traits(&self) -> bool {
        true
    }

    fn supports_inherent_methods(&self) -> bool {
        true
    }

    fn get_language(&self) -> Language {
        self.language.clone()
    }

    fn module_path_from_file(&self, file_path: &Path, project_root: &Path) -> Option<String> {
        // Get relative path from project root
        let relative_path = file_path.strip_prefix(project_root).ok()?;

        // Remove the "src/" prefix if present
        let path_without_src = relative_path.strip_prefix("src/").unwrap_or(relative_path);

        // Remove the file extension
        let path_str = path_without_src.to_str()?;
        let path_without_ext = path_str.strip_suffix(".rs").unwrap_or(path_str);

        // Handle special cases for mod.rs files BEFORE converting separators
        let module_path = if let Some(stripped) = path_without_ext.strip_suffix("/mod") {
            // foo/mod.rs -> foo
            stripped.to_string()
        } else {
            path_without_ext.to_string()
        };

        // Convert path separators to module separators
        let module_path = module_path.replace('/', "::");

        // Handle special cases - main, lib, and empty paths all map to crate root
        let module_path = if module_path == "main" || module_path == "lib" || module_path.is_empty()
        {
            "crate".to_string()
        } else {
            format!("crate::{module_path}")
        };

        Some(module_path)
    }

    // Override resolution methods to use Rust-specific implementations

    fn create_resolution_context(&self, file_id: FileId) -> Box<dyn ResolutionScope> {
        Box::new(RustResolutionContext::new(file_id))
    }

    fn create_inheritance_resolver(&self) -> Box<dyn InheritanceResolver> {
        Box::new(RustTraitResolver::new())
    }

    fn add_trait_impl(&self, _type_name: String, _trait_name: String, _file_id: FileId) {
        // This will be properly integrated in Sprint 4 when SimpleIndexer is updated
        // For now, this is a placeholder that maintains the interface
        eprintln!("RustBehavior::add_trait_impl called - will be integrated in Sprint 4");
    }

    fn add_inherent_methods(&self, _type_name: String, _methods: Vec<String>) {
        // This will be properly integrated in Sprint 4 when SimpleIndexer is updated
        eprintln!("RustBehavior::add_inherent_methods called - will be integrated in Sprint 4");
    }

    fn add_trait_methods(&self, _trait_name: String, _methods: Vec<String>) {
        // This will be properly integrated in Sprint 4 when SimpleIndexer is updated
        eprintln!("RustBehavior::add_trait_methods called - will be integrated in Sprint 4");
    }

    fn resolve_method_trait(&self, _type_name: &str, _method: &str) -> Option<&str> {
        // This will be properly integrated in Sprint 4 when SimpleIndexer is updated
        // For now, return None to maintain backward compatibility
        None
    }

    fn format_method_call(&self, receiver: &str, method: &str) -> String {
        // Rust uses :: for associated functions and . for methods
        // Since we don't have context here, default to method syntax
        format!("{receiver}.{method}")
    }

    fn inheritance_relation_name(&self) -> &'static str {
        // Rust uses "implements" for traits
        "implements"
    }

    fn map_relationship(&self, language_specific: &str) -> crate::relationship::RelationKind {
        use crate::relationship::RelationKind;
        match language_specific {
            "implements" => RelationKind::Implements,
            "uses" => RelationKind::Uses,
            "calls" => RelationKind::Calls,
            "defines" => RelationKind::Defines,
            "references" => RelationKind::References,
            _ => RelationKind::References,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_module_path() {
        let behavior = RustBehavior::new();
        assert_eq!(
            behavior.format_module_path("crate::module", "function"),
            "crate::module::function"
        );
    }

    #[test]
    fn test_parse_visibility() {
        let behavior = RustBehavior::new();

        assert_eq!(
            behavior.parse_visibility("pub fn foo()"),
            Visibility::Public
        );
        assert_eq!(behavior.parse_visibility("fn foo()"), Visibility::Private);
        assert_eq!(
            behavior.parse_visibility("pub(crate) fn foo()"),
            Visibility::Crate
        );
        assert_eq!(
            behavior.parse_visibility("pub(super) fn foo()"),
            Visibility::Module
        );
    }

    #[test]
    fn test_module_separator() {
        let behavior = RustBehavior::new();
        assert_eq!(behavior.module_separator(), "::");
    }

    #[test]
    fn test_supports_features() {
        let behavior = RustBehavior::new();
        assert!(behavior.supports_traits());
        assert!(behavior.supports_inherent_methods());
    }

    #[test]
    fn test_abi_version() {
        let behavior = RustBehavior::new();
        // Rust should be on ABI-15
        assert_eq!(behavior.get_abi_version(), 15);
    }

    #[test]
    fn test_validate_node_kinds() {
        let behavior = RustBehavior::new();

        // Valid Rust node kinds
        assert!(behavior.validate_node_kind("function_item"));
        assert!(behavior.validate_node_kind("struct_item"));
        assert!(behavior.validate_node_kind("impl_item"));
        assert!(behavior.validate_node_kind("trait_item"));

        // Invalid node kind
        assert!(!behavior.validate_node_kind("made_up_node"));
    }

    #[test]
    fn test_module_path_from_file() {
        let behavior = RustBehavior::new();
        let root = Path::new("/project");

        // Test main.rs
        let main_path = Path::new("/project/src/main.rs");
        assert_eq!(
            behavior.module_path_from_file(main_path, root),
            Some("crate".to_string())
        );

        // Test lib.rs
        let lib_path = Path::new("/project/src/lib.rs");
        assert_eq!(
            behavior.module_path_from_file(lib_path, root),
            Some("crate".to_string())
        );

        // Test regular module
        let module_path = Path::new("/project/src/foo/bar.rs");
        assert_eq!(
            behavior.module_path_from_file(module_path, root),
            Some("crate::foo::bar".to_string())
        );

        // Test mod.rs
        let mod_path = Path::new("/project/src/foo/mod.rs");
        assert_eq!(
            behavior.module_path_from_file(mod_path, root),
            Some("crate::foo".to_string())
        );

        // Test nested module
        let nested_path = Path::new("/project/src/a/b/c.rs");
        assert_eq!(
            behavior.module_path_from_file(nested_path, root),
            Some("crate::a::b::c".to_string())
        );

        // Test file outside src
        let outside_path = Path::new("/project/tests/integration.rs");
        assert_eq!(
            behavior.module_path_from_file(outside_path, root),
            Some("crate::tests::integration".to_string())
        );
    }
}
