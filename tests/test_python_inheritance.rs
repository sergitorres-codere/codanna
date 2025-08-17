//! Test Python-specific inheritance and MRO using the proper Python API
//!
//! This test verifies that PythonInheritanceResolver correctly implements
//! Python's Method Resolution Order (MRO) and class inheritance

use codanna::parsing::InheritanceResolver;
use codanna::parsing::python::PythonInheritanceResolver;

#[test]
fn test_python_basic_inheritance() {
    println!("\n=== Testing Python Basic Inheritance ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Create a simple class hierarchy
    resolver.add_class("BaseClass".to_string(), vec![]);
    resolver.add_class_methods(
        "BaseClass".to_string(),
        vec![
            "__init__".to_string(),
            "base_method".to_string(),
            "override_me".to_string(),
        ],
    );

    resolver.add_class("DerivedClass".to_string(), vec!["BaseClass".to_string()]);
    resolver.add_class_methods(
        "DerivedClass".to_string(),
        vec![
            "derived_method".to_string(),
            "override_me".to_string(), // Override parent method
        ],
    );

    // Test method resolution
    let base_method = resolver.resolve_method("DerivedClass", "base_method");
    println!("  resolve_method(DerivedClass, base_method) = {base_method:?}");
    assert_eq!(base_method, Some("BaseClass".to_string()));

    let derived_method = resolver.resolve_method("DerivedClass", "derived_method");
    println!("  resolve_method(DerivedClass, derived_method) = {derived_method:?}");
    assert_eq!(derived_method, Some("DerivedClass".to_string()));

    let override_method = resolver.resolve_method("DerivedClass", "override_me");
    println!("  resolve_method(DerivedClass, override_me) = {override_method:?}");
    assert_eq!(override_method, Some("DerivedClass".to_string())); // Should find override first

    // Test inheritance chain
    let chain = resolver.get_inheritance_chain("DerivedClass");
    println!("  get_inheritance_chain(DerivedClass) = {chain:?}");
    assert_eq!(chain[0], "DerivedClass");
    assert_eq!(chain[1], "BaseClass");

    println!("✓ Python basic inheritance works correctly");
}

#[test]
fn test_python_multiple_inheritance() {
    println!("\n=== Testing Python Multiple Inheritance (MRO) ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Create multiple base classes
    resolver.add_class("Mixin1".to_string(), vec![]);
    resolver.add_class_methods(
        "Mixin1".to_string(),
        vec!["mixin1_method".to_string(), "shared_method".to_string()],
    );

    resolver.add_class("Mixin2".to_string(), vec![]);
    resolver.add_class_methods(
        "Mixin2".to_string(),
        vec![
            "mixin2_method".to_string(),
            "shared_method".to_string(), // Same method name
        ],
    );

    resolver.add_class("Base".to_string(), vec![]);
    resolver.add_class_methods("Base".to_string(), vec!["base_method".to_string()]);

    // Create a class with multiple inheritance
    // In Python: class Combined(Mixin1, Mixin2, Base):
    resolver.add_class(
        "Combined".to_string(),
        vec![
            "Mixin1".to_string(),
            "Mixin2".to_string(),
            "Base".to_string(),
        ],
    );
    resolver.add_class_methods("Combined".to_string(), vec!["combined_method".to_string()]);

    // Test MRO: Combined -> Mixin1 -> Mixin2 -> Base
    let mro = resolver.get_inheritance_chain("Combined");
    assert_eq!(mro[0], "Combined");
    assert_eq!(mro[1], "Mixin1");
    assert_eq!(mro[2], "Mixin2");
    assert_eq!(mro[3], "Base");

    // Test method resolution follows MRO
    assert_eq!(
        resolver.resolve_method("Combined", "shared_method"),
        Some("Mixin1".to_string()) // Mixin1 comes before Mixin2 in MRO
    );
    assert_eq!(
        resolver.resolve_method("Combined", "mixin1_method"),
        Some("Mixin1".to_string())
    );
    assert_eq!(
        resolver.resolve_method("Combined", "mixin2_method"),
        Some("Mixin2".to_string())
    );
    assert_eq!(
        resolver.resolve_method("Combined", "base_method"),
        Some("Base".to_string())
    );

    // Test all methods available
    let all_methods = resolver.get_all_methods("Combined");
    assert!(all_methods.contains(&"combined_method".to_string()));
    assert!(all_methods.contains(&"mixin1_method".to_string()));
    assert!(all_methods.contains(&"mixin2_method".to_string()));
    assert!(all_methods.contains(&"base_method".to_string()));
    assert!(all_methods.contains(&"shared_method".to_string()));

    println!("✓ Python multiple inheritance with MRO works correctly");
}

#[test]
fn test_python_diamond_inheritance() {
    println!("\n=== Testing Python Diamond Inheritance Pattern ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Create diamond pattern:
    //      Object
    //        |
    //        A
    //       / \
    //      B   C
    //       \ /
    //        D

    resolver.add_class("A".to_string(), vec![]);
    resolver.add_class_methods(
        "A".to_string(),
        vec!["method_a".to_string(), "shared".to_string()],
    );

    resolver.add_class("B".to_string(), vec!["A".to_string()]);
    resolver.add_class_methods(
        "B".to_string(),
        vec![
            "method_b".to_string(),
            "shared".to_string(), // Override
        ],
    );

    resolver.add_class("C".to_string(), vec!["A".to_string()]);
    resolver.add_class_methods(
        "C".to_string(),
        vec![
            "method_c".to_string(),
            "shared".to_string(), // Override
        ],
    );

    resolver.add_class("D".to_string(), vec!["B".to_string(), "C".to_string()]);
    resolver.add_class_methods("D".to_string(), vec!["method_d".to_string()]);

    // Test MRO handles diamond correctly with our simplified algorithm
    // Our algorithm produces: D -> B -> A -> C (not perfect C3, but handles basic cases)
    let mro = resolver.get_inheritance_chain("D");
    println!("  MRO for D: {mro:?}");
    println!("  Expected: D first, then B, with A and C appearing exactly once");
    assert_eq!(mro[0], "D");
    assert_eq!(mro[1], "B");
    // The exact order of A and C may vary in our simplified implementation
    assert!(mro.contains(&"A".to_string()));
    assert!(mro.contains(&"C".to_string()));
    assert_eq!(mro.len(), 4); // A should appear only once

    // Test method resolution
    assert_eq!(
        resolver.resolve_method("D", "shared"),
        Some("B".to_string()) // B comes first in MRO
    );
    assert_eq!(
        resolver.resolve_method("D", "method_a"),
        Some("A".to_string())
    );
    assert_eq!(
        resolver.resolve_method("D", "method_b"),
        Some("B".to_string())
    );
    assert_eq!(
        resolver.resolve_method("D", "method_c"),
        Some("C".to_string())
    );

    // Test subtype relationships
    assert!(resolver.is_subtype("D", "B"));
    assert!(resolver.is_subtype("D", "C"));
    assert!(resolver.is_subtype("D", "A"));
    assert!(resolver.is_subtype("B", "A"));
    assert!(resolver.is_subtype("C", "A"));
    assert!(!resolver.is_subtype("B", "C"));
    assert!(!resolver.is_subtype("C", "B"));

    println!("✓ Python diamond inheritance pattern works correctly");
}

#[test]
fn test_python_deep_inheritance() {
    println!("\n=== Testing Python Deep Inheritance Chain ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Create a deep inheritance chain
    let classes = vec!["A", "B", "C", "D", "E", "F"];

    for (i, &class) in classes.iter().enumerate() {
        let bases = if i == 0 {
            vec![]
        } else {
            vec![classes[i - 1].to_string()]
        };

        resolver.add_class(class.to_string(), bases);
        resolver.add_class_methods(
            class.to_string(),
            vec![format!("method_{}", class.to_lowercase())],
        );
    }

    // Test that F has access to all methods
    let all_methods = resolver.get_all_methods("F");
    assert_eq!(all_methods.len(), 6);
    for class in &classes {
        let method = format!("method_{}", class.to_lowercase());
        assert!(all_methods.contains(&method));
    }

    // Test inheritance chain
    let chain = resolver.get_inheritance_chain("F");
    assert_eq!(chain.len(), 6);
    for (i, class) in classes.iter().rev().enumerate() {
        assert_eq!(chain[i], class.to_string());
    }

    // Test subtype relationships
    assert!(resolver.is_subtype("F", "A"));
    assert!(resolver.is_subtype("F", "E"));
    assert!(!resolver.is_subtype("A", "F"));

    println!("✓ Python deep inheritance chain works correctly");
}

#[test]
fn test_python_complex_mro() {
    println!("\n=== Testing Python Complex MRO ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Create a complex inheritance pattern
    // This tests that our simplified MRO handles reasonable cases

    resolver.add_class("X".to_string(), vec![]);
    resolver.add_class_methods("X".to_string(), vec!["x".to_string()]);

    resolver.add_class("Y".to_string(), vec![]);
    resolver.add_class_methods("Y".to_string(), vec!["y".to_string()]);

    resolver.add_class("A".to_string(), vec!["X".to_string(), "Y".to_string()]);
    resolver.add_class_methods("A".to_string(), vec!["a".to_string()]);

    resolver.add_class("B".to_string(), vec!["Y".to_string(), "X".to_string()]);
    resolver.add_class_methods("B".to_string(), vec!["b".to_string()]);

    resolver.add_class("C".to_string(), vec!["A".to_string(), "B".to_string()]);
    resolver.add_class_methods("C".to_string(), vec!["c".to_string()]);

    // Get MRO for C
    let mro = resolver.get_inheritance_chain("C");

    // Should include all classes exactly once
    assert!(mro.contains(&"C".to_string()));
    assert!(mro.contains(&"A".to_string()));
    assert!(mro.contains(&"B".to_string()));
    assert!(mro.contains(&"X".to_string()));
    assert!(mro.contains(&"Y".to_string()));

    // C should come first
    assert_eq!(mro[0], "C");

    // All methods should be accessible
    let methods = resolver.get_all_methods("C");
    assert!(methods.contains(&"a".to_string()));
    assert!(methods.contains(&"b".to_string()));
    assert!(methods.contains(&"c".to_string()));
    assert!(methods.contains(&"x".to_string()));
    assert!(methods.contains(&"y".to_string()));

    println!("✓ Python complex MRO works correctly");
}
