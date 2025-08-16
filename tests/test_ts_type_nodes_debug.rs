//! Debug test to explore TypeScript type nodes

use tree_sitter::{Language, Parser};

#[test]
fn explore_typescript_type_nodes() {
    println!("\n=== TypeScript Type Node Exploration ===\n");

    let code = r#"
function processUser(user: User): Result<User> {
    return { success: true };
}

class UserService implements IService {
    private client: HttpClient;
}
"#;

    let language: Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    let mut parser = Parser::new();
    parser.set_language(&language).unwrap();

    let tree = parser.parse(code, None).unwrap();
    let root = tree.root_node();

    // Walk tree and look for interesting nodes
    explore_node(&root, code, 0);
}

fn explore_node(node: &tree_sitter::Node, code: &str, depth: usize) {
    let indent = "  ".repeat(depth);

    // Look for type-related nodes
    if matches!(
        node.kind(),
        "function_declaration"
            | "formal_parameters"
            | "required_parameter"
            | "type_annotation"
            | "generic_type"
            | "type_identifier"
            | "class_declaration"
            | "implements_clause"
            | "heritage_clause"
            | "public_field_definition"
            | "return_type"
    ) {
        println!(
            "{}{} [{}:{}]",
            indent,
            node.kind(),
            node.start_position().row + 1,
            node.start_position().column
        );

        // Show all fields for function_declaration
        if node.kind() == "function_declaration" {
            for i in 0..10 {
                if let Some(child) = node.child(i) {
                    if let Some(field_name) = node.field_name_for_child(child.id() as u32) {
                        let text = &code[child.byte_range()];
                        let truncated = if text.len() > 30 {
                            format!("{}...", &text[..30])
                        } else {
                            text.to_string()
                        };
                        println!(
                            "{}  field '{}' -> {} = {:?}",
                            indent,
                            field_name,
                            child.kind(),
                            truncated
                        );
                    }
                } else {
                    break;
                }
            }
        }
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        explore_node(&child, code, depth + 1);
    }
}
