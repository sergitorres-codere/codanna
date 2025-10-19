# Corporate Network Troubleshooting (Windows)

This guide helps Windows users resolve issues when using Codanna behind corporate proxies, firewalls, or network filtering.

## Table of Contents
- [Model Download Issues](#model-download-issues)
- [Solutions](#solutions)
- [Manual Model Download](#manual-model-download)
- [Technical Background](#technical-background)

---

## Model Download Issues

### Symptoms

When trying to change to a different embedding model that isn't already cached, you see:

**During indexing:**
```
Warning: Failed to enable semantic search: Failed to initialize semantic search with model 'MultilingualE5Small':
Failed to initialize embedding model: Failed to initialize model 'MultilingualE5Small':
Failed to retrieve onnx/model.onnx
```

**When running the download script:**
```
✗ Failed to download config.json : Invalid username or password.
```

**Note:** The "Invalid username or password" error is misleading - it's actually your corporate network/proxy blocking HuggingFace, not an authentication issue with the script.

### Cause
Codanna downloads embedding models from HuggingFace on first use. Corporate networks may:
- Block HuggingFace URLs (`huggingface.co`)
- Use transparent proxies that interfere with downloads
- Filter HTTPS traffic in ways that break model downloads

The underlying HTTP client (`ureq` via `fastembed` → `hf-hub`) uses a blocking synchronous approach that may not properly handle corporate proxy configurations.

---

## Solution: Manual Model Download

Run the PowerShell download script from a non-restricted network (home, mobile hotspot, etc.):

```powershell
cd C:\Projects\codanna
.\scripts\download-model.ps1 -Model MultilingualE5Small
```

The script downloads the model to `%USERPROFILE%\.codanna\models\` where Codanna will find it.

### Update Configuration

```toml
# .codanna/settings.toml
[semantic_search]
model = "MultilingualE5Small"
```

### Test

```bash
codanna index . --force --progress
```

---

## Manual Model Download

### Available Models

| Model | Languages | Dimensions | Repository |
|-------|-----------|------------|------------|
| **AllMiniLML6V2** | English | 384 | `Qdrant/all-MiniLM-L6-v2-onnx` |
| **MultilingualE5Small** | 94 langs | 384 | `intfloat/multilingual-e5-small` |
| **MultilingualE5Base** | 94 langs | 768 | `intfloat/multilingual-e5-base` |
| **ParaphraseMLMiniLML12V2** | Multilingual | 384 | `Xenova/paraphrase-multilingual-MiniLM-L12-v2` |

**Note:** MultilingualE5Large uses split ONNX format (model.onnx + model.onnx_data) which is not currently supported by fastembed.

### PowerShell Script Usage

```powershell
# Download MultilingualE5Small (recommended for multilingual)
.\scripts\download-model.ps1 -Model MultilingualE5Small

# Download MultilingualE5Base (better quality, larger)
.\scripts\download-model.ps1 -Model MultilingualE5Base

# Download to custom location
.\scripts\download-model.ps1 -Model MultilingualE5Small -CacheDir "D:\models"

# List available models
Get-Help .\scripts\download-model.ps1 -Detailed
```

---

## Technical Background

### Why This Happens

Codanna uses the **fastembed** Rust library for embedding generation, which in turn uses:
- `hf-hub` crate for downloading models from HuggingFace
- `ureq` HTTP client (blocking/synchronous)

The `ureq` client may not properly handle:
- Corporate MITM (Man-In-The-Middle) SSL certificates
- Transparent HTTP/HTTPS proxies
- Deep packet inspection firewalls
- DNS-level filtering

### Why We Can't Easily Fix It

We attempted to switch from `ureq` to `reqwest` (async HTTP client with better proxy support), but:

1. **Transitive Dependency Control**: Cargo doesn't allow controlling features of transitive dependencies (fastembed → hf-hub → ureq)

2. **Feature Unification**: When multiple crates depend on the same library with different features, Cargo unifies them (includes ALL features)

3. **Fastembed's Choices**: The `fastembed` crate controls which features of `hf-hub` are enabled, and we can't override that without forking

### Potential Long-Term Solutions

1. **Submit PR to fastembed-rs**: Add a feature flag to choose between `hf-hub/ureq` and `hf-hub/tokio`
   - Repository: https://github.com/Anush008/fastembed-rs

2. **Fork fastembed**: Create a corporate-friendly fork with configurable HTTP backend

3. **Pre-bundle Models**: Distribute Codanna with common models included (increases binary size significantly)

---

## Verifying Setup

After downloading a model, verify it works:

```powershell
# Check cache directory
dir "$env:USERPROFILE\.codanna\models"

# Try indexing with the model
codanna index . --force --progress

# Should see:
# ✓ Semantic search enabled (model: MultilingualE5Small, threshold: 0.6)
```

---

## Related Documentation

- [Configuration Guide](../user-guide/configuration.md) - Embedding model settings
- [Embedding Models](../architecture/embedding-model.md) - Technical details
- [Installation Guide](../getting-started/installation.md) - Initial setup

---

## Getting Help

If you continue having issues:

1. Check the model files exist in the cache directory
2. Verify file permissions (models should be readable)
3. Try with the default AllMiniLML6V2 model first
4. Report issues at: https://github.com/bartolli/codanna/issues

Include:
- Operating system
- Network environment (corporate/home)
- Full error message
- Output of `dir "$env:USERPROFILE\.codanna\models"`
