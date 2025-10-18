# Release v0.5.3 Commit

## Complete commit command (copy and paste):

```bash
git add -A && git commit -S -m "chore(release): prepare v0.5.3 documentation and assets

- added documentation hub at docs/README.md with section navigation
- updated README.md with new documentation link

Documentation changes since last release:
- docs: reorganize documentation structure for pre-release
- refactor(symbol): consolidate location formatting and enhance relationships display
- docs(semantic): clarify model switching requires re-indexing
- fix(typescript): track calls from object property functions
- feat: configurable embedding model support for multilingual semantic search
- feat(symbol): add file path field with line number
- feat(plugins): add marketplace resolution for external sources
- perf(plugins): skip I/O when update detects no changes
- chore(plugins): cleanup and tests
- feat(plugins): add transactional installs with rollback
- feat(plugins): implement plugin install/remove flow
- feat(plugins): add plugin management infrastructure for Claude Code

Tests passing. Version bumped to 0.5.3 in Cargo.toml."
```

## Changes Summary

**Release Preparation:**
- Version 0.5.3 (already bumped in Cargo.toml)
- Documentation hub created at docs/README.md
- Logotype header added for branding
- Navigation footers added across all doc sections
- Tests passing

**Key Features Since Last Release:**
- Plugin management system (install, remove, marketplace)
- Configurable embedding models for multilingual search
- Enhanced symbol display with file paths and line numbers
- TypeScript call tracking improvements
- Performance optimizations in plugin updates

**Documentation Updates:**
- Reorganized structure for clarity
- Added section navigation
- Enhanced plugins documentation
- Updated contributing guidelines
- Cleaned up completed commit logs
