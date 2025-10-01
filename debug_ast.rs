use tree_sitter::Parser;

fn print_tree(node: tree_sitter::Node, code: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let text = &code[node.byte_range()];
    let preview = if text.len() > 40 {
        format!("{}...", &text[..40])
    } else {
        text.to_string()
    };
    println!("{}{}  '{}'", indent, node.kind(), preview.replace('\n', "\\n"));

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(child, code, depth + 1);
    }
}

fn main() {
    let code = r#"using System;

namespace Test
{
    public class Program
    {
        public static void Main()
        {
            var helper = new Helper();
            helper.DoWork();
        }
    }
}"#;

    let mut parser = Parser::new();
    let language = tree_sitter_c_sharp::LANGUAGE;
    parser.set_language(&language.into()).unwrap();

    let tree = parser.parse(code, None).unwrap();
    print_tree(tree.root_node(), code, 0);
}