# SOM Chatbot - Claude Reference Guide

## Build & Run Commands
- Build: `cargo build`
- Run: `cargo run`
- Release build: `cargo build --release`
- Check: `cargo check`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- Test all: `cargo test`
- Test specific: `cargo test test_name`
- Test with output: `cargo test -- --nocapture`
- Documentation: `cargo doc --open`

## Development Workflow
- Start with unit tests for new features (TDD approach)
- Run `cargo check` after completing any feature
- Run `cargo test` to verify functionality
- Document with RustDoc (///) for all functions, structs, etc.
- Use in-line comments (//) for important implementation details

## Code Style Guidelines
- **Formatting**: Follow Rust standard formatting with `cargo fmt`
- **Naming**: Use snake_case for variables, functions; CamelCase for types
- **Imports**: Group imports by std, external crates, and local modules
- **Error Handling**: Use Result<T, E> with ? operator for propagation
- **Testing**: Write unit tests for every possible function
- **Types**: Prefer strong typing; avoid `impl Trait` in public APIs
- **Modules**: Organize code in logical modules with clear responsibilities