# C# Parser - Remaining Features Plan

## ✅ Completed Features
- ✅ Primary Constructors (C# 12) - 2-3 hours
- ✅ Enhanced Records Support (C# 9+) - 3-4 hours
- ✅ Using Directives
- ✅ File-scoped Types
- ✅ Generic Constraints
- ✅ Nullable Types

**Total Completed**: 6 features

---

## 🎯 Remaining Features (4 features, ~13-17 hours)

### Priority 2: Should Implement (Medium Value)

#### 1. Extension Methods 🟡 MEDIUM PRIORITY
**Estimated Effort**: 4-5 hours
**Business Value**: Medium (helps understand utility patterns in codebases)
**Complexity**: Medium-High

**What it does**:
```csharp
public static class StringExtensions
{
    // Extension method (first param has 'this' modifier)
    public static bool IsEmpty(this string str)
    {
        return string.IsNullOrEmpty(str);
    }

    public static T Parse<T>(this string str) where T : IParseable<T>
    {
        return T.Parse(str);
    }
}

// Usage
string name = "test";
bool empty = name.IsEmpty(); // Tracks that IsEmpty extends string
```

**Implementation Tasks**:
- Detect static methods in static classes
- Check for `this` modifier on first parameter
- Extract extended type (first parameter type)
- Store extension method metadata in symbol
- Track extension method calls with proper relationships
- Add 5-7 comprehensive tests

**⚠️ NOTE**: May require changes outside the C# parser file (Symbol struct changes) - requires permission first

---

#### 2. Improved Documentation Extraction 🟡 MEDIUM PRIORITY
**Estimated Effort**: 3-4 hours
**Business Value**: Medium (better doc support benefits all features)
**Complexity**: Medium

**What it does**:
```csharp
/// <summary>
/// Calculates the sum of two numbers
/// </summary>
/// <param name="a">First number</param>
/// <param name="b">Second number</param>
/// <returns>The sum of a and b</returns>
/// <exception cref="OverflowException">Thrown when overflow</exception>
public int Add(int a, int b) { return a + b; }
```

**Implementation Tasks**:
- Parse XML doc comment tags
- Extract: summary, param, returns, exception, remarks
- Create structured DocComment storage
- Store parsed tags separately (not just raw text)
- Add 5-6 comprehensive tests

**Current State**: Basic `///` comment extraction works, need structured XML parsing

**⚠️ NOTE**: May require changes outside the C# parser file (ParsedDocComment struct) - requires permission first

---

### Priority 3: Nice to Have (Lower Priority)

#### 3. Operator Overloading 🟢 LOW PRIORITY
**Estimated Effort**: 2-3 hours
**Business Value**: Low (niche feature)
**Complexity**: Low-Medium

**What it does**:
```csharp
public class Vector
{
    public static Vector operator +(Vector a, Vector b)
    {
        return new Vector { X = a.X + b.X, Y = a.Y + b.Y };
    }

    public static bool operator ==(Vector a, Vector b)
    {
        return a.X == b.X && a.Y == b.Y;
    }
}
```

**Implementation Tasks**:
- Detect `operator_declaration` nodes
- Extract operator symbol (+, -, ==, !=, etc.)
- Create method symbol with name like "operator+"
- Include full signature with operator keyword
- Add 3-4 comprehensive tests

**Tree-sitter nodes**: `operator_declaration`

---

#### 4. Async/Await Tracking 🟢 LOW PRIORITY
**Estimated Effort**: 2-3 hours
**Business Value**: Low (mostly syntactic, already in signatures)
**Complexity**: Low

**What it does**:
```csharp
public class Service
{
    // Async method
    public async Task<string> GetDataAsync()
    {
        await Task.Delay(100);
        return "data";
    }

    // Async void (event handlers)
    public async void HandleClick(object sender, EventArgs e)
    {
        await ProcessAsync();
    }
}
```

**Implementation Tasks**:
- Detect `async` modifier on methods
- Ensure "async" appears in signature
- Track Task return types
- (Optional) Track `await` expressions
- Add 3-4 comprehensive tests

**Current State**: Async methods likely already work, just need to verify and test

---

## 📊 Quick Stats

| Priority | Count | Time Estimate |
|----------|-------|---------------|
| Should Implement | 2 features | 7-9 hours |
| Nice to Have | 2 features | 4-6 hours |
| **TOTAL REMAINING** | **4 features** | **11-15 hours** |

---

## 🎯 Recommended Implementation Order

### Next Session: Extension Methods + Operator Overloading
**Time**: 6-8 hours
**Features**: Extension Methods + Operator Overloading

**Why these**:
- Both are "advanced method" features
- Operator overloading is simpler warmup (2-3 hours)
- Extension methods more complex but valuable (4-5 hours)
- ⚠️ Extension methods may require Symbol struct changes (ask permission first)

### Future Session: Polish Features
**Time**: 5-7 hours
**Features**: Improved Documentation + Async/Await

**Why last**:
- Lower priority enhancements
- Documentation benefits all existing features
- Async/await is mostly verification work
- Good wrap-up features

---

## ⚠️ Important Notes

1. **Tree-sitter Verification**: Always verify AST nodes exist before implementing
2. **Data Structure Changes**: Extension Methods and Documentation may need Symbol struct changes - **ask for permission first**
3. **Testing**: Aim for 5-8 tests per feature minimum
4. **Quality**: All changes must pass: cargo test, check, fmt, clippy
5. **Scope**: Stay within `src/parsing/csharp/parser.rs` unless explicitly permitted

---

## ❌ Excluded Features

**LINQ Query Syntax** - Tree-sitter-c-sharp v0.23.1 doesn't generate expected AST nodes. Skip until tree-sitter-c-sharp is updated.
