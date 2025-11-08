// Error handling and edge case tests for C# parser
//
// These tests verify that the parser handles malformed code, edge cases,
// and unusual scenarios gracefully without panicking.

use codanna::parsing::csharp::CSharpParser;
use codanna::parsing::LanguageParser;
use codanna::types::{FileId, SymbolCounter};

// ====================
// Malformed Code Tests
// ====================

#[test]
fn test_missing_closing_brace() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Foo {
            public void Bar() {
                // Missing closing braces
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    // Should not panic, should return partial results
    let _symbols = parser.parse(code, file_id, &mut counter);

    // Parser should handle gracefully - may or may not extract symbols
    // The key is it doesn't panic
    assert!(true, "Parser did not panic on malformed code");
}

#[test]
fn test_unclosed_string_literal() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Test {
            public string GetMessage() {
                return "unclosed string
            }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let _symbols = parser.parse(code, file_id, &mut counter);

    // Should not panic
    assert!(true);
}

#[test]
fn test_incomplete_generic_type() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Container {
            public List< GetItems() {
                return new List<string>();
            }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let _symbols = parser.parse(code, file_id, &mut counter);

    // Should not panic
    assert!(true);
}

#[test]
fn test_missing_semicolons() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Test {
            public int x
            public string name
            public void Method() {
                int y = 5
            }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Should handle gracefully
    assert!(true);
}

#[test]
fn test_incomplete_method_signature() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Test {
            public void Method(int a,
            // Incomplete parameter list
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(true);
}

// ====================
// Edge Case Tests
// ====================

#[test]
fn test_empty_file() {
    let mut parser = CSharpParser::new().unwrap();
    let code = "";

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.is_empty(), "Empty file should have no symbols");
}

#[test]
fn test_only_comments() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        // This is a comment
        /// <summary>
        /// This is a doc comment
        /// </summary>
        /* Multi-line
           comment */
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.is_empty(), "File with only comments should have no symbols");
}

#[test]
fn test_only_whitespace() {
    let mut parser = CSharpParser::new().unwrap();
    let code = "   \n\n\t\t\n   \n";

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.is_empty());
}

#[test]
fn test_very_long_identifier() {
    let mut parser = CSharpParser::new().unwrap();
    let long_name = "A".repeat(1000);
    let code = format!(r#"
        public class {} {{
            public void Method() {{ }}
        }}
    "#, long_name);

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(&code, file_id, &mut counter);

    // Should handle long identifiers
    assert!(symbols.iter().any(|s| s.name.len() > 500));
}

#[test]
fn test_deeply_nested_namespaces() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        namespace Level1.Level2.Level3.Level4.Level5.Level6.Level7.Level8.Level9.Level10 {
            public class DeepClass { }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let class = symbols.iter().find(|s| &*s.name == "DeepClass");
    assert!(class.is_some());

    if let Some(module_path) = &class.unwrap().module_path {
        // Should track the full namespace path
        assert!(module_path.contains("Level10"));
    }
}

#[test]
fn test_deeply_nested_classes() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Level1 {
            public class Level2 {
                public class Level3 {
                    public class Level4 {
                        public class Level5 {
                            public void DeepMethod() { }
                        }
                    }
                }
            }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Should extract all nested classes
    assert!(symbols.iter().any(|s| &*s.name == "Level1"));
    assert!(symbols.iter().any(|s| &*s.name == "Level5"));
    assert!(symbols.iter().any(|s| &*s.name == "DeepMethod"));
}

#[test]
fn test_unicode_identifiers() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Café {
            public void Naïve() { }
            public string Résumé { get; set; }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "Café"));
    assert!(symbols.iter().any(|s| &*s.name == "Naïve"));
    assert!(symbols.iter().any(|s| &*s.name == "Résumé"));
}

#[test]
fn test_very_long_generic_list() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Container<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> {
            public void Method<U1, U2, U3, U4, U5>() { }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "Container"));
    assert!(symbols.iter().any(|s| &*s.name == "Method"));
}

#[test]
fn test_multiple_visibility_modifiers_invalid() {
    let mut parser = CSharpParser::new().unwrap();
    // This is invalid C# but should be handled gracefully
    let code = r#"
        public private class InvalidClass {
            public protected void InvalidMethod() { }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Should still extract symbols even from invalid code
    assert!(!symbols.is_empty());
}

// ====================
// Invalid Constructs
// ====================

#[test]
fn test_invalid_generic_constraints() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Test<T> where T : InvalidConstraint, , ,AnotherOne {
            public void Method() { }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Should handle gracefully
    assert!(symbols.iter().any(|s| &*s.name == "Test"));
}

#[test]
fn test_malformed_attributes() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        [Incomplete(
        public class Test {
            [Missing]Argument]
            public void Method() { }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(true); // No panic
}

#[test]
fn test_incomplete_xml_documentation() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        /// <summary>
        /// This summary is not closed
        /// <param name="x">Param without closing tag
        public class Test {
            public void Method(int x) { }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Should still extract symbols
    assert!(symbols.iter().any(|s| &*s.name == "Test"));

    // Documentation should be preserved even if malformed
    let test_class = symbols.iter().find(|s| &*s.name == "Test");
    if let Some(class) = test_class {
        // Should have doc comment (even if malformed)
        assert!(class.doc_comment.is_some());
    }
}

// ====================
// Extreme Cases
// ====================

#[test]
fn test_extremely_long_method_chain() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Test {
            public void Method() {
                var result = obj
                    .Method1()
                    .Method2()
                    .Method3()
                    .Method4()
                    .Method5()
                    .Method6()
                    .Method7()
                    .Method8()
                    .Method9()
                    .Method10()
                    .Method11()
                    .Method12()
                    .Method13()
                    .Method14()
                    .Method15();
            }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "Method"));
}

#[test]
fn test_many_parameters() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Test {
            public void Method(
                int p1, string p2, bool p3, double p4, float p5,
                int p6, string p7, bool p8, double p9, float p10,
                int p11, string p12, bool p13, double p14, float p15,
                int p16, string p17, bool p18, double p19, float p20
            ) { }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "Method"));
}

#[test]
fn test_many_using_directives() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        using System;
        using System.Collections;
        using System.Collections.Generic;
        using System.Linq;
        using System.Text;
        using System.Threading.Tasks;
        using System.IO;
        using System.Net;
        using System.Net.Http;
        using System.Diagnostics;
        using Microsoft.Extensions.DependencyInjection;
        using Microsoft.Extensions.Logging;
        using Microsoft.AspNetCore.Mvc;
        using MyApp.Services;
        using MyApp.Models;
        using MyApp.Controllers;
        using MyApp.Data;
        using MyApp.Repositories;
        using MyApp.Helpers;
        using MyApp.Extensions;

        namespace Test {
            public class TestClass { }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Check imports
    let imports = parser.find_imports(code, file_id);
    assert!(imports.len() >= 10, "Should find many using directives");
}

// ====================
// Partial Code Tests
// ====================

#[test]
fn test_class_without_body() {
    let mut parser = CSharpParser::new().unwrap();
    let code = "public class TestClass";

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // May or may not extract, but shouldn't panic
    assert!(true);
}

#[test]
fn test_method_without_implementation() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public interface ITest {
            void Method();
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Interface methods without implementation are valid
    assert!(symbols.iter().any(|s| &*s.name == "ITest"));
    assert!(symbols.iter().any(|s| &*s.name == "Method"));
}

// ====================
// Special Characters
// ====================

#[test]
fn test_escaped_identifiers() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class @class {
            public void @void() { }
            public int @int { get; set; }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // C# allows @ prefix for keywords as identifiers
    // Parser may or may not strip the @ (implementation detail)
    assert!(
        symbols.iter().any(|s| s.name.contains("class")),
        "Should find class (possibly with @ prefix)"
    );
}

#[test]
fn test_verbatim_strings() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Test {
            public string path = @"C:\Users\Test";
            public string multiline = @"Line 1
                Line 2
                Line 3";
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "Test"));
}

// ====================
// Relationship Tests with Errors
// ====================

#[test]
fn test_find_calls_on_malformed_code() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public class Test {
            public void Caller() {
                Callee(  // Missing closing paren
            }
        }
    "#;

    // Should not panic
    let _calls = parser.find_calls(code);

    // May or may not find calls, but shouldn't crash
    assert!(true);
}

#[test]
fn test_find_implementations_on_incomplete_code() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        public interface ITest { }
        public class TestImpl : ITest
        // Missing class body
    "#;

    // Should not panic
    let _implementations = parser.find_implementations(code);

    assert!(true);
}

// ====================
// Recursion Depth Tests
// ====================

#[test]
fn test_very_deep_nesting() {
    let mut parser = CSharpParser::new().unwrap();

    // Create deeply nested blocks (but not infinite)
    let mut code = String::from("public class Test { public void Method() {");
    for _ in 0..100 {
        code.push_str(" if (true) { ");
    }
    code.push_str(" int x = 1; ");
    for _ in 0..100 {
        code.push_str(" } ");
    }
    code.push_str(" } }");

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();

    // Should handle deep nesting up to recursion limit
    let _symbols = parser.parse(&code, file_id, &mut counter);

    // Parser has recursion depth protection
    assert!(true, "Parser did not overflow stack");
}

// ====================
// Encoding Tests
// ====================

#[test]
fn test_mixed_line_endings() {
    let mut parser = CSharpParser::new().unwrap();
    let code = "public class Test {\r\n    public void Method1() { }\n    public void Method2() { }\r\n}";

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    assert!(symbols.iter().any(|s| &*s.name == "Test"));
    assert!(symbols.iter().any(|s| &*s.name == "Method1"));
    assert!(symbols.iter().any(|s| &*s.name == "Method2"));
}

// ====================
// Graceful Degradation
// ====================

#[test]
fn test_partial_extraction_from_damaged_file() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
        // This class is valid
        public class ValidClass {
            public void ValidMethod() { }
        }

        // This class is broken
        public class BrokenClass {
            public void Method( /* missing parameters and body */

        // This class is valid again
        public class AnotherValidClass {
            public void AnotherMethod() { }
        }
    "#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Should extract at least the valid classes
    assert!(
        symbols.iter().any(|s| &*s.name == "ValidClass"),
        "Should extract valid classes even when others are broken"
    );

    // May or may not extract the broken class (implementation detail)
    // The key is it continues parsing after errors
}
