# Language Support

Parser architecture and supported languages.

## Supported Languages

| Language | Parser | Status |
|----------|--------|--------|
| Rust | tree-sitter-rust | Production |
| Python | tree-sitter-python | Production |
| TypeScript | tree-sitter-typescript | Production |
| Go | tree-sitter-go | Production |
| PHP | tree-sitter-php | Production |
| C | tree-sitter-c | Production |
| C++ | tree-sitter-cpp | Production |

## Parser Technology

Codanna uses tree-sitter for AST parsing - the same technology used by GitHub's code navigator.

### Why tree-sitter?

- Language-agnostic
- Fast incremental parsing
- Error-tolerant
- Battle-tested
- Active ecosystem

## What Gets Extracted

From each supported language:

- Functions and methods
- Classes, structs, traits
- Type definitions
- Imports and includes
- Call relationships
- Type relationships
- Documentation comments
- External type references (for compiled languages like C# and Java)

## Performance

See [Performance Documentation](../advanced/performance.md) for current benchmarks.

## Adding New Languages

For detailed guidance on adding language support, see the contributing documentation in the repository.

## See Also

- [How It Works](how-it-works.md) - Overall architecture
- [Performance](../advanced/performance.md) - Parser benchmarks
- [Contributing](../contributing/) - Development guidelines