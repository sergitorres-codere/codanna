//! TypeScript language parser implementation

pub mod behavior;
pub mod definition;
pub mod parser;

pub use behavior::TypeScriptBehavior;
pub use definition::TypeScriptLanguage;
pub use parser::TypeScriptParser;

// Re-export for registry registration
pub(crate) use definition::register;
