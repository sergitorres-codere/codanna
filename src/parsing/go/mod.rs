//! Go language parser implementation

pub mod behavior;
pub mod definition;
pub mod parser;
pub mod resolution;

pub use behavior::GoBehavior;
pub use definition::GoLanguage;
pub use parser::GoParser;
pub use resolution::{GoInheritanceResolver, GoResolutionContext};

// Re-export for registry registration
pub(crate) use definition::register;
