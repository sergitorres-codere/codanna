# Embedding Model

How Codanna generates and uses semantic embeddings for code search.

## Supported Models

| Model | Dimensions | Languages | Use Case |
|-------|------------|-----------|----------|
| `AllMiniLML6V2` | 384 | English | Default, fast, English codebases |
| `MultilingualE5Small` | 384 | 94 | Multilingual, same performance |
| `MultilingualE5Base` | 768 | 94 | Better quality, slower |
| `MultilingualE5Large` | 1024 | 94 | Best quality, slowest |

## Model Selection

Configure in `.codanna/settings.toml`:

```toml
[semantic]
model = "AllMiniLML6V2"  # Default
# model = "MultilingualE5Small"  # For multilingual teams
```

**Note:** Changing models requires re-indexing:
```bash
codanna index . --force --progress
```

## Embedding Generation

### Input: Documentation Comments

```rust
/// Parse configuration from a TOML file and validate required fields
/// This handles missing files gracefully and provides helpful error messages
fn load_config(path: &Path) -> Result<Config, Error>
```

### Process

1. **Extract**: Doc comment text
2. **Tokenize**: Break into tokens
3. **Embed**: fastembed model generates vector
4. **Normalize**: L2 normalization for cosine similarity
5. **Store**: Memory-mapped vector cache

### Output: Dense Vector

```
[0.123, -0.456, 0.789, ..., 0.321]  // 384/768/1024 floats
```

## Semantic Understanding

Embeddings capture:
- **Conceptual meaning** - Not just keywords
- **Context** - Related terms clustered together
- **Intent** - "error handling" matches "graceful failure recovery"

### Example

Query: "authentication logic"

Matches:
- "user authentication and session management"
- "verify credentials and create tokens"
- "login flow with password hashing"

Doesn't match:
- "configuration parser" (different concept)
- "file system operations" (unrelated)

## Similarity Computation

Uses cosine similarity for comparing vectors:

```
similarity = dot(v1, v2) / (||v1|| × ||v2||)
```

Scores range from 0 to 1:
- **0.7+** - Very similar
- **0.5-0.7** - Related
- **0.3-0.5** - Somewhat related
- **<0.3** - Different concepts

## Language-Aware Embeddings

Each embedding tracks its source language:

```rust
struct EmbeddedSymbol {
    symbol_id: SymbolId,
    vector: Vec<f32>,
    language: LanguageId,  // rust, python, typescript, etc.
}
```

### Language Filtering

Filtering happens **before** similarity computation:

```bash
# Only search Rust code
codanna mcp semantic_search_docs query:"error handling" lang:rust
```

**Performance benefit**: Reduces search space by up to 75% in mixed codebases.

**Accuracy**: Identical documentation in different languages produces identical scores.

## IVFFlat Index

Vectors are organized using Inverted File with Flat vectors for fast search:

### K-means Clustering

1. **Cluster** similar vectors together
2. **Centroids** represent each cluster
3. **Search** checks nearby clusters first

### Search Algorithm

```
1. Query vector → find closest centroid
2. Search vectors in that cluster
3. Optionally search nearby clusters
4. Return top-k results
```

**Speed improvement**: O(sqrt(N)) instead of O(N) comparisons.

## Model Characteristics

### AllMiniLML6V2
- **Size**: ~25MB
- **Speed**: Fast inference
- **Quality**: Good for English
- **Use**: Default choice

### MultilingualE5Small
- **Size**: ~118MB
- **Speed**: Similar to AllMiniLM
- **Quality**: 94 languages
- **Use**: Multilingual teams

### MultilingualE5Base
- **Size**: ~278MB
- **Speed**: Slower inference
- **Quality**: Better accuracy
- **Use**: Quality-critical applications

### MultilingualE5Large
- **Size**: ~560MB
- **Speed**: Slowest
- **Quality**: Best accuracy
- **Use**: Maximum quality needs

## Performance Characteristics

### First Use
- Model download: One-time (~25-560MB depending on model)
- Storage: `~/.cache/fastembed/`
- Subsequent runs: Instant model load

### Embedding Generation
- Per symbol: ~10ms
- Batch of 100: ~100ms
- Parallel: Scales with CPU cores

### Search
- With IVFFlat: <10ms for 100k vectors
- Without clustering: Would be ~1s

## Optimization

### Batch Processing
Generate embeddings in batches during indexing:
- More efficient GPU/CPU usage
- Amortizes model initialization
- Better throughput

### Caching
- Embeddings persist in memory-mapped files
- No re-generation unless code changes
- Symbol-level change detection

### Incremental Updates
Only re-embed changed symbols:
```rust
if symbol.doc_comment != old_symbol.doc_comment {
    regenerate_embedding(symbol);
}
```

## Troubleshooting

### Poor Search Results
1. Check documentation quality
2. Try different model (multilingual if needed)
3. Adjust threshold parameter
4. Use language filtering

### Slow Embedding Generation
1. First run downloads model (one-time)
2. Large codebases take time initially
3. Incremental updates are fast
4. Use `--threads` to parallelize

### Model Not Found
- Check internet connection (first use)
- Verify `~/.cache/fastembed/` permissions
- Re-download with `rm -rf ~/.cache/fastembed/`

## Storage Requirements

For 100,000 symbols:

**AllMiniLML6V2 (384-dim):**
- 100k × 384 floats × 4 bytes = 153.6 MB

**MultilingualE5Base (768-dim):**
- 100k × 768 floats × 4 bytes = 307.2 MB

**MultilingualE5Large (1024-dim):**
- 100k × 1024 floats × 4 bytes = 409.6 MB

## See Also

- [How It Works](how-it-works.md) - System overview
- [Memory Mapping](memory-mapping.md) - Vector storage details
- [Search Guide](../user-guide/search-guide.md) - Writing effective queries
- [Configuration](../user-guide/configuration.md) - Model selection