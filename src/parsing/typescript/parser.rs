//! TypeScript parser implementation
//!
//! **Tree-sitter ABI Version**: ABI-14 (tree-sitter-typescript 0.24.4)
//!
//! Note: This parser uses ABI-14 with 383 node types and 40 fields.
//! When migrating or updating the parser, ensure compatibility with ABI-14 features.

use crate::indexing::Import;
use crate::parsing::{LanguageParser, MethodCall};
use crate::types::SymbolCounter;
use crate::{FileId, Range, Symbol, SymbolKind, Visibility};
use std::any::Any;
use tree_sitter::{Language, Node, Parser};

/// TypeScript language parser
pub struct TypeScriptParser {
    parser: Parser,
}

impl TypeScriptParser {
    /// Helper to create a symbol with all optional fields
    fn create_symbol(
        id: crate::types::SymbolId,
        name: String,
        kind: SymbolKind,
        file_id: FileId,
        range: Range,
        signature: Option<String>,
        doc_comment: Option<String>,
        module_path: &str,
        visibility: Visibility,
    ) -> Symbol {
        let mut symbol = Symbol::new(id, name, kind, file_id, range);

        if let Some(sig) = signature {
            symbol = symbol.with_signature(sig);
        }
        if let Some(doc) = doc_comment {
            symbol = symbol.with_doc(doc);
        }
        if !module_path.is_empty() {
            symbol = symbol.with_module_path(module_path);
        }
        symbol = symbol.with_visibility(visibility);

        symbol
    }

    /// Create a new TypeScript parser
    pub fn new() -> Result<Self, String> {
        let mut parser = Parser::new();
        let language: Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to set TypeScript language: {e}"))?;

        Ok(Self { parser })
    }

    /// Extract symbols from a TypeScript node
    fn extract_symbols_from_node(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        symbols: &mut Vec<Symbol>,
        module_path: &str,
    ) {
        match node.kind() {
            "function_declaration" => {
                if let Some(symbol) =
                    self.process_function(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                }
            }
            "class_declaration" | "abstract_class_declaration" => {
                if let Some(symbol) = self.process_class(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                    // Extract class members
                    self.extract_class_members(node, code, file_id, counter, symbols, module_path);
                }
            }
            "interface_declaration" => {
                if let Some(symbol) =
                    self.process_interface(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                }
            }
            "type_alias_declaration" => {
                if let Some(symbol) =
                    self.process_type_alias(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                }
            }
            "enum_declaration" => {
                if let Some(symbol) = self.process_enum(node, code, file_id, counter, module_path) {
                    symbols.push(symbol);
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                self.process_variable_declaration(
                    node,
                    code,
                    file_id,
                    counter,
                    symbols,
                    module_path,
                );
            }
            "arrow_function" => {
                // Handle arrow functions assigned to variables
                if let Some(symbol) =
                    self.process_arrow_function(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                }
            }
            _ => {}
        }

        // Recursively process children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_symbols_from_node(child, code, file_id, counter, symbols, module_path);
        }
    }

    /// Process a function declaration
    fn process_function(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = &code[name_node.byte_range()];

        let signature = self.extract_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(Self::create_symbol(
            counter.next_id(),
            name.to_string(),
            SymbolKind::Function,
            file_id,
            Range::new(
                node.start_position().row as u32,
                node.start_position().column as u16,
                node.end_position().row as u32,
                node.end_position().column as u16,
            ),
            Some(signature),
            doc_comment,
            module_path,
            visibility,
        ))
    }

    /// Process a class declaration
    fn process_class(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        // For abstract classes, the name isn't a field, it's a type_identifier child
        let name = node
            .children(&mut node.walk())
            .find(|n| n.kind() == "type_identifier")
            .map(|n| &code[n.byte_range()])?;

        let signature = self.extract_class_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(Self::create_symbol(
            counter.next_id(),
            name.to_string(),
            SymbolKind::Class,
            file_id,
            Range::new(
                node.start_position().row as u32,
                node.start_position().column as u16,
                node.end_position().row as u32,
                node.end_position().column as u16,
            ),
            Some(signature),
            doc_comment,
            module_path,
            visibility,
        ))
    }

    /// Extract class members (methods, properties)
    fn extract_class_members(
        &self,
        class_node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        symbols: &mut Vec<Symbol>,
        module_path: &str,
    ) {
        if let Some(body) = class_node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                match child.kind() {
                    "method_definition" => {
                        if let Some(symbol) =
                            self.process_method(child, code, file_id, counter, module_path)
                        {
                            symbols.push(symbol);
                        }
                    }
                    "public_field_definition" | "property_declaration" => {
                        if let Some(symbol) =
                            self.process_property(child, code, file_id, counter, module_path)
                        {
                            symbols.push(symbol);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Process an interface declaration
    fn process_interface(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = &code[name_node.byte_range()];

        let signature = self.extract_interface_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(Self::create_symbol(
            counter.next_id(),
            name.to_string(),
            SymbolKind::Interface,
            file_id,
            Range::new(
                node.start_position().row as u32,
                node.start_position().column as u16,
                node.end_position().row as u32,
                node.end_position().column as u16,
            ),
            Some(signature),
            doc_comment,
            module_path,
            visibility,
        ))
    }

    /// Process a type alias declaration
    fn process_type_alias(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = &code[name_node.byte_range()];

        let signature = &code[node.byte_range()];
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(Self::create_symbol(
            counter.next_id(),
            name.to_string(),
            SymbolKind::TypeAlias,
            file_id,
            Range::new(
                node.start_position().row as u32,
                node.start_position().column as u16,
                node.end_position().row as u32,
                node.end_position().column as u16,
            ),
            Some(signature.to_string()),
            doc_comment,
            module_path,
            visibility,
        ))
    }

    /// Process an enum declaration
    fn process_enum(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = &code[name_node.byte_range()];

        let signature = &code[node.byte_range()];
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(Self::create_symbol(
            counter.next_id(),
            name.to_string(),
            SymbolKind::Enum,
            file_id,
            Range::new(
                node.start_position().row as u32,
                node.start_position().column as u16,
                node.end_position().row as u32,
                node.end_position().column as u16,
            ),
            Some(signature.to_string()),
            doc_comment,
            module_path,
            visibility,
        ))
    }

    /// Process variable declarations
    fn process_variable_declaration(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        symbols: &mut Vec<Symbol>,
        module_path: &str,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if name_node.kind() == "identifier" {
                        let name = &code[name_node.byte_range()];
                        let kind = if code[node.byte_range()].starts_with("const") {
                            SymbolKind::Constant
                        } else {
                            SymbolKind::Variable
                        };

                        let visibility = self.determine_visibility(node, code);

                        symbols.push(Self::create_symbol(
                            counter.next_id(),
                            name.to_string(),
                            kind,
                            file_id,
                            Range::new(
                                child.start_position().row as u32,
                                child.start_position().column as u16,
                                child.end_position().row as u32,
                                child.end_position().column as u16,
                            ),
                            None,
                            None,
                            module_path,
                            visibility,
                        ));
                    }
                }
            }
        }
    }

    /// Process arrow functions
    fn process_arrow_function(
        &self,
        _node: Node,
        _code: &str,
        _file_id: FileId,
        _counter: &mut SymbolCounter,
        _module_path: &str,
    ) -> Option<Symbol> {
        // Arrow functions are typically anonymous
        // We'll handle named arrow functions when assigned to variables
        None
    }

    /// Process a method definition
    fn process_method(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = &code[name_node.byte_range()];

        let signature = self.extract_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_method_visibility(node, code);

        Some(Self::create_symbol(
            counter.next_id(),
            name.to_string(),
            SymbolKind::Method,
            file_id,
            Range::new(
                node.start_position().row as u32,
                node.start_position().column as u16,
                node.end_position().row as u32,
                node.end_position().column as u16,
            ),
            Some(signature),
            doc_comment,
            module_path,
            visibility,
        ))
    }

    /// Process a property/field definition
    fn process_property(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = &code[name_node.byte_range()];

        let visibility = self.determine_method_visibility(node, code);

        Some(Self::create_symbol(
            counter.next_id(),
            name.to_string(),
            SymbolKind::Field,
            file_id,
            Range::new(
                node.start_position().row as u32,
                node.start_position().column as u16,
                node.end_position().row as u32,
                node.end_position().column as u16,
            ),
            None,
            None,
            module_path,
            visibility,
        ))
    }

    /// Extract function/method signature
    fn extract_signature(&self, node: Node, code: &str) -> String {
        // Extract the signature without the body
        let start = node.start_byte();
        let mut end = node.end_byte();

        // Try to find the body and exclude it
        if let Some(body) = node.child_by_field_name("body") {
            end = body.start_byte();
        }

        code[start..end].trim().to_string()
    }

    /// Extract class signature (with extends/implements)
    fn extract_class_signature(&self, node: Node, code: &str) -> String {
        let start = node.start_byte();
        let mut end = node.end_byte();

        // Find the class body and exclude it
        if let Some(body) = node.child_by_field_name("body") {
            end = body.start_byte();
        }

        code[start..end].trim().to_string()
    }

    /// Extract interface signature
    fn extract_interface_signature(&self, node: Node, code: &str) -> String {
        let start = node.start_byte();
        let mut end = node.end_byte();

        // Find the interface body and exclude it
        if let Some(body) = node.child_by_field_name("body") {
            end = body.start_byte();
        }

        code[start..end].trim().to_string()
    }

    /// Determine visibility based on export keywords
    fn determine_visibility(&self, node: Node, code: &str) -> Visibility {
        // Check if preceded by export keyword
        if let Some(prev) = node.prev_sibling() {
            if prev.kind() == "export_statement" {
                return Visibility::Public;
            }
        }

        // Check parent for export
        if let Some(parent) = node.parent() {
            if parent.kind() == "export_statement" {
                return Visibility::Public;
            }
        }

        // Check the signature itself
        let signature = &code[node.byte_range()];
        if signature.starts_with("export ") {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }

    /// Determine method/property visibility
    fn determine_method_visibility(&self, node: Node, code: &str) -> Visibility {
        let signature = &code[node.byte_range()];

        if signature.contains("private ") || signature.starts_with("#") {
            Visibility::Private
        } else if signature.contains("protected ") {
            Visibility::Module // Map TypeScript protected to Module visibility
        } else {
            Visibility::Public // Default for class members
        }
    }

    /// Find implementations (extends and implements) in TypeScript
    fn find_implementations_in_node<'a>(
        &self,
        node: Node,
        code: &'a str,
        implementations: &mut Vec<(&'a str, &'a str, Range)>,
        extends_only: bool,
    ) {
        match node.kind() {
            "class_declaration" | "abstract_class_declaration" => {
                // Get class name first
                let class_name = node
                    .children(&mut node.walk())
                    .find(|n| n.kind() == "type_identifier")
                    .map(|n| &code[n.byte_range()]);

                if let Some(class_name) = class_name {
                    // Look for class_heritage child node (not a field!)
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "class_heritage" {
                            self.process_heritage_clauses(
                                child,
                                code,
                                class_name,
                                implementations,
                                extends_only,
                            );
                        }
                    }
                }
            }
            "interface_declaration" => {
                // Only process interface extends if we're looking for extends relationships
                if extends_only {
                    if let Some(interface_name_node) = node.child_by_field_name("name") {
                        let interface_name = &code[interface_name_node.byte_range()];

                        // Check for extends clause (interface extension)
                        if let Some(extends_clause) = node.child_by_field_name("extends") {
                            self.process_extends_clause(
                                extends_clause,
                                code,
                                interface_name,
                                implementations,
                            );
                        }
                    }
                }
            }
            _ => {}
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.find_implementations_in_node(child, code, implementations, extends_only);
        }
    }

    /// Process heritage clauses (extends and implements)
    fn process_heritage_clauses<'a>(
        &self,
        heritage_node: Node,
        code: &'a str,
        class_name: &'a str,
        implementations: &mut Vec<(&'a str, &'a str, Range)>,
        extends_only: bool,
    ) {
        let mut cursor = heritage_node.walk();
        for child in heritage_node.children(&mut cursor) {
            match child.kind() {
                "extends_clause" => {
                    // Process extends clause for both extends_only and regular find_implementations
                    // Skip "extends" keyword, get the type
                    let mut extends_cursor = child.walk();
                    for extends_child in child.children(&mut extends_cursor) {
                        if extends_child.kind() == "type_identifier"
                            || extends_child.kind() == "identifier"
                            || extends_child.kind() == "nested_type_identifier"
                            || extends_child.kind() == "generic_type"
                        {
                            if let Some(base_name) = self.extract_type_name(extends_child, code) {
                                let range = Range::new(
                                    extends_child.start_position().row as u32,
                                    extends_child.start_position().column as u16,
                                    extends_child.end_position().row as u32,
                                    extends_child.end_position().column as u16,
                                );
                                implementations.push((class_name, base_name, range));
                            }
                        }
                    }
                }
                "implements_clause" => {
                    // Only process implements clause when NOT looking for extends only
                    if !extends_only {
                        // Skip "implements" keyword, get all the interfaces
                        let mut impl_cursor = child.walk();
                        for impl_child in child.children(&mut impl_cursor) {
                            if impl_child.kind() == "type_identifier"
                                || impl_child.kind() == "identifier"
                                || impl_child.kind() == "nested_type_identifier"
                                || impl_child.kind() == "generic_type"
                            {
                                if let Some(interface_name) =
                                    self.extract_type_name(impl_child, code)
                                {
                                    let range = Range::new(
                                        impl_child.start_position().row as u32,
                                        impl_child.start_position().column as u16,
                                        impl_child.end_position().row as u32,
                                        impl_child.end_position().column as u16,
                                    );
                                    implementations.push((class_name, interface_name, range));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Process extends clause for interfaces
    fn process_extends_clause<'a>(
        &self,
        extends_node: Node,
        code: &'a str,
        interface_name: &'a str,
        implementations: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let mut cursor = extends_node.walk();
        for child in extends_node.children(&mut cursor) {
            if child.kind() == "type_identifier" || child.kind() == "nested_type_identifier" {
                if let Some(base_interface) = self.extract_type_name(child, code) {
                    let range = Range::new(
                        child.start_position().row as u32,
                        child.start_position().column as u16,
                        child.end_position().row as u32,
                        child.end_position().column as u16,
                    );
                    implementations.push((interface_name, base_interface, range));
                }
            }
        }
    }

    /// Extract type name from a type node
    #[allow(clippy::only_used_in_recursion)]
    fn extract_type_name<'a>(&self, node: Node, code: &'a str) -> Option<&'a str> {
        match node.kind() {
            "type_identifier" | "identifier" => Some(&code[node.byte_range()]),
            "nested_type_identifier" => {
                // For qualified names like Namespace.Type, get the full name
                Some(&code[node.byte_range()])
            }
            "generic_type" => {
                // For generic types like Array<T>, get just the base type
                if let Some(name_node) = node.child_by_field_name("name") {
                    self.extract_type_name(name_node, code)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl LanguageParser for TypeScriptParser {
    fn parse(
        &mut self,
        code: &str,
        file_id: FileId,
        symbol_counter: &mut SymbolCounter,
    ) -> Vec<Symbol> {
        let mut symbols = Vec::new();

        match self.parser.parse(code, None) {
            Some(tree) => {
                let root_node = tree.root_node();
                self.extract_symbols_from_node(
                    root_node,
                    code,
                    file_id,
                    symbol_counter,
                    &mut symbols,
                    "", // Module path will be determined by behavior
                );
            }
            None => {
                eprintln!("Failed to parse TypeScript file");
            }
        }

        symbols
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn extract_doc_comment(&self, node: &Node, code: &str) -> Option<String> {
        // Look for JSDoc/TSDoc comments (/** ... */)
        if let Some(prev) = node.prev_sibling() {
            if prev.kind() == "comment" {
                let comment = &code[prev.byte_range()];
                if comment.starts_with("/**") {
                    // Clean up the comment
                    let cleaned = comment
                        .trim_start_matches("/**")
                        .trim_end_matches("*/")
                        .lines()
                        .map(|line| line.trim_start_matches(" * ").trim_start_matches(" *"))
                        .collect::<Vec<_>>()
                        .join("\n")
                        .trim()
                        .to_string();

                    return Some(cleaned);
                }
            }
        }
        None
    }

    fn find_calls<'a>(&mut self, _code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        // TODO: Implement call extraction
        Vec::new()
    }

    fn find_method_calls(&mut self, _code: &str) -> Vec<MethodCall> {
        // TODO: Implement method call extraction
        Vec::new()
    }

    fn find_implementations<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let mut implementations = Vec::new();

        if let Some(tree) = self.parser.parse(code, None) {
            self.find_implementations_in_node(tree.root_node(), code, &mut implementations, false);
        }

        implementations
    }

    fn find_extends<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let mut extends = Vec::new();

        if let Some(tree) = self.parser.parse(code, None) {
            self.find_implementations_in_node(tree.root_node(), code, &mut extends, true);
        }

        extends
    }

    fn find_imports(&mut self, _code: &str, _file_id: FileId) -> Vec<Import> {
        // TODO: Implement import extraction
        Vec::new()
    }

    fn find_uses<'a>(&mut self, _code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        // TODO: Implement type usage extraction
        Vec::new()
    }

    fn find_defines<'a>(&mut self, _code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        // TODO: Implement method definition extraction
        Vec::new()
    }

    fn language(&self) -> crate::parsing::Language {
        crate::parsing::Language::TypeScript
    }
}
