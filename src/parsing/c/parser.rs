//! C language parser implementation

use crate::parsing::method_call::MethodCall;
use crate::parsing::{Import, Language, LanguageParser, ParserContext, ScopeType};
use crate::types::{Range, SymbolCounter};
use crate::{FileId, Symbol, SymbolKind};
use std::any::Any;
use tree_sitter::{Node, Parser};

pub struct CParser {
    parser: Parser,
    context: ParserContext,
}

impl std::fmt::Debug for CParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CParser").field("language", &"C").finish()
    }
}

impl CParser {
    pub fn new() -> Result<Self, String> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_c::LANGUAGE.into())
            .map_err(|e| format!("Failed to set C language: {e}"))?;

        Ok(Self {
            parser,
            context: ParserContext::new(),
        })
    }

    /// Parse C code and extract symbols
    ///
    /// This is the main parsing method that can be called directly.
    pub fn parse(
        &mut self,
        code: &str,
        file_id: FileId,
        symbol_counter: &mut SymbolCounter,
    ) -> Vec<Symbol> {
        // Delegate to the LanguageParser trait implementation
        <Self as LanguageParser>::parse(self, code, file_id, symbol_counter)
    }

    /// Extract import statements from the code
    fn extract_imports_from_node(
        node: Node,
        code: &str,
        file_id: FileId,
        imports: &mut Vec<Import>,
    ) {
        if node.kind() == "preproc_include" {
            if let Some(path_node) = node.child_by_field_name("path") {
                let path_text = &code[path_node.byte_range()];
                // Remove quotes
                let clean_path = path_text.trim_matches(|c| c == '"' || c == '<' || c == '>');
                imports.push(Import {
                    path: clean_path.to_string(),
                    alias: None,
                    file_id,
                    is_glob: false,
                    is_type_only: false,
                });
            }
        }

        // Recursively process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::extract_imports_from_node(child, code, file_id, imports);
            }
        }
    }

    /// Create a symbol with C-specific handling
    fn create_symbol(
        &mut self,
        counter: &mut SymbolCounter,
        name_node: Node,
        kind: SymbolKind,
        file_id: FileId,
        code: &str,
    ) -> Option<Symbol> {
        let name = &code[name_node.byte_range()];
        let symbol_id = counter.next_id();

        let range = Range::new(
            name_node.start_position().row as u32,
            name_node.start_position().column as u16,
            name_node.end_position().row as u32,
            name_node.end_position().column as u16,
        );

        let mut symbol = Symbol::new(symbol_id, name.to_string(), kind, file_id, range);

        // Set scope context based on parser's current scope
        symbol.scope_context = Some(self.context.current_scope_context());

        // C has simpler visibility - most symbols are public by default
        // Static storage class makes symbols private to the compilation unit
        if let Some(parent) = name_node.parent() {
            let mut is_static = false;
            for child in parent.children(&mut parent.walk()) {
                if child.kind() == "storage_class_specifier" {
                    let storage_text = &code[child.byte_range()];
                    if storage_text == "static" {
                        is_static = true;
                        break;
                    }
                }
            }

            if is_static {
                symbol = symbol.with_visibility(crate::Visibility::Private);
            } else {
                symbol = symbol.with_visibility(crate::Visibility::Public);
            }
        } else {
            symbol = symbol.with_visibility(crate::Visibility::Public);
        }

        Some(symbol)
    }

    /// Helper to find function name node in C's complex declarator structure
    fn find_function_name_node(declarator: Node) -> Option<Node> {
        // C function declarators can be nested: function_declarator -> declarator -> identifier
        match declarator.kind() {
            "function_declarator" => {
                if let Some(inner) = declarator.child_by_field_name("declarator") {
                    Self::find_function_name_node(inner)
                } else {
                    None
                }
            }
            "identifier" => Some(declarator),
            "parenthesized_declarator" => {
                if let Some(inner) = declarator.child_by_field_name("declarator") {
                    Self::find_function_name_node(inner)
                } else {
                    None
                }
            }
            _ => {
                // Search children for identifier
                for child in declarator.children(&mut declarator.walk()) {
                    if child.kind() == "identifier" {
                        return Some(child);
                    }
                    if let Some(found) = Self::find_function_name_node(child) {
                        return Some(found);
                    }
                }
                None
            }
        }
    }

    /// Helper to find declarator name for variables and parameters
    fn find_declarator_name(node: Node) -> Option<Node> {
        match node.kind() {
            "identifier" => Some(node),
            "init_declarator" => {
                if let Some(declarator) = node.child_by_field_name("declarator") {
                    Self::find_declarator_name(declarator)
                } else {
                    None
                }
            }
            "parameter_declaration" => {
                if let Some(declarator) = node.child_by_field_name("declarator") {
                    Self::find_declarator_name(declarator)
                } else {
                    None
                }
            }
            "pointer_declarator" | "array_declarator" => {
                if let Some(declarator) = node.child_by_field_name("declarator") {
                    Self::find_declarator_name(declarator)
                } else {
                    None
                }
            }
            _ => {
                // Search children for identifier
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "identifier" {
                        return Some(child);
                    }
                    if let Some(found) = Self::find_declarator_name(child) {
                        return Some(found);
                    }
                }
                None
            }
        }
    }

    fn extract_symbols_from_node(
        &mut self,
        node: Node,
        code: &str,
        file_id: FileId,
        symbols: &mut Vec<Symbol>,
        counter: &mut SymbolCounter,
    ) {
        match node.kind() {
            "translation_unit" => {
                // Root node - establish file-level scope context
                // This doesn't create symbols but provides the top-level context for all other nodes
                self.context.enter_scope(ScopeType::Module);

                // Process all top-level declarations
                for child in node.children(&mut node.walk()) {
                    self.extract_symbols_from_node(child, code, file_id, symbols, counter);
                }

                self.context.exit_scope();
                return; // Skip default traversal
            }
            "function_definition" => {
                // C function names are nested in declarator structure
                if let Some(declarator) = node.child_by_field_name("declarator") {
                    if let Some(name_node) = Self::find_function_name_node(declarator) {
                        if let Some(symbol) = self.create_symbol(
                            counter,
                            name_node,
                            SymbolKind::Function,
                            file_id,
                            code,
                        ) {
                            symbols.push(symbol);
                        }
                    }
                }

                // Enter function scope for nested declarations
                self.context
                    .enter_scope(ScopeType::Function { hoisting: false });

                // Process function body for nested symbols
                for child in node.children(&mut node.walk()) {
                    // Skip declarator (already processed) and parameter lists
                    if child.kind() != "function_declarator" && child.kind() != "parameter_list" {
                        self.extract_symbols_from_node(child, code, file_id, symbols, counter);
                    }
                }

                self.context.exit_scope();
                return; // Skip default traversal
            }
            "struct_specifier" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Some(symbol) =
                        self.create_symbol(counter, name_node, SymbolKind::Struct, file_id, code)
                    {
                        symbols.push(symbol);
                    }
                }

                // Process struct fields
                if let Some(body) = node.child_by_field_name("body") {
                    self.context.enter_scope(ScopeType::Class);
                    for child in body.children(&mut body.walk()) {
                        self.extract_symbols_from_node(child, code, file_id, symbols, counter);
                    }
                    self.context.exit_scope();
                }
            }
            "union_specifier" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Some(symbol) =
                        self.create_symbol(counter, name_node, SymbolKind::Struct, file_id, code)
                    {
                        symbols.push(symbol);
                    }
                }

                // Process union fields
                if let Some(body) = node.child_by_field_name("body") {
                    self.context.enter_scope(ScopeType::Class);
                    for child in body.children(&mut body.walk()) {
                        self.extract_symbols_from_node(child, code, file_id, symbols, counter);
                    }
                    self.context.exit_scope();
                }
            }
            "enum_specifier" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Some(symbol) =
                        self.create_symbol(counter, name_node, SymbolKind::Enum, file_id, code)
                    {
                        symbols.push(symbol);
                    }
                }

                // Process enum values
                if let Some(body) = node.child_by_field_name("body") {
                    for child in body.children(&mut body.walk()) {
                        if child.kind() == "enumerator" {
                            if let Some(name_node) = child.child_by_field_name("name") {
                                if let Some(symbol) = self.create_symbol(
                                    counter,
                                    name_node,
                                    SymbolKind::Constant,
                                    file_id,
                                    code,
                                ) {
                                    symbols.push(symbol);
                                }
                            }
                        }
                    }
                }
            }
            "declaration" => {
                // Handle variable declarations
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "init_declarator" {
                        if let Some(name_node) = Self::find_declarator_name(child) {
                            if let Some(symbol) = self.create_symbol(
                                counter,
                                name_node,
                                SymbolKind::Variable,
                                file_id,
                                code,
                            ) {
                                symbols.push(symbol);
                            }
                        }
                    }
                }
            }
            "init_declarator" => {
                // Handle variable initialization (int x = 5, Rectangle *rect = malloc(...), etc.)
                if let Some(name_node) = Self::find_declarator_name(node) {
                    if let Some(symbol) =
                        self.create_symbol(counter, name_node, SymbolKind::Variable, file_id, code)
                    {
                        symbols.push(symbol);
                    }
                }
            }
            "compound_statement" => {
                // Handle block statements { ... } - establish block scope for nested declarations
                self.context.enter_scope(ScopeType::Block);

                // Process all statements and declarations within the block
                for child in node.children(&mut node.walk()) {
                    // Skip braces, process the contents
                    if child.kind() != "{" && child.kind() != "}" {
                        self.extract_symbols_from_node(child, code, file_id, symbols, counter);
                    }
                }

                self.context.exit_scope();
                return; // Skip default traversal
            }
            "parameter_declaration" => {
                // Handle function parameters
                if let Some(name_node) = Self::find_declarator_name(node) {
                    if let Some(symbol) =
                        self.create_symbol(counter, name_node, SymbolKind::Parameter, file_id, code)
                    {
                        symbols.push(symbol);
                    }
                }
            }
            "field_declaration" => {
                // Handle struct/union field declarations
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "field_declarator" {
                        if let Some(name_node) = child.child(0) {
                            if name_node.kind() == "field_identifier" {
                                if let Some(symbol) = self.create_symbol(
                                    counter,
                                    name_node,
                                    SymbolKind::Field,
                                    file_id,
                                    code,
                                ) {
                                    symbols.push(symbol);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Process children
        for child in node.children(&mut node.walk()) {
            self.extract_symbols_from_node(child, code, file_id, symbols, counter);
        }
    }

    fn extract_calls_from_node(node: Node, code: &str, calls: &mut Vec<MethodCall>) {
        if node.kind() == "call_expression" {
            if let Some(function_node) = node.child_by_field_name("function") {
                let function_name = &code[function_node.byte_range()];
                calls.push(MethodCall::new(
                    "", // caller will be set by the indexer
                    function_name,
                    Range::new(
                        node.start_position().row as u32,
                        node.start_position().column as u16,
                        node.end_position().row as u32,
                        node.end_position().column as u16,
                    ),
                ));
            }
        }

        // Process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::extract_calls_from_node(child, code, calls);
            }
        }
    }

    /// Find function calls in AST node recursively
    fn find_calls_in_node<'a>(
        node: Node,
        code: &'a str,
        calls: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Simple implementation that doesn't track containing functions
        if node.kind() == "call_expression" {
            if let Some(function_node) = node.child_by_field_name("function") {
                let target_name = &code[function_node.byte_range()];
                // We don't have caller information in this simple implementation
                let range = Range::new(
                    node.start_position().row as u32,
                    node.start_position().column as u16,
                    node.end_position().row as u32,
                    node.end_position().column as u16,
                );
                // Use empty string for caller as we don't track it in this simple implementation
                calls.push(("", target_name, range));
            }
        }

        // Process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::find_calls_in_node(child, code, calls);
            }
        }
    }

    /// Find variable and function uses in AST nodes recursively
    fn find_uses_in_node<'a>(node: Node, code: &'a str, uses: &mut Vec<(&'a str, &'a str, Range)>) {
        // Identifier nodes represent variable/function uses
        if node.kind() == "identifier" {
            // We need context to determine what this identifier is used in
            // For now, we'll just track the identifier name and its location
            let identifier_name = &code[node.byte_range()];
            let range = Range::new(
                node.start_position().row as u32,
                node.start_position().column as u16,
                node.end_position().row as u32,
                node.end_position().column as u16,
            );
            // Use empty string for context for now
            uses.push(("", identifier_name, range));
        }

        // Process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::find_uses_in_node(child, code, uses);
            }
        }
    }

    /// Find variable and macro definitions in AST nodes recursively
    fn find_defines_in_node<'a>(
        node: Node,
        code: &'a str,
        defines: &mut Vec<(&'a str, &'a str, Range)>,
    ) {
        // Variable declarations
        if node.kind() == "declaration" {
            if let Some(declarator) = node.child_by_field_name("declarator") {
                let declarator_text = &code[declarator.byte_range()];
                // Extract variable name (before = if present)
                if let Some(equals_pos) = declarator_text.find('=') {
                    let var_name = declarator_text[..equals_pos].trim();
                    let range = Range::new(
                        node.start_position().row as u32,
                        node.start_position().column as u16,
                        node.end_position().row as u32,
                        node.end_position().column as u16,
                    );
                    defines.push((var_name, "variable", range));
                } else {
                    let range = Range::new(
                        node.start_position().row as u32,
                        node.start_position().column as u16,
                        node.end_position().row as u32,
                        node.end_position().column as u16,
                    );
                    defines.push((declarator_text.trim(), "variable", range));
                }
            }
        }
        // Preprocessor definitions
        else if node.kind() == "preproc_def" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let macro_name = &code[name_node.byte_range()];
                let range = Range::new(
                    node.start_position().row as u32,
                    node.start_position().column as u16,
                    node.end_position().row as u32,
                    node.end_position().column as u16,
                );
                defines.push((macro_name, "macro", range));
            }
        }

        // Process children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::find_defines_in_node(child, code, defines);
            }
        }
    }
}

impl LanguageParser for CParser {
    fn parse(
        &mut self,
        code: &str,
        file_id: FileId,
        symbol_counter: &mut SymbolCounter,
    ) -> Vec<Symbol> {
        // Reset context for each file
        self.context = ParserContext::new();

        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut symbols = Vec::new();

        self.extract_symbols_from_node(root_node, code, file_id, &mut symbols, symbol_counter);

        symbols
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn extract_doc_comment(&self, _node: &Node, _code: &str) -> Option<String> {
        // C doesn't have standardized doc comments
        None
    }

    fn find_calls<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut calls = Vec::new();

        Self::find_calls_in_node(root_node, code, &mut calls);
        calls
    }

    fn find_method_calls(&mut self, code: &str) -> Vec<MethodCall> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut calls = Vec::new();

        Self::extract_calls_from_node(root_node, code, &mut calls);

        calls
    }

    fn find_implementations<'a>(&mut self, _code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        // C doesn't have interfaces or traits
        Vec::new()
    }

    fn find_uses<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut uses = Vec::new();

        Self::find_uses_in_node(root_node, code, &mut uses);
        uses
    }

    fn find_defines<'a>(&mut self, code: &'a str) -> Vec<(&'a str, &'a str, Range)> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut defines = Vec::new();

        Self::find_defines_in_node(root_node, code, &mut defines);
        defines
    }

    fn find_imports(&mut self, code: &str, file_id: FileId) -> Vec<Import> {
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return Vec::new(),
        };

        let root_node = tree.root_node();
        let mut imports = Vec::new();

        Self::extract_imports_from_node(root_node, code, file_id, &mut imports);

        imports
    }

    fn language(&self) -> Language {
        Language::C
    }
}
