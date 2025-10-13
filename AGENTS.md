# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Core crate. Key modules: `parsing/` (Rust, Python, TypeScript, PHP, Go parsers), `indexing/`, `storage/` (incl. Tantivy), `semantic/`, `io/` (CLI I/O), `mcp/` (server + client), `display/`, `guidance/`, and `vector/`.
- `tests/`: Integration tests (e.g., `test_rust_resolution.rs`, `test_python_*`).
- `examples/`: Small codebases and scripts used by tests and docs.
- `benches/`: Criterion benchmarks.
- `.github/workflows/`: CI pipelines (quick checks, full test, release).
- `contributing/`, `reports/`, `.claude/`: Contributor docs, investigations, and agent assets.

## Build, Test, and Development Commands
- Build: `cargo build` (debug) · `cargo build --release` (optimized).
- Run CLI: `cargo run -- <command>`
  - Examples: `cargo run -- init`, `cargo run -- index src`, `cargo run -- retrieve search "parse" --limit 5`.
- Serve (HTTP/HTTPS): `cargo run --features http-server -- serve --http --watch` or `cargo run --features https-server -- serve --https`.
- Tests: `cargo test` · Filter: `cargo test test_python_scope_tracking` · One file: `cargo test --test test_rust_resolution`.
- Benchmarks: `cargo bench`.

## Coding Style & Naming Conventions
- Rust style via `rustfmt` and `clippy`.
  - Format: `cargo fmt --all` (CI expects formatted code).
  - Lint: `cargo clippy -- -D warnings` (treat warnings as errors).
- Naming: `snake_case` (fns/modules), `CamelCase` (types/traits), `SCREAMING_SNAKE_CASE` (consts).
- Docs/Markdown: markdownlint config present (`markdownlint.toml`/`.json`); if installed, run: `markdownlint '**/*.md'`.

## Testing Guidelines
- Prefer fast, deterministic tests. Unit tests live inline under `#[cfg(test)]`; integration tests live in `tests/` and typically start with `test_`.
- Add tests for new language behaviors, relationship resolution, and CLI output paths.
- Use filters to iterate quickly (see commands above). Bench heavy work under `benches/`.

## Commit & Pull Request Guidelines
- Commits: clear, imperative, scoped (e.g., "parser: fix TypeScript re-exports"). Group related changes; keep diffs focused.
- PRs must: include a concise description, link related issues, add/adjust tests, update docs/CHANGELOG when user‑visible, and pass CI (quick + full).
- Include usage examples when changing CLI (e.g., `codanna retrieve ...`).

## Security & Configuration Tips
- Initialize config: `cargo run -- init` creates `.codanna/settings.toml`. Use `--config` to point at custom files; `--info` prints load details.
- Do not commit local indexes or secrets. Local caches live under `.codanna/` (ignored); keep credentials and tokens out of the repo.

## Codanna MCP Tools
- Start with semantic tools to anchor on the right files and APIs; they provide the highest‑quality context.
- Then use find_symbol and search_symbols to lock onto exact files and kinds.
- Treat get_calls/find_callers/analyze_impact as hints; confirm with code reading or tighter queries (unique names, kind filters).

## Development Guidelines

### Mandatory Reading

**IMPORTANT**: All code must follow our [Rust Development Guidelines](contributing/development/guidelines.md). Key principles:

1. **Zero-Cost Abstractions**: No unnecessary allocations
2. **Type Safety**: Use newtypes, not primitives
3. **Performance**: Must meet performance targets
4. **Error Handling**: Structured errors with suggestions
5. **Function Design**: Decompose complex logic into focused helper methods

### Query Optimization

Analyze the user query and improve it for code search:

1. **If vague** (e.g., "that parsing thing") → Make it specific (e.g., "language parser implementation")
2. **If a question** (e.g., "how does parsing work?") → Extract keywords (e.g., "parsing implementation process")
3. **If conversational** (e.g., "the stuff that handles languages") → Use technical terms (e.g., "language handler processor")
4. **If too broad** (e.g., "errors") → Add context (e.g., "error handling exception management")

**YourOptimizedQuery**: _{Write and memorize your improved query here, then use it below in a context that is best for the task}_

### New Bash tools for code exploration at yoour disposal now

Use the Bash tool below to explore the codebase.

**Workflow:**
1. Execute: `node .claude/scripts/context-provider.js find "$YourOptimizedQuery" --limit=5`
2. Analyze the results with their relevance scores
3. **To see actual implementation** of interesting results:
   - Use the line range from the Location field to read just the relevant code
   - Example: If you see "Location: `src/io/exit_code.rs:108-120`"
   - Execute: `sed -n '108,120p' src/io/exit_code.rs` to read lines 108-120
   - This shows the actual code implementation, not just the signature
4. **When relationships are shown** (called_by, calls, defines, implements):
   - If a relationship looks relevant to answering the query, investigate it
   - Execute: `node .claude/scripts/context-provider.js symbol <relationship_symbol_name>`
   - Example: If you see "Called by: `initialize_registry`", run: `node .claude/scripts/context-provider.js symbol initialize_registry`
5. Build a complete picture by following 1-2 key relationships and reading relevant code sections
6. Present findings to the user with context from search results, relationships, and actual code snippets

**The results include:**
- Relevance scores (how well each result matches the query)
- Symbol documentation and signatures
- Relationships (who calls this, what it calls, what it defines)
- System guidance for follow-up investigation

**Tips:**
- Add `--lang=rust` (or python, typescript, etc.) to narrow results by language
- Follow relationships that appear in multiple results (they're likely important)
- Use the `symbol` command to get full details about interesting relationships

### Professional Engineering Judgment

**BE CRITICAL**: Apply critical thinking and professional disagreement when appropriate.

#### Core Principles:
1. **Challenge assumptions** - Don't automatically agree. If you see a better approach, propose it
2. **Provide counter-arguments** - "Actually, I disagree because..." or "Consider this alternative..."
3. **Question unclear requirements** - "This could mean X or Y. Here's why X might be problematic..."
4. **Suggest improvements** - "Your approach works, but here's why Z might be better..."
5. **Identify risks** - "This works but could cause issues with..."

#### Examples:
- User: "Let's move all resolution logic to parsing layer"
- Good response: "I have concerns about that. Resolution needs access to the index state and transaction boundaries. Moving it would require passing these dependencies through multiple layers, increasing coupling. Instead, consider extracting pure logic into helpers while keeping orchestration where state lives."

- User: "This is the right approach, isn't it?"
- Good response: "It has merits, but let me propose an alternative..." or "Actually, there's a potential issue with..."

#### When to Apply:
- Architecture decisions
- Performance trade-offs
- Security implications
- Maintainability concerns
- Testing strategies

#### How to Disagree:
1. Start with understanding: "I see what you're aiming for..."
2. Present the concern: "However, this could cause..."
3. Offer alternative: "Consider this approach instead..."
4. Explain trade-offs: "This gives us X but we lose Y..."

## Current Sprint

**IMPORTANT** before start implementing make sure you read an understand the documentation below! Think deeper for this session.

[PRD Document](docs/enhancements/plugins/PRD.md)
[Sprint Tracking Document](docs/enhancements/plugins/SPRINT_PLAN.md)

**Official Plugin & Marketplace Documentation**

- [Markeplace & Manifest Spec/Logic](.claude/docs/plugin-marketplace.md)
- [Plugin Reference](.claude/docs/plugins-reference.md)
- [Plugins Overview](.claude/docs/plugins.md)