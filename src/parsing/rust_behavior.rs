//! Rust-specific language behavior implementation

use crate::parsing::language_behavior::LanguageBehavior;
use crate::Visibility;
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
        format!("{}::{}", base_path, symbol_name)
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
        
        assert_eq!(behavior.parse_visibility("pub fn foo()"), Visibility::Public);
        assert_eq!(behavior.parse_visibility("fn foo()"), Visibility::Private);
        assert_eq!(behavior.parse_visibility("pub(crate) fn foo()"), Visibility::Crate);
        assert_eq!(behavior.parse_visibility("pub(super) fn foo()"), Visibility::Module);
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
}