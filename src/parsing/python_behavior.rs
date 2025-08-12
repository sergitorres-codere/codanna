//! Python-specific language behavior implementation

use crate::parsing::language_behavior::LanguageBehavior;
use crate::Visibility;
use tree_sitter::Language;

/// Python language behavior implementation
#[derive(Clone)]
pub struct PythonBehavior {
    language: Language,
}

impl PythonBehavior {
    /// Create a new Python behavior instance
    pub fn new() -> Self {
        Self {
            language: tree_sitter_python::LANGUAGE.into(),
        }
    }
}

impl Default for PythonBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageBehavior for PythonBehavior {
    fn format_module_path(&self, base_path: &str, _symbol_name: &str) -> String {
        // Python typically uses file paths as module paths, not including the symbol name
        base_path.to_string()
    }
    
    fn parse_visibility(&self, signature: &str) -> Visibility {
        // Python uses naming conventions for visibility
        // Check for special/dunder methods first
        if signature.contains("__init__") || signature.contains("__str__") 
            || signature.contains("__repr__") || signature.contains("__eq__")
            || signature.contains("__hash__") || signature.contains("__call__") {
            // Dunder methods are public
            Visibility::Public
        } else if signature.contains("def __") || signature.contains("class __") {
            // Double underscore (not dunder) = private (name mangling)
            Visibility::Private
        } else if signature.contains("def _") || signature.contains("class _") {
            // Single underscore = module-level/protected
            Visibility::Module
        } else {
            // Everything else is public in Python
            Visibility::Public
        }
    }
    
    fn module_separator(&self) -> &'static str {
        "."
    }
    
    fn supports_traits(&self) -> bool {
        false // Python doesn't have traits, it has inheritance and mixins
    }
    
    fn supports_inherent_methods(&self) -> bool {
        false // Python methods are always on classes, not separate
    }
    
    fn get_language(&self) -> Language {
        self.language.clone()
    }
    
    fn get_language_type(&self) -> crate::parsing::Language {
        crate::parsing::Language::Python
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_module_path() {
        let behavior = PythonBehavior::new();
        assert_eq!(
            behavior.format_module_path("module.submodule", "function"),
            "module.submodule"
        );
    }
    
    #[test]
    fn test_parse_visibility() {
        let behavior = PythonBehavior::new();
        
        // Public functions
        assert_eq!(behavior.parse_visibility("def foo():"), Visibility::Public);
        assert_eq!(behavior.parse_visibility("class MyClass:"), Visibility::Public);
        
        // Protected/module-level
        assert_eq!(behavior.parse_visibility("def _internal():"), Visibility::Module);
        assert_eq!(behavior.parse_visibility("class _InternalClass:"), Visibility::Module);
        
        // Private (name mangling)
        assert_eq!(behavior.parse_visibility("def __private():"), Visibility::Private);
        
        // Special methods should be public
        assert_eq!(behavior.parse_visibility("def __init__(self):"), Visibility::Public);
        assert_eq!(behavior.parse_visibility("def __str__(self):"), Visibility::Public);
    }
    
    #[test]
    fn test_module_separator() {
        let behavior = PythonBehavior::new();
        assert_eq!(behavior.module_separator(), ".");
    }
    
    #[test]
    fn test_supports_features() {
        let behavior = PythonBehavior::new();
        assert!(!behavior.supports_traits());
        assert!(!behavior.supports_inherent_methods());
    }
    
    #[test]
    fn test_validate_node_kinds() {
        let behavior = PythonBehavior::new();
        
        // Valid Python node kinds
        assert!(behavior.validate_node_kind("function_definition"));
        assert!(behavior.validate_node_kind("class_definition"));
        assert!(behavior.validate_node_kind("module"));
        
        // Invalid node kind
        assert!(!behavior.validate_node_kind("struct_item")); // Rust-specific
    }
}