use codanna::parsing::LanguageParser;
use codanna::parsing::go::parser::GoParser;
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;
use codanna::{FileId, SymbolKind};

#[test]
fn test_go_scope_resolution() {
    let mut parser = GoParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();
    let mut symbol_counter = SymbolCounter::new();

    let code = r#"
package scoping

var globalVar = "global"
const CONSTANT = 42

type MyStruct struct {
    Field string
}

func (m MyStruct) Method(param string) string {
    localVar := "local"
    
    if param != "" {
        blockVar := "block"
        _ = blockVar
    }
    
    for i := 0; i < 5; i++ {
        loopVar := i * 2
        _ = loopVar
    }
    
    return localVar
}

func ProcessData(input string) {
    result := "processed"
    
    if len(input) > 0 {
        temp := input + result
        _ = temp
    }
    
    switch input {
    case "test":
        caseVar := "test case"
        _ = caseVar
    default:
        defaultVar := "default case"  
        _ = defaultVar
    }
}
"#;

    let symbols = parser.parse(code, file_id, &mut symbol_counter);

    println!("Extracted {} symbols", symbols.len());
    for (i, symbol) in symbols.iter().enumerate() {
        println!(
            "  {}. {} ({:?}) - Scope: {:?}",
            i + 1,
            symbol.name,
            symbol.kind,
            symbol.scope_context
        );
    }

    // Verify package-level symbols
    assert!(symbols.iter().any(|s| s.name.as_ref() == "globalVar"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Module) | None)));

    assert!(
        symbols
            .iter()
            .any(|s| s.name.as_ref() == "CONSTANT" && matches!(s.kind, SymbolKind::Constant))
    );

    assert!(
        symbols
            .iter()
            .any(|s| s.name.as_ref() == "MyStruct" && matches!(s.kind, SymbolKind::Struct))
    );

    // Verify method and receiver
    assert!(
        symbols
            .iter()
            .any(|s| s.name.as_ref() == "Method" && matches!(s.kind, SymbolKind::Method))
    );

    assert!(symbols.iter().any(|s| s.name.as_ref() == "m"
        && matches!(s.kind, SymbolKind::Parameter)
        && matches!(s.scope_context, Some(ScopeContext::Parameter))));

    // Verify function-level variables (short declarations)
    assert!(symbols.iter().any(|s| s.name.as_ref() == "localVar"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    assert!(symbols.iter().any(|s| s.name.as_ref() == "result"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    // Verify block-scoped variables
    assert!(symbols.iter().any(|s| s.name.as_ref() == "blockVar"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    assert!(symbols.iter().any(|s| s.name.as_ref() == "temp"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    assert!(symbols.iter().any(|s| s.name.as_ref() == "caseVar"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    // Verify loop variables
    assert!(symbols.iter().any(|s| s.name.as_ref() == "i"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    assert!(symbols.iter().any(|s| s.name.as_ref() == "loopVar"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    println!("✅ Go scope resolution test passed");
}

#[test]
fn test_go_variable_shadowing() {
    let mut parser = GoParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();
    let mut symbol_counter = SymbolCounter::new();

    let code = r#"
package shadowing

var count = 10

func ProcessData() {
    count := 5  // Shadows package-level count
    
    {
        count := 1  // Shadows function-level count
        _ = count
    }
    
    if true {
        count := 2  // Another shadow in if block
        _ = count
    }
}
"#;

    let symbols = parser.parse(code, file_id, &mut symbol_counter);

    // Find all count variables - should have multiple with different scopes
    let count_vars: Vec<_> = symbols
        .iter()
        .filter(|s| s.name.as_ref() == "count")
        .collect();

    println!("Found {} 'count' variables:", count_vars.len());
    for (i, var) in count_vars.iter().enumerate() {
        println!(
            "  {}. count ({:?}) - Scope: {:?}",
            i + 1,
            var.kind,
            var.scope_context
        );
    }

    // Should have at least 4 count variables with different scopes:
    // 1. Package-level var count
    // 2. Function-level count := 5
    // 3. Block-level count := 1
    // 4. If-block-level count := 2
    assert!(
        count_vars.len() >= 4,
        "Should have at least 4 'count' variables for shadowing test"
    );

    // Verify we have both Variable and Variable kinds
    assert!(
        count_vars
            .iter()
            .any(|s| matches!(s.kind, SymbolKind::Variable))
    );

    println!("✅ Go variable shadowing test passed");
}

#[test]
fn test_go_complex_scoping() {
    let mut parser = GoParser::new().unwrap();
    let file_id = FileId::new(1).unwrap();
    let mut symbol_counter = SymbolCounter::new();

    let code = r#"
package complex

type Config struct {
    Name string
}

func (c Config) Process(data []string) {
    // Method receiver and parameter
    
    for index, value := range data {
        // Range variables in for loop scope
        processed := value + c.Name
        
        if len(processed) > 10 {
            // Block scope within for loop
            truncated := processed[:10]
            _ = truncated
        }
        
        switch {
        case index == 0:
            first := "first item"
            _ = first
        default:
            other := "other item"
            _ = other
        }
    }
}
"#;

    let symbols = parser.parse(code, file_id, &mut symbol_counter);

    println!("Complex scoping symbols:");
    for (i, symbol) in symbols.iter().enumerate() {
        println!(
            "  {}. {} ({:?}) - Scope: {:?}",
            i + 1,
            symbol.name,
            symbol.kind,
            symbol.scope_context
        );
    }

    // Verify receiver
    assert!(symbols.iter().any(|s| s.name.as_ref() == "c"
        && matches!(s.kind, SymbolKind::Parameter)
        && matches!(s.scope_context, Some(ScopeContext::Parameter))));

    // Verify method parameter
    assert!(
        symbols
            .iter()
            .any(|s| s.name.as_ref() == "data" && matches!(s.kind, SymbolKind::Parameter))
    );

    // Verify range variables
    assert!(symbols.iter().any(|s| s.name.as_ref() == "index"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    assert!(symbols.iter().any(|s| s.name.as_ref() == "value"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    // Verify nested scoped variables
    assert!(symbols.iter().any(|s| s.name.as_ref() == "processed"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    assert!(symbols.iter().any(|s| s.name.as_ref() == "truncated"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    assert!(symbols.iter().any(|s| s.name.as_ref() == "first"
        && matches!(s.kind, SymbolKind::Variable)
        && matches!(s.scope_context, Some(ScopeContext::Local { .. }))));

    println!("✅ Go complex scoping test passed");
}
