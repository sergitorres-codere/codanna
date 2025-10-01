# C# Test Examples for Codanna

This directory contains comprehensive C# test files designed to validate the Codanna parser and demonstrate all features for PR submission.

## Files

### 1. ComprehensiveTest.cs

Tests all major C# language features:

- ✅ **Interfaces** (`IDataProcessor`, `ILogger`, `IValidator`)
- ✅ **Interface Implementation** (`DataProcessorService : IDataProcessor`)
- ✅ **Base Classes** (`BaseService`)
- ✅ **Inheritance** (`ConcreteService : BaseService`)
- ✅ **Properties** (Auto-properties, read-only properties)
- ✅ **Fields** (Private, readonly fields)
- ✅ **Enums** (`ProcessingStatus`)
- ✅ **Constants** (`Constants` class with const fields)
- ✅ **Generic Classes** (`Result<T>`)
- ✅ **Async Methods** (`ProcessAsync`, `ProcessFireAndForgetAsync`)
- ✅ **Nested Classes** (`Container.NestedConfig`)
- ✅ **Extension Methods** (`StringExtensions.Reverse`)
- ✅ **Events and Delegates** (`EventPublisher.DataProcessed`)
- ✅ **Documentation Comments** (XML doc comments on all symbols)

### 2. RelationshipTest.cs

Tests relationship tracking and call graphs:

- ✅ **Simple Call Chain** (`ServiceA -> ServiceB -> ServiceC`)
- ✅ **Multiple Callers** (Many services calling `LoggerService.Log`)
- ✅ **Orchestrator Pattern** (`ServiceOrchestrator` calling 5 services)
- ✅ **Internal Call Chains** (`DataService.FetchData` calling 3 internal methods)
- ✅ **Interface Method Calls** (`RepositoryConsumer` calling `IRepository` methods)
- ✅ **Recursive Calls** (`RecursiveService.Factorial` calling itself)
- ✅ **Mutual Recursion** (`CountDownA` ↔ `CountDownB`)
- ✅ **Static Method Calls** (`Calculator` calling `MathUtils.Add/Multiply`)

## Testing Instructions

### Step 1: Index the Examples

```bash
cd examples/csharp
codanna init
codanna index . --force --progress
```

**Expected Output:**
```
Indexing Complete:
  Files indexed: 2
  Symbols found: ~80-100
  Relationships: Should capture method calls and implementations
```

### Step 2: Test Symbol Lookup (WORKS)

```bash
# Find interfaces
codanna retrieve describe IDataProcessor
codanna retrieve describe ILogger

# Find classes
codanna retrieve describe DataProcessorService
codanna retrieve describe ServiceOrchestrator

# Find methods
codanna retrieve describe ProcessData
codanna retrieve describe Execute

# Find enums
codanna retrieve describe ProcessingStatus

# With JSON output
codanna retrieve describe DataProcessorService --json
```

**Expected:** All commands should return correct symbol information with accurate file paths.

### Step 3: Test MCP Tools

#### get_index_info (WORKS ✅)

```bash
codanna mcp get_index_info
```

**Expected Output:**
```
Index contains ~80-100 symbols across 2 files.
Relationships: ~20-30 (should be much higher once bug is fixed)
Symbol Kinds:
  - Classes: ~20
  - Interfaces: ~5
  - Methods: ~50
  - Properties: ~15
  - Enums: ~1
```

#### find_symbol (PARTIAL ⚠️)

```bash
codanna mcp find_symbol DataProcessorService
codanna mcp find_symbol ServiceOrchestrator
```

**Expected:** Returns correct signatures but may show wrong file paths (Bug #10).

#### semantic_search_docs (WORKS ✅)

```bash
codanna mcp semantic_search_docs query:"data processing" limit:5
codanna mcp semantic_search_docs query:"service orchestration" limit:5
codanna mcp semantic_search_docs query:"validation" limit:3
```

**Expected:** Returns semantically related symbols (though similarity scores may be low).

#### get_calls (BROKEN ❌ - Currently)

```bash
# These SHOULD work but currently return empty due to Bug #9
codanna mcp get_calls ProcessData
codanna mcp get_calls Execute
codanna mcp get_calls FetchData
```

**Expected (once fixed):**
- `ProcessData` CALLS: `ValidateData`, `Transform`, `_logger.LogError`, `_logger.LogInfo`
- `Execute` CALLS: `_logger.Log`, `FetchData`, `Validate`, `Process`, `Notify`
- `FetchData` CALLS: `ConnectToDatabase`, `QueryDatabase`, `CloseConnection`, `_logger.Log`

#### find_callers (BROKEN ❌ - Currently)

```bash
# These SHOULD work but currently return empty due to Bug #9
codanna mcp find_callers Log
codanna mcp find_callers MethodC
codanna mcp find_callers Validate
```

**Expected (once fixed):**
- `Log` CALLED BY: `Execute`, `FetchData`, `Validate` (multiple callers)
- `MethodC` CALLED BY: `MethodB`
- `Validate` CALLED BY: `Execute`

#### analyze_impact (BROKEN ❌ - Currently)

```bash
# These SHOULD work but currently return empty due to Bug #9
codanna mcp analyze_impact LoggerService.Log max_depth:2
codanna mcp analyze_impact DataService.FetchData max_depth:3
```

**Expected (once fixed):**
- `Log`: High impact (called by many services)
- `FetchData`: Medium impact (called by orchestrator, which is called by main)

### Step 4: Verify Relationships in JSON

```bash
codanna retrieve describe DataProcessorService --json | grep -A 20 "relationships"
codanna retrieve describe ServiceOrchestrator --json | grep -A 20 "relationships"
```

**Expected:** Should see `implements` field showing interface relationships.

## Known Issues

Based on testing with `Codere.Sci` project, the following issues are expected:

### Critical Issues

1. **Bug #9: Relationship Tools Return Empty** (Severity: Critical)
   - Only ~2-5% of relationships are captured
   - `get_calls`, `find_callers`, `analyze_impact` return empty
   - Root cause: 237 unresolved cross-file relationships

2. **Bug #8: search_symbols Returns No Results** (Severity: High)
   - Full-text search is non-functional
   - `mcp search_symbols` always returns "No results found"

3. **Bug #10: find_symbol Shows Wrong File Paths** (Severity: Medium)
   - Correct signatures but incorrect file locations
   - Shows wrong file names in output

### Partial Issues

4. **Bug #11: Low Semantic Similarity Scores** (Severity: Low)
   - Semantic search returns 0.03-0.08 similarity scores
   - Only 25% of symbols get embeddings

## Expected vs. Actual Results

### What SHOULD Work (Once Bugs Fixed)

**Call Graph for ServiceOrchestrator.Execute():**
```
ServiceOrchestrator.Execute()
├─→ LoggerService.Log() [called 3 times]
├─→ DataService.FetchData()
│   ├─→ ConnectToDatabase()
│   ├─→ QueryDatabase()
│   └─→ CloseConnection()
├─→ ValidationService.Validate()
│   └─→ LoggerService.Log()
├─→ ProcessingService.Process()
│   ├─→ Transform()
│   ├─→ ApplyRules()
│   └─→ ValidateResult()
└─→ NotificationService.Notify()
```

**Callers of LoggerService.Log():**
- ServiceOrchestrator.Execute() (3 calls)
- DataService.FetchData() (1 call)
- ValidationService.Validate() (1 call)

Total: 5 call sites from 3 different methods

### What Currently Works

1. ✅ Symbol extraction (classes, interfaces, methods, properties, enums)
2. ✅ `retrieve describe` command (accurate file paths and symbol info)
3. ✅ `mcp get_index_info` (accurate statistics)
4. ✅ `mcp semantic_search_docs` (functional but low relevance)
5. ⚠️ `mcp find_symbol` (correct signatures, wrong file paths)

## Usage in PR Documentation

These files demonstrate:

1. **Parser Completeness**: All C# language features are extracted correctly
2. **Known Limitations**: Relationship tracking needs improvement (237 unresolved)
3. **File ID Fix**: Symbols now have unique file IDs (verified with `retrieve describe`)
4. **Documentation**: XML comments are parsed and indexed
5. **Real-World Structure**: Mimics actual C# service architecture patterns

## For Contributors

When fixing bugs, use these test files to verify:

1. **Bug #9 Fix**: Run `codanna mcp get_calls Execute` - should show 6 calls
2. **Bug #8 Fix**: Run `codanna mcp search_symbols query:Service` - should find ~10 classes
3. **Bug #10 Fix**: Run `codanna mcp find_symbol ServiceA` - should show RelationshipTest.cs:27
4. **Relationship Resolution**: Re-index and verify relationship count increases from ~5 to ~50+

## Statistics

**ComprehensiveTest.cs:**
- Classes: 11
- Interfaces: 3
- Methods: ~40
- Properties: ~20
- Enums: 1
- Constants: 3

**RelationshipTest.cs:**
- Classes: 13
- Interfaces: 1
- Methods: ~40
- Clear call relationships: ~30

**Total Test Coverage:**
- ~80-100 symbols
- ~30-50 relationships (when fully resolved)
- 2 files, ~600 lines of code
- Comprehensive C# feature coverage
