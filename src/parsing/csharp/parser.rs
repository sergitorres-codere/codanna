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

        // Handle await expressions - unwrap to find the actual invocation
        // Pattern: await _httpClient.GetAsync()
        // The await wraps the invocation, so we need to unwrap it transparently
        if node.kind() == "await_expression" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                // Process children with inherited function context
                Self::extract_calls_recursive(&child, code, function_context, calls);
            }
            return; // Don't fall through to default recursion
        }

        // Handle object creation (constructor calls)
        // Pattern: new Service() or new List<T>()
        if node.kind() == "object_creation_expression" {
            let caller = function_context.unwrap_or("");

            if let Some(type_node) = node.child_by_field_name("type") {
                let type_name = match type_node.kind() {
                    "identifier" => &code[type_node.byte_range()],
                    "generic_name" => {
                        // For Generic<T>, extract just "Generic"
                        type_node
                            .child_by_field_name("name")
                            .map(|n| &code[n.byte_range()])
                            .unwrap_or(&code[type_node.byte_range()])
                    }
                    "qualified_name" => {
                        // For Namespace.Type, extract just "Type" (rightmost identifier)
                        let mut last_ident = None;
                        let mut cursor = type_node.walk();
                        for child in type_node.children(&mut cursor) {
                            if child.kind() == "identifier" {
                                last_ident = Some(&code[child.byte_range()]);
                            }
                        }
                        last_ident.unwrap_or(&code[type_node.byte_range()])
                    }
                    _ => &code[type_node.byte_range()],
                };

                let range = Range::new(
                    node.start_position().row as u32,
                    node.start_position().column as u16,
                    node.end_position().row as u32,
                    node.end_position().column as u16,
                );
                calls.push((caller, type_name, range));
            }

            // Still recurse to catch any method calls in constructor arguments
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                Self::extract_calls_recursive(&child, code, function_context, calls);
            }
            return;
        }

        // Handle invocation expressions with proper caller context
        if node.kind() == "invocation_expression" {
            if let Some(expression_node) = node.child(0) {
                let caller = function_context.unwrap_or("");
                let callee_raw = match expression_node.kind() {
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

                // Strip generic type parameters for better resolution
                // e.g., "GetFromJsonAsync<List<T>>" -> "GetFromJsonAsync"
                let callee = if let Some(idx) = callee_raw.find('<') {
                    &callee_raw[..idx]
                } else {
                    callee_raw
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
            // Handle await expressions - unwrap to find the actual invocation
            "await_expression" => {
                // Await wraps invocation, process children transparently
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_method_calls_from_node(child, code, method_calls);
                }
                // Don't fall through to default case
            }
            // Handle object creation (constructor calls)
            "object_creation_expression" => {
                let caller = self
                    .context
                    .current_function()
                    .or_else(|| self.context.current_class())
                    .unwrap_or("unknown");

                if let Some(type_node) = node.child_by_field_name("type") {
                    let type_name = match type_node.kind() {
                        "identifier" => code[type_node.byte_range()].to_string(),
                        "generic_name" => type_node
                            .child_by_field_name("name")
                            .map(|n| code[n.byte_range()].to_string())
                            .unwrap_or_else(|| code[type_node.byte_range()].to_string()),
                        _ => code[type_node.byte_range()].to_string(),
                    };

                    let range = Range::new(
                        node.start_position().row as u32,
                        node.start_position().column as u16,
                        node.end_position().row as u32,
                        node.end_position().column as u16,
                    );
                    method_calls.push(MethodCall::new(caller, &type_name, range));
                }

                // Still recurse for nested calls in constructor arguments
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.extract_method_calls_from_node(child, code, method_calls);
                }
                // Don't fall through to default case
            }
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

                                    // Strip generic type parameters for better resolution
                                    // e.g., "GetFromJsonAsync<List<T>>" -> "GetFromJsonAsync"
                                    let method_name = if let Some(idx) = method.find('<') {
                                        &method[..idx]
                                    } else {
                                        &method
                                    };

                                    let range = Range::new(
                                        node.start_position().row as u32,
                                        node.start_position().column as u16,
                                        node.end_position().row as u32,
                                        node.end_position().column as u16,
                                    );

                                    // Detect static method calls: if receiver starts with uppercase,
                                    // it's likely a class/type name (PascalCase convention in C#)
                                    let mut call = MethodCall::new(caller, method_name, range)
                                        .with_receiver(&receiver);

                                    if receiver.chars().next().is_some_and(|c| c.is_uppercase()) {
                                        call = call.static_method();
                                    }

                                    method_calls.push(call);
                                }
                            }
                        }
                        "identifier" => {
                            // Simple method calls like Method()
                            let method = code[expression_node.byte_range()].to_string();

                            // Strip generic type parameters for better resolution
                            let method_name = if let Some(idx) = method.find('<') {
                                &method[..idx]
                            } else {
                                &method
                            };

                            let range = Range::new(
                                node.start_position().row as u32,
                                node.start_position().column as u16,
                                node.end_position().row as u32,
                                node.end_position().column as u16,
                            );
                            method_calls.push(
                                MethodCall::new(caller, method_name, range).with_receiver("this"),
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

    /// Extract type uses recursively from C# AST nodes
    ///
    /// This method traverses the AST and identifies where types are referenced/used, tracking:
    /// - Method parameter types
    /// - Return types
    /// - Field/property types
    /// - Base classes
    /// - Interface implementations
    /// - Generic type arguments
    ///
    /// Returns tuples of (context_name, type_name, range) where context_name is the
    /// method/class/variable that uses the type.
    fn extract_type_uses_recursive<'a>(
        &self,
        node: &Node,
        code: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        match node.kind() {
            // Method declarations - track parameter and return types
            "method_declaration" | "constructor_declaration" => {
                let context_name = node
                    .child_by_field_name("name")
                    .map(|n| &code[n.byte_range()])
                    .unwrap_or("anonymous");

                // Extract parameter types
                let params_opt = node.child_by_field_name("parameters");
                if let Some(params) = params_opt {
                    self.extract_parameter_types(&params, code, context_name, uses);
                }

                // Extract return type (methods only, not constructors)
                if node.kind() == "method_declaration" {
                    // Try multiple field names that tree-sitter might use
                    let return_type = node.child_by_field_name("type")
                        .or_else(|| node.child_by_field_name("return_type"))
                        .or_else(|| {
                            // Fallback: find first type node before parameters
                            let params_pos = params_opt.map(|p| p.start_position()).unwrap_or(node.end_position());
                            let mut cursor = node.walk();
                            for child in node.children(&mut cursor) {
                                if child.start_position() < params_pos {
                                    if matches!(child.kind(), "identifier" | "generic_name" | "qualified_name" | "predefined_type" | "array_type" | "nullable_type") {
                                        return Some(child);
                                    }
                                }
                            }
                            None
                        });

                    if let Some(return_type) = return_type {
                        self.extract_type_from_node(&return_type, code, context_name, uses);
                    }
                }
            }

            // Class/struct/record declarations - track base classes and interfaces
            "class_declaration" | "struct_declaration" | "record_declaration" => {
                let class_name = node
                    .child_by_field_name("name")
                    .map(|n| &code[n.byte_range()])
                    .unwrap_or("anonymous");

                // Check for base_list which contains base classes and interfaces
                if let Some(base_list) = node.child_by_field_name("bases") {
                    self.extract_base_types(&base_list, code, class_name, uses);
                } else {
                    // Fallback: search for base_list as a child
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "base_list" {
                            self.extract_base_types(&child, code, class_name, uses);
                            break;
                        }
                    }
                }

                // Extract field and property types from class body
                if let Some(body) = node.child_by_field_name("body") {
                    self.extract_member_types(&body, code, class_name, uses);
                }
            }

            // Interface declarations - track extends
            "interface_declaration" => {
                let interface_name = node
                    .child_by_field_name("name")
                    .map(|n| &code[n.byte_range()])
                    .unwrap_or("anonymous");

                // Check for base_list
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "base_list" {
                        self.extract_base_types(&child, code, interface_name, uses);
                        break;
                    }
                }
            }

            // Note: field_declaration and property_declaration are handled via
            // extract_member_types() when inside a class/struct, so we don't
            // process them here to avoid duplicates

            // Delegate declarations - track parameter and return types
            "delegate_declaration" => {
                let delegate_name = node
                    .child_by_field_name("name")
                    .map(|n| &code[n.byte_range()])
                    .unwrap_or("anonymous");

                if let Some(params) = node.child_by_field_name("parameters") {
                    self.extract_parameter_types(&params, code, delegate_name, uses);
                }

                if let Some(return_type) = node.child_by_field_name("type") {
                    self.extract_type_from_node(&return_type, code, delegate_name, uses);
                }
            }

            _ => {}
        }

        // Recurse to children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_type_uses_recursive(&child, code, uses);
        }
    }

    /// Extract parameter types from a parameter list
    fn extract_parameter_types<'a>(
        &self,
        params_node: &Node,
        code: &'a str,
        context: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "parameter" {
                // Extract type from parameter
                if let Some(type_node) = child.child_by_field_name("type") {
                    self.extract_type_from_node(&type_node, code, context, uses);
                }
            }
        }
    }

    /// Extract type from a type node, handling various C# type syntaxes
    fn extract_type_from_node<'a>(
        &self,
        type_node: &Node,
        code: &'a str,
        context: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let type_name = self.extract_simple_type_name_for_uses(type_node, code);

        // Filter out primitive types
        if !Self::is_primitive_type(type_name) {
            let range = Range::new(
                type_node.start_position().row as u32,
                type_node.start_position().column as u16,
                type_node.end_position().row as u32,
                type_node.end_position().column as u16,
            );
            uses.push((context, type_name, range));
        }

        // Also extract generic type arguments if present
        if type_node.kind() == "generic_name" {
            // Try to find type_argument_list as a child (field name might vary)
            let mut cursor = type_node.walk();
            for child in type_node.children(&mut cursor) {
                if child.kind() == "type_argument_list" {
                    self.extract_generic_type_arguments(&child, code, context, uses);
                    break;
                }
            }
        }
    }

    /// Extract simple type name from a type node for uses tracking
    fn extract_simple_type_name_for_uses<'a>(&self, type_node: &Node, code: &'a str) -> &'a str {
        match type_node.kind() {
            "identifier" => &code[type_node.byte_range()],
            "generic_name" => {
                // For generic types, extract just the base name (e.g., "List" from "List<T>")
                // Try to find the identifier child (first child is usually the base type name)
                let mut cursor = type_node.walk();
                for child in type_node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        return &code[child.byte_range()];
                    }
                }

                // Fallback: parse string to extract part before '<'
                let full_text = &code[type_node.byte_range()];
                if let Some(bracket_pos) = full_text.find('<') {
                    &full_text[..bracket_pos]
                } else {
                    full_text
                }
            }
            "qualified_name" => {
                // For qualified names, take the last identifier (e.g., "Helper" from "MyApp.Helper")
                let mut last_ident = None;
                let mut cursor = type_node.walk();
                for child in type_node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        last_ident = Some(&code[child.byte_range()]);
                    }
                }
                last_ident.unwrap_or(&code[type_node.byte_range()])
            }
            "predefined_type" => &code[type_node.byte_range()],
            "array_type" => {
                // For arrays, extract the element type
                if let Some(element_type) = type_node.child_by_field_name("type") {
                    self.extract_simple_type_name_for_uses(&element_type, code)
                } else {
                    &code[type_node.byte_range()]
                }
            }
            "nullable_type" => {
                // For nullable types, extract the underlying type
                if let Some(underlying_type) = type_node.child_by_field_name("type") {
                    self.extract_simple_type_name_for_uses(&underlying_type, code)
                } else {
                    &code[type_node.byte_range()]
                }
            }
            _ => &code[type_node.byte_range()],
        }
    }

    /// Check if a type name is a C# primitive type
    fn is_primitive_type(type_name: &str) -> bool {
        matches!(
            type_name,
            "void"
                | "int"
                | "string"
                | "bool"
                | "float"
                | "double"
                | "decimal"
                | "byte"
                | "sbyte"
                | "short"
                | "ushort"
                | "uint"
                | "long"
                | "ulong"
                | "char"
                | "object"
                | "dynamic"
                | "var"
        )
    }

    /// Extract types from base_list (base classes and interfaces)
    fn extract_base_types<'a>(
        &self,
        base_list: &Node,
        code: &'a str,
        context: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let mut cursor = base_list.walk();
        for child in base_list.children(&mut cursor) {
            match child.kind() {
                "identifier" | "generic_name" | "qualified_name" => {
                    self.extract_type_from_node(&child, code, context, uses);
                }
                _ => {}
            }
        }
    }

    /// Extract types from generic type arguments
    fn extract_generic_type_arguments<'a>(
        &self,
        type_args: &Node,
        code: &'a str,
        context: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let mut cursor = type_args.walk();
        for child in type_args.children(&mut cursor) {
            match child.kind() {
                "identifier" | "generic_name" | "qualified_name" | "predefined_type" => {
                    self.extract_type_from_node(&child, code, context, uses);
                }
                "type_argument_list" => {
                    // Recurse into nested type argument lists
                    self.extract_generic_type_arguments(&child, code, context, uses);
                }
                _ => {}
            }
        }
    }

    /// Extract types from class/struct members (fields, properties)
    fn extract_member_types<'a>(
        &self,
        body: &Node,
        code: &'a str,
        _class_name: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "field_declaration" => {
                    self.extract_field_types(&child, code, uses);
                }
                "property_declaration" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let prop_name = &code[name_node.byte_range()];
                        if let Some(type_node) = child.child_by_field_name("type") {
                            self.extract_type_from_node(&type_node, code, prop_name, uses);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Extract types from field declarations
    fn extract_field_types<'a>(
        &self,
        field_node: &Node,
        code: &'a str,
        uses: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Field declarations have a variable_declaration child
        let mut cursor = field_node.walk();
        for child in field_node.children(&mut cursor) {
            if child.kind() == "variable_declaration" {
                // Get the type from the variable declaration
                if let Some(type_node) = child.child_by_field_name("type") {
                    // Get field names from variable_declarator children
                    let mut var_cursor = child.walk();
                    for var_child in child.children(&mut var_cursor) {
                        if var_child.kind() == "variable_declarator" {
                            if let Some(name_node) = var_child.child_by_field_name("name") {
                                let field_name = &code[name_node.byte_range()];
                                self.extract_type_from_node(&type_node, code, field_name, uses);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Extract definition relationships recursively from C# AST nodes
    ///
    /// This method traverses the AST and identifies containment/definition relationships:
    /// - Classes define methods, fields, properties, and nested types
    /// - Methods define local variables and local functions
    /// - Interfaces define method signatures
    ///
    /// Returns tuples of (definer_name, defined_symbol, range) where definer_name is the
    /// class/method/interface that defines the symbol.
    fn extract_defines_recursive<'a>(
        &self,
        node: &Node,
        code: &'a str,
        current_class: Option<&'a str>,
        current_method: Option<&'a str>,
        defines: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        match node.kind() {
            // Class declarations - track nested types, methods, fields, properties
            "class_declaration" | "struct_declaration" | "record_declaration" => {
                let class_name = node
                    .child_by_field_name("name")
                    .map(|n| &code[n.byte_range()])
                    .unwrap_or("anonymous");

                // Process class body
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        match child.kind() {
                            // Methods defined by class
                            "method_declaration" | "constructor_declaration" => {
                                if let Some(name_node) = child.child_by_field_name("name") {
                                    let method_name = &code[name_node.byte_range()];
                                    let range = Range::new(
                                        child.start_position().row as u32,
                                        child.start_position().column as u16,
                                        child.end_position().row as u32,
                                        child.end_position().column as u16,
                                    );
                                    defines.push((class_name, method_name, range));

                                    // Recurse into method body to find local definitions
                                    if let Some(method_body) = child.child_by_field_name("body") {
                                        self.extract_defines_recursive(
                                            &method_body,
                                            code,
                                            Some(class_name),
                                            Some(method_name),
                                            defines,
                                        );
                                    }
                                }
                            }

                            // Fields defined by class
                            "field_declaration" => {
                                self.extract_field_definitions(&child, code, class_name, defines);
                            }

                            // Properties defined by class
                            "property_declaration" => {
                                if let Some(name_node) = child.child_by_field_name("name") {
                                    let prop_name = &code[name_node.byte_range()];
                                    let range = Range::new(
                                        child.start_position().row as u32,
                                        child.start_position().column as u16,
                                        child.end_position().row as u32,
                                        child.end_position().column as u16,
                                    );
                                    defines.push((class_name, prop_name, range));
                                }
                            }

                            // Events defined by class
                            "event_declaration" | "event_field_declaration" => {
                                // Try to get name directly first
                                if let Some(name_node) = child.child_by_field_name("name") {
                                    let event_name = &code[name_node.byte_range()];
                                    let range = Range::new(
                                        child.start_position().row as u32,
                                        child.start_position().column as u16,
                                        child.end_position().row as u32,
                                        child.end_position().column as u16,
                                    );
                                    defines.push((class_name, event_name, range));
                                } else {
                                    // Fallback: search for variable_declaration (similar to fields)
                                    let mut ev_cursor = child.walk();
                                    for ev_child in child.children(&mut ev_cursor) {
                                        if ev_child.kind() == "variable_declaration" {
                                            let mut var_cursor = ev_child.walk();
                                            for var_child in ev_child.children(&mut var_cursor) {
                                                if var_child.kind() == "variable_declarator" {
                                                    if let Some(name_node) = var_child.child_by_field_name("name") {
                                                        let event_name = &code[name_node.byte_range()];
                                                        let range = Range::new(
                                                            var_child.start_position().row as u32,
                                                            var_child.start_position().column as u16,
                                                            var_child.end_position().row as u32,
                                                            var_child.end_position().column as u16,
                                                        );
                                                        defines.push((class_name, event_name, range));
                                                    }
                                                }
                                            }
                                        } else if ev_child.kind() == "identifier" {
                                            // Sometimes the name is a direct identifier child
                                            let event_name = &code[ev_child.byte_range()];
                                            let range = Range::new(
                                                child.start_position().row as u32,
                                                child.start_position().column as u16,
                                                child.end_position().row as u32,
                                                child.end_position().column as u16,
                                            );
                                            defines.push((class_name, event_name, range));
                                            break;
                                        }
                                    }
                                }
                            }

                            // Nested types defined by class
                            "class_declaration"
                            | "struct_declaration"
                            | "record_declaration"
                            | "interface_declaration"
                            | "enum_declaration" => {
                                if let Some(name_node) = child.child_by_field_name("name") {
                                    let nested_name = &code[name_node.byte_range()];
                                    let range = Range::new(
                                        child.start_position().row as u32,
                                        child.start_position().column as u16,
                                        child.end_position().row as u32,
                                        child.end_position().column as u16,
                                    );
                                    defines.push((class_name, nested_name, range));

                                    // Recurse into nested type
                                    self.extract_defines_recursive(
                                        &child,
                                        code,
                                        Some(nested_name),
                                        None,
                                        defines,
                                    );
                                }
                            }

                            _ => {}
                        }
                    }
                }
            }

            // Interface declarations - track method signatures
            "interface_declaration" => {
                let interface_name = node
                    .child_by_field_name("name")
                    .map(|n| &code[n.byte_range()])
                    .unwrap_or("anonymous");

                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        match child.kind() {
                            "method_declaration" => {
                                if let Some(name_node) = child.child_by_field_name("name") {
                                    let method_name = &code[name_node.byte_range()];
                                    let range = Range::new(
                                        child.start_position().row as u32,
                                        child.start_position().column as u16,
                                        child.end_position().row as u32,
                                        child.end_position().column as u16,
                                    );
                                    defines.push((interface_name, method_name, range));
                                }
                            }
                            "property_declaration" => {
                                if let Some(name_node) = child.child_by_field_name("name") {
                                    let prop_name = &code[name_node.byte_range()];
                                    let range = Range::new(
                                        child.start_position().row as u32,
                                        child.start_position().column as u16,
                                        child.end_position().row as u32,
                                        child.end_position().column as u16,
                                    );
                                    defines.push((interface_name, prop_name, range));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Enum declarations - track enum members
            "enum_declaration" => {
                let enum_name = node
                    .child_by_field_name("name")
                    .map(|n| &code[n.byte_range()])
                    .unwrap_or("anonymous");

                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        if child.kind() == "enum_member_declaration" {
                            if let Some(name_node) = child.child_by_field_name("name") {
                                let member_name = &code[name_node.byte_range()];
                                let range = Range::new(
                                    child.start_position().row as u32,
                                    child.start_position().column as u16,
                                    child.end_position().row as u32,
                                    child.end_position().column as u16,
                                );
                                defines.push((enum_name, member_name, range));
                            }
                        }
                    }
                }
            }

            // Local variable declarations within methods
            "local_declaration_statement" | "variable_declaration" => {
                if let Some(method_name) = current_method {
                    self.extract_variable_definitions(node, code, method_name, defines);
                }
            }

            // Local function statements within methods
            "local_function_statement" => {
                if let Some(method_name) = current_method {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        let local_func_name = &code[name_node.byte_range()];
                        let range = Range::new(
                            node.start_position().row as u32,
                            node.start_position().column as u16,
                            node.end_position().row as u32,
                            node.end_position().column as u16,
                        );
                        defines.push((method_name, local_func_name, range));
                    }
                }
            }

            _ => {}
        }

        // Recurse to children (unless we're already in a method or we handled it above)
        if !matches!(
            node.kind(),
            "method_declaration"
                | "constructor_declaration"
                | "class_declaration"
                | "struct_declaration"
                | "record_declaration"
                | "interface_declaration"
                | "enum_declaration"
        ) {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.extract_defines_recursive(
                    &child,
                    code,
                    current_class,
                    current_method,
                    defines,
                );
            }
        }
    }

    /// Extract field definitions from a field_declaration node
    fn extract_field_definitions<'a>(
        &self,
        field_node: &Node,
        code: &'a str,
        class_name: &'a str,
        defines: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Field declarations have a variable_declaration child
        let mut cursor = field_node.walk();
        for child in field_node.children(&mut cursor) {
            if child.kind() == "variable_declaration" {
                // Get field names from variable_declarator children
                let mut var_cursor = child.walk();
                for var_child in child.children(&mut var_cursor) {
                    if var_child.kind() == "variable_declarator" {
                        if let Some(name_node) = var_child.child_by_field_name("name") {
                            let field_name = &code[name_node.byte_range()];
                            let range = Range::new(
                                var_child.start_position().row as u32,
                                var_child.start_position().column as u16,
                                var_child.end_position().row as u32,
                                var_child.end_position().column as u16,
                            );
                            defines.push((class_name, field_name, range));
                        }
                    }
                }
            }
        }
    }

    /// Extract variable definitions from a variable declaration node
    fn extract_variable_definitions<'a>(
        &self,
        var_decl: &Node,
        code: &'a str,
        method_name: &'a str,
        defines: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Look for variable_declarator nodes
        let mut cursor = var_decl.walk();
        for child in var_decl.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let var_name = &code[name_node.byte_range()];
                    let range = Range::new(
                        child.start_position().row as u32,
                        child.start_position().column as u16,
                        child.end_position().row as u32,
                        child.end_position().column as u16,
                    );
                    defines.push((method_name, var_name, range));
                }
            } else if child.kind() == "variable_declaration" {
                // Nested variable_declaration (for local_declaration_statement)
                self.extract_variable_definitions(&child, code, method_name, defines);
            }
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
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => {
                eprintln!("Failed to parse C# file for type uses");
                return Vec::new();
            }
        };

        let root_node = tree.root_node();
        let mut uses = Vec::new();

        self.extract_type_uses_recursive(&root_node, code, &mut uses);

        uses
    }

    fn find_defines<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => {
                eprintln!("Failed to parse C# file for definitions");
                return Vec::new();
            }
        };

        let root_node = tree.root_node();
        let mut defines = Vec::new();

        self.extract_defines_recursive(&root_node, code, None, None, &mut defines);

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
    fn test_csharp_find_uses_type_tracking() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public interface IRepository {
                void Save();
            }

            public class UserService : IRepository {
                private Helper _helper;
                private ILogger _logger;

                public UserService(Helper helper, ILogger logger) {
                    _helper = helper;
                    _logger = logger;
                }

                public Helper GetHelper() {
                    return _helper;
                }

                public void Save() {
                    _helper.DoWork();
                }
            }

            public class Helper {
                public void DoWork() { }
            }
        "#;

        let uses = parser.find_uses(code);

        // Debug output
        println!("\nFound {} type uses:", uses.len());
        for (context, type_name, _) in &uses {
            println!("  {} uses {}", context, type_name);
        }

        // Test base class/interface implementation
        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "UserService" && *typ == "IRepository"),
            "Should detect UserService uses IRepository (implements). Found: {:?}",
            uses
        );

        // Test field types
        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "_helper" && *typ == "Helper"),
            "Should detect _helper uses Helper (field type). Found: {:?}",
            uses
        );

        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "_logger" && *typ == "ILogger"),
            "Should detect _logger uses ILogger (field type). Found: {:?}",
            uses
        );

        // Test parameter types
        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "UserService" && *typ == "Helper"),
            "Should detect UserService uses Helper (constructor parameter). Found: {:?}",
            uses
        );

        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "UserService" && *typ == "ILogger"),
            "Should detect UserService uses ILogger (constructor parameter). Found: {:?}",
            uses
        );

        // Test return types
        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "GetHelper" && *typ == "Helper"),
            "Should detect GetHelper uses Helper (return type). Found: {:?}",
            uses
        );

        // Verify no primitive types are tracked
        assert!(
            !uses.iter().any(|(_, typ, _)| *typ == "void"),
            "Should not track primitive type 'void'. Found: {:?}",
            uses
        );
    }

    #[test]
    fn test_csharp_find_uses_generic_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class DataService {
                private List<User> _users;
                private Dictionary<int, Product> _products;

                public List<User> GetUsers() {
                    return _users;
                }

                public void ProcessData(List<Order> orders) {
                    // Implementation
                }
            }
        "#;

        let uses = parser.find_uses(code);

        println!("\nFound {} type uses in generic test:", uses.len());
        for (context, type_name, _) in &uses {
            println!("  {} uses {}", context, type_name);
        }

        // Test generic types - should track both the container and type arguments
        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "_users" && *typ == "List"),
            "Should detect _users uses List. Found: {:?}",
            uses
        );

        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "_users" && *typ == "User"),
            "Should detect _users uses User (generic type argument). Found: {:?}",
            uses
        );

        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "_products" && *typ == "Dictionary"),
            "Should detect _products uses Dictionary. Found: {:?}",
            uses
        );

        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "_products" && *typ == "Product"),
            "Should detect _products uses Product (generic type argument). Found: {:?}",
            uses
        );

        // Test generic parameter types
        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "ProcessData" && *typ == "List"),
            "Should detect ProcessData uses List. Found: {:?}",
            uses
        );

        assert!(
            uses.iter()
                .any(|(ctx, typ, _)| *ctx == "ProcessData" && *typ == "Order"),
            "Should detect ProcessData uses Order. Found: {:?}",
            uses
        );
    }

    #[test]
    fn test_csharp_find_defines_class_members() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class UserService {
                private ILogger _logger;
                private Helper _helper;

                public string Name { get; set; }
                public event EventHandler OnUpdate;

                public UserService(ILogger logger) {
                    _logger = logger;
                }

                public void Process() {
                    var result = 5;
                    int count = 10;
                }

                private void Helper() {
                    // Local function
                    void LocalFunc() { }
                }
            }
        "#;

        let defines = parser.find_defines(code);

        // Debug output
        println!("\nFound {} definitions:", defines.len());
        for (definer, defined, _) in &defines {
            println!("  {} defines {}", definer, defined);
        }

        // Test class defines methods
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "UserService"),
            "Should detect UserService defines UserService (constructor). Found: {:?}",
            defines
        );

        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "Process"),
            "Should detect UserService defines Process. Found: {:?}",
            defines
        );

        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "Helper"),
            "Should detect UserService defines Helper. Found: {:?}",
            defines
        );

        // Test class defines fields
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "_logger"),
            "Should detect UserService defines _logger. Found: {:?}",
            defines
        );

        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "_helper"),
            "Should detect UserService defines _helper. Found: {:?}",
            defines
        );

        // Test class defines properties
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "Name"),
            "Should detect UserService defines Name (property). Found: {:?}",
            defines
        );

        // Test class defines events
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "OnUpdate"),
            "Should detect UserService defines OnUpdate (event). Found: {:?}",
            defines
        );

        // Test method defines local variables
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Process" && *defined == "result"),
            "Should detect Process defines result (local variable). Found: {:?}",
            defines
        );

        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Process" && *defined == "count"),
            "Should detect Process defines count (local variable). Found: {:?}",
            defines
        );

        // Test method defines local functions
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Helper" && *defined == "LocalFunc"),
            "Should detect Helper defines LocalFunc (local function). Found: {:?}",
            defines
        );

        // Test constructor defines local variables
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "UserService" && *defined == "_logger"),
            "Should detect UserService (constructor) has access to _logger assignment. Found: {:?}",
            defines
        );
    }

    #[test]
    fn test_csharp_find_defines_nested_types() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public class OuterClass {
                public class InnerClass {
                    private int _value;

                    public void InnerMethod() { }
                }

                public struct InnerStruct {
                    public int X;
                }

                public enum Status {
                    Active,
                    Inactive
                }
            }
        "#;

        let defines = parser.find_defines(code);

        println!(
            "\nFound {} definitions in nested types test:",
            defines.len()
        );
        for (definer, defined, _) in &defines {
            println!("  {} defines {}", definer, defined);
        }

        // Test outer class defines nested types
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "OuterClass" && *defined == "InnerClass"),
            "Should detect OuterClass defines InnerClass. Found: {:?}",
            defines
        );

        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "OuterClass" && *defined == "InnerStruct"),
            "Should detect OuterClass defines InnerStruct. Found: {:?}",
            defines
        );

        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "OuterClass" && *defined == "Status"),
            "Should detect OuterClass defines Status (enum). Found: {:?}",
            defines
        );

        // Test inner class defines its own members
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "InnerClass" && *defined == "_value"),
            "Should detect InnerClass defines _value. Found: {:?}",
            defines
        );

        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "InnerClass" && *defined == "InnerMethod"),
            "Should detect InnerClass defines InnerMethod. Found: {:?}",
            defines
        );

        // Test struct defines fields
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "InnerStruct" && *defined == "X"),
            "Should detect InnerStruct defines X. Found: {:?}",
            defines
        );

        // Test enum defines members
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Status" && *defined == "Active"),
            "Should detect Status defines Active. Found: {:?}",
            defines
        );

        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "Status" && *defined == "Inactive"),
            "Should detect Status defines Inactive. Found: {:?}",
            defines
        );
    }

    #[test]
    fn test_csharp_find_defines_interfaces() {
        let mut parser = CSharpParser::new().unwrap();
        let code = r#"
            public interface IRepository {
                void Save();
                void Delete();
                string Name { get; set; }
            }

            public interface ILogger {
                void Log(string message);
            }
        "#;

        let defines = parser.find_defines(code);

        println!("\nFound {} definitions in interfaces test:", defines.len());
        for (definer, defined, _) in &defines {
            println!("  {} defines {}", definer, defined);
        }

        // Test interface defines methods
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "IRepository" && *defined == "Save"),
            "Should detect IRepository defines Save. Found: {:?}",
            defines
        );

        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "IRepository" && *defined == "Delete"),
            "Should detect IRepository defines Delete. Found: {:?}",
            defines
        );

        // Test interface defines properties
        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "IRepository" && *defined == "Name"),
            "Should detect IRepository defines Name (property). Found: {:?}",
            defines
        );

        assert!(
            defines
                .iter()
                .any(|(definer, defined, _)| *definer == "ILogger" && *defined == "Log"),
            "Should detect ILogger defines Log. Found: {:?}",
            defines
        );
    }
}
