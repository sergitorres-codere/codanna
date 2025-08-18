//! Symbol context aggregation for comprehensive metadata display

use crate::{Symbol, SymbolKind, Visibility};
use bitflags::bitflags;
use serde::Serialize;
use std::fmt;

/// Comprehensive context for a symbol including all relationships
#[derive(Debug, Clone, Serialize)]
pub struct SymbolContext {
    /// The symbol itself with all its metadata
    pub symbol: Symbol,
    /// Resolved file path for easy navigation
    pub file_path: String,
    /// All relationships this symbol has
    pub relationships: SymbolRelationships,
}

/// Container for all types of symbol relationships
#[derive(Debug, Clone, Default, Serialize)]
pub struct SymbolRelationships {
    /// What traits this type implements
    pub implements: Option<Vec<Symbol>>,
    /// What types implement this trait
    pub implemented_by: Option<Vec<Symbol>>,
    /// What methods/fields this symbol defines
    pub defines: Option<Vec<Symbol>>,
    /// What this symbol calls (with metadata)
    pub calls: Option<Vec<(Symbol, Option<String>)>>,
    /// What calls this symbol (with metadata)
    pub called_by: Option<Vec<(Symbol, Option<String>)>>,
}

bitflags! {
    /// Flags to control what context to include
    pub struct ContextIncludes: u8 {
        const IMPLEMENTATIONS = 0b00000001;
        const DEFINITIONS    = 0b00000010;
        const CALLS         = 0b00000100;
        const CALLERS       = 0b00001000;
        const ALL           = 0b00001111;
    }
}

impl fmt::Display for SymbolContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use the existing format_location_with_type method
        // This provides: "Function calculate_similarity at src/vector/similarity.rs:101"
        write!(f, "{}", self.format_location_with_type())
    }
}

impl SymbolContext {
    /// Format just the location line
    pub fn format_location(&self) -> String {
        // Note: file_path already includes line number (e.g., "src/file.rs:123")
        format!("{} at {}", self.symbol.name, self.file_path)
    }

    /// Format location with type info
    pub fn format_location_with_type(&self) -> String {
        // Note: file_path already includes line number (e.g., "src/file.rs:123")
        format!(
            "{:?} {} at {}",
            self.symbol.kind, self.symbol.name, self.file_path
        )
    }

    /// Format comprehensive output
    pub fn format_full(&self, indent: &str) -> String {
        let mut output = String::new();
        self.append_header(&mut output, indent);
        self.append_metadata(&mut output, indent);
        self.append_relationships(&mut output, indent);
        output
    }

    fn append_header(&self, output: &mut String, indent: &str) {
        // Note: file_path already includes line number (e.g., "src/file.rs:123")
        output.push_str(&format!(
            "{}{} ({:?}) at {}\n",
            indent, self.symbol.name, self.symbol.kind, self.file_path
        ));
    }

    fn append_metadata(&self, output: &mut String, indent: &str) {
        // Module path
        if let Some(module) = self.symbol.as_module_path() {
            output.push_str(&format!("{indent}Module: {module}\n"));
        }

        // Signature for methods/functions
        if matches!(self.symbol.kind, SymbolKind::Function | SymbolKind::Method) {
            if let Some(sig) = self.symbol.as_signature() {
                output.push_str(&format!("{indent}Signature: {sig}\n"));
            }
        }

        // Visibility for appropriate symbols
        if !matches!(self.symbol.visibility, Visibility::Private) {
            output.push_str(&format!(
                "{}Visibility: {:?}\n",
                indent, self.symbol.visibility
            ));
        }

        // Documentation preview
        if let Some(doc) = self.symbol.as_doc_comment() {
            let preview: Vec<&str> = doc.lines().take(2).collect();
            if !preview.is_empty() {
                output.push_str(&format!("{}Doc: {}", indent, preview.join(" ")));
                if doc.lines().count() > 2 {
                    output.push_str("...");
                }
                output.push('\n');
            }
        }
    }

    fn append_relationships(&self, output: &mut String, indent: &str) {
        // Implementations
        if let Some(impls) = &self.relationships.implements {
            if !impls.is_empty() {
                output.push_str(&format!(
                    "{}Implements: {}\n",
                    indent,
                    impls
                        .iter()
                        .map(|s| s.as_name())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        // Implemented by
        if let Some(impl_by) = &self.relationships.implemented_by {
            if !impl_by.is_empty() {
                output.push_str(&format!(
                    "{}Implemented by {} type(s):\n",
                    indent,
                    impl_by.len()
                ));
                for impl_type in impl_by {
                    // Note: In real implementation, would need to resolve each impl's file path
                    output.push_str(&format!(
                        "{}  - {} at <file>:{}\n",
                        indent,
                        impl_type.name,
                        impl_type.range.start_line + 1
                    ));
                }
            }
        }

        // Methods defined
        if let Some(defines) = &self.relationships.defines {
            let methods: Vec<_> = defines
                .iter()
                .filter(|s| s.kind == SymbolKind::Method)
                .collect();
            if !methods.is_empty() {
                output.push_str(&format!("{indent}Methods: "));
                output.push_str(
                    &methods
                        .iter()
                        .map(|m| {
                            if let Some(sig) = m.as_signature() {
                                sig.to_string()
                            } else {
                                m.name.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                output.push('\n');
            }
        }

        // Calls
        if let Some(calls) = &self.relationships.calls {
            if !calls.is_empty() {
                output.push_str(&format!("{}Calls {} function(s)\n", indent, calls.len()));
            }
        }

        // Called by
        if let Some(callers) = &self.relationships.called_by {
            if !callers.is_empty() {
                output.push_str(&format!(
                    "{}Called by {} function(s)\n",
                    indent,
                    callers.len()
                ));
            }
        }
    }
}
