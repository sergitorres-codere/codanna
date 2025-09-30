# C# Parser - Practical Examples

Real-world examples demonstrating C# parser capabilities.

## Table of Contents

1. [Basic Symbol Extraction](#basic-symbol-extraction)
2. [Relationship Detection](#relationship-detection)
3. [Real-World Scenarios](#real-world-scenarios)
4. [MCP Query Examples](#mcp-query-examples)
5. [Integration Examples](#integration-examples)

---

## Basic Symbol Extraction

### Example 1: Simple Class

**Input: `UserService.cs`**
```csharp
using System;

namespace MyApp.Services
{
    public class UserService
    {
        private readonly IRepository _repository;

        public UserService(IRepository repository)
        {
            _repository = repository;
        }

        public User GetUser(int id)
        {
            return _repository.FindById(id);
        }
    }
}
```

**Index:**
```bash
codanna index . --progress
```

**Query:**
```bash
codanna retrieve search "UserService"
```

**Output:**
```
Class UserService at MyApp/Services/UserService.cs:5
Method UserService at MyApp/Services/UserService.cs:9  # Constructor
Method GetUser at MyApp/Services/UserService.cs:14
Field _repository at MyApp/Services/UserService.cs:7
```

**What was extracted:**
- ✅ Class name and location
- ✅ Constructor with parameters
- ✅ Method with signature
- ✅ Private field
- ✅ Namespace `MyApp.Services` as module path
- ✅ Using directives

---

### Example 2: Interface and Implementation

**Input: `IUserService.cs`**
```csharp
namespace MyApp.Services
{
    public interface IUserService
    {
        User GetUser(int id);
        Task<User> GetUserAsync(int id);
        void DeleteUser(int id);
    }
}
```

**Input: `UserServiceImpl.cs`**
```csharp
namespace MyApp.Services
{
    public class UserServiceImpl : IUserService
    {
        public User GetUser(int id) { /* ... */ }
        public Task<User> GetUserAsync(int id) { /* ... */ }
        public void DeleteUser(int id) { /* ... */ }
    }
}
```

**Query implementations:**
```bash
codanna retrieve implementations "IUserService"
```

**Output:**
```
IUserService is implemented by:
  - UserServiceImpl at MyApp/Services/UserServiceImpl.cs:3
```

**What was detected:**
- ✅ Interface definition
- ✅ All interface methods
- ✅ Implementation relationship
- ✅ Implementing class

---

### Example 3: Generic Types

**Input: `Repository.cs`**
```csharp
namespace MyApp.Data
{
    public class Repository<T> where T : class
    {
        private readonly DbContext _context;

        public async Task<T> FindAsync(int id)
        {
            return await _context.Set<T>().FindAsync(id);
        }

        public async Task<IEnumerable<T>> GetAllAsync()
        {
            return await _context.Set<T>().ToListAsync();
        }
    }
}
```

**Query:**
```bash
codanna retrieve search "Repository"
```

**Output:**
```
Class Repository at MyApp/Data/Repository.cs:3
Method FindAsync at MyApp/Data/Repository.cs:7
Method GetAllAsync at MyApp/Data/Repository.cs:12
Field _context at MyApp/Data/Repository.cs:5
```

**What was extracted:**
- ✅ Generic class with type parameter
- ✅ Async methods with Task<T> return types
- ✅ Generic method signatures preserved
- ✅ Where clause constraints recognized

---

## Relationship Detection

### Example 4: Method Call Tracking

**Input: `OrderController.cs`**
```csharp
namespace MyApp.Controllers
{
    public class OrderController : ControllerBase
    {
        private readonly IOrderService _orderService;
        private readonly ILogger _logger;

        public OrderController(IOrderService orderService, ILogger logger)
        {
            _orderService = orderService;
            _logger = logger;
        }

        public IActionResult CreateOrder(OrderDto dto)
        {
            _logger.LogInfo("Creating order");
            var order = _orderService.CreateOrder(dto);
            return Ok(order);
        }
    }
}
```

**After indexing, method calls are tracked:**

```
CreateOrder method contains calls to:
  - LogInfo (receiver: _logger)
  - CreateOrder (receiver: _orderService)
  - Ok (receiver: implicit)
```

**Caller context is preserved:**
```
Caller: CreateOrder
Target: LogInfo
Receiver: _logger
```

This enables queries like:
- "What methods does CreateOrder call?"
- "What external dependencies does OrderController have?"
- "Show the call chain starting from CreateOrder"

---

### Example 5: Complex Inheritance

**Input: `BaseController.cs`**
```csharp
public abstract class BaseController : ControllerBase
{
    protected readonly ILogger Logger;

    protected BaseController(ILogger logger)
    {
        Logger = logger;
    }

    protected IActionResult HandleError(Exception ex)
    {
        Logger.LogError(ex.Message);
        return StatusCode(500);
    }
}
```

**Input: `UserController.cs`**
```csharp
public class UserController : BaseController
{
    public UserController(ILogger logger) : base(logger) { }

    public IActionResult GetUser(int id)
    {
        try
        {
            // ... get user logic
        }
        catch (Exception ex)
        {
            return HandleError(ex);
        }
    }
}
```

**What's tracked:**
- ✅ Abstract base class
- ✅ Inheritance relationship (UserController extends BaseController)
- ✅ Protected members accessible to derived classes
- ✅ Method calls to base class methods
- ✅ Constructor chaining

---

## Real-World Scenarios

### Scenario 1: Analyzing Service Dependencies

**Goal:** Understand what a service depends on

**Code: `UserService.cs`**
```csharp
public class UserService : IUserService
{
    private readonly IUserRepository _repo;
    private readonly IEmailService _email;
    private readonly ILogger _logger;
    private readonly ICache _cache;

    public async Task<User> GetUserAsync(int id)
    {
        _logger.LogInfo($"Fetching user {id}");

        var cached = await _cache.GetAsync<User>($"user:{id}");
        if (cached != null) return cached;

        var user = await _repo.GetByIdAsync(id);
        await _cache.SetAsync($"user:{id}", user);

        return user;
    }

    public async Task SendWelcomeEmail(int userId)
    {
        var user = await GetUserAsync(userId);
        await _email.SendAsync(user.Email, "Welcome!");
    }
}
```

**MCP Query:** "What does UserService depend on?"

**Claude's Analysis:**
```
UserService Dependencies:

Injected Dependencies:
- IUserRepository (_repo) - Data access
- IEmailService (_email) - Email functionality
- ILogger (_logger) - Logging
- ICache (_cache) - Caching layer

Method Dependencies:
GetUserAsync calls:
  - LogInfo on _logger
  - GetAsync on _cache
  - GetByIdAsync on _repo
  - SetAsync on _cache

SendWelcomeEmail calls:
  - GetUserAsync (internal)
  - SendAsync on _email

External Types Used:
- Task<T> (System.Threading.Tasks)
- User (domain model)
```

---

### Scenario 2: Refactoring Impact Analysis

**Goal:** Determine impact of changing a method signature

**Original Method:**
```csharp
public User GetUser(int id)
{
    return _repository.FindById(id);
}
```

**MCP Query:** "What would break if I change GetUser to async?"

**Claude's Analysis:**
```
Impact Analysis: GetUser -> async Task<User> GetUserAsync(int id)

Direct Callers (5 locations):
1. UserController.GetUserDetails() - Line 42
   Status: ⚠️ Needs await

2. OrderService.CreateOrder() - Line 87
   Status: ⚠️ Needs await

3. ProfileService.GetProfile() - Line 23
   Status: ⚠️ Needs await

4. AdminDashboard.LoadUsers() - Line 156
   Status: ⚠️ Needs await, method must become async

5. BackgroundJob.ProcessUsers() - Line 92
   Status: ✅ Already async, easy migration

Transitive Impact:
- AdminDashboard.LoadUsers calls LoadDashboard (synchronous)
  → LoadDashboard would also need to become async

Recommendation:
Consider creating GetUserAsync alongside GetUser to enable
gradual migration rather than breaking all callers at once.
```

---

### Scenario 3: Finding All Repository Classes

**Goal:** Audit all data access layer classes

**MCP Query:** "Find all repository classes in the project"

**Claude's Response:**
```
Found 8 Repository Classes:

1. UserRepository (MyApp.Data)
   - Implements: IUserRepository
   - Methods: GetById, GetAll, Save, Delete
   - Line: UserRepository.cs:15

2. OrderRepository (MyApp.Data)
   - Implements: IOrderRepository
   - Methods: GetById, GetByUserId, Create, Update
   - Line: OrderRepository.cs:12

3. ProductRepository (MyApp.Data)
   - Implements: IProductRepository
   - Methods: GetById, GetByCategory, Search
   - Line: ProductRepository.cs:8

... [continues for all 8]

Common Pattern Detected:
All repositories implement:
- GetById(int id) method
- Inject DbContext
- Follow repository pattern interface

Architecture Note:
All repositories are in MyApp.Data namespace, good separation.
```

---

## MCP Query Examples

### Discovery Queries

**Query:** "Show me all public APIs in Controllers"

```bash
# Claude uses: semantic_search_docs + filter
```

**Response:**
```
Found 12 Controller Classes:

UserController (5 endpoints):
  - GET /api/users/{id}
  - GET /api/users
  - POST /api/users
  - PUT /api/users/{id}
  - DELETE /api/users/{id}

OrderController (4 endpoints):
  [... continues ...]
```

---

**Query:** "Find all async methods that don't use ConfigureAwait"

```bash
# Claude uses: search_symbols + semantic_search
```

**Response:**
```
Potential Issues Found:

18 async methods without ConfigureAwait(false):

1. UserService.GetUserAsync - Line 45
   await _repository.GetAsync(id);
   Suggestion: await _repository.GetAsync(id).ConfigureAwait(false);

[... continues with recommendations ...]
```

---

### Architectural Queries

**Query:** "Show me the layered architecture of this application"

**Response:**
```
Architecture Analysis:

Controllers Layer (12 classes):
  → Services Layer (8 classes)
      → Repositories Layer (8 classes)
          → DbContext (Entity Framework)

Cross-Cutting:
  - Logging: ILogger injected in 18 classes
  - Caching: ICache used in 5 services
  - Email: IEmailService used in 3 services

Observations:
✅ Clean separation of concerns
✅ Dependency injection throughout
⚠️ UserController directly accesses UserRepository (bypasses service layer)
```

---

**Query:** "Which classes have the most dependencies?"

**Response:**
```
Classes by Dependency Count:

1. OrderService (8 dependencies)
   - IOrderRepository
   - IUserService
   - IProductService
   - IPaymentService
   - IEmailService
   - ILogger
   - ICache
   - IConfiguration

2. CheckoutController (6 dependencies)
   [... continues ...]

Recommendation:
OrderService has high coupling. Consider:
- Breaking into smaller services
- Using mediator pattern
- Introducing a facade
```

---

## Integration Examples

### Example: CI/CD Integration

**`.github/workflows/code-analysis.yml`**
```yaml
name: C# Code Analysis

on: [push, pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Setup .NET
        uses: actions/setup-dotnet@v3
        with:
          dotnet-version: '8.0.x'

      - name: Install codanna
        run: cargo install codanna

      - name: Index codebase
        run: codanna index ./src --progress

      - name: Verify symbol count
        run: |
          SYMBOLS=$(codanna retrieve search "*" --limit 10000 | wc -l)
          echo "::notice::Found $SYMBOLS symbols"

          if [ $SYMBOLS -lt 100 ]; then
            echo "::error::Symbol count too low: $SYMBOLS"
            exit 1
          fi

      - name: Check for large classes
        run: |
          # Find classes with many methods (potential refactor candidates)
          codanna retrieve search "Service" |
            while read line; do
              CLASS=$(echo $line | awk '{print $2}')
              echo "Analyzing $CLASS"
            done

      - name: Generate symbol report
        run: |
          echo "# Code Analysis Report" > report.md
          echo "" >> report.md
          echo "## Symbol Statistics" >> report.md
          echo "Total Symbols: $(codanna retrieve search '*' --limit 10000 | wc -l)" >> report.md
          echo "" >> report.md
          echo "## Public Classes" >> report.md
          codanna retrieve search "*" --limit 100 | grep "Class" >> report.md

      - name: Upload report
        uses: actions/upload-artifact@v3
        with:
          name: code-analysis-report
          path: report.md
```

---

### Example: Pre-commit Hook

**`.git/hooks/pre-commit`**
```bash
#!/bin/bash

echo "Running codanna analysis..."

# Re-index if C# files changed
CHANGED_CS=$(git diff --cached --name-only --diff-filter=ACM | grep '\.cs$')

if [ ! -z "$CHANGED_CS" ]; then
    echo "C# files changed, re-indexing..."
    codanna index . --progress

    # Verify no syntax errors
    FILES_INDEXED=$(codanna retrieve search "*" --limit 1000 | wc -l)

    if [ $FILES_INDEXED -eq 0 ]; then
        echo "❌ Error: No symbols indexed. Check for syntax errors."
        exit 1
    fi

    echo "✅ Indexed $FILES_INDEXED symbols"
fi

exit 0
```

---

### Example: VS Code Task

**`.vscode/tasks.json`**
```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Index C# Code",
      "type": "shell",
      "command": "codanna",
      "args": ["index", ".", "--progress"],
      "problemMatcher": [],
      "presentation": {
        "reveal": "always",
        "panel": "new"
      }
    },
    {
      "label": "Search Symbols",
      "type": "shell",
      "command": "codanna",
      "args": [
        "retrieve",
        "search",
        "${input:symbolName}",
        "--limit",
        "10"
      ],
      "problemMatcher": []
    },
    {
      "label": "Start MCP Server",
      "type": "shell",
      "command": "codanna",
      "args": ["mcp"],
      "isBackground": true,
      "problemMatcher": {
        "pattern": {
          "regexp": "^(.*)$",
          "file": 1
        },
        "background": {
          "activeOnStart": true,
          "beginsPattern": "^Starting MCP server",
          "endsPattern": "^Server started"
        }
      }
    }
  ],
  "inputs": [
    {
      "id": "symbolName",
      "type": "promptString",
      "description": "Symbol name to search for",
      "default": "*"
    }
  ]
}
```

---

## Advanced Example: Custom Analysis Script

**`analyze_csharp.sh`**
```bash
#!/bin/bash
# Custom C# codebase analysis script

set -e

PROJECT_DIR=${1:-.}
OUTPUT_DIR="./analysis_output"

echo "🔍 Analyzing C# codebase in: $PROJECT_DIR"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Index the codebase
echo "📚 Indexing..."
codanna index "$PROJECT_DIR" --progress

# Extract all symbols
echo "📊 Extracting symbols..."
codanna retrieve search "*" --limit 10000 > "$OUTPUT_DIR/all_symbols.txt"

# Count by type
echo "📈 Generating statistics..."
cat "$OUTPUT_DIR/all_symbols.txt" | awk '{print $1}' | sort | uniq -c | sort -rn > "$OUTPUT_DIR/symbol_counts.txt"

# Find large classes (heuristic: many methods)
echo "🔎 Finding large classes..."
grep "^Class" "$OUTPUT_DIR/all_symbols.txt" | awk '{print $2}' | while read class; do
    METHOD_COUNT=$(grep "Method.*$class" "$OUTPUT_DIR/all_symbols.txt" | wc -l)
    if [ $METHOD_COUNT -gt 10 ]; then
        echo "$class: $METHOD_COUNT methods" >> "$OUTPUT_DIR/large_classes.txt"
    fi
done

# Extract public API surface
echo "🌐 Extracting public API..."
grep "^Class\|^Interface\|^Method" "$OUTPUT_DIR/all_symbols.txt" | grep -v "^Method.*private" > "$OUTPUT_DIR/public_api.txt"

# Generate report
echo "📄 Generating report..."
cat > "$OUTPUT_DIR/report.md" << EOF
# C# Codebase Analysis Report

Generated: $(date)

## Statistics

\`\`\`
$(cat "$OUTPUT_DIR/symbol_counts.txt")
\`\`\`

## Large Classes (>10 methods)

$(cat "$OUTPUT_DIR/large_classes.txt" 2>/dev/null || echo "None found")

## Public API Surface

Total public symbols: $(wc -l < "$OUTPUT_DIR/public_api.txt")

See \`public_api.txt\` for full list.

## Recommendations

- Review large classes for potential refactoring
- Consider splitting classes with >15 methods
- Audit public API for breaking changes before release

EOF

echo "✅ Analysis complete! Report saved to $OUTPUT_DIR/report.md"
open "$OUTPUT_DIR/report.md"  # macOS
# xdg-open "$OUTPUT_DIR/report.md"  # Linux
# start "$OUTPUT_DIR/report.md"  # Windows
```

**Usage:**
```bash
chmod +x analyze_csharp.sh
./analyze_csharp.sh /path/to/csharp/project
```

---

## Summary

These examples demonstrate:

✅ **Symbol Extraction** - All C# constructs properly indexed
✅ **Relationship Tracking** - Method calls, implementations tracked
✅ **Real-World Usage** - CI/CD, pre-commit hooks, custom analysis
✅ **MCP Integration** - Natural language queries on your codebase
✅ **Architectural Analysis** - Understand codebase structure

For more details, see:
- Full documentation: `MANUAL.md`
- Quick start: `QUICKSTART.md`
- Troubleshooting: `MANUAL.md#troubleshooting`