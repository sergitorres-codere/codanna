//! Generic type information extraction for C#
//!
//! This module provides utilities for extracting and querying generic type parameters
//! and constraints from C# code. C# supports complex generic constraints including:
//! - Base class constraints (`where T : BaseClass`)
//! - Interface constraints (`where T : IInterface`)
//! - Constructor constraints (`where T : new()`)
//! - Reference/value type constraints (`where T : class`, `where T : struct`)
//!
//! # Example
//!
//! ```csharp
//! public class Container<T, U>
//!     where T : class, IDisposable, new()
//!     where U : struct
//! {
//!     public void Method<V>() where V : T { }
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Information about a generic type parameter
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericTypeParam {
    /// Parameter name (e.g., "T", "TKey", "TValue")
    pub name: String,

    /// Constraints on this type parameter
    pub constraints: Vec<GenericConstraint>,

    /// Variance modifier (in, out, none)
    pub variance: Variance,
}

/// Generic type constraint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericConstraint {
    /// Base class constraint (`where T : BaseClass`)
    BaseClass(String),

    /// Interface constraint (`where T : IInterface`)
    Interface(String),

    /// Reference type constraint (`where T : class`)
    ReferenceType,

    /// Value type constraint (`where T : struct`)
    ValueType,

    /// Constructor constraint (`where T : new()`)
    Constructor,

    /// Unmanaged constraint (`where T : unmanaged`)
    Unmanaged,

    /// Not null constraint (`where T : notnull`)
    NotNull,
}

/// Variance modifier for generic type parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Variance {
    /// No variance modifier
    None,

    /// Covariant (`out T`)
    Covariant,

    /// Contravariant (`in T`)
    Contravariant,
}

/// Information about generic types in a symbol (class or method)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GenericInfo {
    /// List of generic type parameters
    pub type_parameters: Vec<GenericTypeParam>,

    /// Whether this is a generic type/method
    pub is_generic: bool,
}

impl GenericInfo {
    /// Create a new empty GenericInfo
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this has generic type parameters
    pub fn has_type_parameters(&self) -> bool {
        !self.type_parameters.is_empty()
    }

    /// Get the number of type parameters
    pub fn param_count(&self) -> usize {
        self.type_parameters.len()
    }

    /// Get a type parameter by name
    pub fn get_param(&self, name: &str) -> Option<&GenericTypeParam> {
        self.type_parameters.iter().find(|p| p.name == name)
    }

    /// Parse generic information from a C# signature
    ///
    /// Extracts generic type parameters and their constraints from signatures like:
    /// - `public class Foo<T, U>`
    /// - `public T Method<T>() where T : class`
    /// - `public class Container<T> where T : IDisposable, new()`
    ///
    /// # Example
    ///
    /// ```
    /// use codanna::parsing::csharp::generic_types::GenericInfo;
    ///
    /// let sig = "public class Container<T, U> where T : class where U : struct";
    /// let info = GenericInfo::from_signature(sig);
    ///
    /// assert_eq!(info.param_count(), 2);
    /// assert_eq!(info.type_parameters[0].name, "T");
    /// ```
    pub fn from_signature(signature: &str) -> Self {
        let mut info = GenericInfo::new();

        // Extract type parameters from angle brackets
        if let Some(params) = Self::extract_type_parameter_names(signature) {
            info.type_parameters = params
                .iter()
                .map(|name| GenericTypeParam {
                    name: name.to_string(),
                    constraints: Vec::new(),
                    variance: Variance::None,
                })
                .collect();
        }

        // Parse where clauses for constraints
        info.parse_where_clauses(signature);

        // Parse variance modifiers
        info.parse_variance(signature);

        info.is_generic = !info.type_parameters.is_empty();
        info
    }

    /// Extract type parameter names from angle brackets
    ///
    /// Example: `Foo<T, U, V>` → ["T", "U", "V"]
    fn extract_type_parameter_names(signature: &str) -> Option<Vec<String>> {
        // Find first < and matching >
        let start = signature.find('<')?;
        let mut depth = 0;
        let mut end = start;

        for (i, ch) in signature[start..].chars().enumerate() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + i;
                        break;
                    }
                }
                _ => {}
            }
        }

        if end > start {
            let params_str = &signature[start + 1..end];
            let params: Vec<String> = params_str
                .split(',')
                .map(|s| {
                    // Strip variance modifiers (in, out)
                    let trimmed = s.trim();
                    if let Some(name) = trimmed.strip_prefix("in ") {
                        name.trim().to_string()
                    } else if let Some(name) = trimmed.strip_prefix("out ") {
                        name.trim().to_string()
                    } else {
                        trimmed.to_string()
                    }
                })
                .filter(|s| !s.is_empty())
                .collect();

            if !params.is_empty() {
                return Some(params);
            }
        }

        None
    }

    /// Parse where clauses and populate constraints
    ///
    /// Example: `where T : class, IDisposable, new()` → parses all constraints for T
    fn parse_where_clauses(&mut self, signature: &str) {
        // Find all "where" clauses
        let mut search_pos = 0;
        while let Some(where_pos) = signature[search_pos..].find(" where ") {
            let absolute_pos = search_pos + where_pos + 7; // len(" where ")

            // Extract parameter name after "where"
            if let Some(param_name_end) = signature[absolute_pos..].find(':') {
                let param_name = signature[absolute_pos..absolute_pos + param_name_end]
                    .trim()
                    .to_string();

                // Find end of constraint list (next "where" or end of signature)
                let constraints_start = absolute_pos + param_name_end + 1;
                let constraints_end = signature[constraints_start..]
                    .find(" where ")
                    .map(|pos| constraints_start + pos)
                    .or_else(|| signature[constraints_start..].find('{').map(|pos| constraints_start + pos))
                    .unwrap_or(signature.len());

                let constraints_str = &signature[constraints_start..constraints_end];

                // Parse individual constraints
                let constraints = Self::parse_constraint_list(constraints_str);

                // Apply constraints to the matching parameter
                if let Some(param) = self
                    .type_parameters
                    .iter_mut()
                    .find(|p| p.name == param_name)
                {
                    param.constraints = constraints;
                }

                search_pos = constraints_end;
            } else {
                break;
            }
        }
    }

    /// Parse a comma-separated list of constraints
    ///
    /// Example: `class, IDisposable, new()` → [ReferenceType, Interface("IDisposable"), Constructor]
    fn parse_constraint_list(constraints_str: &str) -> Vec<GenericConstraint> {
        constraints_str
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                match trimmed {
                    "class" => Some(GenericConstraint::ReferenceType),
                    "struct" => Some(GenericConstraint::ValueType),
                    "new()" => Some(GenericConstraint::Constructor),
                    "unmanaged" => Some(GenericConstraint::Unmanaged),
                    "notnull" => Some(GenericConstraint::NotNull),
                    other if !other.is_empty() => {
                        // Assume it's an interface if it starts with I, otherwise base class
                        if other.starts_with('I') && other.len() > 1 {
                            Some(GenericConstraint::Interface(other.to_string()))
                        } else {
                            Some(GenericConstraint::BaseClass(other.to_string()))
                        }
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Parse variance modifiers (in/out) from the signature
    fn parse_variance(&mut self, signature: &str) {
        // Find type parameter declaration with variance
        if let Some(start) = signature.find('<') {
            let params_str = &signature[start..];

            for param in &mut self.type_parameters {
                // Look for "in ParamName" or "out ParamName"
                let in_pattern = format!("in {}", param.name);
                let out_pattern = format!("out {}", param.name);

                if params_str.contains(&in_pattern) {
                    param.variance = Variance::Contravariant;
                } else if params_str.contains(&out_pattern) {
                    param.variance = Variance::Covariant;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_generic_class() {
        let sig = "public class Container<T>";
        let info = GenericInfo::from_signature(sig);

        assert!(info.is_generic);
        assert_eq!(info.param_count(), 1);
        assert_eq!(info.type_parameters[0].name, "T");
        assert!(info.type_parameters[0].constraints.is_empty());
    }

    #[test]
    fn test_multiple_type_parameters() {
        let sig = "public class Dictionary<TKey, TValue>";
        let info = GenericInfo::from_signature(sig);

        assert_eq!(info.param_count(), 2);
        assert_eq!(info.type_parameters[0].name, "TKey");
        assert_eq!(info.type_parameters[1].name, "TValue");
    }

    #[test]
    fn test_class_constraint() {
        let sig = "public class Container<T> where T : class";
        let info = GenericInfo::from_signature(sig);

        assert_eq!(info.type_parameters[0].constraints.len(), 1);
        assert_eq!(
            info.type_parameters[0].constraints[0],
            GenericConstraint::ReferenceType
        );
    }

    #[test]
    fn test_struct_constraint() {
        let sig = "public class Container<T> where T : struct";
        let info = GenericInfo::from_signature(sig);

        assert_eq!(info.type_parameters[0].constraints.len(), 1);
        assert_eq!(
            info.type_parameters[0].constraints[0],
            GenericConstraint::ValueType
        );
    }

    #[test]
    fn test_multiple_constraints() {
        let sig = "public class Container<T> where T : class, IDisposable, new()";
        let info = GenericInfo::from_signature(sig);

        let constraints = &info.type_parameters[0].constraints;
        assert_eq!(constraints.len(), 3);
        assert!(constraints.contains(&GenericConstraint::ReferenceType));
        assert!(constraints.contains(&GenericConstraint::Interface("IDisposable".to_string())));
        assert!(constraints.contains(&GenericConstraint::Constructor));
    }

    #[test]
    fn test_multiple_where_clauses() {
        let sig = "public class Container<T, U> where T : class where U : struct";
        let info = GenericInfo::from_signature(sig);

        assert_eq!(info.param_count(), 2);
        assert_eq!(
            info.type_parameters[0].constraints[0],
            GenericConstraint::ReferenceType
        );
        assert_eq!(
            info.type_parameters[1].constraints[0],
            GenericConstraint::ValueType
        );
    }

    #[test]
    fn test_variance_covariant() {
        let sig = "public interface IEnumerable<out T>";
        let info = GenericInfo::from_signature(sig);

        assert_eq!(info.type_parameters[0].variance, Variance::Covariant);
    }

    #[test]
    fn test_variance_contravariant() {
        let sig = "public interface IComparer<in T>";
        let info = GenericInfo::from_signature(sig);

        assert_eq!(info.type_parameters[0].variance, Variance::Contravariant);
    }

    #[test]
    fn test_generic_method() {
        let sig = "public T GetValue<T>() where T : class";
        let info = GenericInfo::from_signature(sig);

        assert!(info.is_generic);
        assert_eq!(info.param_count(), 1);
        assert_eq!(info.type_parameters[0].name, "T");
        assert_eq!(
            info.type_parameters[0].constraints[0],
            GenericConstraint::ReferenceType
        );
    }

    #[test]
    fn test_get_param_by_name() {
        let sig = "public class Container<T, U>";
        let info = GenericInfo::from_signature(sig);

        let param_t = info.get_param("T");
        assert!(param_t.is_some());
        assert_eq!(param_t.unwrap().name, "T");

        let param_u = info.get_param("U");
        assert!(param_u.is_some());
        assert_eq!(param_u.unwrap().name, "U");

        assert!(info.get_param("V").is_none());
    }

    #[test]
    fn test_base_class_constraint() {
        let sig = "public class MyList<T> where T : BaseItem";
        let info = GenericInfo::from_signature(sig);

        assert_eq!(info.type_parameters[0].constraints.len(), 1);
        assert_eq!(
            info.type_parameters[0].constraints[0],
            GenericConstraint::BaseClass("BaseItem".to_string())
        );
    }

    #[test]
    fn test_non_generic_class() {
        let sig = "public class SimpleClass";
        let info = GenericInfo::from_signature(sig);

        assert!(!info.is_generic);
        assert_eq!(info.param_count(), 0);
    }

    #[test]
    fn test_complex_generic_signature() {
        let sig = "public class Repository<TEntity, TKey> where TEntity : class, IEntity<TKey>, new() where TKey : struct";
        let info = GenericInfo::from_signature(sig);

        assert_eq!(info.param_count(), 2);

        let entity_constraints = &info.type_parameters[0].constraints;
        assert_eq!(entity_constraints.len(), 3);
        assert!(entity_constraints.contains(&GenericConstraint::ReferenceType));
        assert!(entity_constraints.contains(&GenericConstraint::Interface("IEntity<TKey>".to_string())));
        assert!(entity_constraints.contains(&GenericConstraint::Constructor));

        let key_constraints = &info.type_parameters[1].constraints;
        assert_eq!(key_constraints.len(), 1);
        assert_eq!(key_constraints[0], GenericConstraint::ValueType);
    }
}
