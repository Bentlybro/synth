# Synth Test Suite

Integration and unit tests for Synth.

## Test Files

### test_integration.sh

Comprehensive integration test suite covering all major features:

- Health check
- Basic search (Quick mode)
- Deep search with query expansion
- Code repository analysis (basic mode)
- Code repository analysis (deep mode)  
- Semantic search embedding storage

**Usage:**
```bash
./test_integration.sh
```

### test_code_repo.sh

Focused tests for GitHub repository extraction:

- Basic repository extraction
- Repository analysis with query
- Large repository handling

**Usage:**
```bash
./test_code_repo.sh
```

### test_extractors.rs

Rust example demonstrating content type detection.

**Usage:**
```bash
cargo run --example test_extractors
```

## Running Tests

```bash
# Run all shell-based tests
cd tests
./test_integration.sh
./test_code_repo.sh

# Run Rust tests
cargo test

# Run Rust examples
cargo run --example test_extractors
```

## Notes

- Ensure Synth service is running on http://localhost:8765 before running tests
- Tests use live API endpoints and will make real requests
- Some tests require API keys to be configured (ANTHROPIC_API_KEY, OPENAI_API_KEY)
