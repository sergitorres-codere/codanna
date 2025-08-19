---
allowed-tools: Bash(codanna retrieve:*), Bash(jq:*), Bash(xargs:*), Bash(sh:*), Bash(echo:*), Bash(head:*)
description: Trace call graph from a starting point
argument-hint: <function_name> [depth]
---

## Call Graph Trace for $ARGUMENTS

### Starting Symbol
!`codanna retrieve symbol $ARGUMENTS --json | jq -r 'if .status == "success" then .data.items[0] else {"error": "Symbol not found"} end'`

### Direct Calls (Level 1)
!`codanna retrieve calls $ARGUMENTS --json | jq -r 'if .status == "success" then {direct_calls: .data.items[:20], total: .data.count} else {direct_calls: [], note: "No calls found or not a callable symbol"} end'`

### Call Chain (2 Levels Deep)
!`codanna retrieve calls $ARGUMENTS --json | jq -r '.data.items[:5].symbol.name // empty' | head -5 | xargs -I {} sh -c 'echo "=== Calls from {} ===" && codanna retrieve calls {} --json | jq -r "if .status == \"success\" then .data.items[:3] else {message: \"No further calls\"} end"'`

### Reverse Trace - Who Calls This
!`codanna retrieve callers $ARGUMENTS --json | jq -r 'if .status == "success" then {callers: .data.items[:10], total: .data.count} else {callers: [], note: "No callers found"} end'`

## Execution Flow Analysis

Based on the call graph trace for `$ARGUMENTS`:

1. **Entry Point**: Whether this is called by other functions or is an entry point
2. **Call Depth**: How deep the dependency chain goes
3. **Critical Path**: The most important execution paths
4. **Side Effects**: Functions that might have external effects
5. **Recursion**: Any recursive patterns detected

### Visualization
```
$ARGUMENTS
├── [direct calls listed above]
│   ├── [their calls]
│   └── ...
└── [more direct calls]
```

This trace helps understand the execution flow and dependencies starting from `$ARGUMENTS`.