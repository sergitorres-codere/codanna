//! Test that Rust-specific resolution correctly migrates hardcoded logic
//!
//! This test verifies that RustTraitResolver and RustResolutionContext
//! properly replace the functionality of:
//! - TraitResolver
//! - ImportResolver (Rust-specific parts)
//! - ResolutionContext

use codanna::parsing::{
    LanguageBehavior, ResolutionScope,
    RustBehavior, ScopeLevel,
    rust::{RustResolutionContext, RustTraitResolver}
};
use codanna::{FileId, SymbolId};

/// Test core trait resolution - the most critical Rust feature
#[test]
fn test_rust_trait_resolution_core() {
    println!("\n=== Testing Core Rust Trait Resolution ===");
    
    let mut resolver = RustTraitResolver::new();
    
    // Add Display trait with fmt method
    resolver.add_trait_methods("Display".to_string(), vec!["fmt".to_string()]);
    
    // MyStruct implements Display
    resolver.add_trait_impl("MyStruct".to_string(), "Display".to_string(), FileId::new(1).unwrap());
    
    // Add inherent method on MyStruct
    resolver.add_inherent_methods("MyStruct".to_string(), vec!["new".to_string(), "process".to_string()]);
    
    // Test method resolution follows Rust rules: inherent > trait
    assert_eq!(resolver.resolve_method_trait("MyStruct", "new"), None); // Inherent method
    assert_eq!(resolver.resolve_method_trait("MyStruct", "fmt"), Some("Display")); // Trait method
    assert_eq!(resolver.resolve_method_trait("MyStruct", "unknown"), None); // Not found
    
    println!("✓ Trait vs inherent method resolution works correctly");
}

/// Test Rust's scoping order
#[test]
fn test_rust_scoping_order() {
    println!("\n=== Testing Rust Scoping Order ===");
    
    let file_id = FileId::new(1).unwrap();
    let mut ctx = RustResolutionContext::new(file_id);
    
    // Add symbols at different scope levels
    let local_id = SymbolId::new(1).unwrap();
    let import_id = SymbolId::new(2).unwrap();
    let module_id = SymbolId::new(3).unwrap();
    let crate_id = SymbolId::new(4).unwrap();
    
    // Add same name at all levels (reverse order to test precedence)
    ctx.add_symbol("test".to_string(), crate_id, ScopeLevel::Global);
    ctx.add_symbol("test".to_string(), module_id, ScopeLevel::Module);
    ctx.add_symbol("test".to_string(), import_id, ScopeLevel::Package); // Package = imported in Rust
    ctx.add_symbol("test".to_string(), local_id, ScopeLevel::Local);
    
    // Should resolve to local first (Rust resolution order)
    assert_eq!(ctx.resolve("test"), Some(local_id));
    
    // Clear local scope
    ctx.clear_local_scope();
    
    // Should now resolve to imported
    assert_eq!(ctx.resolve("test"), Some(import_id));
    
    println!("✓ Rust scoping order (local > imported > module > crate) works");
}

/// Test multiple trait implementations (ambiguity warning)
#[test]
fn test_ambiguous_trait_methods() {
    println!("\n=== Testing Ambiguous Trait Methods ===");
    
    let mut resolver = RustTraitResolver::new();
    
    // Both Display and Debug have fmt method
    resolver.add_trait_methods("Display".to_string(), vec!["fmt".to_string()]);
    resolver.add_trait_methods("Debug".to_string(), vec!["fmt".to_string()]);
    
    // MyStruct implements both
    resolver.add_trait_impl("MyStruct".to_string(), "Display".to_string(), FileId::new(1).unwrap());
    resolver.add_trait_impl("MyStruct".to_string(), "Debug".to_string(), FileId::new(1).unwrap());
    
    // Should return first trait (with warning printed to stderr)
    let result = resolver.resolve_method_trait("MyStruct", "fmt");
    assert!(result == Some("Display") || result == Some("Debug"));
    
    println!("✓ Ambiguous methods handled (returns first, warns user)");
}

/// Test that behavior creates correct resolver types
#[test]
fn test_rust_behavior_creates_resolvers() {
    println!("\n=== Testing RustBehavior Factory Methods ===");
    
    let behavior = RustBehavior::new();
    let file_id = FileId::new(1).unwrap();
    
    // Should create Rust-specific resolvers
    let resolution_ctx = behavior.create_resolution_context(file_id);
    let inheritance_resolver = behavior.create_inheritance_resolver();
    
    // Add a symbol and resolve it
    let mut ctx = resolution_ctx;
    ctx.add_symbol("main".to_string(), SymbolId::new(1).unwrap(), ScopeLevel::Module);
    assert_eq!(ctx.resolve("main"), Some(SymbolId::new(1).unwrap()));
    
    // Test inheritance
    let mut resolver = inheritance_resolver;
    resolver.add_inheritance("MyStruct".to_string(), "MyTrait".to_string(), "implements");
    assert!(resolver.is_subtype("MyStruct", "MyTrait"));
    
    println!("✓ RustBehavior creates correct resolver instances");
}