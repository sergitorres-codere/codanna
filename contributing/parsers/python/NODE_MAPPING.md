# Python Tree-sitter Node Mapping

This document provides the official mapping of Python language constructs to their Tree-sitter node types, as discovered through ABI-15 exploration.

## Overview

- **Python Tree-sitter Version**: Latest
- **ABI Version**: 14
- **Total Node Types**: 275
- **Discovery Date**: Generated from `tests/abi15_exploration.rs`

## Critical Implementation Notes

⚠️ **NEVER guess node type names** - Always use the exact names documented below
⚠️ **Check field names** - Use `node.field_name_for_child()` to extract specific parts
⚠️ **Python patterns differ from other languages** - Do not assume similarities

## Function System

### Regular Functions
```python
def add(a: int, b: int) -> int:
    """Add two numbers."""
    return a + b
```
- **Node Type**: `function_definition` ✅ (ID: 146)
- **Parts**:
  - Name: `identifier` (ID: 1) via `name` field
  - Parameters: `parameters` ✅ (ID: 147) via `parameters` field
  - Body: Block of statements via `body` field
  - Return type: `type` ✅ (ID: 209) via `return_type` field

### Lambda Functions
```python
square = lambda x: x * x
```
- **Node Type**: `lambda` ✅ (ID: 197)

### Decorated Functions
```python
@property
@staticmethod
def method():
    pass
```
- **Wrapper**: `decorated_definition` ✅ (ID: 159)
- **Decorator**: `decorator` ✅ (ID: 160)

### Async Functions
```python
async def fetch_data():
    await client.get("/api")
```
- **Node Type**: ❌ `async_function_definition` NOT FOUND
- **Note**: Python Tree-sitter may use `function_definition` with `async` modifier

## Class System

### Class Definitions
```python
class User(BaseModel):
    """User model class."""
    name: str = "default"
    
    def __init__(self, name: str):
        self.name = name
```
- **Node Type**: `class_definition` ✅ (ID: 155)
- **Parts**:
  - Name: `identifier` (ID: 1) via `name` field
  - Base classes: `argument_list` ✅ (ID: 158) via `superclasses` field
  - Body: Block of statements via `body` field

### Method Definitions
```python
def method(self):
    pass
```
- **Node Type**: `function_definition` ✅ (ID: 146) within class body
- **Note**: Methods are functions within class definitions

## Variable System

### Simple Assignments
```python
count = 42
name = "example"
```
- **Node Type**: `assignment` ✅ (ID: 199)
- **Parts**:
  - Target: `identifier` (ID: 1) via `left` field
  - Value: Expression via `right` field

### Augmented Assignments
```python
count += 1
values *= 2
```
- **Node Type**: `augmented_assignment` ✅ (ID: 200)

### Type Annotations
```python
name: str = "default"
count: int
```
- **Node Type**: ❌ `annotated_assignment` NOT FOUND
- **Fallback**: Use `assignment` with type information

### Global/Nonlocal
```python
global counter
nonlocal value
```
- **Global**: `global_statement` ✅ (ID: 151)
- **Nonlocal**: `nonlocal_statement` ✅ (ID: 152)

## Import System

### Simple Imports
```python
import os
import sys
```
- **Node Type**: `import_statement` ✅ (ID: 111)

### From Imports
```python
from typing import Dict, List
from . import utils
```
- **Node Type**: `import_from_statement` ✅ (ID: 115)
- **Relative**: `relative_import` ✅ (ID: 113)

### Aliased Imports
```python
import numpy as np
from os.path import join as path_join
```
- **Node Type**: `aliased_import` ✅ (ID: 117)

### Wildcard Imports
```python
from math import *
```
- **Node Type**: `wildcard_import` ✅ (ID: 118)

### Dotted Names
```python
import os.path.join
```
- **Node Type**: `dotted_name` ✅ (ID: 163)

## Type System

### Basic Types
```python
x: int = 5
y: str = "hello"
```
- **Type Reference**: `type` ✅ (ID: 209)
- **Type Identifier**: `identifier` (ID: 1)

### Generic Types
```python
List[str]
Dict[str, int]
```
- **Generic**: `generic_type` ✅ (ID: 211)

### Union Types
```python
Union[str, int]
str | int  # Python 3.10+
```
- **Union**: `union_type` ✅ (ID: 212)

### Type Aliases
```python
UserId = int
Vector = List[float]
```
- **Node Type**: `type_alias_statement` ✅ (ID: 154)

### Type Parameters
```python
class Generic[T]:
    pass
```
- **Type Parameter**: `type_parameter` ✅ (ID: 156)

## Expression System

### Expression Statements
```python
print("hello")
x.method()
```
- **Node Type**: `expression_statement` ✅ (ID: 122)
- **Critical**: This wraps standalone expressions at statement level

### Attribute Access
```python
user.name
obj.method()
```
- **Node Type**: `attribute` ✅ (ID: 204)

### Subscript Access
```python
items[0]
data['key']
```
- **Node Type**: `subscript` ✅ (ID: 205)

## Literal System

### Numeric Literals
- **Integer**: `integer` ✅ (ID: 93)
- **Float**: `float` ✅ (ID: 94)

### Boolean and None
- **True**: `true` ✅ (ID: 96)
- **False**: `false` ✅ (ID: 97)
- **None**: `none` ✅ (ID: 98)

### String Literals
```python
"hello"
'world'
"""multiline"""
```
- **Node Type**: `string` ✅ (ID: 232)

### Collection Literals
- **List**: `list` ✅ (ID: 216)
- **Dictionary**: `dictionary` ✅ (ID: 219)
- **Set**: `set` ✅ (ID: 217)
- **Tuple**: `tuple` ✅ (ID: 218)

## Async System

### Missing Async Constructs
❌ **Not Found in Tree-sitter Python**:
- `async_function_definition` NOT FOUND
- `async_with_statement` NOT FOUND
- `async_for_statement` NOT FOUND
- `await_expression` NOT FOUND
- `yield_expression` NOT FOUND

### Available Async
- **Generator Expression**: `generator_expression` ✅ (ID: 224)

## Documentation System

### Comments
```python
# This is a comment
```
- **Node Type**: `comment` ✅ (ID: 99)

### Docstrings
```python
def func():
    """This is a docstring."""
    pass
```
- **String as First Statement**: `string` ✅ (ID: 232) within `expression_statement` ✅ (ID: 122)
- **Note**: ❌ `docstring` NOT FOUND as separate node type

## NOT FOUND Nodes

These Python constructs were tested but do not exist in Python Tree-sitter:

❌ **Missing Critical Nodes**:
- `async_function_definition` - Use `function_definition` with async detection
- `annotated_assignment` - Use `assignment` with type checking
- `async_with_statement` - May use regular `with_statement`
- `async_for_statement` - May use regular `for_statement`
- `await_expression` - Not found as separate node
- `yield_expression` - Not found as separate node
- `type_comment` - Not found
- `type_hint` - Not found
- `type_annotation` - Not found
- `class_body` - Body is implicit in `class_definition`
- `inheritance` - Use `superclasses` field
- `base_list` - Use `argument_list`
- `metaclass` - Not found
- `docstring` - Use `string` in `expression_statement`

## Parser Implementation Issues

### Current Symbol Extraction Problems

Based on TDD analysis, the Python parser has fundamental gaps:

**Only Processes These Node Types**:
- ✅ `function_definition`
- ✅ `class_definition`  
- ✅ `expression_statement` (for module-level assignments)

**Missing Processing For**:
- ❌ `decorated_definition` - Decorated functions/classes not extracted
- ❌ Standalone functions outside classes
- ❌ Many class methods inside class bodies
- ❌ Module-level constants and variables
- ❌ Type alias statements
- ❌ Import statements as symbols

### Symbol Extraction Recommendations

1. **Add `decorated_definition` processing** - Critical for `@property`, `@staticmethod`, etc.
2. **Fix `expression_statement` handling** - Currently only handles assignments, misses function calls
3. **Add comprehensive class body processing** - Methods inside classes not being found
4. **Add module-level symbol detection** - Constants, variables, type aliases
5. **Consider import statements as symbols** - For better cross-file resolution

## Field Name Reference

Key field names discovered:

- **name** - Names in various declarations
- **parameters** - Function parameters
- **body** - Function/class/statement bodies
- **left**/**right** - Assignment sides
- **superclasses** - Class inheritance list
- **return_type** - Function return type annotation
- **type** - Type annotations
- **target** - Assignment target
- **value** - Assignment value
- **attribute** - Attribute access field name

## Implementation Guidelines

1. **Always verify node names** against this document
2. **Use field names** to extract specific parts of nodes
3. **Handle missing async nodes** by detecting patterns in regular nodes
4. **Process `decorated_definition`** as wrapper around functions/classes
5. **Treat docstrings** as `string` nodes within `expression_statement`
6. **Fix fundamental symbol extraction** before enhancing doc comments
7. **Add comprehensive node type processing** to match available Tree-sitter nodes

## Critical Action Items

**Before implementing doc comment enhancements**:

1. ✅ **Fix symbol extraction** - Add missing node types to `extract_symbols_from_node`
2. ✅ **Add `decorated_definition` support** - Critical for Python patterns
3. ✅ **Improve class body processing** - Methods inside classes must be found
4. ✅ **Add module-level symbol detection** - Variables, constants, type aliases
5. ✅ **Test with real Python files** - Ensure basic symbol extraction works

**Only after symbol extraction is complete**:
- Enhance docstring extraction to match TypeScript quality
- Add comprehensive docstring pattern support
- Implement multi-line docstring collection

---

*Generated from Tree-sitter Python ABI-14 exploration*
*Always refer to the raw node discovery output for complete details*