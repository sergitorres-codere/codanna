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
fn test_csharp_operator_overloading_arithmetic() {
    let code = r#"
/// <summary>
/// A vector class demonstrating operator overloading
/// </summary>
public class Vector
{
    public double X { get; set; }
    public double Y { get; set; }

    /// <summary>
    /// Adds two vectors together
    /// </summary>
    public static Vector operator +(Vector a, Vector b)
    {
        return new Vector { X = a.X + b.X, Y = a.Y + b.Y };
    }

    /// <summary>
    /// Subtracts one vector from another
    /// </summary>
    public static Vector operator -(Vector a, Vector b)
    {
        return new Vector { X = a.X - b.X, Y = a.Y - b.Y };
    }

    /// <summary>
    /// Multiplies a vector by a scalar
    /// </summary>
    public static Vector operator *(Vector v, double scalar)
    {
        return new Vector { X = v.X * scalar, Y = v.Y * scalar };
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    // Check that operator+ is detected
    let plus_op = symbols.iter().find(|s| &*s.name == "operator+");
    assert!(
        plus_op.is_some(),
        "Should detect operator+ overload. Found symbols: {:?}",
        symbols.iter().map(|s| &*s.name).collect::<Vec<_>>()
    );

    // Verify operator+ has proper signature
    let plus_symbol = plus_op.unwrap();
    assert!(
        plus_symbol.signature.is_some(),
        "Operator+ should have a signature"
    );
    let sig = plus_symbol.signature.as_ref().unwrap();
    assert!(
        sig.contains("operator"),
        "Signature should contain 'operator', got: {sig}"
    );
    assert!(
        sig.contains("+"),
        "Signature should contain '+', got: {sig}"
    );

    // Verify documentation
    assert!(
        plus_symbol.doc_comment.is_some(),
        "Operator+ should have documentation"
    );
    let doc = plus_symbol.doc_comment.as_ref().unwrap();
    assert!(
        doc.contains("Adds two vectors together"),
        "Documentation should be preserved, got: {doc}"
    );

    // Check that operator- is detected
    let minus_op = symbols.iter().find(|s| &*s.name == "operator-");
    assert!(minus_op.is_some(), "Should detect operator- overload");

    // Check that operator* is detected
    let mult_op = symbols.iter().find(|s| &*s.name == "operator*");
    assert!(mult_op.is_some(), "Should detect operator* overload");
}

#[test]
fn test_csharp_operator_overloading_comparison() {
    let code = r#"
public class Point
{
    public int X { get; set; }
    public int Y { get; set; }

    /// <summary>
    /// Determines if two points are equal
    /// </summary>
    public static bool operator ==(Point a, Point b)
    {
        return a.X == b.X && a.Y == b.Y;
    }

    /// <summary>
    /// Determines if two points are not equal
    /// </summary>
    public static bool operator !=(Point a, Point b)
    {
        return !(a == b);
    }

    public override bool Equals(object obj)
    {
        if (obj is Point p) return this == p;
        return false;
    }

    public override int GetHashCode()
    {
        return X.GetHashCode() ^ Y.GetHashCode();
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    // Check that operator== is detected
    let eq_op = symbols.iter().find(|s| &*s.name == "operator==");
    assert!(
        eq_op.is_some(),
        "Should detect operator== overload. Found symbols: {:?}",
        symbols.iter().map(|s| &*s.name).collect::<Vec<_>>()
    );

    // Verify operator== has documentation
    let eq_symbol = eq_op.unwrap();
    assert!(
        eq_symbol.doc_comment.is_some(),
        "Operator== should have documentation"
    );
    let doc = eq_symbol.doc_comment.as_ref().unwrap();
    assert!(
        doc.contains("equal"),
        "Documentation should mention equality, got: {doc}"
    );

    // Check that operator!= is detected
    let neq_op = symbols.iter().find(|s| &*s.name == "operator!=");
    assert!(neq_op.is_some(), "Should detect operator!= overload");

    // Verify both operators return bool
    let eq_sig = eq_symbol.signature.as_ref().unwrap();
    assert!(
        eq_sig.contains("bool"),
        "Operator== should return bool, got: {eq_sig}"
    );
}

#[test]
fn test_csharp_operator_overloading_unary() {
    let code = r#"
public class Counter
{
    public int Value { get; set; }

    /// <summary>
    /// Increments the counter
    /// </summary>
    public static Counter operator ++(Counter c)
    {
        return new Counter { Value = c.Value + 1 };
    }

    /// <summary>
    /// Decrements the counter
    /// </summary>
    public static Counter operator --(Counter c)
    {
        return new Counter { Value = c.Value - 1 };
    }

    /// <summary>
    /// Negates the counter value
    /// </summary>
    public static Counter operator -(Counter c)
    {
        return new Counter { Value = -c.Value };
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    // Check that operator++ is detected
    let inc_op = symbols.iter().find(|s| &*s.name == "operator++");
    assert!(
        inc_op.is_some(),
        "Should detect operator++ overload. Found symbols: {:?}",
        symbols.iter().map(|s| &*s.name).collect::<Vec<_>>()
    );

    // Check that operator-- is detected
    let dec_op = symbols.iter().find(|s| &*s.name == "operator--");
    assert!(dec_op.is_some(), "Should detect operator-- overload");

    // Check that unary operator- is detected (same name as binary minus)
    let neg_ops: Vec<_> = symbols.iter().filter(|s| &*s.name == "operator-").collect();
    assert!(!neg_ops.is_empty(), "Should detect operator- overload");
}

#[test]
fn test_csharp_operator_overloading_advanced() {
    let code = r#"
public class Matrix
{
    private double[,] data;

    /// <summary>
    /// Performs matrix multiplication
    /// </summary>
    public static Matrix operator *(Matrix a, Matrix b)
    {
        // Matrix multiplication logic
        return new Matrix();
    }

    /// <summary>
    /// Converts matrix to boolean (true if non-zero)
    /// </summary>
    public static bool operator true(Matrix m)
    {
        return m != null && m.data != null;
    }

    /// <summary>
    /// Converts matrix to boolean (false if zero)
    /// </summary>
    public static bool operator false(Matrix m)
    {
        return m == null || m.data == null;
    }

    /// <summary>
    /// Accesses matrix elements by index
    /// </summary>
    public double this[int row, int col]
    {
        get { return data[row, col]; }
        set { data[row, col] = value; }
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    // Check that operator* is detected
    let mult_op = symbols.iter().find(|s| &*s.name == "operator*");
    assert!(mult_op.is_some(), "Should detect operator* overload");

    // Check that operator true is detected
    let true_op = symbols.iter().find(|s| &*s.name == "operator true");
    assert!(
        true_op.is_some(),
        "Should detect operator true overload. Found symbols: {:?}",
        symbols.iter().map(|s| &*s.name).collect::<Vec<_>>()
    );

    // Check that operator false is detected
    let false_op = symbols.iter().find(|s| &*s.name == "operator false");
    assert!(false_op.is_some(), "Should detect operator false overload");
}

#[test]
fn test_csharp_async_method_signatures() {
    let code = r#"
public class DataService
{
    /// <summary>
    /// Asynchronously fetches data from the server
    /// </summary>
    public async Task<string> GetDataAsync()
    {
        await Task.Delay(100);
        return "data";
    }

    /// <summary>
    /// Asynchronously saves data to the server
    /// </summary>
    public async Task SaveDataAsync(string data)
    {
        await Task.Delay(50);
        // Save logic here
    }

    /// <summary>
    /// Asynchronously processes multiple items
    /// </summary>
    private async Task<List<int>> ProcessItemsAsync(int count)
    {
        await Task.Delay(10);
        return new List<int>();
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    // Check that async method is detected
    let get_data_method = symbols.iter().find(|s| &*s.name == "GetDataAsync");
    assert!(
        get_data_method.is_some(),
        "Should detect GetDataAsync method. Found symbols: {:?}",
        symbols.iter().map(|s| &*s.name).collect::<Vec<_>>()
    );

    // Verify async modifier appears in signature
    let method = get_data_method.unwrap();
    let sig = method.signature.as_ref().unwrap();
    assert!(
        sig.contains("async"),
        "Method signature should include 'async' modifier, got: {sig}"
    );
    assert!(
        sig.contains("Task"),
        "Method signature should include Task return type, got: {sig}"
    );
    assert!(
        sig.contains("<string>"),
        "Method signature should include generic type parameter, got: {sig}"
    );

    // Check SaveDataAsync with Task return type (no result)
    let save_data_method = symbols.iter().find(|s| &*s.name == "SaveDataAsync");
    assert!(
        save_data_method.is_some(),
        "Should detect SaveDataAsync method"
    );
    let save_sig = save_data_method.unwrap().signature.as_ref().unwrap();
    assert!(
        save_sig.contains("async"),
        "SaveDataAsync should have async modifier, got: {save_sig}"
    );
    assert!(
        save_sig.contains("Task"),
        "SaveDataAsync should have Task return type, got: {save_sig}"
    );

    // Check private async method
    let process_method = symbols.iter().find(|s| &*s.name == "ProcessItemsAsync");
    assert!(
        process_method.is_some(),
        "Should detect ProcessItemsAsync method"
    );
    let process_sig = process_method.unwrap().signature.as_ref().unwrap();
    assert!(
        process_sig.contains("async"),
        "ProcessItemsAsync should have async modifier, got: {process_sig}"
    );
}

#[test]
fn test_csharp_async_interface_and_implementation() {
    let code = r#"
/// <summary>
/// Interface for asynchronous data operations
/// </summary>
public interface IDataRepository
{
    /// <summary>
    /// Gets an item by ID asynchronously
    /// </summary>
    Task<Item> GetByIdAsync(int id);

    /// <summary>
    /// Saves an item asynchronously
    /// </summary>
    Task SaveAsync(Item item);
}

public class DataRepository : IDataRepository
{
    public async Task<Item> GetByIdAsync(int id)
    {
        await Task.Delay(10);
        return new Item();
    }

    public async Task SaveAsync(Item item)
    {
        await Task.Delay(10);
    }
}

public class Item
{
    public int Id { get; set; }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    // Check interface method declarations have Task return types
    let interface_methods: Vec<_> = symbols
        .iter()
        .filter(|s| {
            s.signature
                .as_ref()
                .is_some_and(|sig| sig.contains("Task"))
        })
        .collect();

    assert!(
        interface_methods.len() >= 4,
        "Should find at least 4 methods with Task return types (2 in interface + 2 in implementation), found: {}",
        interface_methods.len()
    );

    // Check that implementation methods have async modifier
    let get_by_id_impl = symbols.iter().find(|s| {
        &*s.name == "GetByIdAsync"
            && s.signature
                .as_ref()
                .is_some_and(|sig| sig.contains("async"))
    });
    assert!(
        get_by_id_impl.is_some(),
        "Should find async GetByIdAsync implementation"
    );

    // Check interface implementation tracking
    let implementations = parser.find_implementations(code);
    assert!(
        implementations
            .iter()
            .any(|(from, to, _)| *from == "DataRepository" && *to == "IDataRepository"),
        "Should detect DataRepository implements IDataRepository"
    );
}

#[test]
fn test_csharp_async_await_expressions() {
    let code = r#"
public class AsyncService
{
    /// <summary>
    /// Chains multiple async operations
    /// </summary>
    public async Task<string> ChainOperationsAsync()
    {
        var step1 = await Step1Async();
        var step2 = await Step2Async(step1);
        var step3 = await Step3Async(step2);
        return step3;
    }

    private async Task<string> Step1Async()
    {
        await Task.Delay(10);
        return "step1";
    }

    private async Task<string> Step2Async(string input)
    {
        await Task.Delay(10);
        return input + "_step2";
    }

    private async Task<string> Step3Async(string input)
    {
        await Task.Delay(10);
        return input + "_step3";
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    // Verify all async methods are detected
    let async_methods: Vec<_> = symbols
        .iter()
        .filter(|s| {
            s.signature
                .as_ref()
                .is_some_and(|sig| sig.contains("async"))
        })
        .collect();

    assert!(
        async_methods.len() >= 4,
        "Should detect all 4 async methods, found: {}",
        async_methods.len()
    );

    // Check that ChainOperationsAsync is detected
    let chain_method = symbols.iter().find(|s| &*s.name == "ChainOperationsAsync");
    assert!(
        chain_method.is_some(),
        "Should detect ChainOperationsAsync method"
    );

    // Verify method calls are tracked (await calls to other async methods)
    let method_calls = parser.find_method_calls(code);

    // Should find calls from ChainOperationsAsync to Step1Async, Step2Async, Step3Async
    let chain_calls: Vec<_> = method_calls
        .iter()
        .filter(|c| c.caller == "ChainOperationsAsync")
        .collect();

    assert!(
        chain_calls.len() >= 3,
        "Should find at least 3 method calls from ChainOperationsAsync, found: {}",
        chain_calls.len()
    );
}

#[test]
fn test_csharp_async_void_and_task_variations() {
    let code = r#"
public class EventHandlers
{
    /// <summary>
    /// Async event handler (async void - not recommended but valid)
    /// </summary>
    public async void OnButtonClick()
    {
        await Task.Delay(100);
    }

    /// <summary>
    /// Returns ValueTask for performance
    /// </summary>
    public async ValueTask<int> GetCachedValueAsync()
    {
        await Task.Yield();
        return 42;
    }

    /// <summary>
    /// Uses ConfigureAwait for library code
    /// </summary>
    public async Task<string> LibraryMethodAsync()
    {
        await Task.Delay(10).ConfigureAwait(false);
        return "result";
    }

    /// <summary>
    /// Returns Task.FromResult for synchronous completion
    /// </summary>
    public Task<int> GetImmediateValueAsync()
    {
        return Task.FromResult(100);
    }
}
"#;

    let mut parser = CSharpParser::new().expect("Failed to create parser");
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(code, file_id, &mut counter);

    // Check async void method
    let async_void_method = symbols.iter().find(|s| &*s.name == "OnButtonClick");
    assert!(
        async_void_method.is_some(),
        "Should detect async void method"
    );
    let async_void_sig = async_void_method.unwrap().signature.as_ref().unwrap();
    assert!(
        async_void_sig.contains("async"),
        "OnButtonClick should have async modifier, got: {async_void_sig}"
    );

    // Check ValueTask method
    let value_task_method = symbols.iter().find(|s| &*s.name == "GetCachedValueAsync");
    assert!(
        value_task_method.is_some(),
        "Should detect ValueTask method"
    );
    let value_task_sig = value_task_method.unwrap().signature.as_ref().unwrap();
    assert!(
        value_task_sig.contains("ValueTask"),
        "GetCachedValueAsync should have ValueTask return type, got: {value_task_sig}"
    );

    // Check ConfigureAwait method
    let configure_await_method = symbols.iter().find(|s| &*s.name == "LibraryMethodAsync");
    assert!(
        configure_await_method.is_some(),
        "Should detect LibraryMethodAsync"
    );

    // Check non-async method returning Task
    let sync_task_method = symbols
        .iter()
        .find(|s| &*s.name == "GetImmediateValueAsync");
    assert!(
        sync_task_method.is_some(),
        "Should detect GetImmediateValueAsync"
    );
    let sync_task_sig = sync_task_method.unwrap().signature.as_ref().unwrap();
    assert!(
        sync_task_sig.contains("Task"),
        "GetImmediateValueAsync should have Task return type, got: {sync_task_sig}"
    );
    // This method should NOT have async modifier
    assert!(
        !sync_task_sig.contains("async"),
        "GetImmediateValueAsync should NOT have async modifier (it's synchronous), got: {sync_task_sig}"
    );
}
