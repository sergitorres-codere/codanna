//! Test TypeScript type usage and method definition tracking
//!
//! This test file verifies that find_uses() and find_defines() work correctly
//! for TypeScript, tracking type usage and method definitions as required by
//! the LanguageParser trait.

use codanna::parsing::LanguageParser;
use codanna::parsing::typescript::TypeScriptParser;

#[test]
fn test_typescript_find_uses_proof() {
    println!("\n=== TypeScript find_uses() Proof Test ===\n");

    let code = r#"
// Function with type parameters and return type
function processUser(user: User): Result<User> {
    return { success: true, data: user };
}

// Class with field types and implements
class UserService implements IService {
    private client: HttpClient;
    private cache: Map<string, User>;
    
    async getUser(id: string): Promise<User> {
        return this.client.get(`/users/${id}`);
    }
}

// Interface extending another
interface AdminUser extends User {
    permissions: Permission[];
}

// Variable with type annotation
const config: AppConfig = {
    apiUrl: 'https://api.example.com',
    timeout: 5000
};

// Type alias
type Handler = (event: Event) => void;
"#;

    let mut parser = TypeScriptParser::new().expect("Failed to create parser");
    let uses = parser.find_uses(code);

    println!("Found {} type uses:", uses.len());
    for (context, used_type, range) in &uses {
        println!(
            "  '{}' uses type '{}' at line {}",
            context,
            used_type,
            range.start_line + 1
        );
    }

    // Verify function parameter type
    assert!(
        uses.iter()
            .any(|(ctx, typ, _)| ctx == &"processUser" && typ == &"User"),
        "Should find User type in processUser parameter"
    );

    // Verify function return type
    assert!(
        uses.iter()
            .any(|(ctx, typ, _)| ctx == &"processUser" && typ == &"Result"),
        "Should find Result type in processUser return"
    );

    // Verify class implements
    assert!(
        uses.iter()
            .any(|(ctx, typ, _)| ctx == &"UserService" && typ == &"IService"),
        "Should find IService in UserService implements"
    );

    // Verify class field types
    assert!(
        uses.iter()
            .any(|(ctx, typ, _)| ctx == &"UserService" && typ == &"HttpClient"),
        "Should find HttpClient type in UserService field"
    );

    assert!(
        uses.iter()
            .any(|(ctx, typ, _)| ctx == &"UserService" && typ == &"Map"),
        "Should find Map type in UserService field"
    );

    // Verify interface extends
    assert!(
        uses.iter()
            .any(|(ctx, typ, _)| ctx == &"AdminUser" && typ == &"User"),
        "Should find User type in AdminUser extends"
    );

    // Verify variable type annotation
    assert!(
        uses.iter()
            .any(|(ctx, typ, _)| ctx == &"config" && typ == &"AppConfig"),
        "Should find AppConfig type in config variable"
    );

    println!("\n✅ Type usage extraction verified");
}

#[test]
fn test_typescript_find_defines_proof() {
    println!("\n=== TypeScript find_defines() Proof Test ===\n");

    let code = r#"
// Interface with method signatures
interface IUserService {
    getUser(id: string): User;
    createUser(data: UserData): User;
    deleteUser(id: string): void;
}

// Class with method definitions
class UserService implements IUserService {
    getUser(id: string): User {
        // Implementation
        return {} as User;
    }
    
    createUser(data: UserData): User {
        // Implementation
        return {} as User;
    }
    
    deleteUser(id: string): void {
        // Implementation
    }
    
    private validateUser(user: User): boolean {
        return true;
    }
}

// Abstract class with abstract methods
abstract class BaseService {
    abstract connect(): void;
    abstract disconnect(): void;
    
    protected log(message: string): void {
        console.log(message);
    }
}

// Type alias with method signatures
type EventHandler = {
    onStart(): void;
    onStop(): void;
    onError(error: Error): void;
};
"#;

    let mut parser = TypeScriptParser::new().expect("Failed to create parser");
    let defines = parser.find_defines(code);

    println!("Found {} method definitions:", defines.len());
    for (definer, method, range) in &defines {
        println!(
            "  '{}' defines method '{}' at line {}",
            definer,
            method,
            range.start_line + 1
        );
    }

    // Verify interface method signatures
    assert!(
        defines
            .iter()
            .any(|(def, meth, _)| def == &"IUserService" && meth == &"getUser"),
        "Should find getUser in IUserService"
    );

    assert!(
        defines
            .iter()
            .any(|(def, meth, _)| def == &"IUserService" && meth == &"createUser"),
        "Should find createUser in IUserService"
    );

    assert!(
        defines
            .iter()
            .any(|(def, meth, _)| def == &"IUserService" && meth == &"deleteUser"),
        "Should find deleteUser in IUserService"
    );

    // Verify class method definitions
    assert!(
        defines
            .iter()
            .any(|(def, meth, _)| def == &"UserService" && meth == &"getUser"),
        "Should find getUser in UserService class"
    );

    assert!(
        defines
            .iter()
            .any(|(def, meth, _)| def == &"UserService" && meth == &"validateUser"),
        "Should find validateUser in UserService class"
    );

    // Verify abstract class methods
    assert!(
        defines
            .iter()
            .any(|(def, meth, _)| def == &"BaseService" && meth == &"connect"),
        "Should find connect in BaseService"
    );

    assert!(
        defines
            .iter()
            .any(|(def, meth, _)| def == &"BaseService" && meth == &"log"),
        "Should find log in BaseService"
    );

    // Verify type alias methods
    assert!(
        defines
            .iter()
            .any(|(def, meth, _)| def == &"EventHandler" && meth == &"onStart"),
        "Should find onStart in EventHandler type"
    );

    assert!(
        defines
            .iter()
            .any(|(def, meth, _)| def == &"EventHandler" && meth == &"onError"),
        "Should find onError in EventHandler type"
    );

    println!("\n✅ Method definition extraction verified");
}

#[test]
fn test_typescript_type_tracking_comprehensive() {
    println!("\n=== TypeScript Type Tracking Comprehensive Test ===\n");

    // Use the comprehensive.ts content
    let code = include_str!("../examples/typescript/comprehensive.ts");

    let mut parser = TypeScriptParser::new().expect("Failed to create parser");

    // Test find_uses
    let uses = parser.find_uses(code);
    println!("Comprehensive file type uses: {}", uses.len());

    // Should find User type usage (used in many places)
    assert!(
        uses.iter().any(|(_, typ, _)| typ == &"User"),
        "Should find User type usage in comprehensive.ts"
    );

    // Should find Promise type usage
    assert!(
        uses.iter().any(|(_, typ, _)| typ == &"Promise"),
        "Should find Promise type usage in comprehensive.ts"
    );

    // Test find_defines
    let defines = parser.find_defines(code);
    println!("Comprehensive file method definitions: {}", defines.len());

    // Should find method definitions in classes
    assert!(
        defines
            .iter()
            .any(|(def, meth, _)| def == &"SimpleClass" && meth == &"publicMethod"),
        "Should find publicMethod in SimpleClass"
    );

    println!("\n✅ Comprehensive type tracking verified");
}
