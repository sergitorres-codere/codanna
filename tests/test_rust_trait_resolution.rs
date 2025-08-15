//! Test Rust-specific trait resolution using the proper Rust API
//!
//! This test verifies that RustTraitResolver correctly implements
//! the same functionality as the original hardcoded TraitResolver

use codanna::parsing::rust::{RustTraitResolver, RustBehavior};
use codanna::FileId;

#[test]
fn test_rust_trait_resolution_proper_api() {
    println!("\n=== Testing Rust Trait Resolution with Proper API ===");
    
    let mut resolver = RustTraitResolver::new();
    
    // Use the PROPER Rust API - separate methods for traits vs types
    
    // 1. Register trait methods (using add_trait_methods)
    resolver.add_trait_methods("Display".to_string(), vec!["fmt".to_string()]);
    resolver.add_trait_methods("Debug".to_string(), vec!["fmt".to_string()]);
    
    // 2. Register type's inherent methods (using add_inherent_methods)
    resolver.add_inherent_methods("MyStruct".to_string(), vec!["inherent_method".to_string()]);
    
    // 3. Register trait implementations (using add_trait_impl)
    resolver.add_trait_impl("MyStruct".to_string(), "Display".to_string(), FileId::new(1).unwrap());
    resolver.add_trait_impl("MyStruct".to_string(), "Debug".to_string(), FileId::new(1).unwrap());
    
    // Test resolution using the proper Rust method
    
    // Inherent methods return None (they're not from a trait)
    assert_eq!(resolver.resolve_method_trait("MyStruct", "inherent_method"), None);
    
    // Trait methods return the trait name
    let fmt_trait = resolver.resolve_method_trait("MyStruct", "fmt");
    assert!(fmt_trait == Some("Display") || fmt_trait == Some("Debug"));
    
    // Non-existent methods return None
    assert_eq!(resolver.resolve_method_trait("MyStruct", "unknown"), None);
    
    println!("✓ Rust trait resolution with proper API works correctly");
}

#[test]
fn test_rust_matches_original_trait_resolver() {
    // This test verifies our new implementation matches the original exactly
    let mut resolver = RustTraitResolver::new();
    
    // Exact same test from the original trait_resolver.rs tests
    resolver.add_trait_methods("Display".to_string(), vec!["fmt".to_string()]);
    resolver.add_trait_impl("MyStruct".to_string(), "Display".to_string(), FileId::new(1).unwrap());
    
    assert_eq!(
        resolver.resolve_method_trait("MyStruct", "fmt"),
        Some("Display")
    );
    
    assert_eq!(resolver.resolve_method_trait("MyStruct", "unknown"), None);
    
    println!("✓ New implementation matches original TraitResolver behavior");
}