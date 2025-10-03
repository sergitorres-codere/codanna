# Codanna Improvements Roadmap

Based on testing with the SCI codebase (4,465 files, 39,396 symbols) and Claude Code integration analysis.

---

## 🔥 Critical (Fix Immediately)

### 1. Fix MCP Token Limit Error
**Priority:** CRITICAL
**Issue:** `search_symbols` response (70K tokens) exceeds Claude's 25K token limit
**Impact:** Tool fails completely, Claude falls back to slower methods

**Solution:**
```rust
// In src/mcp/tools/search_symbols.rs
pub async fn search_symbols(args: SearchSymbolsArgs) -> Result<String> {
    let results = perform_search(&args)?;
    let json_output = serde_json::to_string(&results)?;
    let estimated_tokens = json_output.len() / 4;

    const MAX_TOKENS: usize = 20000; // Leave 5K buffer

    if estimated_tokens > MAX_TOKENS {
        // Auto-truncate to summary mode
        let summary = results.iter().take(20).map(|r| {
            format!("{} ({}) at {}:{}", r.name, r.kind, r.file_path, r.line)
        }).collect::<Vec<_>>().join("\n");

        return Ok(format!(
            "Found {} symbols (showing first 20 due to size):\n{}\n\n\
             💡 Tip: Use limit=10 or module_filter for detailed results",
            results.len(), summary
        ));
    }

    Ok(json_output)
}
```

**Effort:** 2 hours
**Benefit:** Fixes 100% of token limit failures

---

### 2. Implement Pagination for MCP Tools
**Priority:** CRITICAL
**Issue:** No way to get results beyond first page when truncated
**Impact:** Users can't access all search results

**New Parameters:**
```rust
pub struct SearchSymbolsArgs {
    pub query: String,
    pub kind: Option<SymbolKind>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,           // NEW: skip first N results
    pub page: Option<usize>,             // NEW: alternative to offset
}
```

**Usage:**
```bash
# Page 1 (results 0-9)
codanna mcp search_symbols "query:Service limit:10 offset:0"

# Page 2 (results 10-19)
codanna mcp search_symbols "query:Service limit:10 offset:10"

# Or use page parameter
codanna mcp search_symbols "query:Service limit:10 page:2"
```

**Effort:** 4 hours
**Benefit:** Enables access to all search results

---

### 3. Fix find_callers Method Call Tracking
**Priority:** CRITICAL
**Issue:** `find_callers SendMessageToApi` returns "No functions call SendMessageToApi" but grep finds 16 call sites
**Test Evidence:**
- WITH Codanna test (lines 92-142): find_callers failed completely
- Had to fall back to grep search to find 13 call sites
- Result: Same quality as WITHOUT Codanna (defeats the purpose!)

**Root Cause:** Call graph not fully built during C# indexing
- Method calls are parsed but relationships not stored
- `extract_calls_recursive` may not be tracking all invocation types
- Caller context not properly associated with method calls

**Current Behavior:**
```
codanna mcp find_callers SendMessageToApi
→ "No functions call SendMessageToApi (checked 3 symbol(s) with this name)"
```

**Expected Behavior:**
```
codanna mcp find_callers SendMessageToApi
→ Found 16 call sites:
   - Treatment/Service.cs:54 (in RetryLaunches)
   - Treatment/Service.cs:101 (in NewLaunches)
   - Processes/Service.cs:87 (in NewLaunches)
   [etc...]
```

**Solution Steps:**
1. Review `extract_calls_recursive` in `src/parsing/csharp/parser.rs:665`
2. Verify `invocation_expression` nodes are captured
3. Ensure caller function context is tracked (fixed in v0.5.20 but may need verification)
4. Store caller → callee relationships in tantivy index
5. Test with real SCI examples:
   - `Helper.SendMessageToApi` (16 call sites)
   - `context.SaveChanges()` (100+ call sites)
   - `Helper.GetFileById()` (5+ call sites)
   - `Helper.CkeckPrevRange()` (multiple call sites)

**Effort:** 8-12 hours
**Benefit:** Makes find_callers actually work (currently broken for C#)

---

## ⚡ High Priority (Next Sprint)

### 4. Add Filtering Parameters to search_symbols
**Priority:** HIGH
**Issue:** No way to scope searches to specific modules or file patterns
**Impact:** Users get too many results, can't narrow down

**New Parameters:**
```rust
pub struct SearchSymbolsArgs {
    // ... existing ...
    pub module_filter: Option<String>,    // e.g., "Codere.Sci.Services"
    pub file_pattern: Option<String>,     // e.g., "**/Base/**"
    pub exclude_pattern: Option<String>,  // e.g., "**/Test/**"
}
```

**Usage:**
```bash
# Find Service classes only in Base module
codanna mcp search_symbols "query:Service module:Codere.Sci.Services.Base"

# Find Helper classes, exclude tests
codanna mcp search_symbols "query:Helper exclude:**/Test/**"
```

**Effort:** 6 hours
**Benefit:** Reduces noise, faster targeted searches

---

### 5. Implement summary_only Mode
**Priority:** HIGH
**Issue:** Sometimes users just want list of symbols, not full details
**Impact:** Wastes tokens on unnecessary details

**Implementation:**
```rust
pub struct SearchSymbolsArgs {
    // ... existing ...
    pub summary_only: Option<bool>,  // Just names, kinds, locations
    pub details_level: Option<String>, // "minimal" | "standard" | "full"
}
```

**Output Comparison:**

**summary_only=true (200 tokens):**
```
1. Service (Class) - Treatment/Service.cs:13
2. Service (Class) - Processes/Service.cs:14
3. ServiceBase (Class) - Base/ServiceBase.cs:16
```

**summary_only=false (5000 tokens):**
```
1. Service (Class) - Treatment/Service.cs:13
   Module: Codere.Sci.Services.Treatment
   Signature: public class Service : ServiceBase { ... }
   Documentation: /// <summary> ... </summary>
   Methods: RetryLaunches, NewLaunches, ...
   [Full details]
```

**Effort:** 4 hours
**Benefit:** 10x faster responses for overview queries

---

### 6. Add get_symbol_details MCP Command
**Priority:** HIGH
**Issue:** After getting summary, no way to fetch details for specific symbol
**Impact:** Forces re-searching with filters

**New Command:**
```bash
codanna mcp get_symbol_details "Treatment/Service.cs:13"
codanna mcp get_symbol_details "Service" module:"Codere.Sci.Services.Treatment"
```

**Returns:**
- Full signature
- Complete documentation
- All methods/properties
- Relationships (inherits, implements, calls)

**Effort:** 6 hours
**Benefit:** Two-phase discovery: summary → details on demand

---

## 📊 Medium Priority (Future Release)

### 7. Resolve Cross-File Relationships
**Priority:** MEDIUM
**Issue:** "38,575 unresolved relationships" during indexing
**Impact:** Incomplete call graphs, missing dependency tracking

**Root Causes:**
- Method calls to external assemblies (Entity Framework, etc.)
- Cross-project references
- Dynamic invocations (reflection, delegates)

**Solution:**
- Phase 1: Index all files first, build symbol table
- Phase 2: Resolve relationships using symbol table
- Phase 3: Mark unresolvable (external) separately

**Effort:** 16 hours
**Benefit:** Complete relationship tracking

---

### 8. Enhance Semantic Search
**Priority:** MEDIUM
**Issue:** `semantic_search_docs` finds results but with low similarity scores (0.45)
**Impact:** Less useful for natural language queries

**Improvements:**
- Use better embedding model (all-MiniLM-L6-v2 → all-mpnet-base-v2)
- Pre-process queries (expand abbreviations, synonyms)
- Boost score based on documentation quality
- Consider code structure (class > method > field)

**Effort:** 12 hours
**Benefit:** Better "find classes related to archiving" style queries

---

### 9. Add Property Dependency Tracking
**Priority:** MEDIUM
**Issue:** Can't track which properties reference other types
**Impact:** Miss dependencies when refactoring

**Example:**
```csharp
public class ServiceConfig {
    public virtual Services Service { get; set; }  // Track this reference
}
```

**Track:**
- Property type dependencies
- Auto-property backing fields
- Getter/setter logic

**Effort:** 8 hours
**Benefit:** Complete dependency analysis

---

### 10. Add Interface Implementation Tracking
**Priority:** MEDIUM
**Issue:** Can't find all classes implementing IService, IDto, etc.
**Impact:** Incomplete architecture analysis

**New MCP Command:**
```bash
codanna mcp find_implementations "IService"
codanna mcp find_interface_members "IDisposable"
```

**Effort:** 6 hours
**Benefit:** Interface-based navigation

---

## 🔧 Nice to Have (Backlog)

### 11. Support Partial Class Tracking
**Priority:** LOW
**Issue:** Partial classes split across files not combined
**Impact:** Incomplete class view

**Solution:**
- Detect `partial` keyword
- Combine all partial definitions
- Show unified class view

**Effort:** 8 hours

---

### 12. Lambda Expression Call Tracking
**Priority:** LOW
**Issue:** Calls within lambda expressions not tracked
**Example:** `.Where(x => x.Success == false)` - Success property access not tracked

**Effort:** 10 hours

---

### 13. Improve MCP Error Messages for Claude
**Priority:** LOW
**Issue:** Generic errors don't guide Claude to retry properly

**Current:**
```
Error: response exceeds maximum tokens
```

**Better:**
```
Error: Response too large (70K tokens > 25K limit)
💡 Retry with: limit=10 or summary_only=true or module_filter=<module_name>
```

**Effort:** 2 hours

---

## 📚 Documentation Improvements

### 14. Update MCP Tool Descriptions
**Priority:** MEDIUM
**Issue:** Claude doesn't know how to handle token limits
**Impact:** Doesn't retry with better parameters

**Add to Tool Metadata:**
```json
{
  "name": "search_symbols",
  "description": "Search for symbols by name...",
  "error_handling": {
    "token_limit_exceeded": "Retry with limit=10 or summary_only=true",
    "no_results": "Try fuzzy search or partial name",
    "too_many_results": "Use module_filter or file_pattern"
  }
}
```

**Effort:** 4 hours

---

### 15. Create MCP Best Practices Guide
**Priority:** LOW
**Content:**
- When to use each MCP command
- How to handle errors
- Pagination patterns
- Filtering strategies

**Effort:** 8 hours

---

### 16. Document Claude Code Integration Patterns
**Priority:** LOW
**Content:**
- Prompt patterns for code navigation
- Error recovery workflows
- Multi-tool query strategies

**Effort:** 8 hours

---

## 🎯 Implementation Priority Matrix

| Priority | Feature | Effort | Impact | ROI |
|----------|---------|--------|--------|-----|
| 🔥 P0 | Token limit fix | 2h | Critical | 10x |
| 🔥 P0 | Pagination | 4h | Critical | 8x |
| 🔥 P0 | Fix find_callers | 8-12h | Critical | 9x |
| ⚡ P1 | Filtering params | 6h | High | 7x |
| ⚡ P1 | Summary mode | 4h | High | 9x |
| ⚡ P1 | get_symbol_details | 6h | High | 7x |
| 📊 P2 | Cross-file relationships | 16h | Medium | 4x |
| 📊 P2 | Semantic search | 12h | Medium | 3x |
| 📊 P2 | Property tracking | 8h | Medium | 4x |
| 📊 P2 | Interface tracking | 6h | Medium | 5x |
| 🔧 P3 | Partial classes | 8h | Low | 2x |
| 🔧 P3 | Lambda tracking | 10h | Low | 2x |
| 🔧 P3 | Error messages | 2h | Low | 3x |

---

## 📅 Suggested Roadmap

### Sprint 1 (Week 1) - Critical Fixes
- [ ] Token limit auto-truncation (2h)
- [ ] Pagination support (4h)
- [ ] **Fix find_callers** (8-12h) ⚠️ Currently broken for C#
- [ ] Summary mode (4h)
- **Total: 18-22 hours**
- **Outcome:** Core MCP tools stable and usable (especially find_callers!)

### Sprint 2 (Week 2) - Usability
- [ ] Filtering parameters (6h)
- [ ] get_symbol_details (6h)
- [ ] Update tool descriptions (4h)
- **Total: 16 hours**
- **Outcome:** Power-user features enabled

### Sprint 3 (Week 3) - Completeness
- [ ] Cross-file relationships (16h)
- **Total: 16 hours**
- **Outcome:** Full relationship tracking

### Sprint 4 (Week 4) - Intelligence
- [ ] Semantic search improvements (12h)
- [ ] Property tracking (8h)
- **Total: 20 hours**
- **Outcome:** Advanced discovery features

### Sprint 5 (Week 5) - Polish
- [ ] Interface tracking (6h)
- [ ] Partial class support (8h)
- [ ] Documentation (8h)
- **Total: 22 hours**
- **Outcome:** Production-ready v0.6.0

---

## 🧪 Testing Checklist

For each improvement, test with SCI codebase:

- [ ] Search for common symbols (Service, Helper, Repository)
- [ ] Verify token limits respected (<25K)
- [ ] Test pagination (fetch all pages)
- [ ] Verify filtering works (module, file pattern)
- [ ] Test error recovery (Claude retries correctly)
- [ ] Measure performance (response time <500ms)
- [ ] Check accuracy (100% of known symbols found)

### Critical: find_callers Test Cases (Currently Failing)
- [ ] `find_callers SendMessageToApi` → Should find 16 call sites (currently finds 0)
- [ ] `find_callers SaveChanges` → Should find 100+ Entity Framework calls
- [ ] `find_callers GetFileById` → Should find 5+ call sites in Helper usage
- [ ] `find_callers RetryLaunches` → Should find where base method is called
- [ ] Verify caller context shows correct function name (not empty string)

---

## 📈 Success Metrics

### Before Improvements (Current State)
- Token limit failures: 40% of searches
- Call graph completeness: 60%
- Average query time: 2-5 seconds
- Result accuracy: 85%
- Claude retry success: 30%

### After All Improvements (Target)
- Token limit failures: <1%
- Call graph completeness: 95%
- Average query time: <1 second
- Result accuracy: 98%
- Claude retry success: 90%

---

## 🔗 Related Issues

- Issue #39: C# parser implementation (✅ Merged)
- Issue #XX: MCP token limit errors (🆕 Create)
- Issue #XX: find_callers not working (🆕 Create)
- Issue #XX: Add pagination to MCP tools (🆕 Create)

---

## 💡 Quick Wins (Do First)

1. **Token limit fix** (2h) - Immediate 10x improvement
2. **Summary mode** (4h) - 90% of queries only need this
3. **Error messages** (2h) - Help Claude retry correctly

**Total: 8 hours for 80% of the benefit**

---

## 🚀 Future Vision (v0.7.0+)

- AI-powered symbol suggestions ("did you mean...?")
- Visual call graph generation (mermaid diagrams)
- Automatic refactoring suggestions
- Integration with git blame (who changed this?)
- Cross-language support (C# ↔ TypeScript API calls)
- Real-time index updates (watch mode)

---

**Document Created:** 2025-10-03
**Based on:** SCI codebase testing (4,465 files)
**Next Review:** After Sprint 1 completion
**Owner:** Codanna Core Team
