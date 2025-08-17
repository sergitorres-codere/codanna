//! Test to verify TypeScript interfaces and types have correct scope
//!
//! This test specifically addresses the bug where interfaces and type aliases
//! at module level are incorrectly marked as Local { hoisted: true } instead of Module

use codanna::parsing::{LanguageParser, TypeScriptParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;
use codanna::{FileId, SymbolKind};

#[test]
fn test_typescript_interface_module_scope() {
    let mut parser = TypeScriptParser::new().unwrap();
    let code = r#"
// These should ALL have Module scope, not Local
interface UserInterface {
    id: string;
    name: string;
}

type UserType = {
    id: string;
    name: string;
};

type StringOrNumber = string | number;

interface ExtendedInterface extends UserInterface {
    email: string;
}

// Function for comparison - should also be Module
function moduleFunction() {
    // This interface is truly local
    interface LocalInterface {
        value: number;
    }

    // This type is truly local
    type LocalType = string;

    return null;
}

// Class for comparison - should be Module
class UserClass {
    id: string;
    name: string;
}

// Enum should be Module
enum Status {
    Active,
    Inactive
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== TYPESCRIPT INTERFACE/TYPE SCOPE TEST ===\n");
    println!("Checking scope for all module-level types...\n");

    // Check module-level interface
    let user_interface = symbols
        .iter()
        .find(|s| s.name.as_ref() == "UserInterface")
        .expect("UserInterface should be found");

    println!(
        "UserInterface: kind={:?}, scope={:?}",
        user_interface.kind, user_interface.scope_context
    );
    assert_eq!(user_interface.kind, SymbolKind::Interface);
    assert_eq!(
        user_interface.scope_context,
        Some(ScopeContext::Module),
        "Module-level interface should have Module scope, not Local"
    );

    // Check module-level type alias
    let user_type = symbols
        .iter()
        .find(|s| s.name.as_ref() == "UserType")
        .expect("UserType should be found");

    println!(
        "UserType: kind={:?}, scope={:?}",
        user_type.kind, user_type.scope_context
    );
    assert_eq!(user_type.kind, SymbolKind::TypeAlias);
    assert_eq!(
        user_type.scope_context,
        Some(ScopeContext::Module),
        "Module-level type alias should have Module scope, not Local"
    );

    // Check union type alias
    let string_or_number = symbols
        .iter()
        .find(|s| s.name.as_ref() == "StringOrNumber")
        .expect("StringOrNumber should be found");

    println!(
        "StringOrNumber: kind={:?}, scope={:?}",
        string_or_number.kind, string_or_number.scope_context
    );
    assert_eq!(
        string_or_number.scope_context,
        Some(ScopeContext::Module),
        "Module-level type alias should have Module scope"
    );

    // Check extended interface
    let extended_interface = symbols
        .iter()
        .find(|s| s.name.as_ref() == "ExtendedInterface")
        .expect("ExtendedInterface should be found");

    println!(
        "ExtendedInterface: kind={:?}, scope={:?}",
        extended_interface.kind, extended_interface.scope_context
    );
    assert_eq!(
        extended_interface.scope_context,
        Some(ScopeContext::Module),
        "Module-level interface should have Module scope"
    );

    // Check that function is Module scope (for comparison)
    let module_func = symbols
        .iter()
        .find(|s| s.name.as_ref() == "moduleFunction")
        .expect("moduleFunction should be found");

    println!(
        "moduleFunction: kind={:?}, scope={:?}",
        module_func.kind, module_func.scope_context
    );
    assert_eq!(
        module_func.scope_context,
        Some(ScopeContext::Module),
        "Module-level function should have Module scope"
    );

    // Check that class is Module scope (for comparison)
    let user_class = symbols
        .iter()
        .find(|s| s.name.as_ref() == "UserClass")
        .expect("UserClass should be found");

    println!(
        "UserClass: kind={:?}, scope={:?}",
        user_class.kind, user_class.scope_context
    );
    assert_eq!(
        user_class.scope_context,
        Some(ScopeContext::Module),
        "Module-level class should have Module scope"
    );

    // Check that enum is Module scope
    let status_enum = symbols
        .iter()
        .find(|s| s.name.as_ref() == "Status")
        .expect("Status enum should be found");

    println!(
        "Status: kind={:?}, scope={:?}",
        status_enum.kind, status_enum.scope_context
    );
    assert_eq!(
        status_enum.scope_context,
        Some(ScopeContext::Module),
        "Module-level enum should have Module scope"
    );

    // Now check the truly local interface/type (inside function)
    let local_interface = symbols.iter().find(|s| s.name.as_ref() == "LocalInterface");

    if let Some(li) = local_interface {
        println!(
            "\nLocalInterface (inside function): kind={:?}, scope={:?}",
            li.kind, li.scope_context
        );
        // This one SHOULD be Local since it's inside a function
        assert_eq!(
            li.scope_context,
            Some(ScopeContext::Local {
                hoisted: true,
                parent_name: Some("moduleFunction".into()),
                parent_kind: Some(SymbolKind::Function)
            }),
            "Interface inside function should have Local scope with parent context"
        );
    }

    let local_type = symbols.iter().find(|s| s.name.as_ref() == "LocalType");

    if let Some(lt) = local_type {
        println!(
            "LocalType (inside function): kind={:?}, scope={:?}",
            lt.kind, lt.scope_context
        );
        // This one SHOULD be Local since it's inside a function
        assert_eq!(
            lt.scope_context,
            Some(ScopeContext::Local {
                hoisted: true,
                parent_name: Some("moduleFunction".into()),
                parent_kind: Some(SymbolKind::Function)
            }),
            "Type alias inside function should have Local scope with parent context"
        );
    }

    println!("\n=== TEST COMPLETE ===\n");
}

#[test]
fn test_typescript_nested_interface_scope() {
    let mut parser = TypeScriptParser::new().unwrap();
    let code = r#"
// Module level
namespace MyNamespace {
    // Should be Module (or potentially Package for namespace members)
    export interface NamespaceInterface {
        value: string;
    }

    export type NamespaceType = number;
}

// Module level function with nested types
export function processData() {
    // These are truly local to the function
    interface ProcessorInterface {
        process(): void;
    }

    type ProcessorType = {
        id: number;
    };

    class ProcessorClass {
        process() {}
    }

    return null;
}

// Module level
export interface ExportedInterface {
    data: any;
}

export type ExportedType = string[];
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== TYPESCRIPT NESTED INTERFACE SCOPE TEST ===\n");

    // Check exported interface
    let exported_interface = symbols
        .iter()
        .find(|s| s.name.as_ref() == "ExportedInterface");

    if let Some(ei) = exported_interface {
        println!(
            "ExportedInterface: kind={:?}, scope={:?}",
            ei.kind, ei.scope_context
        );
        assert_eq!(
            ei.scope_context,
            Some(ScopeContext::Module),
            "Exported interface should have Module scope"
        );
    }

    // Check exported type
    let exported_type = symbols.iter().find(|s| s.name.as_ref() == "ExportedType");

    if let Some(et) = exported_type {
        println!(
            "ExportedType: kind={:?}, scope={:?}",
            et.kind, et.scope_context
        );
        assert_eq!(
            et.scope_context,
            Some(ScopeContext::Module),
            "Exported type should have Module scope"
        );
    }

    println!("\n=== NESTED TEST COMPLETE ===\n");
}
