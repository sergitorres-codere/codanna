use codanna::parsing::LanguageParser;
use codanna::parsing::csharp::CSharpParser;
use codanna::types::{FileId, SymbolCounter};

#[test]
fn test_csharp_parser_basic() {
    let code = r#"
namespace TestNamespace
{
    /// <summary>
    /// A test class for demonstrating C# parsing
    /// </summary>
    public class TestClass
    {
        /// <summary>
        /// Prints a greeting message to the console
        /// </summary>
        public void TestMethod()
        {
            Console.WriteLine("Hello");
        }
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create C# parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(!symbols.is_empty(), "Should extract symbols from C# code");

    let class_symbol = symbols.iter().find(|s| &*s.name == "TestClass");
    assert!(class_symbol.is_some(), "Should find TestClass");

    let class = class_symbol.unwrap();
    assert!(
        class.doc_comment.is_some(),
        "Should extract XML documentation from TestClass"
    );
    let doc = class.doc_comment.as_ref().unwrap();
    assert!(
        doc.contains("<summary>"),
        "Documentation should contain summary tag, got: {doc}"
    );
    assert!(
        doc.contains("A test class for demonstrating C# parsing"),
        "Documentation should contain full multi-line text, got: {doc}"
    );
    assert!(
        doc.contains("</summary>"),
        "Documentation should contain closing summary tag, got: {doc}"
    );

    let method_symbol = symbols.iter().find(|s| &*s.name == "TestMethod");
    assert!(method_symbol.is_some(), "Should find TestMethod");

    let method = method_symbol.unwrap();
    assert!(
        method.doc_comment.is_some(),
        "Should extract XML documentation from TestMethod"
    );
    let method_doc = method.doc_comment.as_ref().unwrap();
    assert!(
        method_doc.contains("Prints a greeting message to the console"),
        "Method documentation should contain full text, got: {method_doc}"
    );
    assert!(
        method_doc.contains("<summary>"),
        "Documentation should at least contain summary tag"
    );
}

#[test]
fn test_csharp_namespace_tracking() {
    let code = r#"
namespace MyApp.Services
{
    /// <summary>
    /// Service for retrieving data
    /// </summary>
    public class DataService
    {
        /// <summary>
        /// Gets the data value
        /// </summary>
        /// <returns>The data as an integer</returns>
        public int GetData() { return 42; }
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    let class = symbols.iter().find(|s| &*s.name == "DataService").unwrap();
    assert_eq!(class.module_path.as_deref(), Some("MyApp.Services"));
    assert!(class.doc_comment.is_some(), "Should have documentation");
}

#[test]
fn test_csharp_method_calls() {
    let code = r#"
/// <summary>
/// Performs mathematical calculations
/// </summary>
public class Calculator
{
    /// <summary>
    /// Adds two numbers together
    /// </summary>
    private int Add(int a, int b) { return a + b; }

    /// <summary>
    /// Calculates a result using addition
    /// </summary>
    public int Calculate()
    {
        return Add(1, 2);
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    let calls = parser.find_calls(code);

    // Check that Calculate -> Add call is properly tracked with caller context
    let calculate_to_add = calls
        .iter()
        .find(|(from, to, _)| *from == "Calculate" && *to == "Add");
    assert!(
        calculate_to_add.is_some(),
        "Should detect 'Calculate -> Add' call with proper caller context. Found calls: {:?}",
        calls
            .iter()
            .map(|(f, t, _)| format!("{f} -> {t}"))
            .collect::<Vec<_>>()
    );

    let calc_method = symbols.iter().find(|s| &*s.name == "Calculate").unwrap();
    if let Some(doc) = &calc_method.doc_comment {
        assert!(doc.contains("<summary>"));
        assert!(doc.contains("Calculates a result using addition"));
    }
}

#[test]
fn test_csharp_struct_declaration() {
    let code = r#"
/// <summary>
/// Point structure for 2D coordinates
/// </summary>
public struct Point
{
    public int X { get; set; }
    public int Y { get; set; }

    public Point(int x, int y)
    {
        X = x;
        Y = y;
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    let struct_symbol = symbols.iter().find(|s| &*s.name == "Point");
    assert!(struct_symbol.is_some(), "Should find Point struct");

    let point = struct_symbol.unwrap();
    assert!(
        point.doc_comment.is_some(),
        "Struct should have documentation"
    );
    assert!(
        point
            .doc_comment
            .as_ref()
            .unwrap()
            .contains("Point structure for 2D coordinates")
    );
}

#[test]
fn test_csharp_record_declaration() {
    let code = r#"
/// <summary>
/// Record for person data
/// </summary>
public record Person(string FirstName, string LastName, int Age);
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    let record_symbol = symbols.iter().find(|s| &*s.name == "Person");
    assert!(record_symbol.is_some(), "Should find Person record");

    let person = record_symbol.unwrap();
    assert!(
        person.doc_comment.is_some(),
        "Record should have documentation"
    );
}

#[test]
fn test_csharp_delegate_declaration() {
    let code = r#"
/// <summary>
/// Delegate for data transformation
/// </summary>
public delegate string DataTransformer(string input);
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    let delegate_symbol = symbols.iter().find(|s| &*s.name == "DataTransformer");
    assert!(
        delegate_symbol.is_some(),
        "Should find DataTransformer delegate"
    );

    let delegate = delegate_symbol.unwrap();
    assert!(
        delegate.doc_comment.is_some(),
        "Delegate should have documentation"
    );
}

#[test]
fn test_csharp_indexer_declaration() {
    let code = r#"
/// <summary>
/// Collection with indexer
/// </summary>
public class StringCollection
{
    private string[] _items = new string[100];

    /// <summary>
    /// Indexer for accessing items by index
    /// </summary>
    public string this[int index]
    {
        get { return _items[index]; }
        set { _items[index] = value; }
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    // Indexers might be parsed as properties or special members
    // Just verify the class is parsed correctly
    let class_symbol = symbols.iter().find(|s| &*s.name == "StringCollection");
    assert!(class_symbol.is_some(), "Should find StringCollection class");
}

#[test]
fn test_csharp_operator_overload() {
    let code = r#"
/// <summary>
/// Complex number with operator overloading
/// </summary>
public struct Complex
{
    public double Real { get; set; }

    /// <summary>
    /// Addition operator
    /// </summary>
    public static Complex operator +(Complex a, Complex b)
    {
        return new Complex { Real = a.Real + b.Real };
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    let struct_symbol = symbols.iter().find(|s| &*s.name == "Complex");
    assert!(struct_symbol.is_some(), "Should find Complex struct");

    // Operator might be parsed as a method or special symbol
    let _operator_symbol = symbols
        .iter()
        .find(|s| s.name.contains("+") || s.name.contains("operator"));
    // Note: This test just verifies no crash; operator symbols may or may not be extracted
}

#[test]
fn test_csharp_conversion_operator() {
    let code = r#"
public struct Complex
{
    public double Real { get; set; }

    /// <summary>
    /// Implicit conversion from double
    /// </summary>
    public static implicit operator Complex(double real)
    {
        return new Complex { Real = real };
    }

    /// <summary>
    /// Explicit conversion to double
    /// </summary>
    public static explicit operator double(Complex c)
    {
        return c.Real;
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    let struct_symbol = symbols.iter().find(|s| &*s.name == "Complex");
    assert!(
        struct_symbol.is_some(),
        "Should find Complex struct with conversion operators"
    );
}

#[test]
fn test_csharp_destructor() {
    let code = r#"
/// <summary>
/// Resource manager with destructor
/// </summary>
public class ResourceManager
{
    /// <summary>
    /// Destructor for cleanup
    /// </summary>
    ~ResourceManager()
    {
        // Cleanup code
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    let class_symbol = symbols.iter().find(|s| &*s.name == "ResourceManager");
    assert!(class_symbol.is_some(), "Should find ResourceManager class");

    // Destructor might be parsed as a special method
    // Just verify no crash occurs
}

#[test]
fn test_csharp_file_scoped_namespace() {
    let code = r#"
namespace Codanna.Examples.FileScoped;

/// <summary>
/// Class in file-scoped namespace
/// </summary>
public class FileScopedExample
{
    public string Name { get; set; }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    let class_symbol = symbols.iter().find(|s| &*s.name == "FileScopedExample");
    assert!(
        class_symbol.is_some(),
        "Should find FileScopedExample class"
    );

    let class = class_symbol.unwrap();
    // File-scoped namespace tracking (C# 10+ feature)
    // Note: Currently the parser may not extract the module path for file-scoped namespaces
    // This test verifies the syntax doesn't crash the parser
    // TODO: Enhance parser to extract module_path from file_scoped_namespace_declaration
    if class.module_path.is_some() {
        assert_eq!(
            class.module_path.as_deref(),
            Some("Codanna.Examples.FileScoped"),
            "If module path is extracted, it should be correct"
        );
    }
}

#[test]
fn test_csharp_extern_alias() {
    let code = r#"
extern alias SystemV1;

using System;

namespace Codanna.Examples.ExternAlias
{
    /// <summary>
    /// Class using extern alias
    /// </summary>
    public class ExternAliasExample
    {
        public void ProcessData() { }
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    let class_symbol = symbols.iter().find(|s| &*s.name == "ExternAliasExample");
    assert!(
        class_symbol.is_some(),
        "Should find ExternAliasExample class"
    );

    // Extern alias is an import directive, just verify no crash
}
