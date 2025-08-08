# TEST_TEMPLATE.md - TDD Test Blueprint

## ⚠️ READ-ONLY REFERENCE - Do Not Edit

This template captures the essential patterns for writing high-quality TDD tests in the Codanna project.
Use this as a blueprint when creating new tests. Copy relevant sections, don't modify this file.

### Required File Header

```rust
//! Purpose: [Describe what this test validates]
//! TDD Phase: [POC|Integration|Production]
//! 
//! Key validations:
//! - [List main test objectives]
//! - [Expected behaviors]
//! - [Performance targets if applicable]

use anyhow::Result;
use thiserror::Error;
use std::num::NonZeroU32;

/// Structured errors for the feature being tested
#[derive(Error, Debug)]
pub enum FeatureError {
    #[error("Invalid parameter: {0}\nSuggestion: Check parameter constraints and ensure values are within valid ranges")]
    InvalidParameter(String),
    
    #[error("Operation failed: {0}\nSuggestion: Review the operation requirements and verify all preconditions are met")]
    OperationFailed(String),
    
    // Add specific errors for your feature
}

// Type-safe wrappers for domain concepts (no primitive obsession!)
// MUST derive standard traits as per guidelines:
// - Debug: MANDATORY for all public types (except those with secrets)
// - Clone: Required if the type can logically be cloned
// - PartialEq/Eq/Hash: Required for types used as keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureId(NonZeroU32);

impl FeatureId {
    pub fn new(id: u32) -> Option<Self> {
        NonZeroU32::new(id).map(Self)
    }
    
    pub fn get(&self) -> u32 {
        self.0.get()
    }
}

// Example of a type that might not implement all traits
#[derive(Debug)] // Debug is MANDATORY
pub struct Feature {
    dimension: usize,
    capacity: usize,
    threshold: f32,
    // If this contained a file handle or secret, we'd skip Clone
}

// Constants for test configuration
const DEFAULT_BATCH_SIZE: usize = 100;
const PERFORMANCE_TARGET_MS: u64 = 10;
const MAX_RETRIES: u32 = 3;
```

### Test Structure Pattern

```rust
/// Test N: [Descriptive name explaining what is being validated]
/// Goal: [Specific behavior or requirement being tested]
#[test]
fn test_descriptive_name() -> Result<()> {
    // Given: Setup test data with clear intent
    let input = setup_test_data()?;
    let expected_count = 42;
    
    // When: Execute the operation being tested
    let result = perform_operation(&input)?;
    
    // Then: Validate all expected outcomes
    assert_eq!(result.len(), expected_count);
    assert!(result.iter().all(|item| item.is_valid()));
    
    // Print results for debugging (helps when tests fail)
    println!("\n=== Test N: [Name] ===");
    println!("✓ Processed {} items successfully", result.len());
    println!("✓ All items passed validation");
    println!("=== PASSED ===\n");
    
    Ok(())
}
```

### Builder Pattern for Complex Types

```rust
/// Builder pattern for ergonomic test data creation
/// MUST be infallible until build() is called - all validation happens in build()
#[derive(Debug, Default)]
pub struct FeatureBuilder {
    dimension: Option<usize>,
    capacity: Option<usize>,
    threshold: Option<f32>,
}

impl FeatureBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Builder methods use with_* naming convention
    // MUST be infallible - no validation here
    pub fn with_dimension(mut self, dim: usize) -> Self {
        self.dimension = Some(dim);
        self
    }
    
    pub fn with_capacity(mut self, cap: usize) -> Self {
        self.capacity = Some(cap);
        self
    }
    
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = Some(threshold);
        self
    }
    
    #[must_use = "Building the feature may fail, check the Result"]
    pub fn build(self) -> Result<Feature, FeatureError> {
        // ALL validation happens here, not in individual setters
        let dimension = self.dimension
            .ok_or_else(|| FeatureError::InvalidParameter("dimension required".into()))?;
        
        if dimension == 0 {
            return Err(FeatureError::InvalidParameter(
                "dimension must be greater than 0".into()
            ));
        }
        
        // Validate and construct
        Ok(Feature { 
            dimension, 
            capacity: self.capacity.unwrap_or(1000),
            threshold: self.threshold.unwrap_or(0.5),
        })
    }
}
```

### Error Handling Test Pattern

```rust
#[test]
fn test_handles_invalid_input_gracefully() -> Result<()> {
    // Test specific error conditions
    let result = FeatureBuilder::new()
        .with_dimension(0) // Invalid!
        .build();
    
    match result {
        Err(FeatureError::InvalidParameter(msg)) => {
            assert!(msg.contains("dimension"));
        }
        _ => panic!("Expected InvalidParameter error"),
    }
    
    Ok(())
}
```

### Performance Test Pattern

```rust
#[test]
fn test_meets_performance_target() -> Result<()> {
    use std::time::Instant;
    
    // Setup performance test data
    let large_dataset = generate_test_data(10_000)?;
    
    // Measure operation time
    let start = Instant::now();
    let _result = process_dataset(&large_dataset)?;
    let elapsed = start.elapsed();
    
    // Verify performance target
    assert!(
        elapsed.as_millis() < PERFORMANCE_TARGET_MS,
        "Operation took {}ms, expected <{}ms",
        elapsed.as_millis(),
        PERFORMANCE_TARGET_MS
    );
    
    println!("Performance: {}ms for {} items", 
             elapsed.as_millis(), large_dataset.len());
    
    Ok(())
}
```

### Helper Functions Pattern

```rust
// Group helper functions at the bottom of the test file
// Keep them focused and reusable

fn setup_test_data() -> Result<TestData> {
    // Create consistent test data
    Ok(TestData {
        items: vec![/* ... */],
    })
}

fn generate_test_data(count: usize) -> Result<Vec<Item>> {
    // Generate larger datasets for performance tests
    (0..count)
        .map(|i| Ok(Item::new(i)?))
        .collect()
}

// Use generic bounds for flexibility
fn perform_operation<T: AsRef<[Item]>>(items: T) -> Result<Output> {
    let items = items.as_ref();
    // Implementation
    Ok(Output::new())
}
```

### Integration Test Pattern

```rust
#[test]
fn test_end_to_end_workflow() -> Result<()> {
    // Test complete user workflow
    
    // Step 1: Initialize system
    let system = System::new()?;
    
    // Step 2: Process input
    let input = load_test_fixture("sample.txt")?;
    let processed = system.process(&input)?;
    
    // Step 3: Verify output
    assert_eq!(processed.status(), Status::Success);
    assert!(processed.validate()?);
    
    Ok(())
}
```

### Code Quality Checklist

Before submitting any test:

- [ ] Uses `Result<T, E>` everywhere (no `.unwrap()` in tests)
- [ ] Defines structured errors with `thiserror`
- [ ] Error messages include "Suggestion:" with actionable recovery steps
- [ ] Type-safe wrappers for all IDs and domain concepts  
- [ ] Generic bounds (`&str` not `String`, `&[T]` not `Vec<T>`)
- [ ] Performance targets defined as constants
- [ ] Clear Given/When/Then structure in tests
- [ ] Descriptive test output for debugging
- [ ] Helper functions use generic bounds
- [ ] No magic numbers - use named constants
- [ ] Tests are independent (no shared mutable state)
- [ ] Builder patterns have `#[must_use]` on `build()` methods
- [ ] All public types derive standard traits (Debug, Clone, PartialEq where appropriate)
- [ ] All public types MUST derive `Debug` (except those containing secrets)
- [ ] Types that can be cloned MUST derive `Clone`
- [ ] Types used as keys MUST derive `PartialEq`/`Eq`/`Hash`
- [ ] Builders MUST be infallible until `build()` is called

### File Organization Rules

#### File Size Limits
- **Target**: 300-400 lines per test file
- **Maximum**: 500 lines (must split if exceeded)
- **Minimum**: 100 lines (don't create tiny files)

#### Split by Logical Boundaries
```
tests/
  feature_core_test.rs        # Basic operations (300-400 lines)
  feature_errors_test.rs      # Error scenarios (200-300 lines)
  feature_perf_test.rs        # Performance tests (200-300 lines)
  feature_integration_test.rs # End-to-end tests (300-400 lines)
```

#### Common Test Utilities
```
tests/
  common/
    mod.rs         # Re-export all common items
    fixtures.rs    # Test data and constants
    builders.rs    # Shared test builders
    assertions.rs  # Custom assert macros
```

### Migration Path from POC to Production

1. **Phase 1: POC Exploration**
   ```rust
   // tests/poc_feature_test.rs
   use codanna::future::module::{Feature, Config}; // Import from where it WILL live
   ```

2. **Phase 2: API Stabilization**
   - Define complete public API through tests
   - All error cases covered
   - Performance requirements validated

3. **Phase 3: Production Migration**
   - Create module structure in `src/`
   - Move code piece by piece
   - Keep all tests green

4. **Phase 4: Cleanup**
   - Remove POC prefix when stable
   - Archive or delete POC tests

### Common Anti-patterns to AVOID

❌ **DON'T** use primitive types for IDs
```rust
// Bad
fn get_item(id: u32) -> Item

// Good  
fn get_item(id: ItemId) -> Item
```

❌ **DON'T** forget standard trait derivations
```rust
// Bad - missing required traits
pub struct Config {
    path: PathBuf,
}

// Good - all required traits
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    path: PathBuf,
}
```

❌ **DON'T** validate in builder setters
```rust
// Bad - validation in setter
impl Builder {
    pub fn with_size(mut self, size: usize) -> Result<Self, Error> {
        if size == 0 { return Err(Error::InvalidSize); }
        self.size = Some(size);
        Ok(self)
    }
}

// Good - infallible setter, validate in build()
impl Builder {
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = Some(size);
        self
    }
}
```

❌ **DON'T** use `.unwrap()` in tests
```rust
// Bad
let result = operation().unwrap();

// Good
let result = operation()?;
```

❌ **DON'T** create huge test files
```rust
// Bad: 2000+ line test file

// Good: Split into logical files under 500 lines
```

❌ **DON'T** use mocks or test doubles
```rust
// Bad: Mock implementations

// Good: Test against real implementations
```

### Performance Targets Reference

Common targets for code intelligence operations:
- Parsing: 10,000+ files/second
- Indexing: <100ms per file
- Search: <10ms for 1M symbols  
- Memory: ~100 bytes per symbol
- Startup: <1 second

Always define specific targets for your feature!