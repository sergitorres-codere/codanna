// Gateway file to expose integration tests from the integration/ subdirectory
// This file allows Rust's test runner to discover tests in subdirectories

// Re-export the integration test modules
// Each test file in integration/ needs to be included here
#[path = "integration/test_c_resolution.rs"]
mod test_c_resolution;

#[path = "integration/test_cpp_resolution.rs"]
mod test_cpp_resolution;

#[path = "integration/test_mcp_schema.rs"]
mod test_mcp_schema;

#[path = "integration/embedding_model_comparison.rs"]
mod embedding_model_comparison;
