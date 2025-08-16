//! TypeScript parser implementation
//!
//! **Tree-sitter ABI Version**: ABI-14 (tree-sitter-typescript 0.24.4)
//!
//! Note: This parser uses ABI-14 with 383 node types and 40 fields.
//! When migrating or updating the parser, ensure compatibility with ABI-14 features.

use crate::indexing::Import;
use crate::parsing::{LanguageParser, MethodCall, ParserContext, ScopeType};
use crate::types::SymbolCounter;
use crate::{FileId, Range, Symbol, SymbolKind, Visibility};
use std::any::Any;
use tree_sitter::{Language, Node, Parser};

/// TypeScript language parser
pub struct TypeScriptParser {
    parser: Parser,
    context: ParserContext,
}

impl TypeScriptParser {
    /// Helper to create a symbol with all optional fields
    fn create_symbol(
        &self,
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

        // Set scope context based on parser's current scope
        symbol.scope_context = Some(self.context.current_scope_context());

        symbol
    }

    /// Create a new TypeScript parser
    pub fn new() -> Result<Self, String> {
        let mut parser = Parser::new();
        let language: Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to set TypeScript language: {e}"))?;

        Ok(Self {
            parser,
            context: ParserContext::new(),
        })
    }

    /// Extract symbols from a TypeScript node
    fn extract_symbols_from_node(
        &mut self,
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
                // Note: In TypeScript, function declarations are hoisted
                // But we process nested symbols in the function's scope
                self.context.enter_scope(ScopeType::hoisting_function());
                // Process children for nested functions/classes
                for child in node.children(&mut node.walk()) {
                    if child.kind() != "identifier" && child.kind() != "formal_parameters" {
                        self.extract_symbols_from_node(
                            child,
                            code,
                            file_id,
                            counter,
                            symbols,
                            module_path,
                        );
                    }
                }
                self.context.exit_scope();
            }
            "class_declaration" | "abstract_class_declaration" => {
                if let Some(symbol) = self.process_class(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                    // Enter class scope for processing members
                    self.context.enter_scope(ScopeType::Class);
                    // Extract class members
                    self.extract_class_members(node, code, file_id, counter, symbols, module_path);
                    self.context.exit_scope();
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
            _ => {
                // For unhandled node types, recursively process children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_symbols_from_node(
                        child,
                        code,
                        file_id,
                        counter,
                        symbols,
                        module_path,
                    );
                }
            }
        }
    }

    /// Process a function declaration
    fn process_function(
        &mut self,
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

        Some(self.create_symbol(
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
        &mut self,
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

        Some(self.create_symbol(
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
        &mut self,
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
                        // Also process the method body for nested classes/functions
                        if let Some(body) = child.child_by_field_name("body") {
                            for body_child in body.children(&mut body.walk()) {
                                self.extract_symbols_from_node(
                                    body_child,
                                    code,
                                    file_id,
                                    counter,
                                    symbols,
                                    module_path,
                                );
                            }
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
        &mut self,
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

        Some(self.create_symbol(
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
        &mut self,
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

        Some(self.create_symbol(
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
        &mut self,
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

        Some(self.create_symbol(
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
        &mut self,
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

                        symbols.push(self.create_symbol(
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
        &mut self,
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
        &mut self,
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

        Some(self.create_symbol(
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
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = &code[name_node.byte_range()];

        let visibility = self.determine_method_visibility(node, code);

        Some(self.create_symbol(
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

    /// Extract imports from AST node recursively
    fn extract_imports_from_node(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        imports: &mut Vec<Import>,
    ) {
        match node.kind() {
            "import_statement" => {
                self.process_import_statement(node, code, file_id, imports);
            }
            "export_statement" => {
                // Check if it's a re-export (has source)
                if node.child_by_field_name("source").is_some() {
                    self.process_export_statement(node, code, file_id, imports);
                }
            }
            _ => {
                // Recurse into children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_imports_from_node(child, code, file_id, imports);
                }
            }
        }
    }

    /// Process an import statement node
    fn process_import_statement(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        imports: &mut Vec<Import>,
    ) {
        eprintln!(
            "ENTERING process_import_statement, code: {}",
            &code[node.byte_range()]
        );

        // Debug: print all children
        let mut cursor = node.walk();
        eprintln!("  Node has {} children:", node.child_count());

        // Check if this is a type-only import (has 'type' keyword after 'import')
        let mut is_type_only = false;
        for (i, child) in node.children(&mut cursor).enumerate() {
            eprintln!(
                "    child[{}]: kind='{}', field_name={:?}",
                i,
                child.kind(),
                node.field_name_for_child(i as u32)
            );
            // Check for 'type' keyword (appears in type-only imports)
            if child.kind() == "type" && i == 1 {
                is_type_only = true;
                eprintln!("    Detected type-only import!");
            }
        }

        // Get the source (the module being imported from)
        let source_node = match node.child_by_field_name("source") {
            Some(n) => n,
            None => return,
        };

        let source_path = &code[source_node.byte_range()];
        let source_path = source_path.trim_matches(|c| c == '"' || c == '\'' || c == '`');

        // Process import clause (what's being imported)
        // Note: import_clause is not a named field, we need to find it by kind
        let import_clause = {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find(|c| c.kind() == "import_clause")
        };

        if let Some(import_clause) = import_clause {
            eprintln!(
                "  Found import_clause: {}",
                &code[import_clause.byte_range()]
            );

            // Check for different import types
            let mut has_default = false;
            let mut has_named = false;
            let mut has_namespace = false;
            let mut default_name = None;
            let mut namespace_name = None;

            let mut cursor = import_clause.walk();
            for child in import_clause.children(&mut cursor) {
                eprintln!(
                    "    Child kind: {}, text: {}",
                    child.kind(),
                    &code[child.byte_range()]
                );
                match child.kind() {
                    "identifier" => {
                        // Default import
                        has_default = true;
                        let name = code[child.byte_range()].to_string();
                        eprintln!("      Setting default_name = {name}");
                        default_name = Some(name);
                    }
                    "named_imports" => {
                        // Named imports exist
                        has_named = true;
                    }
                    "namespace_import" => {
                        // * as name
                        has_namespace = true;
                        let mut ns_cursor = child.walk();
                        let children: Vec<_> = child.children(&mut ns_cursor).collect();
                        if let Some(identifier) =
                            children.iter().rev().find(|n| n.kind() == "identifier")
                        {
                            namespace_name = Some(code[identifier.byte_range()].to_string());
                        }
                    }
                    _ => {}
                }
            }

            // Add imports based on what we found
            // Following Rust pattern: one Import per module, with alias for default/namespace
            eprintln!(
                "  Summary: has_default={has_default}, has_named={has_named}, has_namespace={has_namespace}"
            );
            eprintln!("  default_name={default_name:?}, namespace_name={namespace_name:?}");

            if has_namespace {
                // Namespace import: import * as utils from './utils'
                imports.push(Import {
                    path: source_path.to_string(),
                    alias: namespace_name,
                    file_id,
                    is_glob: true,
                    is_type_only,
                });
            } else if has_default && has_named {
                // Mixed import: import React, { Component } from 'react'
                // We create one import with the default as alias
                imports.push(Import {
                    path: source_path.to_string(),
                    alias: default_name,
                    file_id,
                    is_glob: false,
                    is_type_only,
                });
            } else if has_default {
                // Default only: import React from 'react'
                eprintln!(
                    "  Adding default import: path='{source_path}', alias={default_name:?}, type_only={is_type_only}"
                );
                imports.push(Import {
                    path: source_path.to_string(),
                    alias: default_name,
                    file_id,
                    is_glob: false,
                    is_type_only,
                });
            } else if has_named {
                // Named only: import { Component } from 'react'
                imports.push(Import {
                    path: source_path.to_string(),
                    alias: None,
                    file_id,
                    is_glob: false,
                    is_type_only,
                });
            }
        } else {
            // Side-effect import (no import clause)
            imports.push(Import {
                path: source_path.to_string(),
                alias: None,
                file_id,
                is_glob: false,
                is_type_only: false, // Side-effect imports are never type-only
            });
        }
    }

    /// Process export statements (for re-exports)
    fn process_export_statement(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        imports: &mut Vec<Import>,
    ) {
        // Get the source module
        let source_node = match node.child_by_field_name("source") {
            Some(n) => n,
            None => return,
        };

        let source_path = &code[source_node.byte_range()];
        let source_path = source_path.trim_matches(|c| c == '"' || c == '\'' || c == '`');

        // Check what's being exported
        let node_text = &code[node.byte_range()];
        if node_text.contains("* from") {
            // export * from './module'
            imports.push(Import {
                path: source_path.to_string(),
                alias: None,
                file_id,
                is_glob: true,
                is_type_only: false, // Re-exports are not type-only
            });
        } else {
            // Named re-exports - just track the module being imported from
            imports.push(Import {
                path: source_path.to_string(),
                alias: None,
                file_id,
                is_glob: false,
                is_type_only: false, // Re-exports are not type-only
            });
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
        // Reset context for each file
        self.context = ParserContext::new();
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

    fn find_imports(&mut self, code: &str, file_id: FileId) -> Vec<Import> {
        let mut imports = Vec::new();

        if let Some(tree) = self.parser.parse(code, None) {
            let root = tree.root_node();
            self.extract_imports_from_node(root, code, file_id, &mut imports);
        }

        imports
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileId;

    #[test]
    fn test_typescript_import_extraction() {
        println!("\n=== TypeScript Import Extraction Test ===\n");

        let mut parser = TypeScriptParser::new().unwrap();
        let file_id = FileId::new(1).unwrap();

        let code = r#"
import { Component, useState } from 'react';
import React from 'react';
import * as utils from './utils';
import type { Props } from './types';
import './styles.css';
export { Button } from './Button';
export * from './common';
"#;

        println!("Test code:\n{code}");

        let imports = parser.find_imports(code, file_id);

        println!("\nExtracted {} imports:", imports.len());
        for (i, import) in imports.iter().enumerate() {
            println!(
                "  {}. {} -> {:?} (glob: {})",
                i + 1,
                import.path,
                import.alias,
                import.is_glob
            );
        }

        // Verify counts
        assert_eq!(imports.len(), 7, "Should extract 7 imports");

        // Verify specific imports
        // Named import creates one import with no alias
        assert!(
            imports
                .iter()
                .any(|i| i.path == "react" && i.alias.is_none())
        );
        // Default import has alias
        assert!(
            imports
                .iter()
                .any(|i| i.path == "react" && i.alias == Some("React".to_string()))
        );
        // Namespace import has alias and is_glob
        assert!(
            imports
                .iter()
                .any(|i| i.path == "./utils" && i.alias == Some("utils".to_string()) && i.is_glob)
        );
        // Type import (named)
        assert!(
            imports
                .iter()
                .any(|i| i.path == "./types" && i.alias.is_none())
        );
        // Side-effect import
        assert!(
            imports
                .iter()
                .any(|i| i.path == "./styles.css" && i.alias.is_none())
        );
        // Re-export
        assert!(imports.iter().any(|i| i.path == "./Button"));
        // Re-export all
        assert!(imports.iter().any(|i| i.path == "./common" && i.is_glob));

        println!("\n✅ Import extraction test passed");
    }

    #[test]
    fn test_complex_import_patterns() {
        println!("\n=== Complex Import Patterns Test ===\n");

        let mut parser = TypeScriptParser::new().unwrap();
        let file_id = FileId::new(1).unwrap();

        let code = r#"
// Mixed default and named
import React, { Component, useState as useStateHook } from 'react';

// Type mixed with value imports
import { type Config, createConfig } from './config';

// Aliased imports
import { Helper as H } from './helper';

// Re-export with rename
export { default as MyButton } from './Button';
"#;

        let imports = parser.find_imports(code, file_id);

        println!("Found {} imports in complex patterns", imports.len());
        for import in &imports {
            println!(
                "  - {} -> {:?} (glob: {})",
                import.path, import.alias, import.is_glob
            );
        }

        // Should have 4 imports: react (default), ./config, ./helper, ./Button
        assert_eq!(imports.len(), 4, "Should have 4 imports");

        // Check for React default import
        let react_default = imports
            .iter()
            .find(|i| i.path == "react" && i.alias == Some("React".to_string()));
        assert!(react_default.is_some(), "Should find React default import");

        // Check for config import (named imports, no alias)
        let config = imports
            .iter()
            .find(|i| i.path == "./config" && i.alias.is_none());
        assert!(config.is_some(), "Should find config import");

        // Check for helper import (named imports, no alias on Import struct)
        let helper = imports
            .iter()
            .find(|i| i.path == "./helper" && i.alias.is_none());
        assert!(helper.is_some(), "Should find helper import");

        println!("✅ Complex patterns handled correctly");
    }

    #[test]
    fn test_import_path_formats() {
        println!("\n=== Import Path Formats Test ===\n");

        let mut parser = TypeScriptParser::new().unwrap();
        let file_id = FileId::new(1).unwrap();

        let code = r#"
// Relative paths
import { a } from './sibling';
import { b } from '../parent';
import { c } from '../../grandparent';

// Node modules
import express from 'express';
import { Request } from '@types/express';

// Path aliases (would need tsconfig to resolve)
import { service } from '@app/services';

// Index imports
import utils from './utils';  // implies ./utils/index
"#;

        let imports = parser.find_imports(code, file_id);

        println!("Path formats found:");
        for import in &imports {
            println!("  - {}", import.path);
        }

        assert!(imports.iter().any(|i| i.path == "./sibling"));
        assert!(imports.iter().any(|i| i.path == "../parent"));
        assert!(imports.iter().any(|i| i.path == "express"));
        assert!(imports.iter().any(|i| i.path.starts_with("@")));

        println!("✅ Various path formats extracted correctly");
    }

    #[test]
    fn test_export_variations() {
        println!("\n=== Export Variations Test ===\n");

        let mut parser = TypeScriptParser::new().unwrap();
        let file_id = FileId::new(1).unwrap();

        let code = r#"
// Re-exports
export { Component } from 'react';
export { Helper as PublicHelper } from './helper';
export * from './utils';

// Type re-exports
export type { Props } from './types';
"#;

        let imports = parser.find_imports(code, file_id);

        println!("Export-based imports found:");
        for import in &imports {
            println!(
                "  - {} -> {:?} (glob: {})",
                import.path, import.alias, import.is_glob
            );
        }

        assert!(
            imports
                .iter()
                .any(|i| i.path == "react" && i.alias.is_none())
        );
        assert!(imports.iter().any(|i| i.path == "./helper"));
        assert!(imports.iter().any(|i| i.path == "./utils" && i.is_glob));

        println!("✅ Export variations handled correctly");
    }
}
