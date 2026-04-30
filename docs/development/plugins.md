# Plugin System

AlertView supports a plugin system that allows you to extend its functionality with custom alert sources, transformations, and actions.

## Overview

The plugin system is designed to be:
- **Flexible**: Support various types of extensions
- **Simple**: Easy to implement new plugins
- **Type-safe**: Leverage Rust's type system
- **Performant**: Minimal overhead for plugin operations

## Plugin Types

AlertView supports several types of plugins:

### Source Plugins

Add support for new alert sources beyond the built-in Alertmanager, Grafana, and Zabbix.

**Use cases:**
- Custom monitoring systems
- Proprietary alerting tools
- Specialized data sources
- Database queries

### Transformation Plugins

Transform alerts after they're fetched but before they're displayed.

**Use cases:**
- Normalize alerts from different sources
- Enrich alerts with additional data
- Filter or modify alerts based on custom logic
- Add computed fields

### Action Plugins

Perform actions when alerts are received or their state changes.

**Use cases:**
- Send notifications to external systems
- Update external databases
- Trigger automated remediation
- Log to specialized systems

### Display Plugins

Customize how alerts are displayed in the UI.

**Use cases:**
- Custom alert rendering
- Additional UI components
- Theme customizations
- Localization

## Architecture

### Plugin Trait

All plugins implement a common trait that defines their interface:

```rust
pub trait Plugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;
    
    /// Plugin version
    fn version(&self) -> &str;
    
    /// Plugin description
    fn description(&self) -> &str;
    
    /// Plugin author
    fn author(&self) -> &str;
    
    /// Initialize the plugin
    async fn initialize(&mut self, config: &PluginConfig) -> Result<(), PluginError>;
    
    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<(), PluginError>;
}
```

### Source Plugin Trait

```rust
#[async_trait]
pub trait SourcePlugin: Plugin {
    /// Source type identifier (e.g., "custom-monitor")
    fn source_type(&self) -> &str;
    
    /// Fetch alerts from the source
    async fn fetch_alerts(
        &self,
        config: &SourceConfig,
        client: &reqwest::Client,
    ) -> Result<Vec<Alert>, PluginError>;
    
    /// Validate source configuration
    fn validate_config(&self, config: &SourceConfig) -> Result<(), PluginError>;
}
```

### Transformation Plugin Trait

```rust
#[async_trait]
pub trait TransformationPlugin: Plugin {
    /// Transform alerts
    async fn transform(
        &self,
        alerts: Vec<Alert>,
        config: &TransformationConfig,
    ) -> Result<Vec<Alert>, PluginError>;
}
```

### Action Plugin Trait

```rust
#[async_trait]
pub trait ActionPlugin: Plugin {
    /// Action type identifier (e.g., "slack-notify")
    fn action_type(&self) -> &str;
    
    /// Execute action for an alert
    async fn execute(
        &self,
        alert: &Alert,
        action: &AlertAction,
        config: &ActionConfig,
    ) -> Result<(), PluginError>;
}

## Creating a Plugin

### Step 1: Create a New Crate

Create a new Rust crate for your plugin:

```bash
cargo new --lib alertview-my-plugin
cd alertview-my-plugin
```

### Step 2: Add Dependencies

Add AlertView and other dependencies to `Cargo.toml`:

```toml
[package]
name = "alertview-my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # For dynamic loading

[dependencies]
alertview = { path = "../alertview" }  # Or from crates.io
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
```

### Step 3: Implement the Plugin

Create a source plugin that fetches alerts from a custom API:

```rust
// src/lib.rs
use alertview::plugin::{SourcePlugin, Plugin, PluginError, SourceConfig};
use alertview::alerts::Alert;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct MyCustomSource {
    // Plugin state
}

#[async_trait]
impl Plugin for MyCustomSource {
    fn name(&self) -> &str {
        "my-custom-source"
    }
    
    fn version(&self) -> &str {
        "0.1.0"
    }
    
    fn description(&self) -> &str {
        "A custom alert source plugin"
    }
    
    fn author(&self) -> &str {
        "Your Name"
    }
    
    async fn initialize(&mut self, _config: &alertview::plugin::PluginConfig) -> Result<(), PluginError> {
        // Initialize any resources
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        // Clean up resources
        Ok(())
    }
}

#[async_trait]
impl SourcePlugin for MyCustomSource {
    fn source_type(&self) -> &str {
        "my-custom-source"
    }
    
    async fn fetch_alerts(
        &self,
        config: &SourceConfig,
        client: &reqwest::Client,
    ) -> Result<Vec<Alert>, PluginError> {
        // Extract custom config
        let api_url = config.url.clone();
        let api_key = config.extra.get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PluginError::Configuration("api_key is required".to_string()))?;
        
        // Fetch alerts from custom API
        let response = client
            .get(&api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        
        let alerts: Vec<CustomAlert> = response
            .json()
            .await
            .map_err(|e| PluginError::Parse(e.to_string()))?;
        
        // Transform to AlertView alerts
        Ok(alerts.into_iter().map(|a| a.into()).collect())
    }
    
    fn validate_config(&self, config: &SourceConfig) -> Result<(), PluginError> {
        if config.url.is_empty() {
            return Err(PluginError::Configuration("url is required".to_string()));
        }
        if !config.extra.contains_key("api_key") {
            return Err(PluginError::Configuration("api_key is required".to_string()));
        }
        Ok(())
    }
}

// Define custom alert type
#[derive(serde::Deserialize)]
struct CustomAlert {
    pub id: String,
    pub title: String,
    pub severity: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub starts_at: String,
}

// Convert to AlertView alert
impl From<CustomAlert> for Alert {
    fn from(alert: CustomAlert) -> Self {
        Alert {
            id: alert.id,
            labels: alert.labels,
            annotations: alert.annotations,
            severity: Some(alert.severity),
            state: "firing".to_string(),
            source: "my-custom-source".to_string(),
            ..Default::default()
        }
    }
}

// Export the plugin
#[no_mangle]
pub extern "C" fn _plugin_create() -> *mut dyn SourcePlugin {
    Box::into_raw(Box::new(MyCustomSource))
}

#[no_mangle]
pub extern "C" fn _plugin_destroy(ptr: *mut dyn SourcePlugin) {
    unsafe { Box::from_raw(ptr); }
}
```

### Step 4: Build the Plugin

```bash
cargo build --release
```

This creates a dynamic library at `target/release/libalertview_my_plugin.so` (Linux) or `target/release/alertview_my_plugin.dll` (Windows).

## Plugin Configuration

### Loading Plugins

AlertView loads plugins from a configured directory:

```yaml
# config.yaml
plugins:
  directory: /etc/alertview/plugins
  
  # Optional: specific plugins to load
  # If not specified, all plugins in the directory are loaded
  enabled:
    - my-custom-source
    - slack-notify
```

### Plugin Configuration

Each plugin can have its own configuration:

```yaml
sources:
  - name: my-custom
    kind: my-custom-source  # Matches the source_type from the plugin
    url: https://api.example.com/alerts
    extra:
      api_key: "your-api-key"
      timeout: 30
      
transformations:
  - name: my-transform
    kind: my-transform-plugin
    config:
      some_option: value
      
actions:
  - name: slack-notify
    kind: slack-notify
    config:
      webhook_url: "https://hooks.slack.com/..."
      channel: "#alerts"
```

## Plugin Development Best Practices

### Error Handling

- Return meaningful error messages
- Use appropriate error types from `PluginError`
- Handle network errors gracefully
- Validate inputs before processing

### Performance

- Avoid blocking operations
- Use async/await properly
- Implement timeouts for network operations
- Cache results when appropriate

### Configuration

- Validate configuration in `validate_config`
- Provide clear error messages for invalid config
- Document required and optional configuration

### Testing

- Test with various configurations
- Test error cases
- Test with different input data
- Mock external dependencies

### Documentation

- Document plugin functionality
- Document configuration options
- Provide examples
- Document limitations

## Example Plugins

### Example: Slack Notification Plugin

```rust
use alertview::plugin::{ActionPlugin, Plugin, PluginError, Alert, ActionConfig};
use alertview::alerts::AlertAction;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

pub struct SlackPlugin {
    client: Client,
}

#[derive(Deserialize)]
struct SlackConfig {
    pub webhook_url: String,
    pub channel: Option<String>,
    pub username: Option<String>,
}

#[async_trait]
impl Plugin for SlackPlugin {
    fn name(&self) -> &str { "slack-notify" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Send notifications to Slack" }
    fn author(&self) -> &str { "Your Name" }
    
    async fn initialize(&mut self, _config: &alertview::plugin::PluginConfig) -> Result<(), PluginError> {
        self.client = Client::new();
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl ActionPlugin for SlackPlugin {
    fn action_type(&self) -> &str { "slack" }
    
    async fn execute(
        &self,
        alert: &Alert,
        _action: &AlertAction,
        config: &ActionConfig,
    ) -> Result<(), PluginError> {
        let slack_config: SlackConfig = serde_yaml::from_value(config.extra.clone())
            .map_err(|e| PluginError::Configuration(e.to_string()))?;
        
        let message = format!(
            "*Alert* {} ({})\\n{}\\
*Source*: {}",
            alert.labels.get("alertname").unwrap_or(&"Unknown".to_string()),
            alert.severity.as_deref().unwrap_or("unknown"),
            alert.annotations.get("summary").unwrap_or(&"No description".to_string()),
            alert.source
        );
        
        let payload = serde_json::json!({
            "channel": slack_config.channel.unwrap_or_else(|| "#alerts".to_string()),
            "username": slack_config.username.unwrap_or_else(|| "AlertView".to_string()),
            "text": message,
        });
        
        self.client
            .post(&slack_config.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| PluginError::Network(e.to_string()))?;
        
        Ok(())
    }
}
```

### Example: Label Transformation Plugin

```rust
use alertview::plugin::{TransformationPlugin, Plugin, PluginError, TransformationConfig};
use alertview::alerts::Alert;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct LabelTransformPlugin;

#[async_trait]
impl Plugin for LabelTransformPlugin {
    fn name(&self) -> &str { "label-transform" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Transform alert labels" }
    fn author(&self) -> &str { "Your Name" }
    
    async fn initialize(&mut self, _config: &alertview::plugin::PluginConfig) -> Result<(), PluginError> {
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl TransformationPlugin for LabelTransformPlugin {
    async fn transform(
        &self,
        alerts: Vec<Alert>,
        config: &TransformationConfig,
    ) -> Result<Vec<Alert>, PluginError> {
        let prefix = config.extra.get("prefix")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        Ok(alerts.into_iter().map(|mut alert| {
            // Add prefix to all labels
            let mut new_labels = HashMap::new();
            for (key, value) in alert.labels.drain() {
                new_labels.insert(format!("{}{}", prefix, key), value);
            }
            alert.labels = new_labels;
            alert
        }).collect())
    }
}
```

## Plugin API Reference

### PluginError

```rust
pub enum PluginError {
    /// Configuration error
    Configuration(String),
    /// Network error
    Network(String),
    /// Parse error (JSON, YAML, etc.)
    Parse(String),
    /// Plugin-specific error
    Custom(String),
    /// Timeout error
    Timeout,
    /// Plugin not found
    NotFound,
    /// Plugin already loaded
    AlreadyLoaded,
}
```

### PluginConfig

```rust
pub struct PluginConfig {
    /// Plugin name
    pub name: String,
    /// Plugin-specific configuration
    pub config: serde_yaml::Value,
}
```

### SourceConfig

```rust
pub struct SourceConfig {
    pub name: String,
    pub kind: String,
    pub url: String,
    pub timeout: Option<u64>,
    pub link_template: Option<String>,
    /// Plugin-specific configuration
    pub extra: HashMap<String, serde_yaml::Value>,
}
```

### TransformationConfig

```rust
pub struct TransformationConfig {
    pub name: String,
    pub kind: String,
    /// Plugin-specific configuration
    pub extra: HashMap<String, serde_yaml::Value>,
}
```

### ActionConfig

```rust
pub struct ActionConfig {
    pub name: String,
    pub kind: String,
    /// Plugin-specific configuration
    pub extra: HashMap<String, serde_yaml::Value>,
}
```

## Plugin Lifecycle

```
┌─────────────────────────────────────────────────────────────┐
│                        Plugin Lifecycle                         │
├─────────────────────────────────────────────────────────────┤
│  1. Discovery: AlertView scans the plugins directory          │
│  2. Loading: Plugin library is loaded dynamically             │
│  3. Creation: Plugin instance is created                       │
│  4. Initialization: initialize() is called                     │
│  5. Registration: Plugin registers its capabilities           │
│  6. Usage: Plugin methods are called as needed                │
│  7. Shutdown: shutdown() is called on graceful shutdown         │
│  8. Unloading: Plugin library is unloaded                     │
└─────────────────────────────────────────────────────────────┘
```

## Dynamic Loading

AlertView uses dynamic loading to load plugins at runtime:

```rust
// In AlertView's plugin manager
pub struct PluginManager {
    plugins: HashMap<String, Box<dyn Plugin>>,
    source_plugins: HashMap<String, Box<dyn SourcePlugin>>,
    transformation_plugins: HashMap<String, Box<dyn TransformationPlugin>>,
    action_plugins: HashMap<String, Box<dyn ActionPlugin>>,
}

impl PluginManager {
    pub async fn load_plugin(&mut self, path: &Path) -> Result<(), PluginError> {
        // Load the dynamic library
        let lib = libloading::Library::new(path)?;
        
        // Get the create function
        let create: libloading::Symbol<fn() -> *mut dyn Plugin> = lib.get(b"_plugin_create")?;
        let destroy: libloading::Symbol<fn(*mut dyn Plugin)> = lib.get(b"_plugin_destroy")?;
        
        // Create the plugin instance
        let plugin_raw = create();
        let mut plugin = unsafe { Box::from_raw(plugin_raw) };
        
        // Initialize the plugin
        let plugin_config = PluginConfig {
            name: path.file_stem().unwrap().to_string_lossy().into_owned(),
            config: serde_yaml::Value::Null,
        };
        plugin.initialize(&plugin_config).await?;
        
        // Check what kind of plugin it is
        if let Some(source_plugin) = plugin.as_any().downcast_ref::<dyn SourcePlugin>() {
            self.source_plugins.insert(source_plugin.source_type().to_string(), 
                                       Box::from_raw(plugin_raw));
        }
        // ... similar for other plugin types
        
        Ok(())
    }
}
```

## Plugin Security

### Sandboxing

Plugins run in the same process as AlertView, so they have full access to:
- AlertView's memory
- System resources
- Network access

**Security considerations:**
- Only load plugins from trusted sources
- Validate plugin signatures if possible
- Run AlertView with least privileges
- Use containerization for isolation

### Best Practices

1. **Validate all inputs**: Don't trust data from external sources
2. **Limit network access**: Only make necessary network requests
3. **Handle errors gracefully**: Don't crash on invalid input
4. **Use timeouts**: All network operations should have timeouts
5. **Limit resource usage**: Don't consume excessive memory or CPU

## Plugin Distribution

### Packaging

Plugins can be distributed as:
- **Source code**: Users compile the plugin themselves
- **Pre-compiled binaries**: Distribute `.so`, `.dll`, or `.dylib` files
- **Docker images**: Include plugins in custom Docker images
- **Package managers**: Publish to crate registries

### Version Compatibility

Plugins should:
- Specify the AlertView version they're compatible with
- Handle version mismatches gracefully
- Document compatibility requirements

### Plugin Registry

In the future, AlertView may have a plugin registry where users can:
- Discover available plugins
- Install plugins with a single command
- Get updates for installed plugins

## Debugging Plugins

### Logging

Use AlertView's logging system:

```rust
use alertview::logging::{debug, info, warn, error};

info!("Plugin initialized");
debug!("Processing alert: {}", alert.id);
warn!("Failed to fetch alerts: {}", error);
error!("Critical error in plugin");
```

### Testing

Test plugins in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fetch_alerts() {
        let plugin = MyCustomSource;
        let config = SourceConfig {
            name: "test".to_string(),
            kind: "my-custom-source".to_string(),
            url: "http://localhost:8080".to_string(),
            extra: HashMap::from([
                ("api_key".to_string(), serde_yaml::Value::String("test-key".to_string())),
            ]),
        };
        
        let client = reqwest::Client::new();
        let result = plugin.fetch_alerts(&config, &client).await;
        
        assert!(result.is_ok());
    }
}
```

### Debug Builds

Build plugins with debug symbols:

```bash
cargo build
```

Then use a debugger to attach to AlertView and debug the plugin.

## Example: Complete Plugin Project

```
alertview-my-plugin/
├── Cargo.toml
├── src/
│   └── lib.rs          # Plugin implementation
├── examples/
│   └── config.yaml     # Example configuration
├── README.md           # Plugin documentation
└── CHANGELOG.md        # Plugin changelog
```

### Cargo.toml

```toml
[package]
name = "alertview-my-plugin"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your@email.com>"]
description = "A custom plugin for AlertView"
license = "MIT"
repository = "https://github.com/your-username/alertview-my-plugin"

[lib]
crate-type = ["cdylib"]

[dependencies]
alertview = "0.1"  # Or path to local AlertView
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
libloading = "0.8"
```

### README.md

```markdown
# AlertView My Plugin

A custom plugin for AlertView that adds support for My Monitoring System.

## Features

- Fetches alerts from My Monitoring System API
- Supports custom authentication
- Provides rich alert metadata

## Installation

1. Build the plugin:
   ```bash
   cargo build --release
   ```

2. Copy the plugin to AlertView's plugins directory:
   ```bash
   cp target/release/libalertview_my_plugin.so /etc/alertview/plugins/
   ```

3. Update AlertView configuration:
   ```yaml
   plugins:
     directory: /etc/alertview/plugins
     enabled:
       - my-plugin
   
   sources:
     - name: my-monitor
       kind: my-monitoring-system
       url: https://api.mymonitor.com/alerts
       extra:
         api_key: "your-api-key"
   ```

## Configuration

| Option | Type | Required | Default | Description |
|--------|------|----------|---------|-------------|
| url | string | Yes | - | API endpoint URL |
| api_key | string | Yes | - | API authentication key |
| timeout | number | No | 30 | Request timeout in seconds |

## License

MIT
```

## Future Enhancements

The plugin system is designed to be extensible. Future enhancements may include:

1. **Plugin isolation**: Run plugins in separate processes or containers
2. **Plugin marketplace**: Central registry for discovering and installing plugins
3. **Plugin dependencies**: Allow plugins to depend on other plugins
4. **Plugin hot-reloading**: Reload plugins without restarting AlertView
5. **Plugin configuration UI**: Web UI for configuring plugins
6. **Plugin metrics**: Track plugin performance and usage
7. **Plugin health checks**: Monitor plugin health and status

## Troubleshooting

### Plugin Not Loading

1. Check the plugin file exists in the plugins directory
2. Verify the file has the correct extension (.so, .dll, .dylib)
3. Check file permissions
4. Look for errors in AlertView logs

### Plugin Fails to Initialize

1. Check the plugin's configuration is valid
2. Verify all required configuration options are provided
3. Look for errors in AlertView logs

### Plugin Crashes

1. Check AlertView logs for error messages
2. Verify the plugin works with the current AlertView version
3. Test the plugin in isolation

### Performance Issues

1. Check for blocking operations in the plugin
2. Verify async/await is used properly
3. Profile the plugin to identify bottlenecks

## Best Practices for Plugin Developers

1. **Start small**: Implement a minimal plugin first, then add features
2. **Test thoroughly**: Test with various configurations and inputs
3. **Document everything**: Document configuration, behavior, and limitations
4. **Handle errors gracefully**: Don't crash on invalid input or network errors
5. **Follow conventions**: Use the same coding style as AlertView
6. **Version your plugins**: Use semantic versioning for your plugins
7. **Specify compatibility**: Document which AlertView versions are supported
8. **Provide examples**: Include example configurations
9. **Engage with the community**: Share your plugins and get feedback
10. **Keep plugins updated**: Update plugins for new AlertView versions

## Additional Resources

- [AlertView Documentation](../README.md)
- [Rust Plugin System Examples](https://github.com/rust-lang/rfcs/blob/master/text/1955-dynamic-loading.md)
- [libloading Documentation](https://docs.rs/libloading/latest/libloading/)
- [async-trait Documentation](https://docs.rs/async-trait/latest/async_trait/)
- [Dynamic Loading in Rust](https://blog.rust-lang.org/2019/11/20/Rust-1.40.0.html#stabilizing-library-features)
