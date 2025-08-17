#[test]
fn test_typescript_find_implementations() {
    use codanna::Settings;
    use codanna::parsing::LanguageDefinition;
    use std::sync::Arc;

    let settings = Arc::new(Settings::default());
    let ts_lang = codanna::parsing::typescript::TypeScriptLanguage;
    let mut parser = ts_lang.create_parser(&settings).unwrap();

    let code = r#"
interface ITest {
    test(): void;
}

class TestClass implements ITest {
    test(): void {
        console.log("test");
    }
}

class Another extends TestClass {
    // Inheritance test
}

class Third implements ITest {
    test(): void {}
}
"#;

    let impls = parser.find_implementations(code);
    println!("Found {} implementations:", impls.len());
    for (implementor, implemented, range) in &impls {
        println!(
            "  {} implements {} at line {}",
            implementor, implemented, range.start_line
        );
    }

    let extends = parser.find_extends(code);
    println!("Found {} extends relationships:", extends.len());
    for (child, parent, range) in &extends {
        println!(
            "  {} extends {} at line {}",
            child, parent, range.start_line
        );
    }

    // We should find:
    // - TestClass implements ITest
    // - Third implements ITest
    assert!(impls.len() >= 2, "Should find at least 2 implementations");

    // Check specific implementations
    assert!(
        impls
            .iter()
            .any(|(imp, trait_name, _)| imp == &"TestClass" && trait_name == &"ITest"),
        "TestClass should implement ITest"
    );
    assert!(
        impls
            .iter()
            .any(|(imp, trait_name, _)| imp == &"Third" && trait_name == &"ITest"),
        "Third should implement ITest"
    );

    // Check extends relationship
    assert!(
        extends
            .iter()
            .any(|(child, parent, _)| child == &"Another" && parent == &"TestClass"),
        "Another should extend TestClass"
    );
}
