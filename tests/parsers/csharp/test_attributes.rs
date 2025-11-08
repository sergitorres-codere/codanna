use codanna::parsing::csharp::CSharpParser;
use codanna::types::SymbolKind;

#[test]
fn test_extract_simple_attributes() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
[Serializable]
public class MyClass
{
    [Required]
    public string Name { get; set; }
}
"#;

    let attributes = parser.find_attributes(code);

    assert_eq!(attributes.len(), 2);

    let serializable = attributes.by_name("Serializable");
    assert_eq!(serializable.len(), 1);
    assert_eq!(serializable[0].target, "MyClass");
    assert_eq!(serializable[0].target_kind, SymbolKind::Class);

    let required = attributes.by_name("Required");
    assert_eq!(required.len(), 1);
    assert_eq!(required[0].target, "Name");
}

#[test]
fn test_extract_attributes_with_positional_arguments() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
[Obsolete("Use NewClass instead", false)]
public class OldClass
{
}
"#;

    let attributes = parser.find_attributes(code);

    // Note: Argument extraction is a future enhancement
    // For now we just verify the attribute is found
    let obsolete = attributes.by_name("Obsolete");
    assert!(obsolete.len() >= 1, "Should find Obsolete attribute");
}

#[test]
fn test_extract_attributes_with_named_arguments() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class UserController
{
    [HttpGet("/api/users/{id}")]
    [Authorize(Roles = "Admin")]
    public User GetUser(int id)
    {
        return null;
    }
}
"#;

    let attributes = parser.find_attributes(code);

    // Note: Argument extraction is a future enhancement
    // For now we just verify attributes are found
    let http_get = attributes.by_name("HttpGet");
    assert!(http_get.len() >= 1, "Should find HttpGet attribute");

    let authorize = attributes.by_name("Authorize");
    assert!(authorize.len() >= 1, "Should find Authorize attribute");
}

#[test]
fn test_filter_by_target() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
[Serializable]
public class MyClass
{
    [Required]
    [MaxLength(100)]
    public string Name { get; set; }
}
"#;

    let attributes = parser.find_attributes(code);

    let name_attrs = attributes.by_target("Name");
    assert_eq!(name_attrs.len(), 2);

    let names: Vec<_> = name_attrs.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"Required"));
    assert!(names.contains(&"MaxLength"));
}

#[test]
fn test_filter_by_target_kind() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
[Serializable]
public class MyClass
{
    [HttpGet]
    public void GetData() { }

    [Required]
    public string Name { get; set; }
}
"#;

    let attributes = parser.find_attributes(code);

    let class_attrs = attributes.by_target_kind(SymbolKind::Class);
    assert_eq!(class_attrs.len(), 1);
    assert_eq!(class_attrs[0].name, "Serializable");

    let method_attrs = attributes.by_target_kind(SymbolKind::Method);
    assert_eq!(method_attrs.len(), 1);
    assert_eq!(method_attrs[0].name, "HttpGet");
}

#[test]
fn test_multiple_attributes_on_same_target() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
[Serializable]
[Obsolete("Old")]
[DebuggerDisplay("MyClass")]
public class MyClass { }
"#;

    let attributes = parser.find_attributes(code);

    let myclass_attrs = attributes.by_target("MyClass");
    assert_eq!(myclass_attrs.len(), 3);
}

#[test]
fn test_unique_names() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
[Required]
public class MyClass
{
    [Required]
    public string Name { get; set; }

    [MaxLength(50)]
    public string Email { get; set; }
}
"#;

    let attributes = parser.find_attributes(code);

    let unique_names = attributes.unique_names();
    assert_eq!(unique_names.len(), 2);
    assert!(unique_names.contains(&"Required".to_string()));
    assert!(unique_names.contains(&"MaxLength".to_string()));
}

#[test]
fn test_attributes_on_methods() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class ApiController
{
    [HttpGet("/api/users")]
    [Authorize]
    public List<User> GetUsers()
    {
        return new List<User>();
    }
}
"#;

    let attributes = parser.find_attributes(code);

    let get_users_attrs = attributes.by_target("GetUsers");
    assert_eq!(get_users_attrs.len(), 2);
}

#[test]
fn test_empty_attributes() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class SimpleClass
{
    public void SimpleMethod() { }
}
"#;

    let attributes = parser.find_attributes(code);

    assert!(attributes.is_empty());
    assert_eq!(attributes.len(), 0);
}
