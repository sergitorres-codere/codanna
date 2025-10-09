# PHP Parser Coverage Report

*Generated: 2025-10-08 21:30:53 UTC*

## Summary
- Nodes in file: 177
- Nodes handled by parser: 176
- Symbol kinds extracted: 8

## Coverage Table

| Node Type | ID | Status |
|-----------|-----|--------|
| namespace_definition | 206 | ✅ implemented |
| namespace_use_declaration | 207 | ✅ implemented |
| class_declaration | 222 | ✅ implemented |
| interface_declaration | 216 | ✅ implemented |
| trait_declaration | 215 | ✅ implemented |
| enum_declaration | 218 | ✅ implemented |
| method_declaration | 237 | ✅ implemented |
| function_definition | 244 | ✅ implemented |
| property_declaration | 231 | ✅ implemented |
| const_declaration | 229 | ✅ implemented |
| class_const_declaration | - | ❌ not found |
| simple_parameter | 252 | ✅ implemented |
| property_promotion_parameter | 251 | ✅ implemented |
| variadic_parameter | 253 | ✅ implemented |
| anonymous_function | 245 | ✅ implemented |
| arrow_function | 249 | ✅ implemented |
| attribute_list | 357 | ✅ implemented |
| attribute_group | 356 | ✅ implemented |
| attribute | 358 | ✅ implemented |

## Legend

- ✅ **implemented**: Node type is recognized and handled by the parser
- ⚠️ **gap**: Node type exists in the grammar but not handled by parser (needs implementation)
- ❌ **not found**: Node type not present in the example file (may need better examples)

## Recommended Actions

### Priority 2: Missing Examples
These nodes aren't in the comprehensive example. Consider:

- `class_const_declaration`: Add example to comprehensive.php or verify node name

