//! Test TypeScript relationship resolution with the new resolve_relationship API
//!
//! Following the same TDD approach as Rust and Python, we test that TypeScript-specific
//! relationship resolution works correctly for:
//! - Implements relationships (interfaces vs classes)
//! - Extends relationships (class inheritance, interface extension)
//! - Calls relationships (Class.method patterns)

use codanna::parsing::typescript::TypeScriptResolutionContext;
use codanna::parsing::{ResolutionScope, ScopeLevel};
use codanna::{FileId, RelationKind, SymbolId};

/// Test that Implements relationships are resolved correctly for TypeScript
///
/// TypeScript-specific considerations:
/// - Classes implement interfaces
/// - Classes can implement multiple interfaces
/// - Interfaces can extend other interfaces
#[test]
fn test_typescript_implements_relationship_resolution() {
    println!("\n=== Testing TypeScript Implements Relationship Resolution ===");

    let file_id = FileId::new(1).unwrap();
    let mut context = TypeScriptResolutionContext::new(file_id);

    // Add interface and class symbols
    let serializable_id = SymbolId::new(1).unwrap();
    context.add_symbol(
        "Serializable".to_string(),
        serializable_id,
        ScopeLevel::Module,
    );

    let comparable_id = SymbolId::new(2).unwrap();
    context.add_symbol("Comparable".to_string(), comparable_id, ScopeLevel::Module);

    let disposable_id = SymbolId::new(3).unwrap();
    context.add_symbol("IDisposable".to_string(), disposable_id, ScopeLevel::Module);

    // Test the resolve_relationship method

    // Test 1: MyClass implements Serializable
    let resolved =
        context.resolve_relationship("MyClass", "Serializable", RelationKind::Implements, file_id);
    println!("Test 1 - MyClass implements Serializable:");
    println!("  Expected: Some({serializable_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(serializable_id),
        "Serializable interface should resolve"
    );

    // Test 2: MyClass implements Comparable
    let resolved =
        context.resolve_relationship("MyClass", "Comparable", RelationKind::Implements, file_id);
    println!("\nTest 2 - MyClass implements Comparable:");
    println!("  Expected: Some({comparable_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(comparable_id),
        "Comparable interface should resolve"
    );

    // Test 3: Component implements IDisposable (I-prefixed interface)
    let resolved = context.resolve_relationship(
        "Component",
        "IDisposable",
        RelationKind::Implements,
        file_id,
    );
    println!("\nTest 3 - Component implements IDisposable:");
    println!("  Expected: Some({disposable_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(disposable_id),
        "IDisposable interface should resolve"
    );

    // Test 4: External interface (from node_modules)
    let resolved = context.resolve_relationship(
        "MyComponent",
        "React.Component",
        RelationKind::Extends,
        file_id,
    );
    println!("\nTest 4 - MyComponent extends React.Component (external):");
    println!("  Expected: None");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved, None,
        "React.Component should not resolve (external)"
    );

    println!("\n✓ TypeScript Implements relationships resolved correctly");
}

/// Test that Extends relationships handle inheritance properly
#[test]
fn test_typescript_extends_relationship_resolution() {
    println!("\n=== Testing TypeScript Extends Relationship Resolution ===");

    let file_id = FileId::new(1).unwrap();
    let mut context = TypeScriptResolutionContext::new(file_id);

    // Add base classes and interfaces
    let base_class_id = SymbolId::new(10).unwrap();
    context.add_symbol("BaseClass".to_string(), base_class_id, ScopeLevel::Module);

    let base_interface_id = SymbolId::new(11).unwrap();
    context.add_symbol(
        "BaseInterface".to_string(),
        base_interface_id,
        ScopeLevel::Module,
    );

    let generic_class_id = SymbolId::new(12).unwrap();
    context.add_symbol(
        "GenericClass".to_string(),
        generic_class_id,
        ScopeLevel::Module,
    );

    // Test 1: Class extends BaseClass
    let resolved =
        context.resolve_relationship("DerivedClass", "BaseClass", RelationKind::Extends, file_id);
    println!("Test 1 - DerivedClass extends BaseClass:");
    println!("  Expected: Some({base_class_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(base_class_id), "BaseClass should resolve");

    // Test 2: Interface extends BaseInterface
    let resolved = context.resolve_relationship(
        "ExtendedInterface",
        "BaseInterface",
        RelationKind::Extends,
        file_id,
    );
    println!("\nTest 2 - ExtendedInterface extends BaseInterface:");
    println!("  Expected: Some({base_interface_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(base_interface_id),
        "BaseInterface should resolve"
    );

    // Test 3: Generic class extension
    let resolved = context.resolve_relationship(
        "SpecializedClass",
        "GenericClass",
        RelationKind::Extends,
        file_id,
    );
    println!("\nTest 3 - SpecializedClass extends GenericClass<T>:");
    println!("  Expected: Some({generic_class_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(generic_class_id),
        "GenericClass should resolve"
    );

    println!("\n✓ TypeScript Extends relationships resolved correctly");
}

/// Test that Calls relationships handle TypeScript patterns
#[test]
fn test_typescript_calls_relationship_resolution() {
    println!("\n=== Testing TypeScript Calls Relationship Resolution ===");

    let file_id = FileId::new(1).unwrap();
    let mut context = TypeScriptResolutionContext::new(file_id);

    // Add function and method symbols
    let console_log_id = SymbolId::new(20).unwrap();
    context.add_symbol(
        "console.log".to_string(),
        console_log_id,
        ScopeLevel::Module,
    );

    let array_map_id = SymbolId::new(21).unwrap();
    context.add_symbol("map".to_string(), array_map_id, ScopeLevel::Module);

    let promise_then_id = SymbolId::new(22).unwrap();
    context.add_symbol("then".to_string(), promise_then_id, ScopeLevel::Module);

    let utils_helper_id = SymbolId::new(23).unwrap();
    context.add_symbol("helper".to_string(), utils_helper_id, ScopeLevel::Module);
    context.add_symbol(
        "Utils.helper".to_string(),
        utils_helper_id,
        ScopeLevel::Module,
    );

    // Test 1: console.log - global object method
    let resolved =
        context.resolve_relationship("main", "console.log", RelationKind::Calls, file_id);
    println!("Test 1 - main calls console.log:");
    println!("  Expected: Some({console_log_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(console_log_id), "console.log should resolve");

    // Test 2: Array.map - method call
    let resolved =
        context.resolve_relationship("processArray", "map", RelationKind::Calls, file_id);
    println!("\nTest 2 - processArray calls map:");
    println!("  Expected: Some({array_map_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(array_map_id), "map should resolve");

    // Test 3: Promise.then - chained method
    let resolved =
        context.resolve_relationship("asyncFunction", "then", RelationKind::Calls, file_id);
    println!("\nTest 3 - asyncFunction calls then:");
    println!("  Expected: Some({promise_then_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(promise_then_id), "then should resolve");

    // Test 4: Utils.helper - static method
    let resolved =
        context.resolve_relationship("main", "Utils.helper", RelationKind::Calls, file_id);
    println!("\nTest 4 - main calls Utils.helper:");
    println!("  Expected: Some({utils_helper_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(utils_helper_id),
        "Utils.helper should resolve"
    );

    // Test 5: External library call
    let resolved =
        context.resolve_relationship("component", "ReactDOM.render", RelationKind::Calls, file_id);
    println!("\nTest 5 - component calls ReactDOM.render (external):");
    println!("  Expected: None");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved, None,
        "ReactDOM.render should not resolve (external)"
    );

    println!("\n✓ TypeScript Calls relationships resolved correctly");
}

/// Test TypeScript-specific type relationships
#[test]
fn test_typescript_type_relationships() {
    println!("\n=== Testing TypeScript Type Relationships ===");

    let file_id = FileId::new(1).unwrap();
    let mut context = TypeScriptResolutionContext::new(file_id);

    // Add type-related symbols
    let user_type_id = SymbolId::new(30).unwrap();
    context.add_symbol("User".to_string(), user_type_id, ScopeLevel::Module);

    let role_type_id = SymbolId::new(31).unwrap();
    context.add_symbol("Role".to_string(), role_type_id, ScopeLevel::Module);

    let permission_enum_id = SymbolId::new(32).unwrap();
    context.add_symbol(
        "Permission".to_string(),
        permission_enum_id,
        ScopeLevel::Module,
    );

    // Test 1: Type alias uses another type
    let resolved = context.resolve_relationship("AdminUser", "User", RelationKind::Uses, file_id);
    println!("Test 1 - AdminUser type uses User:");
    println!("  Expected: Some({user_type_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(user_type_id), "User type should resolve");

    // Test 2: Interface property uses type
    let resolved = context.resolve_relationship("UserProfile", "Role", RelationKind::Uses, file_id);
    println!("\nTest 2 - UserProfile uses Role type:");
    println!("  Expected: Some({role_type_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(role_type_id), "Role type should resolve");

    // Test 3: Enum usage
    let resolved =
        context.resolve_relationship("checkAccess", "Permission", RelationKind::Uses, file_id);
    println!("\nTest 3 - checkAccess uses Permission enum:");
    println!("  Expected: Some({permission_enum_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(permission_enum_id),
        "Permission enum should resolve"
    );

    println!("\n✓ TypeScript type relationships resolved correctly");
}
