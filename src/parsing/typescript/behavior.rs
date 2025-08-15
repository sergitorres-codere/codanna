//! TypeScript-specific language behavior implementation

use crate::Visibility;
use crate::parsing::LanguageBehavior;
use crate::parsing::resolution::{InheritanceResolver, ResolutionScope};
use crate::types::FileId;
use std::path::Path;
use tree_sitter::Language;

use super::resolution::{TypeScriptInheritanceResolver, TypeScriptResolutionContext};

/// TypeScript language behavior implementation
pub struct TypeScriptBehavior;

impl LanguageBehavior for TypeScriptBehavior {
    fn format_module_path(&self, base_path: &str, _symbol_name: &str) -> String {
        // TypeScript uses file paths as module paths, not including the symbol name
        // All symbols in the same file share the same module path for visibility
        base_path.to_string()
    }

    fn get_language(&self) -> Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }
    fn module_separator(&self) -> &'static str {
        "."
    }

    fn module_path_from_file(&self, file_path: &Path, project_root: &Path) -> Option<String> {
        // Convert file path to module path relative to project root
        // e.g., src/utils/helpers.ts -> src.utils.helpers

        // Get relative path from project root
        let relative_path = file_path
            .strip_prefix(project_root)
            .ok()
            .or_else(|| file_path.strip_prefix("./").ok())
            .unwrap_or(file_path);

        let path = relative_path.to_str()?;

        // Remove common directory prefixes and file extensions
        let module_path = path
            .trim_start_matches("./")
            .trim_start_matches("src/")
            .trim_start_matches("lib/")
            .trim_end_matches(".ts")
            .trim_end_matches(".tsx")
            .trim_end_matches(".mts")
            .trim_end_matches(".cts")
            .trim_end_matches(".d.ts")
            .trim_end_matches("/index");

        // Replace path separators with module separators
        Some(module_path.replace('/', "."))
    }

    fn parse_visibility(&self, signature: &str) -> Visibility {
        // TypeScript visibility modifiers
        if signature.contains("export ") || signature.contains("export default") {
            Visibility::Public
        } else if signature.contains("private ") || signature.contains("#") {
            Visibility::Private
        } else if signature.contains("protected ") {
            // TypeScript has protected but Rust's Visibility enum doesn't
            // Map protected to Module visibility as a reasonable approximation
            Visibility::Module
        } else {
            // Default visibility for TypeScript symbols
            // Module-level symbols are private by default unless exported
            Visibility::Private
        }
    }

    fn supports_traits(&self) -> bool {
        true // TypeScript has interfaces
    }

    fn supports_inherent_methods(&self) -> bool {
        true // TypeScript has class methods
    }

    // TypeScript-specific resolution overrides

    fn create_resolution_context(&self, file_id: FileId) -> Box<dyn ResolutionScope> {
        Box::new(TypeScriptResolutionContext::new(file_id))
    }

    fn create_inheritance_resolver(&self) -> Box<dyn InheritanceResolver> {
        Box::new(TypeScriptInheritanceResolver::new())
    }

    fn inheritance_relation_name(&self) -> &'static str {
        // TypeScript uses both "extends" and "implements"
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
}
