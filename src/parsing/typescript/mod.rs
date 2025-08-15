//! TypeScript language parser implementation

pub mod behavior;
pub mod definition;
pub mod parser;
pub mod resolution;

pub use behavior::TypeScriptBehavior;
pub use definition::TypeScriptLanguage;
pub use parser::TypeScriptParser;
pub use resolution::{TypeScriptInheritanceResolver, TypeScriptResolutionContext};

// Re-export for registry registration
pub(crate) use definition::register;
