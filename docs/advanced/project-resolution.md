# Project-Specific Path Resolution

Codanna understands project configuration files and uses them to resolve imports correctly.

## TypeScript

Reads `tsconfig.json` to resolve path aliases.

### Configuration

```toml
# .codanna/settings.toml
[languages.typescript]
enabled = true
config_files = [
    "tsconfig.json",
    "packages/web/tsconfig.json"  # For monorepos
]
```

### How It Works

When your TypeScript code imports `@app/utils`, Codanna uses your `tsconfig.json` path mappings to resolve it to the actual file location (`src/app/utils`). This works across modules in monorepos.

**Process:**
1. Codanna reads your project config files (`tsconfig.json`)
2. Extracts path aliases, baseUrl, and other resolution rules
3. Stores them in `.codanna/index/resolvers/`
4. Uses these rules during indexing to resolve imports accurately

### Example

Given this `tsconfig.json`:

```json
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@app/*": ["src/app/*"],
      "@utils/*": ["src/utils/*"],
      "~/components/*": ["components/*"]
    }
  }
}
```

Codanna will resolve:
- `@app/main` → `src/app/main`
- `@utils/config` → `src/utils/config`
- `~/components/Button` → `components/Button`

### Monorepo Support

For monorepos with multiple `tsconfig.json` files:

```toml
[languages.typescript]
config_files = [
    "tsconfig.json",
    "packages/web/tsconfig.json",
    "packages/api/tsconfig.json"
]
```

Each config's path mappings are applied to files within its scope.

## Coming Soon

### Python
Project-specific import resolution using `pyproject.toml`:
- Package discovery
- Namespace packages
- Editable installs

### Go
Module resolution using `go.mod`:
- Module path resolution
- Replace directives
- Local module references

### Other Languages
Language-specific import resolution as needed.

## Benefits

- **Accurate Import Resolution** - Follows your project's rules
- **Cross-Module Navigation** - Works in monorepos
- **Path Alias Support** - Handles `@app/*`, `~/utils/*` patterns
- **No Manual Configuration** - Reads existing project config

## Troubleshooting

### Imports Not Resolving

Check that config files are listed:
```bash
codanna config | grep config_files
```

Verify paths in your `tsconfig.json` are correct.

### Monorepo Issues

Ensure all relevant `tsconfig.json` files are listed in settings.toml.

### Re-indexing After Changes

After modifying path aliases, re-index:
```bash
codanna index . --force --progress
```

## See Also

- [Configuration](../user-guide/configuration.md) - Complete configuration guide
- [First Index](../getting-started/first-index.md) - Creating your first index