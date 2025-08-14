//! Rust language parser implementation

pub mod behavior;
pub mod definition;
pub mod parser;

pub use behavior::RustBehavior;
pub use definition::RustLanguage;
pub use parser::RustParser;

// Re-export for registry registration
pub(crate) use definition::register;
