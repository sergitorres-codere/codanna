# Configuration Guide

Codanna configuration lives in `.codanna/settings.toml`.

## Configuration File Location

```bash
.codanna/
├── plugins/          # Plugin lockfile 
├── index/            # Index storage
├── .project-id       # Unique project id used in ~/.codanna to manage global configurations
└── settings.toml     # Main configuration
```

## Basic Configuration

```toml
# .codanna/settings.toml

# Semantic search model configuration
[semantic]
# Model to use for embeddings
# - AllMiniLML6V2: English-only, 384 dimensions (default)
# - MultilingualE5Small: 94 languages, 384 dimensions (recommended for multilingual)
# - MultilingualE5Base: 94 languages, 768 dimensions (better quality)
# - ParaphraseMLMiniLML12V2: Multilingual, 384 dimensions (paraphrase-optimized)
# - See architecture/embedding-model.md for full details
model = "AllMiniLML6V2"
```

[Read more about embedding models](../architecture/embedding-model.md)

```toml
# Agent guidance configuration
[guidance]
enabled = true
```
[Learn more about agent guidance](../integrations/agent-guidance.md)

## Language Configuration

### TypeScript

Reads `tsconfig.json` to resolve path aliases:

```toml
[languages.typescript]
enabled = true
config_files = [
    "tsconfig.json",
    "packages/web/tsconfig.json"  # For monorepos
]
```

When your TypeScript code imports `@app/utils`, Codanna uses your `tsconfig.json` path mappings to resolve it to the actual file location (`src/app/utils`). This works across modules in monorepos.

### Other Languages

Coming soon: Python (`pyproject.toml`), Go (`go.mod`), and other languages with project-specific import resolution.

## Semantic Search Models

### Available Models

| Model | Description | Use Case |
|-------|-------------|----------|
| `AllMiniLML6V2` | Fast, English-optimized (default) | English codebases |
| `MultilingualE5Small` | Multilingual, same speed as AllMiniLM | Mixed language teams |
| `MultilingualE5Base` | Better quality, larger size | Quality-critical multilingual projects |
| `ParaphraseMLMiniLML12V2` | Paraphrase-optimized multilingual | Semantic similarity tasks |

### Switching Models

```toml
[semantic]
model = "MultilingualE5Small"
```

**Note:** Changing models requires re-indexing:
```bash
codanna index . --force --progress
```

## Agent Guidance Templates

Configure how Codanna guides AI assistants:

```toml
[guidance]
enabled = true

[guidance.templates.find_callers]
no_results = "No callers found. Might be an entry point or dynamic dispatch."
single_result = "Found 1 caller. Use 'find_symbol' to inspect usage."
multiple_results = "Found {result_count} callers. Try 'analyze_impact' for the full graph."

[guidance.templates.analyze_impact]
no_results = "No impact detected. Likely isolated."
single_result = "Minimal impact radius."
multiple_results = "Impact touches {result_count} symbols. Focus critical paths."

[[guidance.templates.analyze_impact.custom]]
min = 20
template = "Significant impact with {result_count} symbols. Break the change into smaller parts."
```

## Indexing Configuration

```toml
[indexing]
threads = 8  # Number of threads for parallel indexing
max_file_size_mb = 10  # Skip files larger than this
```

## Ignore Patterns

Codanna respects `.gitignore` and adds its own `.codannaignore`:

```bash
# .codannaignore
.codanna/       # Don't index own data
target/         # Skip build artifacts
node_modules/   # Skip dependencies
*_test.rs       # Optionally skip tests
```

## HTTP/HTTPS Server Configuration

For server mode configuration:

```toml
[server]
bind = "127.0.0.1:8080"
watch_interval = 5  # Seconds between index checks
```

## Performance Tuning

```toml
[performance]
cache_size_mb = 100  # Memory cache size
vector_cache_size = 10000  # Number of vectors to keep in memory
```

## Command-Line Overrides

Most settings can be overridden via command-line:

```bash
# Override config file
codanna --config /path/to/custom.toml index .

# Override thread count
codanna index . --threads 16

# Force specific settings
codanna serve --watch --watch-interval 10
```

## Viewing Configuration

```bash
# Display active settings
codanna config

# Show config with custom file
codanna --config custom.toml config
```

## Configuration Precedence

1. Command-line flags (highest priority)
2. Custom config file (via `--config`)
3. Project `.codanna/settings.toml`
4. Built-in defaults (lowest priority)

## Project-Specific Path Resolution

### How It Works

1. Codanna reads your project config files (`tsconfig.json`)
2. Extracts path aliases, baseUrl, and other resolution rules
3. Stores them in `.codanna/index/resolvers/`
4. Uses these rules during indexing to resolve imports accurately

### Benefits

- Accurate import resolution
- Cross-module navigation in monorepos
- Support for path aliases (`@app/*`, `~/utils/*`)
- No manual configuration needed

## Troubleshooting

### Index Not Updating

Check watch interval:
```toml
[server]
watch_interval = 5  # Lower for more frequent checks
```

### Semantic Search Not Working

1. Ensure documentation comments exist
2. Check model is appropriate for your language
3. Re-index after configuration changes

### Path Resolution Issues

Verify config files are listed:
```toml
[languages.typescript]
config_files = ["tsconfig.json"]
```

## See Also

- [First Index](../getting-started/first-index.md) - Creating your first index
- [Agent Guidance](../integrations/agent-guidance.md) - Configuring AI assistant behavior
- [CLI Reference](cli-reference.md) - Command-line options