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
            "  {} implements/extends {} at line {}",
            implementor, implemented, range.start_line
        );
    }

    // We should find:
    // - TestClass implements ITest
    // - Another extends TestClass
    // - Another implements ITest
    assert!(impls.len() >= 2, "Should find at least 2 implementations");

    // Check specific implementations
    assert!(
        impls
            .iter()
            .any(|(imp, trait_name, _)| imp == &"TestClass" && trait_name == &"ITest")
    );
    assert!(
        impls
            .iter()
            .any(|(imp, base, _)| imp == &"Another" && base == &"TestClass")
    );
}
