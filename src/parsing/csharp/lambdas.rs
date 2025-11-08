//! Lambda Expression Extraction for C#
//!
//! This module provides structures and APIs for extracting lambda expressions and anonymous
//! functions from C# code. Lambda expressions are a fundamental feature in modern C# for
//! functional programming patterns, LINQ queries, event handlers, and callbacks.
//!
//! # Lambda Expression Types in C#
//!
//! C# supports several forms of lambda and anonymous function syntax:
//!
//! 1. **Expression Lambdas** - Single expression body
//!    ```csharp
//!    x => x * 2
//!    (x, y) => x + y
//!    ```
//!
//! 2. **Statement Lambdas** - Multi-statement block body
//!    ```csharp
//!    x => { return x * 2; }
//!    (x, y) => { var sum = x + y; return sum; }
//!    ```
//!
//! 3. **Async Lambdas** - Asynchronous lambda expressions
//!    ```csharp
//!    async x => await ProcessAsync(x)
//!    async (x, y) => { await Task.Delay(100); return x + y; }
//!    ```
//!
//! 4. **Anonymous Methods** - Older delegate syntax (C# 2.0)
//!    ```csharp
//!    delegate(int x) { return x * 2; }
//!    ```
//!
//! # Tree-sitter Grammar
//!
//! The tree-sitter-c-sharp grammar represents lambdas as:
//! - `lambda_expression` - Modern lambda syntax
//! - `anonymous_method_expression` - Delegate syntax
//! - `anonymous_object_creation_expression` - Anonymous type initialization
//!
//! # Usage Example
//!
//! ```rust
//! use codanna::parsing::csharp::CSharpParser;
//!
//! let mut parser = CSharpParser::new().unwrap();
//! let code = r#"
//!     var doubled = numbers.Select(x => x * 2).ToList();
//! "#;
//!
//! let lambdas = parser.find_lambdas(code);
//! for lambda in lambdas {
//!     println!("Lambda at line {}: {} parameter(s)",
//!         lambda.range.start_line, lambda.parameters.len());
//! }
//! ```

use crate::Range;
use serde::{Deserialize, Serialize};

/// Information about a lambda expression or anonymous function
///
/// This struct represents a single lambda expression found in C# code, including its
/// parameters, location, and metadata about its type and characteristics.
///
/// # Examples
///
/// For the lambda `(x, y) => x + y`:
/// - `parameters`: `["x", "y"]`
/// - `is_async`: `false`
/// - `is_statement_body`: `false`
/// - `parameter_count`: `2`
///
/// For the async lambda `async x => await Process(x)`:
/// - `parameters`: `["x"]`
/// - `is_async`: `true`
/// - `is_statement_body`: `false`
/// - `parameter_count`: `1`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LambdaInfo {
    /// Parameter names in order of declaration
    ///
    /// For simple lambdas like `x => x * 2`, this contains `["x"]`.
    /// For multi-parameter lambdas like `(x, y) => x + y`, this contains `["x", "y"]`.
    /// For anonymous methods with explicit types like `delegate(int x, string y) { }`,
    /// this contains `["x", "y"]` (types are in the signature, not here).
    pub parameters: Vec<String>,

    /// Source location range of the entire lambda expression
    ///
    /// Includes the parameter list, arrow/delegate keyword, and body.
    pub range: Range,

    /// Whether this is an async lambda
    ///
    /// True for `async x => ...` or `async (x, y) => ...`
    pub is_async: bool,

    /// Whether the lambda has a statement body (block) vs expression body
    ///
    /// - `false`: Expression lambda like `x => x * 2`
    /// - `true`: Statement lambda like `x => { return x * 2; }`
    pub is_statement_body: bool,

    /// Number of parameters
    ///
    /// Convenience field for `parameters.len()`. Useful for quick filtering.
    pub parameter_count: usize,

    /// The lambda type as detected from the AST
    ///
    /// - `"lambda_expression"` - Modern C# lambda (most common)
    /// - `"anonymous_method_expression"` - Delegate syntax
    pub lambda_type: LambdaType,
}

/// The syntactic form of the lambda or anonymous function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LambdaType {
    /// Modern lambda expression syntax: `x => x * 2` or `(x, y) => x + y`
    Lambda,

    /// Anonymous method delegate syntax: `delegate(int x) { return x * 2; }`
    AnonymousMethod,
}

impl LambdaInfo {
    /// Create a new LambdaInfo
    ///
    /// # Arguments
    ///
    /// * `parameters` - List of parameter names
    /// * `range` - Source location of the lambda
    /// * `is_async` - Whether the lambda is async
    /// * `is_statement_body` - Whether the lambda has a block body
    /// * `lambda_type` - The syntactic type of the lambda
    ///
    /// # Examples
    ///
    /// ```rust
    /// use codanna::parsing::csharp::lambdas::{LambdaInfo, LambdaType};
    /// use codanna::Range;
    ///
    /// let lambda = LambdaInfo::new(
    ///     vec!["x".to_string()],
    ///     Range::new(10, 20, 10, 30),
    ///     false,
    ///     false,
    ///     LambdaType::Lambda,
    /// );
    ///
    /// assert_eq!(lambda.parameter_count, 1);
    /// assert_eq!(lambda.parameters[0], "x");
    /// ```
    pub fn new(
        parameters: Vec<String>,
        range: Range,
        is_async: bool,
        is_statement_body: bool,
        lambda_type: LambdaType,
    ) -> Self {
        let parameter_count = parameters.len();
        Self {
            parameters,
            range,
            is_async,
            is_statement_body,
            parameter_count,
            lambda_type,
        }
    }

    /// Check if this is a simple single-expression lambda
    ///
    /// Returns true for lambdas like `x => x * 2` (single parameter, expression body).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use codanna::parsing::csharp::lambdas::{LambdaInfo, LambdaType};
    /// # use codanna::Range;
    /// let simple = LambdaInfo::new(
    ///     vec!["x".to_string()],
    ///     Range::new(0, 0, 0, 0),
    ///     false,
    ///     false,
    ///     LambdaType::Lambda,
    /// );
    /// assert!(simple.is_simple());
    ///
    /// let complex = LambdaInfo::new(
    ///     vec!["x".to_string(), "y".to_string()],
    ///     Range::new(0, 0, 0, 0),
    ///     false,
    ///     true,
    ///     LambdaType::Lambda,
    /// );
    /// assert!(!complex.is_simple());
    /// ```
    pub fn is_simple(&self) -> bool {
        self.parameter_count == 1 && !self.is_statement_body && !self.is_async
    }

    /// Check if this lambda has no parameters
    ///
    /// Returns true for lambdas like `() => DoSomething()`.
    pub fn is_parameterless(&self) -> bool {
        self.parameter_count == 0
    }

    /// Check if this lambda has multiple parameters
    ///
    /// Returns true for lambdas like `(x, y) => x + y`.
    pub fn has_multiple_parameters(&self) -> bool {
        self.parameter_count > 1
    }
}

/// Collection of lambda expressions found in a file or code segment
///
/// Provides convenience methods for querying and filtering lambdas.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LambdaCollection {
    /// All lambda expressions found
    pub lambdas: Vec<LambdaInfo>,
}

impl LambdaCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self {
            lambdas: Vec::new(),
        }
    }

    /// Create a collection from a vector of lambdas
    pub fn from_vec(lambdas: Vec<LambdaInfo>) -> Self {
        Self { lambdas }
    }

    /// Add a lambda to the collection
    pub fn add(&mut self, lambda: LambdaInfo) {
        self.lambdas.push(lambda);
    }

    /// Get the total number of lambdas
    pub fn count(&self) -> usize {
        self.lambdas.len()
    }

    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.lambdas.is_empty()
    }

    /// Get all async lambdas
    pub fn async_lambdas(&self) -> Vec<&LambdaInfo> {
        self.lambdas.iter().filter(|l| l.is_async).collect()
    }

    /// Get all simple lambdas (single parameter, expression body)
    pub fn simple_lambdas(&self) -> Vec<&LambdaInfo> {
        self.lambdas.iter().filter(|l| l.is_simple()).collect()
    }

    /// Get all lambdas with a specific parameter count
    pub fn lambdas_with_param_count(&self, count: usize) -> Vec<&LambdaInfo> {
        self.lambdas
            .iter()
            .filter(|l| l.parameter_count == count)
            .collect()
    }

    /// Get all statement-body lambdas
    pub fn statement_lambdas(&self) -> Vec<&LambdaInfo> {
        self.lambdas
            .iter()
            .filter(|l| l.is_statement_body)
            .collect()
    }

    /// Get all expression-body lambdas
    pub fn expression_lambdas(&self) -> Vec<&LambdaInfo> {
        self.lambdas
            .iter()
            .filter(|l| !l.is_statement_body)
            .collect()
    }

    /// Get all anonymous method (delegate) syntax lambdas
    pub fn anonymous_methods(&self) -> Vec<&LambdaInfo> {
        self.lambdas
            .iter()
            .filter(|l| l.lambda_type == LambdaType::AnonymousMethod)
            .collect()
    }

    /// Get all modern lambda syntax expressions
    pub fn lambda_expressions(&self) -> Vec<&LambdaInfo> {
        self.lambdas
            .iter()
            .filter(|l| l.lambda_type == LambdaType::Lambda)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lambda_info_creation() {
        let lambda = LambdaInfo::new(
            vec!["x".to_string()],
            Range::new(10, 20, 10, 30),
            false,
            false,
            LambdaType::Lambda,
        );

        assert_eq!(lambda.parameter_count, 1);
        assert_eq!(lambda.parameters[0], "x");
        assert!(!lambda.is_async);
        assert!(!lambda.is_statement_body);
        assert_eq!(lambda.lambda_type, LambdaType::Lambda);
    }

    #[test]
    fn test_is_simple() {
        let simple = LambdaInfo::new(
            vec!["x".to_string()],
            Range::new(0, 0, 0, 0),
            false,
            false,
            LambdaType::Lambda,
        );
        assert!(simple.is_simple());

        let not_simple_multi_param = LambdaInfo::new(
            vec!["x".to_string(), "y".to_string()],
            Range::new(0, 0, 0, 0),
            false,
            false,
            LambdaType::Lambda,
        );
        assert!(!not_simple_multi_param.is_simple());

        let not_simple_statement = LambdaInfo::new(
            vec!["x".to_string()],
            Range::new(0, 0, 0, 0),
            false,
            true,
            LambdaType::Lambda,
        );
        assert!(!not_simple_statement.is_simple());

        let not_simple_async = LambdaInfo::new(
            vec!["x".to_string()],
            Range::new(0, 0, 0, 0),
            true,
            false,
            LambdaType::Lambda,
        );
        assert!(!not_simple_async.is_simple());
    }

    #[test]
    fn test_collection_filtering() {
        let mut collection = LambdaCollection::new();

        collection.add(LambdaInfo::new(
            vec!["x".to_string()],
            Range::new(1, 0, 1, 10),
            false,
            false,
            LambdaType::Lambda,
        ));

        collection.add(LambdaInfo::new(
            vec!["x".to_string()],
            Range::new(2, 0, 2, 10),
            true,
            false,
            LambdaType::Lambda,
        ));

        collection.add(LambdaInfo::new(
            vec!["x".to_string(), "y".to_string()],
            Range::new(3, 0, 3, 10),
            false,
            true,
            LambdaType::AnonymousMethod,
        ));

        assert_eq!(collection.count(), 3);
        assert_eq!(collection.async_lambdas().len(), 1);
        assert_eq!(collection.simple_lambdas().len(), 1);
        assert_eq!(collection.statement_lambdas().len(), 1);
        assert_eq!(collection.expression_lambdas().len(), 2);
        assert_eq!(collection.anonymous_methods().len(), 1);
        assert_eq!(collection.lambda_expressions().len(), 2);
    }
}
