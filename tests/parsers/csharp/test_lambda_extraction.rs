use codanna::parsing::csharp::{CSharpParser, LambdaType};

#[test]
fn test_simple_expression_lambda() {
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

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 1);

    let lambda = &lambdas.lambdas[0];
    assert_eq!(lambda.parameters.len(), 1);
    assert_eq!(lambda.parameters[0], "x");
    assert_eq!(lambda.lambda_type, LambdaType::Lambda);
    assert!(!lambda.is_async);
    assert!(!lambda.is_statement_body);
    assert!(lambda.is_simple());
}

#[test]
fn test_multi_parameter_lambda() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Utils
    {
        public void ProcessPairs()
        {
            var result = pairs.Select((x, y) => x + y).ToList();
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 1);

    let lambda = &lambdas.lambdas[0];
    assert_eq!(lambda.parameters.len(), 2);
    assert_eq!(lambda.parameters[0], "x");
    assert_eq!(lambda.parameters[1], "y");
    assert!(!lambda.is_simple());
    assert!(lambda.has_multiple_parameters());
}

#[test]
fn test_statement_body_lambda() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Processor
    {
        public void Process()
        {
            var handler = new Action<int>(x => {
                Console.WriteLine(x);
                DoWork(x);
            });
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 1);

    let lambda = &lambdas.lambdas[0];
    assert_eq!(lambda.parameters.len(), 1);
    assert_eq!(lambda.parameters[0], "x");
    assert!(lambda.is_statement_body);
    assert!(!lambda.is_simple());
}

#[test]
fn test_async_lambda() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class AsyncProcessor
    {
        public void RegisterHandler()
        {
            var handler = new EventHandler(async (sender, args) => {
                await ProcessAsync(args);
            });
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 1);

    let lambda = &lambdas.lambdas[0];
    assert_eq!(lambda.parameters.len(), 2);
    assert_eq!(lambda.parameters[0], "sender");
    assert_eq!(lambda.parameters[1], "args");
    assert!(lambda.is_async);
    assert!(lambda.is_statement_body);
}

#[test]
fn test_async_expression_lambda() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Service
    {
        public void Setup()
        {
            Func<int, Task<int>> processor = async x => await ProcessAsync(x);
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 1);

    let lambda = &lambdas.lambdas[0];
    assert_eq!(lambda.parameters.len(), 1);
    assert_eq!(lambda.parameters[0], "x");
    assert!(lambda.is_async);
    assert!(!lambda.is_statement_body);
}

#[test]
fn test_anonymous_method() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class OldStyle
    {
        public void Setup()
        {
            var handler = new EventHandler(delegate(object sender, EventArgs e) {
                Console.WriteLine("Event fired");
            });
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 1);

    let lambda = &lambdas.lambdas[0];
    assert_eq!(lambda.parameters.len(), 2);
    assert_eq!(lambda.parameters[0], "sender");
    assert_eq!(lambda.parameters[1], "e");
    assert_eq!(lambda.lambda_type, LambdaType::AnonymousMethod);
    assert!(!lambda.is_async);
    assert!(lambda.is_statement_body);
}

#[test]
fn test_multiple_lambdas_in_same_method() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class DataProcessor
    {
        public void Process(List<int> numbers)
        {
            var doubled = numbers.Select(x => x * 2);
            var filtered = doubled.Where(x => x > 10);
            var sorted = filtered.OrderBy(x => x);
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 3);

    // All should be simple expression lambdas with parameter x
    for lambda in &lambdas.lambdas {
        assert_eq!(lambda.parameters.len(), 1);
        assert_eq!(lambda.parameters[0], "x");
        assert!(!lambda.is_async);
        assert!(!lambda.is_statement_body);
        assert!(lambda.is_simple());
    }
}

#[test]
fn test_nested_lambdas() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class NestedExample
    {
        public void Example()
        {
            var outer = list.Select(x => x.Items.Where(y => y.IsActive).ToList());
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 2);

    // Should find both outer lambda (x) and inner lambda (y)
    assert!(lambdas.lambdas.iter().any(|l| l.parameters.len() == 1 && l.parameters[0] == "x"));
    assert!(lambdas.lambdas.iter().any(|l| l.parameters.len() == 1 && l.parameters[0] == "y"));
}

#[test]
fn test_parameterless_lambda() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class Worker
    {
        public void Schedule()
        {
            Task.Run(() => DoWork());
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 1);

    let lambda = &lambdas.lambdas[0];
    assert_eq!(lambda.parameters.len(), 0);
    assert!(lambda.is_parameterless());
    assert!(!lambda.is_simple()); // Simple requires exactly 1 parameter
}

#[test]
fn test_lambda_collection_filtering() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class FilterExample
    {
        public void Example()
        {
            // Simple expression lambda
            var doubled = numbers.Select(x => x * 2);

            // Statement lambda
            var processed = numbers.Select(x => {
                var result = x * 2;
                return result;
            });

            // Async lambda
            var asyncProcessed = items.Select(async x => await ProcessAsync(x));

            // Multi-parameter lambda
            var combined = pairs.Select((a, b) => a + b);

            // Anonymous method
            var handler = new EventHandler(delegate(object sender, EventArgs e) {
                Console.WriteLine("Event");
            });
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 5);

    // Test filtering
    assert_eq!(lambdas.simple_lambdas().len(), 1); // Only the first one
    assert_eq!(lambdas.async_lambdas().len(), 1);
    assert_eq!(lambdas.statement_lambdas().len(), 2); // Statement lambda + anonymous method
    assert_eq!(lambdas.expression_lambdas().len(), 3); // All except statement lambda and anonymous method
    assert_eq!(lambdas.anonymous_methods().len(), 1);
    assert_eq!(lambdas.lambda_expressions().len(), 4); // All except anonymous method

    // Test parameter count filtering
    assert_eq!(lambdas.lambdas_with_param_count(1).len(), 3);
    assert_eq!(lambdas.lambdas_with_param_count(2).len(), 2);
    assert_eq!(lambdas.lambdas_with_param_count(0).len(), 0);
}

#[test]
fn test_lambda_in_linq_query() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class LinqExample
    {
        public void Query(List<User> users)
        {
            var result = users
                .Where(u => u.IsActive)
                .OrderBy(u => u.Name)
                .Select(u => new { u.Id, u.Name })
                .GroupBy(u => u.Name)
                .ToList();
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 4);

    // All should have parameter 'u'
    for lambda in &lambdas.lambdas {
        assert_eq!(lambda.parameters.len(), 1);
        assert_eq!(lambda.parameters[0], "u");
    }
}

#[test]
fn test_lambda_with_explicit_types() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class TypedLambda
    {
        public void Example()
        {
            Func<int, string, bool> comparer = (int x, string y) => x.ToString() == y;
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 1);

    let lambda = &lambdas.lambdas[0];
    assert_eq!(lambda.parameters.len(), 2);
    assert_eq!(lambda.parameters[0], "x");
    assert_eq!(lambda.parameters[1], "y");
}

#[test]
fn test_lambda_capturing_variables() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
namespace MyApp
{
    public class CaptureExample
    {
        public void Example()
        {
            int multiplier = 2;
            string prefix = "Value: ";

            var doubled = numbers.Select(x => x * multiplier);
            var formatted = numbers.Select(x => prefix + x.ToString());
        }
    }
}
"#;

    let lambdas = parser.find_lambdas(code);
    assert_eq!(lambdas.count(), 2);

    // Both lambdas should be detected (captures are tracked implicitly, not explicitly in this API)
    for lambda in &lambdas.lambdas {
        assert_eq!(lambda.parameters.len(), 1);
        assert_eq!(lambda.parameters[0], "x");
        assert!(lambda.is_simple());
    }
}
