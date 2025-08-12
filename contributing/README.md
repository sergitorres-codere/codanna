# Contributing to Codanna

Thank you for your interest in contributing to Codanna! This guide will help you get started with development, testing, and submitting changes.

## Quick Links

- [Development Setup](#development-setup)
- [Development Guidelines](./development/guidelines.md) - Rust coding principles (MUST READ)
- [Adding Language Support](./development/language-support.md) - How to add new language parsers
- [Testing Your Changes](#testing-your-changes)
- [Submitting Pull Requests](#submitting-pull-requests)

## Development Setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- Git
- A code editor (VS Code with rust-analyzer recommended)

### Getting Started

1. **Fork and clone the repository:**
   ```bash
   git clone https://github.com/bartolli/codanna.git
   cd codanna
   ```

2. **Build the project:**
   ```bash
   cargo build --release --all-features
   ```

3. **Run tests:**
   ```bash
   cargo test
   ```

4. **Set up pre-commit checks:**
   ```bash
   # Make scripts executable
   chmod +x contributing/scripts/*.sh
   
   # Run quick checks before committing
   ./contributing/scripts/quick-check.sh
   ```

## Project Structure

```
codebase-intelligence/
├── src/
│   ├── parsing/         # Language parsers
│   ├── indexing/        # Indexing and file processing
│   ├── storage/         # Tantivy and cache storage
│   ├── mcp/            # MCP server implementation
│   └── main.rs         # CLI entry point
├── contributing/
│   ├── scripts/        # Local CI/CD scripts
│   └── development/    # Development documentation
└── tests/             # Integration tests
```

## Testing Your Changes

We provide local scripts that replicate our CI/CD pipeline:

### 1. Quick Checks (2-3 minutes)
Run before every commit to catch common issues:
```bash
./contributing/scripts/quick-check.sh
```
This runs:
- Format checking (cargo fmt --check)
- Clippy linting (cargo clippy)
- Compile check (cargo check --all-features)

### 2. Auto-Fix Issues
Automatically fix formatting and linting issues:
```bash
./contributing/scripts/auto-fix.sh
```
This will:
- Format your code (cargo fmt)
- Fix clippy issues where possible
- Run quick-check to verify fixes

### 3. Full Test Suite
Run before submitting PR to ensure all tests pass:
```bash
./contributing/scripts/full-test.sh
```
This replicates the complete GitHub Actions workflow.

## Development Guidelines

### Mandatory Reading

**IMPORTANT**: All code must follow our [Rust Development Guidelines](./development/guidelines.md). Key principles:

1. **Zero-Cost Abstractions**: No unnecessary allocations
2. **Type Safety**: Use newtypes, not primitives
3. **Performance**: Must meet performance targets
4. **Error Handling**: Structured errors with suggestions
5. **Function Design**: Decompose complex logic into focused helper methods

### Code Style

- Use `cargo fmt` for formatting
- Fix all `cargo clippy` warnings
- Write tests for new features
- Document public APIs with examples

### Performance Requirements

- Parser speed: >10,000 symbols/second (AST parsing only)
- Symbol lookups: <10ms from memory-mapped cache
- Semantic search: <300ms end-to-end (including embedding generation)
- Memory usage: ~100 bytes per symbol in cache
- CLI startup: <500ms for all operations

## Adding New Features

### Language Support

See [Adding Language Support](./development/language-support.md) for the complete guide. Critical checklist:

- [ ] Language enum and methods
- [ ] Parser implementation
- [ ] Factory registration
- [ ] **File walker registration** (often missed!)
- [ ] CLI benchmark support
- [ ] Configuration updates

### New Commands

1. Add command to CLI enum in `src/main.rs`
2. Implement handler function
3. Add tests
4. Update README.md with usage examples
5. Support `--json` flag for structured output

## Submitting Pull Requests

### Before Submitting

1. **Run all local checks:**
   ```bash
   ./contributing/scripts/auto-fix.sh
   ./contributing/scripts/full-test.sh
   ```

2. **Update documentation:**
   - Add/update relevant documentation
   - Include usage examples
   - Update README.md if adding features

3. **Write good commit messages:**
   ```
   feat: Add Go language parser support
   
   - Implement LanguageParser trait for Go
   - Add tree-sitter-python grammar
   - Support classes, functions, and imports
   - Parse >75,000 symbols/second
   ```

### PR Guidelines

1. **Start with an issue**:
   - Create an issue describing your proposed change
   - Wait for feedback before implementing major features
   - Link your PR to the issue with "Fixes #123"
2. **Scope your PRs appropriately**:
   - One feature/major change per PR
   - Multiple small related fixes can be combined
   - Keep PRs focused and reviewable
3. **Include tests** - All new code needs tests
4. **Performance impact** - Document any performance changes
5. **Breaking changes** - Clearly mark if API changes
6. **Screenshots/examples** - Show the feature in action

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Tests pass locally
- [ ] Added new tests
- [ ] Performance verified

## Checklist
- [ ] Code follows guidelines
- [ ] Self-reviewed
- [ ] Documentation updated
- [ ] No new warnings
```

## Getting Help

- **Issues**: Check existing [issues](https://github.com/bartolli/codanna/issues) or create new ones
- **Discussions**: Use GitHub Discussions for questions
- **Documentation**: See `/docs` folder for detailed documentation
- **Before implementing**: Create an issue first to discuss your proposed changes

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive feedback
- Assume good intentions

## Recognition

Contributors are recognized in:
- Release notes
- CONTRIBUTORS.md file
- GitHub contributors page

We're in an era where AI agents are getting smarter and need scalable, fast, and precise context on demand. Context integration matters.

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.