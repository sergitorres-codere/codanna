# Codanna MCP Tools Workflow Guide

## Tool Selection Decision Tree

### 1. Starting Fresh? Get Your Bearings

<tool>
mcp__codanna__get_index_info
</tool>

- **When**: First interaction with a codebase
- **Why**: Understand scope (files, symbols, relationships, semantic search status)
- **What you get**: Overview metrics to calibrate expectations

### 2. Know Exactly What You're Looking For?

<tool>
mcp__codanna__find_symbol (exact name)
mcp__codanna__search_symbols (fuzzy match)
</tool>

- **When**: User mentions specific function/method/class names
- **Why**: Direct path to implementation
- **Pro tip**: Try `find_symbol` first, fall back to `search_symbols` if no exact match
- **New**: `find_symbol` now shows implementation counts and method counts!

### 3. Exploring Concepts or Features?

<tool>
mcp__codanna__semantic_search_docs (initial exploration)
mcp__codanna__semantic_search_with_context (deep dive)
</tool>

- **When**: User asks about features, concepts, or "how does X work?"
- **Why**: Natural language understanding finds relevant code
- **Strategy**: Start with `semantic_search_docs` for overview, use `semantic_search_with_context` for the most relevant result

### 4. Understanding Code Relationships?

#### Call Hierarchy Pattern:

<workflow>
1. mcp__codanna__find_symbol (locate the function)
2. mcp__codanna__get_calls (what it calls)
3. mcp__codanna__find_callers (what calls it)
4. mcp__codanna__analyze_impact (change impact)
</workflow>

- **When**: User asks about dependencies, usage, or refactoring impact
- **Why**: Builds complete picture of code interconnections
- **Enhanced**: Now shows `(receiver.method)` patterns thanks to MethodCall enhancement!
- **New**: `analyze_impact` now includes exact file:line locations for all impacted symbols!

## Optimal Workflows by Task Type

### "How is X implemented?"

<workflow>
1. semantic_search_docs("X feature implementation", limit=5)
2. Pick most relevant result
3. semantic_search_with_context("X feature", limit=1)
4. Get comprehensive context with callers/callees
</workflow>

### "What does function Y do?"

<workflow>
1. find_symbol("Y")
2. If found: get_calls("Y") 
3. If has callers: find_callers("Y")
4. For deeper understanding: semantic_search_with_context("Y", limit=1)
</workflow>

### "Find all places where Z is used"

<workflow>
1. find_symbol("Z")
2. find_callers("Z") 
3. For each interesting caller: get_calls(caller_name)
4. analyze_impact("Z") for full dependency tree
</workflow>

### "Understand this error/bug"

<workflow>
1. semantic_search_docs("error message or symptom", limit=5)
2. For each relevant symbol: find_callers(symbol)
3. Trace call chains with get_calls()
4. Use semantic_search_with_context() on suspicious functions
</workflow>

### "Refactor or modify existing code"

<workflow>
1. find_symbol("target_function")
2. analyze_impact("target_function", max_depth=3)
3. For each impacted symbol: get_calls()
4. semantic_search_docs("similar patterns", limit=5) to ensure consistency
</workflow>

## Pro Tips

### Leverage Enhanced Method Call Information

The new MethodCall enhancement shows:
- `(Type::method)` - static method calls
- `(self.method)` - instance method on self
- `(receiver.method)` - instance method on specific receiver

This helps understand:
- Ownership patterns
- API design (static vs instance)
- Method chaining possibilities

### Semantic Search Strategies

1. **Be descriptive**: "parse AST nodes recursively" > "parse"
2. **Include context**: "error handling in network requests" > "error"
3. **Use domain terms**: Match the codebase's vocabulary
4. **Adjust threshold**: Lower to 0.3-0.4 for broader results

### Tool Chaining Patterns

<pattern>
Broad → Narrow → Deep
semantic_search → find_symbol → get_calls/analyze_impact
</pattern>

### When to Use the Powerhouse (`semantic_search_with_context`)

- First-time exploring a complex feature
- Need to understand full context quickly  
- Investigating cross-cutting concerns
- One-shot comprehensive analysis

### Performance Optimization

<optimization>
CLI Command Chaining for Efficiency:
When investigating multiple symbols, use Unix-style command chaining:

# Find multiple symbols in parallel
codanna mcp find_symbol --args '{"name": "authenticate"}' & \
codanna mcp find_symbol --args '{"name": "authorize"}' & \
codanna mcp find_symbol --args '{"name": "validate_token"}' & \
wait

# Chain analysis: find symbol → get its callers → analyze impact
codanna mcp find_symbol --args '{"name": "validate_token"}' && \
codanna mcp find_callers --args '{"function_name": "validate_token"}' && \
codanna mcp analyze_impact --args '{"symbol_name": "validate_token", "max_depth": 2}'

# Search and drill down: semantic search → pick result → get full context
codanna mcp semantic_search_docs --args '{"query": "parse function calls", "limit": 5}' | \
grep -B1 -A1 "Method" | head -10 && \
codanna mcp find_symbol --args '{"name": "parse_function_call"}'

# Real-world example: Investigate error handling patterns
codanna mcp semantic_search_docs --args '{"query": "error handling", "limit": 3}' && \
echo "=== Analyzing IndexError usage ===" && \
codanna mcp find_symbol --args '{"name": "IndexError"}' && \
codanna mcp search_symbols --args '{"query": "Error", "limit": 5}'
</optimization>

1. Use `&` for parallel execution when analyzing independent symbols
2. Use `&&` to chain dependent commands (second runs only if first succeeds)
3. Use `|` with grep to filter results before deeper analysis
4. Cache symbol names from outputs for subsequent commands

### Common Pitfalls to Avoid

<pitfall>
❌ Using semantic_search for exact symbol names
✅ Use find_symbol for known names - it's faster and more precise
</pitfall>

<pitfall>
❌ Forgetting to check semantic search availability
✅ Always check get_index_info first - semantic search might be disabled
</pitfall>

<pitfall>
❌ Using max_depth > 3 for analyze_impact on large codebases
✅ Start with depth 2, increase only if needed - deep analysis can be slow
</pitfall>

<pitfall>
❌ Calling find_callers on generic method names like "get" or "set"
✅ Be specific or use semantic search to find the right context first
</pitfall>

## Decision Matrix

| User Asks About | Primary Tool | Follow-up Tools |
|----------------|--------------|-----------------|
| Specific function | `find_symbol` | `get_calls`, `find_callers` |
| Feature/concept | `semantic_search_docs` | `semantic_search_with_context` |
| Code relationships | `find_callers`/`get_calls` | `analyze_impact` |
| Impact of changes | `analyze_impact` | `get_calls` for each impacted |
| Similar patterns | `semantic_search_docs` | `search_symbols` for variations |
| "How does X work?" | `semantic_search_with_context` | `get_calls` on key functions |

## Example: Complete Investigation Flow

**User**: "How does authentication work in this codebase?"

<example>
1. semantic_search_docs("authentication login user", limit=5)
   → Find: authenticate_user, validate_token, check_permissions

2. semantic_search_with_context("authenticate_user", limit=1)  
   → See full context: calls hash_password, validates against db
   → Called by: login_handler, api_auth_middleware

3. analyze_impact("authenticate_user", max_depth=2)
   → Understand full auth flow impact

4. get_calls("login_handler")
   → See: (self.validate_input), (AuthService::authenticate_user)
   → Understand the authentication sequence

5. semantic_search_docs("token JWT session", limit=3)
   → Find related security mechanisms
</example>

## Quick Reference

### Tool Capabilities Summary

| Tool | Best For | Returns | Enhanced Output |
|------|----------|---------|-----------------|
| `get_index_info` | Initial overview | Symbol counts, file counts, embedding status | Shows if semantic search is enabled |
| `find_symbol` | Exact name lookup | Symbol location and documentation | **NEW**: Implementation & method counts |
| `search_symbols` | Fuzzy name search | List of matching symbols | Sorted by relevance |
| `get_calls` | Outgoing dependencies | Functions called with receiver context | Shows `(Type::method)` patterns |
| `find_callers` | Incoming dependencies | Functions that call this symbol | Shows `(receiver.method)` patterns |
| `analyze_impact` | Change impact radius | Tree of affected symbols | **NEW**: Exact file:line locations |
| `semantic_search_docs` | Concept search | Symbols with similarity scores | Natural language understanding |
| `semantic_search_with_context` | Deep analysis | Symbol with full context (calls, callers, impact) | The powerhouse - one-stop analysis |

### Enhanced Method Call Patterns

<pattern>
Static method:    (Type::method)
Instance on self: (self.method)  
Instance on var:  (receiver.method)
Unknown receiver: plain method name
</pattern>

## Remember

The tools are designed to complement each other. Start broad, get specific, then understand relationships. The enhanced MethodCall information provides crucial context about how code is actually used, not just where it's defined.

**Golden Rule**: When in doubt, start with `semantic_search_docs` - it's surprisingly good at finding relevant code even with vague queries!