# SOM Chatbot - Claude Reference Guide

## Build & Run Commands
- Build: `cargo build`
- Run: `cargo run`
- Release build: `cargo build --release`
- Check: `cargo check`
- Lint (basic): `cargo clippy`
- Lint (strict): `cargo lint`
- Auto-fix lint issues: `cargo fix`
- Format: `cargo fmt` or `cargo format-all`
- Test all: `cargo test`
- Test specific: `cargo test test_name`
- Test with output: `cargo test -- --nocapture`
- Documentation: `cargo doc --open`

## Development Workflow
- Start with unit tests for new features (TDD approach)
- Run `cargo check` after completing any feature
- Format code with `cargo fmt` before committing
- Run `cargo lint` to catch potential issues
- Run `cargo test` to verify functionality
- Document with RustDoc (///) for all public functions, structs, etc.
- Use in-line comments (//) for important implementation details

## Code Style Guidelines
- **Formatting**: Follow project rustfmt.toml rules with `cargo fmt`
- **Line Length**: Keep lines under 100 characters
- **Naming**: Use snake_case for variables, functions; CamelCase for types
- **Imports**: Group imports by std, external crates, and local modules
- **Error Handling**: Use Result<T, E> with ? operator for propagation, avoid unwrap()/expect()
- **Testing**: Write unit tests for every public function
- **Types**: Prefer strong typing; avoid `impl Trait` in public APIs
- **Modules**: Organize code in logical modules with clear responsibilities
- **Documentation**: Include examples in documentation for public APIs
- **Comments**: Keep comments up-to-date with code changes

## Linting Guidelines
- Run `cargo lint` to apply strict checks
- Address all clippy warnings before submitting PRs
- Use `#[allow(clippy::some_lint)]` sparingly and with justification
- Run `cargo fix` to automatically apply suggested fixes
- For intentional uses of unwrap/expect, use `#[allow(clippy::unwrap_used)]` with a comment explaining why

## CI Workflow
- GitHub Actions checks on every PR:
  - Code formatting with rustfmt
  - Linting with clippy
  - All tests pass
  - Successful build
- All CI checks must pass before merging