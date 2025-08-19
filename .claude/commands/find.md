---
allowed-tools: Bash(codanna mcp:*), Bash(jq:*), Bash(echo:*)
description: Smart semantic search with natural language
argument-hint: <search_query>
---

## Semantic Search: "$ARGUMENTS"

### Search Results with Context
!`codanna mcp semantic_search_with_context query:"$ARGUMENTS" limit:3 --json | jq -r 'if .result then .result else {"error": "Search failed or no results"} end'`

### Alternative Text Search
If semantic search has limited results, here's a text-based search:
!`codanna retrieve search "$ARGUMENTS" --json | jq -r 'if .status == "success" then {results: .data.items[:5], total: .data.count} else {results: [], message: "No matches found"} end'`

## Analysis

Based on the search results for "$ARGUMENTS", I've identified the most relevant symbols:

1. **Primary Matches**: The top results based on semantic similarity
2. **Context**: How these symbols relate to your query
3. **Usage Patterns**: Common ways to use these symbols
4. **Related Concepts**: Other symbols you might be interested in

Let me know if you'd like more details about any specific result or want to refine the search query.