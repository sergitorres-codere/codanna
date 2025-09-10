#[cfg(test)]
mod tests {
    use codanna::parsing::LanguageParser;
    use codanna::parsing::typescript::TypeScriptParser;

    #[test]
    fn test_typescript_call_tracking() {
        let code = r#"
function test() {
    console.log('hello');
    otherFunction();
}

const arrow = () => {
    console.log('arrow');
    helperFunction();
};

class MyClass {
    method() {
        console.log('method');
        this.otherMethod();
    }
}
"#;

        let mut parser = TypeScriptParser::new().expect("Failed to create parser");

        // First check what symbols are created
        let file_id = codanna::types::FileId::new(1).unwrap();
        let mut counter = codanna::types::SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);
        println!("Found {} symbols:", symbols.len());
        for symbol in &symbols {
            println!("  Symbol: {} ({:?})", symbol.name, symbol.kind);
        }

        // Then check what calls are found
        let calls = parser.find_calls(code);
        println!("\nFound {} calls:", calls.len());
        for (caller, called, range) in &calls {
            println!("  {} -> {} at line {}", caller, called, range.start_line);
        }

        // We expect to find console.log and function calls
        assert!(
            !calls.is_empty(),
            "Should find at least some function calls"
        );
    }
}
