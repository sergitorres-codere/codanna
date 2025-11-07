//! C# parser implementation using tree-sitter
//!
//! This module provides comprehensive C# language parsing with support for:
//! - Symbol extraction (classes, interfaces, structs, enums, methods, properties, fields, events)
//! - Method call detection with proper caller context tracking
//! - Interface implementation tracking
//! - Using directive (import) tracking
//! - Visibility modifier handling (public, private, internal, protected)
//! - Signature extraction for methods and types
//! - Namespace/module path tracking
//!
//! **Tree-sitter ABI Version**: ABI-14 (tree-sitter-c-sharp 0.23.1)
//! **Total supported node types**: 503
//!
//! # Architecture
//!
//! The parser maintains scope context while traversing the AST to correctly
//! identify which method/class is making each call. This is critical for
//! relationship resolution.
//!
//! # Limitations
//!
//! - Type usage tracking (find_uses) is not yet implemented
//! - Define relationships (containment) are not yet implemented
//! - External framework references (e.g., System.Console) require special handling

use crate::parsing::Import;
use crate::parsing::parser::check_recursion_depth;
use crate::parsing::{
    HandledNode, LanguageParser, MethodCall, NodeTracker, NodeTrackingState, ParserContext,
    ScopeType,
};
use crate::types::SymbolCounter;
use crate::{FileId, Range, Symbol, SymbolKind, Visibility};
use std::any::Any;
use std::collections::HashSet;
use tree_sitter::{Language, Node, Parser};

/// C# language parser using tree-sitter
///
/// This parser traverses C# Abstract Syntax Trees (AST) to extract symbols,
/// relationships, and other code intelligence data.
///
/// # Fields
///
/// - `parser`: The underlying tree-sitter parser configured for C#
/// - `context`: Tracks current scope (class, method) during traversal for proper caller identification
/// - `node_tracker`: Prevents duplicate processing of tree-sitter nodes
///
/// # Example Usage
///
/// ```no_run
/// use codanna::parsing::csharp::parser::CSharpParser;
/// use codanna::parsing::LanguageParser;
///
/// let mut parser = CSharpParser::new().expect("Failed to create parser");
/// let code = "class Foo { void Bar() { } }";
/// // Parse and extract symbols...
/// ```
pub struct CSharpParser {
    parser: Parser,
    context: ParserContext,
    node_tracker: NodeTrackingState,
}

impl CSharpParser {
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

    /// Parse C# source code and extract all symbols
    pub fn parse(
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
                    0,
                );
            }
            None => {
                eprintln!("Failed to parse C# file");
            }
        }

        symbols
    }

    /// Create a new C# parser
    pub fn new() -> Result<Self, String> {
        let mut parser = Parser::new();
        let language: Language = tree_sitter_c_sharp::LANGUAGE.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to set C# language: {e}"))?;

        Ok(Self {
            parser,
            context: ParserContext::new(),
            node_tracker: NodeTrackingState::new(),
        })
    }

    /// Extract symbols from a C# node
    fn extract_symbols_from_node(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        symbols: &mut Vec<Symbol>,
        module_path: &str,
        depth: usize,
    ) {
        // Guard against stack overflow
        if !check_recursion_depth(depth, node) {
            return;
        }
        match node.kind() {
            // Namespace declarations
            "namespace_declaration" | "file_scoped_namespace_declaration" => {
                self.register_handled_node(node.kind(), node.kind_id());
                if let Some(namespace_path) = self.extract_namespace_name(node, code) {
                    // Process all children in this namespace context
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        self.extract_symbols_from_node(
                            child,
                            code,
                            file_id,
                            counter,
                            symbols,
                            &namespace_path,
                            depth + 1,
                        );
                    }
                }
            }

            // Class declarations
            "class_declaration" => {
                // Register ALL child nodes for audit tracking
                self.register_node_recursively(node);

                let class_name = self.extract_type_name(node, code);

                if let Some(symbol) = self.process_class(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);

                    // Enter class scope for processing members
                    self.context.enter_scope(ScopeType::Class);
                    let saved_class = self.context.current_class().map(|s| s.to_string());
                    self.context.set_current_class(class_name.clone());

                    // Extract class members
                    self.extract_class_members(
                        node,
                        code,
                        file_id,
                        counter,
                        symbols,
                        module_path,
                        depth + 1,
                    );

                    self.context.exit_scope();
                    self.context.set_current_class(saved_class);
                }
            }

            // Interface declarations
            "interface_declaration" => {
                // Register ALL child nodes for audit tracking
                self.register_node_recursively(node);

                if let Some(symbol) =
                    self.process_interface(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);

                    // Process interface members
                    self.context.enter_scope(ScopeType::Class);
                    self.extract_interface_members(
                        node,
                        code,
                        file_id,
                        counter,
                        symbols,
                        module_path,
                        depth + 1,
                    );
                    self.context.exit_scope();
                }
            }

            // Struct declarations
            "struct_declaration" => {
                // Register ALL child nodes for audit tracking
                self.register_node_recursively(node);

                if let Some(symbol) = self.process_struct(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);

                    // Process struct members
                    self.context.enter_scope(ScopeType::Class);
                    self.extract_class_members(
                        node,
                        code,
                        file_id,
                        counter,
                        symbols,
                        module_path,
                        depth + 1,
                    );
                    self.context.exit_scope();
                }
            }

            // Enum declarations
            "enum_declaration" => {
                // Register ALL child nodes for audit tracking
                self.register_node_recursively(node);

                if let Some(symbol) = self.process_enum(node, code, file_id, counter, module_path) {
                    symbols.push(symbol);

                    // Process enum members
                    self.extract_enum_members(node, code, file_id, counter, symbols, module_path);
                }
            }

            // Record declarations (C# 9+)
            "record_declaration" => {
                // Register ALL child nodes for audit tracking
                self.register_node_recursively(node);

                if let Some(symbol) = self.process_record(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);

                    // Process record members
                    self.context.enter_scope(ScopeType::Class);
                    self.extract_class_members(
                        node,
                        code,
                        file_id,
                        counter,
                        symbols,
                        module_path,
                        depth + 1,
                    );
                    self.context.exit_scope();
                }
            }

            // Delegate declarations
            "delegate_declaration" => {
                // Register ALL child nodes for audit tracking
                self.register_node_recursively(node);

                if let Some(symbol) =
                    self.process_delegate(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                }
            }

            // Method declarations (standalone or in classes)
            "method_declaration" => {
                // Register ALL child nodes for audit tracking
                self.register_node_recursively(node);

                if let Some(symbol) = self.process_method(node, code, file_id, counter, module_path)
                {
                    let method_name = symbol.name.to_string();
                    symbols.push(symbol);

                    // Process method body for local functions with proper caller context
                    self.context
                        .enter_scope(ScopeType::Function { hoisting: false });
                    self.context.set_current_function(Some(method_name));
                    self.extract_method_body(node, code, file_id, counter, symbols, module_path);
                    self.context.exit_scope();
                }
            }

            // Local function statements
            "local_function_statement" => {
                self.register_handled_node(node.kind(), node.kind_id());
                if let Some(symbol) =
                    self.process_local_function(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                }
            }

            // Field declarations
            "field_declaration" => {
                self.register_handled_node(node.kind(), node.kind_id());
                self.process_field_declaration(node, code, file_id, counter, symbols, module_path);
            }

            // Property declarations
            "property_declaration" => {
                self.register_handled_node(node.kind(), node.kind_id());
                if let Some(symbol) =
                    self.process_property(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                }
            }

            // Event declarations
            "event_declaration" | "event_field_declaration" => {
                self.register_handled_node(node.kind(), node.kind_id());
                if let Some(symbol) = self.process_event(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                }
            }

            // Constructor declarations
            "constructor_declaration" => {
                self.register_handled_node(node.kind(), node.kind_id());
                if let Some(symbol) =
                    self.process_constructor(node, code, file_id, counter, module_path)
                {
                    symbols.push(symbol);
                }
            }

            // Variable declarations
            "variable_declaration" | "local_declaration_statement" => {
                self.register_handled_node(node.kind(), node.kind_id());
                self.process_variable_declaration(
                    node,
                    code,
                    file_id,
                    counter,
                    symbols,
                    module_path,
                );
            }

            // Default case: recursively process children
            _ => {
                self.register_handled_node(node.kind(), node.kind_id());
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_symbols_from_node(
                        child,
                        code,
                        file_id,
                        counter,
                        symbols,
                        module_path,
                        depth + 1,
                    );
                }
            }
        }
    }

    /// Extract namespace name from namespace declaration
    fn extract_namespace_name(&self, node: Node, code: &str) -> Option<String> {
        if let Some(name_node) = node.child_by_field_name("name") {
            Some(code[name_node.byte_range()].to_string())
        } else {
            // Fallback: look for qualified_name child
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "qualified_name" || child.kind() == "identifier" {
                    return Some(code[child.byte_range()].to_string());
                }
            }
            None
        }
    }

    /// Extract type name (for classes, interfaces, structs, etc.)
    fn extract_type_name(&self, node: Node, code: &str) -> Option<String> {
        if let Some(name_node) = node.child_by_field_name("name") {
            Some(code[name_node.byte_range()].to_string())
        } else {
            // Fallback: look for identifier child
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    return Some(code[child.byte_range()].to_string());
                }
            }
            None
        }
    }

    /// Process class declaration
    fn process_class(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_class_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
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

    /// Process interface declaration
    fn process_interface(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_interface_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
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

    /// Process struct declaration
    fn process_struct(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_struct_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
            SymbolKind::Struct,
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

    /// Process enum declaration
    fn process_enum(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_enum_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
            SymbolKind::Enum,
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

    /// Process record declaration (C# 9+)
    fn process_record(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_record_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
            SymbolKind::Class, // Records are class-like in C#
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

    /// Process delegate declaration
    fn process_delegate(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_delegate_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
            SymbolKind::Function, // Delegates are function types
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

    /// Extract class signature (including generics, base classes, interfaces)
    fn extract_class_signature(&self, node: Node, code: &str) -> String {
        self.extract_signature_excluding_body(node, code, "class_body")
    }

    /// Extract interface signature
    fn extract_interface_signature(&self, node: Node, code: &str) -> String {
        self.extract_signature_excluding_body(node, code, "interface_body")
    }

    /// Extract struct signature
    fn extract_struct_signature(&self, node: Node, code: &str) -> String {
        self.extract_signature_excluding_body(node, code, "struct_body")
    }

    /// Extract enum signature
    fn extract_enum_signature(&self, node: Node, code: &str) -> String {
        self.extract_signature_excluding_body(node, code, "enum_body")
    }

    /// Extract record signature
    fn extract_record_signature(&self, node: Node, code: &str) -> String {
        self.extract_signature_excluding_body(node, code, "record_body")
    }

    /// Extract delegate signature
    fn extract_delegate_signature(&self, node: Node, code: &str) -> String {
        // Delegates don't have bodies, so extract the full node
        code[node.byte_range()].trim().to_string()
    }

    /// Extract method signature (including return type, parameters, generics)
    fn extract_method_signature(&self, node: Node, code: &str) -> String {
        self.extract_signature_excluding_body(node, code, "method_body")
    }

    /// Extract property signature (including type and accessors)
    fn extract_property_signature(&self, node: Node, code: &str) -> String {
        self.extract_signature_excluding_body(node, code, "accessor_list")
    }

    /// Extract constructor signature
    fn extract_constructor_signature(&self, node: Node, code: &str) -> String {
        self.extract_signature_excluding_body(node, code, "constructor_body")
    }

    /// Extract field signature
    fn extract_field_signature(&self, node: Node, code: &str) -> String {
        // Field declarations don't have bodies, but we want just the declaration part
        code[node.byte_range()].trim().to_string()
    }

    /// Extract enum member signature
    fn extract_enum_member_signature(&self, node: Node, code: &str) -> String {
        // Enum members can have values like "Red = 1" or just "Red"
        code[node.byte_range()].trim().to_string()
    }

    /// Extract event signature
    fn extract_event_signature(&self, node: Node, code: &str) -> String {
        // Events can have custom add/remove accessors, but often are just simple declarations
        self.extract_signature_excluding_body(node, code, "accessor_list")
    }

    /// Extract variable signature
    fn extract_variable_signature(&self, node: Node, code: &str) -> String {
        // Variables are declared like "int x = 5;" or "var name = value;"
        code[node.byte_range()].trim().to_string()
    }

    /// Extract calls recursively with function context tracking (TypeScript pattern)
    fn extract_calls_recursive<'a>(
        node: &Node,
        code: &'a str,
        current_function: Option<&'a str>,
        calls: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Handle function context - track which function we're inside
        let function_context = if matches!(
            node.kind(),
            "method_declaration"
                | "constructor_declaration"
                | "property_declaration"
                | "local_function_statement"
        ) {
            // Extract function name
            node.child_by_field_name("name")
                .or_else(|| {
                    // Fallback: find first identifier child
                    let mut cursor = node.walk();
                    node.children(&mut cursor)
                        .find(|n| n.kind() == "identifier")
                })
                .map(|name_node| &code[name_node.byte_range()])
        } else {
            // Not a function, inherit current context
            current_function
        };

        // Handle invocation expressions with proper caller context
        if node.kind() == "invocation_expression" {
            if let Some(expression_node) = node.child(0) {
                let caller = function_context.unwrap_or("");
                let callee = match expression_node.kind() {
                    "member_access_expression" => {
                        // obj.Method() - get method name
                        expression_node
                            .child_by_field_name("name")
                            .map(|n| &code[n.byte_range()])
                            .unwrap_or(&code[expression_node.byte_range()])
                    }
                    "identifier" => {
                        // Simple method call like "DoSomething()"
                        &code[expression_node.byte_range()]
                    }
                    _ => &code[expression_node.byte_range()],
                };

                let range = Range::new(
                    node.start_position().row as u32,
                    node.start_position().column as u16,
                    node.end_position().row as u32,
                    node.end_position().column as u16,
                );
                calls.push((caller, callee, range));
            }
        }

        // Recursively process children with inherited or updated context
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::extract_calls_recursive(&child, code, function_context, calls);
        }
    }

    /// Extract method calls from a node tree with proper caller context
    ///
    /// This method traverses the AST and identifies method invocations while maintaining
    /// proper scope context. It tracks which class and method the call originates from,
    /// which is essential for relationship resolution.
    ///
    /// # Key Features
    ///
    /// - Maintains scope stack (class -> method) during traversal
    /// - Correctly identifies caller for each method invocation
    /// - Handles both member access (`obj.Method()`) and simple calls (`Method()`)
    /// - Extracts receiver information for member access patterns
    ///
    /// # Arguments
    ///
    /// - `node`: Current AST node being processed
    /// - `code`: Source code string for extracting text
    /// - `method_calls`: Output vector to collect method calls
    fn extract_method_calls_from_node(
        &mut self,
        node: Node,
        code: &str,
        method_calls: &mut Vec<MethodCall>,
    ) {
        match node.kind() {
            // Track scope changes to maintain caller context
            "class_declaration"
            | "struct_declaration"
            | "record_declaration"
            | "interface_declaration" => {
                // Extract class/struct name
                let type_name = node
                    .children(&mut node.walk())
                    .find(|child| child.kind() == "identifier")
                    .map(|child| code[child.byte_range()].to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                self.context.enter_scope(ScopeType::Class);
                self.context.set_current_class(Some(type_name));

                // Recursively process children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_method_calls_from_node(child, code, method_calls);
                }

                self.context.exit_scope();
            }
            "method_declaration" | "constructor_declaration" | "property_declaration" => {
                // Extract method name
                let method_name = node
                    .children(&mut node.walk())
                    .find(|child| child.kind() == "identifier")
                    .map(|child| code[child.byte_range()].to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                self.context.enter_scope(ScopeType::function());
                self.context.set_current_function(Some(method_name));

                // Recursively process children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_method_calls_from_node(child, code, method_calls);
                }

                self.context.exit_scope();
            }
            "invocation_expression" => {
                // Get caller from current scope context
                let caller = self
                    .context
                    .current_function()
                    .or_else(|| self.context.current_class())
                    .unwrap_or("unknown");

                if let Some(expression_node) = node.child(0) {
                    match expression_node.kind() {
                        "member_access_expression" => {
                            // obj.Method() calls
                            if let Some(object_node) =
                                expression_node.child_by_field_name("expression")
                            {
                                if let Some(name_node) = expression_node.child_by_field_name("name")
                                {
                                    let receiver = code[object_node.byte_range()].to_string();
                                    let method = code[name_node.byte_range()].to_string();
                                    let range = Range::new(
                                        node.start_position().row as u32,
                                        node.start_position().column as u16,
                                        node.end_position().row as u32,
                                        node.end_position().column as u16,
                                    );
                                    method_calls.push(
                                        MethodCall::new(caller, &method, range)
                                            .with_receiver(&receiver),
                                    );
                                }
                            }
                        }
                        "identifier" => {
                            // Simple method calls like Method()
                            let method = code[expression_node.byte_range()].to_string();
                            let range = Range::new(
                                node.start_position().row as u32,
                                node.start_position().column as u16,
                                node.end_position().row as u32,
                                node.end_position().column as u16,
                            );
                            method_calls.push(
                                MethodCall::new(caller, &method, range).with_receiver("this"),
                            );
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                // Recursively check children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_method_calls_from_node(child, code, method_calls);
                }
            }
        }
    }

    /// Extract interface implementations from a node tree
    fn extract_implementations_from_node<'a>(
        node: Node,
        code: &'a str,
        implementations: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        match node.kind() {
            "class_declaration" | "struct_declaration" | "record_declaration" => {
                // Find class name first (identifier child of the class declaration)
                let mut class_name = "";
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        class_name = &code[child.byte_range()];
                        break;
                    }
                }

                // Find base_list
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "base_list" {
                        // Extract interfaces from base list
                        let mut base_cursor = child.walk();
                        for base_child in child.children(&mut base_cursor) {
                            if base_child.kind() == "identifier"
                                || base_child.kind() == "generic_name"
                            {
                                let interface_name = &code[base_child.byte_range()];
                                // Filter out base classes (heuristic: interfaces start with 'I')
                                if interface_name.starts_with('I') && interface_name.len() > 1 {
                                    let range = Range::new(
                                        base_child.start_position().row as u32,
                                        base_child.start_position().column as u16,
                                        base_child.end_position().row as u32,
                                        base_child.end_position().column as u16,
                                    );
                                    implementations.push((class_name, interface_name, range));
                                }
                            }
                        }
                        break;
                    }
                }
            }
            _ => {
                // Recursively check children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_implementations_from_node(child, code, implementations);
                }
            }
        }
    }

    /// Extract imports from a node tree
    fn extract_imports_from_node(
        node: Node,
        code: &str,
        file_id: FileId,
        imports: &mut Vec<Import>,
    ) {
        match node.kind() {
            "using_directive" => {
                // Try standard field extraction first
                if let Some(name_node) = node.child_by_field_name("name") {
                    let import_path = code[name_node.byte_range()].to_string();
                    imports.push(Import {
                        path: import_path,
                        alias: None,
                        file_id,
                        is_glob: false,
                        is_type_only: false,
                    });
                } else {
                    // Fallback: tree-sitter-c-sharp doesn't consistently expose "name" field
                    // for using_directive nodes. Iterate child nodes to find qualified_name
                    // or identifier nodes directly.
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "qualified_name" || child.kind() == "identifier" {
                            let import_path = code[child.byte_range()].to_string();
                            imports.push(Import {
                                path: import_path,
                                alias: None,
                                file_id,
                                is_glob: false,
                                is_type_only: false,
                            });
                            break;
                        }
                    }
                }
            }
            _ => {
                // Recursively check children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_imports_from_node(child, code, file_id, imports);
                }
            }
        }
    }

    /// Helper to extract signature excluding the body
    fn extract_signature_excluding_body(&self, node: Node, code: &str, body_kind: &str) -> String {
        let start = node.start_byte();
        let mut end = node.end_byte();

        // Find the body and stop before it
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == body_kind {
                end = child.start_byte();
                break;
            }
        }

        code[start..end].trim().to_string()
    }

    /// Extract variable type declarations from a node tree (recursive helper)
    ///
    /// This method recursively traverses the syntax tree looking for variable declarations
    /// and extracts variable→type mappings. These mappings are crucial for resolving
    /// method calls on local variables (e.g., `helper.DoWork()` where `helper` is a local variable).
    ///
    /// ## Tree-sitter C# Grammar
    ///
    /// The C# tree-sitter grammar represents variable declarations as either:
    /// - `local_declaration_statement` - for local variables inside methods
    /// - `variable_declaration` - the actual declaration node containing type and declarators
    ///
    /// Example AST structure for `var helper = new Helper();`:
    /// ```text
    /// local_declaration_statement
    ///   └── variable_declaration
    ///       ├── implicit_type ("var")
    ///       └── variable_declarator
    ///           ├── identifier ("helper")
    ///           ├── =
    ///           └── object_creation_expression ("new Helper()")
    /// ```
    ///
    /// ## Supported Patterns
    ///
    /// - `var x = new Type()` - Infers type from initializer
    /// - `Type x = new Type()` - Uses explicit type annotation
    /// - `var x = expr` - For qualified types (when expr type is explicit)
    ///
    /// ## Parameters
    ///
    /// * `node` - Current AST node being processed
    /// * `code` - Source code as string slice
    /// * `bindings` - Accumulated list of (variable_name, type_name, range) tuples
    ///
    /// ## Returns
    ///
    /// Returns tuples of (variable_name, type_name, range) via the `bindings` parameter
    fn find_variable_types_in_node<'a>(
        &self,
        node: &Node,
        code: &'a str,
        bindings: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Match both node types directly (same pattern as extract_symbols_from_node:304)
        // We need to handle both because the tree structure can vary
        if node.kind() == "variable_declaration" || node.kind() == "local_declaration_statement" {
            self.extract_variable_bindings(node, code, bindings);
        }

        // Handle pattern matching expressions (C# 7.0+)
        // Example: if (obj is string s) { } - creates variable 's' of type 'string'
        if node.kind() == "is_pattern_expression" {
            self.extract_pattern_bindings(node, code, bindings);
        }

        // Handle switch expressions (C# 8.0+)
        // Example: value switch { int i => i * 2, string s => s.Length }
        if node.kind() == "switch_expression" {
            self.extract_switch_expression_bindings(node, code, bindings);
        }

        // Handle LINQ query expressions (C# 3.0+)
        // Example: from user in users where user.Age > 18 select user.Name
        if node.kind() == "query_expression" {
            self.extract_query_expression_bindings(node, code, bindings);
        }

        // Recurse into all children to find nested variable declarations
        // (e.g., variables inside nested blocks, loops, etc.)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.find_variable_types_in_node(&child, code, bindings);
        }
    }

    /// Extract variable bindings from a variable_declaration node
    ///
    /// This method processes a single variable declaration and extracts all variable→type
    /// mappings from it. A single declaration can contain multiple variables (e.g.,
    /// `var x = new A(), y = new B();`).
    ///
    /// ## Strategy
    ///
    /// 1. **Type from initializer** (preferred): If the variable has a `new Type()` initializer,
    ///    extract the type from there. This handles the `var` keyword case.
    /// 2. **Explicit type** (fallback): If no initializer or not a `new` expression, use the
    ///    explicit type annotation (but skip "var" since we can't infer the type without initializer).
    ///
    /// ## Examples
    ///
    /// - `var helper = new Helper()` → (helper, Helper) - type inferred from initializer
    /// - `Helper helper = new Helper()` → (helper, Helper) - explicit type used
    /// - `IService service = factory.Create()` → (service, IService) - explicit type used (can't infer from method call)
    /// - `var x = 5` → skipped (no type info available for primitives without full type inference)
    ///
    /// ## Parameters
    ///
    /// * `var_decl` - The variable_declaration or local_declaration_statement node
    /// * `code` - Source code as string slice
    /// * `bindings` - Output list of (variable_name, type_name, range) tuples
    fn extract_variable_bindings<'a>(
        &self,
        var_decl: &Node,
        code: &'a str,
        bindings: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Variable declaration structure in tree-sitter C#:
        // variable_declaration has:
        // - type field: could be "implicit_type" (var) or explicit type like "Helper", "List<T>", etc.
        // - variable_declarator children: one or more declarators (the actual variables)

        let type_node = var_decl.child_by_field_name("type");
        let mut cursor = var_decl.walk();

        for child in var_decl.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                // Each variable_declarator represents one variable in the declaration
                // Structure: identifier = initializer
                // Example: in "var x = new A(), y = new B()", there are two variable_declarators

                if let Some(name_node) = child.child_by_field_name("name") {
                    // Ensure the name is a simple identifier (not a pattern)
                    if name_node.kind() != "identifier" {
                        continue;
                    }
                    let var_name = &code[name_node.byte_range()];

                    // Strategy 1: Try to extract type from initializer (handles "var" keyword)
                    // Uses multiple inference strategies:
                    // - object creation: new Type()
                    // - method invocation: GetUser() → User
                    // - element access: Users[0] → User
                    // - conditional: flag ? new User() : GetUser() → User
                    // - cast: (User)obj → User
                    if let Some(type_name) = self.try_infer_type_from_initializer(&child, code) {
                        let range = Range::new(
                            child.start_position().row as u32,
                            child.start_position().column as u16,
                            child.end_position().row as u32,
                            child.end_position().column as u16,
                        );
                        bindings.push((var_name, type_name, range));
                        continue; // Successfully extracted, move to next variable
                    }

                    // Strategy 2: Fall back to explicit type annotation
                    // This handles cases like "Helper helper = ..." or "IService service = factory.Create()"
                    if let Some(type_node) = type_node {
                        let type_str = &code[type_node.byte_range()];
                        // Skip "var" keyword - we can't infer type without analyzing the full expression
                        // (which would require complex type inference beyond current scope)
                        if type_str != "var" {
                            let range = Range::new(
                                child.start_position().row as u32,
                                child.start_position().column as u16,
                                child.end_position().row as u32,
                                child.end_position().column as u16,
                            );
                            bindings.push((var_name, type_str, range));
                        }
                    }
                }
            }
        }
    }

    /// Try to infer type from an initializer expression using multiple strategies
    ///
    /// Handles:
    /// - `new Type()` → Some("Type") - object creation
    /// - `GetUser()` → Some("User") - method invocation heuristic
    /// - `Users[0]` → Some("User") - element access heuristic
    /// - `flag ? new User() : GetUser()` → Some("User") - conditional expression
    /// - `(User)obj` → Some("User") - cast expression
    fn try_infer_type_from_initializer<'a>(
        &self,
        variable_declarator: &Node,
        code: &'a str,
    ) -> Option<&'a str> {
        let mut cursor = variable_declarator.walk();
        for child in variable_declarator.children(&mut cursor) {
            match child.kind() {
                "object_creation_expression" => {
                    return self.extract_type_from_object_creation(&child, code);
                }
                "invocation_expression" => {
                    return self.extract_type_from_invocation(&child, code);
                }
                "element_access_expression" => {
                    return self.extract_type_from_element_access(&child, code);
                }
                "conditional_expression" => {
                    return self.extract_type_from_conditional(&child, code);
                }
                "cast_expression" => {
                    return self.extract_type_from_cast(&child, code);
                }
                _ => {}
            }
        }
        None
    }

    /// Extract type from object creation expression
    /// Example: `new User()` → Some("User")
    fn extract_type_from_object_creation<'a>(
        &self,
        init_node: &Node,
        code: &'a str,
    ) -> Option<&'a str> {
        if let Some(type_node) = init_node.child_by_field_name("type") {
            return Some(self.extract_simple_type_name(&type_node, code));
        }
        None
    }

    /// Extract type from method invocation using heuristic patterns
    ///
    /// Patterns:
    /// - `GetUser()` → `User`
    /// - `CreateProduct()` → `Product`
    /// - `factory.BuildService()` → `Service`
    fn extract_type_from_invocation<'a>(
        &self,
        invocation: &Node,
        code: &'a str,
    ) -> Option<&'a str> {
        // Get the function being called
        if let Some(function_node) = invocation.child_by_field_name("function") {
            // Try to extract method name from various patterns:
            // - simple: `Create()`
            // - member access: `factory.Create()`
            // - chain: `factory.Build().Create()`

            let method_name = Self::extract_method_name_from_expression(&function_node, code)?;

            // Heuristic: Common factory/builder patterns
            // GetUser() → User, CreateProduct() → Product, BuildService() → Service
            if let Some(type_name) = self.infer_type_from_method_name(method_name) {
                return Some(type_name);
            }
        }
        None
    }

    /// Extract method name from function expression
    fn extract_method_name_from_expression<'a>(expr: &Node, code: &'a str) -> Option<&'a str> {
        match expr.kind() {
            "identifier" => Some(&code[expr.byte_range()]),
            "member_access_expression" => {
                // Get the rightmost identifier (the method name)
                if let Some(name_node) = expr.child_by_field_name("name") {
                    Some(&code[name_node.byte_range()])
                } else {
                    None
                }
            }
            "invocation_expression" => {
                // Chained calls - get the method from the chain
                if let Some(function_node) = expr.child_by_field_name("function") {
                    Self::extract_method_name_from_expression(&function_node, code)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Infer type from common method naming patterns
    ///
    /// Patterns:
    /// - `GetUser()` → `User`
    /// - `CreateProduct()` → `Product`
    /// - `BuildService()` → `Service`
    /// - `ToList()` → `List`
    /// - `AsEnumerable()` → `Enumerable`
    fn infer_type_from_method_name<'a>(&self, method_name: &'a str) -> Option<&'a str> {
        // Common prefixes that indicate return type
        for prefix in &["Get", "Create", "Build", "Make", "New", "To", "As", "Parse"] {
            if let Some(stripped) = method_name.strip_prefix(prefix) {
                if !stripped.is_empty() && stripped.chars().next()?.is_uppercase() {
                    return Some(stripped);
                }
            }
        }

        // If method name itself is PascalCase, treat as factory method
        // Example: factory.User() → User
        if method_name.chars().next()?.is_uppercase() && method_name.len() > 1 {
            return Some(method_name);
        }

        None
    }

    /// Extract type from element access expression
    ///
    /// Heuristic: Assume collection names are plural, element type is singular
    /// Example: `Users[0]` → `User`
    fn extract_type_from_element_access<'a>(
        &self,
        element_access: &Node,
        code: &'a str,
    ) -> Option<&'a str> {
        // Get the expression being indexed (e.g., "Users" in "Users[0]")
        if let Some(expression) = element_access.child_by_field_name("expression") {
            if expression.kind() == "identifier" {
                let collection_name = &code[expression.byte_range()];
                // Try to convert plural to singular
                return self.pluralize_to_singular(collection_name);
            }
        }
        None
    }

    /// Convert plural collection name to singular type name
    /// Simple heuristic: remove trailing 's'
    fn pluralize_to_singular<'a>(&self, plural: &'a str) -> Option<&'a str> {
        if let Some(stripped) = plural.strip_suffix('s') {
            if stripped.len() > 1 && stripped.chars().next()?.is_uppercase() {
                return Some(stripped);
            }
        }
        None
    }

    /// Extract type from conditional (ternary) expression
    ///
    /// If both branches have the same inferred type, use that type
    /// Example: `flag ? new User() : GetUser()` → `User`
    fn extract_type_from_conditional<'a>(
        &self,
        conditional: &Node,
        code: &'a str,
    ) -> Option<&'a str> {
        let consequence = conditional.child_by_field_name("consequence")?;
        let alternative = conditional.child_by_field_name("alternative")?;

        let consequence_type = self.try_infer_type_from_expression(&consequence, code)?;
        let alternative_type = self.try_infer_type_from_expression(&alternative, code)?;

        // If both branches infer the same type, we can be confident
        if consequence_type == alternative_type {
            return Some(consequence_type);
        }

        None
    }

    /// Try to infer type from any expression node
    fn try_infer_type_from_expression<'a>(&self, expr: &Node, code: &'a str) -> Option<&'a str> {
        match expr.kind() {
            "object_creation_expression" => self.extract_type_from_object_creation(expr, code),
            "invocation_expression" => self.extract_type_from_invocation(expr, code),
            "element_access_expression" => self.extract_type_from_element_access(expr, code),
            "conditional_expression" => self.extract_type_from_conditional(expr, code),
            "cast_expression" => self.extract_type_from_cast(expr, code),
            _ => None,
        }
    }

    /// Extract type from cast expression
    /// Example: `(User)obj` → `User`
    fn extract_type_from_cast<'a>(&self, cast: &Node, code: &'a str) -> Option<&'a str> {
        if let Some(type_node) = cast.child_by_field_name("type") {
            return Some(self.extract_simple_type_name(&type_node, code));
        }
        None
    }

    /// Extract simple type name from a type node, handling qualified names and generics
    ///
    /// Examples:
    /// - `Helper` → "Helper"
    /// - `List<T>` → "List"
    /// - `System.Collections.List` → "List"
    fn extract_simple_type_name<'a>(&self, type_node: &Node, code: &'a str) -> &'a str {
        match type_node.kind() {
            "identifier" => &code[type_node.byte_range()],
            "generic_name" => {
                // Generic name has an identifier child
                if let Some(ident) = type_node.child_by_field_name("name") {
                    &code[ident.byte_range()]
                } else {
                    &code[type_node.byte_range()]
                }
            }
            "qualified_name" => {
                // Take the last identifier (rightmost part)
                let mut cursor = type_node.walk();
                let mut last_ident = None;
                for child in type_node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        last_ident = Some(&code[child.byte_range()]);
                    }
                }
                last_ident.unwrap_or(&code[type_node.byte_range()])
            }
            _ => &code[type_node.byte_range()],
        }
    }

    /// Extract variable bindings from pattern matching expressions (C# 7.0+)
    ///
    /// Handles is-pattern expressions like:
    /// - `if (obj is string s)` → (s, string)
    /// - `if (obj is Person { Age: > 18, Name: var n })` → (n, inferred from context)
    /// - `if (obj is int i)` → (i, int)
    ///
    /// ## Tree-sitter Structure
    ///
    /// is_pattern_expression has:
    /// - left: expression being tested
    /// - pattern: the pattern (declaration_pattern, type_pattern, recursive_pattern, etc.)
    ///
    /// declaration_pattern structure:
    /// - type: the type being matched
    /// - designation: variable_designation with identifier
    fn extract_pattern_bindings<'a>(
        &self,
        pattern_expr: &Node,
        code: &'a str,
        bindings: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Get the pattern part (right side of "is")
        if let Some(pattern_node) = pattern_expr.child_by_field_name("pattern") {
            self.extract_type_from_pattern(&pattern_node, code, bindings);
        }
    }

    /// Extract type and variable information from a pattern node
    ///
    /// Handles various pattern types:
    /// - declaration_pattern: `Type varName`
    /// - type_pattern: `Type` (without variable)
    /// - recursive_pattern: `Type { Property: value }` or `{ Property: value }`
    /// - var_pattern: `var name`
    fn extract_type_from_pattern<'a>(
        &self,
        pattern: &Node,
        code: &'a str,
        bindings: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        match pattern.kind() {
            "declaration_pattern" => {
                // Structure: type + name (variable identifier)
                // Example: "string s" in "obj is string s"
                // Fields: type (required), name (optional identifier)
                if let (Some(type_node), Some(name_node)) = (
                    pattern.child_by_field_name("type"),
                    pattern.child_by_field_name("name"),
                ) {
                    let var_name = &code[name_node.byte_range()];
                    let type_name = self.extract_simple_type_name(&type_node, code);
                    let range = Range::new(
                        pattern.start_position().row as u32,
                        pattern.start_position().column as u16,
                        pattern.end_position().row as u32,
                        pattern.end_position().column as u16,
                    );
                    bindings.push((var_name, type_name, range));
                }
            }
            "recursive_pattern" => {
                // Structure: optional type + { property_pattern_clause }
                // Example: "Person { Age: > 18, Name: var n }"
                // We need to extract variables from property patterns
                let mut cursor = pattern.walk();
                for child in pattern.children(&mut cursor) {
                    if child.kind() == "property_pattern_clause" {
                        self.extract_variables_from_property_pattern(&child, code, bindings);
                    }
                }
            }
            "var_pattern" => {
                // var pattern: "var name"
                // We can't infer the type without full type analysis, so skip for now
                // TODO: Could track as "object" or "dynamic" type
            }
            _ => {
                // Recursively check children for nested patterns
                let mut cursor = pattern.walk();
                for child in pattern.children(&mut cursor) {
                    self.extract_type_from_pattern(&child, code, bindings);
                }
            }
        }
    }

    /// Extract variables from property pattern clauses
    ///
    /// Example: `{ Age: > 18, Name: var n }` → extract n
    fn extract_variables_from_property_pattern<'a>(
        &self,
        prop_pattern: &Node,
        code: &'a str,
        bindings: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Look for subpatterns which contain the actual property patterns
        let mut cursor = prop_pattern.walk();
        for child in prop_pattern.children(&mut cursor) {
            if child.kind() == "subpattern" {
                // subpattern has: name_colon (property name) + pattern
                if let Some(pattern) = child.child_by_field_name("pattern") {
                    // Check if it's a var_pattern or declaration_pattern
                    self.extract_type_from_pattern(&pattern, code, bindings);
                }
            }
        }
    }

    /// Extract variable bindings from switch expressions (C# 8.0+)
    ///
    /// Handles switch expressions like:
    /// ```csharp
    /// var result = value switch {
    ///     int i => i * 2,
    ///     string s => s.Length,
    ///     _ => 0
    /// };
    /// ```
    ///
    /// Each arm can introduce a pattern variable that's scoped to that arm's expression.
    fn extract_switch_expression_bindings<'a>(
        &self,
        switch_expr: &Node,
        code: &'a str,
        bindings: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // switch_expression contains switch_expression_arm children
        let mut cursor = switch_expr.walk();
        for child in switch_expr.children(&mut cursor) {
            if child.kind() == "switch_expression_arm" {
                // Each arm has children: pattern and expression (no fields defined)
                // Need to iterate children to find the pattern node
                let mut arm_cursor = child.walk();
                for arm_child in child.children(&mut arm_cursor) {
                    // Look for pattern nodes (various pattern types)
                    if arm_child.kind().contains("pattern")
                        || arm_child.kind() == "declaration_pattern"
                    {
                        self.extract_type_from_pattern(&arm_child, code, bindings);
                        break; // Only process first pattern in arm
                    }
                }
            }
        }
    }

    /// Extract variable bindings from LINQ query expressions (C# 3.0+)
    ///
    /// Handles query expressions like:
    /// ```csharp
    /// var result = from user in users
    ///              where user.Age > 18
    ///              join order in orders on user.Id equals order.UserId
    ///              select new { user.Name, order.Total };
    /// ```
    ///
    /// LINQ queries introduce "range variables" that are scoped to the query:
    /// - `from` clause introduces the initial range variable
    /// - `join` clause introduces additional range variables
    /// - `group...into` introduces a new range variable for the grouped result
    ///
    /// ## Type Inference Limitations
    ///
    /// Full type inference for range variables requires knowing collection element types
    /// (e.g., `List<User>` → `User`). For now, we only extract explicitly typed range variables
    /// and track variable names for relationship analysis.
    fn extract_query_expression_bindings<'a>(
        &self,
        query_expr: &Node,
        code: &'a str,
        bindings: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Process all clauses in the query expression
        // Special handling: identifier after group_clause is the "into" variable
        let mut cursor = query_expr.walk();
        let children: Vec<_> = query_expr.children(&mut cursor).collect();

        for (i, child) in children.iter().enumerate() {
            match child.kind() {
                "from_clause" => {
                    self.extract_from_clause_binding(child, code, bindings);
                }
                "join_clause" => {
                    self.extract_join_clause_binding(child, code, bindings);
                }
                "group_clause" => {
                    // Look for "into" keyword followed by identifier
                    // Pattern: group_clause ... "into" identifier
                    for j in (i + 1)..children.len() {
                        if &code[children[j].byte_range()] == "into"
                            && j + 1 < children.len()
                            && children[j + 1].kind() == "identifier"
                        {
                            let into_var = &code[children[j + 1].byte_range()];
                            let range = Range::new(
                                children[j + 1].start_position().row as u32,
                                children[j + 1].start_position().column as u16,
                                children[j + 1].end_position().row as u32,
                                children[j + 1].end_position().column as u16,
                            );
                            bindings.push((into_var, "IGrouping", range));
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Extract range variable from a from_clause
    ///
    /// Examples:
    /// - `from user in users` → track "user" (type inferred from collection)
    /// - `from User user in users` → (user, User)
    /// - `from int i in numbers` → (i, int)
    fn extract_from_clause_binding<'a>(
        &self,
        from_clause: &Node,
        code: &'a str,
        bindings: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // from_clause structure:
        // - optional type
        // - identifier (range variable name)
        // - "in" keyword
        // - expression (the collection)
        //
        // Example: "from User user in users"
        //   type=User, identifier=user, in, expression=users

        let mut type_node = None;
        let mut var_identifier = None;
        let mut identifiers = Vec::new();

        let mut cursor = from_clause.walk();
        for child in from_clause.children(&mut cursor) {
            // Stop collecting when we hit "in" keyword - everything after is the collection expression
            if child.kind() == "in" {
                break;
            }

            match child.kind() {
                "identifier" => {
                    identifiers.push(child);
                }
                "predefined_type" | "generic_name" | "qualified_name" if type_node.is_none() => {
                    type_node = Some(child);
                }
                _ => {}
            }
        }

        // If we have 2 identifiers, first is type, second is variable
        // If we have 1 identifier and a type node, the identifier is the variable
        // If we have 1 identifier and no type node, no explicit type (skip for now)
        if identifiers.len() >= 2 {
            // First identifier is type, second is variable
            type_node = Some(identifiers[0]);
            var_identifier = Some(identifiers[1]);
        } else if identifiers.len() == 1 && type_node.is_some() {
            // We have an explicit type node, so the identifier is the variable
            var_identifier = Some(identifiers[0]);
        } else if identifiers.len() == 1 {
            // Only one identifier with no explicit type - this is the variable name
            // but we can't infer the type without collection analysis
            return;
        }

        // If we have both type and identifier, add the binding
        if let (Some(type_node), Some(ident_node)) = (type_node, var_identifier) {
            let var_name = &code[ident_node.byte_range()];
            let type_name = self.extract_simple_type_name(&type_node, code);
            let range = Range::new(
                from_clause.start_position().row as u32,
                from_clause.start_position().column as u16,
                from_clause.end_position().row as u32,
                from_clause.end_position().column as u16,
            );
            bindings.push((var_name, type_name, range));
        }
    }

    /// Extract range variable from a join_clause
    ///
    /// Examples:
    /// - `join order in orders on user.Id equals order.UserId` → track "order"
    /// - `join Order order in orders on ...` → (order, Order)
    fn extract_join_clause_binding<'a>(
        &self,
        join_clause: &Node,
        code: &'a str,
        bindings: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // join_clause has similar structure to from_clause:
        // "join" keyword, optional type, identifier, "in", expression, "on"/"equals"
        // Example: "join Order order in orders on user.Id equals order.UserId"

        let mut type_node = None;
        let mut var_identifier = None;
        let mut identifiers = Vec::new();
        let mut found_join = false;

        let mut cursor = join_clause.walk();
        for child in join_clause.children(&mut cursor) {
            if child.kind() == "join" {
                found_join = true;
                continue;
            }

            if !found_join {
                continue;
            }

            // Stop when we reach "in" keyword or "on"
            if matches!(child.kind(), "in" | "on") {
                break;
            }

            match child.kind() {
                "identifier" => {
                    identifiers.push(child);
                }
                "predefined_type" | "generic_name" | "qualified_name" if type_node.is_none() => {
                    type_node = Some(child);
                }
                _ => {}
            }
        }

        // Same logic as from_clause: 2 identifiers = type + var, 1 identifier + type_node = var only
        if identifiers.len() >= 2 {
            type_node = Some(identifiers[0]);
            var_identifier = Some(identifiers[1]);
        } else if identifiers.len() == 1 && type_node.is_some() {
            var_identifier = Some(identifiers[0]);
        } else if identifiers.len() == 1 {
            return; // Can't infer type
        }

        if let (Some(type_node), Some(ident_node)) = (type_node, var_identifier) {
            let var_name = &code[ident_node.byte_range()];
            let type_name = self.extract_simple_type_name(&type_node, code);
            let range = Range::new(
                join_clause.start_position().row as u32,
                join_clause.start_position().column as u16,
                join_clause.end_position().row as u32,
                join_clause.end_position().column as u16,
            );
            bindings.push((var_name, type_name, range));
        }
    }

    /// Determine visibility from modifiers
    fn determine_visibility(&self, node: Node, code: &str) -> Visibility {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifier" {
                let modifier_text = &code[child.byte_range()];
                if modifier_text.contains("public") {
                    return Visibility::Public;
                } else if modifier_text.contains("private") {
                    return Visibility::Private;
                } else if modifier_text.contains("protected") {
                    return Visibility::Module; // Closest approximation
                } else if modifier_text.contains("internal") {
                    return Visibility::Module;
                }
            }
        }

        // Default C# visibility rules
        match self.context.current_scope_context() {
            crate::symbol::ScopeContext::ClassMember => Visibility::Private, // Class members are private by default
            _ => Visibility::Module, // Top-level types are internal by default
        }
    }

    /// Extract documentation comment
    fn extract_doc_comment(&self, node: &Node, code: &str) -> Option<String> {
        // Collect all consecutive /// comments immediately before this node
        let mut doc_lines = Vec::new();
        let mut current = node.prev_sibling();

        // Walk backwards through siblings, collecting /// comments
        while let Some(sibling) = current {
            if sibling.kind() == "comment" {
                let comment_text = &code[sibling.byte_range()];
                // C# XML documentation comments start with ///
                if comment_text.starts_with("///") {
                    doc_lines.push(comment_text.to_string());
                } else {
                    // Non-doc comment stops the sequence
                    break;
                }
            } else {
                // Non-comment node stops the sequence
                break;
            }
            current = sibling.prev_sibling();
        }

        if doc_lines.is_empty() {
            None
        } else {
            // Reverse to restore original order (we walked backwards)
            doc_lines.reverse();
            Some(doc_lines.join("\n"))
        }
    }

    // Placeholder implementations for member extraction methods
    // These would be implemented similarly to the main symbol extraction

    fn extract_class_members(
        &mut self,
        class_node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        symbols: &mut Vec<Symbol>,
        module_path: &str,
        depth: usize,
    ) {
        // Find the class body
        if let Some(body_node) = class_node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                match child.kind() {
                    "method_declaration" => {
                        if let Some(symbol) =
                            self.process_method(child, code, file_id, counter, module_path)
                        {
                            symbols.push(symbol);
                        }
                    }
                    "property_declaration" => {
                        if let Some(symbol) =
                            self.process_property(child, code, file_id, counter, module_path)
                        {
                            symbols.push(symbol);
                        }
                    }
                    "field_declaration" => {
                        self.process_field_declaration(
                            child,
                            code,
                            file_id,
                            counter,
                            symbols,
                            module_path,
                        );
                    }
                    "constructor_declaration" => {
                        if let Some(symbol) =
                            self.process_constructor(child, code, file_id, counter, module_path)
                        {
                            symbols.push(symbol);
                        }
                    }
                    "event_declaration" | "event_field_declaration" => {
                        if let Some(symbol) =
                            self.process_event(child, code, file_id, counter, module_path)
                        {
                            symbols.push(symbol);
                        }
                    }
                    // Nested types
                    "class_declaration"
                    | "interface_declaration"
                    | "struct_declaration"
                    | "enum_declaration" => {
                        self.extract_symbols_from_node(
                            child,
                            code,
                            file_id,
                            counter,
                            symbols,
                            module_path,
                            depth + 1,
                        );
                    }
                    _ => {
                        // Continue processing other nodes recursively
                        self.extract_symbols_from_node(
                            child,
                            code,
                            file_id,
                            counter,
                            symbols,
                            module_path,
                            depth + 1,
                        );
                    }
                }
            }
        }
    }

    fn extract_interface_members(
        &mut self,
        interface_node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        symbols: &mut Vec<Symbol>,
        module_path: &str,
        depth: usize,
    ) {
        // Find the interface body
        if let Some(body_node) = interface_node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                match child.kind() {
                    "method_declaration" => {
                        if let Some(symbol) =
                            self.process_method(child, code, file_id, counter, module_path)
                        {
                            symbols.push(symbol);
                        }
                    }
                    "property_declaration" => {
                        if let Some(symbol) =
                            self.process_property(child, code, file_id, counter, module_path)
                        {
                            symbols.push(symbol);
                        }
                    }
                    "event_declaration" => {
                        if let Some(symbol) =
                            self.process_event(child, code, file_id, counter, module_path)
                        {
                            symbols.push(symbol);
                        }
                    }
                    _ => {
                        // Continue processing other nodes recursively
                        self.extract_symbols_from_node(
                            child,
                            code,
                            file_id,
                            counter,
                            symbols,
                            module_path,
                            depth + 1,
                        );
                    }
                }
            }
        }
    }

    fn extract_enum_members(
        &mut self,
        enum_node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        symbols: &mut Vec<Symbol>,
        module_path: &str,
    ) {
        // Find the enum body
        if let Some(body_node) = enum_node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                if child.kind() == "enum_member_declaration" {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = code[name_node.byte_range()].to_string();
                        let signature = self.extract_enum_member_signature(child, code);
                        let doc_comment = self.extract_doc_comment(&child, code);

                        let symbol = self.create_symbol(
                            counter.next_id(),
                            name,
                            SymbolKind::Constant, // Enum members are constant values
                            file_id,
                            Range::new(
                                child.start_position().row as u32,
                                child.start_position().column as u16,
                                child.end_position().row as u32,
                                child.end_position().column as u16,
                            ),
                            Some(signature),
                            doc_comment,
                            module_path,
                            Visibility::Public, // Enum members are always public
                        );
                        symbols.push(symbol);
                    }
                }
            }
        }
    }

    fn extract_method_body(
        &mut self,
        method_node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        symbols: &mut Vec<Symbol>,
        module_path: &str,
    ) {
        // Look for local functions and variable declarations within method body
        if let Some(body_node) = method_node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                match child.kind() {
                    "local_function_statement" => {
                        if let Some(symbol) =
                            self.process_local_function(child, code, file_id, counter, module_path)
                        {
                            symbols.push(symbol);
                        }
                    }
                    "local_declaration_statement" => {
                        self.process_variable_declaration(
                            child,
                            code,
                            file_id,
                            counter,
                            symbols,
                            module_path,
                        );
                    }
                    _ => {
                        // Continue recursively for nested blocks
                        self.extract_method_body(
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
    }

    fn process_method(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_method_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
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

    fn process_local_function(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_method_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
            SymbolKind::Function, // Local functions are more like standalone functions
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
            Visibility::Private, // Local functions are always private to their containing method
        ))
    }

    fn process_field_declaration(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        symbols: &mut Vec<Symbol>,
        module_path: &str,
    ) {
        // Field declarations can contain multiple variables
        // e.g., "public int x, y, z;"
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declaration" {
                // Extract each variable declarator
                let mut var_cursor = child.walk();
                for var_child in child.children(&mut var_cursor) {
                    if var_child.kind() == "variable_declarator" {
                        if let Some(name_node) = var_child.child_by_field_name("name") {
                            let name = code[name_node.byte_range()].to_string();
                            let signature = self.extract_field_signature(node, code);
                            let doc_comment = self.extract_doc_comment(&node, code);
                            let visibility = self.determine_visibility(node, code);

                            let symbol = self.create_symbol(
                                counter.next_id(),
                                name,
                                SymbolKind::Variable,
                                file_id,
                                Range::new(
                                    var_child.start_position().row as u32,
                                    var_child.start_position().column as u16,
                                    var_child.end_position().row as u32,
                                    var_child.end_position().column as u16,
                                ),
                                Some(signature.clone()),
                                doc_comment.clone(),
                                module_path,
                                visibility,
                            );
                            symbols.push(symbol);
                        }
                    }
                }
            }
        }
    }

    fn process_property(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_property_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
            SymbolKind::Field, // Properties are field-like in the symbol system
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

    fn process_event(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_event_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
            SymbolKind::Field, // Events are field-like (similar to properties)
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

    fn process_constructor(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        module_path: &str,
    ) -> Option<Symbol> {
        let name = self.extract_type_name(node, code)?;
        let signature = self.extract_constructor_signature(node, code);
        let doc_comment = self.extract_doc_comment(&node, code);
        let visibility = self.determine_visibility(node, code);

        Some(self.create_symbol(
            counter.next_id(),
            name,
            SymbolKind::Method, // Constructors are method-like
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

    fn process_variable_declaration(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        counter: &mut SymbolCounter,
        symbols: &mut Vec<Symbol>,
        module_path: &str,
    ) {
        // Look for variable_declarator nodes within the declaration
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = code[name_node.byte_range()].to_string();
                    let signature = self.extract_variable_signature(node, code);
                    let doc_comment = self.extract_doc_comment(&node, code);

                    let symbol = self.create_symbol(
                        counter.next_id(),
                        name,
                        SymbolKind::Variable,
                        file_id,
                        Range::new(
                            child.start_position().row as u32,
                            child.start_position().column as u16,
                            child.end_position().row as u32,
                            child.end_position().column as u16,
                        ),
                        Some(signature.clone()),
                        doc_comment.clone(),
                        module_path,
                        Visibility::Private, // Local variables are private
                    );
                    symbols.push(symbol);
                }
            }
        }
    }

    /// Recursively register all nodes in the tree for audit tracking
    ///
    /// This ensures the audit system can see which AST nodes we're actually handling,
    /// making it easier to identify gaps in implementation.
    fn register_node_recursively(&mut self, node: Node) {
        self.register_handled_node(node.kind(), node.kind_id());
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.register_node_recursively(child);
        }
    }
}

impl NodeTracker for CSharpParser {
    fn register_handled_node(&mut self, kind: &str, kind_id: u16) {
        self.node_tracker.register_handled_node(kind, kind_id);
    }

    fn get_handled_nodes(&self) -> &HashSet<HandledNode> {
        self.node_tracker.get_handled_nodes()
    }
}

impl LanguageParser for CSharpParser {
    fn parse(&mut self, code: &str, file_id: FileId, counter: &mut SymbolCounter) -> Vec<Symbol> {
        self.parse(code, file_id, counter)
    }

    fn find_calls<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => {
                eprintln!("Failed to parse C# file for calls");
                return Vec::new();
            }
        };

        let root_node = tree.root_node();
        let mut calls = Vec::new();

        // Use recursive extraction with function context tracking (like TypeScript)
        Self::extract_calls_recursive(&root_node, code, None, &mut calls);

        calls
    }

    fn find_method_calls(&mut self, code: &str) -> Vec<MethodCall> {
        let mut method_calls = Vec::new();

        // Reset context to ensure clean state
        self.context = ParserContext::new();

        match self.parser.parse(code, None) {
            Some(tree) => {
                let root_node = tree.root_node();
                self.extract_method_calls_from_node(root_node, code, &mut method_calls);
            }
            None => {
                eprintln!("Failed to parse C# file for method calls");
            }
        }

        method_calls
    }

    fn find_implementations<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let mut implementations = Vec::new();

        match self.parser.parse(code, None) {
            Some(tree) => {
                let root_node = tree.root_node();
                Self::extract_implementations_from_node(root_node, code, &mut implementations);
            }
            None => {
                eprintln!("Failed to parse C# file for implementations");
            }
        }

        implementations
    }

    fn find_uses<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let mut uses = Vec::new();

        match self.parser.parse(code, None) {
            Some(tree) => {
                let root_node = tree.root_node();
                Self::extract_uses_from_node(root_node, code, &mut uses, None, None);
            }
            None => {
                eprintln!("Failed to parse C# file for type uses");
            }
        }

        uses
    }

    fn find_defines<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let mut defines = Vec::new();

        match self.parser.parse(code, None) {
            Some(tree) => {
                let root_node = tree.root_node();
                Self::extract_defines_from_node(root_node, code, &mut defines, None, None);
            }
            None => {
                eprintln!("Failed to parse C# file for defines");
            }
        }

        defines
    }

    /// Extract variable type bindings from C# code
    ///
    /// This method implements variable type tracking for C#, which is essential for
    /// resolving method calls on local variables. Without this, codanna cannot resolve
    /// relationships like `var service = new MyService(); service.DoWork();` because
    /// it doesn't know that `service` is of type `MyService`.
    ///
    /// ## How It Works
    ///
    /// 1. Parse the C# file into an AST using tree-sitter-c-sharp
    /// 2. Recursively traverse the tree looking for variable declarations
    /// 3. For each variable, extract the type either from:
    ///    - The initializer expression (`new Type()`)
    ///    - The explicit type annotation
    /// 4. Return list of (variable_name, type_name, source_location) tuples
    ///
    /// ## Example
    ///
    /// ```csharp
    /// public void Example() {
    ///     var helper = new Helper();  // → ("helper", "Helper", Range)
    ///     helper.DoWork();             // Now codanna can resolve DoWork() on Helper type
    /// }
    /// ```
    ///
    /// ## Limitations
    ///
    /// - Only tracks variables with explicit type or `new Type()` initializers
    /// - Does not perform full type inference (e.g., `var x = 5` is not tracked)
    /// - Does not track method return types without explicit annotation
    ///
    /// ## Returns
    ///
    /// Vector of tuples: (variable_name, type_name, source_range)
    /// where type_name is a string slice pointing into the original source code (zero-copy)
    fn find_variable_types<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let mut bindings = Vec::new();

        if let Some(tree) = self.parser.parse(code, None) {
            let root = tree.root_node();
            self.find_variable_types_in_node(&root, code, &mut bindings);
        }

        bindings
    }

    fn find_imports(&mut self, code: &str, file_id: FileId) -> Vec<Import> {
        let mut imports = Vec::new();

        match self.parser.parse(code, None) {
            Some(tree) => {
                let root_node = tree.root_node();
                Self::extract_imports_from_node(root_node, code, file_id, &mut imports);
            }
            None => {
                eprintln!("Failed to parse C# file for imports");
            }
        }

        imports
    }

    fn extract_doc_comment(&self, node: &Node, code: &str) -> Option<String> {
        self.extract_doc_comment(node, code)
    }

    fn language(&self) -> crate::parsing::Language {
        crate::parsing::Language::CSharp
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Helper methods for find_uses() implementation
impl CSharpParser {
    /// Recursively extract type usage relationships from AST
    ///
    /// Tracks where types are used:
    /// - Method parameter types: `void DoWork(Helper helper)` → ("DoWork", "Helper", range)
    /// - Return types: `Helper GetHelper()` → ("GetHelper", "Helper", range)
    /// - Field/property types: `private Helper _helper;` → ("MyClass", "Helper", range)
    /// - Base classes/interfaces: `class Foo : IBar` → ("Foo", "IBar", range)
    ///
    /// # Parameters
    /// - `node`: Current AST node being processed
    /// - `code`: Source code string
    /// - `uses`: Output vector for (user_symbol, used_type, range) tuples
    /// - `current_class`: Name of the class we're currently inside (if any)
    /// - `current_method`: Name of the method we're currently inside (if any)
    fn extract_uses_from_node<'a>(
        node: Node,
        code: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
        current_class: Option<&'a str>,
        current_method: Option<&'a str>,
    ) {
        match node.kind() {
            // Track class/struct/record declarations for context
            "class_declaration" | "struct_declaration" | "record_declaration" => {
                // Extract class name
                let class_name = Self::find_identifier_child(node, code);

                // Extract base classes and interfaces from base_list
                if let Some(class_name) = class_name {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "base_list" {
                            Self::extract_base_types(child, code, class_name, uses);
                        }
                    }
                }

                // Recursively process children with updated class context
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_uses_from_node(child, code, uses, class_name, None);
                }
            }

            // Track method declarations
            "method_declaration" | "constructor_declaration" | "local_function_statement" => {
                // Use "name" field for method name, not first identifier (which might be return type)
                let method_name = node
                    .child_by_field_name("name")
                    .map(|n| &code[n.byte_range()]);
                let user_context = method_name.or(current_method).or(current_class);

                // Extract return type (for methods, not constructors)
                if node.kind() == "method_declaration" {
                    // Field is called "returns", not "type"
                    if let Some(return_type) = node.child_by_field_name("returns") {
                        if let Some(user) = user_context {
                            Self::extract_type_usage(&return_type, code, user, uses);
                        }
                    }
                }

                // Extract parameter types
                if let Some(params) = node.child_by_field_name("parameters") {
                    if let Some(user) = user_context {
                        Self::extract_parameter_types(params, code, user, uses);
                    }
                }

                // Recursively process method body with updated method context
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_uses_from_node(child, code, uses, current_class, method_name);
                }
            }

            // Track field declarations
            "field_declaration" => {
                if let Some(class_name) = current_class {
                    // field_declaration contains variable_declaration child which has "type" field
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "variable_declaration" {
                            if let Some(type_node) = child.child_by_field_name("type") {
                                Self::extract_type_usage(&type_node, code, class_name, uses);
                            }
                            break; // Only process first variable_declaration
                        }
                    }
                }

                // Continue processing children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_uses_from_node(child, code, uses, current_class, current_method);
                }
            }

            // Track property declarations
            "property_declaration" => {
                if let Some(class_name) = current_class {
                    if let Some(type_node) = node.child_by_field_name("type") {
                        Self::extract_type_usage(&type_node, code, class_name, uses);
                    }
                }

                // Continue processing children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_uses_from_node(child, code, uses, current_class, current_method);
                }
            }

            // Track event declarations
            "event_declaration" => {
                // event_declaration has a "type" field
                if let Some(class_name) = current_class {
                    if let Some(type_node) = node.child_by_field_name("type") {
                        Self::extract_type_usage(&type_node, code, class_name, uses);
                    }
                }

                // Continue processing children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_uses_from_node(child, code, uses, current_class, current_method);
                }
            }

            "event_field_declaration" => {
                // event_field_declaration contains variable_declaration child with "type" field
                if let Some(class_name) = current_class {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "variable_declaration" {
                            if let Some(type_node) = child.child_by_field_name("type") {
                                Self::extract_type_usage(&type_node, code, class_name, uses);
                            }
                            break;
                        }
                    }
                }

                // Continue processing children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_uses_from_node(child, code, uses, current_class, current_method);
                }
            }

            // Recursively process all other nodes
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_uses_from_node(child, code, uses, current_class, current_method);
                }
            }
        }
    }

    /// Helper to find the identifier child of a node (typically the name)
    fn find_identifier_child<'a>(node: Node, code: &'a str) -> Option<&'a str> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                return Some(&code[child.byte_range()]);
            }
        }
        None
    }

    /// Extract base classes and interfaces from base_list
    fn extract_base_types<'a>(
        base_list: Node,
        code: &'a str,
        class_name: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let mut cursor = base_list.walk();
        for child in base_list.children(&mut cursor) {
            match child.kind() {
                "identifier" | "generic_name" | "qualified_name" => {
                    let type_name = &code[child.byte_range()];
                    if !Self::is_primitive_type(type_name) {
                        let range = Range::new(
                            child.start_position().row as u32,
                            child.start_position().column as u16,
                            child.end_position().row as u32,
                            child.end_position().column as u16,
                        );
                        uses.push((class_name, type_name, range));
                    }
                }
                _ => {}
            }
        }
    }

    /// Extract parameter types from parameter_list
    fn extract_parameter_types<'a>(
        params: Node,
        code: &'a str,
        user_name: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let mut cursor = params.walk();
        for child in params.children(&mut cursor) {
            if child.kind() == "parameter" {
                if let Some(type_node) = child.child_by_field_name("type") {
                    Self::extract_type_usage(&type_node, code, user_name, uses);
                }
            }
        }
    }

    /// Extract a single type usage from a type node
    fn extract_type_usage<'a>(
        type_node: &Node,
        code: &'a str,
        user_name: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let type_name = match type_node.kind() {
            "identifier" => &code[type_node.byte_range()],
            "generic_name" => {
                // Extract base type name from generic (e.g., "List" from "List<T>")
                // generic_name has identifier child (not a field)
                let mut cursor = type_node.walk();
                let mut found = None;
                for child in type_node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        found = Some(&code[child.byte_range()]);
                        break;
                    }
                }
                found.unwrap_or(&code[type_node.byte_range()])
            }
            "qualified_name" => {
                // Extract rightmost identifier (e.g., "Helper" from "MyApp.Helper")
                let mut cursor = type_node.walk();
                let mut last_ident = None;
                for child in type_node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        last_ident = Some(&code[child.byte_range()]);
                    }
                }
                last_ident.unwrap_or(&code[type_node.byte_range()])
            }
            "array_type" => {
                // Extract element type from array
                if let Some(element_type) = type_node.child_by_field_name("type") {
                    Self::extract_type_usage(&element_type, code, user_name, uses);
                }
                return; // Early return to avoid adding the array_type itself
            }
            "nullable_type" => {
                // Extract underlying type from nullable
                if let Some(underlying_type) = type_node.child_by_field_name("type") {
                    Self::extract_type_usage(&underlying_type, code, user_name, uses);
                }
                return; // Early return
            }
            "predefined_type" => {
                // This is a primitive type like int, string, etc.
                return; // Skip primitives
            }
            _ => return, // Skip unknown types
        };

        // Filter out primitive types
        if !Self::is_primitive_type(type_name) {
            let range = Range::new(
                type_node.start_position().row as u32,
                type_node.start_position().column as u16,
                type_node.end_position().row as u32,
                type_node.end_position().column as u16,
            );
            uses.push((user_name, type_name, range));
        }
    }

    /// Check if a type name is a C# primitive type
    fn is_primitive_type(type_name: &str) -> bool {
        matches!(
            type_name,
            "void"
                | "bool"
                | "byte"
                | "sbyte"
                | "char"
                | "decimal"
                | "double"
                | "float"
                | "int"
                | "uint"
                | "long"
                | "ulong"
                | "short"
                | "ushort"
                | "object"
                | "string"
                | "dynamic"
                | "var"
        )
    }

    /// Recursively extract definition relationships from AST
    ///
    /// Tracks containment relationships:
    /// - Class defines method: `("UserService", "GetUser", range)`
    /// - Class defines field: `("UserService", "_repository", range)`
    /// - Class defines property: `("UserService", "CurrentUser", range)`
    /// - Method defines variable: `("GetUser", "result", range)`
    ///
    /// # Parameters
    /// - `node`: Current AST node being processed
    /// - `code`: Source code string
    /// - `defines`: Output vector for (definer, defined, range) tuples
    /// - `current_class`: Name of the class we're currently inside (if any)
    /// - `current_method`: Name of the method we're currently inside (if any)
    fn extract_defines_from_node<'a>(
        node: Node,
        code: &'a str,
        defines: &mut Vec<(&'a str, &'a str, Range)>,
        current_class: Option<&'a str>,
        current_method: Option<&'a str>,
    ) {
        match node.kind() {
            // Track class/struct/record declarations as definers
            "class_declaration" | "struct_declaration" | "record_declaration" => {
                let class_name = Self::find_identifier_child(node, code);

                if let Some(class_name) = class_name {
                    // Find all members defined by this class
                    // Members are inside the declaration_list child
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "declaration_list" {
                            // Now iterate the members inside declaration_list
                            let mut decl_cursor = child.walk();
                            for member in child.children(&mut decl_cursor) {
                                match member.kind() {
                                    "method_declaration" | "constructor_declaration" => {
                                        if let Some(method_name) = member
                                            .child_by_field_name("name")
                                            .map(|n| &code[n.byte_range()])
                                        {
                                            let range = Range::new(
                                                member.start_position().row as u32,
                                                member.start_position().column as u16,
                                                member.end_position().row as u32,
                                                member.end_position().column as u16,
                                            );
                                            defines.push((class_name, method_name, range));
                                        }
                                    }
                                    "field_declaration" => {
                                        // field_declaration contains variable_declaration with variable_declarator children
                                        Self::extract_field_names(
                                            &member, code, class_name, defines,
                                        );
                                    }
                                    "property_declaration" => {
                                        if let Some(prop_name) = member
                                            .child_by_field_name("name")
                                            .map(|n| &code[n.byte_range()])
                                        {
                                            let range = Range::new(
                                                member.start_position().row as u32,
                                                member.start_position().column as u16,
                                                member.end_position().row as u32,
                                                member.end_position().column as u16,
                                            );
                                            defines.push((class_name, prop_name, range));
                                        }
                                    }
                                    "event_declaration" => {
                                        if let Some(event_name) = member
                                            .child_by_field_name("name")
                                            .map(|n| &code[n.byte_range()])
                                        {
                                            let range = Range::new(
                                                member.start_position().row as u32,
                                                member.start_position().column as u16,
                                                member.end_position().row as u32,
                                                member.end_position().column as u16,
                                            );
                                            defines.push((class_name, event_name, range));
                                        }
                                    }
                                    "event_field_declaration" => {
                                        // Similar to field_declaration, contains variable_declaration
                                        Self::extract_field_names(
                                            &member, code, class_name, defines,
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    // Recursively process children with updated class context
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        Self::extract_defines_from_node(
                            child,
                            code,
                            defines,
                            Some(class_name),
                            None,
                        );
                    }
                } else {
                    // No class name found, continue recursively
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        Self::extract_defines_from_node(
                            child,
                            code,
                            defines,
                            current_class,
                            current_method,
                        );
                    }
                }
            }

            // Track interface declarations
            "interface_declaration" => {
                let interface_name = Self::find_identifier_child(node, code);

                if let Some(interface_name) = interface_name {
                    // Find all methods defined by this interface
                    // Methods are inside the declaration_list child
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "declaration_list" {
                            let mut decl_cursor = child.walk();
                            for member in child.children(&mut decl_cursor) {
                                if member.kind() == "method_declaration" {
                                    if let Some(method_name) = member
                                        .child_by_field_name("name")
                                        .map(|n| &code[n.byte_range()])
                                    {
                                        let range = Range::new(
                                            member.start_position().row as u32,
                                            member.start_position().column as u16,
                                            member.end_position().row as u32,
                                            member.end_position().column as u16,
                                        );
                                        defines.push((interface_name, method_name, range));
                                    }
                                }
                            }
                        }
                    }

                    // Recursively process children
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        Self::extract_defines_from_node(
                            child,
                            code,
                            defines,
                            Some(interface_name),
                            None,
                        );
                    }
                } else {
                    // No interface name found, continue recursively
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        Self::extract_defines_from_node(
                            child,
                            code,
                            defines,
                            current_class,
                            current_method,
                        );
                    }
                }
            }

            // Track method declarations as definers of local variables
            "method_declaration" | "constructor_declaration" | "local_function_statement" => {
                let method_name = node
                    .child_by_field_name("name")
                    .map(|n| &code[n.byte_range()]);

                // Process method body to find local variable definitions
                if let Some(method_name) = method_name {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        Self::extract_defines_from_node(
                            child,
                            code,
                            defines,
                            current_class,
                            Some(method_name),
                        );
                    }
                } else {
                    // No method name, continue recursively
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        Self::extract_defines_from_node(
                            child,
                            code,
                            defines,
                            current_class,
                            current_method,
                        );
                    }
                }
            }

            // Track local variable declarations
            "local_declaration_statement" => {
                if let Some(definer) = current_method.or(current_class) {
                    // Look for variable_declaration child
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "variable_declaration" {
                            Self::extract_variable_names(&child, code, definer, defines);
                        }
                    }
                }

                // Continue processing children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_defines_from_node(
                        child,
                        code,
                        defines,
                        current_class,
                        current_method,
                    );
                }
            }

            // Recursively process all other nodes
            _ => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    Self::extract_defines_from_node(
                        child,
                        code,
                        defines,
                        current_class,
                        current_method,
                    );
                }
            }
        }
    }

    /// Extract field names from field_declaration or event_field_declaration
    fn extract_field_names<'a>(
        field_decl: &Node,
        code: &'a str,
        class_name: &'a str,
        defines: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let mut cursor = field_decl.walk();
        for child in field_decl.children(&mut cursor) {
            if child.kind() == "variable_declaration" {
                Self::extract_variable_names(&child, code, class_name, defines);
            }
        }
    }

    /// Extract variable names from variable_declaration
    fn extract_variable_names<'a>(
        var_decl: &Node,
        code: &'a str,
        definer: &'a str,
        defines: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let mut cursor = var_decl.walk();
        for child in var_decl.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if name_node.kind() == "identifier" {
                        let var_name = &code[name_node.byte_range()];
                        let range = Range::new(
                            child.start_position().row as u32,
                            child.start_position().column as u16,
                            child.end_position().row as u32,
                            child.end_position().column as u16,
                        );
                        defines.push((definer, var_name, range));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FileId, SymbolCounter};

    #[test]
    fn test_csharp_interface_implementation_tracking() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public interface ILogger {
                void Log(string message);
            }

            public class ConsoleLogger : ILogger {
                public void Log(string message) {
                    Console.WriteLine(message);
                }
            }
        "#;

        let implementations = parser.find_implementations(code);

        // Should find ConsoleLogger implements ILogger
        assert!(
            implementations
                .iter()
                .any(|(from, to, _)| *from == "ConsoleLogger" && *to == "ILogger"),
            "Should detect ConsoleLogger implements ILogger. Found: {implementations:?}"
        );
    }

    #[test]
    fn test_csharp_method_call_tracking_with_context() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class Calculator {
                private int Add(int a, int b) { return a + b; }

                public int Calculate() {
                    return Add(5, 10);
                }
            }
        "#;

        let calls = parser.find_calls(code);

        // Should find Calculate -> Add with proper caller context
        assert!(
            calls
                .iter()
                .any(|(from, to, _)| *from == "Calculate" && *to == "Add"),
            "Should detect Calculate -> Add with caller context. Found: {:?}",
            calls
                .iter()
                .map(|(f, t, _)| format!("{f} -> {t}"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_csharp_enum_extraction() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public enum Status {
                Active,
                Inactive,
                Pending = 5
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        // Should extract enum and its members
        assert!(
            symbols
                .iter()
                .any(|s| s.name.as_ref() == "Status" && s.kind == SymbolKind::Enum)
        );
        assert!(symbols.iter().any(|s| s.name.as_ref() == "Active"));
        assert!(symbols.iter().any(|s| s.name.as_ref() == "Pending"));
    }

    #[test]
    fn test_csharp_multiline_doc_comment_extraction() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            /// <summary>
            /// This is a multi-line
            /// XML documentation comment
            /// </summary>
            public class DocumentedClass {
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let class_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "DocumentedClass")
            .unwrap();
        let doc = class_symbol.doc_comment.as_ref().unwrap();

        // Should capture all lines of XML documentation
        assert!(doc.contains("<summary>"));
        assert!(doc.contains("multi-line"));
        assert!(doc.contains("</summary>"));
    }

    #[test]
    fn test_csharp_method_calls_in_method() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class Service {
                public void Process() {
                    Validate();
                    Transform();
                    Save();
                }

                private void Validate() { }
                private void Transform() { }
                private void Save() { }
            }
        "#;

        let method_calls = parser.find_method_calls(code);

        // Should find all three calls from Process method
        assert!(
            method_calls
                .iter()
                .any(|c| c.caller == "Process" && c.method_name == "Validate")
        );
        assert!(
            method_calls
                .iter()
                .any(|c| c.caller == "Process" && c.method_name == "Transform")
        );
        assert!(
            method_calls
                .iter()
                .any(|c| c.caller == "Process" && c.method_name == "Save")
        );
    }

    #[test]
    #[ignore = "find_imports implementation needs to be completed - currently returns empty"]
    fn test_csharp_using_directive_extraction() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            using System;
            using System.Collections.Generic;
            using MyApp.Services;

            namespace TestNamespace {
                public class TestClass { }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let imports = parser.find_imports(code, file_id);

        // Should extract all using directives
        assert!(
            imports.len() >= 3,
            "Should find at least 3 imports, found: {}",
            imports.len()
        );
        assert!(imports.iter().any(|i| i.path == "System"));
        assert!(
            imports
                .iter()
                .any(|i| i.path == "System.Collections.Generic")
        );
        assert!(imports.iter().any(|i| i.path == "MyApp.Services"));
    }

    #[test]
    fn test_csharp_is_pattern_type_extraction() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void TestMethod(object obj) {
                if (obj is string s) {
                    Console.WriteLine(s);
                }

                if (obj is int i) {
                    Console.WriteLine(i);
                }

                if (obj is Person p) {
                    Console.WriteLine(p);
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should extract variable types from is-patterns
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "s" && *typ == "string"),
            "Should find 's' of type 'string'. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "i" && *typ == "int"),
            "Should find 'i' of type 'int'. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "p" && *typ == "Person"),
            "Should find 'p' of type 'Person'. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_switch_expression_patterns() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public int Calculate(object value) {
                var result = value switch {
                    int i => i * 2,
                    string s => s.Length,
                    double d => (int)d,
                    _ => 0
                };
                return result;
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should extract pattern variables from switch expression arms
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "i" && *typ == "int"),
            "Should find 'i' of type 'int' in switch expression. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "s" && *typ == "string"),
            "Should find 's' of type 'string' in switch expression. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "d" && *typ == "double"),
            "Should find 'd' of type 'double' in switch expression. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_generic_type_pattern() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void Process(object obj) {
                if (obj is List<string> list) {
                    Console.WriteLine(list.Count);
                }

                if (obj is Dictionary<int, string> dict) {
                    Console.WriteLine(dict.Count);
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should extract generic types (extracting base type name)
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "list" && typ.contains("List")),
            "Should find 'list' with List type. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "dict" && typ.contains("Dictionary")),
            "Should find 'dict' with Dictionary type. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_nested_patterns() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void TestNested(object obj) {
                if (obj is string s) {
                    var inner = s switch {
                        string t when t.Length > 0 => t,
                        _ => null
                    };
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should find both outer and inner pattern variables
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "s" && *typ == "string"),
            "Should find outer pattern variable 's'. Found: {bindings:?}"
        );
        // Note: 't' may or may not be extracted depending on when clause handling
    }

    #[test]
    fn test_csharp_multiple_patterns_same_method() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void MultiPattern(object obj1, object obj2) {
                if (obj1 is string s1 && obj2 is int i1) {
                    Console.WriteLine($"{s1}: {i1}");
                }

                var x = obj1 switch {
                    string s2 => s2.Length,
                    _ => 0
                };

                if (obj2 is double d1) {
                    Console.WriteLine(d1);
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should extract all pattern variables
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "s1" && *typ == "string"),
            "Should find 's1' of type 'string'. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "i1" && *typ == "int"),
            "Should find 'i1' of type 'int'. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "s2" && *typ == "string"),
            "Should find 's2' of type 'string'. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "d1" && *typ == "double"),
            "Should find 'd1' of type 'double'. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_pattern_with_qualified_type() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void TestQualified(object obj) {
                if (obj is System.String s) {
                    Console.WriteLine(s);
                }

                if (obj is MyNamespace.MyClass m) {
                    Console.WriteLine(m);
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should extract simple type name from qualified types
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "s" && *typ == "String"),
            "Should find 's' with simple type name 'String'. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "m" && *typ == "MyClass"),
            "Should find 'm' with simple type name 'MyClass'. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_linq_from_clause_with_explicit_type() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void TestQuery() {
                var result = from User user in users
                             where user.Age > 18
                             select user.Name;

                var numbers = from int num in collection
                              select num * 2;
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should extract range variables with explicit types
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "user" && *typ == "User"),
            "Should find 'user' of type 'User' in from clause. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "num" && *typ == "int"),
            "Should find 'num' of type 'int' in from clause. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_linq_join_clause() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void TestJoin() {
                var result = from User user in users
                             join Order order in orders on user.Id equals order.UserId
                             select new { user.Name, order.Total };

                var joined = from Person p in people
                             join Address addr in addresses on p.Id equals addr.PersonId
                             select new { p.Name, addr.City };
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should extract range variables from join clauses
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "user" && *typ == "User"),
            "Should find 'user' from from clause. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "order" && *typ == "Order"),
            "Should find 'order' from join clause. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "p" && *typ == "Person"),
            "Should find 'p' from from clause. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "addr" && *typ == "Address"),
            "Should find 'addr' from join clause. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_linq_group_into() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void TestGrouping() {
                var grouped = from Product item in items
                              group item by item.Category into g
                              select new { Category = g.Key, Count = g.Count() };
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should extract range variable from from clause
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "item" && *typ == "Product"),
            "Should find 'item' of type 'Product'. Found: {bindings:?}"
        );
        // Should extract grouping variable
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "g" && *typ == "IGrouping"),
            "Should find 'g' as IGrouping from group...into. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_linq_multiple_from_clauses() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void TestMultipleFrom() {
                var result = from Customer c in customers
                             from Order o in c.Orders
                             where o.Total > 100
                             select new { c.Name, o.Total };
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should extract both range variables (if they have explicit types)
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "c" && *typ == "Customer"),
            "Should find 'c' of type 'Customer'. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "o" && *typ == "Order"),
            "Should find 'o' of type 'Order'. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_linq_with_generic_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void TestGenericQuery() {
                var result = from List<string> list in collections
                             where list.Count > 0
                             select list;
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should extract generic type (base name)
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "list" && typ.contains("List")),
            "Should find 'list' with List type. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_linq_nested_queries() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void TestNested() {
                var outer = from Customer c in customers
                            select (from Order o in c.Orders
                                   where o.Total > 100
                                   select o);
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should find both outer and inner range variables
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "c" && *typ == "Customer"),
            "Should find outer range variable 'c'. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "o" && *typ == "Order"),
            "Should find inner range variable 'o'. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_combined_patterns_and_linq() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public void TestCombined(object obj) {
                // Pattern matching
                if (obj is List<User> users) {
                    // LINQ query using pattern variable
                    var result = from User u in users
                                 where u.Age > 18
                                 select u.Name;
                }

                // Switch expression with LINQ
                var data = obj switch {
                    IEnumerable<Person> people =>
                        from Person p in people select p.Name,
                    _ => null
                };
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Should find pattern variables
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "users" && typ.contains("List")),
            "Should find 'users' from pattern. Found: {bindings:?}"
        );
        // Should find LINQ range variables
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "u" && *typ == "User"),
            "Should find 'u' from LINQ query. Found: {bindings:?}"
        );
        // Should find both pattern and LINQ variables in switch expression
        assert!(
            bindings.iter().any(|(var, _, _)| *var == "people"),
            "Should find 'people' from switch pattern. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "p" && *typ == "Person"),
            "Should find 'p' from nested LINQ. Found: {bindings:?}"
        );
    }

    // Extended Type Inference Tests

    #[test]
    fn test_csharp_type_inference_method_invocation() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            class UserService {
                void ProcessData() {
                    var user1 = GetUser();
                    var product = CreateProduct();
                    var service = BuildService();
                    var list = ToList();
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "user1" && *typ == "User"),
            "Should infer User from GetUser(). Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "product" && *typ == "Product"),
            "Should infer Product from CreateProduct(). Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "service" && *typ == "Service"),
            "Should infer Service from BuildService(). Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "list" && *typ == "List"),
            "Should infer List from ToList(). Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_type_inference_member_access() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            class FactoryTest {
                void Test() {
                    var user = factory.GetUser();
                    var item = repository.CreateItem();
                    var data = builder.BuildData();
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "user" && *typ == "User"),
            "Should infer User from factory.GetUser(). Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "item" && *typ == "Item"),
            "Should infer Item from repository.CreateItem(). Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "data" && *typ == "Data"),
            "Should infer Data from builder.BuildData(). Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_type_inference_element_access() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            class CollectionTest {
                void Test() {
                    var user = Users[0];
                    var item = Items[index];
                    var product = Products[i];
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "user" && *typ == "User"),
            "Should infer User from Users[0]. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "item" && *typ == "Item"),
            "Should infer Item from Items[index]. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "product" && *typ == "Product"),
            "Should infer Product from Products[i]. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_type_inference_conditional() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            class ConditionalTest {
                void Test() {
                    var user1 = flag ? new User() : new User();
                    var item = condition ? GetItem() : CreateItem();
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "user1" && *typ == "User"),
            "Should infer User from ternary with new User() in both branches. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "item" && *typ == "Item"),
            "Should infer Item from ternary with GetItem() and CreateItem(). Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_type_inference_cast() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            class CastTest {
                void Test() {
                    var user = (User)obj;
                    var item = (IItem)data;
                    var service = (Services.UserService)provider;
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "user" && *typ == "User"),
            "Should infer User from (User)obj. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "item" && *typ == "IItem"),
            "Should infer IItem from (IItem)data. Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "service" && *typ == "UserService"),
            "Should infer UserService from qualified cast. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_type_inference_factory_patterns() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            class FactoryPatternTest {
                void Test() {
                    var user1 = factory.User();
                    var product = builder.Product();
                    var user2 = ParseUser();
                    var data = AsEnumerable();
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "user1" && *typ == "User"),
            "Should infer User from factory.User(). Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "product" && *typ == "Product"),
            "Should infer Product from builder.Product(). Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "user2" && *typ == "User"),
            "Should infer User from ParseUser(). Found: {bindings:?}"
        );
        assert!(
            bindings
                .iter()
                .any(|(var, typ, _)| *var == "data" && *typ == "Enumerable"),
            "Should infer Enumerable from AsEnumerable(). Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_type_inference_complex_scenarios() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            class ComplexTest {
                void Test() {
                    var user1 = GetUser();
                    var user2 = factory.CreateUser();
                    var user3 = Users[0];
                    var user4 = flag ? new User() : GetUser();
                    var user5 = (User)obj;
                }
            }
        "#;

        let bindings = parser.find_variable_types(code);

        // Count how many User variables we found via extended inference
        let user_count = bindings
            .iter()
            .filter(|(var, typ, _)| var.starts_with("user") && *typ == "User")
            .count();

        assert!(
            user_count >= 5,
            "Should find at least 5 User variables from inference. Found: {bindings:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_method_parameters() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    public void Process(User user, IRepository repo) {
        // method body
    }
}
"#;

        let uses = parser.find_uses(code);

        // Should find User and IRepository as used types
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "Process" && *used == "User"),
            "Should find User parameter. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "Process" && *used == "IRepository"),
            "Should find IRepository parameter. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_return_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    public User GetUser() {
        return null;
    }

    public IRepository GetRepository() {
        return null;
    }
}
"#;

        let uses = parser.find_uses(code);

        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "GetUser" && *used == "User"),
            "Should find User return type. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "GetRepository" && *used == "IRepository"),
            "Should find IRepository return type. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_field_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    private IRepository _repository;
    private User _currentUser;
    public Helper helper;
}
"#;

        let uses = parser.find_uses(code);

        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserService" && *used == "IRepository"),
            "Should find IRepository field. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserService" && *used == "User"),
            "Should find User field. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserService" && *used == "Helper"),
            "Should find Helper field. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_property_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    public User CurrentUser { get; set; }
    public IRepository Repository { get; private set; }
    private Helper InternalHelper { get; }
}
"#;

        let uses = parser.find_uses(code);

        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserService" && *used == "User"),
            "Should find User property. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserService" && *used == "IRepository"),
            "Should find IRepository property. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserService" && *used == "Helper"),
            "Should find Helper property. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_base_classes() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService : BaseService {
    // class body
}

class ProductService : BaseService, IDisposable {
    // class body
}
"#;

        let uses = parser.find_uses(code);

        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserService" && *used == "BaseService"),
            "Should find BaseService base class. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "ProductService" && *used == "BaseService"),
            "Should find BaseService for ProductService. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "ProductService" && *used == "IDisposable"),
            "Should find IDisposable interface. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_interface_implementations() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserRepository : IRepository, IDisposable {
    // class body
}
"#;

        let uses = parser.find_uses(code);

        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserRepository" && *used == "IRepository"),
            "Should find IRepository interface. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserRepository" && *used == "IDisposable"),
            "Should find IDisposable interface. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_generic_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class DataService {
    public List<User> GetUsers() {
        return null;
    }

    public Dictionary<string, Product> products;
}
"#;

        let uses = parser.find_uses(code);

        // Should find List (base generic type) used by GetUsers
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "GetUsers" && *used == "List"),
            "Should find List generic type. Found: {uses:?}"
        );

        // Should find Dictionary (base generic type) used by DataService
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "DataService" && *used == "Dictionary"),
            "Should find Dictionary generic type. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_array_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class ArrayService {
    public User[] GetUsers() {
        return null;
    }

    private Product[] _products;
}
"#;

        let uses = parser.find_uses(code);

        // Should find User (element type, not array itself)
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "GetUsers" && *used == "User"),
            "Should find User array element type. Found: {uses:?}"
        );

        // Should find Product (element type)
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "ArrayService" && *used == "Product"),
            "Should find Product array element type. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_nullable_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class NullableService {
    public User? GetUser() {
        return null;
    }

    private Product? _product;
}
"#;

        let uses = parser.find_uses(code);

        // Should find User (underlying type, not nullable wrapper)
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "GetUser" && *used == "User"),
            "Should find User nullable type. Found: {uses:?}"
        );

        // Should find Product (underlying type)
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "NullableService" && *used == "Product"),
            "Should find Product nullable type. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_qualified_names() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class QualifiedService {
    public MyApp.Models.User GetUser() {
        return null;
    }

    private System.Data.SqlClient.SqlConnection _connection;
}
"#;

        let uses = parser.find_uses(code);

        // Should extract rightmost identifier (User)
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "GetUser" && *used == "User"),
            "Should find User from qualified name. Found: {uses:?}"
        );

        // Should extract rightmost identifier (SqlConnection)
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "QualifiedService" && *used == "SqlConnection"),
            "Should find SqlConnection from qualified name. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_no_primitives() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class PrimitiveService {
    public int GetCount() {
        return 0;
    }

    public void Process(string text, bool flag) {
        // method body
    }

    private int _count;
    public string Name { get; set; }
}
"#;

        let uses = parser.find_uses(code);

        // Should NOT find any primitive types
        assert!(
            !uses
                .iter()
                .any(|(_, used, _)| matches!(*used, "int" | "string" | "bool")),
            "Should not find primitive types. Found: {uses:?}"
        );

        // The uses vector should be empty (no non-primitive types)
        assert!(
            uses.is_empty(),
            "Should find no type uses (all primitives). Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_struct_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
struct Point {
    public double X;
    public double Y;
}

struct Vector : IEquatable {
    private Point _start;
    private Point _end;
}
"#;

        let uses = parser.find_uses(code);

        // Should find Point used by Vector (fields)
        assert_eq!(
            uses.iter()
                .filter(|(user, used, _)| *user == "Vector" && *used == "Point")
                .count(),
            2,
            "Should find Point used twice (two fields). Found: {uses:?}"
        );

        // Should find IEquatable implemented by Vector
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "Vector" && *used == "IEquatable"),
            "Should find IEquatable interface. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_record_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
record Person(string Name, int Age);

record Employee : Person {
    public Company Company { get; init; }
}
"#;

        let uses = parser.find_uses(code);

        // Should find Person as base record for Employee
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "Employee" && *used == "Person"),
            "Should find Person base record. Found: {uses:?}"
        );

        // Should find Company property type
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "Employee" && *used == "Company"),
            "Should find Company property. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_event_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class EventService {
    public event EventHandler UserAdded;
    public event Action<User> UserUpdated;
    private event DataChangedHandler _dataChanged;
}
"#;

        let uses = parser.find_uses(code);

        // Should find EventHandler
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "EventService" && *used == "EventHandler"),
            "Should find EventHandler event type. Found: {uses:?}"
        );

        // Should find Action (generic base)
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "EventService" && *used == "Action"),
            "Should find Action event type. Found: {uses:?}"
        );

        // Should find DataChangedHandler
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "EventService" && *used == "DataChangedHandler"),
            "Should find DataChangedHandler event type. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_constructor_parameters() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    public UserService(IRepository repo, Logger logger) {
        // constructor body
    }
}
"#;

        let uses = parser.find_uses(code);

        // Constructors should track parameter types
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserService" && *used == "IRepository"),
            "Should find IRepository constructor parameter. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserService" && *used == "Logger"),
            "Should find Logger constructor parameter. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_uses_complex_scenario() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserRepository : BaseRepository, IRepository {
    private DbContext _context;
    public Logger Logger { get; set; }

    public UserRepository(DbContext context) {
        _context = context;
    }

    public User GetUser(UserId id) {
        return null;
    }

    public List<User> GetAll() {
        return null;
    }
}
"#;

        let uses = parser.find_uses(code);

        // Base class
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserRepository" && *used == "BaseRepository"),
            "Should find BaseRepository. Found: {uses:?}"
        );

        // Interface
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserRepository" && *used == "IRepository"),
            "Should find IRepository. Found: {uses:?}"
        );

        // Field types
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserRepository" && *used == "DbContext"),
            "Should find DbContext field. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserRepository" && *used == "Logger"),
            "Should find Logger property. Found: {uses:?}"
        );

        // Constructor parameter
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "UserRepository" && *used == "DbContext"),
            "Should find DbContext constructor param. Found: {uses:?}"
        );

        // Method return types
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "GetUser" && *used == "User"),
            "Should find User return type. Found: {uses:?}"
        );
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "GetAll" && *used == "List"),
            "Should find List return type. Found: {uses:?}"
        );

        // Method parameter types
        assert!(
            uses.iter()
                .any(|(user, used, _)| *user == "GetUser" && *used == "UserId"),
            "Should find UserId parameter. Found: {uses:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_class_methods() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    public void GetUser() {
        // method body
    }

    private bool ValidateUser() {
        return true;
    }
}
"#;

        let defines = parser.find_defines(code);

        // Should find class defines methods
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "GetUser"),
            "Should find UserService defines GetUser. Found: {defines:?}"
        );
        assert!(
            defines.iter().any(
                |(definer, defined, _)| *definer == "UserService" && *defined == "ValidateUser"
            ),
            "Should find UserService defines ValidateUser. Found: {defines:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_class_fields() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    private IRepository _repository;
    private Logger _logger;
    public int count;
}
"#;

        let defines = parser.find_defines(code);

        // Should find class defines fields
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService"
                    && *defined == "_repository"),
            "Should find UserService defines _repository. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "_logger"),
            "Should find UserService defines _logger. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "count"),
            "Should find UserService defines count. Found: {defines:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_class_properties() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    public User CurrentUser { get; set; }
    private string Name { get; }
    public int Count { get; set; }
}
"#;

        let defines = parser.find_defines(code);

        // Should find class defines properties
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService"
                    && *defined == "CurrentUser"),
            "Should find UserService defines CurrentUser. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "Name"),
            "Should find UserService defines Name. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "Count"),
            "Should find UserService defines Count. Found: {defines:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_class_events() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    public event EventHandler UserAdded;
    public event Action<User> UserUpdated;
    private event DataChangedHandler _dataChanged;
}
"#;

        let defines = parser.find_defines(code);

        // Should find class defines events
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "UserAdded"),
            "Should find UserService defines UserAdded. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService"
                    && *defined == "UserUpdated"),
            "Should find UserService defines UserUpdated. Found: {defines:?}"
        );
        assert!(
            defines.iter().any(
                |(definer, defined, _)| *definer == "UserService" && *defined == "_dataChanged"
            ),
            "Should find UserService defines _dataChanged. Found: {defines:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_interface_methods() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
interface IRepository {
    User Get(int id);
    void Save(User user);
    void Delete(int id);
}
"#;

        let defines = parser.find_defines(code);

        // Should find interface defines methods
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "IRepository" && *defined == "Get"),
            "Should find IRepository defines Get. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "IRepository" && *defined == "Save"),
            "Should find IRepository defines Save. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "IRepository" && *defined == "Delete"),
            "Should find IRepository defines Delete. Found: {defines:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_method_variables() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    public void ProcessUser() {
        var user = GetUser();
        string name = "test";
        int count = 0;
    }
}
"#;

        let defines = parser.find_defines(code);

        // Should find method defines local variables
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "ProcessUser" && *defined == "user"),
            "Should find ProcessUser defines user. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "ProcessUser" && *defined == "name"),
            "Should find ProcessUser defines name. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "ProcessUser" && *defined == "count"),
            "Should find ProcessUser defines count. Found: {defines:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_struct_members() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
struct Point {
    public double X;
    public double Y;

    public double Distance() {
        return 0.0;
    }
}
"#;

        let defines = parser.find_defines(code);

        // Should find struct defines fields and methods
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Point" && *defined == "X"),
            "Should find Point defines X. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Point" && *defined == "Y"),
            "Should find Point defines Y. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Point" && *defined == "Distance"),
            "Should find Point defines Distance. Found: {defines:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_record_members() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
record Person(string Name, int Age) {
    public string GetGreeting() {
        return "Hello";
    }
}
"#;

        let defines = parser.find_defines(code);

        // Should find record defines method (parameters are not tracked as fields in this context)
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Person" && *defined == "GetGreeting"),
            "Should find Person defines GetGreeting. Found: {defines:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_constructor() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class UserService {
    public UserService(IRepository repo) {
        var initialized = true;
    }
}
"#;

        let defines = parser.find_defines(code);

        // Should find class defines constructor
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService"
                    && *defined == "UserService"),
            "Should find UserService defines UserService constructor. Found: {defines:?}"
        );

        // Constructor should also define local variables
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService"
                    && *defined == "initialized"),
            "Should find UserService constructor defines initialized. Found: {defines:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_multiple_variables() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
class DataService {
    private int _count, _total, _average;

    public void Calculate() {
        var x = 1, y = 2, z = 3;
    }
}
"#;

        let defines = parser.find_defines(code);

        // Should find all field declarations
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "DataService" && *defined == "_count"),
            "Should find DataService defines _count. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "DataService" && *defined == "_total"),
            "Should find DataService defines _total. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "DataService" && *defined == "_average"),
            "Should find DataService defines _average. Found: {defines:?}"
        );

        // Should find all local variable declarations
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Calculate" && *defined == "x"),
            "Should find Calculate defines x. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Calculate" && *defined == "y"),
            "Should find Calculate defines y. Found: {defines:?}"
        );
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Calculate" && *defined == "z"),
            "Should find Calculate defines z. Found: {defines:?}"
        );
    }

    #[test]
    fn test_csharp_find_defines_complex_scenario() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
interface IRepository {
    User Get(int id);
}

class UserRepository : IRepository {
    private DbContext _context;
    public Logger Logger { get; set; }
    public event EventHandler DataChanged;

    public UserRepository(DbContext context) {
        _context = context;
    }

    public User Get(int id) {
        var result = _context.Users.Find(id);
        return result;
    }
}
"#;

        let defines = parser.find_defines(code);

        // Interface defines method
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "IRepository" && *defined == "Get"),
            "Should find IRepository defines Get. Found: {defines:?}"
        );

        // Class defines field
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserRepository"
                    && *defined == "_context"),
            "Should find UserRepository defines _context. Found: {defines:?}"
        );

        // Class defines property
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserRepository" && *defined == "Logger"),
            "Should find UserRepository defines Logger. Found: {defines:?}"
        );

        // Class defines event
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserRepository"
                    && *defined == "DataChanged"),
            "Should find UserRepository defines DataChanged. Found: {defines:?}"
        );

        // Class defines constructor
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserRepository"
                    && *defined == "UserRepository"),
            "Should find UserRepository defines constructor. Found: {defines:?}"
        );

        // Class defines method
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserRepository" && *defined == "Get"),
            "Should find UserRepository defines Get. Found: {defines:?}"
        );

        // Method defines local variable
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Get" && *defined == "result"),
            "Should find Get defines result. Found: {defines:?}"
        );
    }
}

#[cfg(test)]
mod debug {
    use super::*;

    #[test]
    #[ignore]
    fn debug_pattern_ast() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
public void Test(object obj) {
    if (obj is string s) {
        Console.WriteLine(s);
    }
}
"#;

        let tree = parser.parser.parse(code, None).unwrap();
        let root = tree.root_node();
        print_tree(&root, code, 0);
    }

    fn print_tree(node: &tree_sitter::Node, code: &str, depth: usize) {
        let indent = "  ".repeat(depth);
        println!("{indent}kind: {}", node.kind());

        if node.kind() == "is_pattern_expression" {
            println!("{indent}  *** FOUND IS_PATTERN_EXPRESSION ***");
            println!("{indent}  text: {}", &code[node.byte_range()]);

            // Check available fields
            if let Some(left) = node.child_by_field_name("left") {
                println!(
                    "{indent}  left: {} '{}'",
                    left.kind(),
                    &code[left.byte_range()]
                );
            }
            if let Some(pattern) = node.child_by_field_name("pattern") {
                println!(
                    "{indent}  pattern: {} '{}'",
                    pattern.kind(),
                    &code[pattern.byte_range()]
                );
            }
            if let Some(right) = node.child_by_field_name("right") {
                println!(
                    "{indent}  right: {} '{}'",
                    right.kind(),
                    &code[right.byte_range()]
                );
            }

            // Print all children
            let mut cursor = node.walk();
            for (i, child) in node.children(&mut cursor).enumerate() {
                println!(
                    "{indent}  child[{i}]: {} '{}'",
                    child.kind(),
                    &code[child.byte_range()]
                );
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            print_tree(&child, code, depth + 1);
        }
    }
}
