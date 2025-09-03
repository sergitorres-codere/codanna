//! Debug test for TypeScript import extraction

use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::types::FileId;

#[test]
fn test_imports_directly() {
    let mut parser = TypeScriptParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();

    // Test 1: Default import
    let code = "import React from 'react';";
    eprintln!("Testing code: {code}");
    let imports = parser.find_imports(code, file_id);
    eprintln!("Found {} imports", imports.len());
    for (i, imp) in imports.iter().enumerate() {
        eprintln!(
            "  Import[{}]: path='{}', alias={:?}, is_glob={}",
            i, imp.path, imp.alias, imp.is_glob
        );
    }
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].path, "react");
    assert_eq!(
        imports[0].alias,
        Some("React".to_string()),
        "Default import should have alias 'React'"
    );
    assert!(!imports[0].is_glob);

    // Test 2: Namespace import
    let code = "import * as utils from './utils';";
    let imports = parser.find_imports(code, file_id);
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].path, "./utils");
    assert_eq!(
        imports[0].alias,
        Some("utils".to_string()),
        "Namespace import should have alias 'utils'"
    );
    assert!(
        imports[0].is_glob,
        "Namespace import should have is_glob=true"
    );

    // Test 3: Named imports only (per-specifier entries)
    let code = "import { Component, useState } from 'react';";
    let imports = parser.find_imports(code, file_id);
    assert_eq!(imports.len(), 2, "should have two per-specifier imports");
    assert!(
        imports
            .iter()
            .any(|i| i.path == "react" && i.alias.as_deref() == Some("Component") && !i.is_glob)
    );
    assert!(
        imports
            .iter()
            .any(|i| i.path == "react" && i.alias.as_deref() == Some("useState") && !i.is_glob)
    );

    // Test 4: Mixed default and named (default + per-specifier)
    let code = "import React, { Component } from 'react';";
    let imports = parser.find_imports(code, file_id);
    assert_eq!(
        imports.len(),
        2,
        "should have default + one named specifier"
    );
    assert!(
        imports
            .iter()
            .any(|i| i.path == "react" && i.alias.as_deref() == Some("React") && !i.is_glob)
    );
    assert!(
        imports
            .iter()
            .any(|i| i.path == "react" && i.alias.as_deref() == Some("Component") && !i.is_glob)
    );
}

#[test]
fn debug_mixed_import() {
    // Force test to fail so we see output
    let mut parser = TypeScriptParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();

    // Test mixed import - React is default, Component and useState are named
    let code = "import React, { Component } from 'react';";
    println!("\n=== Testing: {code} ===");

    let imports = parser.find_imports(code, file_id);
    println!("Found {} imports:", imports.len());
    for imp in &imports {
        println!(
            "  - path: '{}', alias: {:?}, glob: {}",
            imp.path, imp.alias, imp.is_glob
        );
    }

    // Test just default
    let code2 = "import React from 'react';";
    println!("\n=== Testing: {code2} ===");

    let imports2 = parser.find_imports(code2, file_id);
    println!("Found {} imports:", imports2.len());
    for imp in &imports2 {
        println!(
            "  - path: '{}', alias: {:?}, glob: {}",
            imp.path, imp.alias, imp.is_glob
        );
    }

    // Test just namespace
    let code3 = "import * as utils from './utils';";
    println!("\n=== Testing: {code3} ===");

    let imports3 = parser.find_imports(code3, file_id);
    println!("Found {} imports:", imports3.len());
    for imp in &imports3 {
        println!(
            "  - path: '{}', alias: {:?}, glob: {}",
            imp.path, imp.alias, imp.is_glob
        );
    }

    // Force test to show output by checking for aliases
    assert!(
        imports2[0].alias.is_some(),
        "Default import should have alias!"
    );
    assert!(
        imports3[0].alias.is_some(),
        "Namespace import should have alias!"
    );
}
