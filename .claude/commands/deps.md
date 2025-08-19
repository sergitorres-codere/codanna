---
allowed-tools: Bash(codanna retrieve:*), Bash(jq:*), Bash(echo:*)
description: Analyze dependencies of a symbol
argument-hint: <symbol_name>
---

## Dependency Analysis for $ARGUMENTS

### Symbol Information
!`codanna retrieve symbol $ARGUMENTS --json | jq -r 'if .status == "success" then .data.items[0] else {"error": "Symbol not found"} end'`

### Direct Dependencies (What This Depends On)
!`codanna retrieve calls $ARGUMENTS --json | jq -r 'if .status == "success" then {dependencies: .data.items, total_deps: .data.count} else {dependencies: [], note: "No dependencies found or not a callable symbol"} end'`

### Reverse Dependencies (Who Depends on This)
!`codanna retrieve callers $ARGUMENTS --json | jq -r 'if .status == "success" then {dependents: .data.items[:10], total_dependents: .data.count} else {dependents: [], note: "No dependents found"} end'`

### Implementation Dependencies
For traits and interfaces:
!`codanna retrieve implementations $ARGUMENTS --json | jq -r 'if .status == "success" then {implementations: .data.items, total: .data.count} else {implementations: [], note: "Not a trait or no implementations"} end'`

## Dependency Structure Analysis

Based on the dependency analysis for `$ARGUMENTS`:

### Dependency Characteristics
1. **Dependency Count**: Total number of direct dependencies
2. **Dependent Count**: How many other components depend on this
3. **Coupling Level**: Assessment of how tightly coupled this symbol is
4. **Stability**: Based on dependent/dependency ratio

### Risk Assessment
- **High Risk Dependencies**: External or unstable dependencies
- **Breaking Change Impact**: Number of dependents that would be affected
- **Refactoring Difficulty**: Based on coupling and complexity

### Recommendations
1. **Decoupling Opportunities**: Where dependencies could be reduced
2. **Interface Segregation**: If this symbol does too much
3. **Testing Priority**: Based on number of dependents