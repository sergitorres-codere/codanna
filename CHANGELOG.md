# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.25] - 2025-10-08

### Fixed
- C++ parser: Member function call detection for method invocations
  - Extract method names from field_expression nodes (obj->method, obj.method)
  - Extract method names from qualified_identifier in function context (Class::method)
  - Function context tracking now handles qualified method implementations
  - Register call_expression, field_expression, qualified_identifier in audit system
- MCP analyze_impact: Handle all symbols with same name instead of first match only
  - Changed from find_symbol (single) to find_symbols_by_name (all matches)
  - Aggregate impact across all symbols with same name
  - Show locations and direct caller counts for each symbol variant

## [0.5.24] - 2025-10-07

### Fixed
- C++ parser: Extract class methods from declarations and implementations
  - Method declarations inside classes now extracted as SymbolKind::Method
  - Out-of-class implementations (Class::method) identified as methods
  - Qualified_identifier pattern (Class::method) detection in function_definition
  - Class_specifier enters class scope and processes children recursively
  - Field_declaration extracts methods from function_declarator nodes
  - Tested with Qt QWindow: 144 methods extracted (was 0 before)

## [0.5.23] - 2025-10-07

### Changed
- Bump rmcp from 0.7.0 to 0.8.0

## [0.5.22] - 2025-10-07

### Added
- C++ parser: Doxygen doc comment extraction (/** */ and ///)
- C++ parser: Recursive call tracking with function context
- C++ parser: Scope context tracking via ParserContext

### Fixed
- MCP get_index_info now displays all symbol kinds dynamically
- C++ Audit system uses proper tree-sitter node names to generate the report

## [0.5.21] - 2025-10-03

### Added
- Recursion depth guards across all language parsers
  - `check_recursion_depth()` prevents stack overflow on deeply nested AST structures
  - All parsers (TypeScript, Python, Rust, Go, PHP, C++, C#) now track depth in `extract_symbols_from_node()`
  - Safely handles pathological code with excessive nesting (tested on Qt keyboard at depth 3521)

### Changed
- **PERFORMANCE**: Optimized resolution pipeline for large codebases
  - Indexed method calls as HashMap for O(1) lookup instead of linear search
  - Added symbol lookup cache to eliminate duplicate Tantivy queries
  - Qt qtbase (8,508 files, 413K symbols): 7m38s total, relationship resolution processes 4.68M relationships with 4,778 resolved, 4.39M skipped
  - Skipped relationships: external symbols not in index (Qt framework dependencies, system libraries)
- Parser method signatures updated to accept depth parameter
- Audit reports and grammar analysis regenerated for all languages

## [0.5.20] - 2025-10-02

### Added
- C# language support with full parser implementation (PR#39)
  - Symbol extraction for classes, interfaces, structs, enums, methods, properties, fields
  - Relationship tracking for inheritance, interface implementation, and method calls
  - XML documentation comment extraction
  - File extensions: `.cs`, `.csx`, `.cshtml`
- Fuzzy search on non-tokenized name field for whole-word typo tolerance
  - Handles missing character typos in full symbol names (e.g., "ArchivService" finds "ArchiveService")
  - Dual fuzzy strategy: ngram tokens for partial matches + whole words for full name typos

### Changed
- **BREAKING**: Tantivy schema `name` field changed from TEXT to STRING
  - Enables exact matching without tokenization for fuzzy search
  - Requires full reindex: `codanna index --force`
- **PERFORMANCE**: Batch commits every 100 files instead of per-file commits
  - 10-50x faster indexing (varies by platform and file count)
  - macOS: ~10x improvement on typical projects
  - Windows: 25-50x improvement (1-2 files/s → 46 files/s on 4,453 file project)
  - Reduces disk I/O, segment creation, and cache rebuilds
- Automatic reverse relationship creation for bidirectional graph navigation
  - Implements ↔ ImplementedBy, Extends ↔ ExtendedBy, Calls ↔ CalledBy, Uses ↔ UsedBy

### Fixed
- File ID counter race condition during batch operations
  - Pending counter prevents stale committed values from causing duplicate IDs
- Windows file locking issues with proper retry logic and error logging
  - Symbol cache and persistence layer handle OS error 1224 and permission denied

## [0.5.19] - 2025-10-01

### Added
- Full symbol boundary tracking for precise editor navigation
  - `create_symbol()` accepts `full_node` parameter for complete range extraction
  - Tantivy schema extended with `end_line` and `end_column` fields
  - MCP tools now return precise symbol ranges (start_line, start_column, end_line, end_column)

### Changed
- C parser: Functions, structs, unions, enums, fields, and macros now use full boundaries
- Rust parser: Functions, structs, enums, traits, and modules now use full boundaries
- README: Added documentation for precise symbol boundary support

## [0.5.18] - 2025-09-30

### Added
- JSX component usage tracking in TypeScript parser
  - New `component_usages` field tracks function → component relationships
  - `extract_jsx_uses_recursive()` traverses AST to find JSX elements
  - `track_jsx_component_usage()` filters components by uppercase naming convention
  - Supports `jsx_element` and `jsx_self_closing_element` nodes
  - Generator functions (`generator_function_declaration`) included in function context
- Test fixtures for JSX usage patterns
  - Profile.tsx: React component with JSX
  - test_documented_jsx.tsx: JSX with documentation
  - test_jsx_same_file.tsx: JSX defined and used in same file
  - test_jsx_usage.tsx: Multiple components using shared JSX

### Changed
- Audit reports regenerated to reflect JSX and generator function support
- All language parser audit reports updated with latest node counts

## [0.5.17] - 2025-09-29

### Changed
- Refactored relationship compatibility logic from indexer to language behaviors
  - Moved `is_compatible_relationship` from SimpleIndexer to ResolutionScope trait
  - Each language now controls its own relationship validation rules
  - Cleaner separation between orchestration and language-specific logic

### Fixed
- UTF-8 character boundary parsing error when encountering Unicode characters
  - Added `safe_substring_window()` utility for UTF-8-safe string slicing
  - TypeScript parser now handles box-drawing characters and emojis correctly
  - Prevents panic when checking for export modifiers before symbols
  - Fixes Issue #38

## [0.5.16] - 2025-09-28

### Added
- TypeScript path alias resolution with full cross-module support
  - Aliases like `@/*` resolved to actual paths (`./src/*`)
  - Symbols added by module_path for cross-module resolution
  - Import paths enhanced at storage time for correct resolution
- Default export visibility tracking for TypeScript
  - `export default` symbols now marked as Public
  - Enables proper cross-module access to default exports
- React component relationship support
  - Constants and Variables now callable (React functional components)
  - Proper relationship tracking for component hierarchies

### Changed
- **BREAKING**: External stub symbols no longer created for unresolved imports
  - Cleaner index without placeholder symbols
  - Requires full project reindex: `codanna index --force`
- TypeScript behavior enhanced with module_path resolution
- Relationship validation extended for JavaScript/TypeScript patterns

### Fixed
- TypeScript imports using path aliases not resolving across modules
- Default exported symbols incorrectly marked as Private
- React components (Constants) not creating proper call relationships
- Cross-module visibility checks for exported symbols

### Migration Required
To benefit from improved TypeScript resolution:
```bash
codanna index --force
```

## [0.5.15] - 2025-09-27

### Added
- Cross-module resolution: Full qualified path resolution for all languages
  - Symbols now resolvable by both simple name and full module path
  - Example: `crate::init::init_global_dirs`, `app.utils.helper.process_data`
- Python parser: Methods now use qualified names (e.g., `Calculator.__init__`)
- Resolution tests for Rust and Python cross-module calls
- Architectural documentation: Universal vs language-specific concepts

### Changed
- **BREAKING**: Python method naming convention - requires reindexing Python codebases
- Resolution context: Module paths added during symbol population

### Fixed
- Cross-module function calls not being resolved (e.g., `crate::module::function`)
- Python parser tests updated for new qualified naming convention

## [0.5.14] - 2025-09-25

### Added
- Global model cache system at `~/.codanna/models` for shared FastEmbed models across projects
- Project registry tracking all indexed projects with unique IDs
- `codanna init` command to initialize project structure and create model symlinks
- Test isolation with separate directories (`~/.codanna-test`) for development

### Changed
- **BREAKING**: Existing `.fastembed_cache` directories must be deleted before running `init --force`
- Model storage moved from per-project directories to global cache via symlinks
- Settings validation now checks for proper initialization on startup


### Migration Required
To upgrade existing projects:
```bash
rm -rf .fastembed_cache
codanna init --force
```

## [0.5.13] - 2025-09-13

### Fixed
- Python parser: Module-level function calls and class instantiations now tracked (fixes #32)
  - Module symbol created for each Python file to represent module scope
  - Module-level calls tracked with `<module>` as caller, mapped to actual module path for queries
  - `normalize_caller_name()` maps synthetic names to searchable module paths
  - `configure_symbol()` renames module symbols for searchability
  - Module type accepted as valid caller in relationship validation
  - External symbol resolution handles unresolved import targets
  - Method call resolution normalizes caller names for consistent matching

### Added
- Python parser: Module-level execution tracking for better code analysis
- Tests: Module-level class instantiation detection verification

## [0.5.12] - 2025-09-12

### Fixed
- MCP server: Fixed tool discovery issue after rmcp 0.6.4 upgrade (fixes #31)
  - Tools without parameters now generate proper `{"type": "object"}` schema
- Parser safety: Fixed UTF-8 string truncation panic when encountering emojis or multi-byte characters (fixes #29)
  - Added `safe_truncate_str` and `truncate_for_display` utilities that respect UTF-8 boundaries
  - Applied fix to Python and PHP parsers where manual truncation was used
  - Zero-cost implementation returning string slices without allocation

### Improved
- MCP server instructions: Updated workflow guidance to emphasize semantic search first approach for better code exploration

## [0.5.11] - 2025-09-11

### Added
- React example app under `examples/typescript/react` demonstrating call tracking for React hooks and component methods.

### Fixed
- TypeScript parser/indexer: Function call relationships correctly tracked in React projects (fixes #23)
  - React hooks (`useState`, `useEffect`) and component methods properly detected
  - Call relationships preserved during full project indexing
  - External module symbols correctly resolved with unique IDs

## [0.5.10] - 2025-09-11

### Added
- Parse command: output AST nodes in JSONL format for debugging
- Parse command flags: --max-depth, --all-nodes, --output
- Tree-sitter CLI detection in development scripts

### Fixed
- TypeScript parser: improved nested node extraction in arrow functions and JSDoc blocks (123/182 coverage)
- Test parallel execution race conditions with unique temp files
- CLI startup performance for non-index commands (parse, config, benchmark)

### Changed
- Parser audit reports now include timestamps
- Parse command integration tests moved to proper test structure

## [0.5.9] - 2025-09-07

### Enhanced
- **codanna-navigator agent**: Improved code research reports with quantified findings, investigation paths, and actionable insights

### Added
- C/C++ language support with tree-sitter parsing
- Dynamic NodeTracker system for zero-maintenance parser auditing across all languages
- TypeScript tsconfig.json path resolution infrastructure with persistence (.codanna/index/resolvers/)
- Project-agnostic resolution foundation (ProjectResolutionProvider trait, not yet integrated)
- Python parser extensions: assignment, decorated_definition, type_alias extraction
- Parser API documentation for consistent resolution patterns across languages

### Fixed
- Semantic search: SymbolId persistence between embeddings and symbol index (addresses #23)
- CI: clippy --all-targets --all-features compliance across all parsers

### Changed
- Test infrastructure: enable subfolder organization, removed 20k LOC obsolete tests, added ABI-15 audit (supports #20)
- Memory optimization: symbol-cache candidate lookup with relationship deduplication

### Breaking Changes
- Existing codebases need reindexing with --force or clean new index

## [0.5.8] - 2025-09-01

### Security
- Fixed critical slab vulnerability (RUSTSEC-2025-0047) by updating to v0.4.11
- Replaced unmaintained atty (0.2.14) with is-terminal (0.4.16)
- Resolved RUSTSEC-2024-0375 (atty unmaintained warning)
- Resolved RUSTSEC-2021-0145 (atty potential unaligned read)

### Documentation (internal)
- Added security maintenance documentation
- Created paste dependency analysis and monitoring strategy
- Updated security sprint tracking and procedures

### Changed
- Terminal detection now uses is-terminal crate instead of atty

## [0.5.7] - 2025-09-01

### Fixed
- rmcp 0.6.1 compatibility for `cargo install codanna --locked`
- Symbol counts showing as 0 in `get_index_info`

## [0.5.6] - 2025-08-22

### Fixed
- Clippy warnings in Go resolution (unnecessary unwrap, unused parentheses)
- Documentation build errors with escaped bracket syntax in Go parser
- CI timeouts by ignoring hanging regression tests pending investigation

## [0.5.5] - 2025-08-22

### Added
- Go language support with complete parser implementation
- Go-specific symbol extraction: structs, interfaces, methods, functions, constants, variables
- Go generics support (Go 1.18+) with type parameter parsing
- Go package-level visibility handling (exported vs unexported symbols)
- Go import statement parsing and relationship tracking
- Performance benchmark: 74,545 symbols/sec (7.4x above 10k/s target)

### Fixed
- Retrieve commands relationship data parity with MCP tools
- All 6 retrieve functions now use proper SymbolContext with complete relationship data
- retrieve_describe aggregates relationships from all symbols with same name
- JSON output field population for all retrieve commands

### Changed
- Language registry: Go parser integrated with self-registration architecture
- README: Updated supported languages list to include Go (5 production languages)
- Dependencies: Added tree-sitter-go for Go language parsing

## [0.5.4] - 2025-08-22

### Added
- ResolutionScope::resolve_relationship with default + language-specific overrides
- Support for Defines, Calls, Implements, and qualified name resolution (e.g. Config::new, self::method)
- TDD integration tests for Rust, Python, TypeScript, PHP with real parser validation
- Structured, extensible abstractions for relationship resolution

### Fixed
- Replace ordering hack in SimpleIndexer with ResolutionContext delegation
- Update retrieve describe to aggregate relationships across symbols with same name
- Clean ~40 lines of hack code with professional architecture patterns

### Changed
- Architecture: SimpleIndexer = orchestration only; ResolutionContext = owns logic; per-language behaviors encapsulated
- Maintains <10ms resolution via memory-mapped symbol cache

## [0.5.3] - 2025-08-22

### Added
- Function call tracking for all language parsers via PR #17
- Automatic detection and storage of function calls during indexing
- Call relationships now tracked alongside existing symbol relationships

### Fixed
- MCP schema validation: Changed non-standard `uint` format to `uint32`
- Python parser: Exclude method calls from function call tracking (only track function calls)
- PHP parser: Exclude method calls from function call tracking (only track function calls)
- Test deduplication for function call relationships from multiple analysis passes

### Changed
- CI workflow: Switched to PR-triggered CI with concurrency control for better resource management

## [0.5.2] - 2025-08-21

### Added
- Language filtering for semantic search in mixed-language codebases
- `lang` parameter for `semantic_search_docs` and `semantic_search_with_context` MCP tools
- Language mappings persistence in `.codanna/index/semantic/languages.json`
- `similarity_score_analysis.sh` script demonstrating score consistency
- File paths with line numbers in JSON output for all retrieve commands
- Unified output schema with zero-cost abstractions (OutputManager)
- Dual format support for all retrieve commands (positional and key:value)

New slash commands:
- /find: Smart semantic search with natural language query optimization
- /deps: Dependency analysis with coupling metrics and refactoring insights

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

[0.5.2]: https://github.com/bartolli/codanna/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/bartolli/codanna/compare/v0.4.0...v0.5.1
[0.4.0]: https://github.com/bartolli/codanna/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/bartolli/codanna/compare/v0.2.0...v0.3.0
