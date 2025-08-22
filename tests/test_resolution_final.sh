#!/bin/bash

echo "=== Resolution System Test ==="
echo ""

# Clean build
cargo build --release 2>/dev/null || exit 1

# Create a simple Rust test file
cat > test_resolution.rs << 'EOF'
struct MyStruct {
    value: i32,
}

impl MyStruct {
    fn new(value: i32) -> Self {
        MyStruct { value }
    }
    
    fn process(&self) -> i32 {
        self.value * 2
    }
}

fn main() {
    let s = MyStruct::new(42);
    let result = s.process();
    println!("Result: {}", result);
}
EOF

# Index it
echo "Indexing test file..."
./target/release/codanna index test_resolution.rs --force >/dev/null 2>&1

echo ""
echo "1. Checking Symbol Discovery"
echo "============================="
echo "Looking for MyStruct:"
./target/release/codanna mcp find_symbol name:MyStruct --json | jq '.data[0].symbol | {name, kind, scope_context}'

echo ""
echo "Looking for new method:"
./target/release/codanna mcp find_symbol name:new --json | jq '.data[] | select(.symbol.module_path | contains("test_resolution")) | .symbol | {name, kind, scope_context}'

echo ""
echo "Looking for process method:"
./target/release/codanna mcp find_symbol name:process --json | jq '.data[] | select(.symbol.module_path | contains("test_resolution")) | .symbol | {name, kind, scope_context}'

echo ""
echo "2. Checking Method Calls"
echo "========================"
echo "What does main call?"
# First find main's symbol_id
MAIN_ID=$(./target/release/codanna mcp search_symbols query:main --json | jq '.data[] | select(.file_path == "test_resolution.rs") | .symbol_id')
echo "Main function ID: $MAIN_ID"

# Now check its calls
echo "Checking calls from main:"
./target/release/codanna mcp get_calls function_name:main --json | jq '.data[] | select(.[0].id == '$MAIN_ID') | .[1]' 2>/dev/null || echo "No calls found for this main"

echo ""
echo "3. Testing Examples Directory"
echo "============================="
echo "Re-indexing examples..."
./target/release/codanna index examples --force >/dev/null 2>&1

echo ""
echo "Rust - HelperStruct from import_resolution_test.rs:"
./target/release/codanna mcp find_symbol name:HelperStruct --json | jq '.data[] | select(.file_path | contains("import_resolution_test")) | .symbol | {name, kind, scope_context}'

echo ""
echo "Python - AuthService from services/auth.py:"
./target/release/codanna mcp find_symbol name:AuthService --json | jq '.data[] | select(.symbol.language_id == "python") | .symbol | {name, kind, scope_context}'

echo ""
echo "PHP - UserController:"
./target/release/codanna mcp find_symbol name:UserController --json | jq '.data[] | select(.symbol.language_id == "php") | .symbol | {name, kind, module_path}' | head -20

echo ""
echo "4. Check Resolution Context Usage"
echo "================================="
echo "Looking for methods with ClassMember scope:"
./target/release/codanna mcp search_symbols query:new limit:5 --json | jq '.data[] | select(.kind == "Method") | {name, kind, file_path}'

echo ""
echo "5. Language-Filtered Search"
echo "==========================="
echo "Rust methods named 'new':"
./target/release/codanna mcp search_symbols query:new lang:rust limit:3 --json | jq '.data[] | {name, kind, file_path}'

echo ""
echo "Python methods named 'authenticate':"
./target/release/codanna mcp search_symbols query:authenticate lang:python limit:3 --json | jq '.data[] | {name, kind, file_path}'

# Cleanup
rm -f test_resolution.rs

echo ""
echo "=== Test Complete ==="