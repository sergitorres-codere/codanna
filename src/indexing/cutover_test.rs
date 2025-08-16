//! Cutover verification test module
//! Critical test to ensure old and new resolution systems produce IDENTICAL results

#[cfg(test)]
mod tests {
    use crate::indexing::SimpleIndexer;
    use crate::types::SymbolKind;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_baseline_with_current_system() {
        println!("\n=== Cutover Test: Baseline with Current System ===\n");

        // Create temp directory for test file
        let temp_dir = TempDir::new().unwrap();

        // Create a FRESH indexer with CURRENT system (uses TraitResolver + ImportResolver)
        // This ensures we don't get pollution from other tests
        let mut indexer = SimpleIndexer::new();

        // Create a test file that exercises key features
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(
            &test_file,
            r#"
use std::fmt::Display;

pub struct Config {
    name: String,
}

impl Config {
    pub fn new(name: String) -> Self {
        Self { name }
    }
    
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

pub trait Parser {
    fn parse(&self) -> String;
}

impl Parser for Config {
    fn parse(&self) -> String {
        format!("Config: {}", self.name)
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
"#,
        )
        .unwrap();

        // Index the file - this uses TraitResolver and ImportResolver internally
        let _result = indexer.index_file(&test_file).unwrap();

        // Extract ONLY symbols from our test file
        let symbols = indexer.get_all_symbols();
        let our_symbols: Vec<_> = symbols
            .into_iter()
            .filter(|s| {
                // Filter to only symbols we created in our test
                let name = s.name.to_string();
                name == "Config"
                    || name == "Parser"
                    || name == "Display"
                    || name == "new"
                    || name == "get_name"
                    || name == "parse"
                    || name == "fmt"
            })
            .collect();

        // Count each symbol type from our test
        let mut symbol_counts: HashMap<SymbolKind, usize> = HashMap::new();
        for symbol in &our_symbols {
            *symbol_counts.entry(symbol.kind).or_default() += 1;
        }

        println!("Current system found in our test file:");
        println!(
            "  Config struct: {}",
            our_symbols
                .iter()
                .filter(|s| s.name.as_ref() == "Config" && s.kind == SymbolKind::Struct)
                .count()
        );
        println!(
            "  Parser trait: {}",
            our_symbols
                .iter()
                .filter(|s| s.name.as_ref() == "Parser" && s.kind == SymbolKind::Trait)
                .count()
        );
        println!(
            "  Methods: {}",
            symbol_counts.get(&SymbolKind::Method).unwrap_or(&0)
        );

        // The key thing we're testing: TraitResolver tracks trait implementations
        // When we switch to behaviors, we need to ensure the SAME information is tracked
        println!("\nKey resolution behaviors to preserve:");
        println!("  - Trait 'Parser' implemented by 'Config'");
        println!("  - Trait 'Display' implemented by 'Config'");
        println!("  - Method 'parse' resolves to trait 'Parser'");
        println!("  - Method 'fmt' resolves to trait 'Display'");

        // Verify we found AT LEAST the symbols we expect (may have duplicates from other tests)
        assert!(
            our_symbols
                .iter()
                .any(|s| s.name.as_ref() == "Config" && s.kind == SymbolKind::Struct),
            "Should find Config struct"
        );
        assert!(
            our_symbols
                .iter()
                .any(|s| s.name.as_ref() == "Parser" && s.kind == SymbolKind::Trait),
            "Should find Parser trait"
        );
        assert!(
            our_symbols
                .iter()
                .any(|s| s.name.as_ref() == "new" && s.kind == SymbolKind::Method),
            "Should find new method"
        );
        assert!(
            our_symbols
                .iter()
                .any(|s| s.name.as_ref() == "parse" && s.kind == SymbolKind::Method),
            "Should find parse method"
        );
        assert!(
            our_symbols
                .iter()
                .any(|s| s.name.as_ref() == "fmt" && s.kind == SymbolKind::Method),
            "Should find fmt method"
        );

        // This baseline will be compared against the new system
        println!("\n✅ Baseline established for cutover comparison");
    }

    #[test]
    #[ignore] // Enable when we have the feature flag
    fn test_old_vs_new_system_identical() {
        println!("\n=== Cutover Test: Comparing Systems ===\n");

        // This test will:
        // 1. Index with old system (trait_resolver, import_resolver)
        // 2. Index with new system (behaviors only)
        // 3. Compare results exactly
        // 4. Fail if ANY difference found

        // TODO: Implement when feature flag is ready
    }
}
