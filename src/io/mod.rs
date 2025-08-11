//! Input/Output handling for CLI and tool integration.
//!
//! This module provides:
//! - Unified output formatting (text, JSON)
//! - Consistent error handling and exit codes
//! - Future: JSON-RPC 2.0 support for IDE integration

pub mod exit_code;
pub mod format;
pub mod input;
pub mod output;
#[cfg(test)]
mod test;

pub use exit_code::ExitCode;
pub use format::{ErrorDetails, JsonResponse, OutputFormat, ResponseMeta};
pub use output::OutputManager;
// Future: pub use input::{JsonRpcRequest, JsonRpcResponse};
