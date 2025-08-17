//! Tests for Python parser scope tracking

use codanna::parsing::{LanguageParser, PythonParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;
use codanna::{FileId, SymbolKind};

#[test]
fn test_python_module_level_symbols() {
    let mut parser = PythonParser::new().unwrap();
    let code = r#"
# Module-level constant
MAX_SIZE = 100

# Module-level variable
count = 0

def module_func():
    """Module function"""
    pass

class MyClass:
    """Module class"""
    pass
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Find specific symbols and check their scope
    let max_size = symbols.iter().find(|s| s.name.as_ref() == "MAX_SIZE");
    assert!(max_size.is_some());
    assert_eq!(max_size.unwrap().scope_context, Some(ScopeContext::Module));
    assert_eq!(max_size.unwrap().kind, SymbolKind::Constant);

    let count = symbols.iter().find(|s| s.name.as_ref() == "count");
    assert!(count.is_some());
    assert_eq!(count.unwrap().scope_context, Some(ScopeContext::Module));
    assert_eq!(count.unwrap().kind, SymbolKind::Variable);

    let func = symbols.iter().find(|s| s.name.as_ref() == "module_func");
    assert!(func.is_some());
    assert_eq!(func.unwrap().scope_context, Some(ScopeContext::Module));
    assert_eq!(func.unwrap().kind, SymbolKind::Function);

    let cls = symbols.iter().find(|s| s.name.as_ref() == "MyClass");
    assert!(cls.is_some());
    assert_eq!(cls.unwrap().scope_context, Some(ScopeContext::Module));
    assert_eq!(cls.unwrap().kind, SymbolKind::Class);
}

#[test]
fn test_python_class_member_scope() {
    let mut parser = PythonParser::new().unwrap();
    let code = r#"
class MyClass:
    """Test class"""

    def __init__(self):
        """Constructor"""
        self.instance_var = 0

    def method(self):
        """Instance method"""
        return self.instance_var

    @classmethod
    def class_method(cls):
        """Class method"""
        pass
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Class itself should be module-level
    let cls = symbols.iter().find(|s| s.name.as_ref() == "MyClass");
    assert!(cls.is_some());
    assert_eq!(cls.unwrap().scope_context, Some(ScopeContext::Module));

    // Methods should be class members
    let init = symbols.iter().find(|s| s.name.as_ref() == "__init__");
    assert!(init.is_some());
    assert_eq!(init.unwrap().scope_context, Some(ScopeContext::ClassMember));
    assert_eq!(init.unwrap().kind, SymbolKind::Method);

    let method = symbols.iter().find(|s| s.name.as_ref() == "method");
    assert!(method.is_some());
    assert_eq!(
        method.unwrap().scope_context,
        Some(ScopeContext::ClassMember)
    );

    let class_method = symbols.iter().find(|s| s.name.as_ref() == "class_method");
    assert!(class_method.is_some());
    assert_eq!(
        class_method.unwrap().scope_context,
        Some(ScopeContext::ClassMember)
    );
}

#[test]
fn test_python_nested_function_scope() {
    let mut parser = PythonParser::new().unwrap();
    let code = r#"
def outer():
    """Outer function"""

    def inner():
        """Inner function"""

        def deeply_nested():
            """Deeply nested function"""
            pass

        return deeply_nested

    return inner
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Outer function is module-level
    let outer = symbols.iter().find(|s| s.name.as_ref() == "outer");
    assert!(outer.is_some());
    assert_eq!(outer.unwrap().scope_context, Some(ScopeContext::Module));

    // Inner function is local to outer
    let inner = symbols.iter().find(|s| s.name.as_ref() == "inner");
    assert!(inner.is_some());
    assert_eq!(
        inner.unwrap().scope_context,
        Some(ScopeContext::Local {
            hoisted: false,
            parent_name: Some("outer".into()),
            parent_kind: Some(SymbolKind::Function)
        })
    );

    // Deeply nested is local to inner
    let deeply = symbols.iter().find(|s| s.name.as_ref() == "deeply_nested");
    assert!(deeply.is_some());
    assert_eq!(
        deeply.unwrap().scope_context,
        Some(ScopeContext::Local {
            hoisted: false,
            parent_name: Some("inner".into()),
            parent_kind: Some(SymbolKind::Function)
        })
    );
}

#[test]
fn test_python_mixed_scopes() {
    let mut parser = PythonParser::new().unwrap();
    let code = r#"
GLOBAL_CONST = 42

def process_data():
    local_var = 10

    class LocalClass:
        def local_method(self):
            pass

    return LocalClass()

class GlobalClass:
    CLASS_VAR = 100

    def method(self):
        method_local = 5
        return method_local
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    // Global constant
    let global_const = symbols.iter().find(|s| s.name.as_ref() == "GLOBAL_CONST");
    assert!(global_const.is_some());
    assert_eq!(
        global_const.unwrap().scope_context,
        Some(ScopeContext::Module)
    );

    // Global function
    let process = symbols.iter().find(|s| s.name.as_ref() == "process_data");
    assert!(process.is_some());
    assert_eq!(process.unwrap().scope_context, Some(ScopeContext::Module));

    // Class defined inside function (local scope)
    let local_class = symbols.iter().find(|s| s.name.as_ref() == "LocalClass");
    assert!(local_class.is_some());
    assert_eq!(
        local_class.unwrap().scope_context,
        Some(ScopeContext::Local {
            hoisted: false,
            parent_name: Some("process_data".into()),
            parent_kind: Some(SymbolKind::Function)
        })
    );

    // Global class
    let global_class = symbols.iter().find(|s| s.name.as_ref() == "GlobalClass");
    assert!(global_class.is_some());
    assert_eq!(
        global_class.unwrap().scope_context,
        Some(ScopeContext::Module)
    );
}
