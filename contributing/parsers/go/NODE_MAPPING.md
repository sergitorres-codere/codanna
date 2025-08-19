# Go Tree-sitter Node Mapping

This document provides the official mapping of Go language constructs to their Tree-sitter node types, as discovered through ABI-15 exploration.

## Overview

- **Go Tree-sitter Version**: 0.23.4
- **ABI Version**: 14  
- **Total Node Types**: 219
- **Discovery Date**: Generated from `tests/abi15_exploration.rs`

## Critical Implementation Notes

⚠️ **NEVER guess node type names** - Always use the exact names documented below
⚠️ **Check field names** - Use `node.field_name_for_child()` to extract specific parts
⚠️ **Go patterns differ significantly from TypeScript** - Do not assume similarities

## Package and Import System

### Package Declaration
```go
package main
```
- **Node Type**: `package_clause` (ID: 96)
- **Contains**: `package_identifier` (ID: 216)

### Import Declarations

#### Simple Import
```go
import "fmt"
```
- **Node Type**: `import_declaration` (ID: 97)
- **Contains**: `import_spec` (ID: 98) with `path` field

#### Import Groups
```go
import (
    "fmt"
    "os"
    m "math"           // alias
    . "strings"        // dot import  
    _ "database/sql"   // blank import
)
```
- **Node Type**: `import_declaration` (ID: 97)
- **Contains**: `import_spec_list` (ID: 100)
- **Import Types**:
  - Alias: `import_spec` with `name` field containing `package_identifier` (ID: 216)
  - Dot import: `import_spec` with `name` field containing `dot` (ID: 99)
  - Blank import: `import_spec` with `name` field containing `blank_identifier` (ID: 8)

### String Literals
- **Interpreted**: `interpreted_string_literal` (ID: 190)
- **Raw**: `raw_string_literal` (ID: 189)

## Type System

### Struct Types
```go
type User struct {
    Name string `json:"name"`
}
```
- **Declaration**: `type_declaration` (ID: 115)
- **Spec**: `type_spec` (ID: 116) with `name` and `type` fields
- **Struct**: `struct_type` (ID: 126)
- **Fields**: `field_declaration_list` (ID: 128) containing `field_declaration` (ID: 129)
- **Field Parts**: 
  - Name: `field_identifier` (ID: 214)
  - Type: `type_identifier` (ID: 218)  
  - Tag: `raw_string_literal` (ID: 189)

### Interface Types
```go
type Writer interface {
    Write([]byte) (int, error)
}
```
- **Declaration**: `type_declaration` (ID: 115)
- **Interface**: `interface_type` (ID: 130)
- **Method**: `method_elem` (ID: 131)
- **Method Parts**:
  - Name: `field_identifier` (ID: 214) 
  - Parameters: `parameter_list` (ID: 111)
  - Result: `parameter_list` (ID: 111)

### Type References
- **Basic Types**: `type_identifier` (ID: 218)
- **Pointer Types**: `pointer_type` (ID: 122)
- **Array Types**: `array_type` (ID: 123)  
- **Slice Types**: `slice_type` (ID: 125)
- **Map Types**: `map_type` (ID: 133)
- **Channel Types**: `channel_type` (ID: 134)
- **Generic Types**: `generic_type` (ID: 120)
- **Qualified Types**: `qualified_type` (ID: 188)

### Type Aliases
```go
type StringAlias = string
```
- **Node Type**: `type_alias` (ID: 114)

## Function System

### Regular Functions
```go
func Add(a, b int) int { return a + b }
```
- **Declaration**: `function_declaration` (ID: 107)
- **Parts**:
  - Name: `identifier` (ID: 1)
  - Parameters: `parameter_list` (ID: 111)
  - Result: `type_identifier` (ID: 218)
  - Body: `block` (ID: 136)

### Methods with Receivers
```go
func (u *User) String() string { return u.Name }
```
- **Declaration**: `method_declaration` (ID: 108)
- **Receiver**: `parameter_list` (ID: 111) - marked with `receiver` field
- **Receiver Parts**:
  - Name: `identifier` (ID: 1)
  - Type: `pointer_type` (ID: 122) or `type_identifier` (ID: 218)

### Generic Functions (Go 1.18+)
```go
func Add[T int | float64](a, b T) T { return a + b }
```
- **Type Parameters**: `type_parameter_list` (ID: 109)
- **Type Parameter**: `type_parameter_declaration` (ID: 110)
- **Constraint**: `type_constraint` (ID: 217)

### Function Literals/Closures
```go
func(x int) int { return x * 2 }
```
- **Node Type**: `func_literal` (ID: 185)

### Function Types
```go
type Handler func(int) error
```
- **Node Type**: `function_type` (ID: 135)

## Variable and Constant System

### Variable Declarations
```go
var count int = 42
var name string
```
- **Declaration**: `var_declaration` (ID: 104)
- **Spec**: `var_spec` (ID: 105)
- **Parts**:
  - Name: `identifier` (ID: 1)
  - Type: `type_identifier` (ID: 218)  
  - Value: `expression_list` (ID: 117)

### Constant Declarations
```go
const Pi = 3.14159
const MaxSize int = 100
```
- **Declaration**: `const_declaration` (ID: 102)
- **Spec**: `const_spec` (ID: 103)

### Short Variable Declarations
```go
users := make([]User, 0)
```
- **Node Type**: `short_var_declaration` (ID: 147)
- **Parts**:
  - Left: `expression_list` (ID: 117)
  - Right: `expression_list` (ID: 117)

## Expression System

### Function Calls
```go
fmt.Println("hello")
make([]int, 5)
```
- **Node Type**: `call_expression` (ID: 171)
- **Parts**:
  - Function: `selector_expression` (ID: 175) or `identifier` (ID: 1)
  - Arguments: `argument_list` (ID: 174)

### Selector Expressions (Method/Field Access)  
```go
user.Name
file.Close
```
- **Node Type**: `selector_expression` (ID: 175)
- **Parts**:
  - Operand: `identifier` (ID: 1)
  - Field: `field_identifier` (ID: 214)

### Binary and Unary Expressions
- **Binary**: `binary_expression` (ID: 187)
- **Unary**: `unary_expression` (ID: 186)

## Control Flow

### Statements
- **If**: `if_statement` (ID: 157)
- **For**: `for_statement` (ID: 158)
- **Switch**: ❌ `switch_statement` (NOT FOUND - use `expression_case`)
- **Type Switch**: `type_switch_statement` (ID: 164)
- **Select**: `select_statement` (ID: 167)
- **Return**: `return_statement` (ID: 154)
- **Break**: `break_statement` (ID: 151)
- **Continue**: `continue_statement` (ID: 152)
- **Go**: `go_statement` (ID: 155)
- **Defer**: `defer_statement` (ID: 156)
- **Goto**: `goto_statement` (ID: 153)
- **Labeled**: `labeled_statement` (ID: 148)
- **Fallthrough**: `fallthrough_statement` (ID: 150)

## Concurrency System

### Channel Operations
```go
ch <- value        // send
value := <-ch      // receive
```
- **Send**: `send_statement` (ID: 142)
- **Receive**: `unary_expression` (ID: 186) with `<-` operator
- **Channel Type**: `channel_type` (ID: 134)

### Select Statement Cases
- **Communication Case**: `communication_case` (ID: 168)
- **Default Case**: `default_case` (ID: 163)

## Literal Values

### Numeric Literals
- **Integer**: `int_literal` (ID: 86)
- **Float**: `float_literal` (ID: 87)
- **Imaginary**: `imaginary_literal` (ID: 88)
- **Rune**: `rune_literal` (ID: 89)

### Boolean and Nil
- **True**: `true` (ID: 91)
- **False**: `false` (ID: 92)
- **Nil**: `nil` (ID: 90)
- **Iota**: `iota` (ID: 93)

### Composite Literals
```go
User{Name: "John", Age: 30}
[]int{1, 2, 3}
```
- **Composite**: `composite_literal` (ID: 181)
- **Element**: `literal_element` (ID: 183)
- **Keyed Element**: `keyed_element` (ID: 184)

## Documentation

### Comments
- **General Comment**: `comment` (ID: 94)
- ❌ **Line Comment**: NOT FOUND (use `comment`)
- ❌ **Block Comment**: NOT FOUND (use `comment`)

## Assignment Operations

### Assignment Types
- **Regular Assignment**: `assignment_statement` (ID: 146)
- **Increment**: `inc_statement` (ID: 144)  
- **Decrement**: `dec_statement` (ID: 145)

## NOT FOUND Nodes

These nodes were tested but do not exist in Go Tree-sitter:

❌ `dot_import` - Use `import_spec` with `dot` field
❌ `blank_import` - Use `import_spec` with `blank_identifier` field  
❌ `import_alias` - Use `import_spec` with `name` field
❌ `tag` - Use `raw_string_literal` in field declaration
❌ `struct_literal` - Use `composite_literal`
❌ `struct_field` - Use `field_declaration`
❌ `embedded_field` - Use `field_declaration` without name
❌ `method_spec` - Use `method_elem`
❌ `method_spec_list` - Contained directly in `interface_type`
❌ `type_set` - Not in Go Tree-sitter
❌ `embedded_interface` - Use `type_elem`
❌ `union_type` - Not in Go Tree-sitter (use `type_constraint`)
❌ `receiver` - Use `parameter_list` with `receiver` field
❌ `result` - Use `parameter_list` or direct `type_identifier`
❌ `identifier_list` - Use `expression_list`
❌ `type_instantiation` - Use `generic_type`  
❌ `type_parameter` - Use `type_parameter_declaration`
❌ `method_expression` - Use `selector_expression`
❌ `switch_statement` - Not found, use specific switch types
❌ `receive_statement` - Use `unary_expression` with `<-`

## Field Name Reference

Key field names discovered:

- **path** - Import path in `import_spec`
- **name** - Names in various declarations  
- **type** - Type specifications
- **receiver** - Method receiver in `method_declaration`
- **parameters** - Function parameters
- **result** - Function return type
- **body** - Function/method body
- **tag** - Struct field tags
- **value** - Variable/constant values
- **left**/**right** - Assignment sides
- **operand**/**operator** - Expression parts
- **function**/**arguments** - Call expression parts
- **field** - Selector field access

## Usage Examples

See the complete node structure examples in `contributing/parsers/go/node_discovery.txt`.

## Implementation Guidelines

1. **Always verify node names** against this document
2. **Use field names** to extract specific parts of nodes  
3. **Handle Go-specific patterns** like receivers and channel operations
4. **Check for embedded fields** by looking for `field_declaration` without names
5. **Use `method_elem`** for interface methods, not `method_spec`
6. **Import handling** requires parsing `import_spec` children carefully
7. **Generic constraints** use `type_constraint`, not union types

---

*Generated from Tree-sitter Go v0.23.4 ABI-15 exploration*
*Always refer to the raw node discovery output for complete details*