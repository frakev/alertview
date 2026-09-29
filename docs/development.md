# Development

## Layout

```
src/main.rs        server, background poller, API, Content-Security-Policy
src/config.rs      configuration: parsing, validation, unknown-key messages
src/alerts.rs      source clients (Alertmanager, Grafana, Zabbix), links, grouping
static/            the web page — embedded in the binary at build time
tests/frontend/    frontend tests (Node)
0*-*.yaml          Kubernetes manifests
```

There is no bundler: `static/app.js` is served as written. Changing a file in
`static/` needs a rebuild, since it is embedded in the binary.

## Build and run

Needs a stable Rust toolchain ([rustup](https://rustup.rs)).

```bash
cargo run -- config.example       # http://localhost:8080
cargo build --release             # target/release/alertview
docker build -t alertview .
```

The version shown in the page and in `--version` is the crate version, or
`APP_VERSION` when set at build time (CI sets it from the git tag).

## Tests

Two suites, both run by CI:

```bash
cargo test                          # Rust
node tests/frontend/render.test.js  # frontend, Node only, no npm install
```

**Rust tests** sit next to the code, in `#[cfg(test)]` modules. There is no
library target, so tests that need a server spawn one: a stub source is a small
axum router on a random port (`spawn_stub` in `src/alerts.rs`,
`spawn_counting_stub` in `src/main.rs`). The poller and the API are tested by
calling `poll_once` and `get_alerts` directly, e.g.
`test_a_browser_request_never_reaches_a_source`.

Parse configuration through `Config::from_yaml` (the `config_from` helper in
`src/config.rs`), not `serde_yaml` directly: it is the entry point that refuses
unknown keys and validates.

**The documentation is tested too**: `mod docs` in `src/main.rs` fails when a
link-template placeholder, an `ALERTVIEW_*` variable or an internal link in the
docs does not match the code, and when a YAML block marked `alertview-config`
is not a configuration AlertView accepts.

**Frontend tests** run the real functions of `static/app.js`:
`tests/frontend/extract.js` lifts them out of the file unchanged. To test a new
function, add its name to `FNS` in `render.test.js` and call it through `H`.

Name a test after the property it protects
(`test_a_failing_source_is_reported_not_hidden`), and say in a comment which
bug it guards against.

## Before opening a pull request

The same checks as CI:

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo audit                         # cargo install cargo-audit, once
node --check static/app.js && node --check static/sw.js && node --check static/theme.js
node tests/frontend/render.test.js
```

- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/):
  `feat:`, `fix:`, `docs:`, `chore:`…
- Add a line to the `[Unreleased]` section of the [CHANGELOG](../CHANGELOG.md).
- A new configuration option goes in `config.example` and in
  [Configuration](configuration.md) or [Display](display.md).

## Releasing

1. Move `[Unreleased]` to a new version in the CHANGELOG, and bump the version
   in `Cargo.toml` and in the image of `03-deployment.yaml`.
2. Commit `chore(release): x.y.z`, tag `vx.y.z`, push the tag.

The tag triggers the Docker image (`ghcr.io/frakev/alertview:x.y.z` and
`latest`) and the GitHub release with the Linux binary.
