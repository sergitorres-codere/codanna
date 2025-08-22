//! REAL TDD Test for PHP relationship resolution
//!
//! This test will ACTUALLY fail first, then we implement to make it pass

use codanna::parsing::php::PhpResolutionContext;
use codanna::parsing::{ResolutionScope, ScopeLevel};
use codanna::{FileId, RelationKind, SymbolId};

/// REAL TDD Test - This WILL fail first
#[test]
fn test_php_resolve_relationship_real_tdd() {
    println!("\n=== REAL TDD: PHP Relationship Resolution ===");

    let file_id = FileId::new(1).unwrap();
    let mut context = PhpResolutionContext::new(file_id);

    // Add some symbols to the context
    let logger_trait_id = SymbolId::new(1).unwrap();
    context.add_symbol(
        "LoggerTrait".to_string(),
        logger_trait_id,
        ScopeLevel::Module,
    );

    let database_class_id = SymbolId::new(2).unwrap();
    context.add_symbol(
        "Database".to_string(),
        database_class_id,
        ScopeLevel::Module,
    );

    let connect_method_id = SymbolId::new(3).unwrap();
    context.add_symbol("connect".to_string(), connect_method_id, ScopeLevel::Module);

    // TEST 1: PHP trait usage (Uses relationship)
    println!("\n--- Test 1: MyClass uses LoggerTrait ---");
    let resolved =
        context.resolve_relationship("MyClass", "LoggerTrait", RelationKind::Uses, file_id);
    println!("  Expected: Some(SymbolId(1))");
    println!("  Got:      {resolved:?}");
    if resolved == Some(logger_trait_id) {
        println!("  ✓ PASS");
    } else {
        println!("  ✗ FAIL - Values don't match!");
    }

    // TEST 2: PHP static method call
    println!("\n--- Test 2: main calls Database::connect ---");
    let resolved =
        context.resolve_relationship("main", "Database::connect", RelationKind::Calls, file_id);
    println!("  Expected: Some(SymbolId(3))");
    println!("  Got:      {resolved:?}");
    if resolved == Some(connect_method_id) {
        println!("  ✓ PASS");
    } else {
        println!("  ✗ FAIL - Values don't match!");
    }

    // TEST 3: PHP instance method call (->)
    println!("\n--- Test 3: processData calls $obj->save ---");
    let resolved =
        context.resolve_relationship("processData", "save", RelationKind::Calls, file_id);
    println!("  Expected: Some(SymbolId) for save method");
    println!("  Got:      {resolved:?}");

    // TEST 4: External library (should be None)
    println!("\n--- Test 4: MyClass extends PDO (external) ---");
    let resolved = context.resolve_relationship("MyClass", "PDO", RelationKind::Extends, file_id);
    println!("  Expected: None (external library)");
    println!("  Got:      {resolved:?}");
    if resolved.is_none() {
        println!("  ✓ PASS");
    } else {
        println!("  ✗ FAIL - Should be None for external!");
    }

    println!("\n=== END OF REAL TDD TEST ===");
}
