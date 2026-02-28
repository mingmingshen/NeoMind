# Extension Process Isolation

## Overview

NeoMind supports process-level isolation for extensions, ensuring that extension crashes
cannot affect the main NeoMind server process.

## Architecture

```
NeoMind Main Process                    Extension Runner Process
┌─────────────────────┐                ┌─────────────────────┐
│ IsolatedExtension   │                │ extension-runner    │
│ ┌─────────────────┐ │    stdin       │ ┌─────────────────┐ │
│ │ stdin (pipe)    │ ├───────────────►│ │ IPC Receiver    │ │
│ └─────────────────┘ │                │ └─────────────────┘ │
│ ┌─────────────────┐ │    stdout      │ ┌─────────────────┐ │
│ │ stdout (pipe)   │ │◄───────────────┤ │ IPC Sender      │ │
│ └─────────────────┘ │                │ └─────────────────┘ │
│ ┌─────────────────┐ │    stderr      │ ┌─────────────────┐ │
│ │ stderr (pipe)   │ │◄───────────────┤ │ Logs/Errors     │ │
│ └─────────────────┘ │                │ └─────────────────┘ │
└─────────────────────┘                │ ┌─────────────────┐ │
                                       │ │ Extension (.dylib)
                                       │ └─────────────────┘ │
                                       └─────────────────────┘
```

## Safety Guarantees

1. **Process Isolation**: Extension runs in a separate process
2. **No Shared Memory**: Extension cannot corrupt main process memory
3. **Controlled Communication**: All communication via IPC protocol
4. **Automatic Recovery**: Extension can be automatically restarted on crash
5. **Resource Limits**: Memory limits can be applied to extension process

## Usage

### Configuration

Add to your `config.toml`:

```toml
[extensions]
# Run all extensions in isolated mode by default
isolated_by_default = true

# Force specific extensions to run in isolated mode
force_isolated = ["weather-extension", "untrusted-extension"]

# Force specific extensions to run in-process
force_in_process = ["core-extension"]

[extensions.isolated]
startup_timeout_secs = 30
command_timeout_secs = 30
max_memory_mb = 512
restart_on_crash = true
max_restart_attempts = 3
restart_cooldown_secs = 60
```

### Building the Extension Runner

The `neomind-extension-runner` binary must be available in the same directory as the
main NeoMind executable or in your PATH.

```bash
# Build the extension runner
cargo build --release -p neomind-extension-runner

# The binary will be at:
# target/release/neomind-extension-runner
```

### Loading Extensions in Isolated Mode

```rust
use neomind_core::extension::loader::{
    IsolatedExtensionLoader,
    IsolatedLoaderConfig,
    LoadedExtension,
};

// Create loader with isolated mode enabled
let config = IsolatedLoaderConfig {
    use_isolated_by_default: true,
    ..Default::default()
};

let loader = IsolatedExtensionLoader::new(config);

// Load extension (will run in isolated process)
let loaded = loader.load(&path).await?;

match loaded {
    LoadedExtension::Isolated(isolated) => {
        // Extension is running in separate process
        let result = isolated.execute_command("test", &json!({})).await?;
    }
    LoadedExtension::Native(ext) => {
        // Extension is running in-process
    }
}
```

## IPC Protocol

The IPC protocol uses JSON messages with a 4-byte length prefix (little-endian).

### Message Types

**Host → Extension:**
- `Init { config }` - Initialize extension with configuration
- `ExecuteCommand { command, args, request_id }` - Execute a command
- `ProduceMetrics { request_id }` - Request current metrics
- `HealthCheck { request_id }` - Check extension health
- `GetMetadata { request_id }` - Request extension metadata
- `Shutdown` - Graceful shutdown
- `Ping { timestamp }` - Keep-alive ping

**Extension → Host:**
- `Ready { metadata }` - Extension is ready
- `Success { request_id, data }` - Command executed successfully
- `Error { request_id, error, kind }` - Error occurred
- `Metrics { request_id, metrics }` - Metrics response
- `Health { request_id, healthy }` - Health check response
- `Metadata { request_id, metadata }` - Metadata response
- `Pong { timestamp }` - Ping response

## When to Use Isolated Mode

**Use isolated mode when:**
- Extension is from an untrusted source
- Extension uses C/C++ libraries that might crash
- Extension has complex dependencies
- Extension needs resource limits
- You want automatic restart on crash

**Use in-process mode when:**
- Extension is trusted and well-tested
- Maximum performance is required
- Extension has complex async operations
- Extension needs shared memory access

## Limitations

1. **Performance**: IPC communication adds overhead (~1-5ms per call)
2. **Startup Time**: Extension process needs to start (~100-500ms)
3. **Metrics**: `produce_metrics()` is async for isolated extensions
4. **Complexity**: More moving parts to manage

## Troubleshooting

### Extension Runner Not Found

```
Error: Could not find neomind-extension-runner binary
```

Solution: Ensure the runner binary is in the same directory as neomind-api or in PATH.

### Extension Process Crashes

Check the extension logs in stderr. The main process will log:

```
Extension process crashed: signal: 11 (SIGSEGV)
```

If `restart_on_crash` is enabled, the extension will be automatically restarted.

### Timeout Errors

```
Error: Extension operation timed out after 30000ms
```

Increase the timeout in configuration:

```toml
[extensions.isolated]
command_timeout_secs = 60
```
