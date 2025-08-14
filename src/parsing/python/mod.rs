//! Python language parser implementation

pub mod behavior;
pub mod definition;
pub mod parser;

pub use behavior::PythonBehavior;
pub use parser::PythonParser;
pub use definition::PythonLanguage;

// Re-export for registry registration
pub(crate) use definition::register;