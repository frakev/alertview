# Testing AlertView

This guide covers testing AlertView, including running existing tests, writing new tests, and testing strategies.

## Test Structure

AlertView has two test suites, both run by CI on every push and pull request:

```
alertview/
├── src/
│   ├── main.rs           # Poller, /api/alerts, CSP — against stub sources
│   ├── config.rs         # Config parsing, validation, unknown-key messages
│   └── alerts.rs         # Source clients, against a stub server
└── tests/frontend/
    ├── extract.js        # Lifts functions out of static/app.js, unchanged
    └── render.test.js    # Runs them: escaping, TV grid, titles, empty state…
```

The Rust tests live in `#[cfg(test)] mod tests` blocks next to the code they
cover; `cargo test` runs them. The frontend tests need only Node:

```bash
node tests/frontend/render.test.js
```

They run the real functions from `static/app.js` rather than copies, so a
change to the page is tested as the browser will run it.

## Running Tests

### Run All Tests

```bash
# Run all unit and integration tests
cargo test

# Run with verbose output
cargo test -- --nocapture
```

### Run Specific Tests

```bash
# Run tests for a specific module
cargo test --lib

# Run tests for the binary
cargo test --bin alertview

# Run a specific test
cargo test test_config_loading

# Run tests matching a pattern
cargo test config::
```

### Run Tests with Coverage

Install `cargo-tarpaulin` for code coverage:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run tests with coverage
cargo tarpaulin

# Generate HTML report
cargo tarpaulin --out Html
```

### Run Tests in Release Mode

```bash
# Run tests with release optimizations
cargo test --release
```

### Run Tests with Logging

```bash
# Enable debug logging during tests
RUST_LOG=debug cargo test

# Enable trace logging
RUST_LOG=trace cargo test -- --nocapture
```

## Unit Tests

Unit tests verify individual functions and modules in isolation. They are located in the same file as the code they test, in a `#[cfg(test)]` module.

### Example: Testing Configuration

In `src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_from_yaml() {
        let yaml = r#"
            sources:
              - name: test
                type: alertmanager
                url: http://localhost:9093
            port: 8080
        "#;

        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.port, 8080);
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].name, "test");
    }

    #[test]
    fn test_default_values() {
        let yaml = r#"
            sources: []
        "#;

        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.port, 8080); // Default port
        assert_eq!(config.refresh_interval, Some(30)); // Default refresh
    }

    #[test]
    fn test_invalid_config() {
        let yaml = r#"
            sources:
              - name: test
                type: invalid
                url: http://localhost
        "#;

        let result: Result<Config, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }
}
```

### Example: Testing Alert Processing

In `src/alerts.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_apply_link_template() {
        let template = "https://example.com/alerts/{{.Labels.alertname}}";
        let alert = Alert {
            labels: HashMap::from([("alertname".to_string(), "TestAlert".to_string())]),
            ..Default::default()
        };

        let result = apply_link_template(template, &alert);
        assert_eq!(result, "https://example.com/alerts/TestAlert");
    }

    #[test]
    fn test_transform_alertmanager_alert() {
        let am_alert = AlertmanagerAlert {
            labels: HashMap::from([
                ("alertname".to_string(), "Test".to_string()),
                ("severity".to_string(), "critical".to_string()),
            ]),
            annotations: HashMap::from([
                ("summary".to_string(), "Test alert".to_string()),
            ]),
            starts_at: Utc::now(),
            ends_at: None,
            generator_url: "http://localhost:9093".to_string(),
        };

        let alerts = transform_alertmanager_alerts(&[am_alert]);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].labels["alertname"], "Test");
        assert_eq!(alerts[0].severity, Some("critical".to_string()));
    }
}
```

## Integration Tests

Integration tests verify that multiple components work together correctly. They are located in the `tests/` directory.

### Example: API Integration Test

In `tests/integration.rs`:

```rust
use alertview::config::{Config, Source, SourceKind};
use alertview::alerts::Alert;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_api_returns_alerts() {
    // Setup test configuration
    let config = Config {
        sources: vec![Source {
            name: "test".to_string(),
            source_type: SourceType::Alertmanager,
            url: "http://localhost:9093".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let shared_config = Arc::new(RwLock::new(config));
    
    // Create a mock HTTP client that returns test data
    let client = reqwest::Client::new();
    
    // Call the alert fetching function
    let alerts = alertview::alerts::fetch_all_alerts(&shared_config, &client).await;
    
    // Verify results
    assert!(alerts.is_ok());
}
```

### Example: End-to-End Test

```rust
#[tokio::test]
async fn test_health_endpoint() {
    // Start the server in test mode
    let app = alertview::main::create_app().await;
    
    // Create a test client
    let client = reqwest::Client::new();
    
    // Start a test server
    let server = axum::Server::bind(&"0.0.0.0:0".parse().unwrap())
        .serve(app.into_make_service());
    
    let addr = server.local_addr();
    
    // Spawn the server in the background
    tokio::spawn(async move {
        server.await.unwrap();
    });
    
    // Give the server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Make a request to the health endpoint
    let response = client
        .get(&format!("http://{}/health", addr))
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body = response.text().await.unwrap();
    assert_eq!(body, "OK");
}
```

## Test Utilities

### Test Fixtures

Create reusable test data:

```rust
// In tests/fixtures.rs or src/test_utils.rs
pub fn create_test_config() -> Config {
    Config {
        sources: vec![Source {
            name: "test".to_string(),
            source_type: SourceType::Alertmanager,
            url: "http://localhost:9093".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

pub fn create_test_alert() -> Alert {
    Alert {
        id: "test-1".to_string(),
        labels: HashMap::from([
            ("alertname".to_string(), "TestAlert".to_string()),
            ("severity".to_string(), "critical".to_string()),
        ]),
        annotations: HashMap::from([
            ("summary".to_string(), "Test alert".to_string()),
        ]),
        starts_at: Utc::now(),
        ends_at: None,
        state: "firing".to_string(),
        severity: Some("critical".to_string()),
        source: "test".to_string(),
        generator_url: Some("http://localhost:9093".to_string()),
    }
}
```

### Integration tests against a stub Alertmanager

No mocking library is needed: axum is already a dependency, so a stub server can
be spawned inside a `#[tokio::test]` on a random port. See
`test_fetch_from_a_plain_base_url` and `test_fetch_reports_http_status` in
`src/alerts.rs`.

```rust
async fn spawn_stub(alerts: &'static str, silences: &'static str) -> String {
    use axum::{routing::get, Router};
    let json = |body: &'static str| async move {
        ([("content-type", "application/json")], body)
    };
    let app = Router::new()
        .route("/api/v2/alerts", get(move || json(alerts)))
        .route("/api/v2/silences", get(move || json(silences)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{}", addr)
}
```

These cover what unit tests cannot: the API path being appended to the source
URL, the wire format actually sent by Alertmanager (`silencedBy`, `startsAt`),
link sanitising, and HTTP errors surfacing as a typed `HttpStatusError`.

## Test Strategies

### Unit Test Strategy

1. **Test each public function** - Every public function should have at least one test
2. **Test edge cases** - Empty inputs, boundary values, error conditions
3. **Test happy paths** - Normal, expected usage
4. **Test error handling** - How functions handle errors
5. **Test invariants** - Properties that should always be true

### Integration Test Strategy

1. **Test module interactions** - How modules work together
2. **Test API endpoints** - HTTP routes and responses
3. **Test configuration loading** - From files and environment
4. **Test error propagation** - How errors flow through the system

### End-to-End Test Strategy

1. **Test complete workflows** - From request to response
2. **Test with real dependencies** - Databases, external services (in test environments)
3. **Test deployment scenarios** - Docker, Kubernetes

## Test Coverage

### Measuring Coverage

Use `cargo-tarpaulin` to measure test coverage:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run tests with coverage
cargo tarpaulin

# Generate HTML report
cargo tarpaulin --out Html --output-dir coverage

# Open the report
xdg-open coverage/index.html
```

### Coverage Targets

| Component | Target Coverage |
|-----------|-----------------|
| Core logic (alerts, config) | 90%+ |
| API handlers | 80%+ |
| Utilities | 70%+ |
| Overall | 80%+ |

### Improving Coverage

1. Identify untested code:

```bash
cargo tarpaulin --line
```

2. Write tests for missing coverage
3. Focus on critical paths first
4. Consider whether untested code needs tests or can be removed

## Property-Based Testing

Use `proptest` for property-based testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_link_template_never_panics(template in ".*", label_value in ".*") {
            let alert = Alert {
                labels: HashMap::from([("test".to_string(), label_value)]),
                ..Default::default()
            };
            
            // This should never panic
            let _ = apply_link_template(&template, &alert);
        }

        #[test]
        fn test_config_roundtrip(config in any::<Config>()) {
            // Serialize and deserialize should be identity
            let yaml = serde_yaml::to_string(&config).unwrap();
            let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
            
            assert_eq!(config, parsed);
        }
    }
}
```

## Performance Testing

### Benchmarks

Create benchmarks in the `benches/` directory:

```rust
// benches/benchmark.rs
use alertview::alerts::{Alert, transform_alertmanager_alerts};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_transform(c: &mut Criterion) {
    let am_alerts: Vec<AlertmanagerAlert> = (0..1000)
        .map(|i| AlertmanagerAlert {
            labels: HashMap::from([
                ("alertname".to_string(), format!("Alert{}", i)),
                ("severity".to_string(), "critical".to_string()),
            ]),
            annotations: HashMap::from([
                ("summary".to_string(), "Test alert".to_string()),
            ]),
            starts_at: Utc::now(),
            ends_at: None,
            generator_url: "http://localhost:9093".to_string(),
        })
        .collect();

    c.bench_function("transform 1000 alerts", |b| {
        b.iter(|| {
            transform_alertmanager_alerts(black_box(&am_alerts))
        })
    });
}

criterion_group!(benches, benchmark_transform);
criterion_main!(benches);
```

Run benchmarks:

```bash
cargo bench
```

### Load Testing

Use tools like `wrk`, `ab`, or `k6` to load test the API:

```bash
# Using wrk
wrk -t12 -c400 -d30s http://localhost:8080/api/alerts

# Using Apache Bench (ab)
ab -n 10000 -c 100 http://localhost:8080/api/alerts

# Using k6
k6 run --vus 100 --duration 30s script.js
```

Example k6 script:

```javascript
// script.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export default function () {
    let res = http.get('http://localhost:8080/api/alerts');
    check(res, {
        'status is 200': (r) => r.status === 200,
        'response time < 500ms': (r) => r.timings.duration < 500,
    });
    sleep(1);
}
```

## Test Configuration

### Test-Specific Configuration

Create a `config.test.yaml` for tests:

```yaml
# config.test.yaml
sources:
  - name: test-alertmanager
    type: alertmanager
    url: http://localhost:19093  # Test port
    timeout: 5
    retry_policy:
      max_retries: 0  # No retries in tests
      initial_delay_ms: 100
      max_delay_ms: 500

display:
  refresh_interval: 1
  theme: light
  timezone: UTC

port: 18080  # Test port
log_format: text
```

### Environment Variables for Tests

```bash
# Run tests with test configuration
ALERTVIEW_CONFIG=config.test.yaml cargo test

# Or set multiple variables
ALERTVIEW_CONFIG=config.test.yaml \
ALERTVIEW_PORT=18080 \
RUST_LOG=debug \
cargo test
```

## Test Organization

### Test File Structure

See [Test Structure](#test-structure) at the top of this page: Rust tests sit
next to the code in `src/`, frontend tests in `tests/frontend/`. There is no
`tests/*.rs` integration crate and no benchmark suite.

### Test Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Unit test | `test_<function>_<scenario>` | `test_load_config_valid_yaml` |
| Integration test | `test_<module>_<scenario>` | `test_api_returns_alerts` |
| Property test | `test_<property>_<condition>` | `test_config_roundtrip` |
| Benchmark | `bench_<function>_<input>` | `bench_transform_1000_alerts` |

### Test Categories

1. **Happy path tests** - Normal, expected usage
2. **Edge case tests** - Boundary values, empty inputs
3. **Error tests** - Invalid inputs, error conditions
4. **Property tests** - Invariants that should always hold
5. **Integration tests** - Multiple components working together
6. **End-to-end tests** - Complete workflows
7. **Performance tests** - Benchmarks and load tests

## Continuous Integration Testing

### What CI runs

`.github/workflows/ci.yml` runs three jobs. Running the same commands locally
before pushing gives the same verdict:

```bash
# Rust
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked

# Dependency advisories (cargo install cargo-audit, once)
cargo audit

# Frontend
node --check static/app.js && node --check static/sw.js && node --check static/theme.js
node tests/frontend/render.test.js
```

## Debugging Tests

### Run a Single Test

```bash
cargo test test_name -- --nocapture
```

### Run Tests with Logging

```bash
RUST_LOG=debug cargo test -- --nocapture
```

### Inspect Test Output

```bash
# Show stdout/stderr from tests
cargo test -- --nocapture

# Show backtrace on panic
RUST_BACKTRACE=1 cargo test

# Show full backtrace
RUST_BACKTRACE=full cargo test
```

### Debug with LLDB

```bash
# Install lldb
sudo apt-get install lldb

# Build with debug symbols
cargo build --bin alertview

# Debug a test
cargo test --bin alertview test_name -- --nocapture --exact
```

### Debug with GDB

```bash
# Install gdb
sudo apt-get install gdb

# Build with debug symbols
cargo build --bin alertview

# Debug a test
rust-gdb target/debug/alertview
```

## Test Maintenance

### Keeping Tests Fast

1. **Avoid I/O in tests** - Use mocks instead of real files/network
2. **Use small test data** - Don't load large files in tests
3. **Parallelize tests** - Tests run in parallel by default
4. **Avoid sleep** - Use async/await or mock time

### Keeping Tests Reliable

1. **Avoid flaky tests** - Tests should be deterministic
2. **Isolate tests** - Tests shouldn't depend on each other
3. **Clean up after tests** - Remove temporary files, reset state
4. **Use unique names** - Avoid collisions between tests

### Keeping Tests Maintainable

1. **Follow DRY principle** - Extract common test code into utilities
2. **Use descriptive names** - Test names should describe what they test
3. **Keep tests focused** - Each test should verify one thing
4. **Update tests with code** - Keep tests in sync with implementation

## Test Examples from AlertView

### Config Tests

From `src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_from_yaml() {
        let yaml = r#"
sources:
  - name: test
    type: alertmanager
    url: http://localhost:9093
port: 9090
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.port, 9090);
    }

    #[test]
    fn test_default_retry_policy() {
        let config = Config::default();
        let source = &config.sources[0];
        if let Some(retry) = &source.retry_policy {
            assert_eq!(retry.max_retries, 3);
            assert_eq!(retry.initial_delay_ms, 1000);
            assert_eq!(retry.max_delay_ms, 10000);
        }
    }

    #[test]
    fn test_log_format_from_env() {
        std::env::set_var("ALERTVIEW_LOG_FORMAT", "json");
        let config = Config::from_env().unwrap();
        assert_eq!(config.log_format, "json");
        std::env::remove_var("ALERTVIEW_LOG_FORMAT");
    }
}
```

### Alert Tests

From `src/alerts.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_link_template_simple() {
        let template = "https://example.com/{{.Labels.alertname}}";
        let mut labels = HashMap::new();
        labels.insert("alertname".to_string(), "MyAlert".to_string());
        let alert = Alert {
            labels,
            ..Default::default()
        };
        let result = apply_link_template(template, &alert);
        assert_eq!(result, "https://example.com/MyAlert");
    }

    #[test]
    fn test_apply_link_template_missing_label() {
        let template = "https://example.com/{{.Labels.missing}}";
        let alert = Alert::default();
        let result = apply_link_template(template, &alert);
        assert_eq!(result, "https://example.com/");
    }

    #[test]
    fn test_transform_grafana_alert() {
        let grafana_alert = GrafanaAlert {
            
            panel_id: 1,
            rule_name: "Test Rule".to_string(),
            rule_url: "http://grafana:3000/d/abc123".to_string(),
            state: "alerting".to_string(),
            labels: HashMap::from([
                ("severity".to_string(), "critical".to_string()),
            ]),
            annotations: HashMap::from([
                ("summary".to_string(), "Test alert".to_string()),
            ]),
            starts_at: Utc::now(),
            ends_at: None,
        };

        let alerts = transform_grafana_alerts(&[grafana_alert]);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].source, "grafana");
    }
}
```

## Best Practices

1. **Test behavior, not implementation** - Tests should verify what the code does, not how it does it
2. **Keep tests fast** - Slow tests discourage running them
3. **Keep tests isolated** - Tests shouldn't affect each other
4. **Test edge cases** - Don't just test the happy path
5. **Use assertions wisely** - Each assertion should verify one thing
6. **Clean up resources** - Close files, connections, etc.
7. **Use helper functions** - Extract common test code
8. **Document test purpose** - Use comments to explain what each test verifies
9. **Review test coverage** - Regularly check for untested code
10. **Update tests with code** - When you change the code, update the tests

## Additional Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Cargo Test Documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [Tarpaulin Coverage Tool](https://github.com/xd009642/tarpaulin)
- [Proptest Property Testing](https://github.com/altsysrq/proptest)
- [Criterion Benchmarking](https://github.com/bheisler/criterion.rs)
- [k6 Load Testing](https://k6.io/)
