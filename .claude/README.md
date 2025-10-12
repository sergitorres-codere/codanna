# Codanna Slash Commands for Claude

This directory contains custom slash commands that integrate Codanna's CLI with Claude for powerful code analysis workflows.

## Available Commands

### Core Analysis Commands

- `/impact <symbol>` - Complete impact analysis for any symbol
- `/context <symbol>` - Build comprehensive context including relationships
- `/find <query>` - Smart semantic search with natural language
- `/trace <function> [depth]` - Trace call graph from a starting point
- `/deps <symbol>` - Analyze dependencies of a symbol

## Philosophy

These commands follow the Unix philosophy:
- Simple CLI commands that do one thing well
- Composed via pipes and bash for complex workflows
- No disk I/O needed - everything pipes through memory
- Clean JSON output for structured analysis

## Creating Your Own Commands

1. Create a new `.md` file in this directory
2. Add YAML frontmatter with `allowed-tools` and `description`
3. Use `$ARGUMENTS` for dynamic values
4. Use `!` prefix for bash command execution
5. Pipe commands together for complex analysis

### Example Template

```markdown
---
allowed-tools: Bash(codanna retrieve:*), Bash(jq:*)
description: Your command description
argument-hint: <symbol_name>
---

## Analysis for $ARGUMENTS

!`codanna retrieve symbol $ARGUMENTS --json | jq '.data.items[0]'`

Provide analysis based on the results.
```

## Requirements

- Codanna must be installed and in PATH
- Commands must be indexed (`codanna index . --force`)
- jq must be available for JSON processing

## Performance

All commands target < 1 second total execution time through:
- Limiting result sets with `jq` filters
- Using `head` to cap output
- Focusing on relevant data only