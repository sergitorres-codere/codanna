# Codanna

Semantic code search and relationship tracking via MCP and Unix CLI.

## How It Works

1. **Parse** - Tree-sitter AST parsing for Rust and Python (JavaScript/TypeScript coming)
2. **Extract** - Symbols, call graphs, implementations, and type relationships
3. **Embed** - 384-dimensional vectors from doc comments via AllMiniLML6V2
4. **Index** - Tantivy for full-text search + memory-mapped symbol cache for <10ms lookups
5. **Serve** - MCP protocol for AI assistants, ~300ms response time

## Installation

```bash
# Install latest version
cargo install codanna

# Install with HTTP server (OAuth authentication)
cargo install codanna --features http-server

# Install with HTTPS server (TLS + optional OAuth)
cargo install codanna --features https-server

# Install from local path (development)
cargo install --path . --all-features
```

## Quick Start

1. **Initialize and configure:**
```bash
# Initialize codanna index space and create .codanna/settings.toml
codanna init

# Enable semantic search in .codanna/settings.toml
```

2. **Enable semantic search in `.codanna/settings.toml`:**
```toml
[semantic_search]
enabled = true
```

3. **Index your codebase:**
```bash
# Index with progress display
codanna index src --progress

# See what would be indexed (dry run)
codanna index . --dry-run

# Index a specific file
codanna index src/main.rs
```

4. **Search your code:**
```bash
# Semantic search with new simplified syntax
codanna mcp semantic_search_docs query:"parse rust files" limit:3 --json

# Find symbols with JSON output
codanna retrieve symbol Parser --json

# Analyze function relationships
codanna mcp find_callers process_file --json | jq '.data[].name'

# Legacy format still works
codanna mcp semantic_search_with_context --args '{"query": "parse rust files and extract symbols", "limit": 3}'
```

## Claude Integration

### MCP Server (Recommended)

Add to your `.mcp.json`:

```json
{
  "mcpServers": {
    "codanna": {
      "command": "codanna",
      "args": ["serve", "--watch", "--watch-interval", "5"]
    }
  }
}
```

### HTTP/HTTPS Server

For persistent server with real-time file watching:

```bash
# HTTP server with OAuth authentication (requires http-server feature)
codanna serve --http --watch

# HTTPS server with TLS encryption (requires https-server feature)
codanna serve --https --watch
```

Configure in `.mcp.json`:
```json
{
  "mcpServers": {
    "codanna-sse": {
      "type": "sse",
      "url": "http://127.0.0.1:8080/mcp/sse"
    }
  }
}
```

For HTTPS configuration, see the [HTTPS Server Mode documentation](mcp-https-self-signed.md).

### Claude Sub Agent

We include a codanna-navigator sub agent at `.claude/agents/codanna-navigator.md`. This agent is optimized for using the codanna MCP server.

### Unix-Style Integration

Codanna CLI is unix-friendly with positional arguments and JSON output for easy command chaining:

```bash
# New simplified syntax - positional arguments for simple tools
codanna mcp find_symbol main --json
codanna mcp get_calls process_file
codanna mcp find_callers init

# Key:value pairs for complex tools  
codanna mcp semantic_search_docs query:"error handling" limit:3 --json
codanna mcp search_symbols query:parse kind:function --json

# Powerful Unix piping with JSON output
echo "error handling" | codanna mcp semantic_search_docs --json | jq '.data[].name'
codanna mcp find_symbol Parser --json | jq -r '.data[].callers[].name' | \
  xargs -I {} codanna mcp find_symbol {} --json

# Legacy format still supported for backward compatibility
codanna mcp find_symbol --args '{"name": "main"}'
```

All MCP tools support `--json` flag for structured output, making integration with other tools seamless.

## Configuration

Configure Codanna in `.codanna/settings.toml`:

```toml
[semantic_search]
enabled = true
model = "AllMiniLML6V2"
threshold = 0.6  # Similarity threshold (0-1)

[indexing]
parallel_threads = 16  # Auto-detected by default
include_tests = true   # Index test files
```

Codanna respects `.gitignore` and adds its own `.codannaignore`:

```bash
# Created automatically by codanna init
.codanna/       # Don't index own data
target/         # Skip build artifacts
node_modules/   # Skip dependencies
*_test.rs       # Optionally skip tests
```

## Documentation Comments for Better Search

Semantic search works by understanding your documentation comments:

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

This encourages better documentation → better AI understanding → more motivation to document.

## CLI Commands

### Core Commands

| Command | Description | Example |
|---------|-------------|---------|
| `codanna init` | Set up .codanna directory with default configuration | `codanna init --force` |
| `codanna index <PATH>` | Build searchable index from your codebase | `codanna index src --progress` |
| `codanna config` | Display active settings | `codanna config` |
| `codanna serve` | Start MCP server for AI assistants | `codanna serve --watch` |

### Retrieval Commands

All retrieve commands support `--json` flag for structured output (exit code 3 when not found).

| Command | Description | Example |
|---------|-------------|---------|
| `retrieve symbol <NAME>` | Find a symbol by name | `codanna retrieve symbol main --json` |
| `retrieve calls <FUNCTION>` | Show what functions a given function calls | `codanna retrieve calls parse_file --json` |
| `retrieve callers <FUNCTION>` | Show what functions call a given function | `codanna retrieve callers main --json` |
| `retrieve implementations <TRAIT>` | Show what types implement a trait | `codanna retrieve implementations Parser --json` |
| `retrieve impact <SYMBOL>` | Show the impact radius of changing a symbol | `codanna retrieve impact main --depth 3 --json` |
| `retrieve search <QUERY>` | Search for symbols using full-text search | `codanna retrieve search "parse" --limit 5 --json` |
| `retrieve describe <SYMBOL>` | Show comprehensive information about a symbol | `codanna retrieve describe SimpleIndexer --json` |

### Testing and Utilities

| Command | Description | Example |
|---------|-------------|---------|
| `codanna mcp-test` | Verify Claude can connect and list available tools | `codanna mcp-test` |
| `codanna mcp <TOOL>` | Execute MCP tools without spawning server | `codanna mcp find_symbol main --json` |
| `codanna benchmark` | Benchmark parser performance | `codanna benchmark rust --file my_code.rs` |

### Common Flags

- `--config`, `-c`: Path to custom settings.toml file
- `--force`, `-f`: Force operation (overwrite, re-index, etc.)
- `--progress`, `-p`: Show progress during operations
- `--threads`, `-t`: Number of threads to use
- `--dry-run`: Show what would happen without executing

## MCP Tools

Available tools when using the MCP server. All tools support `--json` flag for structured output.

### Simple Tools (Positional Arguments)
| Tool | Description | Example |
|------|-------------|---------|
| `find_symbol` | Find a symbol by exact name | `codanna mcp find_symbol main --json` |
| `get_calls` | Show functions called by a given function | `codanna mcp get_calls process_file` |
| `find_callers` | Show functions that call a given function | `codanna mcp find_callers init` |
| `analyze_impact` | Analyze the impact radius of symbol changes | `codanna mcp analyze_impact Parser --json` |
| `get_index_info` | Get index statistics and metadata | `codanna mcp get_index_info --json` |

### Complex Tools (Key:Value Arguments)
| Tool | Description | Example |
|------|-------------|---------|
| `search_symbols` | Search symbols with full-text fuzzy matching | `codanna mcp search_symbols query:parse kind:function limit:10` |
| `semantic_search_docs` | Search using natural language queries | `codanna mcp semantic_search_docs query:"error handling" limit:5` |
| `semantic_search_with_context` | Search with enhanced context | `codanna mcp semantic_search_with_context query:"parse files" threshold:0.7` |

### Parameters Reference
| Tool | Parameters |
|------|------------|
| `find_symbol` | `name` (required) |
| `search_symbols` | `query`, `limit`, `kind`, `module` |
| `semantic_search_docs` | `query`, `limit`, `threshold` |
| `semantic_search_with_context` | `query`, `limit`, `threshold` |
| `get_calls` | `function_name` |
| `find_callers` | `function_name` |
| `analyze_impact` | `symbol_name`, `max_depth` |
| `get_index_info` | None |

## Performance

Parser benchmarks on a 750-symbol test file:

| Language | Parsing Speed | vs. Target (10k/s) | Status |
|----------|---------------|-------------------|--------|
| **Rust** | 91,318 symbols/sec | 9.1x faster ✅ | Production |
| **Python** | 75,047 symbols/sec | 7.5x faster ✅ | Production |
| JavaScript | - | - | Coming soon |
| TypeScript | - | - | Coming soon |

Key achievements:
- **Zero-cost abstractions**: All parsers use borrowed string slices with no allocations in hot paths
- **Parallel processing**: Multi-threaded indexing that scales with CPU cores
- **Memory efficiency**: Approximately 100 bytes per symbol including all metadata
- **Real-time capability**: Fast enough for incremental parsing during editing
- **Optimized CLI startup**: ~300ms for all operations (53x improvement from v0.2)
- **JSON output**: Zero overhead - structured output adds <1ms to response time

Run performance benchmarks:
```bash
codanna benchmark all          # Test all parsers
codanna benchmark python       # Test specific language
```

## Architecture Highlights

**Memory-mapped storage**: Two caches for different access patterns:
- `symbol_cache.bin` - FNV-1a hashed symbol lookups, <10ms response time
- `segment_0.vec` - 384-dimensional vectors, <1μs access after OS page cache warm-up

**Embedding lifecycle management**: Old embeddings deleted when files are re-indexed to prevent accumulation.

**Lock-free concurrency**: DashMap for concurrent symbol reads, write coordination via single writer lock.

**Single-pass indexing**: Symbols, relationships, and embeddings extracted in one AST traversal.

**Hot reload**: File watcher with 500ms debounce triggers re-indexing of changed files only.

## Requirements

- Rust 1.75+ (for development)
- ~150MB for model storage (downloaded on first use)
- A few MB for index storage (varies by codebase size)

## Current Limitations

- Supports Rust and Python (JavaScript, TypeScript coming soon)
- Semantic search requires English documentation/comments
- Windows support is experimental

## Roadmap

### Versioning Strategy

- **0.2.x** - Patches and fixes only (bug fixes, dependency updates, performance improvements)
- **0.3.x** - Feature releases (JSON output, exit codes, new capabilities)
- **0.4.x** - Major features (JavaScript/TypeScript support, advanced analysis)

### Status Overview

| Priority | Feature | Status | Target |
|----------|---------|--------|--------|
| 1 | [JSON Output Support](#2-json-output-support) | ✅ Completed | v0.3.0 |
| 2 | [Exit Codes for Common Conditions](#5-exit-codes-for-common-conditions) | ✅ Completed | v0.3.0 |
| 3 | [Batch Symbol Operations](#3-batch-symbol-operations) | Planning | v0.3.1 |
| 4 | [Output Format Control](#4-output-format-control) | Planning | v0.3.1 |
| 5 | [Direct CLI Semantic Search](#1-direct-cli-semantic-search) | Partial | v0.3.1 |
| 6 | [Incremental Index Updates](#7-incremental-index-updates) | ✅ Completed | v0.2.0 |
| 7 | [Query Language for Complex Searches](#6-query-language-for-complex-searches) | Partial | -- |
| 8 | [Configuration Profiles](#8-configuration-profiles) | Pending | -- |
| 9 | [Machine-Readable Progress](#9-machine-readable-progress) | Pending | -- |

---

### 1. Direct CLI Semantic Search

**Partially Implemented**: Simplified syntax available through MCP interface.

```bash
# NEW: Simplified syntax (no JSON escaping needed!)
codanna mcp semantic_search_docs query:authentication limit:10 --json

# Still TODO: Direct retrieve command
codanna retrieve semantic "authentication" --limit 10
```

**Delivered**:
- ✅ Simpler command syntax (key:value pairs)
- ✅ Better Unix integration (positional args)
- ✅ No JSON escaping needed

**Remaining**: Direct `retrieve semantic` command for consistency

### 2. JSON Output Support

**Implemented in v0.3.0**: All retrieve commands and MCP tools now support `--json` flag.

```bash
# All retrieve commands support --json
codanna retrieve symbol MyFunction --json
codanna retrieve calls process_file --json
codanna retrieve callers init --json

# All MCP tools support --json
codanna mcp find_symbol main --json
codanna mcp semantic_search_docs query:"error handling" --json
```

**Delivered Benefits**:
- ✅ Stable API for scripts and tools
- ✅ Zero performance overhead (<1ms)
- ✅ Consistent JsonResponse format across all commands
- ✅ Proper exit codes (3 for not found)

### 3. Batch Symbol Operations

**Why**: Reduce overhead when analyzing multiple symbols

```bash
# Current: Multiple invocations
for sym in func1 func2 func3; do
  codanna retrieve symbol "$sym"
done

# Wishlist: Single command
codanna retrieve symbols func1 func2 func3
```

**Benefits**:
- One index load instead of N
- Faster CI/CD pipelines
- Better for parallel analysis

### 4. Output Format Control

**Why**: Different use cases need different detail levels

```bash
# Compact output for scripts
codanna retrieve callers MyFunc --format=compact
validate_input:src/validation.rs:45
process_request:src/handler.rs:120

# Full output for humans (current default)
codanna retrieve callers MyFunc --format=full
```

### 5. Exit Codes for Common Conditions

**Implemented in v0.3.0**: All commands now return appropriate exit codes.

```bash
# Exit codes implemented:
# 0 - Success
# 1 - General error
# 3 - Not found (symbol, function, etc.)

if codanna retrieve symbol MyFunc --json >/dev/null 2>&1; then
  echo "Symbol exists"
else
  if [ $? -eq 3 ]; then
    echo "Symbol not found"
  else
    echo "Error occurred"
  fi
fi
```

**Actual JSON output**:
```bash
$ codanna retrieve symbol NonExistent --json
{
  "status": "error",
  "code": "NOT_FOUND",
  "message": "Symbol 'NonExistent' not found",
  "error": {
    "suggestions": [
      "Check the spelling",
      "Ensure the index is up to date"
    ]
  },
  "exit_code": 3
}
# Exit code: 3
```

### 6. Query Language for Complex Searches

**Partially Implemented**: Key:value syntax available for MCP tools.

```bash
# NOW AVAILABLE: Key:value syntax for MCP tools
codanna mcp search_symbols query:Parser kind:method limit:20 --json
codanna mcp semantic_search_docs query:"error handling" limit:5 --json

# Still TODO: Advanced query combinations
codanna query "kind:method visibility:public calls:*Parser*"
codanna query "kind:function visibility:private callers:0"
```

**Delivered**: Basic key:value parameter parsing for MCP tools
**Remaining**: Full query language with wildcards and combinations

### 7. Incremental Index Updates

**Implemented**: Watch mode with notification channels for coordinated updates.

```bash
# Watch mode auto-indexes changed files
codanna serve --watch --watch-interval 5

# Server output shows notification flow:
# Detected change in indexed file: src/main.rs
#   Re-indexing...
#   ✓ Re-indexed successfully (file updated)
# File watcher received IndexReloaded notification
#   Refreshing watched file list...
#   ✓ Now watching 60 files
```

**Delivered**:
- ✅ Automatic file watching with `--watch` flag
- ✅ Broadcast channels coordinate index and file watchers
- ✅ File deletions trigger index and cache cleanup
- ✅ Only changed files are re-indexed
- ✅ Event-driven with debouncing for efficiency

### 8. Configuration Profiles

**Why**: Different settings for different use cases

```bash
# .codanna/profiles.toml
[profiles.ci]
semantic_search = false
max_file_size = "1MB"

[profiles.dev]
semantic_search = true
watch_mode = true

# Use profile
codanna --profile=ci index .
```

### 9. Machine-Readable Progress

**Why**: Better CI/CD integration

```bash
# Current: Human-readable progress
# Wishlist: Machine-readable option
codanna index . --progress=json
{"phase":"parsing","files_done":45,"files_total":200,"percent":22.5}
{"phase":"parsing","files_done":46,"files_total":200,"percent":23.0}
```

### Implementation Priority

1. **JSON output** - Enables everything else
2. **Exit codes** - Minimal change, big impact
3. **Batch operations** - Performance win
4. **Format control** - Flexibility for users
5. **Rest** - Nice to have

## Contributing

This is an early release focused on core functionality. Contributions welcome! See CONTRIBUTING.md for guidelines.

## License

Licensed under the Apache License, Version 2.0 - See [LICENSE](LICENSE) file for details.

Attribution required when using Codanna in your project. See [NOTICE](NOTICE) file.

---

Built with 🦀 by developers who wanted their AI assistants to actually understand their code.