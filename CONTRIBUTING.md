# Contributing to mdns-reflector

Thank you for your interest in contributing to mdns-reflector! This document provides guidelines and information for contributors.

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo (comes with Rust)
- Git

### Building from Source

```bash
git clone https://github.com/jdfalk/mdns-reflector
cd mdns-reflector
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Lints

```bash
cargo clippy -- -D warnings
cargo fmt --check
```

## Code Style

- Follow the official Rust style guidelines
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix any warnings
- Write tests for new functionality
- Add documentation comments for public APIs

## Project Structure

```
mdns-reflector/
├── src/
│   ├── cli.rs         # Command-line interface
│   ├── config.rs      # Configuration parsing
│   ├── mdns.rs        # mDNS protocol handling
│   ├── network.rs     # Network interface management
│   ├── reflector.rs   # Core reflection engine
│   ├── lib.rs         # Library entry point
│   └── main.rs        # Binary entry point
├── Cargo.toml         # Dependencies and metadata
├── README.md          # User documentation
├── CONTRIBUTING.md    # This file
└── LICENSE            # Apache 2.0 license
```

## Security

Security is a top priority for mdns-reflector. When contributing:

- **Never introduce unsafe code without thorough justification and review**
- Validate all inputs from network packets
- Use Rust's type system to prevent errors at compile time
- Avoid panics in production code - use `Result` for error handling
- Be mindful of resource consumption (memory, CPU, network)
- Consider DOS attack vectors in network code

### Reporting Security Issues

Please do not report security vulnerabilities through public GitHub issues. Instead, email the maintainers directly.

## Pull Request Process

1. **Fork the repository** and create your branch from `main`
2. **Make your changes** following the code style guidelines
3. **Add tests** for new functionality
4. **Run the test suite** and ensure all tests pass
5. **Run lints** and fix any warnings
6. **Update documentation** if you've changed APIs or added features
7. **Write a clear commit message** describing your changes
8. **Submit a pull request** with a detailed description

### Commit Message Guidelines

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests liberally after the first line

Example:
```
Add MAC address filtering support

Implements per-device MAC address filtering as specified in #123.
Includes configuration parsing, packet filtering logic, and tests.

Fixes #123
```

## Feature Requests

Feature requests are welcome! Before submitting a feature request:

1. Check if the feature has already been requested
2. Clearly describe the use case and benefits
3. Consider whether the feature aligns with the project's goals
4. Be open to discussion and alternative solutions

## Testing

### Unit Tests

Run unit tests with:
```bash
cargo test
```

### Integration Tests

For network functionality, integration tests require special setup:
```bash
# May require root/admin privileges for network operations
sudo cargo test -- --ignored
```

### Manual Testing

When testing network functionality manually:

1. Use a test environment with virtual interfaces or VMs
2. Test with both IPv4 and IPv6
3. Verify loop prevention works
4. Test rate limiting under load
5. Check resource usage (CPU, memory, network)

## Documentation

- Add documentation comments (`///`) for all public items
- Include examples in documentation when helpful
- Update README.md for user-facing changes
- Keep CHANGELOG.md up to date (if present)

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.

## Questions?

Feel free to open an issue for questions about contributing or development.
