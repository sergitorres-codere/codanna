//! Test helpers for Go parser unit tests
//! 
//! This module provides utilities for creating test data, assertions, and common
//! test patterns used across Go parser tests.

use anyhow::Result;
use thiserror::Error;
use std::num::NonZeroU32;

use crate::{FileId, Symbol, SymbolKind, Visibility};
use crate::types::SymbolCounter;
use crate::parsing::LanguageParser;
use super::parser::GoParser;

/// Errors specific to Go parser testing
#[derive(Error, Debug)]
pub enum GoTestError {
    #[error(
        "Parser initialization failed: {0}\nSuggestion: Check that tree-sitter-go is properly configured"
    )]
    InitializationFailed(String),

    #[error(
        "Symbol extraction failed: {0}\nSuggestion: Verify Go fixture files are valid and contain expected symbols"
    )]
    SymbolExtractionFailed(String),

    #[error(
        "Test assertion failed: {0}\nSuggestion: Check test expectations against actual parser output"
    )]
    AssertionFailed(String),
}

// Type-safe wrappers for test data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TestFileId(NonZeroU32);

impl TestFileId {
    pub fn new(id: u32) -> Option<Self> {
        NonZeroU32::new(id).map(Self)
    }

    pub fn get(&self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub signature: Option<String>,
    pub is_exported: bool,
    pub doc_comment: Option<String>,
}

impl TestSymbol {
    pub fn new(name: impl Into<String>, kind: SymbolKind) -> Self {
        Self {
            name: name.into(),
            kind,
            signature: None,
            is_exported: false,
            doc_comment: None,
        }
    }

    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }

    pub fn with_exported(mut self, exported: bool) -> Self {
        self.is_exported = exported;
        self
    }

    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc_comment = Some(doc.into());
        self
    }
}

/// Test fixture builder for creating Go code samples
#[derive(Debug, Default)]
pub struct GoCodeBuilder {
    package: Option<String>,
    imports: Vec<String>,
    constants: Vec<String>,
    variables: Vec<String>,
    types: Vec<String>,
    functions: Vec<String>,
}

impl GoCodeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    pub fn with_import(mut self, import: impl Into<String>) -> Self {
        self.imports.push(import.into());
        self
    }

    pub fn with_const(mut self, const_decl: impl Into<String>) -> Self {
        self.constants.push(const_decl.into());
        self
    }

    pub fn with_var(mut self, var_decl: impl Into<String>) -> Self {
        self.variables.push(var_decl.into());
        self
    }

    pub fn with_type(mut self, type_decl: impl Into<String>) -> Self {
        self.types.push(type_decl.into());
        self
    }

    pub fn with_function(mut self, func_decl: impl Into<String>) -> Self {
        self.functions.push(func_decl.into());
        self
    }

    #[must_use = "Building the Go code may produce an incomplete result"]
    pub fn build(self) -> String {
        let mut code = String::new();

        // Package declaration
        if let Some(package) = self.package {
            code.push_str(&format!("package {package}\n\n"));
        }

        // Imports
        if !self.imports.is_empty() {
            if self.imports.len() == 1 {
                code.push_str(&format!("import {}\n\n", self.imports[0]));
            } else {
                code.push_str("import (\n");
                for import in &self.imports {
                    code.push_str(&format!("    {import}\n"));
                }
                code.push_str(")\n\n");
            }
        }

        // Constants
        for const_decl in &self.constants {
            code.push_str(&format!("{const_decl}\n\n"));
        }

        // Variables
        for var_decl in &self.variables {
            code.push_str(&format!("{var_decl}\n\n"));
        }

        // Types
        for type_decl in &self.types {
            code.push_str(&format!("{type_decl}\n\n"));
        }

        // Functions
        for func_decl in &self.functions {
            code.push_str(&format!("{func_decl}\n\n"));
        }

        code
    }
}

/// Helper function to parse Go code and extract symbols
pub fn parse_go_code(code: &str) -> Result<Vec<Symbol>, GoTestError> {
    let mut parser = GoParser::new().map_err(|e| {
        GoTestError::InitializationFailed(format!("Failed to create parser: {e}"))
    })?;

    let mut symbol_counter = SymbolCounter::new();
    let file_id = FileId::new(1).expect("Failed to create file ID");
    let symbols = parser.parse(code, file_id, &mut symbol_counter);

    Ok(symbols)
}

/// Convert internal Symbol to TestSymbol for easier assertions
pub fn to_test_symbol(symbol: &Symbol) -> TestSymbol {
    TestSymbol {
        name: symbol.name.to_string(),
        kind: symbol.kind.clone(),
        signature: symbol.signature.as_ref().map(|s| s.to_string()),
        is_exported: matches!(symbol.visibility, Visibility::Public),
        doc_comment: symbol.doc_comment.as_ref().map(|s| s.to_string()),
    }
}

/// Filter symbols by kind
pub fn filter_by_kind(symbols: &[Symbol], kind: SymbolKind) -> Vec<&Symbol> {
    symbols.iter().filter(|s| s.kind == kind).collect()
}

/// Filter symbols by visibility
pub fn filter_exported(symbols: &[Symbol]) -> Vec<&Symbol> {
    symbols.iter()
        .filter(|s| matches!(s.visibility, Visibility::Public))
        .collect()
}

pub fn filter_unexported(symbols: &[Symbol]) -> Vec<&Symbol> {
    symbols.iter()
        .filter(|s| !matches!(s.visibility, Visibility::Public))
        .collect()
}

/// Find symbol by name
pub fn find_symbol_by_name<'a>(symbols: &'a [Symbol], name: &str) -> Option<&'a Symbol> {
    symbols.iter().find(|s| s.name.as_ref() == name)
}

/// Assert that a symbol exists with the given name and kind
pub fn assert_symbol_exists(symbols: &[Symbol], name: &str, kind: SymbolKind) -> Result<(), GoTestError> {
    let symbol = find_symbol_by_name(symbols, name)
        .ok_or_else(|| GoTestError::AssertionFailed(
            format!("Symbol '{name}' not found")
        ))?;
    
    if symbol.kind != kind {
        return Err(GoTestError::AssertionFailed(
            format!("Symbol '{name}' has kind {:?}, expected {:?}", symbol.kind, kind)
        ));
    }

    Ok(())
}

/// Assert that a symbol has the expected signature
pub fn assert_symbol_signature(symbols: &[Symbol], name: &str, expected_signature: &str) -> Result<(), GoTestError> {
    let symbol = find_symbol_by_name(symbols, name)
        .ok_or_else(|| GoTestError::AssertionFailed(
            format!("Symbol '{name}' not found")
        ))?;
    
    let signature = symbol.signature.as_ref()
        .ok_or_else(|| GoTestError::AssertionFailed(
            format!("Symbol '{name}' has no signature")
        ))?;
    
    if !signature.contains(expected_signature) {
        return Err(GoTestError::AssertionFailed(
            format!("Symbol '{name}' signature '{signature}' does not contain '{expected_signature}'")
        ));
    }

    Ok(())
}

/// Assert that a symbol has the expected visibility
pub fn assert_symbol_visibility(symbols: &[Symbol], name: &str, is_exported: bool) -> Result<(), GoTestError> {
    let symbol = find_symbol_by_name(symbols, name)
        .ok_or_else(|| GoTestError::AssertionFailed(
            format!("Symbol '{name}' not found")
        ))?;
    
    let actual_exported = matches!(symbol.visibility, Visibility::Public);
    if actual_exported != is_exported {
        return Err(GoTestError::AssertionFailed(
            format!("Symbol '{name}' visibility is {}, expected {}", 
                   if actual_exported { "exported" } else { "unexported" },
                   if is_exported { "exported" } else { "unexported" })
        ));
    }

    Ok(())
}

/// Common Go code snippets for testing
pub mod snippets {
    /// Basic function declarations
    pub const BASIC_FUNCTIONS: &str = r#"
package main

// ExportedFunction is a public function
func ExportedFunction() string {
    return "exported"
}

// unexportedFunction is a private function
func unexportedFunction() int {
    return 42
}

func main() {
    fmt.Println("Hello, World!")
}
"#;

    /// Struct definitions with methods
    pub const STRUCT_WITH_METHODS: &str = r#"
package main

// User represents a user in the system
type User struct {
    Name string
    Age  int
    email string  // unexported field
}

// GetName returns the user's name
func (u *User) GetName() string {
    return u.Name
}

// setEmail sets the user's email (unexported method)
func (u *User) setEmail(email string) {
    u.email = email
}

// String implements the Stringer interface
func (u User) String() string {
    return fmt.Sprintf("User{Name: %s, Age: %d}", u.Name, u.Age)
}
"#;

    /// Interface definitions
    pub const INTERFACES: &str = r#"
package main

// Reader interface for reading operations
type Reader interface {
    Read([]byte) (int, error)
}

// Writer interface for writing operations
type Writer interface {
    Write([]byte) (int, error)
}

// ReadWriter embeds both Reader and Writer
type ReadWriter interface {
    Reader
    Writer
}

// processor is an unexported interface
type processor interface {
    process(data []byte) error
}
"#;

    /// Generic types and functions
    pub const GENERICS: &str = r#"
package main

// Generic function with type constraint
func Identity[T any](value T) T {
    return value
}

// Generic function with comparable constraint
func Equal[T comparable](a, b T) bool {
    return a == b
}

// Generic struct
type Container[T any] struct {
    items []T
}

// Generic method
func (c *Container[T]) Add(item T) {
    c.items = append(c.items, item)
}

// Generic interface
type Processor[T any] interface {
    Process(T) error
}
"#;

    /// Variable and constant declarations
    pub const VARS_AND_CONSTS: &str = r#"
package main

// Exported constants
const (
    Version     = "1.0.0"
    MaxRetries  = 3
    DefaultPort = 8080
)

// unexported constants
const (
    bufferSize = 1024
    timeout    = 30
)

// Exported variables
var (
    GlobalConfig *Config
    Logger       *log.Logger
)

// unexported variables
var (
    cache      map[string]interface{}
    mutex      sync.RWMutex
    isDebug    bool
)
"#;

    /// Complex import patterns
    pub const COMPLEX_IMPORTS: &str = r#"
package main

import (
    "fmt"
    "log"
    "net/http"
    
    // Aliased import
    httputil "net/http/httputil"
    
    // Dot import
    . "math"
    
    // Blank import
    _ "net/http/pprof"
    
    // External modules
    "github.com/gorilla/mux"
    "github.com/user/project/internal/config"
)
"#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_code_builder() -> Result<()> {
        let code = GoCodeBuilder::new()
            .with_package("main")
            .with_import(r#""fmt""#)
            .with_const("const Version = \"1.0.0\"")
            .with_function("func main() {\n    fmt.Println(\"Hello, World!\")\n}")
            .build();

        assert!(code.contains("package main"));
        assert!(code.contains("import \"fmt\""));
        assert!(code.contains("const Version"));
        assert!(code.contains("func main()"));

        Ok(())
    }

    #[test]
    fn test_parse_basic_functions() -> Result<()> {
        let symbols = parse_go_code(snippets::BASIC_FUNCTIONS)?;
        
        // Should find at least 3 functions
        let functions = filter_by_kind(&symbols, SymbolKind::Function);
        assert!(functions.len() >= 3, "Should find at least 3 functions");

        // Check specific functions
        assert_symbol_exists(&symbols, "ExportedFunction", SymbolKind::Function)?;
        assert_symbol_exists(&symbols, "unexportedFunction", SymbolKind::Function)?;
        assert_symbol_exists(&symbols, "main", SymbolKind::Function)?;

        // Check visibility
        assert_symbol_visibility(&symbols, "ExportedFunction", true)?;
        assert_symbol_visibility(&symbols, "unexportedFunction", false)?;

        Ok(())
    }

    #[test]
    fn test_test_symbol_conversion() -> Result<()> {
        let symbols = parse_go_code(snippets::BASIC_FUNCTIONS)?;
        let test_symbols: Vec<TestSymbol> = symbols.iter().map(to_test_symbol).collect();

        assert!(!test_symbols.is_empty(), "Should convert symbols");
        
        let exported_func = test_symbols.iter()
            .find(|s| s.name == "ExportedFunction")
            .expect("Should find ExportedFunction");
        
        assert!(exported_func.is_exported, "ExportedFunction should be exported");
        assert_eq!(exported_func.kind, SymbolKind::Function);

        Ok(())
    }
}