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
        let base_sig = self.extract_signature_excluding_body(node, code, "class_body");
        let constraints = self.extract_generic_constraints(node, code);
        format!("{base_sig}{constraints}")
    }

    /// Extract interface signature
    fn extract_interface_signature(&self, node: Node, code: &str) -> String {
        let base_sig = self.extract_signature_excluding_body(node, code, "interface_body");
        let constraints = self.extract_generic_constraints(node, code);
        format!("{base_sig}{constraints}")
    }

    /// Extract struct signature
    fn extract_struct_signature(&self, node: Node, code: &str) -> String {
        let base_sig = self.extract_signature_excluding_body(node, code, "struct_body");
        let constraints = self.extract_generic_constraints(node, code);
        format!("{base_sig}{constraints}")
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
        let base_sig = self.extract_signature_excluding_body(node, code, "method_body");
        let constraints = self.extract_generic_constraints(node, code);
        format!("{base_sig}{constraints}")
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
    ///
    /// Supports:
    /// - Basic using: `using System;`
    /// - Static using: `using static System.Math;`
    /// - Global using: `global using System;`
    /// - Alias using: `using Json = System.Text.Json;`
    fn extract_imports_from_node(
        node: Node,
        code: &str,
        file_id: FileId,
        imports: &mut Vec<Import>,
    ) {
        match node.kind() {
            "using_directive" => {
                // Check for alias (using Alias = Namespace.Type)
                let mut alias: Option<String> = None;
                let mut import_path = String::new();
                let mut is_static = false;
                let full_text = code[node.byte_range()].trim();

                // Check if it's a static using
                if full_text.contains("using static ") {
                    is_static = true;
                }

                // Check for alias pattern using text parsing as fallback
                // Pattern: using [global] [static] Alias = Namespace.Type;
                if full_text.contains(" = ") {
                    // This is an alias using
                    let parts: Vec<&str> = full_text.split(" = ").collect();
                    if parts.len() == 2 {
                        // Extract alias (after "using" keyword)
                        let alias_part = parts[0].trim();
                        // Remove "using", "global", "static" keywords
                        let alias_name = alias_part
                            .replace("using", "")
                            .replace("global", "")
                            .replace("static", "")
                            .trim()
                            .to_string();

                        if !alias_name.is_empty() {
                            alias = Some(alias_name);
                        }

                        // Extract import path (before semicolon)
                        import_path = parts[1].trim().trim_end_matches(';').trim().to_string();
                    }
                } else {
                    // Not an alias - try standard field extraction
                    if let Some(name_node) = node.child_by_field_name("name") {
                        import_path = code[name_node.byte_range()].to_string();
                    } else {
                        // Fallback: tree-sitter-c-sharp doesn't consistently expose "name" field
                        // for using_directive nodes. Iterate child nodes to find qualified_name
                        // or identifier nodes.
                        let mut cursor = node.walk();
                        for child in node.children(&mut cursor) {
                            if child.kind() == "qualified_name" || child.kind() == "identifier" {
                                import_path = code[child.byte_range()].to_string();
                                break;
                            }
                        }
                    }
                }

                if !import_path.is_empty() {
                    // Add prefix for static usings to distinguish them
                    if is_static {
                        import_path = format!("static {import_path}");
                    }

                    imports.push(Import {
                        path: import_path,
                        alias,
                        file_id,
                        is_glob: false,
                        is_type_only: false,
                    });
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
                    // In tree-sitter C#, object_creation_expression is a direct child of variable_declarator
                    // Example structure: variable_declarator -> [identifier, "=", object_creation_expression]
                    let mut init_expr = None;
                    let mut sub_cursor = child.walk();
                    for vchild in child.children(&mut sub_cursor) {
                        if vchild.kind() == "object_creation_expression" {
                            init_expr = Some(vchild);
                            break;
                        }
                    }

                    if let Some(init_node) = init_expr {
                        // Found a "new Type()" expression - extract the type
                        if let Some(type_name) =
                            self.extract_type_from_initializer(&init_node, code)
                        {
                            let range = Range::new(
                                child.start_position().row as u32,
                                child.start_position().column as u16,
                                child.end_position().row as u32,
                                child.end_position().column as u16,
                            );
                            bindings.push((var_name, type_name, range));
                            continue; // Successfully extracted, move to next variable
                        }
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

    /// Extract type name from an initializer expression
    ///
    /// Handles:
    /// - `new Type()` → Some("Type")
    /// - `new Generic<T>()` → Some("Generic")
    /// - `new Namespace.Type()` → Some("Type")
    fn extract_type_from_initializer<'a>(
        &self,
        init_node: &Node,
        code: &'a str,
    ) -> Option<&'a str> {
        // Look for object_creation_expression
        if init_node.kind() == "object_creation_expression" {
            // object_creation_expression has a 'type' field
            if let Some(type_node) = init_node.child_by_field_name("type") {
                return Some(self.extract_simple_type_name(&type_node, code));
            }
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

    /// Extract generic constraints from a node (class, interface, struct, or method)
    ///
    /// Looks for type_parameter_constraints_clause children and formats them
    /// into a where clause string.
    ///
    /// Returns: " where T : IEntity, new()" or empty string if no constraints
    fn extract_generic_constraints(&self, node: Node, code: &str) -> String {
        let mut constraints = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "type_parameter_constraints_clause" {
                // Structure: type_parameter_constraints_clause has:
                // - identifier (the type parameter, e.g., "T")
                // - one or more type_constraint nodes

                let mut type_param = None;
                let mut constraint_parts = Vec::new();

                let mut constraint_cursor = child.walk();
                for constraint_child in child.children(&mut constraint_cursor) {
                    match constraint_child.kind() {
                        "identifier" if type_param.is_none() => {
                            type_param = Some(&code[constraint_child.byte_range()]);
                        }
                        "type_constraint"
                        | "class_constraint"
                        | "struct_constraint"
                        | "constructor_constraint"
                        | "notnull_constraint" => {
                            constraint_parts.push(&code[constraint_child.byte_range()]);
                        }
                        _ => {}
                    }
                }

                if let Some(param) = type_param {
                    if !constraint_parts.is_empty() {
                        constraints.push(format!("{} : {}", param, constraint_parts.join(", ")));
                    }
                }
            }
        }

        if constraints.is_empty() {
            String::new()
        } else {
            format!(" where {}", constraints.join(" where "))
        }
    }

    /// Extract type name including nullable marker
    ///
    /// Returns "string?" if the type is nullable, "string" otherwise
    #[allow(dead_code)]
    fn extract_type_with_nullable(&self, type_node: Node, code: &str) -> String {
        match type_node.kind() {
            "nullable_type" => {
                // nullable_type wraps the actual type and adds "?"
                // Get the inner type and append "?"
                let mut cursor = type_node.walk();
                for child in type_node.children(&mut cursor) {
                    if child.kind() != "?" {
                        return format!("{}?", &code[child.byte_range()]);
                    }
                }
                // Fallback: just get the whole text
                code[type_node.byte_range()].to_string()
            }
            "predefined_type" | "identifier_name" | "generic_name" | "qualified_name" => {
                // Non-nullable type
                code[type_node.byte_range()].to_string()
            }
            _ => {
                // Unknown type node, get text as-is
                code[type_node.byte_range()].to_string()
            }
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
                } else if modifier_text.contains("file") {
                    // File-scoped types (C# 11) - only visible in current file
                    return Visibility::Private; // Map to Private as closest match
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

    fn find_uses<'a>(&mut self, _code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        // TODO: Implement proper type usage tracking for C#
        //
        // This should track where types are referenced/used, for example:
        // - Method parameter types: `void DoWork(Helper helper)`
        // - Return types: `Helper GetHelper()`
        // - Field/property types: `private Helper _helper;`
        // - Base classes/interfaces: `class Foo : IBar`
        //
        // Previously disabled because it was creating invalid relationships with "use" as context
        // instead of proper semantic relationships.
        //
        // Implementation approach:
        // 1. Traverse AST looking for type references
        // 2. Extract (user_symbol, used_type, range) tuples
        // 3. Filter out primitive types (int, string, etc.)
        // 4. Ensure proper context (method names, not node kinds)
        Vec::new()
    }

    fn find_defines<'a>(&mut self, _code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        // TODO: Implement proper defines tracking for C#
        //
        // This should track definition relationships, for example:
        // - Variable definitions: `var x = 5;` → (containing_method, "x", range)
        // - Field definitions: `private int _count;` → (containing_class, "_count", range)
        // - Property definitions: `public string Name { get; set; }` → (containing_class, "Name", range)
        //
        // Previously disabled because it was using node.kind() instead of actual definer names,
        // creating relationships like "variable_declaration defines x" instead of "Method defines x".
        //
        // Implementation approach:
        // 1. Track current scope context (class, method, etc.)
        // 2. For each definition, extract (definer_name, defined_symbol, range)
        // 3. Ensure definer_name is the actual symbol name, not AST node type
        Vec::new()
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
    fn test_csharp_static_using() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            using static System.Math;
            using static System.Console;

            namespace TestNamespace {
                public class Calculator {
                    public double Calculate() {
                        return Sqrt(16); // Using static Math.Sqrt
                    }
                }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let imports = parser.find_imports(code, file_id);

        assert!(
            imports.len() >= 2,
            "Should find at least 2 static imports, found: {}",
            imports.len()
        );
        assert!(
            imports.iter().any(|i| i.path == "static System.Math"),
            "Should find static System.Math import"
        );
        assert!(
            imports.iter().any(|i| i.path == "static System.Console"),
            "Should find static System.Console import"
        );
    }

    #[test]
    fn test_csharp_using_alias() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            using Json = System.Text.Json;
            using Http = System.Net.Http;

            namespace TestNamespace {
                public class ApiClient {
                    private Http.HttpClient client;
                }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let imports = parser.find_imports(code, file_id);

        assert!(
            imports.len() >= 2,
            "Should find at least 2 alias imports, found: {}",
            imports.len()
        );

        let json_import = imports
            .iter()
            .find(|i| i.path == "System.Text.Json")
            .expect("Should find System.Text.Json import");
        assert_eq!(
            json_import.alias.as_deref(),
            Some("Json"),
            "Should have Json alias"
        );

        let http_import = imports
            .iter()
            .find(|i| i.path == "System.Net.Http")
            .expect("Should find System.Net.Http import");
        assert_eq!(
            http_import.alias.as_deref(),
            Some("Http"),
            "Should have Http alias"
        );
    }

    #[test]
    fn test_csharp_global_using() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            global using System;
            global using System.Linq;

            namespace TestNamespace {
                public class TestClass { }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let imports = parser.find_imports(code, file_id);

        // Global usings should be extracted just like regular usings
        assert!(
            imports.len() >= 2,
            "Should find at least 2 global imports, found: {}",
            imports.len()
        );
        assert!(
            imports.iter().any(|i| i.path == "System"),
            "Should find System import"
        );
        assert!(
            imports.iter().any(|i| i.path == "System.Linq"),
            "Should find System.Linq import"
        );
    }

    #[test]
    fn test_csharp_mixed_using_directives() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            using System;
            using static System.Math;
            using Json = System.Text.Json;
            global using System.Linq;

            namespace TestNamespace {
                public class MixedUsings { }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let imports = parser.find_imports(code, file_id);

        assert!(
            imports.len() >= 4,
            "Should find at least 4 imports, found: {}",
            imports.len()
        );

        // Regular using
        assert!(imports.iter().any(|i| i.path == "System"));

        // Static using
        assert!(imports.iter().any(|i| i.path == "static System.Math"));

        // Alias using
        assert!(
            imports
                .iter()
                .any(|i| { i.path == "System.Text.Json" && i.alias.as_deref() == Some("Json") })
        );

        // Global using
        assert!(imports.iter().any(|i| i.path == "System.Linq"));
    }

    // === Generic Constraints Tests ===

    #[test]
    fn test_generic_class_with_single_constraint() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class Repository<T> where T : IEntity
            {
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let class_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Repository")
            .expect("Should find Repository class");

        assert!(
            class_symbol
                .signature
                .as_ref()
                .unwrap()
                .contains("where T : IEntity"),
            "Signature should include constraint clause. Got: {}",
            class_symbol.signature.as_ref().unwrap()
        );
    }

    #[test]
    fn test_generic_class_with_multiple_constraints() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class Repository<T> where T : IEntity, new()
            {
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let class_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Repository")
            .expect("Should find Repository class");

        let sig = class_symbol.signature.as_ref().unwrap();
        assert!(sig.contains("where T : IEntity"));
        assert!(sig.contains("new()"));
    }

    #[test]
    fn test_generic_class_with_multiple_type_parameters() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class Cache<TKey, TValue>
                where TKey : notnull
                where TValue : class, ISerializable
            {
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let class_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Cache")
            .expect("Should find Cache class");

        let sig = class_symbol.signature.as_ref().unwrap();
        assert!(sig.contains("where TKey : notnull"));
        assert!(sig.contains("where TValue : class"));
    }

    #[test]
    fn test_generic_method_with_constraints() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class Repository<T>
            {
                public void Add<U>(U item) where U : T, IComparable<U>
                {
                }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let method_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Add")
            .expect("Should find Add method");

        let sig = method_symbol.signature.as_ref().unwrap();
        assert!(sig.contains("where U : T, IComparable<U>"));
    }

    #[test]
    fn test_struct_with_constraints() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public struct ValueWrapper<T> where T : struct
            {
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let struct_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "ValueWrapper")
            .expect("Should find ValueWrapper struct");

        assert!(
            struct_symbol
                .signature
                .as_ref()
                .unwrap()
                .contains("where T : struct")
        );
    }

    #[test]
    fn test_interface_with_constraints() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public interface IRepository<T> where T : IEntity
            {
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let interface_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "IRepository")
            .expect("Should find IRepository interface");

        assert!(
            interface_symbol
                .signature
                .as_ref()
                .unwrap()
                .contains("where T : IEntity")
        );
    }

    // === File-scoped Types Tests (C# 11) ===

    #[test]
    fn test_file_scoped_class() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            file class InternalHelper
            {
                public void Process() { }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let class_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "InternalHelper")
            .expect("Should find InternalHelper class");

        // File-scoped types should have Private visibility (file-level)
        assert_eq!(
            class_symbol.visibility,
            Visibility::Private,
            "File-scoped class should have Private visibility"
        );

        // Signature should include "file" modifier
        assert!(
            class_symbol
                .signature
                .as_ref()
                .unwrap()
                .contains("file class"),
            "Signature should include 'file class'. Got: {}",
            class_symbol.signature.as_ref().unwrap()
        );
    }

    #[test]
    fn test_file_scoped_interface() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            file interface IInternalService
            {
                void Execute();
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let interface_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "IInternalService")
            .expect("Should find IInternalService interface");

        assert_eq!(
            interface_symbol.visibility,
            Visibility::Private,
            "File-scoped interface should have Private visibility"
        );

        assert!(
            interface_symbol
                .signature
                .as_ref()
                .unwrap()
                .contains("file interface"),
            "Signature should include 'file interface'"
        );
    }

    #[test]
    fn test_file_scoped_struct() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            file struct Point
            {
                public int X;
                public int Y;
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let struct_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Point" && s.kind == SymbolKind::Struct)
            .expect("Should find Point struct");

        assert_eq!(
            struct_symbol.visibility,
            Visibility::Private,
            "File-scoped struct should have Private visibility"
        );

        assert!(
            struct_symbol
                .signature
                .as_ref()
                .unwrap()
                .contains("file struct"),
            "Signature should include 'file struct'"
        );
    }

    #[test]
    fn test_file_scoped_record() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            file record InternalData(string Name, int Value);
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let record_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "InternalData")
            .expect("Should find InternalData record");

        assert_eq!(
            record_symbol.visibility,
            Visibility::Private,
            "File-scoped record should have Private visibility"
        );

        assert!(
            record_symbol
                .signature
                .as_ref()
                .unwrap()
                .contains("file record"),
            "Signature should include 'file record'"
        );
    }

    #[test]
    fn test_mixed_file_and_public_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class PublicService { }
            file class InternalHelper { }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let public_class = symbols
            .iter()
            .find(|s| s.name.as_ref() == "PublicService")
            .expect("Should find PublicService");

        let file_class = symbols
            .iter()
            .find(|s| s.name.as_ref() == "InternalHelper")
            .expect("Should find InternalHelper");

        assert_eq!(
            public_class.visibility,
            Visibility::Public,
            "PublicService should be Public"
        );

        assert_eq!(
            file_class.visibility,
            Visibility::Private,
            "InternalHelper should be Private (file-scoped)"
        );
    }

    // === Nullable Reference Types Tests ===

    #[test]
    fn test_nullable_property() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class User
            {
                public string? Email { get; set; }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let prop_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Email")
            .expect("Should find Email property");

        assert!(
            prop_symbol.signature.as_ref().unwrap().contains("string?"),
            "Property signature should show nullable type. Got: {}",
            prop_symbol.signature.as_ref().unwrap()
        );
    }

    #[test]
    fn test_non_nullable_property() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class User
            {
                public string Name { get; set; }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let prop_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Name")
            .expect("Should find Name property");

        let sig = prop_symbol.signature.as_ref().unwrap();
        assert!(sig.contains("string"));
        assert!(!sig.contains("string?"), "Should NOT be nullable");
    }

    #[test]
    fn test_nullable_method_return_type() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class UserService
            {
                public string? FindEmail(int userId)
                {
                    return null;
                }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let method_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "FindEmail")
            .expect("Should find FindEmail method");

        assert!(
            method_symbol
                .signature
                .as_ref()
                .unwrap()
                .contains("string?"),
            "Method return type should be nullable. Got: {}",
            method_symbol.signature.as_ref().unwrap()
        );
    }

    #[test]
    fn test_nullable_method_parameters() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class UserService
            {
                public string? ProcessEmail(string? input)
                {
                    return input?.Trim();
                }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let method_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "ProcessEmail")
            .expect("Should find ProcessEmail method");

        let sig = method_symbol.signature.as_ref().unwrap();
        // Both return type and parameter should be nullable
        assert!(sig.contains("string? ProcessEmail") || sig.contains("string?"));
        assert!(sig.contains("string? input") || sig.contains("(string? input)"));
    }

    #[test]
    fn test_nullable_generic_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class DataService
            {
                public List<string>? GetNames()
                {
                    return null;
                }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let method_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "GetNames")
            .expect("Should find GetNames method");

        assert!(
            method_symbol
                .signature
                .as_ref()
                .unwrap()
                .contains("List<string>?"),
            "Generic type should be nullable. Got: {}",
            method_symbol.signature.as_ref().unwrap()
        );
    }

    #[test]
    fn test_mixed_nullable_and_non_nullable_parameters() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class UserService
            {
                public void Update(int id, string name, string? email)
                {
                }
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let method_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Update")
            .expect("Should find Update method");

        let sig = method_symbol.signature.as_ref().unwrap();
        // id is int (value type, can't be nullable in this context)
        assert!(sig.contains("int id"));
        // name is non-nullable reference type
        assert!(sig.contains("string name"));
        // email is nullable reference type
        assert!(sig.contains("string? email"));
    }

    #[test]
    fn test_nullable_field() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class User
            {
                private string? _cachedEmail;
            }
        "#;

        let file_id = FileId::new(1).unwrap();
        let mut counter = SymbolCounter::new();
        let symbols = parser.parse(code, file_id, &mut counter);

        let field_symbol = symbols
            .iter()
            .find(|s| s.name.as_ref() == "_cachedEmail")
            .expect("Should find _cachedEmail field");

        assert!(
            field_symbol.signature.as_ref().unwrap().contains("string?"),
            "Field signature should show nullable type"
        );
    }
}
