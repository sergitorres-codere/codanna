//! Kotlin language parser implementation
//!
//! Provides symbol extraction for Kotlin using tree-sitter.

use crate::parsing::Import;
use crate::parsing::parser::check_recursion_depth;
use crate::parsing::{
    HandledNode, Language, LanguageParser, NodeTracker, NodeTrackingState, ParserContext, ScopeType,
};
use crate::types::SymbolCounter;
use crate::{FileId, Range, Symbol, SymbolKind, Visibility};
use std::any::Any;
use tree_sitter::{Node, Parser};

const FILE_SCOPE: &str = "<file>";

/// Parser for Kotlin source files
pub struct KotlinParser {
    parser: Parser,
    node_tracker: NodeTrackingState,
}

impl std::fmt::Debug for KotlinParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KotlinParser")
            .field("language", &"Kotlin")
            .finish()
    }
}

impl KotlinParser {
    /// Create a new parser instance
    pub fn new() -> Result<Self, String> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_kotlin::language())
            .map_err(|e| format!("Failed to initialize Kotlin parser: {e}"))?;

        Ok(Self {
            parser,
            node_tracker: NodeTrackingState::new(),
        })
    }

    /// Convert a tree-sitter node into a Range
    fn node_to_range(&self, node: Node) -> Range {
        let start = node.start_position();
        let end = node.end_position();
        Range {
            start_line: start.row as u32,
            start_column: start.column as u16,
            end_line: end.row as u32,
            end_column: end.column as u16,
        }
    }

    /// Helper to register handled node kinds for audit tracking
    fn register_node(&mut self, node: &Node) {
        self.node_tracker
            .register_handled_node(node.kind(), node.kind_id());
    }

    /// Extract raw source text for a node
    fn text_for_node<'a>(&self, code: &'a str, node: Node) -> &'a str {
        &code[node.byte_range()]
    }

    /// Extract documentation comments (/** */ or //)
    fn doc_comment_for(&self, node: &Node, code: &str) -> Option<String> {
        let mut comments = Vec::new();
        let mut current = node.prev_sibling();

        // Special case: if previous sibling is package_header, check its last child for comments
        if let Some(sibling) = current {
            if sibling.kind() == "package_header" {
                // Check last named children of package_header for comments
                let mut cursor = sibling.walk();
                for child in sibling.named_children(&mut cursor) {
                    if child.kind() == "multiline_comment" || child.kind() == "line_comment" {
                        let raw = self.text_for_node(code, child).trim();
                        if raw.starts_with("/**") || raw.starts_with("///") {
                            let cleaned = raw
                                .trim_start_matches("/**")
                                .trim_end_matches("*/")
                                .trim_start_matches("///")
                                .trim();
                            comments.push(cleaned.to_string());
                        }
                    }
                }
                if !comments.is_empty() {
                    return Some(comments.join("\n"));
                }
            }
        }

        // Standard case: check previous siblings for doc comments
        current = node.prev_sibling();
        while let Some(sibling) = current {
            // Kotlin uses multiline_comment and line_comment node kinds
            if sibling.kind() != "multiline_comment" && sibling.kind() != "line_comment" {
                break;
            }

            let raw = self.text_for_node(code, sibling).trim();
            if raw.starts_with("/**") || raw.starts_with("///") {
                let cleaned = raw
                    .trim_start_matches("/**")
                    .trim_end_matches("*/")
                    .trim_start_matches("///")
                    .trim();
                comments.push(cleaned.to_string());
                current = sibling.prev_sibling();
            } else {
                break;
            }
        }

        if comments.is_empty() {
            None
        } else {
            comments.reverse();
            Some(comments.join("\n"))
        }
    }

    /// Determine visibility from modifiers
    fn determine_visibility(&self, node: Node, code: &str) -> Visibility {
        // Look for modifiers node
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifiers" {
                let modifiers_text = self.text_for_node(code, child);
                if modifiers_text.contains("private") {
                    return Visibility::Private;
                } else if modifiers_text.contains("protected") {
                    return Visibility::Module; // Map protected to module-level
                } else if modifiers_text.contains("internal") {
                    return Visibility::Crate; // Map internal to crate-level
                }
            }
        }
        Visibility::Public // Kotlin default is public
    }

    /// Extract signature text for a node
    fn extract_signature(&self, node: Node, code: &str) -> String {
        let mut cursor = node.walk();
        let mut sig_parts = Vec::new();

        // Collect modifiers
        for child in node.children(&mut cursor) {
            match child.kind() {
                "modifiers" => {
                    sig_parts.push(self.text_for_node(code, child).to_string());
                }
                "simple_identifier" | "type_identifier" => {
                    sig_parts.push(self.text_for_node(code, child).to_string());
                }
                "function_value_parameters" | "class_parameters" => {
                    sig_parts.push(self.text_for_node(code, child).to_string());
                }
                "type" | "user_type" | "type_reference" => {
                    sig_parts.push(format!(": {}", self.text_for_node(code, child)));
                }
                _ => {}
            }
        }

        sig_parts.join(" ")
    }

    /// Process AST recursively and collect symbols
    fn extract_symbols_from_node(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        symbols: &mut Vec<Symbol>,
        counter: &mut SymbolCounter,
        context: &mut ParserContext,
        depth: usize,
    ) {
        if !check_recursion_depth(depth, node) {
            return;
        }

        match node.kind() {
            "class_declaration" => {
                self.handle_class_declaration(
                    node, code, file_id, symbols, counter, context, depth,
                );
                return;
            }
            "object_declaration" => {
                self.handle_object_declaration(
                    node, code, file_id, symbols, counter, context, depth,
                );
                return;
            }
            "function_declaration" => {
                self.handle_function_declaration(
                    node, code, file_id, symbols, counter, context, depth,
                );
                return;
            }
            "property_declaration" => {
                self.handle_property_declaration(node, code, file_id, symbols, counter, context);
            }
            "secondary_constructor" => {
                self.handle_secondary_constructor(node, code, file_id, symbols, counter, context);
            }
            "package_header" | "import_list" | "type_alias" => {
                self.register_node(&node);
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_symbols_from_node(
                child,
                code,
                file_id,
                symbols,
                counter,
                context,
                depth + 1,
            );
        }
    }

    fn handle_class_declaration(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        symbols: &mut Vec<Symbol>,
        counter: &mut SymbolCounter,
        context: &mut ParserContext,
        depth: usize,
    ) {
        self.register_node(&node);

        // Check if this is actually an interface or enum (keywords are children of class_declaration)
        let mut is_interface = false;
        let mut is_enum = false;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "interface" {
                is_interface = true;
                self.register_node(&child); // Register the interface keyword node
                break;
            } else if child.kind() == "enum" {
                is_enum = true;
                self.register_node(&child); // Register the enum keyword node
                break;
            }
        }

        // Extract class/interface name - find the type_identifier child
        let mut class_name = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_identifier" {
                class_name = Some(self.text_for_node(code, child).trim().to_string());
                break;
            }
        }

        let class_name = if let Some(name) = class_name {
            name
        } else {
            return;
        };

        let symbol_id = counter.next_id();
        let range = self.node_to_range(node);
        let visibility = self.determine_visibility(node, code);
        let signature = self.extract_signature(node, code);
        let doc_comment = self.doc_comment_for(&node, code);

        // Determine symbol kind based on modifiers
        let symbol_kind = if is_interface {
            SymbolKind::Interface
        } else if is_enum {
            SymbolKind::Enum
        } else {
            SymbolKind::Class
        };

        let mut symbol = Symbol::new(symbol_id, class_name.as_str(), symbol_kind, file_id, range);
        symbol.visibility = visibility;
        symbol.signature = Some(signature.into());
        if let Some(doc) = doc_comment {
            symbol.doc_comment = Some(doc.into());
        }
        symbol.scope_context = Some(crate::symbol::ScopeContext::ClassMember);

        // Add to context
        context.enter_scope(ScopeType::Class);
        context.set_current_class(Some(class_name.clone()));
        symbols.push(symbol);

        // Process class/interface/enum body
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "class_body" || child.kind() == "enum_class_body" {
                if child.kind() == "enum_class_body" {
                    self.register_node(&child); // Register enum_class_body
                }
                let mut body_cursor = child.walk();
                for body_child in child.children(&mut body_cursor) {
                    // Extract enum entries as constants
                    if is_enum && body_child.kind() == "enum_entry" {
                        self.handle_enum_entry(body_child, code, file_id, symbols, counter);
                    } else {
                        self.extract_symbols_from_node(
                            body_child,
                            code,
                            file_id,
                            symbols,
                            counter,
                            context,
                            depth + 1,
                        );
                    }
                }
                break;
            }
        }

        context.exit_scope();
    }

    fn handle_object_declaration(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        symbols: &mut Vec<Symbol>,
        counter: &mut SymbolCounter,
        context: &mut ParserContext,
        depth: usize,
    ) {
        self.register_node(&node);

        // Extract object name - find the type_identifier child
        let mut object_name = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_identifier" {
                object_name = Some(self.text_for_node(code, child).trim().to_string());
                break;
            }
        }

        let object_name = if let Some(name) = object_name {
            name
        } else {
            return;
        };

        let symbol_id = counter.next_id();
        let range = self.node_to_range(node);
        let visibility = self.determine_visibility(node, code);
        let signature = self.extract_signature(node, code);
        let doc_comment = self.doc_comment_for(&node, code);

        let mut symbol = Symbol::new(symbol_id, object_name.as_str(), SymbolKind::Class, file_id, range);
        symbol.visibility = visibility;
        symbol.signature = Some(signature.into());
        if let Some(doc) = doc_comment {
            symbol.doc_comment = Some(doc.into());
        }
        symbol.scope_context = Some(crate::symbol::ScopeContext::ClassMember);

        context.enter_scope(ScopeType::Class);
        context.set_current_class(Some(object_name.clone()));
        symbols.push(symbol);

        // Process object body
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "class_body" {
                let mut body_cursor = child.walk();
                for body_child in child.children(&mut body_cursor) {
                    self.extract_symbols_from_node(
                        body_child,
                        code,
                        file_id,
                        symbols,
                        counter,
                        context,
                        depth + 1,
                    );
                }
                break;
            }
        }

        context.exit_scope();
    }

    fn handle_function_declaration(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        symbols: &mut Vec<Symbol>,
        counter: &mut SymbolCounter,
        context: &mut ParserContext,
        depth: usize,
    ) {
        self.register_node(&node);

        // Extract function name - find the simple_identifier child
        let mut func_name = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "simple_identifier" {
                func_name = Some(self.text_for_node(code, child).trim().to_string());
                break;
            }
        }

        let func_name = if let Some(name) = func_name {
            name
        } else {
            return;
        };

        let symbol_id = counter.next_id();
        let range = self.node_to_range(node);
        let visibility = self.determine_visibility(node, code);
        let signature = self.extract_signature(node, code);
        let doc_comment = self.doc_comment_for(&node, code);

        // Determine if it's a method or top-level function
        let kind = if context.is_in_class() {
            SymbolKind::Method
        } else {
            SymbolKind::Function
        };

        let mut symbol = Symbol::new(symbol_id, func_name.as_str(), kind, file_id, range);
        symbol.visibility = visibility;
        symbol.signature = Some(signature.into());
        if let Some(doc) = doc_comment {
            symbol.doc_comment = Some(doc.into());
        }

        context.enter_scope(ScopeType::function());
        context.set_current_function(Some(func_name.clone()));
        symbols.push(symbol);

        // Process function body
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "function_body" {
                let mut body_cursor = child.walk();
                for body_child in child.children(&mut body_cursor) {
                    self.extract_symbols_from_node(
                        body_child,
                        code,
                        file_id,
                        symbols,
                        counter,
                        context,
                        depth + 1,
                    );
                }
                break;
            }
        }

        context.exit_scope();
    }

    fn handle_property_declaration(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        symbols: &mut Vec<Symbol>,
        counter: &mut SymbolCounter,
        _context: &mut ParserContext,
    ) {
        self.register_node(&node);

        // Extract property name - find within variable_declaration
        let mut prop_name = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declaration" {
                let mut var_cursor = child.walk();
                for var_child in child.children(&mut var_cursor) {
                    if var_child.kind() == "simple_identifier" {
                        prop_name = Some(self.text_for_node(code, var_child).trim().to_string());
                        break;
                    }
                }
                break;
            }
        }

        let prop_name = if let Some(name) = prop_name {
            name
        } else {
            return;
        };

        let symbol_id = counter.next_id();
        let range = self.node_to_range(node);
        let visibility = self.determine_visibility(node, code);
        let signature = self.extract_signature(node, code);
        let doc_comment = self.doc_comment_for(&node, code);

        let mut symbol = Symbol::new(symbol_id, prop_name.as_str(), SymbolKind::Field, file_id, range);
        symbol.visibility = visibility;
        symbol.signature = Some(signature.into());
        if let Some(doc) = doc_comment {
            symbol.doc_comment = Some(doc.into());
        }

        symbols.push(symbol);
    }

    fn handle_secondary_constructor(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        symbols: &mut Vec<Symbol>,
        counter: &mut SymbolCounter,
        _context: &mut ParserContext,
    ) {
        self.register_node(&node);

        let symbol_id = counter.next_id();
        let range = self.node_to_range(node);
        let signature = self.extract_signature(node, code);

        let mut symbol = Symbol::new(symbol_id, "constructor", SymbolKind::Method, file_id, range);
        symbol.signature = Some(signature.into());

        symbols.push(symbol);
    }

    fn handle_enum_entry(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        symbols: &mut Vec<Symbol>,
        counter: &mut SymbolCounter,
    ) {
        // Extract enum entry name - find the simple_identifier child
        let mut entry_name = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "simple_identifier" {
                entry_name = Some(self.text_for_node(code, child).trim().to_string());
                break;
            }
        }

        let entry_name = if let Some(name) = entry_name {
            name
        } else {
            return;
        };

        let symbol_id = counter.next_id();
        let range = self.node_to_range(node);
        let signature = self.extract_signature(node, code);

        let mut symbol = Symbol::new(symbol_id, entry_name.as_str(), SymbolKind::Constant, file_id, range);
        symbol.signature = Some(signature.into());
        symbol.visibility = Visibility::Public; // Enum entries are always public

        symbols.push(symbol);
    }

    /// Recursively find imports in the AST
    fn find_imports_in_node(
        &self,
        node: Node,
        code: &str,
        file_id: FileId,
        imports: &mut Vec<Import>,
    ) {
        if node.kind() == "import_header" {
            if let Some(identifier) = node.child_by_field_name("identifier") {
                let path = self.text_for_node(code, identifier).trim().to_string();
                if !path.is_empty() {
                    let is_glob = path.ends_with(".*") || path.contains("*");
                    imports.push(Import {
                        file_id,
                        path,
                        alias: None,
                        is_glob,
                        is_type_only: false,
                    });
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.find_imports_in_node(child, code, file_id, imports);
        }
    }

    /// Collect function calls recursively
    fn collect_calls<'a>(
        &self,
        node: Node,
        code: &'a str,
        calls: &mut Vec<(&'a str, &'a str, Range)>,
        current_function: Option<&'a str>,
    ) {
        if node.kind() == "call_expression" {
            if let Some(callee) = node.child(0) {
                let caller = current_function.unwrap_or(FILE_SCOPE);
                let callee_text = self.text_for_node(code, callee).trim();
                if !callee_text.is_empty() {
                    calls.push((caller, callee_text, self.node_to_range(node)));
                }
            }
        }

        // Track current function
        let new_function = if node.kind() == "function_declaration" {
            // Find the simple_identifier child (function name)
            let mut cursor = node.walk();
            let mut func_name = None;
            for child in node.children(&mut cursor) {
                if child.kind() == "simple_identifier" {
                    func_name = Some(self.text_for_node(code, child).trim());
                    break;
                }
            }
            func_name
        } else {
            None
        };

        let func_context = new_function.or(current_function);

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_calls(child, code, calls, func_context);
        }
    }

    /// Collect inheritance relationships (extends/implements)
    fn collect_extends<'a>(
        &self,
        node: Node,
        code: &'a str,
        results: &mut Vec<(&'a str, &'a str, Range)>,
        current_class: Option<&'a str>,
    ) {
        // Track current class
        let new_class = if node.kind() == "class_declaration" || node.kind() == "object_declaration"
        {
            // Find the type_identifier child (class name)
            let mut cursor = node.walk();
            let mut class_name = None;
            for child in node.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    class_name = Some(self.text_for_node(code, child).trim());
                    break;
                }
            }
            class_name
        } else {
            None
        };

        let class_context = new_class.or(current_class);

        // Look for delegation specifiers (: SuperClass, Interface)
        if node.kind() == "delegation_specifier" {
            if let Some(derived) = class_context {
                if let Some(type_node) = node.child(0) {
                    let base = self.text_for_node(code, type_node).trim();
                    // Remove constructor call syntax if present
                    let base_clean = base.split('(').next().unwrap_or(base).trim();
                    if !base_clean.is_empty() {
                        results.push((derived, base_clean, self.node_to_range(node)));
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_extends(child, code, results, class_context);
        }
    }

    /// Extract type usage relationships recursively
    fn extract_type_uses_recursive<'a>(
        &self,
        node: Node,
        code: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
        _current_context: Option<&'a str>,
    ) {
        match node.kind() {
            "class_declaration" | "object_declaration" => {
                // Find the type_identifier child (class name)
                let mut class_name = None;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "type_identifier" {
                        class_name = Some(self.text_for_node(code, child).trim());
                        break;
                    }
                }

                if let Some(class_name) = class_name {
                    // Extract types from primary constructor parameters
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "primary_constructor" {
                            self.extract_parameter_types(child, code, class_name, uses);
                        }
                    }

                    // Process class body recursively
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "class_body" {
                            let mut body_cursor = child.walk();
                            for body_child in child.children(&mut body_cursor) {
                                self.extract_type_uses_recursive(body_child, code, uses, Some(class_name));
                            }
                        }
                    }
                }
                return;
            }
            "function_declaration" => {
                // Find the simple_identifier child (function name)
                let mut func_name = None;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "simple_identifier" {
                        func_name = Some(self.text_for_node(code, child).trim());
                        break;
                    }
                }

                if let Some(func_name) = func_name {
                    // Extract types from function parameters and return type
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "function_value_parameters" {
                            self.extract_parameter_types(child, code, func_name, uses);
                        } else if child.kind() == "user_type" || child.kind() == "type_reference" {
                            // This is the return type
                            if let Some(type_name) = self.extract_type_name(child, code) {
                                uses.push((func_name, type_name, self.node_to_range(child)));
                            }
                        }
                    }
                }
                return;
            }
            "property_declaration" => {
                // Property structure: property_declaration > variable_declaration > (simple_identifier, user_type)
                let mut prop_name = None;
                let mut prop_type = None;

                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "variable_declaration" {
                        // Extract name and type from variable_declaration
                        let mut var_cursor = child.walk();
                        for var_child in child.children(&mut var_cursor) {
                            if var_child.kind() == "simple_identifier" && prop_name.is_none() {
                                prop_name = Some(self.text_for_node(code, var_child).trim());
                            } else if (var_child.kind() == "user_type" || var_child.kind() == "type_reference") && prop_type.is_none() {
                                if let Some(type_name) = self.extract_type_name(var_child, code) {
                                    prop_type = Some((type_name, self.node_to_range(var_child)));
                                }
                            }
                        }
                        break;
                    }
                }

                if let (Some(prop_name), Some((type_name, range))) = (prop_name, prop_type) {
                    uses.push((prop_name, type_name, range));
                }
                return;
            }
            _ => {}
        }

        // Recursively process children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_type_uses_recursive(child, code, uses, _current_context);
        }
    }

    /// Extract parameter types from a parameters node
    fn extract_parameter_types<'a>(
        &self,
        params_node: Node,
        code: &'a str,
        context_name: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let mut cursor = params_node.walk();
        for param in params_node.children(&mut cursor) {
            if param.kind() == "parameter" || param.kind() == "class_parameter" {
                // Look for user_type or type_reference nodes within the parameter
                let mut param_cursor = param.walk();
                for child in param.children(&mut param_cursor) {
                    if child.kind() == "user_type" || child.kind() == "type_reference" {
                        if let Some(type_name) = self.extract_type_name(child, code) {
                            uses.push((context_name, type_name, self.node_to_range(child)));
                        }
                    }
                }
            }
        }
    }

    /// Extract a simple type name from a type node
    fn extract_type_name<'a>(&self, type_node: Node, code: &'a str) -> Option<&'a str> {
        // Handle different type node kinds
        match type_node.kind() {
            "type_reference" | "user_type" | "simple_user_type" => {
                // Look for type_identifier or simple_identifier
                let mut cursor = type_node.walk();
                for child in type_node.children(&mut cursor) {
                    if child.kind() == "type_identifier" || child.kind() == "simple_identifier" {
                        let type_name = self.text_for_node(code, child).trim();
                        // Filter out primitive types
                        if !matches!(
                            type_name,
                            "Int" | "Long" | "Short" | "Byte" | "Float" | "Double" | "Boolean"
                                | "Char" | "String" | "Unit" | "Any" | "Nothing"
                        ) {
                            return Some(type_name);
                        }
                    }
                }
            }
            "type_identifier" | "simple_identifier" => {
                let type_name = self.text_for_node(code, type_node).trim();
                // Filter out primitive types
                if !matches!(
                    type_name,
                    "Int" | "Long" | "Short" | "Byte" | "Float" | "Double" | "Boolean" | "Char"
                        | "String" | "Unit" | "Any" | "Nothing"
                ) {
                    return Some(type_name);
                }
            }
            _ => {}
        }
        None
    }

    /// Extract method definitions from classes and interfaces
    fn extract_method_defines_recursive<'a>(
        &self,
        node: Node,
        code: &'a str,
        defines: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        match node.kind() {
            "class_declaration" | "object_declaration" => {
                // Find the type_identifier child (class name)
                let mut class_name = None;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "type_identifier" {
                        class_name = Some(self.text_for_node(code, child).trim());
                        break;
                    }
                }

                let class_name = class_name.unwrap_or("anonymous");

                // Extract methods from class body
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "class_body" {
                        let mut body_cursor = child.walk();
                        for body_child in child.children(&mut body_cursor) {
                            if body_child.kind() == "function_declaration" {
                                // Find the simple_identifier child (method name)
                                let mut method_cursor = body_child.walk();
                                for method_child in body_child.children(&mut method_cursor) {
                                    if method_child.kind() == "simple_identifier" {
                                        let method_name = self.text_for_node(code, method_child).trim();
                                        defines.push((class_name, method_name, self.node_to_range(body_child)));
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Recursively process children to handle nested classes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_method_defines_recursive(child, code, defines);
        }
    }

    /// Access handled nodes for audit tooling
    pub fn get_handled_nodes(&self) -> &std::collections::HashSet<HandledNode> {
        self.node_tracker.get_handled_nodes()
    }
}

impl LanguageParser for KotlinParser {
    fn parse(
        &mut self,
        code: &str,
        file_id: FileId,
        symbol_counter: &mut SymbolCounter,
    ) -> Vec<Symbol> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let root = tree.root_node();
        let mut symbols = Vec::new();
        let mut context = ParserContext::new();

        // Create a file-level symbol to represent the file itself
        let module_id = symbol_counter.next_id();
        let mut module_symbol = Symbol::new(
            module_id,
            "<file>",
            SymbolKind::Module,
            file_id,
            self.node_to_range(root),
        );
        module_symbol.scope_context = Some(crate::symbol::ScopeContext::Module);
        symbols.push(module_symbol);

        self.extract_symbols_from_node(
            root,
            code,
            file_id,
            &mut symbols,
            symbol_counter,
            &mut context,
            0,
        );

        symbols
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn extract_doc_comment(&self, node: &Node, code: &str) -> Option<String> {
        self.doc_comment_for(node, code)
    }

    fn find_calls<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let mut calls = Vec::new();
        self.collect_calls(tree.root_node(), code, &mut calls, None);
        calls
    }

    fn find_implementations<'a>(&mut self, _code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        // Kotlin doesn't have explicit "implements" - it uses delegation specifiers
        // This is handled in find_extends
        Vec::new()
    }

    fn find_extends<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let mut results = Vec::new();
        self.collect_extends(tree.root_node(), code, &mut results, None);
        results
    }

    fn find_uses<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let mut uses = Vec::new();
        self.extract_type_uses_recursive(tree.root_node(), code, &mut uses, None);
        uses
    }

    fn find_defines<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let mut defines = Vec::new();
        self.extract_method_defines_recursive(tree.root_node(), code, &mut defines);
        defines
    }

    fn find_imports(&mut self, code: &str, file_id: FileId) -> Vec<Import> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut imports = Vec::new();

        self.find_imports_in_node(root_node, code, file_id, &mut imports);

        imports
    }

    fn language(&self) -> Language {
        Language::Kotlin
    }
}

impl NodeTracker for KotlinParser {
    fn get_handled_nodes(&self) -> &std::collections::HashSet<HandledNode> {
        self.node_tracker.get_handled_nodes()
    }

    fn register_handled_node(&mut self, node_kind: &str, node_id: u16) {
        self.node_tracker.register_handled_node(node_kind, node_id);
    }
}
