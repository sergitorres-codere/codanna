# C# Support - Quick Start Guide

Get started with codanna's C# support in 5 minutes.

## Installation

```bash
# Install codanna (if not already installed)
cargo install codanna

# Verify installation
codanna --version
```

## Basic Usage

### 1. Index Your C# Code

```bash
cd /path/to/your/csharp/project
codanna index . --progress
```

**Expected output:**
```
Indexing directory: /path/to/your/csharp/project
Indexing: 42/42 files (100%)

Indexing Complete:
  Files indexed: 42
  Symbols found: 387
  Time elapsed: 2.3s

Index saved to: ./.codanna/index
```

### 2. Search for Symbols

```bash
# Find a specific class
codanna retrieve search "MyClass"

# Find all controllers
codanna retrieve search "Controller"

# List all symbols
codanna retrieve search "*" --limit 20
```

### 3. Use with AI (MCP Server)

```bash
# Start MCP server
codanna mcp
```

Then configure Claude Desktop (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "codanna": {
      "command": "/path/to/codanna",
      "args": ["mcp"],
      "cwd": "/path/to/your/csharp/project"
    }
  }
}
```

Now ask Claude natural language questions about your C# code!

## What Gets Indexed

✅ **All symbol types:**
- Classes, interfaces, structs, records, enums
- Methods, constructors, properties, fields
- Events, delegates
- Extension methods, generic types

✅ **Relationships:**
- Method calls (with caller context)
- Interface implementations
- Using directives

✅ **Metadata:**
- Visibility modifiers
- Signatures
- Namespaces
- Documentation comments

## Common Commands

```bash
# Re-index after changes
codanna index . --force --progress

# Search with limit
codanna retrieve search "Service" --limit 10

# Find implementations
codanna retrieve implementations "IUserService"

# Get index statistics
codanna retrieve search "*" --limit 1 | wc -l
```

## Example Queries (with MCP/Claude)

Natural language queries you can ask:

- "Find all public classes in the Services namespace"
- "Show me what methods UserController calls"
- "Which classes implement IRepository?"
- "What would break if I change SaveUser?"
- "Explain the authentication flow"
- "Find all async methods"

## Troubleshooting

**No symbols found?**
```bash
# Check C# files exist
ls *.cs

# Verify C# is enabled
cat .codanna/settings.toml | grep csharp

# Force re-index
codanna index . --force --progress
```

**Slow indexing?**
```bash
# Use more threads
codanna index . --threads 8 --progress

# Exclude build artifacts
echo "bin/" >> .codannaignore
echo "obj/" >> .codannaignore
```

**Permission errors (Windows)?**
- Close Visual Studio
- Check antivirus
- Run as Administrator

## Next Steps

📖 **Full Manual:** See `MANUAL.md` for complete documentation

🔧 **Configuration:** Create `.codanna/settings.toml` for custom settings

🚀 **MCP Integration:** Configure Claude Desktop for AI-powered queries

💡 **Examples:** Check `EXAMPLES.md` for real-world usage patterns

## Need Help?

- Documentation: `MANUAL.md`
- Issues: https://github.com/yourusername/codanna/issues
- Examples: `EXAMPLES.md`

---

**Quick tip:** Start with `codanna index . --progress` and `codanna mcp` - that's all you need for basic usage!