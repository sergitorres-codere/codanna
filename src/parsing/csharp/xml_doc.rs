//! Structured XML documentation parsing for C#
//!
//! This module provides parsing and structured representation of C# XML documentation comments.
//! C# uses XML tags like `<summary>`, `<param>`, `<returns>`, etc. in `///` comments.
//!
//! # Example
//!
//! ```csharp
//! /// <summary>
//! /// Calculates the sum of two numbers
//! /// </summary>
//! /// <param name="a">First number</param>
//! /// <param name="b">Second number</param>
//! /// <returns>The sum of a and b</returns>
//! public int Add(int a, int b) { return a + b; }
//! ```
//!
//! This gets parsed into a structured `XmlDocumentation` object.

use serde::{Deserialize, Serialize};

/// Structured representation of C# XML documentation
///
/// Contains parsed fields from XML documentation comments.
/// All fields are optional as not all documentation will have all tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct XmlDocumentation {
    /// Content of `<summary>` tag - main description
    pub summary: Option<String>,

    /// Content of `<remarks>` tag - additional remarks/notes
    pub remarks: Option<String>,

    /// Content of `<returns>` tag - return value description
    pub returns: Option<String>,

    /// Content of `<value>` tag - property value description
    pub value: Option<String>,

    /// All `<param>` tags - parameter descriptions
    pub params: Vec<XmlParam>,

    /// All `<typeparam>` tags - generic type parameter descriptions
    pub type_params: Vec<XmlTypeParam>,

    /// All `<exception>` tags - exception documentation
    pub exceptions: Vec<XmlException>,

    /// All `<example>` tags - code examples
    pub examples: Vec<String>,

    /// All `<seealso>` tags - cross-references
    pub see_also: Vec<String>,

    /// The original raw XML text (for fallback)
    pub raw: String,
}

/// Parameter documentation from `<param>` tag
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XmlParam {
    /// Parameter name from `name` attribute
    pub name: String,
    /// Parameter description (tag content)
    pub description: String,
}

/// Generic type parameter documentation from `<typeparam>` tag
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XmlTypeParam {
    /// Type parameter name from `name` attribute
    pub name: String,
    /// Type parameter description (tag content)
    pub description: String,
}

/// Exception documentation from `<exception>` tag
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XmlException {
    /// Exception type from `cref` attribute
    pub cref: String,
    /// Exception description (tag content)
    pub description: String,
}

impl XmlDocumentation {
    /// Parse raw C# XML documentation comments into structured form
    ///
    /// Takes the raw `///` comment text and extracts all XML tags into
    /// structured fields.
    ///
    /// # Example
    ///
    /// ```
    /// use codanna::parsing::csharp::xml_doc::XmlDocumentation;
    ///
    /// let raw = r#"
    /// /// <summary>
    /// /// Does something
    /// /// </summary>
    /// /// <param name="x">Input value</param>
    /// "#;
    ///
    /// let doc = XmlDocumentation::parse(raw);
    /// assert_eq!(doc.summary.as_deref(), Some("Does something"));
    /// assert_eq!(doc.params.len(), 1);
    /// assert_eq!(doc.params[0].name, "x");
    /// ```
    ///
    /// # Implementation Notes
    ///
    /// - Uses simple string parsing (not full XML parser) for performance
    /// - Strips `///` prefixes automatically
    /// - Handles multiline tag content
    /// - Preserves original text in `raw` field
    pub fn parse(raw_comment: &str) -> Self {
        let mut doc = XmlDocumentation {
            raw: raw_comment.to_string(),
            ..Default::default()
        };

        // Strip /// prefixes and build clean XML content
        let cleaned = Self::strip_doc_comment_markers(raw_comment);

        // Parse individual tags
        doc.summary = Self::extract_single_tag(&cleaned, "summary");
        doc.remarks = Self::extract_single_tag(&cleaned, "remarks");
        doc.returns = Self::extract_single_tag(&cleaned, "returns");
        doc.value = Self::extract_single_tag(&cleaned, "value");

        // Parse tags with attributes
        doc.params = Self::extract_params(&cleaned);
        doc.type_params = Self::extract_type_params(&cleaned);
        doc.exceptions = Self::extract_exceptions(&cleaned);
        doc.examples = Self::extract_all_tags(&cleaned, "example");
        doc.see_also = Self::extract_all_see_also(&cleaned);

        doc
    }

    /// Check if this documentation is empty (no meaningful content)
    pub fn is_empty(&self) -> bool {
        self.summary.is_none()
            && self.remarks.is_none()
            && self.returns.is_none()
            && self.value.is_none()
            && self.params.is_empty()
            && self.type_params.is_empty()
            && self.exceptions.is_empty()
            && self.examples.is_empty()
            && self.see_also.is_empty()
    }

    /// Strip `///` markers from doc comment lines
    ///
    /// Converts:
    /// ```text
    /// /// <summary>
    /// /// Text here
    /// /// </summary>
    /// ```
    ///
    /// To:
    /// ```text
    /// <summary>
    /// Text here
    /// </summary>
    /// ```
    fn strip_doc_comment_markers(raw: &str) -> String {
        raw.lines()
            .map(|line| {
                let trimmed = line.trim();
                if let Some(stripped) = trimmed.strip_prefix("///") {
                    stripped.trim_start()
                } else {
                    trimmed
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract content from a single tag like `<summary>`
    ///
    /// Returns the trimmed content between opening and closing tags.
    fn extract_single_tag(content: &str, tag: &str) -> Option<String> {
        let opening = format!("<{tag}>");
        let closing = format!("</{tag}>");

        if let Some(start_idx) = content.find(&opening) {
            let content_start = start_idx + opening.len();
            if let Some(end_idx) = content[content_start..].find(&closing) {
                let tag_content = &content[content_start..content_start + end_idx];
                let trimmed = tag_content.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        None
    }

    /// Extract all instances of a tag (like `<example>`)
    fn extract_all_tags(content: &str, tag: &str) -> Vec<String> {
        let mut results = Vec::new();
        let opening = format!("<{tag}>");
        let closing = format!("</{tag}>");

        let mut search_start = 0;
        while let Some(start_idx) = content[search_start..].find(&opening) {
            let absolute_start = search_start + start_idx;
            let content_start = absolute_start + opening.len();

            if let Some(end_idx) = content[content_start..].find(&closing) {
                let tag_content = &content[content_start..content_start + end_idx];
                let trimmed = tag_content.trim();
                if !trimmed.is_empty() {
                    results.push(trimmed.to_string());
                }
                search_start = content_start + end_idx + closing.len();
            } else {
                break;
            }
        }

        results
    }

    /// Extract all `<param name="...">` tags
    fn extract_params(content: &str) -> Vec<XmlParam> {
        Self::extract_named_tags(content, "param")
            .into_iter()
            .map(|(name, desc)| XmlParam {
                name,
                description: desc,
            })
            .collect()
    }

    /// Extract all `<typeparam name="...">` tags
    fn extract_type_params(content: &str) -> Vec<XmlTypeParam> {
        Self::extract_named_tags(content, "typeparam")
            .into_iter()
            .map(|(name, desc)| XmlTypeParam {
                name,
                description: desc,
            })
            .collect()
    }

    /// Extract all `<exception cref="...">` tags
    fn extract_exceptions(content: &str) -> Vec<XmlException> {
        let mut exceptions = Vec::new();
        let tag = "exception";

        let mut search_start = 0;
        while let Some(tag_start) = content[search_start..].find(&format!("<{tag} ")) {
            let absolute_start = search_start + tag_start;
            let tag_content = &content[absolute_start..];

            // Extract cref attribute
            if let Some(cref_start) = tag_content.find("cref=\"") {
                let cref_content_start = cref_start + 6; // len("cref=\"")
                if let Some(cref_end) = tag_content[cref_content_start..].find('"') {
                    let cref = tag_content[cref_content_start..cref_content_start + cref_end]
                        .to_string();

                    // Extract description (content between tags)
                    if let Some(content_start) = tag_content.find('>') {
                        let desc_start = content_start + 1;
                        let closing = format!("</{tag}>");
                        if let Some(desc_end) = tag_content[desc_start..].find(&closing) {
                            let description = tag_content[desc_start..desc_start + desc_end]
                                .trim()
                                .to_string();

                            exceptions.push(XmlException { cref, description });
                            search_start = absolute_start + desc_start + desc_end + closing.len();
                            continue;
                        }
                    }
                }
            }

            break;
        }

        exceptions
    }

    /// Extract all `<seealso cref="..."/>` tags
    fn extract_all_see_also(content: &str) -> Vec<String> {
        let mut results = Vec::new();

        // Look for both <seealso cref="..."/> and <seealso cref="..."></seealso>
        let mut search_start = 0;
        while let Some(tag_start) = content[search_start..].find("<seealso ") {
            let absolute_start = search_start + tag_start;
            let tag_content = &content[absolute_start..];

            if let Some(cref_start) = tag_content.find("cref=\"") {
                let cref_content_start = cref_start + 6;
                if let Some(cref_end) = tag_content[cref_content_start..].find('"') {
                    let cref = tag_content[cref_content_start..cref_content_start + cref_end]
                        .to_string();
                    results.push(cref);
                    search_start = absolute_start + cref_content_start + cref_end;
                    continue;
                }
            }

            break;
        }

        results
    }

    /// Extract tags with `name` attribute like `<param name="x">description</param>`
    fn extract_named_tags(content: &str, tag: &str) -> Vec<(String, String)> {
        let mut results = Vec::new();

        let mut search_start = 0;
        while let Some(tag_start) = content[search_start..].find(&format!("<{tag} ")) {
            let absolute_start = search_start + tag_start;
            let tag_content = &content[absolute_start..];

            // Extract name attribute
            if let Some(name_start) = tag_content.find("name=\"") {
                let name_content_start = name_start + 6; // len("name=\"")
                if let Some(name_end) = tag_content[name_content_start..].find('"') {
                    let name = tag_content[name_content_start..name_content_start + name_end]
                        .to_string();

                    // Extract description (content between tags)
                    if let Some(content_start) = tag_content.find('>') {
                        let desc_start = content_start + 1;
                        let closing = format!("</{tag}>");
                        if let Some(desc_end) = tag_content[desc_start..].find(&closing) {
                            let description = tag_content[desc_start..desc_start + desc_end]
                                .trim()
                                .to_string();

                            results.push((name, description));
                            search_start = absolute_start + desc_start + desc_end + closing.len();
                            continue;
                        }
                    }
                }
            }

            break;
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_summary() {
        let raw = r#"
/// <summary>
/// This is a test method
/// </summary>
"#;

        let doc = XmlDocumentation::parse(raw);
        assert_eq!(doc.summary.as_deref(), Some("This is a test method"));
        assert!(doc.params.is_empty());
    }

    #[test]
    fn test_parse_with_params() {
        let raw = r#"
/// <summary>Adds two numbers</summary>
/// <param name="a">First number</param>
/// <param name="b">Second number</param>
/// <returns>Sum of a and b</returns>
"#;

        let doc = XmlDocumentation::parse(raw);
        assert_eq!(doc.summary.as_deref(), Some("Adds two numbers"));
        assert_eq!(doc.returns.as_deref(), Some("Sum of a and b"));
        assert_eq!(doc.params.len(), 2);
        assert_eq!(doc.params[0].name, "a");
        assert_eq!(doc.params[0].description, "First number");
        assert_eq!(doc.params[1].name, "b");
        assert_eq!(doc.params[1].description, "Second number");
    }

    #[test]
    fn test_parse_with_type_params() {
        let raw = r#"
/// <summary>A generic container</summary>
/// <typeparam name="T">The type to store</typeparam>
/// <typeparam name="U">The key type</typeparam>
"#;

        let doc = XmlDocumentation::parse(raw);
        assert_eq!(doc.type_params.len(), 2);
        assert_eq!(doc.type_params[0].name, "T");
        assert_eq!(doc.type_params[0].description, "The type to store");
        assert_eq!(doc.type_params[1].name, "U");
        assert_eq!(doc.type_params[1].description, "The key type");
    }

    #[test]
    fn test_parse_with_exceptions() {
        let raw = r#"
/// <summary>Performs division</summary>
/// <exception cref="System.DivideByZeroException">Thrown when divisor is zero</exception>
/// <exception cref="ArgumentException">Thrown for invalid arguments</exception>
"#;

        let doc = XmlDocumentation::parse(raw);
        assert_eq!(doc.exceptions.len(), 2);
        assert_eq!(doc.exceptions[0].cref, "System.DivideByZeroException");
        assert_eq!(doc.exceptions[0].description, "Thrown when divisor is zero");
        assert_eq!(doc.exceptions[1].cref, "ArgumentException");
        assert_eq!(doc.exceptions[1].description, "Thrown for invalid arguments");
    }

    #[test]
    fn test_parse_with_see_also() {
        let raw = r#"
/// <summary>Main method</summary>
/// <seealso cref="Helper"/>
/// <seealso cref="System.String"/>
"#;

        let doc = XmlDocumentation::parse(raw);
        assert_eq!(doc.see_also.len(), 2);
        assert_eq!(doc.see_also[0], "Helper");
        assert_eq!(doc.see_also[1], "System.String");
    }

    #[test]
    fn test_parse_multiline_summary() {
        let raw = r#"
/// <summary>
/// This is a long description
/// that spans multiple lines
/// and should be preserved
/// </summary>
"#;

        let doc = XmlDocumentation::parse(raw);
        let summary = doc.summary.unwrap();
        assert!(summary.contains("long description"));
        assert!(summary.contains("multiple lines"));
        assert!(summary.contains("preserved"));
    }

    #[test]
    fn test_empty_documentation() {
        let raw = "/// No XML tags here";
        let doc = XmlDocumentation::parse(raw);
        assert!(doc.is_empty());
        assert_eq!(doc.raw, raw);
    }

    #[test]
    fn test_malformed_xml() {
        let raw = r#"
/// <summary>
/// Unclosed tag
"#;

        let doc = XmlDocumentation::parse(raw);
        // Should not panic, just return None for unclosed tags
        assert!(doc.summary.is_none());
        assert!(!doc.raw.is_empty());
    }
}
