# Search Guide

How to write effective queries and get the best results from Codanna's search capabilities.

## Search Types

### Exact Match: `find_symbol`
For when you know the exact name:
```bash
codanna mcp find_symbol main
codanna mcp find_symbol SimpleIndexer
```

### Fuzzy Search: `search_symbols`
For partial matches and typos:
```bash
codanna mcp search_symbols query:parse
codanna mcp search_symbols query:indx  # Will find "index" functions
```

### Semantic Search: `semantic_search_docs`
For natural language queries:
```bash
codanna mcp semantic_search_docs query:"where do we handle errors"
codanna mcp semantic_search_docs query:"authentication logic"
```

### Context Search: `semantic_search_with_context`
For understanding relationships:
```bash
codanna mcp semantic_search_with_context query:"file processing pipeline"
```

## Writing Better Documentation Comments

Semantic search works by understanding your documentation comments:

### Good Documentation
```rust
/// Parse configuration from a TOML file and validate required fields
/// This handles missing files gracefully and provides helpful error messages
fn load_config(path: &Path) -> Result<Config, Error> {
    // implementation...
}
```

With good comments, semantic search can find this function when prompted for:
- "configuration validation"
- "handle missing config files"
- "TOML parsing with error handling"

### Poor Documentation
```rust
// Load config
fn load_config(path: &Path) -> Result<Config, Error> {
    // implementation...
}
```

This won't be found by semantic search effectively.

## Query Writing Tips

### Be Specific
- **Bad:** "error"
- **Good:** "error handling in file operations"

### Use Domain Terms
- **Bad:** "make things fast"
- **Good:** "performance optimization for indexing"

### Include Context
- **Bad:** "parse"
- **Good:** "parse TypeScript import statements"

## Language Filtering

In mixed-language codebases, use language filters:

```bash
# Search only Rust code
codanna mcp semantic_search_docs query:"memory management" lang:rust

# Search only TypeScript
codanna mcp semantic_search_docs query:"React components" lang:typescript
```

Supported languages: rust, python, typescript, go, php, c, cpp

## Understanding Scores

Similarity scores range from 0 to 1:
- **0.7+** - Very relevant
- **0.5-0.7** - Relevant
- **0.3-0.5** - Somewhat relevant
- **<0.3** - Probably not what you're looking for

Use threshold to filter:
```bash
codanna mcp semantic_search_docs query:"authentication" threshold:0.5
```

## Search Workflows

### Finding Implementation Details
1. Start broad with semantic search
2. Extract symbol_id from results
3. Trace relationships using IDs

```bash
# Find authentication concepts
codanna mcp semantic_search_docs query:"user authentication" limit:5
# Returns: authenticate_user [symbol_id:456]

# Use symbol_id for unambiguous lookup
codanna mcp find_callers symbol_id:456

# Or by name if unambiguous
codanna mcp find_symbol authenticate_user
```

### Understanding Code Flow
1. Find entry point
2. Trace calls using symbol_id
3. Analyze impact

```bash
# Find main processing function
codanna mcp semantic_search_with_context query:"main processing pipeline"
# Returns: process_file [symbol_id:789]

# Trace what it calls (using ID for precision)
codanna mcp get_calls symbol_id:789

# Understand impact
codanna mcp analyze_impact symbol_id:789
```

### Debugging Issues
1. Search for error-related code
2. Find callers using symbol_id
3. Trace to source

```bash
# Find error handling
codanna mcp semantic_search_docs query:"error recovery retry logic"
# Returns: handle_error [symbol_id:234]

# Find who calls the error handler (use ID from previous result)
codanna mcp find_callers symbol_id:234

# Trace back to source
codanna mcp analyze_impact symbol_id:234
```

## Advanced Techniques

### Combining Tools
```bash
# Find all parsers and their callers
codanna mcp search_symbols query:parse kind:function --json | \
jq -r '.data[].name' | \
xargs -I {} codanna mcp find_callers {} --json | \
jq -r '.data[].name' | sort -u
```

### Building Context
```bash
# Get complete context for a concept
codanna mcp semantic_search_with_context query:"dependency injection" limit:1 --json | \
jq '.data[0]'
```

This returns:
- The symbol itself with `[symbol_id:123]`
- What calls it (each with symbol_id)
- What it calls (each with symbol_id)
- Full impact analysis

Use the returned symbol_ids for precise follow-up queries.

## Common Issues

### No Results

**Problem:** Semantic search returns nothing
**Solution:**
- Check documentation exists
- Try broader terms
- Remove technical jargon

### Too Many Results

**Problem:** Search returns too much
**Solution:**
- Add language filter: `lang:rust`
- Increase threshold: `threshold:0.6`
- Reduce limit: `limit:3`
- Be more specific in query

### Wrong Language Results

**Problem:** Getting Python results when wanting TypeScript
**Solution:** Always use language filter in mixed codebases:
```bash
codanna mcp semantic_search_docs query:"components" lang:typescript
```

## Best Practices

1. **Start with semantic_search_with_context** - It provides the most complete picture
2. **Use symbol_id for follow-ups** - Eliminates ambiguity and saves queries
3. **Use language filters** - Reduces noise by up to 75% in mixed codebases
4. **Write good documentation** - Better docs = better search results
5. **Chain searches** - Use symbol_ids from one search in the next
6. **Use JSON output** - Enables powerful piping and filtering

**Example workflow with symbol_id:**
```bash
# Step 1: Find with semantic search
codanna mcp semantic_search_with_context query:"config parser" limit:1 --json
# Extract: parse_config [symbol_id:567]

# Step 2: Direct follow-up (no ambiguity)
codanna mcp get_calls symbol_id:567
codanna mcp find_callers symbol_id:567
codanna mcp analyze_impact symbol_id:567
```

## Performance Tips

- First search after startup may be slower (cache warming)
- Subsequent searches are typically <10ms
- Use `--json` and `jq` for complex filtering instead of multiple searches

## See Also

- [MCP Tools Reference](mcp-tools.md) - Complete tool documentation
- [Unix Piping](../advanced/unix-piping.md) - Advanced search workflows
- [Configuration](configuration.md) - Semantic model configuration