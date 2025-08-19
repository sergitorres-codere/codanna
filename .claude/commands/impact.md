---
allowed-tools: Bash(codanna retrieve:*), Bash(jq:*), Bash(echo:*), Bash(head:*)
description: Complete impact analysis for a symbol
argument-hint: <symbol_name>
---

## Impact Analysis for $ARGUMENTS

### Symbol Details
!`codanna retrieve symbol $ARGUMENTS --json | jq -r '.data.items[0] // {"error": "Symbol not found"}'`

### Who Calls This (Direct Impact)
!`codanna retrieve callers $ARGUMENTS --json | jq -r 'if .status == "success" then .data.items[:10] else {"message": "No callers found or not a callable symbol"} end'`

### What This Calls (Dependencies)
!`codanna retrieve calls $ARGUMENTS --json | jq -r 'if .status == "success" then .data.items[:10] else {"message": "No dependencies found or not a callable symbol"} end'`

### Full Description
!`codanna retrieve describe $ARGUMENTS --json | jq -r 'if .status == "success" then .data else {"message": "No description available"} end'`

## Summary

Based on the above data, here's the impact analysis for `$ARGUMENTS`:

1. **Symbol Type**: Analyze based on the kind (Function, Struct, Trait, etc.)
2. **Direct Impact**: Who directly depends on this symbol
3. **Dependencies**: What this symbol depends on
4. **Change Risk**: Assessment of modification impact
5. **Recommendations**: Suggested approach for changes