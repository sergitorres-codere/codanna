use codanna::parsing::csharp::CSharpParser;
use codanna::parsing::LanguageParser;
use codanna::types::{FileId, SymbolCounter};

/// Test that methods containing lambda expressions are properly parsed
#[test]
fn test_method_with_simple_lambda() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Calculator
    {
        public int[] DoubleAll(int[] numbers)
        {
            return numbers.Select(x => x * 2).ToArray();
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // The method containing the lambda should be parsed
    let double_all = symbols.iter().find(|s| &*s.name == "DoubleAll").unwrap();
    assert_eq!(double_all.module_path.as_deref(), Some("MyApp"));
}

/// Test lambda expressions in LINQ queries
#[test]
fn test_linq_with_lambdas() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp.Data
{
    public class UserRepository
    {
        public List<User> GetActiveUsers(List<User> users)
        {
            return users
                .Where(u => u.IsActive)
                .OrderBy(u => u.Name)
                .Select(u => new { u.Id, u.Name })
                .ToList();
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Verify the repository class and method are parsed
    let repo = symbols.iter().find(|s| &*s.name == "UserRepository").unwrap();
    assert_eq!(repo.module_path.as_deref(), Some("MyApp.Data"));

    let method = symbols.iter().find(|s| &*s.name == "GetActiveUsers").unwrap();
    assert!(method.signature.is_some());
}

/// Test lambda with multiple parameters
#[test]
fn test_lambda_with_multiple_parameters() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class MathOperations
    {
        public int Aggregate(int[] numbers)
        {
            return numbers.Aggregate((sum, x) => sum + x);
        }

        public void ProcessPairs()
        {
            var pairs = list.Select((item, index) => new { item, index });
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Both methods should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "Aggregate"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessPairs"));
}

/// Test lambda with statement body
#[test]
fn test_lambda_with_statement_body() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class DataProcessor
    {
        public void Process(List<string> items)
        {
            items.ForEach(item => {
                var trimmed = item.Trim();
                Console.WriteLine(trimmed);
                LogItem(trimmed);
            });
        }

        private void LogItem(string item) { }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Method containing lambda with statement body should be parsed
    let process = symbols.iter().find(|s| &*s.name == "Process").unwrap();
    assert!(process.signature.is_some());

    // Method called from within lambda
    assert!(symbols.iter().any(|s| &*s.name == "LogItem"));
}

/// Test anonymous method (delegate syntax)
#[test]
fn test_anonymous_method_delegate() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class EventHandler
    {
        public void SetupHandler()
        {
            button.Click += delegate(object sender, EventArgs e)
            {
                Console.WriteLine("Button clicked");
                HandleClick();
            };
        }

        private void HandleClick() { }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Method containing anonymous delegate should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "SetupHandler"));
    assert!(symbols.iter().any(|s| &*s.name == "HandleClick"));
}

/// Test lambda assigned to local variable
#[test]
fn test_lambda_assigned_to_variable() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class FunctionContainer
    {
        public void DefineOperations()
        {
            Func<int, int> square = x => x * x;
            Func<int, int, int> add = (a, b) => a + b;
            Action<string> log = message => Console.WriteLine(message);

            var result = square(5);
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Method defining lambda variables should be parsed
    let method = symbols.iter().find(|s| &*s.name == "DefineOperations").unwrap();
    assert_eq!(method.module_path.as_deref(), Some("MyApp"));
}

/// Test nested lambdas
#[test]
fn test_nested_lambdas() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class QueryBuilder
    {
        public void BuildComplexQuery()
        {
            var result = users
                .Select(user => user.Orders
                    .Where(order => order.Total > 100)
                    .Select(order => order.Items
                        .Where(item => item.IsAvailable)
                    )
                );
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Method with nested lambdas should be parsed
    let method = symbols.iter().find(|s| &*s.name == "BuildComplexQuery").unwrap();
    assert!(method.signature.is_some());
}

/// Test lambda in method call chain
#[test]
fn test_lambda_in_method_chain() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp.Services
{
    public class DataService
    {
        public IEnumerable<Result> GetResults()
        {
            return database
                .Query()
                .Where(x => x.IsValid())
                .OrderBy(x => x.Priority)
                .ThenBy(x => x.Date)
                .Select(x => Transform(x))
                .ToList();
        }

        private Result Transform(object data) { return null; }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Verify method chaining with lambdas is parsed
    assert!(symbols.iter().any(|s| &*s.name == "GetResults"));
    assert!(symbols.iter().any(|s| &*s.name == "Transform"));
}

/// Test lambda capturing local variables
#[test]
fn test_lambda_capturing_locals() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Calculator
    {
        public Func<int, int> CreateMultiplier(int factor)
        {
            var offset = 10;
            return x => (x * factor) + offset;
        }

        public void ProcessWithCapture()
        {
            int threshold = 100;
            var filtered = items.Where(x => x.Value > threshold);
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Methods with variable-capturing lambdas should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "CreateMultiplier"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessWithCapture"));
}

/// Test async lambda expressions
#[test]
fn test_async_lambda() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class AsyncProcessor
    {
        public async Task ProcessAsync()
        {
            var tasks = items.Select(async item =>
            {
                await Task.Delay(100);
                return await ProcessItem(item);
            });

            await Task.WhenAll(tasks);
        }

        private async Task<string> ProcessItem(object item)
        {
            return await Task.FromResult("processed");
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Async methods with async lambdas should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "ProcessAsync"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessItem"));
}

/// Test lambda in event subscription
#[test]
fn test_lambda_in_event_handlers() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp.UI
{
    public class ButtonHandler
    {
        public void Initialize()
        {
            button.Click += (sender, e) => HandleClick();
            button.MouseEnter += (s, e) => {
                Console.WriteLine("Mouse entered");
                UpdateUI();
            };
        }

        private void HandleClick() { }
        private void UpdateUI() { }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // All methods should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "Initialize"));
    assert!(symbols.iter().any(|s| &*s.name == "HandleClick"));
    assert!(symbols.iter().any(|s| &*s.name == "UpdateUI"));
}

/// Test lambda expressions with complex types
#[test]
fn test_lambda_with_complex_types() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class GenericProcessor<T> where T : class
    {
        public IEnumerable<TResult> Transform<TResult>(
            Func<T, TResult> selector)
        {
            return items.Select(item => selector(item));
        }

        public void FilterAndMap()
        {
            var result = items
                .Where(x => x != null)
                .Select(x => new { Value = x, Count = GetCount(x) });
        }

        private int GetCount(T item) { return 0; }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Generic class with lambda-using methods should be parsed
    let processor = symbols.iter().find(|s| &*s.name == "GenericProcessor").unwrap();

    // Check it's generic
    let gen_info = parser.get_generic_info(processor.signature.as_ref().unwrap());
    assert!(gen_info.is_generic);
    assert_eq!(gen_info.param_count(), 1);

    // Methods using lambdas should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "Transform"));
    assert!(symbols.iter().any(|s| &*s.name == "FilterAndMap"));
}

/// Test lambda in switch expression (C# 8+)
#[test]
fn test_lambda_in_switch_expression() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class PatternMatcher
    {
        public string Match(object value)
        {
            return value switch
            {
                int n => $"Number: {n}",
                string s => $"String: {s}",
                _ => "Unknown"
            };
        }

        public void ProcessSwitch()
        {
            var processor = type switch
            {
                TypeA => (x => ProcessA(x)),
                TypeB => (x => ProcessB(x)),
                _ => (x => ProcessDefault(x))
            };
        }

        private void ProcessA(object x) { }
        private void ProcessB(object x) { }
        private void ProcessDefault(object x) { }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // All methods should be parsed
    assert!(symbols.iter().any(|s| &*s.name == "Match"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessSwitch"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessA"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessB"));
    assert!(symbols.iter().any(|s| &*s.name == "ProcessDefault"));
}

/// Test method calls within lambdas are tracked
#[test]
fn test_method_calls_within_lambdas() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class DataProcessor
    {
        public void Process()
        {
            items.ForEach(item => {
                Validate(item);
                Transform(item);
                Save(item);
            });
        }

        private void Validate(object item) { }
        private void Transform(object item) { }
        private void Save(object item) { }
    }
}
"#;

    let calls = parser.find_calls(code);

    // Calls from within lambdas should be tracked
    // The caller might be "Process" (the containing method)
    assert!(calls.iter().any(|(_, callee, _)| *callee == "Validate"));
    assert!(calls.iter().any(|(_, callee, _)| *callee == "Transform"));
    assert!(calls.iter().any(|(_, callee, _)| *callee == "Save"));
}
