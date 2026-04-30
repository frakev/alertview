# Development Guide

This section covers everything you need to know to develop, build, test, and contribute to AlertView.

## Getting Started

- **[Structure](structure.md)** - Project structure and organization
- **[Building](building.md)** - How to build AlertView from source
- **[Testing](testing.md)** - Running tests and writing new ones
- **[Contributing](contributing.md)** - How to contribute to the project

## Reference

- **[API Documentation](../../api.md)** - REST API reference
- **[Configuration](../../configuration/config-file.md)** - Configuration options

## Quick Start for Developers

1. **Clone the repository:**
   ```bash
   git clone https://github.com/your-org/alertview.git
   cd alertview
   ```

2. **Install Rust:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

3. **Build and run:**
   ```bash
   cargo run
   ```

4. **Run tests:**
   ```bash
   cargo test
   ```

## Development Workflow

1. Create a feature branch: `git checkout -b feature/my-feature`
2. Make your changes
3. Run tests: `cargo test`
4. Run clippy: `cargo clippy`
5. Format code: `cargo fmt`
6. Commit your changes
7. Push to GitHub and create a Pull Request

## Useful Commands

| Command | Description |
|---------|-------------|
| `cargo build` | Build the project |
| `cargo run` | Build and run |
| `cargo check` | Check for compilation errors |
| `cargo test` | Run all tests |
| `cargo test -- --nocapture` | Run tests with output |
| `cargo clippy` | Run the linter |
| `cargo fmt` | Format code |
| `cargo doc --open` | Generate and open documentation |
| `cargo audit` | Check for vulnerabilities |
| `cargo update` | Update dependencies |

## Project Structure

```
alertview/
├── src/                  # Rust source code
│   ├── main.rs           # Main application entry point
│   ├── config.rs         # Configuration loading and parsing
│   ├── alerts.rs         # Alert fetching and processing
│   ├── cache.rs          # Caching implementation
│   └── ...
├── static/               # Static files (HTML, CSS, JS)
│   ├── index.html        # Main HTML page
│   ├── app.js            # Frontend JavaScript
│   └── style.css         # Stylesheet
├── docs/                 # Documentation
├── 01-namespace.yaml    # Kubernetes namespace manifest
├── 02-configmap.yaml    # Kubernetes ConfigMap manifest
├── 03-deployment.yaml   # Kubernetes Deployment manifest
├── 04-service.yaml      # Kubernetes Service manifest
├── 05-ingress.yaml      # Kubernetes Ingress manifest
├── Cargo.toml            # Rust package manifest
├── Cargo.lock            # Dependency lock file
├── config.example        # Example configuration
└── README.md             # Project README
```

## Coding Standards

- Follow Rust's standard style guidelines
- Use `cargo fmt` to format code
- Use `cargo clippy` to catch common issues
- Write tests for new functionality
- Keep commits atomic and well-described
- Use descriptive variable and function names
- Add comments for complex logic

## Debugging

### Logging

AlertView uses the `tracing` crate for logging. Set the `RUST_LOG` environment variable to control log level:

```bash
# Debug level logging
RUST_LOG=debug cargo run

# Trace level for maximum detail
RUST_LOG=trace cargo run

# Only show errors
RUST_LOG=error cargo run
```

### Debugging with VS Code

Create a `.vscode/launch.json` file:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug AlertView",
      "program": "${workspaceFolder}/target/debug/alertview",
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

### Debugging with GDB

```bash
# Build with debug symbols
cargo build

# Run with gdb
gdb -ex run --args ./target/debug/alertview
```

## Performance Profiling

### Using perf

```bash
# Build with debug symbols
cargo build --release

# Profile with perf
perf record -g ./target/release/alertview
perf report
```

### Using flamegraph

```bash
# Install flamegraph
cargo install flamegraph

# Profile and generate flamegraph
cargo flamegraph --bench my_benchmark
```

## Dependency Management

Dependencies are managed in `Cargo.toml`. To add a new dependency:

1. Add to `Cargo.toml`:
   ```toml
   [dependencies]
   new_crate = "1.0.0"
   ```

2. Update the lock file:
   ```bash
   cargo update
   ```

3. Rebuild:
   ```bash
   cargo build
   ```

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create Git tag:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```
4. Build release:
   ```bash
   cargo build --release
   ```
5. Create GitHub release with binaries

## Continuous Integration

The project uses GitHub Actions for CI/CD. Workflows include:
- Building and testing on push and pull requests
- Linting with clippy
- Running all tests
- Building release binaries

See `.github/workflows/` for the CI configuration.
