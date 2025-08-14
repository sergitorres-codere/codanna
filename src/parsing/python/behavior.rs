//! Python-specific language behavior implementation

use crate::Visibility;
use crate::parsing::LanguageBehavior;
use std::path::Path;
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
        if signature.contains("__init__")
            || signature.contains("__str__")
            || signature.contains("__repr__")
            || signature.contains("__eq__")
            || signature.contains("__hash__")
            || signature.contains("__call__")
        {
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

    fn module_path_from_file(&self, file_path: &Path, project_root: &Path) -> Option<String> {
        // Get relative path from project root
        let relative_path = file_path.strip_prefix(project_root).ok()?;

        // Convert path to string
        let path_str = relative_path.to_str()?;

        // Remove common Python source directories if present
        let path_without_src = path_str
            .strip_prefix("src/")
            .or_else(|| path_str.strip_prefix("lib/"))
            .or_else(|| path_str.strip_prefix("app/"))
            .unwrap_or(path_str);

        // Remove the .py extension
        let path_without_ext = path_without_src
            .strip_suffix(".py")
            .or_else(|| path_without_src.strip_suffix(".pyx"))
            .or_else(|| path_without_src.strip_suffix(".pyi"))
            .unwrap_or(path_without_src);

        // Handle __init__.py - it represents the package itself
        let module_path = if path_without_ext.ends_with("/__init__") {
            // Remove /__init__ to get the package path
            path_without_ext
                .strip_suffix("/__init__")
                .unwrap_or(path_without_ext)
                .to_string()
        } else {
            path_without_ext.to_string()
        };

        // Convert path separators to Python module separators
        let module_path = module_path.replace('/', ".");

        // Handle special cases
        if module_path.is_empty() || module_path == "__init__" {
            // Root __init__.py or empty path
            None
        } else if module_path == "__main__" || module_path == "main" {
            // __main__.py is the entry point
            Some("__main__".to_string())
        } else {
            Some(module_path)
        }
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
        assert_eq!(
            behavior.parse_visibility("class MyClass:"),
            Visibility::Public
        );

        // Protected/module-level
        assert_eq!(
            behavior.parse_visibility("def _internal():"),
            Visibility::Module
        );
        assert_eq!(
            behavior.parse_visibility("class _InternalClass:"),
            Visibility::Module
        );

        // Private (name mangling)
        assert_eq!(
            behavior.parse_visibility("def __private():"),
            Visibility::Private
        );

        // Special methods should be public
        assert_eq!(
            behavior.parse_visibility("def __init__(self):"),
            Visibility::Public
        );
        assert_eq!(
            behavior.parse_visibility("def __str__(self):"),
            Visibility::Public
        );
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

    #[test]
    fn test_module_path_from_file() {
        let behavior = PythonBehavior::new();
        let root = Path::new("/project");

        // Test regular module
        let module_path = Path::new("/project/src/package/module.py");
        assert_eq!(
            behavior.module_path_from_file(module_path, root),
            Some("package.module".to_string())
        );

        // Test __init__.py (represents the package)
        let init_path = Path::new("/project/src/package/__init__.py");
        assert_eq!(
            behavior.module_path_from_file(init_path, root),
            Some("package".to_string())
        );

        // Test nested module
        let nested_path = Path::new("/project/src/package/subpackage/module.py");
        assert_eq!(
            behavior.module_path_from_file(nested_path, root),
            Some("package.subpackage.module".to_string())
        );

        // Test __main__.py
        let main_path = Path::new("/project/__main__.py");
        assert_eq!(
            behavior.module_path_from_file(main_path, root),
            Some("__main__".to_string())
        );

        // Test root __init__.py (should return None)
        let root_init = Path::new("/project/__init__.py");
        assert_eq!(behavior.module_path_from_file(root_init, root), None);

        // Test without src directory
        let no_src_path = Path::new("/project/mypackage/mymodule.py");
        assert_eq!(
            behavior.module_path_from_file(no_src_path, root),
            Some("mypackage.mymodule".to_string())
        );

        // Test .pyi stub file
        let stub_path = Path::new("/project/typings/module.pyi");
        assert_eq!(
            behavior.module_path_from_file(stub_path, root),
            Some("typings.module".to_string())
        );
    }
}
