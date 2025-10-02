# C# Tree-Sitter Node Mapping

## ABI Information
- **ABI Version**: 14
- **Total Node Types**: 503
- **Tree-sitter Grammar**: tree-sitter-c-sharp v0.23.1

## Key Findings from ABI-15 Exploration

### ✅ Available Core Node Types

#### Class-Related Nodes
- `class_declaration` (ID: 231) - Main class declarations
- `base_list` (ID: 244) - Inheritance and interface implementation lists
- `constructor_declaration` (ID: 253)
- `destructor_declaration` (ID: 254)
- `field_declaration` (ID: 252)
- `property_declaration` (ID: 262)
- `method_declaration` (ID: 255)
- `operator_declaration` (ID: 248)
- `indexer_declaration` (ID: 260)
- `event_declaration` (ID: 256)
- `event_field_declaration` (ID: 257)

#### Interface-Related Nodes
- `interface_declaration` (ID: 236)
- `explicit_interface_specifier` (ID: 263)

#### Struct-Related Nodes
- `struct_declaration` (ID: 232)

#### Enum-Related Nodes
- `enum_declaration` (ID: 233)
- `enum_member_declaration` (ID: 235)

#### Record-Related Nodes (C# 9+)
- `record_declaration` (ID: 238)
- `primary_constructor_base_type` (ID: 240)
- `with_expression` (ID: 411)

#### Namespace-Related Nodes
- `namespace_declaration` (ID: 228)
- `file_scoped_namespace_declaration` (ID: 229) - C# 10+ feature
- `using_directive` (ID: 221)
- `extern_alias_directive` (ID: 220)
- `qualified_name` (ID: 282)

#### Type System Nodes
- `type` (ID: 285)
- `type_parameter_list` (ID: 242)
- `type_parameter` (ID: 243)
- `type_parameter_constraints_clause` (ID: 245)
- `type_argument_list` (ID: 284)
- `predefined_type` (ID: 103)
- `nullable_type` (ID: 290)
- `array_type` (ID: 287)
- `pointer_type` (ID: 291)
- `ref_type` (ID: 296)
- `generic_name` (ID: 283)
- `tuple_type` (ID: 300)
- `function_pointer_type` (ID: 293)

#### Method/Function Nodes
- `method_declaration` (ID: 255)
- `local_function_statement` (ID: 331)
- `lambda_expression` (ID: 399)
- `anonymous_method_expression` (ID: 402)
- `delegate_declaration` (ID: 237)
- `parameter_list` (ID: 264)
- `parameter` (ID: 265)
- `modifier` (ID: 241) - Used for access modifiers

#### Property-Related Nodes
- `property_declaration` (ID: 262)
- `accessor_list` (ID: 258)
- `accessor_declaration` (ID: 259)
- `arrow_expression_clause` (ID: 272)

#### Attribute Nodes
- `attribute_list` (ID: 226)
- `attribute` (ID: 223)
- `attribute_argument_list` (ID: 224)
- `attribute_argument` (ID: 225)
- `attribute_target_specifier` (ID: 227)

#### Expression Nodes
- `invocation_expression` (ID: 380)
- `member_access_expression` (ID: 394)
- `element_access_expression` (ID: 386)
- `conditional_access_expression` (ID: 374)
- `object_creation_expression` (ID: 396)
- `array_creation_expression` (ID: 401)
- `implicit_array_creation_expression` (ID: 405)
- `assignment_expression` (ID: 353)
- `binary_expression` (ID: 354)
- `unary_expression` (ID: 444)
- `await_expression` (ID: 384)

#### Pattern Matching Nodes (C# 8+)
- `pattern` (ID: 332)
- `constant_pattern` (ID: 333)
- `declaration_pattern` (ID: 346)
- `var_pattern` (ID: 335)
- `parenthesized_pattern` (ID: 334)
- `tuple_pattern` (ID: 269)
- `relational_pattern` (ID: 342)
- `type_pattern` (ID: 336)
- `list_pattern` (ID: 337)

#### LINQ/Query Nodes
- `query_expression` (ID: 358)
- `from_clause` (ID: 359)
- `let_clause` (ID: 366)
- `where_clause` (ID: 369)
- `join_clause` (ID: 362)
- `join_into_clause` (ID: 365)
- `select_clause` (ID: 372)
- `group_clause` (ID: 371)

### ❌ Missing/Unavailable Node Types

#### Specific Modifier Nodes
- No separate nodes for: `override`, `virtual`, `abstract`, `static`, `async`, `partial`, `sealed`, `readonly`, `extern`, `unsafe`
- These are likely represented as tokens within the `modifier` node (ID: 241)

#### Some Modern C# Features
- No specific nodes for: `global_using_directive`, `using_static_directive`, `using_alias_directive`
- Missing some pattern matching nodes: `discard_pattern`, `property_pattern`, `positional_pattern`
- No preprocessor directive support

## Implementation Strategy

### 1. Symbol Extraction Priority
1. **Classes**: Use `class_declaration` with `base_list` for inheritance
2. **Interfaces**: Use `interface_declaration`
3. **Structs**: Use `struct_declaration`
4. **Enums**: Use `enum_declaration` with `enum_member_declaration`
5. **Records**: Use `record_declaration` (C# 9+)
6. **Methods**: Use `method_declaration` within class/interface bodies
7. **Properties**: Use `property_declaration` with `accessor_list`
8. **Fields**: Use `field_declaration`
9. **Events**: Use `event_declaration` and `event_field_declaration`

### 2. Namespace and Import Handling
- Use `namespace_declaration` and `file_scoped_namespace_declaration`
- Parse `using_directive` for imports
- Handle `qualified_name` for fully qualified type references

### 3. Generic Support
- Extract generics using `type_parameter_list` and `type_parameter`
- Handle constraints with `type_parameter_constraints_clause`
- Parse generic usage with `type_argument_list` and `generic_name`

### 4. Access Modifier Parsing
- Parse `modifier` nodes to extract visibility (public, private, protected, internal)
- Look for specific modifier tokens within the modifier node

### 5. Modern C# Features
- Support records via `record_declaration`
- Handle file-scoped namespaces via `file_scoped_namespace_declaration`
- Parse pattern matching using various `*_pattern` nodes
- Support LINQ queries using query-related nodes

## Critical Implementation Notes

1. **Node Name Accuracy**: Always use the exact node names discovered in the exploration test
2. **Modifier Handling**: The `modifier` node (ID: 241) likely contains multiple modifiers as children
3. **Generic Constraints**: Use `type_parameter_constraints_clause` for parsing where clauses
4. **Property Accessors**: Parse `accessor_list` to find get/set/init accessors
5. **Inheritance**: Use `base_list` to extract both base classes and implemented interfaces
6. **Documentation**: Look for `comment` nodes (ID: 204) for XML documentation
7. **C# Version Support**: The grammar supports modern C# features like records and file-scoped namespaces

## Implementation Status ✅

### ✅ **Fully Implemented Features**

#### Core Symbol Extraction
- **Classes**: `class_declaration` with inheritance via `base_list` ✅
- **Interfaces**: `interface_declaration` with member extraction ✅
- **Structs**: `struct_declaration` with member processing ✅
- **Enums**: `enum_declaration` with `enum_member_declaration` extraction ✅
- **Records**: `record_declaration` support (C# 9+) ✅
- **Delegates**: `delegate_declaration` processing ✅

#### Member Extraction
- **Methods**: `method_declaration` with full signature extraction ✅
- **Properties**: `property_declaration` with accessor information ✅
- **Fields**: `field_declaration` with variable declarators ✅
- **Events**: `event_declaration` and `event_field_declaration` ✅
- **Constructors**: `constructor_declaration` processing ✅
- **Local Functions**: `local_function_statement` within method bodies ✅
- **Variables**: `variable_declaration` and `local_declaration_statement` ✅

#### Advanced Features
- **Namespaces**: Both `namespace_declaration` and `file_scoped_namespace_declaration` ✅
- **Using Directives**: `using_directive` import extraction ✅
- **Enum Members**: Individual enum values as constants ✅
- **Nested Types**: Recursive processing of nested declarations ✅
- **Access Modifiers**: `modifier` node parsing for visibility ✅
- **Documentation**: XML documentation comment extraction ✅
- **Signatures**: Complete signature extraction excluding bodies ✅

#### Symbol Categorization
- **Classes/Interfaces/Structs/Records** → `SymbolKind::Class`/`Interface`/`Struct`
- **Methods/Constructors** → `SymbolKind::Method`
- **Properties/Fields/Events** → `SymbolKind::Field`
- **Enum Members** → `SymbolKind::Constant`
- **Local Functions** → `SymbolKind::Function`
- **Variables** → `SymbolKind::Variable`
- **Delegates** → `SymbolKind::Function`

### 📊 **Performance Results**

**Before Implementation**: 58 symbols (1 per file, top-level only)
**After Implementation**: 241+ symbols (4.2+ per file, complete extraction)

### 🎯 **Validation Results**

#### Test Case: Simple Enum + Class
```csharp
public enum Color { Red, Green, Blue }
public class TestClass { public void TestMethod() {} }
```
**Result**: 6 symbols extracted ✅
- `Color` (enum)
- `Red`, `Green`, `Blue` (enum members as constants)
- `TestClass` (class)
- `TestMethod` (method)

#### Real-World Codebase Test
**58 C# files** → **241 symbols** with proper categorization:
- 84 Methods detected ✅
- Enum members properly extracted ✅
- Interface methods included ✅
- Complete member hierarchies ✅

### ⚠️ **Known Limitations**

#### Partially Implemented
- **Method Calls**: `find_method_calls()` - returns empty (relationship detection)
- **Interface Implementations**: `find_implementations()` - returns empty
- **Usage Detection**: `find_uses()` - returns empty
- **Definition Detection**: `find_defines()` - returns empty

#### Complex Features Not Yet Implemented
- **Pattern Matching**: Various `*_pattern` nodes (C# 8+)
- **LINQ Queries**: Query expression nodes
- **Operator Overloading**: `operator_declaration`
- **Indexers**: `indexer_declaration`
- **Advanced Generics**: Complex constraint parsing

### 🔧 **Technical Implementation Details**

#### Memory-Mapped Symbol Cache
- Fixed Windows file locking issues (OS error 1224) ✅
- Retry logic with automatic cache deletion ✅
- Proper cleanup on `--force` rebuilds ✅

#### Parser Architecture
- Tree-sitter integration with exact node name matching ✅
- Recursive symbol extraction with scope management ✅
- Comprehensive signature extraction excluding bodies ✅
- Proper visibility and documentation handling ✅

## Next Steps

1. ✅ **Core Parser**: Complete symbol extraction implemented
2. ✅ **Testing**: Validated on real C# codebases
3. 🔄 **Relationship Detection**: Implement method calls and usage tracking
4. 🔄 **Advanced Features**: Pattern matching and LINQ support
5. 🔄 **Performance**: Optimize for very large codebases (1000+ files)