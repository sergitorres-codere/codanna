//! Integration tests for CLI language filtering functionality
//!
//! Tests that language filtering works correctly when using retrieve commands
//! with the lang: parameter.

use codanna::{Settings, SimpleIndexer};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn test_retrieve_symbol_with_language_filter() {
    println!("\n=== Testing retrieve symbol with language filtering ===");

    // Create a temporary directory with mixed language files
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create a Rust file with a process function
    let rust_file = temp_path.join("main.rs");
    fs::write(
        &rust_file,
        r#"
fn process() {
    println!("Rust process");
}

fn main() {
    process();
}
"#,
    )
    .unwrap();

    // Create a Python file with a process function
    let python_file = temp_path.join("main.py");
    fs::write(
        &python_file,
        r#"
def process():
    """Python process function"""
    print("Python process")

if __name__ == "__main__":
    process()
"#,
    )
    .unwrap();

    // Create a TypeScript file with a process function
    let ts_file = temp_path.join("main.ts");
    fs::write(
        &ts_file,
        r#"
function process(): void {
    console.log("TypeScript process");
}

process();
"#,
    )
    .unwrap();

    // Create settings and indexer
    let settings = Arc::new(Settings {
        workspace_root: Some(temp_path.to_path_buf()),
        ..Default::default()
    });

    let mut indexer = SimpleIndexer::with_settings(settings);

    // Index all files
    println!("Indexing test files...");
    indexer.index_directory(temp_path, false, false).unwrap();

    // Test 1: Find symbols without language filter (should find all)
    println!("\nTest 1: Find 'process' without language filter");
    let all_symbols = indexer.find_symbols_by_name("process", None);
    println!("Found {} 'process' symbols", all_symbols.len());
    for symbol in &all_symbols {
        println!(
            "  - {} in file: {:?} with language: {:?}",
            symbol.name,
            indexer.get_file_path(symbol.file_id).unwrap_or_default(),
            symbol.language_id
        );
    }
    assert!(
        all_symbols.len() >= 3,
        "Should find process in all 3 language files"
    );

    // Test 2: Filter by Rust
    println!("\nTest 2: Find 'process' with Rust filter");
    let rust_symbols = indexer.find_symbols_by_name("process", Some("rust"));
    println!("Found {} Rust 'process' symbols", rust_symbols.len());
    assert_eq!(rust_symbols.len(), 1, "Should find 1 Rust process function");
    assert!(rust_symbols[0].language_id.is_some());
    assert_eq!(rust_symbols[0].language_id.unwrap().as_str(), "rust");

    // Test 3: Filter by Python
    println!("\nTest 3: Find 'process' with Python filter");
    let python_symbols = indexer.find_symbols_by_name("process", Some("python"));
    println!("Found {} Python 'process' symbols", python_symbols.len());
    assert_eq!(
        python_symbols.len(),
        1,
        "Should find 1 Python process function"
    );
    assert!(python_symbols[0].language_id.is_some());
    assert_eq!(python_symbols[0].language_id.unwrap().as_str(), "python");

    // Test 4: Filter by TypeScript
    println!("\nTest 4: Find 'process' with TypeScript filter");
    let ts_symbols = indexer.find_symbols_by_name("process", Some("typescript"));
    println!("Found {} TypeScript 'process' symbols", ts_symbols.len());
    assert_eq!(
        ts_symbols.len(),
        1,
        "Should find 1 TypeScript process function"
    );
    assert!(ts_symbols[0].language_id.is_some());
    assert_eq!(ts_symbols[0].language_id.unwrap().as_str(), "typescript");

    // Test 5: Filter by non-existent language
    println!("\nTest 5: Find 'process' with Java filter (non-existent)");
    let java_symbols = indexer.find_symbols_by_name("process", Some("java"));
    println!("Found {} Java 'process' symbols", java_symbols.len());
    assert_eq!(java_symbols.len(), 0, "Should find no Java symbols");

    println!("\n=== All retrieve symbol language filter tests passed ===");
}

#[test]
fn test_retrieve_search_with_language_filter() {
    println!("\n=== Testing retrieve search with language filtering ===");

    // Create a temporary directory with mixed language files
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create files with 'parse' functions in different languages
    let rust_file = temp_path.join("parser.rs");
    fs::write(
        &rust_file,
        r#"
fn parse_config() -> Config {
    // Parse configuration
    Config::default()
}

fn parse_json(input: &str) -> Value {
    // Parse JSON
    serde_json::from_str(input).unwrap()
}
"#,
    )
    .unwrap();

    let python_file = temp_path.join("parser.py");
    fs::write(
        &python_file,
        r#"
def parse_yaml(content):
    """Parse YAML content"""
    pass

def parse_toml(content):
    """Parse TOML content"""
    pass
"#,
    )
    .unwrap();

    let ts_file = temp_path.join("parser.ts");
    fs::write(
        &ts_file,
        r#"
function parse_html(html: string): Document {
    // Parse HTML
    return new DOMParser().parseFromString(html, 'text/html');
}

function parse_markdown(md: string): string {
    // Parse Markdown
    return marked(md);
}
"#,
    )
    .unwrap();

    // Create settings and indexer
    let settings = Arc::new(Settings {
        workspace_root: Some(temp_path.to_path_buf()),
        ..Default::default()
    });

    let mut indexer = SimpleIndexer::with_settings(settings);

    // Index all files
    println!("Indexing test files...");
    indexer.index_directory(temp_path, false, false).unwrap();

    // Test 1: Search without language filter
    println!("\nTest 1: Search 'parse' without language filter");
    let all_results = indexer.search("parse", 20, None, None, None).unwrap();
    println!("Found {} 'parse' results", all_results.len());
    for result in &all_results {
        println!("  - {}: {}", result.name, result.file_path);
    }
    assert!(
        all_results.len() >= 4,
        "Should find parse functions in all languages"
    );

    // Test 2: Search with Rust filter
    println!("\nTest 2: Search 'parse' with Rust filter");
    let rust_results = indexer
        .search("parse", 20, None, None, Some("rust"))
        .unwrap();
    println!("Found {} Rust 'parse' results", rust_results.len());
    for result in &rust_results {
        println!("  - {}: {}", result.name, result.file_path);
    }
    assert_eq!(rust_results.len(), 2, "Should find 2 Rust parse functions");

    // Test 3: Search with Python filter
    println!("\nTest 3: Search 'parse' with Python filter");
    let python_results = indexer
        .search("parse", 20, None, None, Some("python"))
        .unwrap();
    println!("Found {} Python 'parse' results", python_results.len());
    for result in &python_results {
        println!("  - {}: {}", result.name, result.file_path);
    }
    assert_eq!(
        python_results.len(),
        2,
        "Should find 2 Python parse functions"
    );

    // Test 4: Search with TypeScript filter
    println!("\nTest 4: Search 'parse' with TypeScript filter");
    let ts_results = indexer
        .search("parse", 20, None, None, Some("typescript"))
        .unwrap();
    println!("Found {} TypeScript 'parse' results", ts_results.len());
    for result in &ts_results {
        println!("  - {}: {}", result.name, result.file_path);
    }
    assert_eq!(
        ts_results.len(),
        2,
        "Should find 2 TypeScript parse functions"
    );

    // Test 5: Search with non-existent language
    println!("\nTest 5: Search 'parse' with Java filter (non-existent)");
    let java_results = indexer
        .search("parse", 20, None, None, Some("java"))
        .unwrap();
    println!("Found {} Java 'parse' results", java_results.len());
    assert_eq!(java_results.len(), 0, "Should find no Java parse functions");

    println!("\n=== All retrieve search language filter tests passed ===");
}
