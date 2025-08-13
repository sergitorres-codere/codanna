//! PHP-specific language behavior implementation

use crate::Visibility;
use crate::parsing::language_behavior::LanguageBehavior;
use tree_sitter::Language;

/// PHP language behavior implementation
#[derive(Clone)]
pub struct PhpBehavior {
    language: Language,
}

impl PhpBehavior {
    /// Create a new PHP behavior instance
    pub fn new() -> Self {
        Self {
            language: tree_sitter_php::LANGUAGE_PHP.into(),
        }
    }
}

impl Default for PhpBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageBehavior for PhpBehavior {
    fn format_module_path(&self, base_path: &str, _symbol_name: &str) -> String {
        // PHP typically uses file paths as module paths, not including the symbol name
        // PHP parsers should set more specific paths for methods in the parser itself
        base_path.to_string()
    }

    fn parse_visibility(&self, signature: &str) -> Visibility {
        // PHP has explicit visibility modifiers
        if signature.contains("private ") {
            Visibility::Private
        } else if signature.contains("protected ") {
            Visibility::Module // Protected in PHP = Module visibility
        } else if signature.contains("public ") {
            Visibility::Public
        } else {
            // PHP defaults to public if no modifier specified
            Visibility::Public
        }
    }

    fn module_separator(&self) -> &'static str {
        "\\" // PHP namespace separator
    }

    fn supports_traits(&self) -> bool {
        true // PHP has traits
    }

    fn supports_inherent_methods(&self) -> bool {
        false // PHP methods are always in classes/traits
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
        let behavior = PhpBehavior::new();
        assert_eq!(
            behavior.format_module_path("App\\Controllers", "UserController"),
            "App\\Controllers"
        );
    }

    #[test]
    fn test_parse_visibility() {
        let behavior = PhpBehavior::new();

        // Explicit visibility
        assert_eq!(
            behavior.parse_visibility("public function foo()"),
            Visibility::Public
        );
        assert_eq!(
            behavior.parse_visibility("private function bar()"),
            Visibility::Private
        );
        assert_eq!(
            behavior.parse_visibility("protected function baz()"),
            Visibility::Module
        );

        // Default visibility (public in PHP)
        assert_eq!(
            behavior.parse_visibility("function legacy()"),
            Visibility::Public
        );
        assert_eq!(
            behavior.parse_visibility("static function helper()"),
            Visibility::Public
        );
    }

    #[test]
    fn test_module_separator() {
        let behavior = PhpBehavior::new();
        assert_eq!(behavior.module_separator(), "\\");
    }

    #[test]
    fn test_supports_features() {
        let behavior = PhpBehavior::new();
        assert!(behavior.supports_traits()); // PHP has traits
        assert!(!behavior.supports_inherent_methods());
    }

    #[test]
    fn test_validate_node_kinds() {
        let behavior = PhpBehavior::new();

        // Valid PHP node kinds
        assert!(behavior.validate_node_kind("function_definition"));
        assert!(behavior.validate_node_kind("class_declaration"));
        assert!(behavior.validate_node_kind("method_declaration"));

        // Invalid node kind
        assert!(!behavior.validate_node_kind("struct_item")); // Rust-specific
    }
}
