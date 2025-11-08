use codanna::parsing::csharp::{CSharpParser, GenericConstraint, Variance};
use codanna::parsing::LanguageParser;
use codanna::types::{FileId, SymbolCounter};

#[test]
fn test_extract_generic_info_from_simple_class() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class Container<T>
{
    public void Add(T item) { }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let container = symbols.iter().find(|s| &*s.name == "Container").unwrap();
    let generic_info = parser.get_generic_info(container.signature.as_ref().unwrap());

    assert!(generic_info.is_generic);
    assert_eq!(generic_info.param_count(), 1);
    assert_eq!(generic_info.type_parameters[0].name, "T");
    assert!(generic_info.type_parameters[0].constraints.is_empty());
}

#[test]
fn test_extract_generic_info_with_constraints() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class Repository<TEntity> where TEntity : class, IEntity, new()
{
    public void Save(TEntity entity) { }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let repo = symbols.iter().find(|s| &*s.name == "Repository").unwrap();
    let generic_info = parser.get_generic_info(repo.signature.as_ref().unwrap());

    assert!(generic_info.is_generic);
    assert_eq!(generic_info.param_count(), 1);

    let constraints = &generic_info.type_parameters[0].constraints;
    assert_eq!(constraints.len(), 3);
    assert!(constraints.contains(&GenericConstraint::ReferenceType));
    assert!(constraints.contains(&GenericConstraint::Interface("IEntity".to_string())));
    assert!(constraints.contains(&GenericConstraint::Constructor));
}

#[test]
fn test_extract_generic_info_from_dictionary() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class Dictionary<TKey, TValue>
    where TKey : IComparable
    where TValue : class
{
    public void Add(TKey key, TValue value) { }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let dict = symbols.iter().find(|s| &*s.name == "Dictionary").unwrap();
    let generic_info = parser.get_generic_info(dict.signature.as_ref().unwrap());

    assert_eq!(generic_info.param_count(), 2);

    // TKey constraints
    let key_param = generic_info.get_param("TKey").unwrap();
    assert_eq!(key_param.constraints.len(), 1);
    assert_eq!(
        key_param.constraints[0],
        GenericConstraint::Interface("IComparable".to_string())
    );

    // TValue constraints
    let value_param = generic_info.get_param("TValue").unwrap();
    assert_eq!(value_param.constraints.len(), 1);
    assert_eq!(value_param.constraints[0], GenericConstraint::ReferenceType);
}

#[test]
fn test_extract_generic_info_from_method() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class Utilities
{
    public T GetValue<T>() where T : struct
    {
        return default(T);
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let method = symbols.iter().find(|s| &*s.name == "GetValue").unwrap();
    let generic_info = parser.get_generic_info(method.signature.as_ref().unwrap());

    assert!(generic_info.is_generic);
    assert_eq!(generic_info.param_count(), 1);
    assert_eq!(generic_info.type_parameters[0].name, "T");
    assert_eq!(
        generic_info.type_parameters[0].constraints[0],
        GenericConstraint::ValueType
    );
}

#[test]
fn test_extract_generic_info_with_variance() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public interface IEnumerable<out T>
{
    T GetNext();
}

public interface IComparer<in T>
{
    int Compare(T x, T y);
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Check covariant
    let enumerable = symbols.iter().find(|s| &*s.name == "IEnumerable").unwrap();
    let enum_info = parser.get_generic_info(enumerable.signature.as_ref().unwrap());
    assert_eq!(enum_info.type_parameters[0].variance, Variance::Covariant);

    // Check contravariant
    let comparer = symbols.iter().find(|s| &*s.name == "IComparer").unwrap();
    let comp_info = parser.get_generic_info(comparer.signature.as_ref().unwrap());
    assert_eq!(comp_info.type_parameters[0].variance, Variance::Contravariant);
}

#[test]
fn test_extract_generic_info_complex_constraints() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class DataProcessor<TInput, TOutput>
    where TInput : class, ISerializable, new()
    where TOutput : struct
{
    public TOutput Process(TInput input) { return default(TOutput); }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let processor = symbols.iter().find(|s| &*s.name == "DataProcessor").unwrap();
    let generic_info = parser.get_generic_info(processor.signature.as_ref().unwrap());

    assert_eq!(generic_info.param_count(), 2);

    // TInput should have 3 constraints
    let input_param = generic_info.get_param("TInput").unwrap();
    assert_eq!(input_param.constraints.len(), 3);
    assert!(input_param.constraints.contains(&GenericConstraint::ReferenceType));
    assert!(input_param
        .constraints
        .contains(&GenericConstraint::Interface("ISerializable".to_string())));
    assert!(input_param.constraints.contains(&GenericConstraint::Constructor));

    // TOutput should have struct constraint
    let output_param = generic_info.get_param("TOutput").unwrap();
    assert_eq!(output_param.constraints.len(), 1);
    assert_eq!(output_param.constraints[0], GenericConstraint::ValueType);
}

#[test]
fn test_non_generic_class_has_no_generic_info() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class SimpleClass
{
    public void DoWork() { }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let simple = symbols.iter().find(|s| &*s.name == "SimpleClass").unwrap();
    let generic_info = parser.get_generic_info(simple.signature.as_ref().unwrap());

    assert!(!generic_info.is_generic);
    assert_eq!(generic_info.param_count(), 0);
    assert!(!generic_info.has_type_parameters());
}

#[test]
fn test_extract_generic_info_from_nested_generic() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class Outer<TOuter>
{
    public class Inner<TInner>
    {
        public void Process(TOuter outer, TInner inner) { }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let outer = symbols.iter().find(|s| &*s.name == "Outer").unwrap();
    let outer_info = parser.get_generic_info(outer.signature.as_ref().unwrap());
    assert_eq!(outer_info.param_count(), 1);
    assert_eq!(outer_info.type_parameters[0].name, "TOuter");

    let inner = symbols.iter().find(|s| &*s.name == "Inner").unwrap();
    let inner_info = parser.get_generic_info(inner.signature.as_ref().unwrap());
    assert_eq!(inner_info.param_count(), 1);
    assert_eq!(inner_info.type_parameters[0].name, "TInner");
}

#[test]
fn test_generic_method_in_generic_class() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class Container<T>
{
    public U Convert<U>(T value) where U : class
    {
        return null;
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Class has T parameter
    let container = symbols.iter().find(|s| &*s.name == "Container").unwrap();
    let class_info = parser.get_generic_info(container.signature.as_ref().unwrap());
    assert_eq!(class_info.param_count(), 1);
    assert_eq!(class_info.type_parameters[0].name, "T");

    // Method has U parameter with constraint
    let convert = symbols.iter().find(|s| &*s.name == "Convert").unwrap();
    let method_info = parser.get_generic_info(convert.signature.as_ref().unwrap());
    assert_eq!(method_info.param_count(), 1);
    assert_eq!(method_info.type_parameters[0].name, "U");
    assert_eq!(
        method_info.type_parameters[0].constraints[0],
        GenericConstraint::ReferenceType
    );
}

#[test]
fn test_base_class_constraint_detection() {
    let mut parser = CSharpParser::new().unwrap();
    let code = r#"
public class Repository<T> where T : BaseEntity
{
    public void Save(T entity) { }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    let repo = symbols.iter().find(|s| &*s.name == "Repository").unwrap();
    let generic_info = parser.get_generic_info(repo.signature.as_ref().unwrap());

    let t_param = generic_info.get_param("T").unwrap();
    assert_eq!(t_param.constraints.len(), 1);
    assert_eq!(
        t_param.constraints[0],
        GenericConstraint::BaseClass("BaseEntity".to_string())
    );
}
