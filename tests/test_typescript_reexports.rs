//! Sprint 4 Test: TypeScript Re-export Handling
//!
//! This test verifies that re-exports are properly extracted and tracked.

use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::types::FileId;

#[test]
fn test_typescript_reexport_extraction() {
    println!("\n=== TypeScript Re-export Extraction Test ===\n");

    let code = r#"
// Regular re-exports
export { Component } from 'react';
export { Helper as PublicHelper } from './utils/helper';

// Wildcard re-export
export * from './utils';

// Default re-export
export { default as MyButton } from './Button';

// Type-only re-export
export type { Props } from './types';

// Mixed re-export (type and value)
export { type Config, createConfig } from './config';
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();
    let imports = parser.find_imports(code, file_id);

    println!("Found {} re-exports:", imports.len());
    for import in &imports {
        println!(
            "  - path: '{}', alias: {:?}, is_type_only: {}",
            import.path, import.alias, import.is_type_only
        );
    }

    // Verify we found all re-exports
    assert!(!imports.is_empty(), "Should find re-exports");

    // Check specific re-exports
    assert!(
        imports.iter().any(|i| i.path == "react"),
        "Should find 'react' re-export"
    );
    assert!(
        imports.iter().any(|i| i.path == "./utils"),
        "Should find './utils' wildcard re-export"
    );
    assert!(
        imports.iter().any(|i| i.path == "./Button"),
        "Should find './Button' default re-export"
    );
    assert!(
        imports
            .iter()
            .any(|i| i.path == "./types" && i.is_type_only),
        "Should find './types' type-only re-export"
    );

    println!("\n✅ Re-export extraction working correctly!");
}

#[test]
fn test_typescript_barrel_file() {
    println!("\n=== TypeScript Barrel File Test ===\n");

    // Typical barrel file pattern
    let code = r#"
// Barrel file exporting everything from subdirectories
export * from './components/Button';
export * from './components/Input';
export * from './components/Modal';

// Selective exports
export { Header, Footer } from './layout';
export { useAuth, useApi } from './hooks';

// Type exports
export type { ButtonProps } from './components/Button';
export type { ModalOptions } from './components/Modal';
"#;

    let mut parser = TypeScriptParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();
    let imports = parser.find_imports(code, file_id);

    println!("Barrel file re-exports: {}", imports.len());

    let components = imports
        .iter()
        .filter(|i| i.path.contains("components"))
        .count();
    println!("  - Component exports: {components}");

    let type_exports = imports.iter().filter(|i| i.is_type_only).count();
    println!("  - Type-only exports: {type_exports}");

    assert!(imports.len() >= 7, "Should find multiple barrel exports");
    assert!(components >= 3, "Should find component exports");
    assert!(type_exports >= 2, "Should find type exports");

    println!("\n✅ Barrel file handling working!");
}
