//! Test Python relationship resolution with the new resolve_relationship API
//!
//! Following the same TDD approach as Rust, we test that Python-specific
//! relationship resolution works correctly for:
//! - Defines relationships (methods, properties, class methods)
//! - Calls relationships (module.function patterns)
//! - Inheritance relationships

use codanna::parsing::{ResolutionScope, ScopeLevel, python::PythonResolutionContext};
use codanna::{FileId, RelationKind, SymbolId};

/// Test that Defines relationships are resolved correctly for Python
///
/// Python-specific considerations:
/// - @property, @classmethod, @staticmethod decorators
/// - Methods are always on classes (no trait/inherent distinction)
#[test]
fn test_python_defines_relationship_resolution() {
    println!("\n=== Testing Python Defines Relationship Resolution ===");

    let file_id = FileId::new(1).unwrap();
    let mut context = PythonResolutionContext::new(file_id);

    // Add symbols to the context
    // MyClass
    let myclass_id = SymbolId::new(1).unwrap();
    context.add_symbol("MyClass".to_string(), myclass_id, ScopeLevel::Module);

    // MyClass.__init__ method
    let init_id = SymbolId::new(2).unwrap();
    context.add_symbol("__init__".to_string(), init_id, ScopeLevel::Module);

    // MyClass.process method
    let process_id = SymbolId::new(3).unwrap();
    context.add_symbol("process".to_string(), process_id, ScopeLevel::Module);

    // MyClass.value property (decorated with @property)
    let value_id = SymbolId::new(4).unwrap();
    context.add_symbol("value".to_string(), value_id, ScopeLevel::Module);

    // Test the resolve_relationship method

    // Test 1: MyClass defines __init__
    let resolved =
        context.resolve_relationship("MyClass", "__init__", RelationKind::Defines, file_id);
    println!("Test 1 - MyClass defines __init__:");
    println!("  Expected: Some({init_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(init_id), "MyClass.__init__ should resolve");

    // Test 2: MyClass defines process (regular method)
    let resolved =
        context.resolve_relationship("MyClass", "process", RelationKind::Defines, file_id);
    println!("\nTest 2 - MyClass defines process:");
    println!("  Expected: Some({process_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(process_id), "MyClass.process should resolve");

    // Test 3: MyClass defines value (property)
    let resolved = context.resolve_relationship("MyClass", "value", RelationKind::Defines, file_id);
    println!("\nTest 3 - MyClass defines value (property):");
    println!("  Expected: Some({value_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(value_id),
        "MyClass.value property should resolve"
    );

    println!("\n✓ Python Defines relationships resolved correctly");
}

/// Test that Calls relationships handle module patterns properly
#[test]
fn test_python_calls_relationship_resolution() {
    println!("\n=== Testing Python Calls Relationship Resolution ===");

    let file_id = FileId::new(1).unwrap();
    let mut context = PythonResolutionContext::new(file_id);

    // Add module and function symbols
    let os_path_join_id = SymbolId::new(10).unwrap();
    context.add_symbol("join".to_string(), os_path_join_id, ScopeLevel::Module);
    context.add_symbol(
        "os.path.join".to_string(),
        os_path_join_id,
        ScopeLevel::Module,
    );

    let json_loads_id = SymbolId::new(11).unwrap();
    context.add_symbol("loads".to_string(), json_loads_id, ScopeLevel::Module);
    context.add_symbol("json.loads".to_string(), json_loads_id, ScopeLevel::Module);

    let custom_func_id = SymbolId::new(12).unwrap();
    context.add_symbol(
        "custom_function".to_string(),
        custom_func_id,
        ScopeLevel::Module,
    );

    // Test 1: os.path.join - module qualified call
    let resolved =
        context.resolve_relationship("main", "os.path.join", RelationKind::Calls, file_id);
    println!("Test 1 - main calls os.path.join:");
    println!("  Expected: Some({os_path_join_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(os_path_join_id),
        "os.path.join should resolve"
    );

    // Test 2: json.loads - another module call
    let resolved =
        context.resolve_relationship("process_data", "json.loads", RelationKind::Calls, file_id);
    println!("\nTest 2 - process_data calls json.loads:");
    println!("  Expected: Some({json_loads_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(json_loads_id), "json.loads should resolve");

    // Test 3: Simple function call
    let resolved =
        context.resolve_relationship("main", "custom_function", RelationKind::Calls, file_id);
    println!("\nTest 3 - main calls custom_function:");
    println!("  Expected: Some({custom_func_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(custom_func_id),
        "custom_function should resolve"
    );

    // Test 4: External library call (should return None)
    let resolved =
        context.resolve_relationship("main", "numpy.array", RelationKind::Calls, file_id);
    println!("\nTest 4 - main calls numpy.array (external):");
    println!("  Expected: None");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, None, "numpy.array should not resolve (external)");

    println!("\n✓ Python Calls relationships resolved correctly");
}

/// Test inheritance relationship resolution
#[test]
fn test_python_inheritance_relationship_resolution() {
    println!("\n=== Testing Python Inheritance Relationship Resolution ===");

    let file_id = FileId::new(1).unwrap();
    let mut context = PythonResolutionContext::new(file_id);

    // Add class symbols
    let base_class_id = SymbolId::new(20).unwrap();
    context.add_symbol("BaseClass".to_string(), base_class_id, ScopeLevel::Module);

    let mixin_id = SymbolId::new(21).unwrap();
    context.add_symbol("LoggerMixin".to_string(), mixin_id, ScopeLevel::Module);

    let abc_id = SymbolId::new(22).unwrap();
    context.add_symbol("ABC".to_string(), abc_id, ScopeLevel::Module);

    // Test 1: Single inheritance
    let resolved =
        context.resolve_relationship("DerivedClass", "BaseClass", RelationKind::Extends, file_id);
    println!("Test 1 - DerivedClass extends BaseClass:");
    println!("  Expected: Some({base_class_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(base_class_id), "BaseClass should resolve");

    // Test 2: Multiple inheritance (mixin pattern)
    let resolved =
        context.resolve_relationship("MyClass", "LoggerMixin", RelationKind::Extends, file_id);
    println!("\nTest 2 - MyClass extends LoggerMixin:");
    println!("  Expected: Some({mixin_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(mixin_id), "LoggerMixin should resolve");

    // Test 3: ABC inheritance
    let resolved =
        context.resolve_relationship("ConcreteClass", "ABC", RelationKind::Extends, file_id);
    println!("\nTest 3 - ConcreteClass extends ABC:");
    println!("  Expected: Some({abc_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, Some(abc_id), "ABC should resolve");

    // Test 4: External base class (e.g., from stdlib)
    let resolved =
        context.resolve_relationship("MyException", "Exception", RelationKind::Extends, file_id);
    println!("\nTest 4 - MyException extends Exception (stdlib):");
    println!("  Expected: None");
    println!("  Got:      {resolved:?}");
    assert_eq!(resolved, None, "Exception should not resolve (stdlib)");

    println!("\n✓ Python inheritance relationships resolved correctly");
}

/// Test Python-specific decorators and their impact on resolution
#[test]
fn test_python_decorator_aware_resolution() {
    println!("\n=== Testing Python Decorator-Aware Resolution ===");

    let file_id = FileId::new(1).unwrap();
    let mut context = PythonResolutionContext::new(file_id);

    // Add decorated method symbols
    let property_getter_id = SymbolId::new(30).unwrap();
    context.add_symbol("value".to_string(), property_getter_id, ScopeLevel::Module);

    let classmethod_id = SymbolId::new(31).unwrap();
    context.add_symbol("from_dict".to_string(), classmethod_id, ScopeLevel::Module);

    let staticmethod_id = SymbolId::new(32).unwrap();
    context.add_symbol("validate".to_string(), staticmethod_id, ScopeLevel::Module);

    // Test 1: @property decorator
    let resolved = context.resolve_relationship("MyClass", "value", RelationKind::Defines, file_id);
    println!("Test 1 - MyClass defines value (@property):");
    println!("  Expected: Some({property_getter_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(property_getter_id),
        "@property should resolve"
    );

    // Test 2: @classmethod decorator
    let resolved =
        context.resolve_relationship("MyClass", "from_dict", RelationKind::Defines, file_id);
    println!("\nTest 2 - MyClass defines from_dict (@classmethod):");
    println!("  Expected: Some({classmethod_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(classmethod_id),
        "@classmethod should resolve"
    );

    // Test 3: @staticmethod decorator
    let resolved =
        context.resolve_relationship("MyClass", "validate", RelationKind::Defines, file_id);
    println!("\nTest 3 - MyClass defines validate (@staticmethod):");
    println!("  Expected: Some({staticmethod_id:?})");
    println!("  Got:      {resolved:?}");
    assert_eq!(
        resolved,
        Some(staticmethod_id),
        "@staticmethod should resolve"
    );

    println!("\n✓ Python decorator-aware resolution works correctly");
}
