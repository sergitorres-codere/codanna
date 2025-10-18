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
2. Narrow with specific symbol search
3. Trace relationships

```bash
# Find authentication concepts
codanna mcp semantic_search_docs query:"user authentication" limit:5

# Find specific auth function
codanna mcp find_symbol authenticate_user

# See what calls it
codanna mcp find_callers authenticate_user
```

### Understanding Code Flow
1. Find entry point
2. Trace calls
3. Analyze impact

```bash
# Find main processing function
codanna mcp semantic_search_with_context query:"main processing pipeline"

# Trace what it calls
codanna mcp get_calls process_file

# Understand impact
codanna mcp analyze_impact process_file
```

### Debugging Issues
1. Search for error-related code
2. Find callers
3. Trace to source

```bash
# Find error handling
codanna mcp semantic_search_docs query:"error recovery retry logic"

# Find who calls the error handler
codanna mcp find_callers handle_error

# Trace back to source
codanna mcp analyze_impact handle_error
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
- The symbol itself
- What calls it
- What it calls
- Full impact analysis

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
2. **Use language filters** - Reduces noise by up to 75% in mixed codebases
3. **Write good documentation** - Better docs = better search results
4. **Chain searches** - Use results from one search to inform the next
5. **Use JSON output** - Enables powerful piping and filtering

## Performance Tips

- First search after startup may be slower (cache warming)
- Subsequent searches are typically <10ms
- Use `--json` and `jq` for complex filtering instead of multiple searches

## See Also

- [MCP Tools Reference](mcp-tools.md) - Complete tool documentation
- [Unix Piping](../advanced/unix-piping.md) - Advanced search workflows
- [Configuration](configuration.md) - Semantic model configuration