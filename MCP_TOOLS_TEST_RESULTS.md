# MCP Tools Test Results

**Date:** 2025-10-01
**Context:** Testing with C# codebase (Codere.Sci project, 58 files, 280 symbols)

---

## Summary: MCP Tools Status

| Tool | Status | Works Correctly? | Notes |
|------|--------|------------------|-------|
| `find_symbol` | ⚠️ PARTIAL | Partial | Returns correct signature but wrong file path |
| `search_symbols` | ❌ BROKEN | No | Returns "No results found" even for existing symbols |
| `semantic_search_docs` | ✅ WORKS | Yes | Returns semantically similar symbols (low scores though) |
| `semantic_search_with_context` | ⚠️ PARTIAL | Partial | Works but returns low relevance results |
| `get_calls` | ❌ BROKEN | No | Reports no calls even when they exist |
| `find_callers` | ❌ BROKEN | No | Reports no callers even when they exist |
| `analyze_impact` | ❌ BROKEN | No | Reports no impact even when relationships exist |
| `get_index_info` | ✅ WORKS | Yes | Accurate statistics |

**Summary:** 2 out of 8 tools work correctly, 2 are partially functional, 4 are broken.

---

## Detailed Test Results

### 1. find_symbol ✅⚠️ PARTIAL

**Command:**
```bash
codanna mcp find_symbol ArchiveAppService
```

**Result:** ⚠️ Partially works
- ✅ Returns correct class signature
- ✅ Shows full method implementations
- ✅ Includes documentation comments
- ❌ Shows wrong file path (points to Periodicities.cs instead of ArchiveAppService.cs)
- ❌ Returns 2 results when should return 1

**Sample Output:**
```
Found 2 symbol(s) named 'ArchiveAppService':

Constant Recurrent at .\Codere.Sci.Application.Dtos\Application\Enums\Periodicities.cs:22
Module: Codere.Sci.Application.Services.Application.Services.ArchiveAppService
Signature: public class ArchiveAppService : BaseAppService, IArchiveAppService
    {
        private readonly IArchiveDomService _archiveService;
        ...
    }
```

**Assessment:** Useful for getting symbol signatures and documentation, but file paths are unreliable.

---

### 2. search_symbols ❌ BROKEN

**Command:**
```bash
codanna mcp search_symbols query:Archive limit:3
```

**Result:** ❌ Completely broken
- Returns: `No results found for query: Archive`
- Symbol `ArchiveAppService` exists and can be found with `find_symbol`

**Assessment:** Full-text search is non-functional. Cannot discover symbols without knowing exact names.

---

### 3. semantic_search_docs ✅ WORKS

**Command:**
```bash
codanna mcp semantic_search_docs query:"archive compression" limit:3
```

**Result:** ✅ Works correctly
- Returns 3 semantically related symbols
- Includes similarity scores (low: 0.077)
- Shows full signatures and documentation
- Results are not closely related to "archive compression" but search is functional

**Sample Output:**
```
Found 3 semantically similar result(s) for 'archive compression':

1. DataRetrieveLauncher (Class) - Similarity: 0.077
   File: .\Codere.Sci.Launchers\Launchers\DataRetrieveLauncher.cs:12
   ...
```

**Assessment:** Functional but similarity scores are low. May need better embeddings or larger corpus.

---

### 4. semantic_search_with_context ⚠️ PARTIAL

**Command:**
```bash
codanna mcp semantic_search_with_context query:"archive service" limit:2
```

**Result:** ⚠️ Partially works
- ✅ Returns results with call relationships
- ✅ Shows what the symbol calls
- ✅ Shows what calls the symbol
- ❌ Results not relevant to query (returns "Build" and "Encode" for "archive service")
- ❌ Very low similarity scores (0.032)

**Sample Output:**
```
Found 2 results for query: 'archive service'

1. Build - Method at .\Codere.Sci.Application.Contracts\Application\Services\IBuildAppService.cs:24
   Similarity Score: 0.032

   Build calls 2 function(s):
     -> Method (_archiveService.Build)
```

**Assessment:** Context feature works, but relevance scoring needs improvement.

---

### 5. get_calls ❌ BROKEN

**Command:**
```bash
codanna mcp get_calls UnZip
```

**Result:** ❌ Completely broken
- Returns: `UnZip doesn't call any functions (checked 4 symbol(s) with this name)`
- UnZip method clearly calls `_archiveService.UnZip()` (visible in find_symbol output)

**Assessment:** Call relationship extraction or lookup is broken.

---

### 6. find_callers ❌ BROKEN

**Command:**
```bash
codanna mcp find_callers UnZip
```

**Result:** ❌ Completely broken
- Returns: `No functions call UnZip (checked 4 symbol(s) with this name)`
- UnZip is called by launcher classes (should show callers)

**Assessment:** Caller relationship lookup is broken.

---

### 7. analyze_impact ❌ BROKEN

**Command:**
```bash
codanna mcp analyze_impact UnZip max_depth:2
```

**Result:** ❌ Completely broken
- Returns: `No symbols would be impacted by changing UnZip`
- Should show all callers and transitive dependencies

**Assessment:** Impact analysis depends on relationship data which appears to be missing or inaccessible.

---

### 8. get_index_info ✅ WORKS

**Command:**
```bash
codanna mcp get_index_info
```

**Result:** ✅ Works perfectly
- Shows accurate symbol count (280)
- Shows file count (58)
- Shows relationship count (7)
- Shows semantic search status and model info
- Shows breakdown by symbol kind

**Sample Output:**
```
Index contains 280 symbols across 58 files.

Breakdown:
  - Symbols: 280
  - Relationships: 7

Symbol Kinds:
  - Functions: 0
  - Methods: 84
  - Structs: 0
  - Traits: 0

Semantic Search:
  - Status: Enabled
  - Model: AllMiniLML6V2
  - Embeddings: 69
  - Dimensions: 384
```

**Assessment:** Fully functional and useful for understanding index state.

---

## Root Cause Analysis

### Why Are Most Tools Broken?

**Problem 1: Missing Relationships**
- Only 7 relationships stored for 280 symbols (2.5% relationship coverage)
- 237 unresolved cross-file relationships
- This explains why `get_calls`, `find_callers`, and `analyze_impact` don't work

**Problem 2: Search Index Issues**
- `search_symbols` returns no results for any query
- Tantivy full-text search may not be indexing symbol names correctly
- Search index might not be built or populated

**Problem 3: Low Semantic Similarity Scores**
- Semantic search returns scores of 0.032-0.077 (very low)
- May indicate:
  - Small embedding corpus (only 69 embeddings for 280 symbols = 25% coverage)
  - Model not well-tuned for code
  - Documentation may be insufficient for semantic matching

**Problem 4: File Path Mapping**
- `find_symbol` shows wrong file paths
- Related to the file ID bug that was fixed, but output still shows old cached data

---

## Comparison: MCP Tools vs retrieve Commands

| Feature | MCP Tool | retrieve Command | Winner |
|---------|----------|------------------|--------|
| Symbol lookup | `find_symbol` (⚠️) | `retrieve describe` (✅) | retrieve |
| Full-text search | `search_symbols` (❌) | `retrieve search` (❌) | TIE (both broken) |
| Semantic search | `semantic_search_docs` (✅) | N/A | MCP |
| Call graph | `get_calls` (❌) | `retrieve calls` (❌) | TIE (both broken) |
| Caller lookup | `find_callers` (❌) | `retrieve callers` (❌) | TIE (both broken) |
| Impact analysis | `analyze_impact` (❌) | `retrieve dependencies` (❌) | TIE (both broken) |
| Index stats | `get_index_info` (✅) | N/A | MCP |

**Key Finding:** `retrieve describe` is more reliable than `mcp find_symbol` for basic symbol lookup.

---

## Recommendations

### For Users (Short Term)

**What to Use:**
1. ✅ `retrieve describe <name>` - Most reliable for symbol lookup
2. ✅ `mcp get_index_info` - Check index statistics
3. ⚠️ `mcp semantic_search_docs` - Discovery (expect low relevance)
4. ⚠️ `mcp find_symbol <name>` - Get signatures (ignore file paths)

**What to Avoid:**
- ❌ All relationship/call graph tools (broken)
- ❌ All full-text search tools (broken)

### For Developers (Long Term)

**Priority 1: Fix Relationship Extraction/Storage**
- Current: Only 7 relationships for 280 symbols
- Target: Should have 100+ relationships
- Affects: `get_calls`, `find_callers`, `analyze_impact`

**Priority 2: Fix Full-Text Search**
- `search_symbols` returns no results
- Investigate Tantivy indexing pipeline
- Verify symbol names are being indexed

**Priority 3: Improve Semantic Search**
- Only 69/280 symbols have embeddings (25%)
- Low similarity scores (0.032-0.077)
- Consider better embedding model or fine-tuning

**Priority 4: Fix File Path Mapping**
- `find_symbol` shows wrong file paths
- May be caching issue after file ID bug fix

---

## Test Environment

- **OS:** Windows 11
- **Project:** Codere.Sci C# application
- **Files:** 58 .cs files
- **Symbols:** 280 indexed
- **Relationships:** 7 stored (2.5% coverage)
- **Embeddings:** 69 generated (25% coverage)
- **Codanna Version:** 0.5.16
- **Build:** Release mode

---

## Conclusion

**MCP tools are mostly broken for C# projects.** Only 2 out of 8 tools work correctly:
- ✅ `get_index_info` - Fully functional
- ✅ `semantic_search_docs` - Functional but low relevance

The root cause appears to be:
1. Relationship extraction/storage is incomplete (2.5% coverage)
2. Full-text search index is not populated
3. Semantic embeddings are only generated for 25% of symbols

**Current State:** MCP interface is not production-ready for C# codebases. Use `retrieve describe` for basic symbol lookup until these issues are resolved.
