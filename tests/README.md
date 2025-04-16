# Tests for conda-env-inspect

This directory contains tests for the conda-env-inspect project.

## Test Files

- `models_test.rs`: Tests for the core data models used in the project
- `parsers_test.rs`: Tests for parsing environment and conda-lock files
- `analysis_test.rs`: Tests for environment analysis logic 
- `cli_integration_test.rs`: Integration tests for CLI functionality

## Running Tests

To run all tests:

```
cargo test
```

To run a specific test:

```
cargo test <test_name>
```

## Adding New Tests

When adding new tests, please follow these guidelines:

1. Create separate test files for different modules
2. Use descriptive test function names with the `test_` prefix
3. Use temporary directories for file-based tests
4. Mock external dependencies where possible

## Test Data

The tests use sample environment files created in the tests themselves. There are also example environment files in the `/examples` directory that you can use for manual testing. 