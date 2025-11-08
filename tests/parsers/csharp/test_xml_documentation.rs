use codanna::parsing::csharp::{CSharpParser, XmlDocumentation};
use codanna::parsing::LanguageParser;
use codanna::types::{FileId, SymbolCounter};

#[test]
fn test_parse_xml_doc_from_symbol() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    /// <summary>
    /// A test class for demonstrating XML documentation parsing
    /// </summary>
    /// <remarks>This is a comprehensive example</remarks>
    public class Calculator
    {
        /// <summary>
        /// Adds two integers together
        /// </summary>
        /// <param name="a">First operand</param>
        /// <param name="b">Second operand</param>
        /// <returns>The sum of a and b</returns>
        public int Add(int a, int b)
        {
            return a + b;
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Find the Calculator class
    let calculator = symbols.iter().find(|s| &*s.name == "Calculator").unwrap();
    assert!(calculator.doc_comment.is_some());

    // Parse the XML documentation
    let xml_doc = parser.parse_xml_doc(calculator.doc_comment.as_ref().unwrap());

    assert_eq!(
        xml_doc.summary.as_deref(),
        Some("A test class for demonstrating XML documentation parsing")
    );
    assert_eq!(
        xml_doc.remarks.as_deref(),
        Some("This is a comprehensive example")
    );

    // Find the Add method
    let add_method = symbols.iter().find(|s| &*s.name == "Add").unwrap();
    assert!(add_method.doc_comment.is_some());

    // Parse the method's XML documentation
    let method_xml = parser.parse_xml_doc(add_method.doc_comment.as_ref().unwrap());

    assert_eq!(
        method_xml.summary.as_deref(),
        Some("Adds two integers together")
    );
    assert_eq!(method_xml.returns.as_deref(), Some("The sum of a and b"));
    assert_eq!(method_xml.params.len(), 2);
    assert_eq!(method_xml.params[0].name, "a");
    assert_eq!(method_xml.params[0].description, "First operand");
    assert_eq!(method_xml.params[1].name, "b");
    assert_eq!(method_xml.params[1].description, "Second operand");
}

#[test]
fn test_parse_xml_doc_with_generic_type_params() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
/// <summary>
/// A generic collection wrapper
/// </summary>
/// <typeparam name="T">The type of items to store</typeparam>
/// <typeparam name="TKey">The key type for indexing</typeparam>
public class Container<T, TKey>
{
    /// <summary>
    /// Adds an item with a key
    /// </summary>
    /// <typeparam name="TValue">The value type</typeparam>
    /// <param name="key">The indexing key</param>
    /// <param name="value">The value to store</param>
    public void Add<TValue>(TKey key, TValue value) { }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Find the Container class
    let container = symbols.iter().find(|s| &*s.name == "Container").unwrap();
    let xml_doc = parser.parse_xml_doc(container.doc_comment.as_ref().unwrap());

    assert_eq!(xml_doc.summary.as_deref(), Some("A generic collection wrapper"));
    assert_eq!(xml_doc.type_params.len(), 2);
    assert_eq!(xml_doc.type_params[0].name, "T");
    assert_eq!(xml_doc.type_params[0].description, "The type of items to store");
    assert_eq!(xml_doc.type_params[1].name, "TKey");
    assert_eq!(xml_doc.type_params[1].description, "The key type for indexing");

    // Find the Add method
    let add = symbols.iter().find(|s| &*s.name == "Add").unwrap();
    let method_xml = parser.parse_xml_doc(add.doc_comment.as_ref().unwrap());

    assert_eq!(method_xml.summary.as_deref(), Some("Adds an item with a key"));
    assert_eq!(method_xml.type_params.len(), 1);
    assert_eq!(method_xml.type_params[0].name, "TValue");
    assert_eq!(method_xml.type_params[0].description, "The value type");
}

#[test]
fn test_parse_xml_doc_with_exceptions() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class MathOperations
{
    /// <summary>
    /// Performs division operation
    /// </summary>
    /// <param name="numerator">The number to divide</param>
    /// <param name="denominator">The divisor</param>
    /// <returns>The quotient</returns>
    /// <exception cref="System.DivideByZeroException">Thrown when denominator is zero</exception>
    /// <exception cref="ArgumentException">Thrown for invalid arguments</exception>
    public int Divide(int numerator, int denominator)
    {
        if (denominator == 0)
            throw new DivideByZeroException();
        return numerator / denominator;
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let divide = symbols.iter().find(|s| &*s.name == "Divide").unwrap();
    let xml_doc = parser.parse_xml_doc(divide.doc_comment.as_ref().unwrap());

    assert_eq!(xml_doc.summary.as_deref(), Some("Performs division operation"));
    assert_eq!(xml_doc.returns.as_deref(), Some("The quotient"));

    assert_eq!(xml_doc.exceptions.len(), 2);
    assert_eq!(xml_doc.exceptions[0].cref, "System.DivideByZeroException");
    assert_eq!(xml_doc.exceptions[0].description, "Thrown when denominator is zero");
    assert_eq!(xml_doc.exceptions[1].cref, "ArgumentException");
    assert_eq!(xml_doc.exceptions[1].description, "Thrown for invalid arguments");
}

#[test]
fn test_parse_xml_doc_with_see_also_and_examples() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
/// <summary>
/// Main calculation engine
/// </summary>
/// <remarks>
/// This class handles complex calculations
/// </remarks>
/// <example>
/// var engine = new CalculationEngine();
/// var result = engine.Calculate(10);
/// </example>
/// <seealso cref="Calculator"/>
/// <seealso cref="System.Math"/>
public class CalculationEngine
{
    public int Calculate(int value) { return value * 2; }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let engine = symbols.iter().find(|s| &*s.name == "CalculationEngine").unwrap();
    let xml_doc = parser.parse_xml_doc(engine.doc_comment.as_ref().unwrap());

    assert_eq!(xml_doc.summary.as_deref(), Some("Main calculation engine"));
    assert!(xml_doc.remarks.is_some());
    assert!(xml_doc.remarks.as_ref().unwrap().contains("complex calculations"));

    assert_eq!(xml_doc.examples.len(), 1);
    assert!(xml_doc.examples[0].contains("new CalculationEngine()"));
    assert!(xml_doc.examples[0].contains("Calculate(10)"));

    assert_eq!(xml_doc.see_also.len(), 2);
    assert_eq!(xml_doc.see_also[0], "Calculator");
    assert_eq!(xml_doc.see_also[1], "System.Math");
}

#[test]
fn test_parse_xml_doc_property_with_value_tag() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class Person
{
    /// <summary>
    /// Gets or sets the person's name
    /// </summary>
    /// <value>The full name as a string</value>
    public string Name { get; set; }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let name_property = symbols.iter().find(|s| &*s.name == "Name").unwrap();
    let xml_doc = parser.parse_xml_doc(name_property.doc_comment.as_ref().unwrap());

    assert_eq!(xml_doc.summary.as_deref(), Some("Gets or sets the person's name"));
    assert_eq!(xml_doc.value.as_deref(), Some("The full name as a string"));
}

#[test]
fn test_parse_xml_doc_multiline_content() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class DataProcessor
{
    /// <summary>
    /// This is a method with
    /// a very long description
    /// that spans multiple lines
    /// </summary>
    /// <param name="data">
    /// This parameter also has
    /// a multi-line description
    /// </param>
    /// <returns>
    /// Returns a complex object
    /// with multiple properties
    /// </returns>
    public object ProcessData(string data) { return null; }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let method = symbols.iter().find(|s| &*s.name == "ProcessData").unwrap();
    let xml_doc = parser.parse_xml_doc(method.doc_comment.as_ref().unwrap());

    // Check that multiline content is preserved
    let summary = xml_doc.summary.as_ref().unwrap();
    assert!(summary.contains("very long description"));
    assert!(summary.contains("spans multiple lines"));

    assert_eq!(xml_doc.params.len(), 1);
    assert!(xml_doc.params[0].description.contains("multi-line description"));

    let returns = xml_doc.returns.as_ref().unwrap();
    assert!(returns.contains("complex object"));
    assert!(returns.contains("multiple properties"));
}

#[test]
fn test_direct_xml_documentation_parse() {
    // Test the XmlDocumentation::parse method directly
    let raw = r#"
/// <summary>Test summary</summary>
/// <param name="x">First param</param>
/// <param name="y">Second param</param>
/// <returns>Some value</returns>
"#;

    let doc = XmlDocumentation::parse(raw);

    assert_eq!(doc.summary.as_deref(), Some("Test summary"));
    assert_eq!(doc.params.len(), 2);
    assert_eq!(doc.params[0].name, "x");
    assert_eq!(doc.params[1].name, "y");
    assert_eq!(doc.returns.as_deref(), Some("Some value"));
    assert!(!doc.is_empty());
}

#[test]
fn test_empty_xml_documentation() {
    let raw = "/// Just a comment without tags";
    let doc = XmlDocumentation::parse(raw);

    assert!(doc.is_empty());
    assert_eq!(doc.raw, raw);
    assert!(doc.summary.is_none());
}

#[test]
fn test_partial_xml_documentation() {
    // Documentation with only some tags
    let raw = r#"
/// <summary>Only has summary</summary>
"#;

    let doc = XmlDocumentation::parse(raw);

    assert!(!doc.is_empty());
    assert_eq!(doc.summary.as_deref(), Some("Only has summary"));
    assert!(doc.params.is_empty());
    assert!(doc.returns.is_none());
}
