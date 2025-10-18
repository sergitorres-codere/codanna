# Documentation Reorganization Commit

## Complete commit command (copy and paste):

```bash
git add -A && git commit -S -m "docs: reorganize documentation structure for pre-release"
```

## Changes Summary

**Documentation reorganization (26 new files in docs/):**
- Getting started: installation, quick-start, first-index guides
- User guide: CLI reference, configuration, MCP tools, search guide
- Advanced: performance, project-resolution, slash-commands, unix-piping
- Architecture: how-it-works, embedding-model, language-support, memory-mapping
- Integrations: Claude Code, Claude Desktop, HTTPS setup, HTTP server, agent guidance, Codex CLI
- Plugins: plugin system documentation
- Contributing: contribution guidelines
- Reference: API and reference documentation

**Cleanup:**
- Removed .mcp.json (obsolete)
- Removed AGENTS.md (migrated to docs/)
- Removed tsconfig.json (not applicable)

**Version updates:**
- Cargo.lock dependency refresh
- Minor import adjustment in src/main.rs

**Reports:**
- Added agent lifecycle analysis report (2025-10-17-14-00)

Total changes: +3,790 lines, -190 lines across 34 files
