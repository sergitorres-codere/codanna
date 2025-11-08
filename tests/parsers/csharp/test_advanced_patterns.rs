use codanna::parsing::csharp::CSharpParser;
use codanna::parsing::LanguageParser;
use codanna::types::{FileId, SymbolCounter};

/// Test pattern matching with 'is' expressions
#[test]
fn test_pattern_matching_is_expression() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class PatternMatcher
    {
        public string GetType(object value)
        {
            if (value is int number)
                return $"Integer: {number}";

            if (value is string { Length: > 0 } text)
                return $"String: {text}";

            if (value is null)
                return "Null";

            return "Unknown";
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let method = symbols.iter().find(|s| &*s.name == "GetType").unwrap();
    assert!(method.signature.is_some());
}

/// Test switch expressions (C# 8+)
#[test]
fn test_switch_expressions() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Calculator
    {
        public int Calculate(string operation, int a, int b) => operation switch
        {
            "+" => a + b,
            "-" => a - b,
            "*" => a * b,
            "/" when b != 0 => a / b,
            _ => throw new ArgumentException()
        };

        public string GetDescription(object obj) => obj switch
        {
            int n when n > 0 => "Positive number",
            int n when n < 0 => "Negative number",
            string s => $"String of length {s.Length}",
            null => "Null value",
            _ => "Unknown type"
        };
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "Calculate"));
    assert!(symbols.iter().any(|s| &*s.name == "GetDescription"));
}

/// Test record types (C# 9+)
#[test]
fn test_record_types() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp.Models
{
    public record Person(string FirstName, string LastName, int Age);

    public record Employee(string FirstName, string LastName, int Age, string Department)
        : Person(FirstName, LastName, Age);

    public record struct Point(int X, int Y);

    public record Customer
    {
        public string Id { get; init; }
        public string Name { get; init; }
        public DateTime Created { get; init; } = DateTime.Now;
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // All record types should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "Person"));
    assert!(symbols.iter().any(|s| &*s.name == "Employee"));
    assert!(symbols.iter().any(|s| &*s.name == "Point"));
    assert!(symbols.iter().any(|s| &*s.name == "Customer"));
}

/// Test local functions
#[test]
fn test_local_functions() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Calculator
    {
        public int Fibonacci(int n)
        {
            return Calculate(n);

            int Calculate(int num)
            {
                if (num <= 1) return num;
                return Calculate(num - 1) + Calculate(num - 2);
            }
        }

        public void ProcessData()
        {
            void LogMessage(string message) => Console.WriteLine(message);

            async Task<int> FetchDataAsync()
            {
                await Task.Delay(100);
                return 42;
            }

            LogMessage("Starting");
            var result = FetchDataAsync().Result;
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Main methods should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "Fibonacci"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessData"));

    // Local functions may or may not be extracted depending on parser implementation
    // The main test is that methods containing local functions are parsed without errors
}

/// Test nullable reference types (C# 8+)
#[test]
fn test_nullable_reference_types() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
#nullable enable
namespace MyApp
{
    public class UserService
    {
        public string? FindUser(int id)
        {
            return id > 0 ? "User" : null;
        }

        public void ProcessUser(string? name)
        {
            if (name is not null)
            {
                Console.WriteLine(name.Length);
            }
        }

        public string GetName(User? user) => user?.Name ?? "Unknown";
    }

    public class User
    {
        public string Name { get; set; } = string.Empty;
        public string? OptionalEmail { get; set; }
    }
}
#nullable restore
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // All methods and classes should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "UserService"));
    assert!(symbols.iter().any(|s| &*s.name == "FindUser"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessUser"));
    assert!(symbols.iter().any(|s| &*s.name == "GetName"));
    assert!(symbols.iter().any(|s| &*s.name == "User"));
}

/// Test tuple deconstruction and patterns
#[test]
fn test_tuple_patterns() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class TupleProcessor
    {
        public (int sum, int product) Calculate(int a, int b)
        {
            return (a + b, a * b);
        }

        public void ProcessTuple()
        {
            var (sum, product) = Calculate(5, 10);
            Console.WriteLine($"Sum: {sum}, Product: {product}");
        }

        public string GetQuadrant(int x, int y) => (x, y) switch
        {
            (> 0, > 0) => "Quadrant I",
            (< 0, > 0) => "Quadrant II",
            (< 0, < 0) => "Quadrant III",
            (> 0, < 0) => "Quadrant IV",
            _ => "Origin or axis"
        };
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "Calculate"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessTuple"));
    assert!(symbols.iter().any(|s| &*s.name == "GetQuadrant"));
}

/// Test init-only properties (C# 9+)
#[test]
fn test_init_only_properties() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp.Models
{
    public class Person
    {
        public string FirstName { get; init; }
        public string LastName { get; init; }
        public int Age { get; init; }

        public string FullName { get => $"{FirstName} {LastName}"; init => throw new NotSupportedException(); }
    }

    public class ImmutableData
    {
        public int Id { get; init; } = 0;
        public string Value { get; init; } = "";
        public DateTime Created { get; init; } = DateTime.Now;
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Classes and properties should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "Person"));
    assert!(symbols.iter().any(|s| &*s.name == "FirstName"));
    assert!(symbols.iter().any(|s| &*s.name == "LastName"));
    assert!(symbols.iter().any(|s| &*s.name == "ImmutableData"));
}

/// Test property patterns
#[test]
fn test_property_patterns() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class PatternMatcher
    {
        public string DescribePoint(Point point) => point switch
        {
            { X: 0, Y: 0 } => "Origin",
            { X: var x, Y: 0 } => $"On X-axis at {x}",
            { X: 0, Y: var y } => $"On Y-axis at {y}",
            { X: > 0, Y: > 0 } => "Quadrant I",
            _ => "Other"
        };

        public bool IsValid(User user) => user is { Name.Length: > 0, Age: >= 18 };

        public void ProcessUser(User user)
        {
            if (user is { IsActive: true, Role: "Admin" })
            {
                GrantAdminAccess();
            }
        }

        private void GrantAdminAccess() { }
    }

    public class Point
    {
        public int X { get; set; }
        public int Y { get; set; }
    }

    public class User
    {
        public string Name { get; set; }
        public int Age { get; set; }
        public bool IsActive { get; set; }
        public string Role { get; set; }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "DescribePoint"));
    assert!(symbols.iter().any(|s| &*s.name == "IsValid"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessUser"));
}

/// Test target-typed new expressions (C# 9+)
#[test]
fn test_target_typed_new() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Container
    {
        private List<string> items = new();
        private Dictionary<string, int> map = new();

        public void Process()
        {
            Point p = new(10, 20);
            List<int> numbers = new() { 1, 2, 3 };

            ProcessPoint(new(5, 10));
        }

        private void ProcessPoint(Point point) { }
    }

    public class Point
    {
        public int X { get; set; }
        public int Y { get; set; }
        public Point(int x, int y) { X = x; Y = y; }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "Container"));
    assert!(symbols.iter().any(|s| &*s.name == "Process"));
}

/// Test static local functions (C# 8+)
#[test]
fn test_static_local_functions() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Calculator
    {
        public int ProcessData(int value)
        {
            int multiplier = 10;

            static int Square(int x)
            {
                return x * x;
            }

            int MultiplyWithCapture(int x)
            {
                return x * multiplier;
            }

            return Square(value) + MultiplyWithCapture(value);
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Main method should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "ProcessData"));

    // Local functions might be extracted
    // Static local function shouldn't capture variables
}

/// Test with expressions and records (C# 10+)
#[test]
fn test_with_expressions() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public record Person(string Name, int Age);

    public class PersonProcessor
    {
        public Person UpdateAge(Person person, int newAge)
        {
            return person with { Age = newAge };
        }

        public void ProcessPeople()
        {
            var person1 = new Person("Alice", 30);
            var person2 = person1 with { Name = "Bob" };
            var person3 = person1 with { Age = person1.Age + 1 };
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "Person"));
    assert!(symbols.iter().any(|s| &*s.name == "UpdateAge"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessPeople"));
}

/// Test global using directives (C# 10+)
#[test]
fn test_global_using_directives() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
global using System;
global using System.Collections.Generic;
global using System.Linq;

namespace MyApp
{
    public class DataProcessor
    {
        public void Process()
        {
            var list = new List<int>();
            var filtered = list.Where(x => x > 0);
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let imports = parser.find_imports(code, file_id);

    // Global usings should be extracted
    assert!(imports.iter().any(|i| i.path == "System"));
    assert!(imports.iter().any(|i| i.path == "System.Collections.Generic"));
    assert!(imports.iter().any(|i| i.path == "System.Linq"));
}

/// Test file-scoped namespaces (C# 10+)
#[test]
fn test_file_scoped_namespace() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp.Services;

public class UserService
{
    public void CreateUser(string name) { }
}

public class ProductService
{
    public void CreateProduct(string name) { }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Both services should be parsed (namespace handling may vary)
    let user_service = symbols.iter().find(|s| &*s.name == "UserService");
    assert!(user_service.is_some(), "UserService should be parsed");

    let product_service = symbols.iter().find(|s| &*s.name == "ProductService");
    assert!(product_service.is_some(), "ProductService should be parsed");

    // File-scoped namespaces may or may not set module_path depending on parser support
    // The main test is that classes in file-scoped namespaces are parsed without errors
}

/// Test primary constructors (C# 12+)
#[test]
fn test_primary_constructors() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Person(string firstName, string lastName)
    {
        public string FullName => $"{firstName} {lastName}";

        public void Display()
        {
            Console.WriteLine($"{firstName} {lastName}");
        }
    }

    public class Point(int x, int y)
    {
        public int X { get; } = x;
        public int Y { get; } = y;

        public double Distance => Math.Sqrt(x * x + y * y);
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Classes with primary constructors should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "Person"));
    assert!(symbols.iter().any(|s| &*s.name == "Point"));
}

/// Test collection expressions (C# 12+)
#[test]
fn test_collection_expressions() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class CollectionDemo
    {
        public void UseCollections()
        {
            int[] array = [1, 2, 3, 4, 5];
            List<string> list = ["apple", "banana", "cherry"];
            Span<int> span = [10, 20, 30];

            ProcessItems([1, 2, 3]);
        }

        private void ProcessItems(int[] items) { }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Methods using collection expressions should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "UseCollections"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessItems"));
}

/// Test required members (C# 11+)
#[test]
fn test_required_members() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp.Models
{
    public class Person
    {
        public required string FirstName { get; init; }
        public required string LastName { get; init; }
        public int Age { get; init; }
    }

    public class User
    {
        public required int Id { get; set; }
        public required string Username { get; set; }
        public string? Email { get; set; }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Classes with required properties should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "Person"));
    assert!(symbols.iter().any(|s| &*s.name == "User"));
    assert!(symbols.iter().any(|s| &*s.name == "FirstName"));
    assert!(symbols.iter().any(|s| &*s.name == "LastName"));
}
