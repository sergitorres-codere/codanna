//! Verification test for TypeScript import extraction
//! This test provides PROOF that imports are correctly extracted
//! by parsing the actual examples/typescript/import_test.ts file

use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::types::FileId;
use std::fs;

#[test]
fn verify_typescript_imports_with_proof() {
    println!("\n=== TYPESCRIPT IMPORT EXTRACTION VERIFICATION ===");
    println!("Parsing: examples/typescript/import_test.ts");
    println!("This test provides PROOF of what imports are extracted\n");

    // Read the actual test file
    let code =
        fs::read_to_string("examples/typescript/import_test.ts").expect("Failed to read test file");

    let mut parser = TypeScriptParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();

    // Extract imports
    let imports = parser.find_imports(&code, file_id);

    println!("FOUND {} IMPORTS:\n", imports.len());
    println!("{:<50} | {:<20} | {:<10}", "PATH", "ALIAS", "IS_GLOB");
    println!("{}", "-".repeat(85));

    for (i, import) in imports.iter().enumerate() {
        println!(
            "{:2}. {:<47} | {:<20} | {:<10}",
            i + 1,
            import.path,
            import.alias.as_deref().unwrap_or("None"),
            if import.is_glob { "true" } else { "false" }
        );
    }

    println!("\n=== VERIFICATION AGAINST EXPECTATIONS ===\n");

    // Define our expectations based on the ACTUAL test file content
    let expectations = vec![
        // Line 5: import { Component, useState } from 'react';
        ("react", None, false, "Named imports from react"),
        // Line 6: import { Helper as H, util } from './utils/helper';
        (
            "./utils/helper",
            None,
            false,
            "Named imports with alias from helpers",
        ),
        // Line 9: import React from 'react';
        ("react", Some("React"), false, "Default import React"),
        // Line 10: import Button from './components/Button';
        (
            "./components/Button",
            Some("Button"),
            false,
            "Default import Button",
        ),
        // Line 13: import * as lodash from 'lodash';
        ("lodash", Some("lodash"), true, "Namespace import lodash"),
        // Line 14: import * as utils from '../utils';
        ("../utils", Some("utils"), true, "Namespace import utils"),
        // Line 17: import DefaultExport, { namedExport } from './mixed';
        (
            "./mixed",
            Some("DefaultExport"),
            false,
            "Mixed default and named",
        ),
        // Line 20: import type { Props, State } from './types';
        ("./types", None, false, "Type-only named imports"),
        // Line 21: import { type Config, createConfig } from './config';
        ("./config", None, false, "Mixed type and value imports"),
        // Line 24: import './styles.css';
        ("./styles.css", None, false, "Side-effect import"),
        // Line 25: import 'polyfill';
        ("polyfill", None, false, "Side-effect import (no extension)"),
        // Line 28: export { Component } from 'react';
        ("react", None, false, "Re-export named"),
        // Line 29: export * from './utils';
        ("./utils", None, true, "Re-export all"),
        // Line 30: export { default as MyButton } from './Button';
        ("./Button", None, false, "Re-export default as named"),
        // Line 33: export { Helper as PublicHelper } from './utils/helper';
        ("./utils/helper", None, false, "Re-export with rename"),
        // Line 36: export type { Props } from './types';
        ("./types", None, false, "Type re-export"),
        // Line 39: import { something } from './sibling';
        ("./sibling", None, false, "Sibling import"),
        // Line 40: import { parent } from '../parent';
        ("../parent", None, false, "Parent directory import"),
        // Line 41: import { deep } from '../../deep/module';
        ("../../deep/module", None, false, "Deep parent import"),
        // Line 42: import { indexed } from './folder';
        ("./folder", None, false, "Folder import (implies index)"),
        // Line 45: import { Request } from '@types/express';
        ("@types/express", None, false, "Scoped package import"),
        // Line 46: import { service } from '@app/services';
        ("@app/services", None, false, "App-scoped import"),
    ];

    println!(
        "{:<5} | {:<25} | {:<15} | {:<10} | {:<10} | Description",
        "Test", "Path", "Alias", "IsGlob", "Result"
    );
    println!("{}", "-".repeat(100));

    let mut all_passed = true;
    for (i, (expected_path, expected_alias, expected_glob, description)) in
        expectations.iter().enumerate()
    {
        let found = imports.iter().find(|imp| {
            imp.path == *expected_path
                && imp.alias.as_deref() == *expected_alias
                && imp.is_glob == *expected_glob
        });

        let passed = found.is_some();
        if !passed {
            all_passed = false;
        }

        println!(
            "{:4}. | {:<25} | {:<15} | {:<10} | {:<10} | {}",
            i + 1,
            expected_path,
            expected_alias.unwrap_or("None"),
            if *expected_glob { "true" } else { "false" },
            if passed { "✅ PASS" } else { "❌ FAIL" },
            description
        );

        if !passed {
            // Show what we actually found with this path
            let matching_path = imports
                .iter()
                .filter(|imp| imp.path == *expected_path)
                .collect::<Vec<_>>();
            if !matching_path.is_empty() {
                println!("      FOUND WITH PATH '{expected_path}': ");
                for imp in matching_path {
                    println!("        - alias={:?}, is_glob={}", imp.alias, imp.is_glob);
                }
            } else {
                println!("      NOT FOUND: No import with path '{expected_path}'");
            }
        }
    }

    println!("\n=== SUMMARY ===");
    println!("Expected imports: {}", expectations.len());
    println!("Found imports: {}", imports.len());
    println!(
        "All tests passed: {}",
        if all_passed { "✅ YES" } else { "❌ NO" }
    );

    // Show any unexpected imports
    println!("\n=== UNEXPECTED IMPORTS (if any) ===");
    let mut has_unexpected = false;
    for import in &imports {
        let is_expected = expectations.iter().any(|(path, alias, glob, _)| {
            import.path == *path && import.alias.as_deref() == *alias && import.is_glob == *glob
        });

        if !is_expected {
            has_unexpected = true;
            println!(
                "  - path='{}', alias={:?}, is_glob={}",
                import.path, import.alias, import.is_glob
            );
        }
    }

    if !has_unexpected {
        println!("  None - all imports were expected!");
    }

    assert!(all_passed, "Some import expectations were not met!");
    assert_eq!(
        imports.len(),
        expectations.len(),
        "Number of imports doesn't match expectations"
    );

    println!("\n✅ ALL VERIFICATIONS PASSED!");
}
