//! PHP language parser implementation

pub mod behavior;
pub mod definition;
pub mod parser;

pub use behavior::PhpBehavior;
pub use parser::PhpParser;
pub use definition::PhpLanguage;

// Re-export for registry registration
pub(crate) use definition::register;