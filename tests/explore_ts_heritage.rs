use tree_sitter::{Language, Node, Parser};

#[allow(dead_code)]
#[allow(clippy::only_used_in_recursion)]
fn explore_node(node: Node, code: &str, indent: usize) {
    let indent_str = "  ".repeat(indent);
    println!("{}{}:", indent_str, node.kind());

    // Show field names if any
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(field_name) = node.field_name_for_child(child.id() as u32) {
            println!("{}  field '{}' -> {}", indent_str, field_name, child.kind());
        }
    }

    // Recurse for specific nodes
    if node.kind() == "class_declaration"
        || node.kind() == "class_heritage"
        || node.kind().contains("implements")
        || node.kind().contains("extends")
        || node.kind().contains("heritage")
    {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            explore_node(child, code, indent + 1);
        }
    }
}

#[test]
fn explore_typescript_heritage() {
    let mut parser = Parser::new();
    let language: Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    parser.set_language(&language).unwrap();

    let code = "class TestClass extends Base implements ITest, IAnother { }";

    if let Some(tree) = parser.parse(code, None) {
        println!("\nExploring TypeScript class heritage structure:");
        println!("Code: {code}");
        println!();

        let root = tree.root_node();
        let mut cursor = root.walk();

        // Find the class_declaration node
        for child in root.children(&mut cursor) {
            if child.kind() == "class_declaration" {
                println!("Found class_declaration");

                // Check all children and their field names
                let mut class_cursor = child.walk();
                for class_child in child.children(&mut class_cursor) {
                    if let Some(field_name) = child.field_name_for_child(class_child.id() as u32) {
                        println!(
                            "  Field '{}' -> {} = '{}'",
                            field_name,
                            class_child.kind(),
                            &code[class_child.byte_range()]
                        );
                    } else {
                        println!(
                            "  Child: {} = '{}'",
                            class_child.kind(),
                            &code[class_child.byte_range()]
                        );
                    }

                    // Explore class_heritage structure
                    if class_child.kind() == "class_heritage" {
                        println!("    Exploring class_heritage:");
                        let mut heritage_cursor = class_child.walk();
                        for heritage_child in class_child.children(&mut heritage_cursor) {
                            println!(
                                "      {} = '{}'",
                                heritage_child.kind(),
                                &code[heritage_child.byte_range()]
                            );
                        }
                    }
                }
            }
        }
    }
}
