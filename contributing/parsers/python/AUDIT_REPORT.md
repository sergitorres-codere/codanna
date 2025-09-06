# Python Parser Coverage Report

## Summary
- Nodes in file: 121
- Nodes handled by parser: 23
- Symbol kinds extracted: 5

## Coverage Table

| Node Type | ID | Status |
|-----------|-----|--------|
| class_definition | 155 | ✅ implemented |
| function_definition | 146 | ✅ implemented |
| decorated_definition | 159 | ✅ implemented |
| assignment | 199 | ✅ implemented |
| augmented_assignment | - | ❌ not found |
| annotated_assignment | - | ❌ not found |
| typed_parameter | 208 | ✅ implemented |
| typed_default_parameter | 183 | ✅ implemented |
| parameters | 147 | ✅ implemented |
| import_statement | 111 | ✅ implemented |
| import_from_statement | 115 | ✅ implemented |
| aliased_import | - | ❌ not found |
| lambda | 73 | ✅ implemented |
| list_comprehension | 221 | ✅ implemented |
| dictionary_comprehension | 222 | ✅ implemented |
| set_comprehension | 223 | ✅ implemented |
| generator_expression | 224 | ✅ implemented |
| async_function_definition | - | ❌ not found |
| async_for_statement | - | ❌ not found |
| async_with_statement | - | ❌ not found |
| decorator | 160 | ✅ implemented |
| type_alias_statement | - | ❌ not found |
| type | 209 | ✅ implemented |
| global_statement | - | ❌ not found |
| nonlocal_statement | - | ❌ not found |
| with_statement | - | ❌ not found |
| for_statement | 137 | ⚠️ gap |
| while_statement | - | ❌ not found |

## Legend

- ✅ **implemented**: Node type is recognized and handled by the parser
- ⚠️ **gap**: Node type exists in the grammar but not handled by parser (needs implementation)
- ❌ **not found**: Node type not present in the example file (may need better examples)

## Recommended Actions

### Priority 1: Implementation Gaps
These nodes exist in your code but aren't being captured:

- `for_statement`: Add parsing logic in parser.rs

### Priority 2: Missing Examples
These nodes aren't in the comprehensive example. Consider:

- `augmented_assignment`: Add example to comprehensive.py or verify node name
- `annotated_assignment`: Add example to comprehensive.py or verify node name
- `aliased_import`: Add example to comprehensive.py or verify node name
- `async_function_definition`: Add example to comprehensive.py or verify node name
- `async_for_statement`: Add example to comprehensive.py or verify node name
- `async_with_statement`: Add example to comprehensive.py or verify node name
- `type_alias_statement`: Add example to comprehensive.py or verify node name
- `global_statement`: Add example to comprehensive.py or verify node name
- `nonlocal_statement`: Add example to comprehensive.py or verify node name
- `with_statement`: Add example to comprehensive.py or verify node name
- `while_statement`: Add example to comprehensive.py or verify node name

