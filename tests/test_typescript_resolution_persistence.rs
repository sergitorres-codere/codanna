//! Integration test for TypeScript resolution index persistence
//! Verifies that resolution index is saved to `.codanna/index/resolvers/typescript_resolution.json`

use codanna::config::{LanguageConfig, Settings};
use codanna::project_resolver::persist::{ResolutionIndex, ResolutionPersistence, ResolutionRules};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn test_resolution_index_persisted_to_codanna_directory() {
    // Create settings with TypeScript config files
    let mut settings = Settings::default();
    let ts_config = LanguageConfig {
        enabled: true,
        config_files: vec![PathBuf::from("examples/typescript/tsconfig.json")],
        extensions: vec![".ts".to_string(), ".tsx".to_string()],
        parser_options: Default::default(),
    };
    settings
        .languages
        .insert("typescript".to_string(), ts_config);

    // Get config from settings
    let ts_configs = &settings
        .languages
        .get("typescript")
        .expect("Should have TypeScript config")
        .config_files;

    if ts_configs.is_empty() || !ts_configs[0].exists() {
        println!("Skipping test - tsconfig not found");
        return;
    }

    // Use the actual .codanna directory
    let codanna_dir = Path::new(".codanna");
    let expected_file = codanna_dir
        .join("index")
        .join("resolvers")
        .join("typescript_resolution.json");

    // Clean up any existing file
    if expected_file.exists() {
        fs::remove_file(&expected_file).ok();
    }

    // Create persistence manager pointing to .codanna
    let persistence = ResolutionPersistence::new(codanna_dir);

    // Create an index using settings config
    let mut index = ResolutionIndex::new();
    let tsconfig_path = &ts_configs[0];

    // Use real tsconfig from settings
    let sha = codanna::project_resolver::sha::compute_file_sha(tsconfig_path)
        .expect("Should compute SHA");
    let config = codanna::parsing::typescript::tsconfig::read_tsconfig(tsconfig_path)
        .expect("Should parse tsconfig");

    index.update_sha(tsconfig_path, &sha);
    index.add_mapping("examples/typescript/**/*.ts", tsconfig_path);
    index.set_rules(
        tsconfig_path,
        ResolutionRules {
            base_url: config.compilerOptions.baseUrl,
            paths: config.compilerOptions.paths,
        },
    );

    // Save the index
    persistence
        .save("typescript", &index)
        .expect("Should save resolution index");

    // Verify file was created at expected location
    assert!(
        expected_file.exists(),
        "Resolution index should be saved at {expected_file:?}"
    );

    // Verify content is valid JSON
    let content = fs::read_to_string(&expected_file).expect("Should read resolution index file");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("Resolution index should be valid JSON");

    // Verify schema version
    assert_eq!(
        parsed["version"].as_str(),
        Some("1.0"),
        "Should have correct schema version"
    );

    // Clean up after test
    fs::remove_file(&expected_file).ok();

    println!(
        "✓ Resolution index successfully persisted to .codanna/index/resolvers/typescript_resolution.json"
    );
}
