---
description: Smart semantic search with natural language
argument-hint: <search_query>
---

## Search Query Analysis

**User's Original Query**: "$ARGUMENTS"

### Query Optimization

Claude, analyze the query above and improve it for code search:

1. **If vague** (e.g., "that parsing thing") → Make it specific (e.g., "language parser implementation")
2. **If a question** (e.g., "how does parsing work?") → Extract keywords (e.g., "parsing implementation process")
3. **If conversational** (e.g., "the stuff that handles languages") → Use technical terms (e.g., "language handler processor")
4. **If too broad** (e.g., "errors") → Add context (e.g., "error handling exception management")

**Optimized Search Query**: _{Claude: Write your improved query here, then use it below}_

---

### Semantic Search with Context

Execute this command with your optimized query:

```bash
codanna mcp semantic_search_with_context query:"{YourOptimizedQuery}" limit:5
```

### Alternative: Full-Text Search

If semantic search needs different keywords, try this (use your optimized query):

```bash
codanna retrieve search "{YourOptimizedQuery}" --limit 10
```

**Instructions for Claude**: 
1. First, write an optimized version of the user's query
2. Replace `{YourOptimizedQuery}` in both commands with your optimized query
3. Execute the semantic search command
4. If results are poor, try adjusting the query and searching again