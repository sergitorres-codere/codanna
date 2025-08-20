//! Language Parser Regression Tests
//!
//! This module ensures that the integration of the Go parser doesn't break
//! existing functionality. It verifies:
//! 1. All existing language parsers still work correctly
//! 2. Go parser is properly registered and functional
//! 3. MCP server recognizes Go files correctly
//! 4. No conflicts exist between parsers
//!
//! These tests serve as a safety net to catch regressions when adding new
//! language support or modifying existing parsers.

use codanna::Settings;
use codanna::parsing::registry::{LanguageId, get_registry};
use codanna::parsing::{Language, ParserFactory};
use codanna::types::{FileId, SymbolCounter, SymbolKind};
use std::collections::HashSet;
use std::sync::Arc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Sample code for each supported language to test parsing
struct LanguageTestData {
    language: Language,
    sample_code: &'static str,
    expected_symbols: &'static [SymbolKind],
}

const LANGUAGE_TEST_DATA: &[LanguageTestData] = &[
    LanguageTestData {
        language: Language::Rust,
        sample_code: r#"
pub struct Person {
    pub name: String,
    age: u32,
}

impl Person {
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }
    
    pub fn greet(&self) -> String {
        format!("Hello, I'm {}", self.name)
    }
}

pub fn main() {
    let person = Person::new("Alice".to_string(), 30);
    println!("{}", person.greet());
}

pub const MAX_AGE: u32 = 150;
"#,
        expected_symbols: &[
            SymbolKind::Struct,
            SymbolKind::Function,
            SymbolKind::Method,
            SymbolKind::Constant,
        ],
    },
    LanguageTestData {
        language: Language::Python,
        sample_code: r#"
class Person:
    def __init__(self, name: str, age: int):
        self.name = name
        self.age = age
    
    def greet(self) -> str:
        return f"Hello, I'm {self.name}"

def main():
    person = Person("Alice", 30)
    print(person.greet())

MAX_AGE = 150
"#,
        expected_symbols: &[
            SymbolKind::Class,
            SymbolKind::Function,
            SymbolKind::Method,
            SymbolKind::Variable,
        ],
    },
    LanguageTestData {
        language: Language::TypeScript,
        sample_code: r#"
interface PersonInterface {
    name: string;
    age: number;
}

class Person implements PersonInterface {
    public name: string;
    private age: number;
    
    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
    }
    
    public greet(): string {
        return `Hello, I'm ${this.name}`;
    }
}

function main(): void {
    const person = new Person("Alice", 30);
    console.log(person.greet());
}

const MAX_AGE: number = 150;
"#,
        expected_symbols: &[
            SymbolKind::Interface,
            SymbolKind::Class,
            SymbolKind::Function,
            SymbolKind::Method,
            SymbolKind::Variable,
        ],
    },
    LanguageTestData {
        language: Language::Php,
        sample_code: r#"
<?php

class Person {
    public string $name;
    private int $age;
    
    public function __construct(string $name, int $age) {
        $this->name = $name;
        $this->age = $age;
    }
    
    public function greet(): string {
        return "Hello, I'm " . $this->name;
    }
}

function main(): void {
    $person = new Person("Alice", 30);
    echo $person->greet();
}

const MAX_AGE = 150;
"#,
        expected_symbols: &[
            SymbolKind::Class,
            SymbolKind::Function,
            SymbolKind::Method,
            SymbolKind::Constant,
        ],
    },
    LanguageTestData {
        language: Language::Go,
        sample_code: r#"
package main

import "fmt"

type Person struct {
    Name string
    age  int
}

func NewPerson(name string, age int) *Person {
    return &Person{Name: name, age: age}
}

func (p *Person) Greet() string {
    return fmt.Sprintf("Hello, I'm %s", p.Name)
}

func main() {
    person := NewPerson("Alice", 30)
    fmt.Println(person.Greet())
}

const MaxAge = 150
"#,
        expected_symbols: &[
            SymbolKind::Struct,
            SymbolKind::Function,
            SymbolKind::Method,
            SymbolKind::Constant,
        ],
    },
];

/// Test 1: Verify all existing language parsers still work correctly
#[test]
fn test_all_parsers_still_work() -> Result<()> {
    println!("\n=== Regression Test 1: All Parsers Still Work ===");

    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings);

    for test_data in LANGUAGE_TEST_DATA {
        println!("Testing {} parser...", test_data.language);

        // Create parser and behavior
        let parser_with_behavior = factory
            .create_parser_with_behavior(test_data.language)
            .map_err(|e| format!("Failed to create {} parser: {}", test_data.language, e))?;

        let mut parser = parser_with_behavior.parser;
        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();

        // Parse the sample code
        let symbols = parser.parse(test_data.sample_code, file_id, &mut counter);

        // Verify we got symbols
        assert!(
            !symbols.is_empty(),
            "{} parser should extract symbols from sample code",
            test_data.language
        );

        // Check that we have expected symbol kinds
        let extracted_kinds: HashSet<SymbolKind> = symbols.iter().map(|s| s.kind).collect();

        for expected_kind in test_data.expected_symbols {
            assert!(
                extracted_kinds.contains(expected_kind),
                "{} parser should extract {:?} symbols, but found kinds: {:?}",
                test_data.language,
                expected_kind,
                extracted_kinds
            );
        }

        println!(
            "  ✓ {} parser works correctly ({} symbols extracted)",
            test_data.language,
            symbols.len()
        );
    }

    println!("✅ All parsers are working correctly!");
    Ok(())
}

/// Test 2: Verify Go parser is properly registered in the language registry
#[test]
fn test_go_parser_registration() -> Result<()> {
    println!("\n=== Regression Test 2: Go Parser Registration ===");

    let registry_guard = get_registry().lock().unwrap();
    let go_id = LanguageId::new("go");

    // Test 2a: Go language is available
    assert!(
        registry_guard.is_available(go_id),
        "Go language should be available in the registry"
    );
    println!("  ✓ Go language is available in registry");

    // Test 2b: Go language is enabled by default
    let settings = Settings::default();
    assert!(
        registry_guard.is_enabled(go_id, &settings),
        "Go language should be enabled by default"
    );
    println!("  ✓ Go language is enabled by default");

    // Test 2c: Can get Go language definition
    let go_def = registry_guard.get(go_id);
    assert!(
        go_def.is_some(),
        "Should be able to get Go language definition"
    );

    let go_def = go_def.unwrap();
    assert_eq!(go_def.id(), go_id);
    assert_eq!(go_def.name(), "Go");
    println!("  ✓ Go language definition is correct");

    // Test 2d: Go file extensions are mapped correctly
    assert!(
        registry_guard.get_by_extension("go").is_some(),
        "Should recognize .go file extension"
    );
    assert!(
        registry_guard.get_by_extension(".go").is_some(),
        "Should recognize .go file extension with dot"
    );
    println!("  ✓ Go file extensions are mapped correctly");

    println!("✅ Go parser is properly registered!");
    Ok(())
}

/// Test 3: Verify MCP server recognizes Go files correctly
#[test]
fn test_mcp_recognizes_go_files() -> Result<()> {
    println!("\n=== Regression Test 3: MCP Recognizes Go Files ===");

    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings.clone());

    // Test 3a: Can create Go parser through factory
    let go_parser_result = factory.create_parser_with_behavior(Language::Go);
    assert!(
        go_parser_result.is_ok(),
        "Should be able to create Go parser through factory: {:?}",
        go_parser_result.err()
    );
    println!("  ✓ Can create Go parser through factory");

    // Test 3b: Registry recognizes .go files
    let registry_guard = get_registry().lock().unwrap();
    let go_by_ext = registry_guard.get_by_extension("go");
    assert!(
        go_by_ext.is_some(),
        "Registry should recognize .go file extension"
    );

    let go_lang = go_by_ext.unwrap();
    assert_eq!(go_lang.id(), LanguageId::new("go"));
    println!("  ✓ Registry correctly maps .go extension to Go language");

    // Test 3c: Enabled extensions include Go
    let enabled_extensions: Vec<&str> = registry_guard.enabled_extensions(&settings).collect();
    assert!(
        enabled_extensions.contains(&"go"),
        "Enabled extensions should include 'go', but found: {enabled_extensions:?}"
    );
    println!("  ✓ Go extension is in enabled extensions list");

    println!("✅ MCP server properly recognizes Go files!");
    Ok(())
}

/// Test 4: Ensure parsers don't interfere with each other
#[test]
fn test_no_parser_conflicts() -> Result<()> {
    println!("\n=== Regression Test 4: No Parser Conflicts ===");

    let settings = Arc::new(Settings::default());
    let factory = ParserFactory::new(settings);

    // Create all parsers simultaneously
    let mut parsers = Vec::new();
    for test_data in LANGUAGE_TEST_DATA {
        let parser_with_behavior = factory
            .create_parser_with_behavior(test_data.language)
            .map_err(|e| format!("Failed to create {} parser: {}", test_data.language, e))?;
        parsers.push((test_data, parser_with_behavior));
    }

    println!(
        "  ✓ Successfully created all {} parsers simultaneously",
        parsers.len()
    );

    // Test each parser with its own code and others' code
    for (i, (test_data, _)) in parsers.iter().enumerate() {
        // Create a fresh parser for this test
        let parser_with_behavior = factory
            .create_parser_with_behavior(test_data.language)
            .map_err(|e| format!("Failed to create {} parser: {}", test_data.language, e))?;
        let mut parser = parser_with_behavior.parser;
        let file_id = FileId::new((i + 1) as u32).unwrap();
        let mut counter = SymbolCounter::new();

        // Parse its own code - should work
        let symbols = parser.parse(test_data.sample_code, file_id, &mut counter);
        assert!(
            !symbols.is_empty(),
            "{} parser should successfully parse its own code",
            test_data.language
        );

        // Test parsing other languages' code - should not crash but may return no symbols
        for other_test_data in LANGUAGE_TEST_DATA {
            if other_test_data.language != test_data.language {
                let mut other_counter = SymbolCounter::new();
                let other_file_id = FileId::new((i + 100) as u32).unwrap();

                // This should not panic, though it may return empty results
                let _other_symbols = parser.parse(
                    other_test_data.sample_code,
                    other_file_id,
                    &mut other_counter,
                );
                // We don't assert on results here since parsers may legitimately
                // fail to parse other languages' code
            }
        }
    }

    println!("  ✓ All parsers handle cross-language code gracefully");
    println!("✅ No conflicts detected between parsers!");
    Ok(())
}

/// Test 5: Verify language registry integrity
#[test]
fn test_language_registry_integrity() -> Result<()> {
    println!("\n=== Regression Test 5: Language Registry Integrity ===");

    let registry_guard = get_registry().lock().unwrap();
    let settings = Settings::default();

    // Test 5a: All expected languages are present
    let expected_languages = vec![
        LanguageId::new("rust"),
        LanguageId::new("python"),
        LanguageId::new("typescript"),
        LanguageId::new("php"),
        LanguageId::new("go"),
    ];

    for lang_id in &expected_languages {
        assert!(
            registry_guard.is_available(*lang_id),
            "Language {lang_id:?} should be available in registry"
        );
        assert!(
            registry_guard.is_enabled(*lang_id, &settings),
            "Language {lang_id:?} should be enabled by default"
        );
    }
    println!("  ✓ All expected languages are present and enabled");

    // Test 5b: No duplicate extension mappings
    let mut seen_extensions = HashSet::new();
    for lang_def in registry_guard.iter_all() {
        for ext in lang_def.extensions() {
            assert!(
                seen_extensions.insert(ext),
                "File extension '{ext}' is mapped to multiple languages"
            );
        }
    }
    println!("  ✓ No duplicate extension mappings found");

    // Test 5c: All languages can create parsers
    for lang_id in &expected_languages {
        let lang_def = registry_guard.get(*lang_id).unwrap();
        let parser_result = lang_def.create_parser(&settings);
        assert!(
            parser_result.is_ok(),
            "Should be able to create parser for {:?}: {:?}",
            lang_id,
            parser_result.err()
        );

        let behavior = lang_def.create_behavior();
        // Just verify the behavior was created (no specific assertions needed)
        assert!(
            !behavior.module_separator().is_empty(),
            "Behavior should have a module separator"
        );
    }
    println!("  ✓ All languages can successfully create parsers and behaviors");

    // Test 5d: Registry iteration works correctly
    let all_count = registry_guard.iter_all().count();
    let enabled_count = registry_guard.iter_enabled(&settings).count();

    assert_eq!(
        all_count,
        expected_languages.len(),
        "iter_all() should return {} languages, got {}",
        expected_languages.len(),
        all_count
    );

    assert_eq!(
        enabled_count,
        expected_languages.len(),
        "iter_enabled() should return {} languages, got {}",
        expected_languages.len(),
        enabled_count
    );
    println!("  ✓ Registry iteration works correctly");

    println!("✅ Language registry integrity verified!");
    Ok(())
}

/// Integration test that runs all regression tests together
#[test]
fn test_complete_regression_suite() -> Result<()> {
    println!("\n🧪 Running Complete Language Parser Regression Suite");
    println!("{}", "=".repeat(60));

    // Run all regression tests in sequence
    test_all_parsers_still_work()?;
    test_go_parser_registration()?;
    test_mcp_recognizes_go_files()?;
    test_no_parser_conflicts()?;
    test_language_registry_integrity()?;

    println!("\n🎉 All regression tests passed!");
    println!("   The Go parser integration is working correctly");
    println!("   and has not broken any existing functionality.");
    println!("{}", "=".repeat(60));

    Ok(())
}
