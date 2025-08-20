# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.2] - 2025-08-20

### Added
- Language filtering for semantic search in mixed-language codebases
- `lang` parameter for `semantic_search_docs` and `semantic_search_with_context` MCP tools
- Language mappings persistence in `.codanna/index/semantic/languages.json`
- `similarity_score_analysis.sh` script demonstrating score consistency
- File paths with line numbers in JSON output for all retrieve commands
- Unified output schema with zero-cost abstractions (OutputManager)
- Dual format support for all retrieve commands (positional and key:value)

### Fixed
- TypeScript JSDoc extraction for exported functions
- TypeScript parser now correctly finds JSDoc comments above `export function` declarations

### Changed
- Semantic search filters embeddings by language before computing similarity
- Search performance improved in mixed-language projects (up to 75% noise reduction)
- All retrieve commands migrated to OutputManager infrastructure
- Deprecated `impact` command in favor of `analyze_impact` MCP tool

## [0.5.1] - 2025-08-17

### Added
- Comprehensive signature extraction across all language parsers
- Parent context tracking for nested symbols
- PHP Resolution API with namespace resolution and PSR-4 support
- Python Resolution API with LEGB scoping and MRO
- TypeScript type tracking and call graph analysis
- TypeScript re-export and barrel file support

### Fixed
- All scope tests and language behavior doctests
- TypeScript import parsing foundation

## [0.5.0] - Unreleased
_Note: v0.5.0 was an internal milestone, not a public release. Changes were included in v0.5.1._

### Added
- Language registry architecture for modular parser system
- PHP language support with full parser implementation
- TypeScript support with type annotations and interfaces
- Language-agnostic module resolution system

### Changed
- Parser directory reorganization into language-specific subdirectories
- Core systems migrated to registry-based language detection
- ParserFactory integrated with language registry

### Fixed
- Rust symbol extraction for enums, types, and constants
- Inherent methods trait signature handling

## [0.4.0] - 2025-08-13

### Added
- Language registry system for self-registering parsers
- Comprehensive SimpleIndexer refactoring
- Language-specific behavior traits

### Changed
- Major refactor of parsing architecture to support modular languages
- Migration from hard-coded language support to registry pattern

## [0.3.0] - 2025-08-11

### Added
- Unix interface with positional arguments
- JSON output support for all commands
- MCP notifications support
- Optimized CI/CD workflow for rapid development

### Changed
- Improved quick-check workflow for faster feedback

### Performance
- Significant CI pipeline optimization

[Unreleased]: https://github.com/bartolli/codanna/compare/v0.5.2...HEAD
[0.5.2]: https://github.com/bartolli/codanna/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/bartolli/codanna/compare/v0.4.0...v0.5.1
[0.4.0]: https://github.com/bartolli/codanna/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/bartolli/codanna/compare/v0.2.0...v0.3.0