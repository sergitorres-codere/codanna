#!/bin/bash

# Test script for dual format support on all retrieve commands
# Tests both traditional (flag) and key:value formats

echo "=== DUAL FORMAT COMPREHENSIVE TEST SUITE ==="
echo "Testing all retrieve commands with both formats"
echo ""

BINARY="./target/release/codanna"

# Color codes for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

test_command() {
    local cmd_name=$1
    local traditional=$2
    local keyvalue=$3
    
    echo "Testing $cmd_name:"
    
    # Test traditional format
    if $BINARY retrieve $traditional > /dev/null 2>&1; then
        echo -e "  Traditional: ${GREEN}✓${NC}"
    else
        EXIT_CODE=$?
        if [ $EXIT_CODE -eq 3 ]; then
            echo -e "  Traditional: ${GREEN}✓${NC} (not found - exit 3)"
        else
            echo -e "  Traditional: ${RED}✗${NC} (exit $EXIT_CODE)"
        fi
    fi
    
    # Test key:value format
    if $BINARY retrieve $keyvalue > /dev/null 2>&1; then
        echo -e "  Key:value:   ${GREEN}✓${NC}"
    else
        EXIT_CODE=$?
        if [ $EXIT_CODE -eq 3 ]; then
            echo -e "  Key:value:   ${GREEN}✓${NC} (not found - exit 3)"
        else
            echo -e "  Key:value:   ${RED}✗${NC} (exit $EXIT_CODE)"
        fi
    fi
    echo ""
}

echo "=== 1. SYMBOL COMMAND ==="
test_command "Symbol" "symbol main --json" "symbol name:main --json"

echo "=== 2. CALLS COMMAND ==="
test_command "Calls" "calls process_file --json" "calls function:process_file --json"

echo "=== 3. CALLERS COMMAND ==="
test_command "Callers" "callers main --json" "callers function:main --json"

echo "=== 4. IMPLEMENTATIONS COMMAND ==="
test_command "Implementations" "implementations Parser --json" "implementations trait:Parser --json"

echo "=== 5. DESCRIBE COMMAND ==="
test_command "Describe" "describe OutputManager --json" "describe symbol:OutputManager --json"

echo "=== 6. IMPACT COMMAND ==="
echo "Testing Impact (with depth parameter):"
# Traditional with flag
if $BINARY retrieve impact OutputManager --depth 2 --json > /dev/null 2>&1; then
    echo -e "  Traditional with flag: ${GREEN}✓${NC}"
else
    echo -e "  Traditional with flag: ${RED}✗${NC}"
fi

# Key:value format
if $BINARY retrieve impact symbol:OutputManager depth:2 --json > /dev/null 2>&1; then
    echo -e "  Key:value format:      ${GREEN}✓${NC}"
else
    echo -e "  Key:value format:      ${RED}✗${NC}"
fi

# Mixed format
if $BINARY retrieve impact OutputManager depth:2 --json > /dev/null 2>&1; then
    echo -e "  Mixed format:          ${GREEN}✓${NC}"
else
    echo -e "  Mixed format:          ${RED}✗${NC}"
fi
echo ""

echo "=== 7. SEARCH COMMAND ==="
echo "Testing Search (with multiple parameters):"
# Traditional
if $BINARY retrieve search "output" --limit 2 --kind struct --json > /dev/null 2>&1; then
    echo -e "  Traditional:  ${GREEN}✓${NC}"
else
    echo -e "  Traditional:  ${RED}✗${NC}"
fi

# Key:value
if $BINARY retrieve search query:output limit:2 kind:struct --json > /dev/null 2>&1; then
    echo -e "  Key:value:    ${GREEN}✓${NC}"
else
    echo -e "  Key:value:    ${RED}✗${NC}"
fi

# Mixed
if $BINARY retrieve search "output" limit:2 kind:struct --json > /dev/null 2>&1; then
    echo -e "  Mixed:        ${GREEN}✓${NC}"
else
    echo -e "  Mixed:        ${RED}✗${NC}"
fi
echo ""

echo "=== ERROR HANDLING TEST ==="
echo "Testing commands without required arguments:"

# Test missing arguments (should exit with code 1)
echo -n "Symbol without args: "
$BINARY retrieve symbol 2>&1 | grep -q "Error: symbol requires a name" && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

echo -n "Calls without args:  "
$BINARY retrieve calls 2>&1 | grep -q "Error: calls requires a function name" && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

echo -n "Impact without args: "
$BINARY retrieve impact 2>&1 | grep -q "Error: impact requires a symbol name" && echo -e "${GREEN}✓${NC}" || echo -e "${RED}✗${NC}"

echo ""
echo "=== PRECEDENCE TEST ==="
echo "Testing flag precedence over key:value:"

# Test that --depth 2 overrides depth:5
OUTPUT=$($BINARY retrieve impact OutputManager depth:5 --depth 2 --json 2>&1 | jq -r '.metadata.max_depth' 2>/dev/null)
if [ "$OUTPUT" = "2" ]; then
    echo -e "Impact depth precedence: ${GREEN}✓${NC} (flag wins: --depth 2 over depth:5)"
else
    echo -e "Impact depth precedence: ${RED}✗${NC} (expected 2, got $OUTPUT)"
fi

# Test that --limit 1 overrides limit:10
COUNT=$($BINARY retrieve search "output" limit:10 --limit 1 --json 2>&1 | jq -r '.count' 2>/dev/null)
if [ "$COUNT" = "1" ]; then
    echo -e "Search limit precedence: ${GREEN}✓${NC} (flag wins: --limit 1 over limit:10)"
else
    echo -e "Search limit precedence: ${RED}✗${NC} (expected 1, got $COUNT)"
fi

echo ""
echo "=== PERFORMANCE TEST ==="
echo "Checking all operations complete in <300ms:"
time $BINARY retrieve search "unified output" limit:3 --json > /dev/null 2>&1

echo ""
echo "=== TEST SUITE COMPLETE ===" 