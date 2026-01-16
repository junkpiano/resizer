# AGENTS.md - Coding Guidelines for the Resizer Project

This document provides coding guidelines and operational instructions for AI agents working on the Resizer codebase. The Resizer is a CLI tool written in Rust that compresses images to fit under a target size by adjusting quality and downscaling.

## Build/Lint/Test Commands

### Building
- **Full release build**: `cargo build --release`
- **Debug build**: `cargo build`
- **Check compilation without building**: `cargo check`

### Testing
- **Run all tests**: `cargo test`
- **Run specific test**: `cargo test <test_name>`
- **Run tests with output**: `cargo test -- --nocapture`
- **Run benchmark tests**: `cargo bench` (if benchmarks exist)

### Linting and Formatting
- **Lint with clippy**: `cargo clippy`
- **Auto-fix clippy issues**: `cargo clippy --fix`
- **Format code**: `cargo fmt`
- **Check formatting**: `cargo fmt --check`

### Running the Application
- **Run in debug mode**: `cargo run -- [args]`
- **Run release binary**: `./target/release/resizer [args]`

## Code Style Guidelines

### Imports and Dependencies
- Group imports by external crates first, then standard library
- Use selective imports with `{}` syntax: `use std::{fs, io::Write};`
- One import per line for clarity
- Prefer qualified imports for disambiguation: `use image::codecs::jpeg::JpegEncoder;`

### Naming Conventions
- **Functions**: `snake_case` (e.g., `fit_quality`, `encode`, `apply_max_dimensions`)
- **Structs/Enums**: `PascalCase` (e.g., `Args`, `OutFormat`)
- **Variables/Fields**: `snake_case` (e.g., `target_bytes`, `max_quality`)
- **Constants**: `UPPER_CASE` (though none currently used)
- **Modules**: `snake_case`

### Error Handling
- Use `anyhow::Result<T>` as the return type for functions that can fail
- Use `anyhow::Context` for adding context to errors: `.context("descriptive message")?`
- Use `bail!` macro for early returns with error messages
- Prefer descriptive error messages that help users understand what went wrong
- Use `eprintln!` for progress/status output to stderr

### Code Structure and Patterns
- Use `clap` derive macros for CLI argument parsing
- Implement `ValueEnum` for enum arguments
- Use binary search algorithms for optimization problems (like quality fitting)
- Handle different image formats (JPEG/WebP/PNG) with appropriate encoding strategies
- Use early returns and guard clauses to reduce nesting
- Prefer immutable variables; use `mut` only when necessary
- Use pattern matching extensively, especially for enums and Options

### Documentation and Comments
- Use `///` for public API documentation (currently minimal in this codebase)
- Use `//` for implementation comments explaining complex logic
- Document struct fields with `#[arg(...)]` attributes for clap
- Add context to complex algorithms or edge cases

### Type Safety and Performance
- Use strong typing with explicit types where helpful
- Use `Option<T>` for optional parameters/values
- Use `Result<T, E>` for operations that can fail
- Prefer stack allocation; use heap allocation (`Vec`, `String`) judiciously
- Use `u64` for byte counts and sizes to handle large files
- Use `f32` for calculations involving image scaling/resizing

### Testing
- No tests currently exist; follow Rust testing conventions when adding them
- Place unit tests in the same file as the code being tested using `#[cfg(test)]` modules
- Use descriptive test function names: `#[test] fn test_quality_fitting_with_small_target()`
- Test edge cases: empty images, very large images, invalid inputs

### File Organization
- Single main.rs file (small project)
- Keep functions focused and under 50 lines when possible
- Group related functionality together (encoding functions, image processing functions)
- Use meaningful function names that describe what they do

### Code Quality Standards
- All code must compile without warnings: `cargo check`
- All code must pass clippy lints: `cargo clippy`
- All code must be formatted: `cargo fmt --check`
- Prefer readable code over clever optimizations
- Use meaningful variable names; avoid single-letter variables except in tight loops
- Handle all error cases appropriately; don't ignore potential failures

### Dependencies
- **Core**: `anyhow` for error handling, `clap` for CLI parsing
- **Image processing**: `image` crate for image manipulation, `webp` crate for WebP encoding
- Keep dependencies minimal and well-maintained
- Use specific version requirements in Cargo.toml

### Security Considerations
- Validate all input parameters (file paths, numeric ranges)
- Handle file I/O safely with proper error checking
- Avoid buffer overflows by using safe Rust abstractions
- Be cautious with large image files; implement size limits if needed

### Performance Guidelines
- Use efficient algorithms (binary search for quality fitting)
- Pre-downscale very large images to avoid unnecessary computation
- Use appropriate filter types for image resizing (`FilterType::Lanczos3`)
- Minimize allocations in hot paths
- Profile performance-critical sections if optimization is needed

## Development Workflow

1. Make changes to code
2. Run `cargo check` to verify compilation
3. Run `cargo clippy` to check for issues
4. Run `cargo fmt` to format code
5. Test changes manually or add unit tests
6. Commit with descriptive messages

## Project-Specific Patterns

- **Quality fitting**: Use binary search between min/max quality values
- **Downscaling**: Reduce dimensions by 10% increments when quality fitting fails
- **Format-specific handling**: Different strategies for lossy (JPEG/WebP) vs lossless (PNG) formats
- **Progress reporting**: Use `eprintln!` with consistent formatting for user feedback
- **Dimension constraints**: Apply max width/height before compression
- **Alpha channel handling**: Preserve transparency in WebP/PNG, convert appropriately for JPEG</content>
