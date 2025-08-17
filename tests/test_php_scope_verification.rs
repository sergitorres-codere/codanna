//! Verification tests for PHP parser scope tracking with detailed debug output
//!
//! This test shows exactly what scope context is being stored for each PHP symbol
//! to verify the parser is correctly tracking scope at all levels.

use codanna::parsing::{LanguageParser, PhpParser};
use codanna::symbol::ScopeContext;
use codanna::types::SymbolCounter;
use codanna::{FileId, SymbolKind};

#[test]
fn verify_php_scope_with_detailed_output() {
    let mut parser = PhpParser::new().unwrap();
    let code = r#"<?php
// File-level namespace
namespace App\Models;

// Namespace-level use statements
use App\Traits\Timestampable;
use App\Contracts\Cacheable as CacheContract;

// Module-level constant
const MAX_RECORDS = 1000;
define('VERSION', '2.0.0');

// Module-level variables
$globalConfig = ['debug' => true];
$GLOBALS['app_name'] = 'MyApp';

// Module-level function
function moduleHelper($param) {
    // Function local variable
    $localVar = 42;
    
    // Closure (anonymous function)
    $closure = function($x) use ($localVar) {
        // Closure local variable
        $closureLocal = $x * 2;
        return $closureLocal + $localVar;
    };
    
    // Nested function definition (creates at module level in PHP)
    function nestedFunction() {
        $nestedLocal = 'nested';
        return $nestedLocal;
    }
    
    return $closure($param);
}

// Module-level class
class User {
    // Class properties
    private $id;
    protected $email;
    public $name;
    
    // Class constant
    const STATUS_ACTIVE = 'active';
    const STATUS_INACTIVE = 'inactive';
    
    // Static property
    private static $instances = 0;
    
    // Constructor
    public function __construct($name, $email) {
        $this->name = $name;
        $this->email = $email;
        self::$instances++;
    }
    
    // Instance method
    public function getEmail() {
        // Method local variable
        $formatted = strtolower($this->email);
        return $formatted;
    }
    
    // Static method
    public static function getInstanceCount() {
        return self::$instances;
    }
    
    // Method with closure
    public function process($items) {
        $multiplier = 10;
        
        // Closure inside method
        $processor = function($item) use ($multiplier) {
            return $item * $multiplier;
        };
        
        return array_map($processor, $items);
    }
    
    // Protected method
    protected function validate() {
        $rules = ['email' => 'required'];
        return true;
    }
    
    // Private method
    private function generateId() {
        $this->id = uniqid();
    }
}

// Module-level interface
interface UserInterface {
    public function authenticate($password);
    public function getProfile();
}

// Module-level trait
trait SoftDeletes {
    protected $deleted_at = null;
    
    public function delete() {
        $this->deleted_at = new DateTime();
    }
    
    public function restore() {
        $this->deleted_at = null;
    }
    
    public function isDeleted() {
        return $this->deleted_at !== null;
    }
}

// Abstract class
abstract class Model {
    abstract public function save();
    abstract public function load($id);
    
    public function toArray() {
        return get_object_vars($this);
    }
}

// Class extending abstract and using trait
class Product extends Model {
    use SoftDeletes;
    
    private $sku;
    private $price;
    
    public function __construct($sku, $price) {
        $this->sku = $sku;
        $this->price = $price;
    }
    
    public function save() {
        // Implementation
        $timestamp = time();
        return true;
    }
    
    public function load($id) {
        // Implementation
        $data = ['sku' => 'TEST', 'price' => 99.99];
        return $data;
    }
}

// Global function after classes
function globalHelper() {
    return 'global';
}

// Anonymous class
$anonClass = new class {
    public function anonMethod() {
        $anonLocal = 'anonymous';
        return $anonLocal;
    }
};
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PHP SCOPE VERIFICATION WITH DETAILED OUTPUT ===\n");
    println!("Total symbols found: {}", symbols.len());
    println!("\n--- Detailed Symbol Analysis ---\n");

    // Group symbols by kind for better analysis
    let mut by_kind: std::collections::HashMap<SymbolKind, Vec<_>> =
        std::collections::HashMap::new();

    for symbol in &symbols {
        by_kind.entry(symbol.kind).or_default().push(symbol);
    }

    // Print constants
    if let Some(constants) = by_kind.get(&SymbolKind::Constant) {
        println!("CONSTANTS ({} found):", constants.len());
        for c in constants {
            println!(
                "  {:20} | Scope: {:?} | Line: {}",
                c.name.as_ref(),
                c.scope_context,
                c.range.start_line
            );
        }
        println!();
    }

    // Print variables
    if let Some(variables) = by_kind.get(&SymbolKind::Variable) {
        println!("VARIABLES ({} found):", variables.len());
        for v in variables {
            println!(
                "  {:20} | Scope: {:?} | Line: {}",
                v.name.as_ref(),
                v.scope_context,
                v.range.start_line
            );
        }
        println!();
    }

    // Print functions
    if let Some(functions) = by_kind.get(&SymbolKind::Function) {
        println!("FUNCTIONS ({} found):", functions.len());
        for f in functions {
            println!(
                "  {:20} | Scope: {:?} | Line: {}",
                f.name.as_ref(),
                f.scope_context,
                f.range.start_line
            );
        }
        println!();
    }

    // Print classes
    if let Some(classes) = by_kind.get(&SymbolKind::Class) {
        println!("CLASSES ({} found):", classes.len());
        for c in classes {
            println!(
                "  {:20} | Scope: {:?} | Line: {}",
                c.name.as_ref(),
                c.scope_context,
                c.range.start_line
            );
        }
        println!();
    }

    // Print methods
    if let Some(methods) = by_kind.get(&SymbolKind::Method) {
        println!("METHODS ({} found):", methods.len());
        for m in methods {
            println!(
                "  {:20} | Scope: {:?} | Line: {}",
                m.name.as_ref(),
                m.scope_context,
                m.range.start_line
            );
        }
        println!();
    }

    // Print fields
    if let Some(fields) = by_kind.get(&SymbolKind::Field) {
        println!("FIELDS/PROPERTIES ({} found):", fields.len());
        for f in fields {
            println!(
                "  {:20} | Scope: {:?} | Line: {}",
                f.name.as_ref(),
                f.scope_context,
                f.range.start_line
            );
        }
        println!();
    }

    // Print interfaces
    if let Some(interfaces) = by_kind.get(&SymbolKind::Interface) {
        println!("INTERFACES ({} found):", interfaces.len());
        for i in interfaces {
            println!(
                "  {:20} | Scope: {:?} | Line: {}",
                i.name.as_ref(),
                i.scope_context,
                i.range.start_line
            );
        }
        println!();
    }

    // Print traits
    if let Some(traits) = by_kind.get(&SymbolKind::Trait) {
        println!("TRAITS ({} found):", traits.len());
        for t in traits {
            println!(
                "  {:20} | Scope: {:?} | Line: {}",
                t.name.as_ref(),
                t.scope_context,
                t.range.start_line
            );
        }
        println!();
    }

    println!("--- Scope Context Verification ---\n");

    // Verify module-level symbols
    let module_func = symbols.iter().find(|s| s.name.as_ref() == "moduleHelper");
    if let Some(mf) = module_func {
        println!(
            "moduleHelper scope: {:?} (expected: Module)",
            mf.scope_context
        );
        assert_eq!(mf.scope_context, Some(ScopeContext::Module));
    }

    let user_class = symbols.iter().find(|s| s.name.as_ref() == "User");
    if let Some(uc) = user_class {
        println!(
            "User class scope: {:?} (expected: Module)",
            uc.scope_context
        );
        assert_eq!(uc.scope_context, Some(ScopeContext::Module));
    }

    // Verify class members
    let get_email = symbols.iter().find(|s| s.name.as_ref() == "getEmail");
    if let Some(ge) = get_email {
        println!(
            "getEmail method scope: {:?} (expected: ClassMember)",
            ge.scope_context
        );
        assert_eq!(ge.scope_context, Some(ScopeContext::ClassMember));
    }

    let email_field = symbols.iter().find(|s| s.name.as_ref() == "email");
    if let Some(ef) = email_field {
        println!(
            "email field scope: {:?} (expected: ClassMember)",
            ef.scope_context
        );
        assert_eq!(ef.scope_context, Some(ScopeContext::ClassMember));
    }

    // Verify interface
    let interface = symbols.iter().find(|s| s.name.as_ref() == "UserInterface");
    if let Some(i) = interface {
        println!(
            "UserInterface scope: {:?} (expected: Module)",
            i.scope_context
        );
        assert_eq!(i.scope_context, Some(ScopeContext::Module));
    }

    // Verify trait
    let trait_def = symbols.iter().find(|s| s.name.as_ref() == "SoftDeletes");
    if let Some(td) = trait_def {
        println!(
            "SoftDeletes trait scope: {:?} (expected: Module)",
            td.scope_context
        );
        assert_eq!(td.scope_context, Some(ScopeContext::Module));
    }

    // Check for any symbols without scope context
    let unscoped = symbols
        .iter()
        .filter(|s| s.scope_context.is_none())
        .collect::<Vec<_>>();
    if !unscoped.is_empty() {
        println!(
            "\n⚠️ WARNING: {} symbols without scope context:",
            unscoped.len()
        );
        for s in unscoped {
            println!(
                "  - {} ({:?}) at line {}",
                s.name.as_ref(),
                s.kind,
                s.range.start_line
            );
        }
    } else {
        println!("\n✅ All symbols have scope context assigned!");
    }

    println!("\n=== SCOPE VERIFICATION COMPLETE ===\n");
}

#[test]
fn verify_php_namespace_scope_tracking() {
    let mut parser = PhpParser::new().unwrap();
    let code = r#"<?php
namespace App\Controllers;

use App\Models\User;
use App\Services\AuthService;

class UserController {
    private AuthService $authService;
    
    public function __construct(AuthService $service) {
        $this->authService = $service;
    }
    
    public function index() {
        $users = User::all();
        return view('users.index', compact('users'));
    }
}

namespace App\Models;

class User {
    protected $fillable = ['name', 'email'];
    
    public static function all() {
        return [];
    }
}

// Global namespace
namespace {
    function globalFunction() {
        return 'global';
    }
    
    class GlobalClass {
        public function method() {
            return 'method';
        }
    }
}
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PHP NAMESPACE SCOPE TRACKING ===\n");

    // Track symbols by namespace
    let mut by_namespace: std::collections::HashMap<Option<String>, Vec<_>> =
        std::collections::HashMap::new();

    for symbol in &symbols {
        // Extract namespace from module_path if available
        let namespace = symbol
            .module_path
            .as_ref()
            .and_then(|path| path.rsplit_once('\\').map(|(ns, _)| ns.to_string()));
        by_namespace.entry(namespace).or_default().push(symbol);
    }

    println!("Symbols grouped by namespace:\n");

    for (namespace, syms) in by_namespace {
        let ns_display = namespace.as_deref().unwrap_or("(no namespace/global)");
        println!("Namespace: {ns_display}");
        for s in syms {
            println!(
                "  - {} ({:?}) | Scope: {:?}",
                s.name.as_ref(),
                s.kind,
                s.scope_context
            );
        }
        println!();
    }

    // Verify UserController is in correct namespace scope
    let user_controller = symbols.iter().find(|s| s.name.as_ref() == "UserController");
    if let Some(uc) = user_controller {
        println!("UserController:");
        println!("  Scope context: {:?}", uc.scope_context);
        println!("  Module path: {:?}", uc.module_path);
        // In a namespace, classes should still be Module scope
        assert_eq!(uc.scope_context, Some(ScopeContext::Module));
    }

    // Verify global namespace symbols
    let global_func = symbols.iter().find(|s| s.name.as_ref() == "globalFunction");
    if let Some(gf) = global_func {
        println!("globalFunction:");
        println!("  Scope context: {:?}", gf.scope_context);
        println!("  Module path: {:?}", gf.module_path);
        assert_eq!(gf.scope_context, Some(ScopeContext::Module));
    }

    println!("\n=== NAMESPACE SCOPE TRACKING COMPLETE ===\n");
}

#[test]
fn verify_php_closure_and_anonymous_scope() {
    let mut parser = PhpParser::new().unwrap();
    let code = r#"<?php
// Top-level closure
$globalClosure = function($x) {
    $closureLocal = $x * 2;
    
    // Nested closure
    $nestedClosure = function($y) use ($closureLocal) {
        return $y + $closureLocal;
    };
    
    return $nestedClosure($x);
};

class Container {
    private $bindings = [];
    
    public function bind($key, $resolver) {
        // Closure as parameter
        $this->bindings[$key] = function() use ($resolver) {
            return $resolver();
        };
    }
    
    public function resolve($key) {
        $resolver = $this->bindings[$key];
        return $resolver();
    }
}

// Arrow function (PHP 7.4+)
$multiply = fn($x, $y) => $x * $y;

// Anonymous class with closure
$service = new class {
    private $handler;
    
    public function setHandler($handler) {
        $this->handler = $handler;
    }
    
    public function process($data) {
        $processor = function($item) {
            return strtoupper($item);
        };
        
        return array_map($processor, $data);
    }
};
"#;

    let file_id = FileId::new(1).unwrap();
    let mut counter = SymbolCounter::new();
    let symbols = parser.parse(code, file_id, &mut counter);

    println!("\n=== PHP CLOSURE AND ANONYMOUS SCOPE ===\n");

    // Look for all closures/anonymous functions
    let closures = symbols
        .iter()
        .filter(|s| {
            s.signature
                .as_ref()
                .is_some_and(|sig| sig.contains("function(") || sig.contains("fn("))
        })
        .collect::<Vec<_>>();

    println!("Closures/Anonymous functions found: {}\n", closures.len());
    for closure in closures {
        println!("Closure at line {}:", closure.range.start_line);
        println!("  Name: {}", closure.name.as_ref());
        println!("  Scope: {:?}", closure.scope_context);
        if let Some(sig) = closure.signature.as_ref() {
            println!("  Signature: {sig}");
        }
        println!();
    }

    // Look for variables that might be closure assignments
    let closure_vars = symbols
        .iter()
        .filter(|s| {
            s.kind == SymbolKind::Variable
                && (s.name.as_ref().contains("Closure") || s.name.as_ref().contains("multiply"))
        })
        .collect::<Vec<_>>();

    println!("Closure variables found: {}\n", closure_vars.len());
    for var in closure_vars {
        println!(
            "Variable: {} | Scope: {:?}",
            var.name.as_ref(),
            var.scope_context
        );
    }

    // Check Container class and its methods
    let container = symbols.iter().find(|s| s.name.as_ref() == "Container");
    if let Some(c) = container {
        println!("\nContainer class scope: {:?}", c.scope_context);

        let bind_method = symbols.iter().find(|s| s.name.as_ref() == "bind");
        if let Some(bm) = bind_method {
            println!("Container::bind scope: {:?}", bm.scope_context);
        }
    }

    println!("\n=== CLOSURE AND ANONYMOUS SCOPE COMPLETE ===\n");
}
