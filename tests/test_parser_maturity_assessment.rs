//! Comprehensive parser maturity assessment
//! Tests real-world complex code to document what we have, what we miss, and nice-to-haves

use codanna::parsing::{Language, ParserFactory};
// Note: LanguageParser trait is used implicitly through Box<dyn LanguageParser> in ParserWithBehavior
// The parser.parse() method call on line 49 is using the LanguageParser trait method
use codanna::Symbol;
use codanna::types::{FileId, SymbolCounter, SymbolKind};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::Arc;

/// Structure to track parser capabilities
#[derive(Debug, Default)]
struct ParserCapabilities {
    /// Symbols we successfully extract
    pub extracted: HashMap<SymbolKind, usize>,
    /// Total symbols found
    pub total_symbols: usize,
    /// Features we support
    pub supported_features: HashSet<String>,
    /// Known missing features
    pub missing_features: HashSet<String>,
    /// Nice-to-have features
    pub nice_to_have: HashSet<String>,
    /// Sample symbols for inspection
    pub sample_symbols: Vec<String>,
}

fn assess_parser(language: Language, file_path: &str) -> ParserCapabilities {
    let mut capabilities = ParserCapabilities::default();

    // Read the comprehensive test file
    let code =
        fs::read_to_string(file_path).unwrap_or_else(|_| panic!("Failed to read {file_path}"));

    // Parse the file
    let settings = Arc::new(codanna::Settings::default());
    let factory = ParserFactory::new(settings);

    let parser_with_behavior = factory
        .create_parser_with_behavior(language)
        .unwrap_or_else(|_| panic!("Failed to create {language:?} parser"));

    let mut parser = parser_with_behavior.parser;
    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    let symbols = parser.parse(&code, file_id, &mut counter);
    capabilities.total_symbols = symbols.len();

    // Count symbols by kind
    for symbol in &symbols {
        *capabilities.extracted.entry(symbol.kind).or_insert(0) += 1;

        // Collect sample symbols (first 10)
        if capabilities.sample_symbols.len() < 10 {
            capabilities.sample_symbols.push(format!(
                "{} ({:?})",
                symbol.name.as_ref(),
                symbol.kind
            ));
        }
    }

    // Analyze what features we support based on extracted symbols
    analyze_features(language, &symbols, &mut capabilities);

    capabilities
}

fn analyze_features(language: Language, symbols: &[Symbol], capabilities: &mut ParserCapabilities) {
    match language {
        Language::Rust => analyze_rust_features(symbols, capabilities),
        Language::Python => analyze_python_features(symbols, capabilities),
        Language::TypeScript => analyze_typescript_features(symbols, capabilities),
        Language::Php => analyze_php_features(symbols, capabilities),
        _ => {}
    }
}

fn analyze_rust_features(symbols: &[Symbol], cap: &mut ParserCapabilities) {
    // Check what we have
    if cap.extracted.contains_key(&SymbolKind::Function) {
        cap.supported_features.insert("Functions".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Struct) {
        cap.supported_features.insert("Structs".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Trait) {
        cap.supported_features.insert("Traits".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Enum) {
        cap.supported_features.insert("Enums".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::TypeAlias) {
        cap.supported_features.insert("Type aliases".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Constant) {
        cap.supported_features
            .insert("Constants/Statics".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Method) {
        cap.supported_features.insert("Methods".to_string());
    }

    // Check for specific Rust features in symbol names/signatures
    let symbol_names: HashSet<_> = symbols.iter().map(|s| s.name.as_ref()).collect();

    if symbol_names.contains("async_operation") {
        cap.supported_features.insert("Async functions".to_string());
    }
    if symbol_names.contains("GenericContainer") {
        cap.supported_features.insert("Generic types".to_string());
    }
    if symbol_names.contains("const_function") {
        cap.supported_features.insert("Const functions".to_string());
    }

    // Known missing features
    if !symbol_names.contains("generated_func") {
        cap.missing_features
            .insert("Macro-generated code".to_string());
    }
    if !symbol_names.contains("MyUnion") {
        cap.missing_features.insert("Unions".to_string());
    }
    if !cap.extracted.contains_key(&SymbolKind::Macro) {
        cap.missing_features.insert("Macro definitions".to_string());
    }

    // Nice-to-have features
    cap.nice_to_have.insert("Lifetime parameters".to_string());
    cap.nice_to_have
        .insert("Associated constants in traits".to_string());
    cap.nice_to_have
        .insert("Higher-ranked trait bounds".to_string());
    cap.nice_to_have.insert("Extern functions".to_string());
    cap.nice_to_have.insert("Inline modules".to_string());
}

fn analyze_python_features(symbols: &[Symbol], cap: &mut ParserCapabilities) {
    // Check what we have
    if cap.extracted.contains_key(&SymbolKind::Function) {
        cap.supported_features.insert("Functions".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Class) {
        cap.supported_features.insert("Classes".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Method) {
        cap.supported_features.insert("Methods".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Variable) {
        cap.supported_features
            .insert("Module variables".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Constant) {
        cap.supported_features
            .insert("Constants (by convention)".to_string());
    }

    let symbol_names: HashSet<_> = symbols.iter().map(|s| s.name.as_ref()).collect();

    if symbol_names.contains("async_fetch") {
        cap.supported_features.insert("Async functions".to_string());
    }
    if symbol_names.contains("SimpleClass") {
        cap.supported_features
            .insert("Classes with methods".to_string());
    }
    if symbol_names.contains("timer_decorator") {
        cap.supported_features.insert("Decorators".to_string());
    }

    // Check for special methods
    for symbol in symbols {
        if symbol.name.as_ref().starts_with("__") && symbol.name.as_ref().ends_with("__") {
            cap.supported_features
                .insert("Special methods (__init__, __str__, etc.)".to_string());
            break;
        }
    }

    // Missing features
    if !symbol_names.contains("square") && !symbol_names.contains("filtered_map") {
        cap.missing_features
            .insert("Lambda expressions with meaningful names".to_string());
    }
    if !cap.extracted.contains_key(&SymbolKind::Enum) {
        cap.missing_features.insert("Enum classes".to_string());
    }
    if !symbol_names.contains("Color") && !symbol_names.contains("Status") {
        cap.missing_features.insert("Enum members".to_string());
    }
    cap.missing_features.insert("Dataclass fields".to_string());
    cap.missing_features.insert("TypedDict fields".to_string());
    cap.missing_features.insert("Protocol methods".to_string());

    // Nice-to-have
    cap.nice_to_have
        .insert("Type annotations parsing".to_string());
    cap.nice_to_have.insert("Decorator parameters".to_string());
    cap.nice_to_have
        .insert("Generic type parameters".to_string());
    cap.nice_to_have.insert("Metaclass detection".to_string());
    cap.nice_to_have.insert("Context managers".to_string());
    cap.nice_to_have.insert("Generator expressions".to_string());
}

fn analyze_typescript_features(symbols: &[Symbol], cap: &mut ParserCapabilities) {
    if cap.extracted.contains_key(&SymbolKind::Function) {
        cap.supported_features.insert("Functions".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Class) {
        cap.supported_features.insert("Classes".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Interface) {
        cap.supported_features.insert("Interfaces".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Method) {
        cap.supported_features.insert("Methods".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Field) {
        cap.supported_features.insert("Class fields".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::TypeAlias) {
        cap.supported_features.insert("Type aliases".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Enum) {
        cap.supported_features.insert("Enums".to_string());
    }

    let symbol_names: HashSet<_> = symbols.iter().map(|s| s.name.as_ref()).collect();

    // Check for specific TypeScript features found
    if symbol_names.contains("User") || symbol_names.contains("Admin") {
        cap.supported_features
            .insert("Interface inheritance".to_string());
    }
    if symbol_names.contains("GenericContainer") {
        cap.supported_features.insert("Generic classes".to_string());
    }

    // Missing features - check if specific expected symbols exist
    if !symbol_names.contains("Utils") && !symbol_names.contains("formatDate") {
        cap.missing_features.insert("Namespaces".to_string());
    }
    cap.missing_features.insert("Decorators".to_string());
    cap.missing_features
        .insert("Generic constraints".to_string());
    cap.missing_features.insert("Type guards".to_string());
    cap.missing_features
        .insert("Module augmentation".to_string());

    // Nice-to-have
    cap.nice_to_have
        .insert("Union/Intersection types".to_string());
    cap.nice_to_have.insert("Conditional types".to_string());
    cap.nice_to_have
        .insert("Template literal types".to_string());
    cap.nice_to_have.insert("Mapped types".to_string());
    cap.nice_to_have.insert("JSX/TSX support".to_string());
    cap.nice_to_have
        .insert("Import/Export tracking".to_string());
}

fn analyze_php_features(symbols: &[Symbol], cap: &mut ParserCapabilities) {
    if cap.extracted.contains_key(&SymbolKind::Function) {
        cap.supported_features.insert("Functions".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Class) {
        cap.supported_features.insert("Classes".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Interface) {
        cap.supported_features.insert("Interfaces".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Trait) {
        cap.supported_features.insert("Traits".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Method) {
        cap.supported_features.insert("Methods".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Constant) {
        cap.supported_features.insert("Constants".to_string());
    }
    if cap.extracted.contains_key(&SymbolKind::Variable) {
        cap.supported_features
            .insert("Properties/Variables".to_string());
    }

    let symbol_names: HashSet<_> = symbols.iter().map(|s| s.name.as_ref()).collect();

    // Check for specific PHP features by looking for known symbols
    if symbol_names.contains("unionTypes") || symbol_names.contains("intersectionTypes") {
        cap.supported_features.insert("Type hints".to_string());
    }
    if symbol_names.contains("namedParams") {
        cap.supported_features
            .insert("Function signatures".to_string());
    }

    // Missing features - check if specific symbols exist
    if !cap.extracted.contains_key(&SymbolKind::Enum)
        && !symbol_names.contains("Status")
        && !symbol_names.contains("Priority")
    {
        cap.missing_features.insert("PHP 8.1+ Enums".to_string());
    }
    if !symbol_names.contains("Route") && !symbol_names.contains("Inject") {
        cap.missing_features
            .insert("Attributes (PHP 8.0+)".to_string());
    }
    if !symbol_names.contains("ReadonlyClass") {
        cap.missing_features
            .insert("Constructor property promotion".to_string());
    }
    if !symbol_names.contains("anonymousClass") {
        cap.missing_features.insert("Anonymous classes".to_string());
    }
    cap.missing_features
        .insert("Union/Intersection types".to_string());

    // Nice-to-have
    cap.nice_to_have
        .insert("Readonly properties (PHP 8.1+)".to_string());
    cap.nice_to_have
        .insert("Match expressions (PHP 8.0+)".to_string());
    cap.nice_to_have.insert("Named arguments".to_string());
    cap.nice_to_have.insert("Fibers (PHP 8.1+)".to_string());
    cap.nice_to_have
        .insert("Generic annotations in docblocks".to_string());
}

#[test]
fn test_rust_parser_maturity() {
    println!("\n{}", "=".repeat(70));
    println!("RUST PARSER MATURITY ASSESSMENT");
    println!("{}\n", "=".repeat(70));

    let capabilities = assess_parser(Language::Rust, "examples/rust/comprehensive.rs");

    print_assessment(&capabilities, "Rust");
}

#[test]
fn test_python_parser_maturity() {
    println!("\n{}", "=".repeat(70));
    println!("PYTHON PARSER MATURITY ASSESSMENT");
    println!("{}\n", "=".repeat(70));

    let capabilities = assess_parser(Language::Python, "examples/python/comprehensive.py");

    print_assessment(&capabilities, "Python");
}

#[test]
fn test_typescript_parser_maturity() {
    println!("\n{}", "=".repeat(70));
    println!("TYPESCRIPT PARSER MATURITY ASSESSMENT");
    println!("{}\n", "=".repeat(70));

    let capabilities = assess_parser(Language::TypeScript, "examples/typescript/comprehensive.ts");

    print_assessment(&capabilities, "TypeScript");
}

#[test]
fn test_php_parser_maturity() {
    println!("\n{}", "=".repeat(70));
    println!("PHP PARSER MATURITY ASSESSMENT");
    println!("{}\n", "=".repeat(70));

    let capabilities = assess_parser(Language::Php, "examples/php/comprehensive.php");

    print_assessment(&capabilities, "PHP");
}

fn print_assessment(cap: &ParserCapabilities, language: &str) {
    println!("📊 {language} Parser Assessment Results");
    println!("{}", "-".repeat(50));

    println!("\n📈 Symbol Extraction Summary:");
    println!("  Total symbols extracted: {}", cap.total_symbols);
    println!("\n  Breakdown by type:");
    let mut sorted_kinds: Vec<_> = cap.extracted.iter().collect();
    sorted_kinds.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (kind, count) in sorted_kinds {
        println!("    {kind:?}: {count}");
    }

    println!("\n📝 Sample Symbols:");
    for sample in &cap.sample_symbols {
        println!("    - {sample}");
    }

    println!(
        "\n✅ Supported Features ({}):",
        cap.supported_features.len()
    );
    for feature in &cap.supported_features {
        println!("    ✓ {feature}");
    }

    println!("\n❌ Missing Features ({}):", cap.missing_features.len());
    for feature in &cap.missing_features {
        println!("    ✗ {feature}");
    }

    println!("\n💡 Nice-to-Have Features ({}):", cap.nice_to_have.len());
    for feature in &cap.nice_to_have {
        println!("    ○ {feature}");
    }

    // Calculate maturity score
    let base_score = if cap.total_symbols > 0 { 50.0 } else { 0.0 };
    let feature_score = (cap.supported_features.len() as f32) * 2.5;
    let missing_penalty = (cap.missing_features.len() as f32) * 2.0;
    let maturity_score = (base_score + feature_score - missing_penalty).clamp(0.0, 100.0);

    println!("\n🎯 Maturity Score: {maturity_score:.1}/100");
    println!(
        "  (Base: {base_score:.0} + Features: {feature_score:.0} - Missing: {missing_penalty:.0})"
    );

    println!("\n📋 Recommendations:");
    if cap.missing_features.len() > 5 {
        println!("  ⚠️ Several critical features missing - needs attention");
    } else if !cap.missing_features.is_empty() {
        println!("  📌 Address missing features for better coverage");
    } else {
        println!("  ✅ Good coverage of language features");
    }

    if cap.total_symbols < 20 {
        println!("  ⚠️ Low symbol count - parser may be missing constructs");
    }

    println!("\n{}", "=".repeat(70));
}
