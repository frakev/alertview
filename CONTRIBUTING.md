# Contributing to AlertView

Thank you for your interest in contributing to AlertView! We welcome contributions from everyone.

## Ways to Contribute

- **Bug Reports**: Open an issue describing the problem
- **Feature Requests**: Open an issue describing your use case
- **Code Contributions**: Submit pull requests
- **Documentation**: Improve existing docs or add new guides
- **Tests**: Add test cases for better coverage

## Development Setup

### Prerequisites

- Rust 1.75+
- Cargo (comes with Rust)
- Node.js (optional, for frontend development)

### Getting Started

```bash
# Clone the repository
git clone https://github.com/frakev/alertview.git
cd alertview

# Build the project
cargo build

# Run tests
cargo test

# Run the application
cargo run -- config.example
```

### Code Style

- Follow Rust best practices
- Use `cargo clippy` to check for linting issues
- Write tests for new features
- Keep functions focused and well-documented

### Commit Messages

Use conventional commit format:

- `feat: add new feature`
- `fix: fix bug`
- `docs: update documentation`
- `refactor: code refactoring`
- `test: add tests`
- `chore: maintenance tasks`

### Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Commit your changes
4. Push to your fork
5. Open a pull request against the main branch
6. Wait for review and address feedback

## Reporting Issues

When reporting issues, please include:

- Version of AlertView
- Configuration file (remove sensitive data)
- Steps to reproduce
- Expected vs actual behavior
- Logs if applicable

## License

By contributing to AlertView, you agree that your contributions will be licensed under the MIT License.