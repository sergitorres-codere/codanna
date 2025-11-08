use codanna::parsing::csharp::CSharpParser;
use codanna::parsing::LanguageParser;
use codanna::types::{FileId, SymbolCounter};

/// Test that symbols from multiple files maintain their namespaces correctly
#[test]
fn test_cross_file_namespace_tracking() {
    let mut parser = CSharpParser::new().unwrap();

    // File 1: Service definition
    let file1 = r#"
namespace MyApp.Services
{
    public class UserService
    {
        public void CreateUser(string name) { }
    }
}
"#;

    // File 2: Controller in different namespace
    let file2 = r#"
namespace MyApp.Controllers
{
    public class UserController
    {
        public void HandleCreate() { }
    }
}
"#;

    let file_id1 = FileId::new(1).unwrap();
    let file_id2 = FileId::new(2).unwrap();
    let mut counter = SymbolCounter::new();

    // Parse both files
    let symbols1 = parser.parse(file1, file_id1, &mut counter);
    let symbols2 = parser.parse(file2, file_id2, &mut counter);

    // Verify symbols from file 1
    let user_service = symbols1.iter().find(|s| &*s.name == "UserService").unwrap();
    assert_eq!(user_service.module_path.as_deref(), Some("MyApp.Services"));

    // Verify symbols from file 2
    let user_controller = symbols2.iter().find(|s| &*s.name == "UserController").unwrap();
    assert_eq!(user_controller.module_path.as_deref(), Some("MyApp.Controllers"));
}

/// Test that using directives are extracted across files
#[test]
fn test_cross_file_using_directives() {
    let mut parser = CSharpParser::new().unwrap();

    let code = r#"
using System;
using System.Collections.Generic;
using MyApp.Services;
using MyApp.Data;

namespace MyApp.Controllers
{
    public class AppController
    {
        private AuthService _auth;
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let imports = parser.find_imports(code, file_id);

    assert!(imports.iter().any(|i| i.path == "System"));
    assert!(imports.iter().any(|i| i.path == "System.Collections.Generic"));
    assert!(imports.iter().any(|i| i.path == "MyApp.Services"));
    assert!(imports.iter().any(|i| i.path == "MyApp.Data"));
}

/// Test that method calls are properly tracked across file boundaries
#[test]
fn test_cross_file_method_calls() {
    let mut parser = CSharpParser::new().unwrap();

    let code = r#"
namespace MyApp.Controllers
{
    public class UserController
    {
        private UserService _service;

        public void HandleCreate(string name)
        {
            _service.CreateUser(name);
            _service.ValidateUser(name);
        }
    }
}
"#;

    let calls = parser.find_calls(code);

    // Verify calls are found with correct caller context
    assert!(calls
        .iter()
        .any(|(caller, callee, _)| *caller == "HandleCreate" && *callee == "CreateUser"));
    assert!(calls
        .iter()
        .any(|(caller, callee, _)| *caller == "HandleCreate" && *callee == "ValidateUser"));
}

/// Test interface implementation tracking across files
#[test]
fn test_cross_file_interface_implementation() {
    let mut parser = CSharpParser::new().unwrap();

    let code = r#"
namespace MyApp.Data
{
    public class UserRepository : IRepository
    {
        public void Save(object entity) { }
    }

    public class ProductRepository : IRepository
    {
        public void Save(object entity) { }
    }
}
"#;

    let implementations = parser.find_implementations(code);

    // Both classes implement IRepository
    assert!(implementations
        .iter()
        .any(|(impl_class, interface, _)| *impl_class == "UserRepository"
            && *interface == "IRepository"));
    assert!(implementations
        .iter()
        .any(|(impl_class, interface, _)| *impl_class == "ProductRepository"
            && *interface == "IRepository"));
}

/// Test partial class handling (same class across multiple files)
#[test]
fn test_partial_class_symbols() {
    let mut parser = CSharpParser::new().unwrap();

    // File 1: First part of partial class
    let file1 = r#"
namespace MyApp.Models
{
    public partial class User
    {
        public int Id { get; set; }
        public string Name { get; set; }
    }
}
"#;

    // File 2: Second part of partial class
    let file2 = r#"
namespace MyApp.Models
{
    public partial class User
    {
        public string Email { get; set; }
        public void Validate() { }
    }
}
"#;

    let file_id1 = FileId::new(1).unwrap();
    let file_id2 = FileId::new(2).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols1 = parser.parse(file1, file_id1, &mut counter);
    let symbols2 = parser.parse(file2, file_id2, &mut counter);

    // Both files should have User class in same namespace
    let user1 = symbols1.iter().find(|s| &*s.name == "User").unwrap();
    let user2 = symbols2.iter().find(|s| &*s.name == "User").unwrap();

    assert_eq!(user1.module_path.as_deref(), Some("MyApp.Models"));
    assert_eq!(user2.module_path.as_deref(), Some("MyApp.Models"));

    // Verify members from each file
    assert!(symbols1.iter().any(|s| &*s.name == "Id"));
    assert!(symbols1.iter().any(|s| &*s.name == "Name"));
    assert!(symbols2.iter().any(|s| &*s.name == "Email"));
    assert!(symbols2.iter().any(|s| &*s.name == "Validate"));
}

/// Test nested namespace handling
#[test]
fn test_deep_namespace_hierarchy() {
    let mut parser = CSharpParser::new().unwrap();

    let code = r#"
namespace MyApp.Business.Services.Authentication
{
    public class LoginService
    {
        public void Login(string username) { }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let login_service = symbols.iter().find(|s| &*s.name == "LoginService").unwrap();
    assert_eq!(
        login_service.module_path.as_deref(),
        Some("MyApp.Business.Services.Authentication")
    );
}

/// Test attribute extraction across files
#[test]
fn test_cross_file_attributes() {
    let mut parser = CSharpParser::new().unwrap();

    let code = r#"
namespace MyApp.Services
{
    [Logging(Category = "Auth")]
    public class AuthService
    {
        [Logging(Category = "Login")]
        [Required]
        public void Login(string username) { }
    }
}
"#;

    let attributes = parser.find_attributes(code);

    // Verify Logging attributes
    let logging_attrs = attributes.by_name("Logging");
    assert_eq!(logging_attrs.len(), 2);

    // Verify Required attribute
    let required_attrs = attributes.by_name("Required");
    assert_eq!(required_attrs.len(), 1);
}
