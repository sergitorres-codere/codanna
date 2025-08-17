//! Test that Python-specific resolution correctly implements LEGB scoping
//!
//! This test verifies that PythonInheritanceResolver and PythonResolutionContext
//! properly implement Python's unique features:
//! - LEGB scoping (Local, Enclosing, Global, Built-in)
//! - Method Resolution Order (MRO)
//! - Module imports with aliasing

use codanna::parsing::{
    InheritanceResolver, LanguageBehavior, PythonBehavior, ResolutionScope, ScopeLevel,
    python::{PythonInheritanceResolver, PythonResolutionContext},
};
use codanna::{FileId, SymbolId};

/// Test Python's LEGB scoping order
#[test]
fn test_python_legb_scoping() {
    println!("\n=== Testing Python LEGB Scoping Order ===");

    let file_id = FileId::new(1).unwrap();
    let mut ctx = PythonResolutionContext::new(file_id);

    // Add symbols at different scope levels
    let local_id = SymbolId::new(1).unwrap();
    let enclosing_id = SymbolId::new(2).unwrap();
    let global_id = SymbolId::new(3).unwrap();
    let imported_id = SymbolId::new(5).unwrap();

    // Add same name at all levels to test precedence
    ctx.add_symbol("test".to_string(), global_id, ScopeLevel::Global);
    ctx.add_symbol("test".to_string(), imported_id, ScopeLevel::Package); // Imported
    ctx.add_symbol("test".to_string(), local_id, ScopeLevel::Local);

    // Should resolve to local first (L in LEGB)
    assert_eq!(ctx.resolve("test"), Some(local_id));

    // Clear local scope
    ctx.clear_local_scope();

    // Should now resolve to global (G in LEGB, since no E)
    assert_eq!(ctx.resolve("test"), Some(global_id));

    // Test enclosing scope (E in LEGB)
    ctx.add_symbol("nested".to_string(), enclosing_id, ScopeLevel::Local);
    ctx.push_enclosing_scope(); // Move locals to enclosing

    // Add new local with same name
    ctx.add_symbol("nested".to_string(), local_id, ScopeLevel::Local);
    assert_eq!(ctx.resolve("nested"), Some(local_id)); // Local wins

    ctx.clear_local_scope();
    assert_eq!(ctx.resolve("nested"), Some(enclosing_id)); // Now enclosing

    println!("✓ Python LEGB scoping order works correctly");
}

/// Test Python's Method Resolution Order (MRO)
#[test]
fn test_python_mro() {
    println!("\n=== Testing Python Method Resolution Order (MRO) ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Create a diamond inheritance pattern:
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D

    // A has method 'foo'
    resolver.add_class_methods("A".to_string(), vec!["foo".to_string()]);

    // B inherits from A and overrides 'foo', adds 'bar'
    resolver.add_inheritance("B".to_string(), "A".to_string(), "extends");
    resolver.add_class_methods("B".to_string(), vec!["foo".to_string(), "bar".to_string()]);

    // C inherits from A and adds 'baz'
    resolver.add_inheritance("C".to_string(), "A".to_string(), "extends");
    resolver.add_class_methods("C".to_string(), vec!["baz".to_string()]);

    // D inherits from B and C (multiple inheritance)
    resolver.add_inheritance("D".to_string(), "B".to_string(), "extends");
    resolver.add_inheritance("D".to_string(), "C".to_string(), "extends");
    resolver.add_class_methods("D".to_string(), vec!["qux".to_string()]);

    // Test MRO for D: D -> B -> C -> A
    let mro = resolver.get_inheritance_chain("D");
    assert_eq!(mro[0], "D");
    assert_eq!(mro[1], "B");
    assert!(mro.contains(&"C".to_string()));
    assert!(mro.contains(&"A".to_string()));

    // Test method resolution
    assert_eq!(resolver.resolve_method("D", "qux"), Some("D".to_string())); // D's own method
    assert_eq!(resolver.resolve_method("D", "bar"), Some("B".to_string())); // From B
    assert_eq!(resolver.resolve_method("D", "baz"), Some("C".to_string())); // From C
    assert_eq!(resolver.resolve_method("D", "foo"), Some("B".to_string())); // B's override wins

    // Test all methods available to D
    let all_methods = resolver.get_all_methods("D");
    assert!(all_methods.contains(&"qux".to_string()));
    assert!(all_methods.contains(&"bar".to_string()));
    assert!(all_methods.contains(&"baz".to_string()));
    assert!(all_methods.contains(&"foo".to_string()));

    println!("✓ Python MRO with multiple inheritance works correctly");
}

/// Test single inheritance chain
#[test]
fn test_python_single_inheritance() {
    println!("\n=== Testing Python Single Inheritance ===");

    let mut resolver = PythonInheritanceResolver::new();

    // Animal -> Dog -> Puppy
    resolver.add_class_methods(
        "Animal".to_string(),
        vec!["eat".to_string(), "sleep".to_string()],
    );
    resolver.add_inheritance("Dog".to_string(), "Animal".to_string(), "inherits");
    resolver.add_class_methods(
        "Dog".to_string(),
        vec!["bark".to_string(), "eat".to_string()],
    ); // Override eat
    resolver.add_inheritance("Puppy".to_string(), "Dog".to_string(), "inherits");
    resolver.add_class_methods("Puppy".to_string(), vec!["play".to_string()]);

    // Test inheritance chain
    assert!(resolver.is_subtype("Puppy", "Dog"));
    assert!(resolver.is_subtype("Puppy", "Animal"));
    assert!(resolver.is_subtype("Dog", "Animal"));
    assert!(!resolver.is_subtype("Animal", "Dog"));

    // Test method resolution
    assert_eq!(
        resolver.resolve_method("Puppy", "play"),
        Some("Puppy".to_string())
    );
    assert_eq!(
        resolver.resolve_method("Puppy", "bark"),
        Some("Dog".to_string())
    );
    assert_eq!(
        resolver.resolve_method("Puppy", "eat"),
        Some("Dog".to_string())
    ); // Dog's override
    assert_eq!(
        resolver.resolve_method("Puppy", "sleep"),
        Some("Animal".to_string())
    );

    println!("✓ Python single inheritance chain works correctly");
}

/// Test Python behavior factory methods
#[test]
fn test_python_behavior_creates_resolvers() {
    println!("\n=== Testing PythonBehavior Factory Methods ===");

    let behavior = PythonBehavior::new();

    // Test that Python behavior has correct language-specific features
    assert!(!behavior.supports_traits()); // Python doesn't have traits
    assert!(!behavior.supports_inherent_methods()); // Python methods are always on classes
    assert_eq!(behavior.module_separator(), ".");

    // Test visibility parsing (Python uses naming conventions)
    use codanna::Visibility;
    assert_eq!(
        behavior.parse_visibility("def public_func():"),
        Visibility::Public
    );
    assert_eq!(
        behavior.parse_visibility("def _protected_func():"),
        Visibility::Module
    );
    assert_eq!(
        behavior.parse_visibility("def __private_func():"),
        Visibility::Private
    );
    assert_eq!(
        behavior.parse_visibility("def __init__(self):"),
        Visibility::Public
    ); // Special method

    println!("✓ PythonBehavior has correct language-specific features");
}

/// Test Python import resolution patterns
#[test]
fn test_python_import_matching() {
    println!("\n=== Testing Python Import Matching ===");

    let behavior = PythonBehavior::new();

    // Test exact match
    let exact_match = behavior.import_matches_symbol("os.path", "os.path", None);
    println!("  Exact match: os.path == os.path? {exact_match}");
    assert!(exact_match);

    // Test relative import with dots
    let relative_import =
        behavior.import_matches_symbol(".utils", "package.utils", Some("package"));
    println!("  Relative import: .utils -> package.utils (from package)? {relative_import}");
    assert!(relative_import);

    // Test parent relative import
    // From parent.child, ..sibling goes up to root and imports sibling
    let parent_import =
        behavior.import_matches_symbol("..sibling", "sibling", Some("parent.child"));
    println!("  Parent import: ..sibling -> sibling (from parent.child)? {parent_import}");
    assert!(parent_import);

    // Test that .sibling from parent.child gives parent.sibling
    let same_level_import =
        behavior.import_matches_symbol(".sibling", "parent.sibling", Some("parent.child"));
    println!("  Same level: .sibling -> parent.sibling (from parent.child)? {same_level_import}");
    assert!(same_level_import);

    // Test absolute import that might be partial
    assert!(behavior.import_matches_symbol("module", "package.module", Some("package.subpackage")));

    // Test multi-part import as suffix
    assert!(behavior.import_matches_symbol("sub.module", "package.sub.module", Some("package")));

    // Test non-match
    assert!(!behavior.import_matches_symbol("wrong.module", "different.module", Some("package")));

    println!("✓ Python import matching patterns work correctly");
}

/// Test Python module path calculation
#[test]
fn test_python_module_paths() {
    use std::path::Path;

    println!("\n=== Testing Python Module Path Calculation ===");

    let behavior = PythonBehavior::new();
    let root = Path::new("/project");

    // Test regular module
    let module_path =
        behavior.module_path_from_file(Path::new("/project/src/mypackage/module.py"), root);
    assert_eq!(module_path, Some("mypackage.module".to_string()));

    // Test __init__.py (represents the package)
    let init_path =
        behavior.module_path_from_file(Path::new("/project/src/mypackage/__init__.py"), root);
    assert_eq!(init_path, Some("mypackage".to_string()));

    // Test nested module
    let nested_path =
        behavior.module_path_from_file(Path::new("/project/mypackage/subpackage/module.py"), root);
    assert_eq!(nested_path, Some("mypackage.subpackage.module".to_string()));

    // Test __main__.py
    let main_path = behavior.module_path_from_file(Path::new("/project/__main__.py"), root);
    assert_eq!(main_path, Some("__main__".to_string()));

    println!("✓ Python module path calculation works correctly");
}
