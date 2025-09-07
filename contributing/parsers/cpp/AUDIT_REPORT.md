# C++ Parser Coverage Report

## Summary
- Nodes in file: 131
- Nodes handled by parser: 0
- Symbol kinds extracted: 2

## Coverage Table

| Node Type | ID | Status |
|-----------|-----|--------|
| translation_unit | 219 | ⚠️ gap |
| function_definition | 254 | ⚠️ gap |
| class_specifier | 379 | ⚠️ gap |
| struct_specifier | - | ❌ not found |
| union_specifier | - | ❌ not found |
| enum_specifier | - | ❌ not found |
| namespace_definition | 430 | ⚠️ gap |
| template_declaration | 386 | ⚠️ gap |
| template_instantiation | - | ❌ not found |
| function_declarator | 286 | ⚠️ gap |
| init_declarator | 294 | ⚠️ gap |
| parameter_declaration | 311 | ⚠️ gap |
| field_declaration | 307 | ⚠️ gap |
| access_specifier | 411 | ⚠️ gap |
| base_class_clause | 383 | ⚠️ gap |
| constructor_definition | - | ❌ not found |
| destructor_definition | - | ❌ not found |
| operator_overload | - | ❌ not found |
| lambda_expression | 464 | ⚠️ gap |
| using_declaration | - | ❌ not found |
| typedef_declaration | - | ❌ not found |

## Legend

- ✅ **implemented**: Node type is recognized and handled by the parser
- ⚠️ **gap**: Node type exists in the grammar but not handled by parser (needs implementation)
- ❌ **not found**: Node type not present in the example file (may need better examples)

## Recommended Actions

### Priority 1: Implementation Gaps
These nodes exist in your code but aren't being captured:

- `translation_unit`: Add parsing logic in parser.rs
- `function_definition`: Add parsing logic in parser.rs
- `class_specifier`: Add parsing logic in parser.rs
- `namespace_definition`: Add parsing logic in parser.rs
- `template_declaration`: Add parsing logic in parser.rs
- `function_declarator`: Add parsing logic in parser.rs
- `init_declarator`: Add parsing logic in parser.rs
- `parameter_declaration`: Add parsing logic in parser.rs
- `field_declaration`: Add parsing logic in parser.rs
- `access_specifier`: Add parsing logic in parser.rs
- `base_class_clause`: Add parsing logic in parser.rs
- `lambda_expression`: Add parsing logic in parser.rs

### Priority 2: Missing Examples
These nodes aren't in the comprehensive example. Consider:

- `struct_specifier`: Add example to comprehensive.cpp or verify node name
- `union_specifier`: Add example to comprehensive.cpp or verify node name
- `enum_specifier`: Add example to comprehensive.cpp or verify node name
- `template_instantiation`: Add example to comprehensive.cpp or verify node name
- `constructor_definition`: Add example to comprehensive.cpp or verify node name
- `destructor_definition`: Add example to comprehensive.cpp or verify node name
- `operator_overload`: Add example to comprehensive.cpp or verify node name
- `using_declaration`: Add example to comprehensive.cpp or verify node name
- `typedef_declaration`: Add example to comprehensive.cpp or verify node name

