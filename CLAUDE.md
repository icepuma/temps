# Claude Development Guidelines for temps

This document provides guidelines for Claude when working on the temps codebase.

## Project Structure

This is a Rust workspace project with four crates:
- `temps-core` - Core functionality without external dependencies (except nom)
- `temps-chrono` - Chrono integration for time operations
- `temps-jiff` - Jiff integration for time operations
- `temps` - Main crate that re-exports functionality from the sub-crates

## Development Workflow

### After Completing Any Task

**ALWAYS run `./check.sh` after making changes** to ensure:
- Code is properly formatted
- All clippy warnings are addressed
- All tests pass
- The workspace builds successfully

```bash
cd /Users/icepuma/development/temps && ./check.sh
```

**IMPORTANT**: Only use `./check.sh` to run tests. Do not use `cargo test` directly.

### If check.sh Reports Issues

1. **Formatting issues**: The script runs `cargo fmt --all` automatically
2. **Clippy warnings**: Fix all warnings before considering the task complete
3. **Test failures**: Debug and fix failing tests
4. **Build errors**: Resolve compilation issues

### Common Issues and Fixes

#### Unused imports/variables
- Remove unused imports
- Prefix unused variables with underscore (e.g., `_unused_var`)

#### Method naming conventions
- Methods starting with `from_`, `to_`, `as_`, `into_` should follow Rust conventions
- Consider renaming methods that trigger clippy's `wrong_self_convention` warning

#### Parser ordering issues
- In nom parsers, order alternatives from most specific to least specific
- Longer strings should come before shorter ones (e.g., "an" before "a", "einem" before "ein")

## Testing Guidelines

### Running Tests
**Always use `./check.sh` to run tests.** This ensures:
- Code is formatted before testing
- Clippy checks are run
- Tests are run with nextest for better output
- All features are properly enabled

### Adding Tests
- Place integration tests in the `tests/` directory of each crate
- Use descriptive test names that explain what is being tested
- Clean up test files by removing unused imports and variables

## Code Quality Standards

1. **No approximations**: Use proper date arithmetic methods (e.g., `checked_add_months` for months/years)
2. **Error handling**: Use proper error types and messages
3. **Documentation**: Add doc comments for public APIs
4. **Feature flags**: Respect feature boundaries - chrono-specific code only when chrono feature is enabled

## The check.sh Script

The `check.sh` script performs the following in order:
1. `cargo fmt --all` - Formats all code
2. `cargo clippy --workspace --tests --all-features --all-targets` - Runs clippy checks
3. `cargo nextest run --workspace --all-features` - Runs all tests with nextest

**Always use this script instead of running individual commands.**

## Important Notes

- This is a library project - avoid creating unnecessary binaries or examples
- The workspace uses resolver version 3
- All crates share workspace-level package metadata
- Always verify changes work by running `./check.sh`
- Never commit code without running `./check.sh` first