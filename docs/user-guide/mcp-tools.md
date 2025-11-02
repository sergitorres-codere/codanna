# MCP Tools Reference

Available tools when using the MCP server. All tools support `--json` flag for structured output.

## Tool Categories

### Discovery Tools
- **find_symbol** - Find symbol by exact name
- **search_symbols** - Full-text search with fuzzy matching
- **semantic_search_docs** - Natural language search
- **semantic_search_with_context** - Natural language search with relationships

### Relationship Tools
- **get_calls** - Functions called by a function
- **find_callers** - Functions that call a function
- **analyze_impact** - Impact radius of symbol changes

### Information Tools
- **get_index_info** - Index statistics

## Tool Details

### `find_symbol`

Find a symbol by exact name.

**Parameters:**
- `name` (required) - Exact symbol name to find

**Example:**
```bash
codanna mcp find_symbol main
codanna mcp find_symbol Parser --json
```

**Returns:** Symbol information including file path, line number, kind, and signature. For ExternalType symbols (types from external assemblies/libraries), includes a helpful note explaining they're not defined in source code.

### `search_symbols`

Search symbols with full-text fuzzy matching.

**Parameters:**
- `query` (required) - Search query (supports fuzzy matching)
- `limit` - Maximum number of results (default: 10)
- `kind` - Filter by symbol kind (e.g., "Function", "Struct", "Trait", "ExternalType")
- `module` - Filter by module path

**Example:**
```bash
codanna mcp search_symbols query:parse kind:function limit:10
codanna mcp search_symbols query:Parser --json
```

**Returns:** List of matching symbols with relevance ranking.

### `semantic_search_docs`

Search using natural language queries.

**Parameters:**
- `query` (required) - Natural language search query
- `limit` - Maximum number of results (default: 10)
- `threshold` - Minimum similarity score (0-1)
- `lang` - Filter by programming language (e.g., "rust", "typescript")

**Example:**
```bash
codanna mcp semantic_search_docs query:"error handling" limit:5
codanna mcp semantic_search_docs query:"authentication" lang:rust limit:5
```

**Returns:** Semantically similar symbols based on documentation.

### `semantic_search_with_context`

Natural language search with enhanced context including relationships.

**Parameters:**
- `query` (required) - Natural language search query
- `limit` - Maximum number of results (default: 5, as each includes full context)
- `threshold` - Minimum similarity score (0-1)
- `lang` - Filter by programming language

**Example:**
```bash
codanna mcp semantic_search_with_context query:"parse files" threshold:0.7
codanna mcp semantic_search_with_context query:"parse config" lang:typescript limit:3
```

**Returns:** Symbols with:
- Their documentation
- What calls them
- What they call
- Complete impact graph (includes ALL relationships: calls, type usage, composition)

### `get_calls`

Show functions called by a given function.

**Parameters:**
- `function_name` OR `symbol_id` (one required) - Function name or symbol ID

**Example:**
```bash
codanna mcp get_calls process_file
codanna mcp get_calls symbol_id:1883
codanna mcp get_calls main --json
```

**Returns:** List of functions that the specified function calls. Each result includes `[symbol_id:123]` for follow-up queries.

### `find_callers`

Show functions that call a given function.

**Parameters:**
- `function_name` OR `symbol_id` (one required) - Function name or symbol ID

**Example:**
```bash
codanna mcp find_callers init
codanna mcp find_callers symbol_id:1883
codanna mcp find_callers parse_file --json
```

**Returns:** List of functions that call the specified function. Each result includes `[symbol_id:123]` for follow-up queries.

### `analyze_impact`

Analyze the impact radius of symbol changes.

**Parameters:**
- `symbol_name` OR `symbol_id` (one required) - Symbol name or symbol ID
- `max_depth` - Maximum depth to search (default: 3)

**Example:**
```bash
codanna mcp analyze_impact Parser
codanna mcp analyze_impact symbol_id:1883
codanna mcp analyze_impact SimpleIndexer --json
```

**Returns:** Complete dependency graph showing:
- What CALLS this function
- What USES this as a type (fields, parameters, returns)
- What RENDERS/COMPOSES this (JSX: `<Component>`, Rust: struct fields, etc.)
- Full dependency graph across files
- Each result includes `[symbol_id:123]` for unambiguous follow-up

### `get_index_info`

Get index statistics and metadata.

**Parameters:** None

**Example:**
```bash
codanna mcp get_index_info
codanna mcp get_index_info --json
```

**Returns:**
- Total symbols indexed
- Symbols by language
- Symbols by kind
- Index creation/update timestamps
- File count

## Understanding Relationship Types

### Calls
Function invocation with parentheses
- `functionA()` invokes `functionB()`
- Shown by: `get_calls`, `find_callers`

### Uses
Type dependencies, composition, rendering
- Function parameters/returns: `fn process(data: MyType)`
- Component rendering: `<CustomButton>` in JSX
- Struct fields: `struct Container { inner: Type }`
- Shown by: `analyze_impact`

## Language Filtering

Mixed codebases (e.g., Python backend + TypeScript frontend): use `lang` parameter to reduce noise.

Supported languages: rust, python, typescript, go, php, c, cpp

Language filtering eliminates duplicate results when similar documentation exists across multiple languages, reducing result sets by up to 75% while maintaining identical similarity scores.

## JSON Output

All tools support `--json` flag for structured output, perfect for piping:

```bash
# Extract specific fields
codanna mcp find_symbol Parser --json | jq '.data[].symbol.name'

# Build call graphs
codanna mcp find_callers parse_file --json | \
jq -r '.data[]? | "\(.name) - \(.file_path):\(.range.start_line)"'

# Filter by score
codanna mcp semantic_search_docs query:"config" --json | \
jq '.data[] | select(.score > 0.5)'
```

## Using symbol_id for Unambiguous Queries

All tools return `[symbol_id:123]` in their results. Use these IDs for precise follow-up queries instead of symbol names.

**Benefits:**
- **Unambiguous** - Works even when multiple symbols share the same name
- **Efficient** - No disambiguation needed, direct lookup
- **Workflow-optimized** - Copy ID from results, paste into next command

**Example workflow:**
```bash
# Step 1: Search returns symbol_id
codanna mcp semantic_search_with_context query:"indexing" limit:1 --json
# Returns: SimpleIndexer [symbol_id:1883]

# Step 2: Use symbol_id for precise follow-up
codanna mcp get_calls symbol_id:1883

# Step 3: Follow relationships with IDs from results
codanna mcp analyze_impact symbol_id:1926
```

## Tool Workflow

### Recommended Approach

**Tier 1: High-Quality Context (Start Here)**
- `semantic_search_with_context` - Returns symbols WITH full context, impact analysis, and relationships
- `analyze_impact` - Shows complete dependency graph (Calls + Uses + Composes)

**Tier 2: Precise Lookups (When You Know Names)**
- `find_symbol` - Exact name lookup
- `search_symbols` - Fuzzy text search with filters

**Tier 3: Relationship Details (Verify Specific Patterns)**
- `get_calls` - Function invocation only (parentheses)
- `find_callers` - Reverse function invocation only

### When to Use What

- **Need complete picture?** → Start with `semantic_search_with_context` or `analyze_impact`
- **Need specific invocations?** → Use `get_calls` or `find_callers`
- **Unsure?** → Use Tier 1 tools, they show everything
- **Following relationships?** → Use `symbol_id:ID` from previous results

## System Messages

Each tool response includes a `system_message` that guides agents toward the next action. These are hidden from users but help AI assistants chain commands effectively.

```bash
# Extract system messages
codanna mcp find_callers walk_and_stream --json | jq -r '.system_message'
# Output: Found 18 callers. Run 'analyze_impact' to map the change radius.
```

## See Also

- [CLI Reference](cli-reference.md#codanna-mcp-tool-positional) - Command-line usage
- [Unix Piping](../advanced/unix-piping.md) - Advanced piping workflows
- [Agent Guidance](../integrations/agent-guidance.md) - Configuring system messages