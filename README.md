# Codanna

High-performance code intelligence that gives AI assistants deep understanding of your codebase through semantic search and relationship tracking.

## What It Does

Codanna indexes your code and provides:
- **Semantic search** - Find code using natural language: "authentication logic", "parse JSON data"
- **Relationship tracking** - Who calls what, implementation hierarchies, dependency graphs
- **MCP integration** - Claude can navigate and understand your codebase in real-time
- **Hot-reload** - Changes are automatically re-indexed
- **Fast searches** - Results in <10ms

Under the hood, Codanna:
1. Parses your code with tree-sitter (currently Rust and Python, more languages coming)
2. Extracts symbols and their relationships using type-aware analysis
3. Generates embeddings from documentation comments using AllMiniLML6V2 (384 dimensions)
4. Stores everything in a Tantivy full-text index with integrated vector search
5. Serves it via MCP so Claude can use it naturally

## Installation

```bash
# Install latest version
cargo install codanna

# Install with HTTP/HTTPS server support
cargo install codanna --features http-server

# Install from git
cargo install --git https://github.com/bartolli/codanna

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

4. **Try semantic search:**
```bash
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
# HTTP server
codanna serve --http --watch

# HTTPS server (requires http-server feature)
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

Codanna CLI is unix-friendly, enabling powerful command chaining and integration with other tools:

```bash
codanna mcp semantic_search_docs --args '{"query": "error handling", "limit": 3}' && \
echo "=== Analyzing IndexError usage ===" && \
codanna mcp find_symbol --args '{"name": "IndexError"}' && \
codanna mcp search_symbols --args '{"query": "Error", "limit": 5}'
```

This approach works well for agentic workflows and custom automation scripts.

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

| Command | Description | Example |
|---------|-------------|---------|
| `retrieve symbol <NAME>` | Find a symbol by name | `codanna retrieve symbol main` |
| `retrieve calls <FUNCTION>` | Show what functions a given function calls | `codanna retrieve calls parse_file` |
| `retrieve callers <FUNCTION>` | Show what functions call a given function | `codanna retrieve callers main` |
| `retrieve implementations <TRAIT>` | Show what types implement a trait | `codanna retrieve implementations Parser` |
| `retrieve impact <SYMBOL>` | Show the impact radius of changing a symbol | `codanna retrieve impact main --depth 3` |
| `retrieve search <QUERY>` | Search for symbols using full-text search | `codanna retrieve search "parse" --limit 5` |
| `retrieve describe <SYMBOL>` | Show comprehensive information about a symbol | `codanna retrieve describe SimpleIndexer` |

### Testing and Utilities

| Command | Description | Example |
|---------|-------------|---------|
| `codanna mcp-test` | Verify Claude can connect and list available tools | `codanna mcp-test` |
| `codanna mcp <TOOL>` | Execute MCP tools without spawning server | `codanna mcp find_symbol --args '{"name":"main"}'` |
| `codanna benchmark` | Benchmark parser performance | `codanna benchmark rust --file my_code.rs` |

### Common Flags

- `--config`, `-c`: Path to custom settings.toml file
- `--force`, `-f`: Force operation (overwrite, re-index, etc.)
- `--progress`, `-p`: Show progress during operations
- `--threads`, `-t`: Number of threads to use
- `--dry-run`: Show what would happen without executing

## MCP Tools

Available tools when using the MCP server:

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `find_symbol` | Find a symbol by exact name | `name` (required) |
| `search_symbols` | Search symbols with full-text fuzzy matching | `query`, `limit`, `kind`, `module` |
| `semantic_search_docs` | Search using natural language queries | `query`, `limit`, `threshold` |
| `semantic_search_with_context` | Search with enhanced context and details | `query`, `limit`, `threshold` |
| `get_calls` | Show functions called by a given function | `function_name` |
| `find_callers` | Show functions that call a given function | `function_name` |
| `analyze_impact` | Analyze the impact radius of symbol changes | `symbol_name`, `max_depth` |
| `get_index_info` | Get index statistics and metadata | None |

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

Run performance benchmarks:
```bash
codanna benchmark all          # Test all parsers
codanna benchmark python       # Test specific language
```

## Architecture Highlights

**Memory-mapped vector storage**: Semantic embeddings are stored in memory-mapped files for instant loading after the OS page cache warms up.

**Embedding lifecycle management**: Old embeddings are automatically cleaned up when files are re-indexed to prevent accumulation over time.

**Lock-free concurrency**: Uses DashMap for concurrent symbol access with minimal blocking for write coordination.

**Single-pass indexing**: Extracts symbols, relationships, and generates embeddings in one complete AST traversal.

**Hot reload capability**: Event-driven file watching with debouncing indexes only changed files for efficient updates.

## Requirements

- Rust 1.75+ (for development)
- ~150MB for model storage (downloaded on first use)
- A few MB for index storage (varies by codebase size)

## Current Limitations

- Supports Rust and Python (JavaScript, TypeScript coming soon)
- Semantic search requires English documentation/comments
- Windows support is experimental

## Contributing

This is an early release focused on core functionality. Contributions welcome! See CONTRIBUTING.md for guidelines.

## License

Licensed under the Apache License, Version 2.0 - See [LICENSE](LICENSE) file for details.

Attribution required when using Codanna in your project. See [NOTICE](NOTICE) file.

---

Built with 🦀 by developers who wanted their AI assistants to actually understand their code.