//! PHP language parser implementation

pub mod behavior;
pub mod definition;
pub mod parser;

pub use behavior::PhpBehavior;
pub use definition::PhpLanguage;
pub use parser::PhpParser;

// Re-export for registry registration
pub(crate) use definition::register;
