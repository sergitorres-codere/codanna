//! Real-world Python pattern tests
//!
//! Tests that verify Python resolution works with common real-world patterns
//! including Django models, Flask apps, data science code, and async patterns

use codanna::parsing::{
    InheritanceResolver, LanguageBehavior, PythonBehavior, ResolutionScope, ScopeLevel,
    python::{PythonInheritanceResolver, PythonResolutionContext},
};
use codanna::{FileId, SymbolId};

#[test]
fn test_python_django_model_pattern() {
    println!("\n=== Testing Python Django Model Pattern ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Simulate Django's Model hierarchy
    resolver.add_class("Model".to_string(), vec![]);
    resolver.add_class_methods(
        "Model".to_string(),
        vec![
            "save".to_string(),
            "delete".to_string(),
            "clean".to_string(),
            "full_clean".to_string(),
        ],
    );

    // User model extends Model
    resolver.add_class("User".to_string(), vec!["Model".to_string()]);
    resolver.add_class_methods(
        "User".to_string(),
        vec![
            "get_full_name".to_string(),
            "set_password".to_string(),
            "check_password".to_string(),
            "save".to_string(), // Override
        ],
    );

    // CustomUser extends User
    resolver.add_class("CustomUser".to_string(), vec!["User".to_string()]);
    resolver.add_class_methods(
        "CustomUser".to_string(),
        vec![
            "get_display_name".to_string(),
            "save".to_string(), // Override again
        ],
    );

    // Test method resolution
    let save_method = resolver.resolve_method("CustomUser", "save");
    println!("  CustomUser.save() resolved to: {save_method:?}");
    assert_eq!(save_method, Some("CustomUser".to_string()));

    let delete_method = resolver.resolve_method("CustomUser", "delete");
    println!("  CustomUser.delete() resolved to: {delete_method:?}");
    assert_eq!(delete_method, Some("Model".to_string()));

    let password_method = resolver.resolve_method("CustomUser", "set_password");
    println!("  CustomUser.set_password() resolved to: {password_method:?}");
    assert_eq!(password_method, Some("User".to_string()));

    // Test inheritance chain
    let chain = resolver.get_inheritance_chain("CustomUser");
    println!("  CustomUser MRO: {chain:?}");
    assert_eq!(chain.len(), 3);
    assert!(chain.contains(&"Model".to_string()));

    println!("✓ Django model pattern works correctly");
}

#[test]
fn test_python_mixin_pattern() {
    println!("\n=== Testing Python Mixin Pattern ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Common mixin pattern in Python
    resolver.add_class("LoggerMixin".to_string(), vec![]);
    resolver.add_class_methods(
        "LoggerMixin".to_string(),
        vec![
            "log".to_string(),
            "log_error".to_string(),
            "log_info".to_string(),
        ],
    );

    resolver.add_class("CacheMixin".to_string(), vec![]);
    resolver.add_class_methods(
        "CacheMixin".to_string(),
        vec![
            "get_cache".to_string(),
            "set_cache".to_string(),
            "invalidate_cache".to_string(),
        ],
    );

    resolver.add_class("BaseService".to_string(), vec![]);
    resolver.add_class_methods(
        "BaseService".to_string(),
        vec!["process".to_string(), "validate".to_string()],
    );

    // Service with multiple mixins (common pattern)
    resolver.add_class(
        "DataService".to_string(),
        vec![
            "LoggerMixin".to_string(),
            "CacheMixin".to_string(),
            "BaseService".to_string(),
        ],
    );
    resolver.add_class_methods(
        "DataService".to_string(),
        vec![
            "fetch_data".to_string(),
            "process".to_string(), // Override
        ],
    );

    // Test that all mixin methods are available
    let log_method = resolver.resolve_method("DataService", "log");
    println!("  DataService.log() from mixin: {log_method:?}");
    assert_eq!(log_method, Some("LoggerMixin".to_string()));

    let cache_method = resolver.resolve_method("DataService", "get_cache");
    println!("  DataService.get_cache() from mixin: {cache_method:?}");
    assert_eq!(cache_method, Some("CacheMixin".to_string()));

    let process_method = resolver.resolve_method("DataService", "process");
    println!("  DataService.process() override: {process_method:?}");
    assert_eq!(process_method, Some("DataService".to_string()));

    // Test all available methods
    let all_methods = resolver.get_all_methods("DataService");
    println!("  All DataService methods: {} total", all_methods.len());
    assert!(all_methods.contains(&"log".to_string()));
    assert!(all_methods.contains(&"get_cache".to_string()));
    assert!(all_methods.contains(&"fetch_data".to_string()));
    assert!(all_methods.contains(&"validate".to_string()));

    println!("✓ Mixin pattern works correctly");
}

#[test]
fn test_python_abstract_base_class() {
    println!("\n=== Testing Python Abstract Base Class (ABC) Pattern ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Simulate Python's ABC pattern
    resolver.add_class("ABC".to_string(), vec![]);

    resolver.add_class("Iterator".to_string(), vec!["ABC".to_string()]);
    resolver.add_class_methods(
        "Iterator".to_string(),
        vec!["__iter__".to_string(), "__next__".to_string()],
    );

    resolver.add_class("Reversible".to_string(), vec!["Iterator".to_string()]);
    resolver.add_class_methods("Reversible".to_string(), vec!["__reversed__".to_string()]);

    resolver.add_class("MyList".to_string(), vec!["Reversible".to_string()]);
    resolver.add_class_methods(
        "MyList".to_string(),
        vec![
            "__init__".to_string(),
            "__iter__".to_string(),     // Override
            "__next__".to_string(),     // Override
            "__reversed__".to_string(), // Override
            "append".to_string(),
            "pop".to_string(),
        ],
    );

    // Test ABC method resolution
    let iter_method = resolver.resolve_method("MyList", "__iter__");
    println!("  MyList.__iter__() resolved to: {iter_method:?}");
    assert_eq!(iter_method, Some("MyList".to_string()));

    let append_method = resolver.resolve_method("MyList", "append");
    println!("  MyList.append() resolved to: {append_method:?}");
    assert_eq!(append_method, Some("MyList".to_string()));

    // Test inheritance chain includes ABC
    let chain = resolver.get_inheritance_chain("MyList");
    println!("  MyList inheritance chain: {chain:?}");
    assert!(chain.contains(&"ABC".to_string()));
    assert!(chain.contains(&"Iterator".to_string()));
    assert!(chain.contains(&"Reversible".to_string()));

    println!("✓ ABC pattern works correctly");
}

#[test]
fn test_python_dataclass_inheritance() {
    println!("\n=== Testing Python Dataclass Inheritance ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Simulate dataclass inheritance pattern
    resolver.add_class("BaseConfig".to_string(), vec![]);
    resolver.add_class_methods(
        "BaseConfig".to_string(),
        vec![
            "__init__".to_string(),
            "__repr__".to_string(),
            "__eq__".to_string(),
            "validate".to_string(),
        ],
    );

    resolver.add_class("DatabaseConfig".to_string(), vec!["BaseConfig".to_string()]);
    resolver.add_class_methods(
        "DatabaseConfig".to_string(),
        vec![
            "__init__".to_string(), // Override for additional fields
            "get_connection_string".to_string(),
            "validate".to_string(), // Override with additional validation
        ],
    );

    resolver.add_class(
        "PostgresConfig".to_string(),
        vec!["DatabaseConfig".to_string()],
    );
    resolver.add_class_methods(
        "PostgresConfig".to_string(),
        vec![
            "get_connection_string".to_string(), // Override for postgres-specific
            "get_pool_size".to_string(),
        ],
    );

    // Test method resolution
    let validate = resolver.resolve_method("PostgresConfig", "validate");
    println!("  PostgresConfig.validate() resolved to: {validate:?}");
    assert_eq!(validate, Some("DatabaseConfig".to_string()));

    let conn_str = resolver.resolve_method("PostgresConfig", "get_connection_string");
    println!("  PostgresConfig.get_connection_string() resolved to: {conn_str:?}");
    assert_eq!(conn_str, Some("PostgresConfig".to_string()));

    let eq_method = resolver.resolve_method("PostgresConfig", "__eq__");
    println!("  PostgresConfig.__eq__() resolved to: {eq_method:?}");
    assert_eq!(eq_method, Some("BaseConfig".to_string()));

    println!("✓ Dataclass inheritance works correctly");
}

#[test]
fn test_python_complex_import_patterns() {
    println!("\n=== Testing Python Complex Import Patterns ===");

    let behavior = PythonBehavior::new();

    // Test 1: Package relative imports
    let result =
        behavior.import_matches_symbol(".models.user", "myapp.models.user", Some("myapp.views"));
    println!("  .models.user from myapp.views -> myapp.models.user: {result}");
    assert!(result);

    // Test 2: Parent package imports
    let result = behavior.import_matches_symbol(
        "...utils.helpers",
        "project.utils.helpers",
        Some("project.app.submodule.views"),
    );
    println!("  ...utils.helpers from deep module -> project.utils.helpers: {result}");
    assert!(result);

    // Test 3: Aliased imports (the alias is handled elsewhere, but path should match)
    let result = behavior.import_matches_symbol(
        "django.contrib.auth.models",
        "django.contrib.auth.models",
        Some("myapp.models"),
    );
    println!("  Full django import path matches: {result}");
    assert!(result);

    // Test 4: Partial path matching
    let result = behavior.import_matches_symbol(
        "auth.models",
        "django.contrib.auth.models",
        Some("myapp.views"),
    );
    println!("  Partial path auth.models -> django.contrib.auth.models: {result}");
    assert!(result);

    // Test 5: Single module import that could be anywhere
    let result = behavior.import_matches_symbol(
        "settings",
        "myproject.settings",
        Some("myproject.apps.core"),
    );
    println!("  Single module 'settings' -> myproject.settings: {result}");
    assert!(result);

    println!("✓ Complex import patterns work correctly");
}

#[test]
fn test_python_async_context_manager_pattern() {
    println!("\n=== Testing Python Async Context Manager Pattern ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Simulate async context manager pattern
    resolver.add_class("AsyncContextManager".to_string(), vec![]);
    resolver.add_class_methods(
        "AsyncContextManager".to_string(),
        vec!["__aenter__".to_string(), "__aexit__".to_string()],
    );

    resolver.add_class(
        "DatabaseConnection".to_string(),
        vec!["AsyncContextManager".to_string()],
    );
    resolver.add_class_methods(
        "DatabaseConnection".to_string(),
        vec![
            "__init__".to_string(),
            "__aenter__".to_string(), // Override
            "__aexit__".to_string(),  // Override
            "execute".to_string(),
            "fetch".to_string(),
            "commit".to_string(),
        ],
    );

    resolver.add_class(
        "PostgresConnection".to_string(),
        vec!["DatabaseConnection".to_string()],
    );
    resolver.add_class_methods(
        "PostgresConnection".to_string(),
        vec![
            "__init__".to_string(), // Override
            "execute".to_string(),  // Override for postgres-specific
            "listen".to_string(),   // Postgres LISTEN/NOTIFY
        ],
    );

    // Test async methods
    let aenter = resolver.resolve_method("PostgresConnection", "__aenter__");
    println!("  PostgresConnection.__aenter__() resolved to: {aenter:?}");
    assert_eq!(aenter, Some("DatabaseConnection".to_string()));

    let execute = resolver.resolve_method("PostgresConnection", "execute");
    println!("  PostgresConnection.execute() resolved to: {execute:?}");
    assert_eq!(execute, Some("PostgresConnection".to_string()));

    let listen = resolver.resolve_method("PostgresConnection", "listen");
    println!("  PostgresConnection.listen() resolved to: {listen:?}");
    assert_eq!(listen, Some("PostgresConnection".to_string()));

    println!("✓ Async context manager pattern works correctly");
}

#[test]
fn test_python_exception_hierarchy() {
    println!("\n=== Testing Python Exception Hierarchy ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Python's exception hierarchy
    resolver.add_class("BaseException".to_string(), vec![]);
    resolver.add_class_methods(
        "BaseException".to_string(),
        vec![
            "__init__".to_string(),
            "__str__".to_string(),
            "with_traceback".to_string(),
        ],
    );

    resolver.add_class("Exception".to_string(), vec!["BaseException".to_string()]);

    resolver.add_class("ValueError".to_string(), vec!["Exception".to_string()]);

    resolver.add_class(
        "ValidationError".to_string(),
        vec!["ValueError".to_string()],
    );
    resolver.add_class_methods(
        "ValidationError".to_string(),
        vec![
            "__init__".to_string(), // Override to add field info
            "get_field_errors".to_string(),
        ],
    );

    resolver.add_class(
        "EmailValidationError".to_string(),
        vec!["ValidationError".to_string()],
    );
    resolver.add_class_methods(
        "EmailValidationError".to_string(),
        vec![
            "__init__".to_string(), // Override
            "get_suggestion".to_string(),
        ],
    );

    // Test exception method resolution
    let str_method = resolver.resolve_method("EmailValidationError", "__str__");
    println!("  EmailValidationError.__str__() resolved to: {str_method:?}");
    assert_eq!(str_method, Some("BaseException".to_string()));

    let field_errors = resolver.resolve_method("EmailValidationError", "get_field_errors");
    println!("  EmailValidationError.get_field_errors() resolved to: {field_errors:?}");
    assert_eq!(field_errors, Some("ValidationError".to_string()));

    // Test full hierarchy
    let chain = resolver.get_inheritance_chain("EmailValidationError");
    println!("  EmailValidationError MRO: {chain:?}");
    assert_eq!(chain[0], "EmailValidationError");
    assert!(chain.contains(&"BaseException".to_string()));
    assert!(chain.contains(&"Exception".to_string()));
    assert!(chain.contains(&"ValueError".to_string()));

    // Test subtype relationships
    assert!(resolver.is_subtype("EmailValidationError", "Exception"));
    assert!(resolver.is_subtype("EmailValidationError", "ValueError"));
    assert!(!resolver.is_subtype("ValueError", "EmailValidationError"));

    println!("✓ Exception hierarchy works correctly");
}

#[test]
fn test_python_metaclass_pattern() {
    println!("\n=== Testing Python Metaclass Pattern ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Simulate metaclass pattern (singleton, etc.)
    resolver.add_class("type".to_string(), vec![]);
    resolver.add_class_methods(
        "type".to_string(),
        vec![
            "__new__".to_string(),
            "__init__".to_string(),
            "__call__".to_string(),
        ],
    );

    resolver.add_class("SingletonMeta".to_string(), vec!["type".to_string()]);
    resolver.add_class_methods(
        "SingletonMeta".to_string(),
        vec![
            "__call__".to_string(), // Override to implement singleton
        ],
    );

    resolver.add_class("DatabaseManager".to_string(), vec![]);
    resolver.add_class_methods(
        "DatabaseManager".to_string(),
        vec![
            "__init__".to_string(),
            "get_connection".to_string(),
            "close_all".to_string(),
        ],
    );
    // Note: In real Python, metaclass is specified differently,
    // but for resolution testing we treat it as inheritance

    // Test metaclass method resolution
    let call_method = resolver.resolve_method("SingletonMeta", "__call__");
    println!("  SingletonMeta.__call__() resolved to: {call_method:?}");
    assert_eq!(call_method, Some("SingletonMeta".to_string()));

    let new_method = resolver.resolve_method("SingletonMeta", "__new__");
    println!("  SingletonMeta.__new__() resolved to: {new_method:?}");
    assert_eq!(new_method, Some("type".to_string()));

    println!("✓ Metaclass pattern works correctly");
}

#[test]
fn test_python_protocol_pattern() {
    println!("\n=== Testing Python Protocol (PEP 544) Pattern ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Simulate Protocol pattern (structural subtyping)
    resolver.add_class("Protocol".to_string(), vec![]);

    resolver.add_class("Drawable".to_string(), vec!["Protocol".to_string()]);
    resolver.add_class_methods(
        "Drawable".to_string(),
        vec!["draw".to_string(), "get_bounds".to_string()],
    );

    resolver.add_class("Clickable".to_string(), vec!["Protocol".to_string()]);
    resolver.add_class_methods(
        "Clickable".to_string(),
        vec!["on_click".to_string(), "is_clicked".to_string()],
    );

    // Widget implements both protocols
    resolver.add_class(
        "Widget".to_string(),
        vec!["Drawable".to_string(), "Clickable".to_string()],
    );
    resolver.add_class_methods(
        "Widget".to_string(),
        vec![
            "__init__".to_string(),
            "draw".to_string(),       // Implement Drawable
            "get_bounds".to_string(), // Implement Drawable
            "on_click".to_string(),   // Implement Clickable
            "is_clicked".to_string(), // Implement Clickable
            "update".to_string(),     // Widget-specific
        ],
    );

    resolver.add_class("Button".to_string(), vec!["Widget".to_string()]);
    resolver.add_class_methods(
        "Button".to_string(),
        vec![
            "draw".to_string(),     // Override
            "on_click".to_string(), // Override
            "set_label".to_string(),
        ],
    );

    // Test protocol method resolution
    let draw_method = resolver.resolve_method("Button", "draw");
    println!("  Button.draw() resolved to: {draw_method:?}");
    assert_eq!(draw_method, Some("Button".to_string()));

    let bounds_method = resolver.resolve_method("Button", "get_bounds");
    println!("  Button.get_bounds() resolved to: {bounds_method:?}");
    assert_eq!(bounds_method, Some("Widget".to_string()));

    // Test that Button has all protocol methods
    let all_methods = resolver.get_all_methods("Button");
    assert!(all_methods.contains(&"draw".to_string()));
    assert!(all_methods.contains(&"on_click".to_string()));
    assert!(all_methods.contains(&"is_clicked".to_string()));
    assert!(all_methods.contains(&"get_bounds".to_string()));

    println!("✓ Protocol pattern works correctly");
}

#[test]
fn test_python_scope_resolution_real_world() {
    println!("\n=== Testing Python Real-World Scope Resolution ===");

    let file_id = FileId::new(1).unwrap();
    let mut ctx = PythonResolutionContext::new(file_id);

    // Simulate a real module with various scopes

    // Module-level symbols (globals)
    ctx.add_symbol(
        "CONFIG".to_string(),
        SymbolId::new(1).unwrap(),
        ScopeLevel::Global,
    );
    ctx.add_symbol(
        "logger".to_string(),
        SymbolId::new(2).unwrap(),
        ScopeLevel::Global,
    );

    // Imported symbols
    ctx.add_symbol(
        "datetime".to_string(),
        SymbolId::new(3).unwrap(),
        ScopeLevel::Package,
    );
    ctx.add_symbol(
        "json".to_string(),
        SymbolId::new(4).unwrap(),
        ScopeLevel::Package,
    );
    ctx.add_symbol(
        "User".to_string(),
        SymbolId::new(5).unwrap(),
        ScopeLevel::Package,
    );

    // Function local variables
    ctx.add_symbol(
        "request".to_string(),
        SymbolId::new(6).unwrap(),
        ScopeLevel::Local,
    );
    ctx.add_symbol(
        "response".to_string(),
        SymbolId::new(7).unwrap(),
        ScopeLevel::Local,
    );

    // Test resolution follows LEGB order

    // Local variable shadows imported
    ctx.add_symbol(
        "json".to_string(),
        SymbolId::new(8).unwrap(),
        ScopeLevel::Local,
    );
    let json_id = ctx.resolve("json");
    println!("  'json' resolves to local (id=8), not imported (id=4): {json_id:?}");
    assert_eq!(json_id, Some(SymbolId::new(8).unwrap()));

    // Global is found when no local exists
    let logger_id = ctx.resolve("logger");
    println!("  'logger' resolves to global: {logger_id:?}");
    assert_eq!(logger_id, Some(SymbolId::new(2).unwrap()));

    // Imported symbol is found
    let user_id = ctx.resolve("User");
    println!("  'User' resolves to imported: {user_id:?}");
    assert_eq!(user_id, Some(SymbolId::new(5).unwrap()));

    // Test clearing local scope (exiting function)
    ctx.clear_local_scope();
    let json_after = ctx.resolve("json");
    println!("  After clearing locals, 'json' resolves to imported: {json_after:?}");
    assert_eq!(json_after, Some(SymbolId::new(4).unwrap()));

    println!("✓ Real-world scope resolution works correctly");
}
