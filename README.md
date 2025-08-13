# Codanna

Semantic code search and relationship tracking via MCP and Unix CLI.

## Table of Contents

- [How It Works](#how-it-works)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Claude Integration](#claude-integration)
  - [MCP Server (Recommended)](#mcp-server-recommended)
  - [HTTP/HTTPS Server](#httphttps-server)
  - [Claude Sub Agent](#claude-sub-agent)
  - [Unix-Style Integration](#unix-style-integration)
- [Configuration](#configuration)
- [Documentation Comments for Better Search](#documentation-comments-for-better-search)
- [CLI Commands](#cli-commands)
  - [Core Commands](#core-commands)
  - [Retrieval Commands](#retrieval-commands)
  - [Testing and Utilities](#testing-and-utilities)
  - [Common Flags](#common-flags)
- [MCP Tools](#mcp-tools)
  - [Simple Tools (Positional Arguments)](#simple-tools-positional-arguments)
  - [Complex Tools (Key:Value Arguments)](#complex-tools-keyvalue-arguments)
  - [Parameters Reference](#parameters-reference)
- [Performance](#performance)
- [Architecture Highlights](#architecture-highlights)
- [Requirements](#requirements)
- [Current Limitations](#current-limitations)
- [Roadmap](#roadmap)
  - [Version Strategy](#version-strategy)
  - [v0.3.0 (Released)](#v030-released)
  - [v0.4.0 (Next Release)](#v040-next-release)
  - [v0.4.1 (Planned)](#v041-planned)
  - [v0.4.2 (Planned)](#v042-planned)
  - [v0.4.3 (Planned)](#v043-planned)
  - [v0.4.4 (Planned)](#v044-planned)
  - [v0.4.5 (Planned)](#v045-planned)
  - [v0.5.0 (Future)](#v050-future)
  - [Supported Languages](#supported-languages)
- [Feature Details](#feature-details)
  - [Completed Features](#completed-features)
  - [Planned Features](#planned-features)
- [Contributing](#contributing)
- [License](#license)

## How It Works

1. **Parse** - Tree-sitter AST parsing for Rust, Python, and PHP ([more languages coming](#supported-languages))
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

1. **Initialize:**
```bash
# Initialize codanna index space and create .codanna/settings.toml
codanna init
```

2. **Index your codebase:**
```bash
# Index with progress display
codanna index src --progress

# See what would be indexed (dry run)
codanna index . --dry-run

# Index a specific file
codanna index src/main.rs
```

3. **Search your code:**
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

# Unix piping with JSON output
time codanna mcp search_symbols query:parse limit:1 --json | \
    jq -r '.data[0].name' | \
    xargs -I {} codanna retrieve callers {} --json | \
    jq -r '.data[] | "\(.name) in \(.module_path)"'

# Result:
#
# main in crate::main
# serve_http in crate::mcp::http_server::serve_http
# serve_http in crate::mcp::http_server::serve_http
# serve_https in crate::mcp::https_server::serve_https
# serve_https in crate::mcp::https_server::serve_https
# parse in crate::parsing::rust::parse
# parse in crate::parsing::rust::parse
# parse in crate::parsing::python::parse
#
# codanna mcp search_symbols query:parse limit:1 --json  0.10s user 0.08s system 122% cpu 0.143 total
# jq -r '.data[0].name'  0.00s user 0.00s system 3% cpu 0.142 total
# xargs -I {} codanna retrieve callers {} --json  0.11s user 0.07s system 63% cpu 0.288 total
# jq -r '.data[] | "\(.name) in \(.module_path)"'  0.00s user 0.00s system 1% cpu 0.288 total

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
| **PHP** | 68,432 symbols/sec | 6.8x faster ✅ | Production |
| JavaScript | - | - | v0.4.1 |
| TypeScript | - | - | v0.4.1 |

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

- Supports Rust, Python, and PHP (JavaScript/TypeScript coming in v0.4.1)
- Semantic search requires English documentation/comments
- Windows support is experimental

## Roadmap

### Version Strategy
- **0.3.x** - CLI improvements and API stability
- **0.4.x** - Language expansion via modular architecture
- **0.5.x** - Enterprise features and advanced analysis

### v0.3.0 (Released)
| Feature | Description | Status |
|---------|-------------|--------|
| [JSON Output Support](#json-output-support) | Structured output for all commands | ✅ |
| [Exit Codes](#exit-codes) | Semantic exit codes for scripting | ✅ |
| [Unix-Friendly CLI](#unix-friendly-cli) | Positional args and key:value syntax | ✅ |
| [Incremental Index Updates](#incremental-index-updates) | File watching with auto re-indexing | ✅ |

### v0.4.0 (Next Release)
| Feature | Description | Status |
|---------|-------------|--------|
| [Language Registry Architecture](#language-registry-architecture) | Modular parser system for easy language additions | ✅ |
| [PHP Support](#php-support) | Full PHP parser implementation | ✅ |
| [Python Enhancements](#python-enhancements) | Complete Python class and decorator support | 🔧 |

### v0.4.1 (Planned)
| Feature | Description | Status |
|---------|-------------|--------|
| [JavaScript Support](#javascript-support) | Full JavaScript/ES6+ parser | 📋 |
| [TypeScript Support](#typescript-support) | TypeScript with type annotations | 📋 |

### v0.4.2 (Planned)
| Feature | Description | Status |
|---------|-------------|--------|
| [Go Support](#go-support) | Go language with interfaces and goroutines | 📋 |

### v0.4.3 (Planned)
| Feature | Description | Status |
|---------|-------------|--------|
| [C# Support](#csharp-support) | C# with .NET ecosystem support | 📋 |

### v0.4.4 (Planned)
| Feature | Description | Status |
|---------|-------------|--------|
| [Java Support](#java-support) | Java with class hierarchies | 📋 |

### v0.4.5 (Planned)
| Feature | Description | Status |
|---------|-------------|--------|
| [C/C++ Support](#c-cpp-support) | C and C++ with headers and templates | 📋 |

### v0.5.0 (Future)
| Feature | Description | Status |
|---------|-------------|--------|
| [Direct Semantic Search](#direct-semantic-search) | `retrieve semantic` command | 📋 |
| [Batch Operations](#batch-operations) | Process multiple symbols in one call | 📋 |
| [Output Format Control](#output-format-control) | Compact/full/json output modes | 📋 |
| [Query Language](#query-language) | Advanced search with complex filters | 📋 |
| [Configuration Profiles](#configuration-profiles) | Environment-specific settings | 📋 |
| [Machine-Readable Progress](#machine-readable-progress) | JSON progress output | 📋 |
| [Cross-Language References](#cross-language-references) | Track references across languages | 📋 |
| [Language Server Protocol](#language-server-protocol) | LSP integration for IDEs | 📋 |

**Legend:** ✅ Complete | 🔧 In Progress | 📋 Planned

### Supported Languages

#### Currently Supported (v0.4.0)
- **Rust** - Full support with trait implementations and generics
- **Python** - Functions, classes, and imports  
- **PHP** - Classes, functions, and namespaces

#### Coming Soon
Based on developer demand and tree-sitter support:
1. **JavaScript/TypeScript** (v0.4.1) - Most requested for web development
2. **Go** (v0.4.2) - Growing popularity in cloud/backend
3. **C#** (v0.4.3) - Enterprise and game development
4. **Java** (v0.4.4) - Enterprise applications
5. **C/C++** (v0.4.5) - Systems programming

---

## Feature Details

### Completed Features

#### json-output-support
All retrieve commands and MCP tools support `--json` flag for structured output with consistent format and proper exit codes (v0.3.0).

#### exit-codes  
Semantic exit codes for scripting: 0 (success), 1 (general error), 3 (not found). Enables reliable automation (v0.3.0).

#### unix-friendly-cli
Simplified syntax with positional arguments for simple tools and key:value pairs for complex tools. No JSON escaping needed (v0.3.0).

#### incremental-index-updates
Watch mode with automatic re-indexing of changed files. Broadcast channels coordinate updates with 500ms debouncing (v0.3.0).

#### language-registry-architecture
Modular parser system where languages self-register via a registry. Enables easy addition of new languages without core code changes (v0.4.0).

#### php-support
Full PHP parser with classes, functions, namespaces, and traits. Supports PHP 5 through PHP 8 syntax (v0.4.0).

### Planned Features

#### direct-semantic-search
Direct `retrieve semantic` command for natural language code search without going through MCP interface.

#### batch-operations
Process multiple symbols in a single command to reduce overhead and improve CI/CD performance.

#### output-format-control
Choose between compact (script-friendly), full (human-readable), and json output formats.

#### javascript-support
Full JavaScript/ES6+ parser with modules, classes, async/await, and JSX support.

#### typescript-support
TypeScript parser with full type annotation support, interfaces, and decorators.

#### go-support
Go language parser with interfaces, goroutines, channels, and struct methods.

#### csharp-support
C# parser with .NET ecosystem support, LINQ, async/await, and attributes.

#### java-support
Java parser with class hierarchies, interfaces, generics, and annotations.

#### c-cpp-support
C and C++ parsers with headers, templates, macros, and cross-compilation units.

#### query-language
Advanced search syntax with wildcards, boolean operators, and complex filters.

#### configuration-profiles
Environment-specific settings (dev, test, production) with profile inheritance.

#### machine-readable-progress
JSON-formatted progress output for better CI/CD integration and monitoring.

#### cross-language-references
Track and analyze references across different programming languages in polyglot codebases.

#### language-server-protocol
LSP implementation for IDE integration with real-time code intelligence.

## Contributing

This is an early release focused on core functionality. Contributions welcome! See [CONTRIBUTING](CONTRIBUTING.md) for guidelines.

## License

Licensed under the Apache License, Version 2.0 - See [LICENSE](LICENSE) file for details.

Attribution required when using Codanna in your project. See [NOTICE](NOTICE) file.

---

Built with 🦀 by developers who wanted their AI assistants to actually understand their code.