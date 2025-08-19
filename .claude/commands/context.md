---
allowed-tools: Bash(codanna:*), Bash(jq:*), Bash(echo:*)
description: Build comprehensive context for any symbol
argument-hint: <symbol_name>
---

## Complete Context for $ARGUMENTS

### Full Symbol Information
!`codanna retrieve describe $ARGUMENTS --json | jq -r 'if .status == "success" then .data else {"error": "Symbol not found"} end'`

### Relationships

#### Callers (Who Uses This)
!`codanna retrieve callers $ARGUMENTS --json | jq -r 'if .status == "success" then {callers: .data.items[:5], total_callers: .data.count} else {callers: [], note: "No callers or not callable"} end'`

#### Dependencies (What This Uses)
!`codanna retrieve calls $ARGUMENTS --json | jq -r 'if .status == "success" then {calls: .data.items[:5], total_calls: .data.count} else {calls: [], note: "No dependencies or not callable"} end'`

### Type-Specific Context

For different symbol types, I'll analyze:
- **Functions/Methods**: Call graph, parameters, return types
- **Structs**: Methods, fields, usage patterns
- **Traits**: Implementations, required methods
- **Enums**: Variants, pattern matching usage

### Related Symbols
!`codanna retrieve search $ARGUMENTS --json | jq -r 'if .status == "success" then {related: .data.items[:3]} else {related: []} end'`

## Comprehensive Overview

Based on the gathered context for `$ARGUMENTS`, I'll provide:
1. **Role in Codebase**: Primary purpose and responsibility
2. **Integration Points**: How it connects with other components
3. **Usage Patterns**: Common ways it's used
4. **Modification Impact**: What to consider when changing this symbol
5. **Testing Considerations**: Related test coverage and requirements