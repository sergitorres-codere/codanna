# Claude Code Integration

Set up Codanna with Claude's official CLI.

## Configuration

Add this to your local `.mcp.json`:

```json
{
  "mcpServers": {
    "codanna": {
      "command": "codanna",
      "args": ["serve", "--watch"]
    }
  }
}
```

## Features

- File watching with `--watch` flag
- Auto-reload on index changes
- stdio transport (default)

## Verification

After configuration, verify the connection:

```bash
codanna mcp-test
```

This will confirm Claude can connect and list available tools.

## Agent Workflow

Tool priority:
- **Tier 1**: semantic_search_with_context, analyze_impact
- **Tier 2**: find_symbol, get_calls, find_callers
- **Tier 3**: search_symbols, semantic_search_docs, get_index_info

Workflow:
1. semantic_search_with_context - Find relevant code with context
2. analyze_impact - Map dependencies and change radius
3. find_symbol, get_calls, find_callers - Get specific details

Start with semantic search, then narrow with specific queries.

## Troubleshooting

- Ensure Codanna is in your PATH
- Check `.codanna/settings.toml` exists in your project
- Run `codanna index` before starting the server

## See Also

- [MCP Tools Reference](../user-guide/mcp-tools.md)
- [Agent Guidance](agent-guidance.md)
- [Configuration](../user-guide/configuration.md)