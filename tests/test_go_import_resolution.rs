/// Integration tests for Go import resolution functionality
///
/// These tests verify that the Go parser can correctly resolve various types
/// of Go imports including relative imports, vendor directories, and go.mod handling.
use tempfile::TempDir;

#[test]
fn test_go_language_detection() {
    // Test that Go language is detected for various file extensions
    assert_eq!(
        codanna::parsing::Language::from_extension("go"),
        Some(codanna::parsing::Language::Go)
    );
    assert_eq!(
        codanna::parsing::Language::from_extension("go.mod"),
        Some(codanna::parsing::Language::Go)
    );
    assert_eq!(
        codanna::parsing::Language::from_extension("go.sum"),
        Some(codanna::parsing::Language::Go)
    );
}

#[test]
fn test_import_resolution_context() {
    use codanna::FileId;
    use codanna::parsing::go::resolution::GoResolutionContext;
    use codanna::storage::DocumentIndex;

    let temp_dir = TempDir::new().unwrap();
    let document_index = DocumentIndex::new(temp_dir.path()).unwrap();
    let mut context = GoResolutionContext::new(FileId::new(1).unwrap());

    // Test relative import resolution
    let result = context.resolve_relative_import("./utils", "myproject/internal");
    assert_eq!(result, Some("myproject/internal/utils".to_string()));

    let result = context.resolve_relative_import("../common", "myproject/internal");
    assert_eq!(result, Some("myproject/common".to_string()));

    // Test standard library detection
    assert!(context.is_standard_library_package("fmt"));
    assert!(context.is_standard_library_package("net/http"));
    assert!(!context.is_standard_library_package("github.com/user/repo"));

    // Test go.mod parsing
    let go_mod_content = r#"
module github.com/test/project

go 1.21

require (
    github.com/gin-gonic/gin v1.9.1
    github.com/lib/pq v1.10.7
)

replace github.com/old/module => ../local/module
"#;

    // Create temporary go.mod for testing
    let go_mod_path = temp_dir.path().join("go.mod");
    std::fs::write(&go_mod_path, go_mod_content).unwrap();

    let info = context.parse_go_mod(go_mod_path.to_str().unwrap());
    assert!(info.is_some());

    let info = info.unwrap();
    assert_eq!(
        info.module_name,
        Some("github.com/test/project".to_string())
    );
    assert_eq!(info.go_version, Some("1.21".to_string()));
    assert!(info.dependencies.contains_key("github.com/gin-gonic/gin"));
    assert!(info.replacements.contains_key("github.com/old/module"));

    // Test module replacements
    let result = context.apply_module_replacements("github.com/old/module", &info);
    assert_eq!(result, "../local/module");

    // Test enhanced import resolution with additional parameters
    context.add_import("fmt".to_string(), None);
    let result = context.resolve_imported_package_symbols(
        "fmt",
        "Println",
        &document_index,
        Some("myproject/main"),
        Some("/project/root"),
    );
    // Should return None since no symbols are indexed, but shouldn't panic
    assert!(result.is_none());
}

#[test]
fn test_go_behavior_import_resolution() {
    use codanna::FileId;
    use codanna::indexing::Import;
    use codanna::parsing::LanguageBehavior;
    use codanna::parsing::go::behavior::GoBehavior;
    use codanna::storage::DocumentIndex;

    let temp_dir = TempDir::new().unwrap();
    let doc_index = DocumentIndex::new(temp_dir.path()).unwrap();

    let behavior = GoBehavior::new();

    // Test basic import resolution doesn't panic
    let import = Import {
        path: "fmt".to_string(),
        alias: None,
        is_type_only: false,
        file_id: FileId::new(1).unwrap(),
        is_glob: false,
    };

    let result = behavior.resolve_import(&import, &doc_index);
    // Result might be None since no symbols are indexed, but should not panic
    assert!(result.is_none()); // No symbols indexed yet

    // Test relative import handling
    let relative_import = Import {
        path: "./utils".to_string(),
        alias: None,
        is_type_only: false,
        file_id: FileId::new(1).unwrap(),
        is_glob: false,
    };

    let result = behavior.resolve_import(&relative_import, &doc_index);
    assert!(result.is_none()); // Should handle gracefully

    // Test vendor import handling
    let vendor_import = Import {
        path: "github.com/vendor/lib".to_string(),
        alias: Some("vendorlib".to_string()),
        is_type_only: false,
        file_id: FileId::new(1).unwrap(),
        is_glob: false,
    };

    let result = behavior.resolve_import(&vendor_import, &doc_index);
    assert!(result.is_none()); // Should handle gracefully
}

#[test]
fn test_go_mod_info_structure() {
    use codanna::parsing::go::resolution::GoModInfo;

    let mut info = GoModInfo {
        module_name: Some("test/module".to_string()),
        go_version: Some("1.21".to_string()),
        ..Default::default()
    };
    info.dependencies
        .insert("dep1".to_string(), "v1.0.0".to_string());
    info.replacements
        .insert("old".to_string(), "new".to_string());

    // Test that the structure works as expected
    assert_eq!(info.module_name, Some("test/module".to_string()));
    assert_eq!(info.go_version, Some("1.21".to_string()));
    assert!(info.dependencies.contains_key("dep1"));
    assert!(info.replacements.contains_key("old"));
}

#[test]
fn test_fixture_files_exist() {
    use std::path::Path;

    // Verify our test fixtures exist
    assert!(
        Path::new("tests/fixtures/go/go.mod").exists(),
        "go.mod fixture should exist"
    );
    assert!(
        Path::new("tests/fixtures/go/imports.go").exists(),
        "imports.go fixture should exist"
    );

    // Check module project structure
    let module_project = Path::new("tests/fixtures/go/module_project");
    if module_project.exists() {
        assert!(
            module_project.join("go.mod").exists(),
            "module project go.mod should exist"
        );
        assert!(
            module_project.join("main.go").exists(),
            "module project main.go should exist"
        );
    }

    // Check vendor project structure
    let vendor_project = Path::new("tests/fixtures/go/vendor_project");
    if vendor_project.exists() {
        assert!(
            vendor_project.join("go.mod").exists(),
            "vendor project go.mod should exist"
        );
        assert!(
            vendor_project.join("main.go").exists(),
            "vendor project main.go should exist"
        );

        let vendor_lib = vendor_project.join("vendor/github.com/external/library/library.go");
        if vendor_lib.exists() {
            println!("Vendor library fixture exists: {}", vendor_lib.display());
        }
    }
}
