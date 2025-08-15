//! Tree-sitter ABI-15 feature exploration and documentation
//!
//! This test file explores available ABI-15 features that could enhance
//! our LanguageBehavior trait implementation in Stage 2 of the refactoring.
//!
//! Run with: cargo test abi15_exploration --nocapture
//!
//! Key findings will be used to inform the design of language-specific
//! behavior abstractions.

#[cfg(test)]
mod abi15_tests {
    use tree_sitter::Language;

    #[test]
    fn explore_rust_abi15_features() {
        let language: Language = tree_sitter_rust::LANGUAGE.into();

        println!("=== Rust Language ABI-15 Metadata ===");
        println!("  ABI Version: {}", language.abi_version());
        println!("  Field count: {}", language.field_count());
        println!("  Node kind count: {}", language.node_kind_count());

        // Explore node types that could inform LanguageBehavior
        println!("\n  Key Node Types for Symbol Extraction:");
        for node_kind in &[
            "function_item",
            "impl_item",
            "struct_item",
            "trait_item",
            "mod_item",
            "enum_item",
            "type_alias",
            "type_item",
            "const_item",
            "static_item",
            "macro_definition",
            "macro_rules",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind} -> ID: {id}");
            }
        }

        // Check field names (useful for extracting specific parts)
        println!("\n  Available Fields: {}", language.field_count());
        for i in 0..5.min(language.field_count()) {
            if let Some(name) = language.field_name_for_id(i as u16) {
                println!("    Field {i}: {name}");
            }
        }

        // TODO: Explore supertype information when API is clearer
        // TODO: Check for reserved word functionality
    }

    #[test]
    fn explore_python_abi15_features() {
        let language: Language = tree_sitter_python::LANGUAGE.into();

        println!("\n=== Python Language ABI-15 Metadata ===");
        println!("  ABI Version: {}", language.abi_version());
        println!("  Field count: {}", language.field_count());
        println!("  Node kind count: {}", language.node_kind_count());

        // Explore node types for symbol extraction
        println!("\n  Key Node Types for Symbol Extraction:");
        for node_kind in &[
            "function_definition",
            "class_definition",
            "assignment",
            "expression_statement",
            "annotated_assignment",
            "type_alias_statement",
            "decorator",
            "decorated_definition",
            "global_statement",
            "identifier",
            "module",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind} -> ID: {id}");
            }
        }
    }

    #[test]
    fn explore_typescript_abi15_comprehensive() {
        let language: Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();

        println!("=== TypeScript Language ABI-15 COMPREHENSIVE NODE MAPPING ===");
        println!("  ABI Version: {}", language.abi_version());
        println!("  Node kind count: {}", language.node_kind_count());

        println!("\n=== CLASS-RELATED NODES ===");
        for node_kind in &[
            "class",
            "class_declaration",
            "abstract_class_declaration",
            "class_body",
            "class_heritage",
            "extends_clause",
            "implements_clause",
            "method_definition",
            "public_field_definition",
            "private_field_definition",
            "property_declaration",
            "constructor",
            "abstract",
            "abstract_method_signature",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind:30} -> ID: {id}");
            }
        }

        println!("\n=== INTERFACE-RELATED NODES ===");
        for node_kind in &[
            "interface",
            "interface_declaration",
            "interface_body",
            "property_signature",
            "method_signature",
            "index_signature",
            "extends_type_clause",
            "extends_clause",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind:30} -> ID: {id}");
            }
        }

        println!("\n=== TYPE-RELATED NODES ===");
        for node_kind in &[
            "type_alias_declaration",
            "type_annotation",
            "type_identifier",
            "type_parameter",
            "type_parameters",
            "type_arguments",
            "generic_type",
            "union_type",
            "intersection_type",
            "conditional_type",
            "literal_type",
            "template_literal_type",
            "nested_type_identifier",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind:30} -> ID: {id}");
            }
        }

        println!("\n=== ENUM-RELATED NODES ===");
        for node_kind in &[
            "enum",
            "enum_declaration",
            "enum_body",
            "enum_assignment",
            "enum_member",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind:30} -> ID: {id}");
            }
        }

        println!("\n=== FUNCTION-RELATED NODES ===");
        for node_kind in &[
            "function",
            "function_declaration",
            "function_expression",
            "arrow_function",
            "generator_function",
            "generator_function_declaration",
            "async_function",
            "async_arrow_function",
            "method_definition",
            "formal_parameters",
            "required_parameter",
            "optional_parameter",
            "rest_parameter",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind:30} -> ID: {id}");
            }
        }

        println!("\n=== VARIABLE/CONST NODES ===");
        for node_kind in &[
            "variable_declaration",
            "variable_declarator",
            "lexical_declaration",
            "const",
            "let",
            "var",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind:30} -> ID: {id}");
            }
        }

        println!("\n=== IMPORT/EXPORT NODES ===");
        for node_kind in &[
            "import_statement",
            "import_clause",
            "named_imports",
            "namespace_import",
            "import_specifier",
            "export_statement",
            "export_clause",
            "export_specifier",
            "export_default",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind:30} -> ID: {id}");
            }
        }

        println!("\n=== MODULE/NAMESPACE NODES ===");
        for node_kind in &[
            "module",
            "internal_module",
            "module_declaration",
            "namespace_declaration",
            "ambient_declaration",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind:30} -> ID: {id}");
            }
        }

        println!("\n=== DECORATOR NODES ===");
        for node_kind in &[
            "decorator",
            "decorator_member_expression",
            "decorator_call_expression",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind:30} -> ID: {id}");
            }
        }

        println!("\n=== IMPORTANT: Use these exact node names in parser implementation!");
        println!("=== DO NOT GUESS node names - always verify with this test first!");
    }

    #[test]
    fn explore_php_abi15_features() {
        let language: Language = tree_sitter_php::LANGUAGE_PHP.into();

        println!("\n=== PHP Language ABI-15 Metadata ===");
        println!("  ABI Version: {}", language.abi_version());
        println!("  Node kind count: {}", language.node_kind_count());

        println!("\n  Key Node Types:");
        for node_kind in &[
            "class_declaration",
            "function_definition",
            "method_declaration",
            "interface_declaration",
            "const_declaration",
            "const_element",
            "function_call_expression",
            "assignment_expression",
            "expression_statement",
            "namespace_definition",
            "enum_declaration",
            "global_declaration",
            "simple_parameter",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind} -> ID: {id}");
            }
        }
    }

    #[test]
    fn explore_typescript_abi15_features() {
        let language: Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();

        println!("\n=== TypeScript Language ABI-15 Metadata ===");
        println!("  ABI Version: {}", language.abi_version());
        println!("  Field count: {}", language.field_count());
        println!("  Node kind count: {}", language.node_kind_count());

        // Explore TypeScript-specific node types for symbol extraction
        println!("\n  Key Node Types for Symbol Extraction:");
        for node_kind in &[
            // Function-related
            "function_declaration",
            "function_expression",
            "arrow_function",
            "generator_function_declaration",
            "method_definition",
            "function_signature",
            // Class-related
            "class_declaration",
            "class_expression",
            "constructor",
            "property_declaration",
            "method_signature",
            "public_field_definition",
            "private_field_definition",
            // Interface & Type
            "interface_declaration",
            "type_alias_declaration",
            "enum_declaration",
            "type_parameter",
            "type_annotation",
            // Variables
            "variable_declaration",
            "lexical_declaration",
            "const_declaration",
            "let_declaration",
            "variable_declarator",
            // Module/Namespace
            "module_declaration",
            "namespace_declaration",
            "export_statement",
            "import_statement",
            "ambient_declaration",
            // Decorators
            "decorator",
            "decorator_expression",
            // JSX/TSX
            "jsx_element",
            "jsx_self_closing_element",
            "jsx_opening_element",
            "jsx_attribute",
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind} -> ID: {id}");
            }
        }

        // Check field names for TypeScript-specific constructs
        println!("\n  Available Fields: {}", language.field_count());
        for i in 0..10.min(language.field_count()) {
            if let Some(name) = language.field_name_for_id(i as u16) {
                println!("    Field {i}: {name}");
            }
        }

        // Test TypeScript vs JavaScript differences
        let js_language: Language = tree_sitter_javascript::LANGUAGE.into();
        println!("\n  TypeScript vs JavaScript Comparison:");
        println!("    TypeScript nodes: {}", language.node_kind_count());
        println!("    JavaScript nodes: {}", js_language.node_kind_count());
        println!(
            "    Difference: {} additional nodes",
            language.node_kind_count() as i32 - js_language.node_kind_count() as i32
        );
    }

    #[test]
    fn explore_language_behavior_candidates() {
        println!("\n=== Potential LanguageBehavior Enhancements ===");

        // Compare capabilities across languages
        let rust_lang: Language = tree_sitter_rust::LANGUAGE.into();
        let python_lang: Language = tree_sitter_python::LANGUAGE.into();
        let php_lang: Language = tree_sitter_php::LANGUAGE_PHP.into();

        println!("\n  Cross-Language Comparison:");
        println!("  Language    | ABI | Node Kinds | Fields");
        println!("  ------------|-----|------------|-------");
        println!(
            "  Rust        | {:3} | {:10} | {:6}",
            rust_lang.abi_version(),
            rust_lang.node_kind_count(),
            rust_lang.field_count()
        );
        println!(
            "  Python      | {:3} | {:10} | {:6}",
            python_lang.abi_version(),
            python_lang.node_kind_count(),
            python_lang.field_count()
        );
        println!(
            "  PHP         | {:3} | {:10} | {:6}",
            php_lang.abi_version(),
            php_lang.node_kind_count(),
            php_lang.field_count()
        );

        // Test common node type mapping
        println!("\n  Common Symbol Types Across Languages:");
        let common_concepts = vec![
            (
                "Function",
                vec![
                    "function_item",
                    "function_definition",
                    "function_definition",
                ],
            ),
            (
                "Class",
                vec!["struct_item", "class_definition", "class_declaration"],
            ),
            (
                "Method",
                vec!["function_item", "function_definition", "method_declaration"],
            ),
        ];

        for (concept, node_kinds) in common_concepts {
            println!("    {concept}:");
            println!(
                "      Rust:   {} (ID: {})",
                node_kinds[0],
                rust_lang.id_for_node_kind(node_kinds[0], true)
            );
            println!(
                "      Python: {} (ID: {})",
                node_kinds[1],
                python_lang.id_for_node_kind(node_kinds[1], true)
            );
            println!(
                "      PHP:    {} (ID: {})",
                node_kinds[2],
                php_lang.id_for_node_kind(node_kinds[2], true)
            );
        }

        println!("\n  Implications for LanguageBehavior:");
        println!("  - Each language has different node naming conventions");
        println!("  - ABI-15 provides consistent metadata access");
        println!("  - Can validate node types at behavior construction");
        println!("  - Field information could enhance symbol extraction");
    }
}
