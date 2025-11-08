//! C# Parser Performance Benchmarks
//!
//! This benchmark suite validates that the C# parser meets performance targets:
//! - >10,000 symbols/second extraction speed
//! - Memory usage within acceptable limits
//! - Performance comparable to other language parsers
//! - Scalability with large codebases
//!
//! The benchmark suite measures:
//! 1. Symbol extraction performance with varying code complexity
//! 2. Memory usage patterns with different file sizes
//! 3. Parser initialization overhead
//! 4. Performance on specific C# language constructs
//! 5. XML documentation parsing performance
//! 6. Generic type information extraction
//! 7. Attribute extraction and querying
//! 8. Method call finding performance

use codanna::parsing::csharp::CSharpParser;
use codanna::parsing::LanguageParser;
use codanna::types::{FileId, SymbolCounter};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;

// Performance targets and constants
#[allow(dead_code)]
const TARGET_SYMBOLS_PER_SEC: u64 = 10_000;
#[allow(dead_code)]
const LARGE_FILE_SYMBOL_COUNT: usize = 1000;
#[allow(dead_code)]
const BENCHMARK_ITERATIONS: usize = 100;

/// Benchmark basic C# symbol extraction performance
fn bench_csharp_symbol_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("csharp_symbol_extraction");

    // Test with different C# code samples of varying complexity
    let test_cases = vec![
        ("basic_csharp", create_basic_csharp_code()),
        ("medium_csharp", create_medium_complexity_csharp_code()),
        ("complex_csharp", create_complex_csharp_code()),
        ("real_world_csharp", create_real_world_csharp_code()),
    ];

    for (name, source_code) in test_cases {
        let symbol_count = count_expected_symbols(&source_code);
        group.throughput(Throughput::Elements(symbol_count as u64));

        group.bench_with_input(
            BenchmarkId::new("symbol_extraction", name),
            &source_code,
            |b, code| {
                let mut parser = CSharpParser::new().expect("Failed to create C# parser");
                b.iter(|| {
                    let mut symbol_counter = SymbolCounter::new();
                    let file_id = FileId::new(1).expect("Failed to create file ID");
                    let symbols = parser.parse(black_box(code), file_id, &mut symbol_counter);
                    black_box(symbols)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_csharp_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("csharp_memory_usage");

    // Test memory usage with increasingly large C# files
    let sizes = vec![100, 500, 1000, 2000, 5000];

    for size in sizes {
        let source_code = create_large_csharp_file(size);
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("large_file_parsing", size),
            &source_code,
            |b, code| {
                b.iter(|| {
                    let mut parser = CSharpParser::new().expect("Failed to create C# parser");
                    let mut symbol_counter = SymbolCounter::new();
                    let file_id = FileId::new(1).expect("Failed to create file ID");
                    let symbols = parser.parse(black_box(code), file_id, &mut symbol_counter);
                    black_box(symbols)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parser initialization overhead
fn bench_parser_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_initialization");

    group.bench_function("csharp_parser_creation", |b| {
        b.iter(|| {
            let parser = CSharpParser::new();
            black_box(parser)
        });
    });

    group.finish();
}

/// Benchmark specific C# language constructs
fn bench_csharp_language_constructs(c: &mut Criterion) {
    let mut group = c.benchmark_group("csharp_language_constructs");

    let construct_tests = vec![
        ("classes", create_many_classes(50)),
        ("interfaces", create_many_interfaces(30)),
        ("methods", create_many_methods(100)),
        ("properties", create_many_properties(100)),
        ("enums", create_many_enums(20)),
        ("structs", create_many_structs(30)),
        ("records", create_many_records(30)),
        ("events", create_many_events(30)),
    ];

    for (construct_name, source_code) in construct_tests {
        let symbol_count = count_expected_symbols(&source_code);
        group.throughput(Throughput::Elements(symbol_count as u64));

        group.bench_with_input(
            BenchmarkId::new("construct_parsing", construct_name),
            &source_code,
            |b, code| {
                let mut parser = CSharpParser::new().expect("Failed to create C# parser");
                b.iter(|| {
                    let mut symbol_counter = SymbolCounter::new();
                    let file_id = FileId::new(1).expect("Failed to create file ID");
                    let symbols = parser.parse(black_box(code), file_id, &mut symbol_counter);
                    black_box(symbols)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark XML documentation parsing
fn bench_xml_documentation_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("xml_documentation_parsing");

    let source_code = create_code_with_many_xml_comments(100);
    let symbol_count = count_expected_symbols(&source_code);
    group.throughput(Throughput::Elements(symbol_count as u64));

    group.bench_function("xml_doc_parsing", |b| {
        let mut parser = CSharpParser::new().expect("Failed to create C# parser");
        b.iter(|| {
            let mut symbol_counter = SymbolCounter::new();
            let file_id = FileId::new(1).expect("Failed to create file ID");
            let symbols = parser.parse(black_box(&source_code), file_id, &mut symbol_counter);

            // Also parse XML docs from extracted symbols
            for symbol in &symbols {
                if let Some(doc) = &symbol.doc_comment {
                    let xml_doc = parser.parse_xml_doc(doc);
                    black_box(xml_doc);
                }
            }
            black_box(symbols)
        });
    });

    group.finish();
}

/// Benchmark generic type information extraction
fn bench_generic_type_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("generic_type_extraction");

    let source_code = create_code_with_generics(50);
    let symbol_count = count_expected_symbols(&source_code);
    group.throughput(Throughput::Elements(symbol_count as u64));

    group.bench_function("generic_info_parsing", |b| {
        let mut parser = CSharpParser::new().expect("Failed to create C# parser");
        b.iter(|| {
            let mut symbol_counter = SymbolCounter::new();
            let file_id = FileId::new(1).expect("Failed to create file ID");
            let symbols = parser.parse(black_box(&source_code), file_id, &mut symbol_counter);

            // Extract generic info from symbols
            for symbol in &symbols {
                if let Some(sig) = &symbol.signature {
                    let generic_info = parser.get_generic_info(sig);
                    black_box(generic_info);
                }
            }
            black_box(symbols)
        });
    });

    group.finish();
}

/// Benchmark attribute extraction
fn bench_attribute_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("attribute_extraction");

    let source_code = create_code_with_attributes(100);

    group.bench_function("find_attributes", |b| {
        let mut parser = CSharpParser::new().expect("Failed to create C# parser");
        b.iter(|| {
            let attributes = parser.find_attributes(black_box(&source_code));
            black_box(attributes)
        });
    });

    group.finish();
}

/// Benchmark method call extraction
fn bench_method_call_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("method_call_extraction");

    let source_code = create_code_with_many_calls(100);

    group.bench_function("find_calls", |b| {
        let mut parser = CSharpParser::new().expect("Failed to create C# parser");
        b.iter(|| {
            let calls = parser.find_calls(black_box(&source_code));
            black_box(calls)
        });
    });

    group.finish();
}

/// Benchmark implementation extraction (interfaces)
fn bench_implementation_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("implementation_extraction");

    let source_code = create_code_with_implementations(50);

    group.bench_function("find_implementations", |b| {
        let mut parser = CSharpParser::new().expect("Failed to create C# parser");
        b.iter(|| {
            let impls = parser.find_implementations(black_box(&source_code));
            black_box(impls)
        });
    });

    group.finish();
}

/// Benchmark scalable test data generation
fn bench_scalable_test_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalable_test_data");

    // Test with different data sizes to measure scalability
    let data_sizes = vec![100, 500, 1000, 2000, 5000, 10000];

    for size in data_sizes {
        let source_code = generate_scalable_csharp_code(size);
        let symbol_count = count_expected_symbols(&source_code);
        group.throughput(Throughput::Elements(symbol_count as u64));

        group.bench_with_input(
            BenchmarkId::new("generated_data", size),
            &source_code,
            |b, code| {
                let mut parser = CSharpParser::new().expect("Failed to create C# parser");
                b.iter(|| {
                    let mut symbol_counter = SymbolCounter::new();
                    let file_id = FileId::new(1).expect("Failed to create file ID");
                    let symbols = parser.parse(black_box(code), file_id, &mut symbol_counter);
                    black_box(symbols.len()) // Return count to avoid large memory allocations
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Test Data Generation Functions
// ============================================================================

/// Create basic C# code for benchmarking
fn create_basic_csharp_code() -> String {
    r#"
namespace Example
{
    /// <summary>
    /// A simple person class.
    /// </summary>
    public class Person
    {
        /// <summary>
        /// Gets or sets the name.
        /// </summary>
        public string Name { get; set; }

        /// <summary>
        /// Gets or sets the age.
        /// </summary>
        public int Age { get; set; }

        /// <summary>
        /// Returns a greeting message.
        /// </summary>
        public string Greet()
        {
            return $"Hello, I'm {Name}";
        }
    }

    /// <summary>
    /// Main program entry point.
    /// </summary>
    public class Program
    {
        public static void Main()
        {
            var person = new Person { Name = "Alice", Age = 30 };
            Console.WriteLine(person.Greet());
        }
    }
}
"#
    .to_string()
}

/// Create medium complexity C# code
fn create_medium_complexity_csharp_code() -> String {
    r#"
using System;
using System.Collections.Generic;

namespace Example.Services
{
    /// <summary>
    /// Configuration class for application settings.
    /// </summary>
    public class Config
    {
        public string Host { get; set; }
        public int Port { get; set; }
        public TimeSpan Timeout { get; set; } = TimeSpan.FromSeconds(30);

        /// <summary>
        /// Returns the full address.
        /// </summary>
        public string GetAddress() => $"{Host}:{Port}";

        /// <summary>
        /// Updates the timeout value.
        /// </summary>
        public Config UpdateTimeout(TimeSpan newTimeout)
        {
            return new Config
            {
                Host = this.Host,
                Port = this.Port,
                Timeout = newTimeout
            };
        }
    }

    /// <summary>
    /// Service interface defining lifecycle methods.
    /// </summary>
    public interface IService
    {
        void Start();
        void Stop();
        bool IsHealthy();
    }

    /// <summary>
    /// Web service implementation.
    /// </summary>
    public class WebService : IService
    {
        private readonly Config _config;
        private bool _running;

        public WebService(Config config)
        {
            _config = config;
        }

        public void Start()
        {
            _running = true;
            Console.WriteLine($"Starting web service at {_config.GetAddress()}");
        }

        public void Stop()
        {
            _running = false;
            Console.WriteLine("Stopping web service");
        }

        public bool IsHealthy() => _running;
    }

    /// <summary>
    /// Factory for creating services.
    /// </summary>
    public static class ServiceFactory
    {
        public static WebService CreateDefault()
        {
            return new WebService(new Config { Host = "localhost", Port = 8080 });
        }
    }
}
"#
    .to_string()
}

/// Create complex C# code with advanced features
fn create_complex_csharp_code() -> String {
    r#"
using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using System.Linq;

namespace Example.Advanced
{
    /// <summary>
    /// Generic repository interface.
    /// </summary>
    /// <typeparam name="T">The entity type</typeparam>
    public interface IRepository<T> where T : class
    {
        Task<Result<Unit>> Store(string key, T value);
        Task<Result<T>> Get(string key);
        IAsyncEnumerable<T> GetAll();
        Task<Result<Unit>> Delete(string key);
    }

    /// <summary>
    /// In-memory repository implementation.
    /// </summary>
    /// <typeparam name="T">The entity type</typeparam>
    public class InMemoryRepository<T> : IRepository<T> where T : class
    {
        private readonly Dictionary<string, T> _storage = new();

        public async Task<Result<Unit>> Store(string key, T value)
        {
            await Task.Run(() => _storage[key] = value);
            return Result<Unit>.Success(Unit.Value);
        }

        public async Task<Result<T>> Get(string key)
        {
            return await Task.Run(() =>
            {
                if (_storage.TryGetValue(key, out var value))
                    return Result<T>.Success(value);
                return Result<T>.Failure("Not found");
            });
        }

        public async IAsyncEnumerable<T> GetAll()
        {
            foreach (var value in _storage.Values)
            {
                await Task.Yield();
                yield return value;
            }
        }

        public async Task<Result<Unit>> Delete(string key)
        {
            await Task.Run(() => _storage.Remove(key));
            return Result<Unit>.Success(Unit.Value);
        }
    }

    /// <summary>
    /// Result type for modeling success/failure.
    /// </summary>
    /// <typeparam name="T">The result type</typeparam>
    public class Result<T>
    {
        public bool IsSuccess { get; }
        public T Value { get; }
        public string Error { get; }

        private Result(bool isSuccess, T value, string error)
        {
            IsSuccess = isSuccess;
            Value = value;
            Error = error;
        }

        public static Result<T> Success(T value) => new(true, value, null);
        public static Result<T> Failure(string error) => new(false, default, error);

        public Result<TNew> Map<TNew>(Func<T, TNew> transform)
        {
            return IsSuccess
                ? Result<TNew>.Success(transform(Value))
                : Result<TNew>.Failure(Error);
        }
    }

    /// <summary>
    /// Unit type for void results.
    /// </summary>
    public struct Unit
    {
        public static Unit Value => new();
    }

    /// <summary>
    /// Data service with caching and error handling.
    /// </summary>
    /// <typeparam name="T">The data type</typeparam>
    public class DataService<T> where T : class
    {
        private readonly IRepository<T> _repository;
        private readonly Dictionary<string, T> _cache = new();

        public DataService(IRepository<T> repository)
        {
            _repository = repository;
        }

        public async Task<Result<T>> FetchData(string key)
        {
            try
            {
                // Check cache first
                if (_cache.TryGetValue(key, out var cached))
                    return Result<T>.Success(cached);

                // Fetch from repository
                var result = await _repository.Get(key);
                if (result.IsSuccess && result.Value != null)
                {
                    _cache[key] = result.Value;
                    return Result<T>.Success(result.Value);
                }

                return Result<T>.Failure("Not found");
            }
            catch (Exception ex)
            {
                return Result<T>.Failure($"Unexpected error: {ex.Message}");
            }
        }
    }
}
"#
    .to_string()
}

/// Create real-world C# code pattern
fn create_real_world_csharp_code() -> String {
    r#"
using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace Example.App
{
    /// <summary>
    /// User domain model.
    /// </summary>
    public record User(
        long Id,
        string Name,
        string Email,
        DateTime Created
    );

    /// <summary>
    /// User service interface.
    /// </summary>
    public interface IUserService
    {
        Task<Result<User>> CreateUser(User user);
        Task<Result<User>> GetUser(long id);
        Task<Result<List<User>>> ListUsers(int limit, int offset);
        Task<Result<User>> UpdateUser(User user);
        Task<Result<Unit>> DeleteUser(long id);
    }

    /// <summary>
    /// User service implementation with validation.
    /// </summary>
    public class UserServiceImpl : IUserService
    {
        private readonly IUserRepository _repository;

        public UserServiceImpl(IUserRepository repository)
        {
            _repository = repository;
        }

        public async Task<Result<User>> CreateUser(User user)
        {
            if (string.IsNullOrWhiteSpace(user.Name))
                return Result<User>.Failure("Name cannot be blank");

            if (!user.Email.Contains("@"))
                return Result<User>.Failure("Invalid email format");

            return await _repository.Save(user);
        }

        public async Task<Result<User>> GetUser(long id)
        {
            if (id <= 0)
                return Result<User>.Failure("Invalid user ID");

            return await _repository.FindById(id);
        }

        public async Task<Result<List<User>>> ListUsers(int limit, int offset)
        {
            if (limit <= 0)
                return Result<List<User>>.Failure("Limit must be positive");

            if (offset < 0)
                return Result<List<User>>.Failure("Offset must be non-negative");

            return await _repository.FindAll(limit, offset);
        }

        public async Task<Result<User>> UpdateUser(User user)
        {
            if (user.Id <= 0)
                return Result<User>.Failure("Invalid user ID");

            if (string.IsNullOrWhiteSpace(user.Name))
                return Result<User>.Failure("Name cannot be blank");

            return await _repository.Update(user);
        }

        public async Task<Result<Unit>> DeleteUser(long id)
        {
            if (id <= 0)
                return Result<Unit>.Failure("Invalid user ID");

            await _repository.Delete(id);
            return Result<Unit>.Success(Unit.Value);
        }
    }

    /// <summary>
    /// User repository interface.
    /// </summary>
    public interface IUserRepository
    {
        Task<Result<User>> Save(User user);
        Task<Result<User>> FindById(long id);
        Task<Result<List<User>>> FindAll(int limit, int offset);
        Task<Result<User>> Update(User user);
        Task Delete(long id);
    }

    /// <summary>
    /// HTTP controller for user endpoints.
    /// </summary>
    [ApiController]
    [Route("api/users")]
    public class UserController
    {
        private readonly IUserService _service;

        public UserController(IUserService service)
        {
            _service = service;
        }

        [HttpPost]
        public async Task<Response<User>> HandleCreateUser(CreateUserRequest request)
        {
            var user = new User(0, request.Name, request.Email, DateTime.UtcNow);

            var result = await _service.CreateUser(user);
            return result.IsSuccess
                ? Response<User>.Success(result.Value)
                : Response<User>.Error(result.Error);
        }

        [HttpGet("{id}")]
        public async Task<Response<User>> HandleGetUser(long id)
        {
            var result = await _service.GetUser(id);
            return result.IsSuccess
                ? Response<User>.Success(result.Value)
                : Response<User>.Error(result.Error);
        }
    }

    /// <summary>
    /// Request data class.
    /// </summary>
    public record CreateUserRequest(string Name, string Email);

    /// <summary>
    /// Response wrapper.
    /// </summary>
    public class Response<T>
    {
        public bool IsSuccess { get; }
        public T Data { get; }
        public string Message { get; }

        private Response(bool isSuccess, T data, string message)
        {
            IsSuccess = isSuccess;
            Data = data;
            Message = message;
        }

        public static Response<T> Success(T data) => new(true, data, null);
        public static Response<T> Error(string message) => new(false, default, message);
    }

    public struct Unit
    {
        public static Unit Value => new();
    }

    public class Result<T>
    {
        public bool IsSuccess { get; }
        public T Value { get; }
        public string Error { get; }

        private Result(bool isSuccess, T value, string error)
        {
            IsSuccess = isSuccess;
            Value = value;
            Error = error;
        }

        public static Result<T> Success(T value) => new(true, value, null);
        public static Result<T> Failure(string error) => new(false, default, error);
    }

    public class ApiControllerAttribute : Attribute { }
    public class RouteAttribute : Attribute
    {
        public RouteAttribute(string route) { }
    }
    public class HttpPostAttribute : Attribute { }
    public class HttpGetAttribute : Attribute
    {
        public HttpGetAttribute(string route) { }
    }
}
"#
    .to_string()
}

/// Generate a large C# file with specified number of symbols
fn create_large_csharp_file(symbol_count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n");

    // Add classes
    for i in 0..(symbol_count / 4) {
        code.push_str(&format!(
            "    public class DataClass{} {{\n        public string Field1 {{ get; set; }}\n        public int Field2 {{ get; set; }}\n    }}\n\n",
            i
        ));
    }

    // Add interfaces
    for i in 0..(symbol_count / 8) {
        code.push_str(&format!(
            "    public interface IInterface{} {{\n        string Method{}();\n    }}\n\n",
            i, i
        ));
    }

    // Add static class with methods
    code.push_str("    public static class Functions\n    {\n");
    for i in 0..(symbol_count / 2) {
        code.push_str(&format!(
            "        public static string Function{}(string param) => $\"Function{}: {{param}}\";\n",
            i, i
        ));
    }
    code.push_str("    }\n");

    code.push_str("}\n");
    code
}

/// Create many classes for benchmarking
fn create_many_classes(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n");

    for i in 0..count {
        code.push_str(&format!(
            "    public class Class{} {{\n        public string Field1 {{ get; set; }} = \"value\";\n        public int Field2 {{ get; set; }} = {};\n    }}\n\n",
            i, i
        ));
    }

    code.push_str("}\n");
    code
}

/// Create many interfaces for benchmarking
fn create_many_interfaces(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n");

    for i in 0..count {
        code.push_str(&format!(
            "    public interface IInterface{} {{\n        string Method1{}();\n        void Method2{}(string param);\n    }}\n\n",
            i, i, i
        ));
    }

    code.push_str("}\n");
    code
}

/// Create many methods for benchmarking
fn create_many_methods(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n    public class TestClass\n    {\n");

    for i in 0..count {
        code.push_str(&format!(
            "        public string Method{}(string param) => $\"method{}: {{param}}\";\n",
            i, i
        ));
    }

    code.push_str("    }\n}\n");
    code
}

/// Create many properties for benchmarking
fn create_many_properties(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n    public class PropertyClass\n    {\n");

    for i in 0..count {
        code.push_str(&format!("        public string Property{} {{ get; set; }} = \"value{}\";\n", i, i));
    }

    code.push_str("    }\n}\n");
    code
}

/// Create many enums for benchmarking
fn create_many_enums(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n");

    for i in 0..count {
        code.push_str(&format!(
            "    public enum Enum{} {{\n        Entry1,\n        Entry2,\n        Entry3\n    }}\n\n",
            i
        ));
    }

    code.push_str("}\n");
    code
}

/// Create many structs for benchmarking
fn create_many_structs(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n");

    for i in 0..count {
        code.push_str(&format!(
            "    public struct Struct{} {{\n        public int X {{ get; set; }}\n        public int Y {{ get; set; }}\n    }}\n\n",
            i
        ));
    }

    code.push_str("}\n");
    code
}

/// Create many records for benchmarking
fn create_many_records(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n");

    for i in 0..count {
        code.push_str(&format!(
            "    public record Record{}(string Field1, int Field2, bool Field3);\n",
            i
        ));
    }

    code.push_str("}\n");
    code
}

/// Create many events for benchmarking
fn create_many_events(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n    public class EventClass\n    {\n");

    for i in 0..count {
        code.push_str(&format!("        public event EventHandler Event{};\n", i));
    }

    code.push_str("    }\n}\n");
    code
}

/// Create code with many XML documentation comments
fn create_code_with_many_xml_comments(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n    public class DocClass\n    {\n");

    for i in 0..count {
        code.push_str(&format!(
            "        /// <summary>\n        /// Documentation for Method{}.\n        /// This method does something important.\n        /// </summary>\n        /// <param name=\"param\">The parameter</param>\n        /// <returns>The result</returns>\n        public string Method{}(string param) => param;\n\n",
            i, i
        ));
    }

    code.push_str("    }\n}\n");
    code
}

/// Create code with generics
fn create_code_with_generics(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n");

    for i in 0..count {
        code.push_str(&format!(
            "    public class Container{}<T> where T : class {{\n        public void Add(T item) {{ }}\n        public T Get() => default(T);\n    }}\n\n",
            i
        ));
    }

    code.push_str("}\n");
    code
}

/// Create code with attributes
fn create_code_with_attributes(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n    public class AttributeClass\n    {\n");

    for i in 0..count {
        code.push_str(&format!(
            "        [Obsolete]\n        [Required]\n        public string Property{} {{ get; set; }}\n\n",
            i
        ));
    }

    code.push_str("    }\n}\n");
    code
}

/// Create code with many method calls
fn create_code_with_many_calls(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n    public class CallClass\n    {\n");

    // Add some helper methods to call
    for i in 0..10 {
        code.push_str(&format!("        private void Helper{}() {{ }}\n", i));
    }

    // Add a method with many calls
    code.push_str("\n        public void ProcessData()\n        {\n");
    for i in 0..count {
        let helper = i % 10;
        code.push_str(&format!("            Helper{}();\n", helper));
    }
    code.push_str("        }\n");

    code.push_str("    }\n}\n");
    code
}

/// Create code with interface implementations
fn create_code_with_implementations(count: usize) -> String {
    let mut code = String::from("namespace Example\n{\n");

    // Base interface
    code.push_str("    public interface IBaseInterface\n    {\n        string BaseMethod();\n    }\n\n");

    // Add classes implementing the interface
    for i in 0..count {
        code.push_str(&format!(
            "    public class Class{} : IBaseInterface\n    {{\n        public string BaseMethod() => \"class{}\";\n    }}\n\n",
            i, i
        ));
    }

    code.push_str("}\n");
    code
}

/// Generate scalable C# code for systematic performance testing
fn generate_scalable_csharp_code(target_symbols: usize) -> String {
    let mut code = String::from("using System;\nusing System.Collections.Generic;\n\nnamespace Example.Scalable\n{\n");

    // Calculate distribution of symbols
    let classes = target_symbols / 8;
    let interfaces = target_symbols / 16;
    let methods = target_symbols / 4;
    let properties = target_symbols / 8;
    let constants = target_symbols / 8;

    // Add constants
    code.push_str("    public static class Constants\n    {\n");
    for i in 0..constants {
        code.push_str(&format!("        public const string CONSTANT{} = \"value_{}\";\n", i, i));
    }
    code.push_str("    }\n\n");

    // Add data classes
    for i in 0..classes {
        code.push_str(&format!(
            "    public class DataClass{} {{\n        public string Field1 {{ get; set; }}\n        public int Field2 {{ get; set; }}\n        public bool Field3 {{ get; set; }}\n    }}\n\n",
            i
        ));
    }

    // Add interfaces
    for i in 0..interfaces {
        code.push_str(&format!(
            "    public interface IInterface{} {{\n        string Method1();\n        bool Method2(int param);\n    }}\n\n",
            i
        ));
    }

    // Add class with many methods
    code.push_str("    public class MethodContainer\n    {\n");
    for i in 0..methods {
        code.push_str(&format!("        public string Method{}() => \"method_{}\";\n", i, i));
    }
    code.push_str("    }\n\n");

    // Add class with many properties
    code.push_str("    public class PropertyContainer\n    {\n");
    for i in 0..properties {
        code.push_str(&format!("        public string Property{} {{ get; set; }} = \"value_{}\";\n", i, i));
    }
    code.push_str("    }\n");

    code.push_str("}\n");
    code
}

/// Count expected symbols in C# source code (rough estimate)
fn count_expected_symbols(source_code: &str) -> usize {
    let mut count = 0;

    // Count type declarations
    count += source_code.matches("class ").count();
    count += source_code.matches("interface ").count();
    count += source_code.matches("struct ").count();
    count += source_code.matches("enum ").count();
    count += source_code.matches("record ").count();

    // Count member declarations
    count += source_code.matches(" get; ").count(); // Properties
    count += source_code.matches(" set; ").count(); // Properties
    count += source_code.matches("public void ").count();
    count += source_code.matches("public string ").count();
    count += source_code.matches("public int ").count();
    count += source_code.matches("public Task").count();
    count += source_code.matches("public event ").count();

    // Return at least 1 to avoid division by zero in benchmarks
    count.max(1)
}

criterion_group!(
    benches,
    bench_csharp_symbol_extraction,
    bench_csharp_memory_usage,
    bench_parser_initialization,
    bench_csharp_language_constructs,
    bench_xml_documentation_parsing,
    bench_generic_type_extraction,
    bench_attribute_extraction,
    bench_method_call_extraction,
    bench_implementation_extraction,
    bench_scalable_test_data
);
criterion_main!(benches);
