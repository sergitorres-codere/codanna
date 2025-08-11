//! Output management for CLI commands.
//!
//! Handles formatting and display for different output formats,
//! providing a unified interface for text and JSON output.

use crate::error::IndexError;
use crate::io::exit_code::ExitCode;
use crate::io::format::{JsonResponse, OutputFormat};
use serde::Serialize;
use std::fmt::Display;
use std::io::{self, Write};

/// Manages output formatting and display.
///
/// Provides methods for outputting success results, collections,
/// and errors in either text or JSON format based on configuration.
pub struct OutputManager {
    format: OutputFormat,
    stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
}

impl OutputManager {
    /// Create a new output manager with the specified format.
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            stdout: Box::new(io::stdout()),
            stderr: Box::new(io::stderr()),
        }
    }

    /// Create an output manager for testing with custom writers.
    #[cfg(test)]
    pub fn new_with_writers(
        format: OutputFormat,
        stdout: Box<dyn Write>,
        stderr: Box<dyn Write>,
    ) -> Self {
        Self {
            format,
            stdout,
            stderr,
        }
    }

    /// Output a successful result.
    ///
    /// In JSON mode, wraps the data in a success response.
    /// In text mode, displays the data using its Display implementation.
    pub fn success<T>(&mut self, data: T) -> io::Result<ExitCode>
    where
        T: Serialize + Display,
    {
        match self.format {
            OutputFormat::Json => {
                let response = JsonResponse::success(&data);
                writeln!(self.stdout, "{}", serde_json::to_string_pretty(&response)?)?;
            }
            OutputFormat::Text => {
                writeln!(self.stdout, "{data}")?;
            }
        }
        Ok(ExitCode::Success)
    }

    /// Output a single item or indicate not found.
    ///
    /// If the item is Some, outputs it as success.
    /// If None, outputs a not found message.
    pub fn item<T>(&mut self, item: Option<T>, entity: &str, name: &str) -> io::Result<ExitCode>
    where
        T: Serialize + Display,
    {
        match item {
            Some(data) => self.success(data),
            None => self.not_found(entity, name),
        }
    }

    /// Output a not found result.
    pub fn not_found(&mut self, entity: &str, name: &str) -> io::Result<ExitCode> {
        match self.format {
            OutputFormat::Json => {
                let response = JsonResponse::not_found(entity, name);
                writeln!(self.stdout, "{}", serde_json::to_string_pretty(&response)?)?;
            }
            OutputFormat::Text => {
                writeln!(self.stderr, "{entity} '{name}' not found")?;
            }
        }
        Ok(ExitCode::NotFound)
    }

    /// Output a collection with proper formatting.
    ///
    /// Empty collections are treated as not found.
    /// Non-empty collections are displayed as a list.
    pub fn collection<T, I>(&mut self, items: I, entity_name: &str) -> io::Result<ExitCode>
    where
        T: Serialize + Display,
        I: IntoIterator<Item = T>,
    {
        let items: Vec<T> = items.into_iter().collect();

        if items.is_empty() {
            return self.not_found(entity_name, "any");
        }

        match self.format {
            OutputFormat::Json => {
                let response = JsonResponse::success(&items);
                writeln!(self.stdout, "{}", serde_json::to_string_pretty(&response)?)?;
            }
            OutputFormat::Text => {
                writeln!(self.stdout, "Found {} {entity_name}:", items.len())?;
                writeln!(self.stdout, "{}", "=".repeat(40))?;
                for item in items {
                    writeln!(self.stdout, "{item}")?;
                }
            }
        }
        Ok(ExitCode::Success)
    }

    /// Output an error with suggestions.
    pub fn error(&mut self, error: &IndexError) -> io::Result<ExitCode> {
        match self.format {
            OutputFormat::Json => {
                let response = JsonResponse::from_error(error);
                writeln!(self.stderr, "{}", serde_json::to_string_pretty(&response)?)?;
            }
            OutputFormat::Text => {
                writeln!(self.stderr, "Error: {error}")?;
                for suggestion in error.recovery_suggestions() {
                    writeln!(self.stderr, "  Suggestion: {suggestion}")?;
                }
            }
        }
        Ok(ExitCode::from_error(error))
    }

    /// Output progress information (text mode only).
    ///
    /// In JSON mode, progress messages are suppressed to avoid
    /// polluting the JSON output.
    pub fn progress(&mut self, message: &str) -> io::Result<()> {
        if matches!(self.format, OutputFormat::Text) {
            writeln!(self.stderr, "{message}")?;
        }
        Ok(())
    }

    /// Output informational message (text mode only).
    pub fn info(&mut self, message: &str) -> io::Result<()> {
        if matches!(self.format, OutputFormat::Text) {
            writeln!(self.stdout, "{message}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_manager_text_success() {
        let stdout = Vec::new();
        let stderr = Vec::new();

        let mut manager =
            OutputManager::new_with_writers(OutputFormat::Text, Box::new(stdout), Box::new(stderr));

        let code = manager.success("Test output").unwrap();
        assert_eq!(code, ExitCode::Success);
    }

    #[test]
    fn test_output_manager_json_success() {
        let stdout = Vec::new();
        let stderr = Vec::new();

        let mut manager =
            OutputManager::new_with_writers(OutputFormat::Json, Box::new(stdout), Box::new(stderr));

        #[derive(Serialize)]
        struct TestData {
            value: i32,
        }

        impl Display for TestData {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "TestData({})", self.value)
            }
        }

        let data = TestData { value: 42 };
        let code = manager.success(data).unwrap();
        assert_eq!(code, ExitCode::Success);
    }
}
