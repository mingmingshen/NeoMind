# NeoMind Testing Strategy

## Test Organization

```
crates/
├── neomind-core/tests/           # Core system tests (unit, integration, e2e)
├── neomind-extension-runner/tests/  # Runner IPC tests
├── neomind-extension-sdk/tests/     # SDK unit tests
├── neomind-testing/               # Testing utilities and fixtures
│   └── smoke-extension/           # Smoke test extension
└── */tests/                       # Per-crate integration tests
```

## Test Categories

### 1. Unit Tests
- Located: Within each crate's `src/` in `#[cfg(test)] mod tests`
- Purpose: Test individual functions and types
- Speed: Fast, no external dependencies

### 2. Integration Tests
- Located: `crates/*/tests/` directories
- Purpose: Test interactions between components
- Speed: Medium, may require setup

### 3. E2E Tests
- Located: `crates/neomind-core/tests/*_e2e_test.rs`
- Purpose: Test complete workflows
- Speed: Slower, full system integration

### 4. Smoke Tests
- Located: `crates/neomind-testing/smoke-extension/`
- Purpose: Verify basic system functionality
- Usage: Loaded at runtime to test extension loading and IPC

## Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p neomind-core
cargo test -p neomind-extension-sdk
cargo test -p neomind-extension-runner

# Run with verbose output
cargo test --workspace -- --nocapture

# Run specific test
cargo test -p neomind-core test_extension_lifecycle

# Run E2E tests only
cargo test -p neomind-core --test "*_e2e_test"
```

## Test Naming Conventions

```
test_<component>_<scenario>_<expected_result>
```

Examples:
- `test_extension_load_success`
- `test_ipc_message_timeout_fails`
- `test_capability_unauthorized_denied`

## ABI Isolation Testing

The smoke extension (`crates/neomind-testing/smoke-extension/`) is used to verify:
1. Extensions can be loaded without `neomind-core` dependency
2. IPC protocol is stable across versions
3. Capability system works end-to-end

## Continuous Integration

Tests run on every PR:
- Unit and integration tests: Required
- E2E tests: Required
- Smoke tests: Required

## Adding New Tests

1. **Unit tests**: Add to `#[cfg(test)] mod tests` in source file
2. **Integration tests**: Add new file to `tests/` directory
3. **E2E tests**: Add to `neomind-core/tests/` with `_e2e_test` suffix

## Test Fixtures

Shared test data and mocks should be placed in:
- `crates/neomind-testing/fixtures/` (to be created as needed)
- Inline fixtures within test files for simple cases
