//! C# attribute (annotation) extraction and querying
//!
//! This module provides utilities for extracting C# attributes from code, including
//! their arguments, target symbols, and metadata. C# attributes are used for metadata
//! annotations similar to Java annotations or Python decorators.
//!
//! # Example
//!
//! ```csharp
//! [Serializable]
//! [Obsolete("Use NewClass", false)]
//! public class OldClass {
//!     [Required]
//!     [MaxLength(100)]
//!     public string Name { get; set; }
//! }
//! ```

use crate::types::{Range, SymbolKind};
use serde::{Deserialize, Serialize};

/// Information about a C# attribute (annotation)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeInfo {
    /// Attribute name (e.g., "Serializable", "HttpGet", "Required")
    pub name: String,

    /// Name of the symbol this attribute is applied to
    pub target: String,

    /// Kind of the target symbol (Class, Method, Property, etc.)
    pub target_kind: SymbolKind,

    /// Positional arguments passed to the attribute
    /// Example: `[Obsolete("message", false)]` → ["message", "false"]
    pub arguments: Vec<String>,

    /// Named arguments (property = value)
    /// Example: `[Authorize(Roles = "Admin")]` → {("Roles", "Admin")}
    pub named_arguments: Vec<(String, String)>,

    /// Source location of the attribute
    pub range: Range,
}

impl AttributeInfo {
    /// Create a new attribute
    pub fn new(
        name: String,
        target: String,
        target_kind: SymbolKind,
        range: Range,
    ) -> Self {
        Self {
            name,
            target,
            target_kind,
            arguments: Vec::new(),
            named_arguments: Vec::new(),
            range,
        }
    }

    /// Add a positional argument
    pub fn with_argument(mut self, arg: String) -> Self {
        self.arguments.push(arg);
        self
    }

    /// Add a named argument
    pub fn with_named_argument(mut self, name: String, value: String) -> Self {
        self.named_arguments.push((name, value));
        self
    }

    /// Check if this attribute has any arguments
    pub fn has_arguments(&self) -> bool {
        !self.arguments.is_empty() || !self.named_arguments.is_empty()
    }

    /// Get the number of positional arguments
    pub fn argument_count(&self) -> usize {
        self.arguments.len()
    }

    /// Get a named argument value by name
    pub fn get_named_argument(&self, name: &str) -> Option<&str> {
        self.named_arguments
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Collection of attributes with query methods
#[derive(Debug, Clone, Default)]
pub struct AttributeCollection {
    attributes: Vec<AttributeInfo>,
}

impl AttributeCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from a vector of attributes
    pub fn from_vec(attributes: Vec<AttributeInfo>) -> Self {
        Self { attributes }
    }

    /// Add an attribute to the collection
    pub fn add(&mut self, attribute: AttributeInfo) {
        self.attributes.push(attribute);
    }

    /// Get all attributes
    pub fn all(&self) -> &[AttributeInfo] {
        &self.attributes
    }

    /// Get the number of attributes
    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }

    /// Filter attributes by name
    ///
    /// # Example
    ///
    /// ```
    /// # use codanna::parsing::csharp::attributes::AttributeCollection;
    /// // Get all HttpGet attributes
    /// let http_gets = collection.by_name("HttpGet");
    /// ```
    pub fn by_name(&self, name: &str) -> Vec<&AttributeInfo> {
        self.attributes
            .iter()
            .filter(|attr| attr.name == name)
            .collect()
    }

    /// Filter attributes by target symbol name
    pub fn by_target(&self, target: &str) -> Vec<&AttributeInfo> {
        self.attributes
            .iter()
            .filter(|attr| attr.target == target)
            .collect()
    }

    /// Filter attributes by target kind
    ///
    /// # Example
    ///
    /// ```
    /// # use codanna::parsing::csharp::attributes::AttributeCollection;
    /// # use codanna::types::SymbolKind;
    /// // Get all attributes applied to classes
    /// let class_attrs = collection.by_target_kind(SymbolKind::Class);
    /// ```
    pub fn by_target_kind(&self, kind: SymbolKind) -> Vec<&AttributeInfo> {
        self.attributes
            .iter()
            .filter(|attr| attr.target_kind == kind)
            .collect()
    }

    /// Find attributes with a specific named argument
    ///
    /// # Example
    ///
    /// ```
    /// # use codanna::parsing::csharp::attributes::AttributeCollection;
    /// // Find all attributes with Roles = "Admin"
    /// let admin_attrs = collection.with_named_argument("Roles", "Admin");
    /// ```
    pub fn with_named_argument(&self, name: &str, value: &str) -> Vec<&AttributeInfo> {
        self.attributes
            .iter()
            .filter(|attr| {
                attr.named_arguments
                    .iter()
                    .any(|(n, v)| n == name && v == value)
            })
            .collect()
    }

    /// Find attributes that have any named argument with the given name
    pub fn with_named_argument_name(&self, name: &str) -> Vec<&AttributeInfo> {
        self.attributes
            .iter()
            .filter(|attr| attr.named_arguments.iter().any(|(n, _)| n == name))
            .collect()
    }

    /// Get all unique attribute names in the collection
    pub fn unique_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self
            .attributes
            .iter()
            .map(|attr| attr.name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Group attributes by target symbol
    pub fn group_by_target(&self) -> Vec<(String, Vec<&AttributeInfo>)> {
        let mut groups: std::collections::HashMap<String, Vec<&AttributeInfo>> =
            std::collections::HashMap::new();

        for attr in &self.attributes {
            groups
                .entry(attr.target.clone())
                .or_default()
                .push(attr);
        }

        groups.into_iter().collect()
    }
}

impl From<Vec<AttributeInfo>> for AttributeCollection {
    fn from(attributes: Vec<AttributeInfo>) -> Self {
        Self::from_vec(attributes)
    }
}

impl IntoIterator for AttributeCollection {
    type Item = AttributeInfo;
    type IntoIter = std::vec::IntoIter<AttributeInfo>;

    fn into_iter(self) -> Self::IntoIter {
        self.attributes.into_iter()
    }
}

impl<'a> IntoIterator for &'a AttributeCollection {
    type Item = &'a AttributeInfo;
    type IntoIter = std::slice::Iter<'a, AttributeInfo>;

    fn into_iter(self) -> Self::IntoIter {
        self.attributes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_attribute(
        name: &str,
        target: &str,
        target_kind: SymbolKind,
    ) -> AttributeInfo {
        AttributeInfo::new(
            name.to_string(),
            target.to_string(),
            target_kind,
            Range::new(0, 0, 0, 0),
        )
    }

    #[test]
    fn test_attribute_with_arguments() {
        let attr = create_test_attribute("Obsolete", "OldClass", SymbolKind::Class)
            .with_argument("Use NewClass".to_string())
            .with_argument("false".to_string());

        assert_eq!(attr.argument_count(), 2);
        assert!(attr.has_arguments());
        assert_eq!(attr.arguments[0], "Use NewClass");
        assert_eq!(attr.arguments[1], "false");
    }

    #[test]
    fn test_attribute_with_named_arguments() {
        let attr = create_test_attribute("Authorize", "GetUser", SymbolKind::Method)
            .with_named_argument("Roles".to_string(), "Admin".to_string());

        assert_eq!(attr.named_arguments.len(), 1);
        assert_eq!(attr.get_named_argument("Roles"), Some("Admin"));
        assert_eq!(attr.get_named_argument("Other"), None);
    }

    #[test]
    fn test_collection_by_name() {
        let mut collection = AttributeCollection::new();
        collection.add(create_test_attribute("Required", "Name", SymbolKind::Field));
        collection.add(create_test_attribute("MaxLength", "Name", SymbolKind::Field));
        collection.add(create_test_attribute("Required", "Email", SymbolKind::Field));

        let required_attrs = collection.by_name("Required");
        assert_eq!(required_attrs.len(), 2);
    }

    #[test]
    fn test_collection_by_target() {
        let mut collection = AttributeCollection::new();
        collection.add(create_test_attribute("Required", "Name", SymbolKind::Field));
        collection.add(create_test_attribute("MaxLength", "Name", SymbolKind::Field));
        collection.add(create_test_attribute("Required", "Email", SymbolKind::Field));

        let name_attrs = collection.by_target("Name");
        assert_eq!(name_attrs.len(), 2);
    }

    #[test]
    fn test_collection_by_target_kind() {
        let mut collection = AttributeCollection::new();
        collection.add(create_test_attribute("Serializable", "MyClass", SymbolKind::Class));
        collection.add(create_test_attribute("Required", "Name", SymbolKind::Field));
        collection.add(create_test_attribute("HttpGet", "GetUser", SymbolKind::Method));

        let class_attrs = collection.by_target_kind(SymbolKind::Class);
        assert_eq!(class_attrs.len(), 1);
        assert_eq!(class_attrs[0].name, "Serializable");
    }

    #[test]
    fn test_collection_unique_names() {
        let mut collection = AttributeCollection::new();
        collection.add(create_test_attribute("Required", "Name", SymbolKind::Field));
        collection.add(create_test_attribute("MaxLength", "Name", SymbolKind::Field));
        collection.add(create_test_attribute("Required", "Email", SymbolKind::Field));

        let names = collection.unique_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Required".to_string()));
        assert!(names.contains(&"MaxLength".to_string()));
    }

    #[test]
    fn test_collection_with_named_argument() {
        let mut collection = AttributeCollection::new();

        let attr1 = create_test_attribute("Authorize", "GetUser", SymbolKind::Method)
            .with_named_argument("Roles".to_string(), "Admin".to_string());
        let attr2 = create_test_attribute("Authorize", "DeleteUser", SymbolKind::Method)
            .with_named_argument("Roles".to_string(), "SuperAdmin".to_string());
        let attr3 = create_test_attribute("Authorize", "ViewUser", SymbolKind::Method)
            .with_named_argument("Roles".to_string(), "Admin".to_string());

        collection.add(attr1);
        collection.add(attr2);
        collection.add(attr3);

        let admin_attrs = collection.with_named_argument("Roles", "Admin");
        assert_eq!(admin_attrs.len(), 2);
    }

    #[test]
    fn test_empty_collection() {
        let collection = AttributeCollection::new();
        assert!(collection.is_empty());
        assert_eq!(collection.len(), 0);
    }
}
