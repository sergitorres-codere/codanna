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

        println!("\nPython Language Metadata:");
        println!("  ABI Version: {:?}", language.abi_version());
        println!("  Node kind count: {}", language.node_kind_count());

        // Check for specific Python constructs
        let class_id = language.id_for_node_kind("class_definition", true);
        println!("  Class definition ID: {class_id:?}");
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
        ] {
            let id = language.id_for_node_kind(node_kind, true);
            if id != 0 {
                println!("    {node_kind} -> ID: {id}");
            }
        }
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
