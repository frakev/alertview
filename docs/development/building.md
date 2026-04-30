# Building AlertView

This guide covers building AlertView from source for development and production use.

## Prerequisites

### Rust Installation

AlertView is written in Rust and requires Rust 1.75 or later.

#### Install Rust

The easiest way to install Rust is using `rustup`:

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the instructions to add rustup to your PATH
source $HOME/.cargo/env
```

#### Verify Installation

```bash
# Check Rust version
rustc --version

# Check Cargo version (Rust's package manager)
cargo --version

# Check rustup version
rustup --version
```

#### Update Rust

```bash
# Update to the latest stable version
rustup update stable

# Check for updates
rustup check
```

### System Dependencies

#### Linux (Ubuntu/Debian)

```bash
# Install required system libraries
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    git
```

#### Linux (Fedora/RHEL/CentOS)

```bash
sudo dnf install -y \
    gcc \
    gcc-c++ \
    make \
    pkg-config \
    openssl-devel \
    git
```

#### macOS

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew (if not already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install required packages
brew install pkg-config openssl
```

#### Windows

1. Install [Visual Studio 2022](https://visualstudio.microsoft.com/) with the "Desktop development with C++" workload
2. Install [Git for Windows](https://git-scm.com/download/win)
3. Install Rust using rustup as shown above

### Additional Tools (Optional)

```bash
# Install clippy (Rust linter)
rustup component add clippy

# Install rustfmt (Rust formatter)
rustup component add rustfmt

# Install cargo-audit (security auditing)
cargo install cargo-audit

# Install cargo-edit (for managing dependencies)
cargo install cargo-edit
```

## Cloning the Repository

```bash
# Clone the repository
git clone https://github.com/your-org/alertview.git
cd alertview

# Check out a specific version (optional)
git checkout v1.0.0
```

## Building

### Debug Build

For development, use a debug build which includes debug symbols and has faster compilation:

```bash
cargo build
```

This creates a debug binary at `target/debug/alertview`.

**Debug build characteristics:**
- Includes debug symbols for better error messages
- No optimizations (slower execution)
- Faster to compile
- Larger binary size

### Release Build

For production, use a release build which is optimized:

```bash
cargo build --release
```

This creates an optimized binary at `target/release/alertview`.

**Release build characteristics:**
- Optimized for performance
- Smaller binary size
- Slower to compile
- No debug symbols (unless configured otherwise)

### Build with All Features

AlertView has optional features that can be enabled:

```bash
# Build with all features
cargo build --all-features

# Build with specific features
cargo build --features gzip,json-logs
```

### Build for a Specific Target

#### Cross-Compilation

Rust supports cross-compilation to different platforms:

```bash
# List available targets
rustup target list

# Add a target (e.g., ARM64)
rustup target add aarch64-unknown-linux-gnu

# Build for ARM64
cargo build --target aarch64-unknown-linux-gnu --release
```

#### Common Targets

| Platform | Target |
|----------|--------|
| Linux x86_64 | x86_64-unknown-linux-gnu |
| Linux ARM64 | aarch64-unknown-linux-gnu |
| Linux ARM | arm-unknown-linux-gnueabi |
| macOS x86_64 | x86_64-apple-darwin |
| macOS ARM64 | aarch64-apple-darwin |
| Windows x86_64 | x86_64-pc-windows-msvc |
| Windows ARM64 | aarch64-pc-windows-msvc |

#### Cross-Compilation Setup

For cross-compilation, you need to install the appropriate toolchain:

```bash
# For Linux ARM64
rustup target add aarch64-unknown-linux-gnu
sudo apt-get install gcc-aarch64-linux-gnu

# For Windows from Linux
rustup target add x86_64-pc-windows-gnu
sudo apt-get install mingw-w64

# For macOS from Linux (requires macOS cross-compilation tools)
rustup target add x86_64-apple-darwin
```

## Running

### Run in Development Mode

```bash
# Run with debug build
cargo run

# Run with release build
cargo run --release
```

### Run with Custom Configuration

```bash
# Specify a custom config file
cargo run -- --config /path/to/config.yaml

# Or use environment variables
ALERTVIEW_PORT=9090 ALERTVIEW_LOG_FORMAT=json cargo run
```

### Run with Logging

```bash
# Set log level to debug
RUST_LOG=debug cargo run

# Set log level to trace (maximum verbosity)
RUST_LOG=trace cargo run

# Set log level to info (default)
RUST_LOG=info cargo run
```

### Command Line Arguments

AlertView supports the following command line arguments:

```bash
# Show help
cargo run -- --help
# or
cargo run -- -h

# Specify config file
cargo run -- /path/to/config.yaml

# Specify port via environment variable
ALERTVIEW_PORT=9090 cargo run
```

**Available Options:**
- `-h, --help` - Show help message and exit
- `CONFIG_FILE` - Path to the configuration file (default: `config.yaml`)

**Environment Variables:**
- `ALERTVIEW_CONFIG` - Path to the configuration file
- `ALERTVIEW_PORT` - Port to listen on (default: 8080)
- `ALERTVIEW_LOG_FORMAT` - Log format: `text` or `json` (default: text)
- `RUST_LOG` - Log level: `error`, `warn`, `info`, `debug`, `trace`

## Docker Build

### Build Docker Image

```bash
# Build the Docker image
docker build -t alertview:latest .

# Build with a specific version
docker build -t alertview:v1.0.0 .
```

### Build for Multiple Architectures

```bash
# Create a builder for multi-arch builds
docker buildx create --use

# Build for AMD64 and ARM64
docker buildx build \
    --platform linux/amd64,linux/arm64 \
    -t alertview:latest \
    --push .
```

### Docker Build Arguments

```dockerfile
# In Dockerfile
ARG VERSION=latest
ARG TARGETARCH

# Build with arguments
docker build \
    --build-arg VERSION=v1.0.0 \
    -t alertview:v1.0.0 .
```

## Installation

### Install System-Wide

```bash
# Build release
cargo build --release

# Install to /usr/local/bin
sudo cp target/release/alertview /usr/local/bin/

# Verify installation
alertview --version
```

### Install with Cargo

```bash
# Install from the current directory
cargo install --path .

# Install from GitHub
cargo install --git https://github.com/your-org/alertview.git

# Install a specific version
cargo install --git https://github.com/your-org/alertview.git --tag v1.0.0
```

### Create a Debian Package

1. Install `cargo-deb`:

```bash
cargo install cargo-deb
```

2. Add deb configuration to `Cargo.toml`:

```toml
[package.metadata.deb]
maintainer = "Your Name <your@email.com>"
copyright = "2024 Your Organization"
license-file = ["LICENSE"]
depends = "$auto"
section = "net"
priority = "optional"
assets = [
    ["target/release/alertview", "usr/bin/", "755"],
    ["config.example", "etc/alertview/", "644"],
    ["README.md", "usr/share/doc/alertview/", "644"],
]
```

3. Build the package:

```bash
cargo deb
```

4. Install the package:

```bash
sudo dpkg -i target/debian/alertview_*.deb
```

### Create a Systemd Service

1. Create a service file at `/etc/systemd/system/alertview.service`:

```ini
[Unit]
Description=AlertView Alert Dashboard
After=network.target

[Service]
Type=simple
User=alertview
Group=alertview
ExecStart=/usr/local/bin/alertview --config /etc/alertview/config.yaml
Restart=on-failure
RestartSec=5s

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/etc/alertview /var/lib/alertview

[Install]
WantedBy=multi-user.target
```

2. Create the alertview user:

```bash
sudo useradd --system --no-create-home --shell /sbin/nologin alertview
```

3. Enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable alertview
sudo systemctl start alertview
sudo systemctl status alertview
```

## Build Optimization

### Incremental Builds

Cargo uses incremental compilation by default. For even faster builds:

```bash
# Use all available CPU cores
cargo build -j $(nproc)

# Or specify the number of jobs
cargo build -j 8
```

### Cargo Cache

Cargo caches dependencies and build artifacts. To manage the cache:

```bash
# Clean the cargo cache (frees disk space)
cargo cache clean

# Or remove specific packages
rm -rf ~/.cargo/registry/cache/
```

### Profile-Guided Optimization (PGO)

For maximum performance, use PGO:

1. Build with instrumentation:

```bash
RUSTFLAGS="-C profile-generate=/tmp/pgo" cargo build --release
```

2. Run the instrumented binary with representative workload:

```bash
./target/release/alertview --config your-config.yaml
# Let it run for a while to collect profile data
```

3. Build with optimization using the profile:

```bash
RUSTFLAGS="-C profile-use=/tmp/pgo" cargo build --release
```

### Link-Time Optimization (LTO)

Enable LTO for additional optimizations:

```bash
# In Cargo.toml
[profile.release]
lto = true
codegen-units = 1
```

Or via environment variable:

```bash
RUSTFLAGS="-C lto -C codegen-units=1" cargo build --release
```

**Note:** LTO increases build time but can improve runtime performance.

## Troubleshooting

### Build Errors

#### Missing Dependencies

```
error: linker `cc` not found
```

Install the C compiler and development tools:

```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# Fedora/RHEL
sudo dnf install gcc gcc-c++ make

# macOS
xcode-select --install
```

#### SSL Errors

```
error: failed to run custom build command for `openssl-sys v0.9.x`
```

Install OpenSSL development libraries:

```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev

# Fedora/RHEL
sudo dnf install openssl-devel

# macOS (with Homebrew)
brew install openssl
```

#### Permission Errors

```
error: could not create directory `/usr/local/cargo`
```

Run with appropriate permissions or install to a user directory:

```bash
# Option 1: Run as root (not recommended)
sudo cargo install --path .

# Option 2: Install to user directory
export CARGO_HOME=$HOME/.cargo
cargo install --path .
```

### Linker Errors

#### Missing Libraries

```
error: linking with `cc` failed: exit code: 1
```

Install required system libraries:

```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# Fedora/RHEL
sudo dnf install openssl-devel pkg-config
```

#### 32-bit vs 64-bit Issues

Ensure you're building for the correct architecture:

```bash
# Check your system architecture
uname -m

# Build for the correct target
cargo build --target x86_64-unknown-linux-gnu
```

### Compilation Timeouts

For large projects or slow machines, compilation might timeout:

```bash
# Increase the timeout
export CARGO_TERM_VERBOSE=true

# Or use a more powerful machine
```

### Disk Space Issues

Cargo can use significant disk space. To clean up:

```bash
# Remove old build artifacts
cargo clean

# Remove the entire target directory
rm -rf target/

# Clean cargo cache
cargo cache clean

# Remove old registry data
rm -rf ~/.cargo/registry/
```

## Build Verification

### Check Binary

```bash
# Check if the binary was built
ls -lh target/release/alertview

# Check binary information
file target/release/alertview

# Check dependencies (Linux)
ldd target/release/alertview
```

### Run Tests

```bash
# Run all tests
cargo test

# Run tests with verbose output
cargo test -- --nocapture

# Run specific tests
cargo test test_name

# Run tests for a specific module
cargo test --lib
cargo test --bin alertview
```

### Run Linter

```bash
# Run clippy
cargo clippy

# Run clippy with all lints
cargo clippy --all-targets --all-features -- -D warnings

# Fix clippy warnings
cargo clippy --fix
```

### Check Formatting

```bash
# Check code formatting
cargo fmt --check

# Format code
cargo fmt
```

### Security Audit

```bash
# Audit dependencies for vulnerabilities
cargo audit

# Update dependencies and check for new vulnerabilities
cargo update
cargo audit
```

## Continuous Integration

### Local CI Testing

Before pushing changes, run the same checks that CI will run:

```bash
#!/bin/bash
set -e

echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "Running tests..."
cargo test --all-features

echo "Checking format..."
cargo fmt --check

echo "Running audit..."
cargo audit

echo "All checks passed!"
```

### GitHub Actions

The project uses GitHub Actions for CI/CD. The workflow file is at `.github/workflows/ci.yml`.

To test locally using [act](https://github.com/nektos/act):

```bash
# Install act
brew install act  # macOS
# or
curl -s https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# Run the workflow locally
act -j build
```

## Build Artifacts

### What Gets Built

| Command | Output | Purpose |
|---------|--------|---------|
| `cargo build` | `target/debug/alertview` | Development |
| `cargo build --release` | `target/release/alertview` | Production |
| `cargo test` | `target/debug/deps/*` | Testing |
| `cargo doc` | `target/doc/` | Documentation |

### Artifact Sizes

| Build Type | Approximate Size | Notes |
|------------|------------------|-------|
| Debug | 20-50 MB | With debug symbols |
| Release | 5-15 MB | Optimized, stripped |
| Release + LTO | 5-15 MB | Slightly larger, better optimized |

### Stripping Symbols

To further reduce binary size:

```bash
# Strip debug symbols from release build
strip target/release/alertview

# Or use upx to compress the binary
upx --best target/release/alertview
```

**Note:** Stripping removes debug information, making debugging harder. Only do this for production builds.

## Version Management

### Update Version

1. Update the version in `Cargo.toml`:

```toml
[package]
name = "alertview"
version = "1.0.0"  # Update this
```

2. Update the version in any other files that reference it
3. Update `CHANGELOG.md` with the new version and changes

### Semantic Versioning

AlertView follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes, incompatible API changes
- **MINOR**: New features, backward-compatible changes
- **PATCH**: Bug fixes, backward-compatible changes

### Git Tags

Create annotated tags for releases:

```bash
# Create an annotated tag
git tag -a v1.0.0 -m "Release v1.0.0"

# Push the tag to GitHub
git push origin v1.0.0
```

## Best Practices

1. **Always build with `--release` for production** - Debug builds are slower and larger
2. **Test your build** - Run the binary and verify it works as expected
3. **Clean up old builds** - Regularly run `cargo clean` to free disk space
4. **Use consistent Rust version** - Specify the Rust version in `rust-toolchain` file
5. **Document build steps** - Keep this documentation updated
6. **Automate builds** - Use CI/CD to automatically build and test
7. **Sign your releases** - Consider signing release binaries for verification

## Additional Resources

- [Rust Installation Guide](https://www.rust-lang.org/tools/install)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Rust Cross-Compilation Guide](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [Docker Multi-Arch Build Guide](https://docs.docker.com/build/building/multi-platform/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
