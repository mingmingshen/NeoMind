//! NeoMind Extension Runner
//!
//! This is a standalone process that loads and runs a single extension.
//! It communicates with the main NeoMind process via stdin/stdout using
//! the IPC protocol.
//!
//! # Usage
//!
//! ```bash
//! neomind-extension-runner --extension-path /path/to/extension.dylib
//! ```
//!
//! # Protocol
//!
//! The runner reads IPC messages from stdin and writes responses to stdout.
//! All messages are framed with a 4-byte length prefix (little-endian).

use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tracing::{debug, error, info};

use neomind_core::extension::isolated::{ErrorKind, IpcFrame, IpcMessage, IpcResponse};
use neomind_core::extension::loader::NativeExtensionLoader;
use neomind_core::extension::system::DynExtension;

/// Extension runner arguments
#[derive(Parser, Debug)]
#[command(name = "neomind-extension-runner")]
#[command(about = "Run a NeoMind extension in isolated mode")]
struct Args {
    /// Path to the extension library
    #[arg(long, short = 'e')]
    extension_path: PathBuf,

    /// Enable verbose logging
    #[arg(long, short = 'v')]
    verbose: bool,
}

/// Extension runner state
struct Runner {
    /// Loaded extension
    extension: DynExtension,
    /// Extension metadata
    metadata: neomind_core::extension::system::ExtensionMetadata,
    /// Stdin reader
    stdin: BufReader<std::io::Stdin>,
    /// Stdout writer
    stdout: BufWriter<std::io::Stdout>,
    /// Running flag
    running: bool,
}

impl Runner {
    /// Load extension and create runner
    fn load(extension_path: &PathBuf) -> Result<Self, String> {
        info!(path = %extension_path.display(), "Loading extension");

        // Load the extension using the native loader
        let loader = NativeExtensionLoader::new();
        let loaded = loader.load(extension_path).map_err(|e| format!("Failed to load extension: {}", e))?;

        // Get metadata
        let ext_guard = loaded.extension.blocking_read();
        let metadata = ext_guard.metadata().clone();
        drop(ext_guard);

        info!(
            extension_id = %metadata.id,
            name = %metadata.name,
            version = %metadata.version,
            "Extension loaded successfully"
        );

        Ok(Self {
            extension: loaded.extension,
            metadata,
            stdin: BufReader::new(std::io::stdin()),
            stdout: BufWriter::new(std::io::stdout()),
            running: true,
        })
    }

    /// Run the main loop
    fn run(&mut self) {
        info!("Starting IPC message loop");

        // Send Ready message
        self.send_response(IpcResponse::Ready {
            metadata: self.metadata.clone(),
        });

        // Message loop
        while self.running {
            match self.receive_message() {
                Ok(Some(message)) => {
                    self.handle_message(message);
                }
                Ok(None) => {
                    // EOF, exit
                    info!("stdin closed, exiting");
                    break;
                }
                Err(e) => {
                    error!(error = %e, "Failed to receive message");
                    break;
                }
            }
        }

        info!("Extension runner shutting down");
    }

    /// Receive an IPC message from stdin
    fn receive_message(&mut self) -> Result<Option<IpcMessage>, String> {
        // Read length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        match self.stdin.read_exact(&mut len_bytes) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(None);
            }
            Err(e) => {
                return Err(format!("Failed to read length: {}", e));
            }
        }

        let len = u32::from_le_bytes(len_bytes) as usize;

        // Sanity check
        if len > 10 * 1024 * 1024 {
            return Err(format!("Message too large: {} bytes", len));
        }

        // Read payload
        let mut payload = vec![0u8; len];
        self.stdin
            .read_exact(&mut payload)
            .map_err(|e| format!("Failed to read payload: {}", e))?;

        // Decode message
        let message = IpcMessage::from_bytes(&payload)
            .map_err(|e| format!("Failed to decode message: {}", e))?;

        debug!(message = ?message, "Received IPC message");
        Ok(Some(message))
    }

    /// Send an IPC response to stdout
    fn send_response(&mut self, response: IpcResponse) {
        debug!(response = ?response, "Sending IPC response");

        let payload = match response.to_bytes() {
            Ok(p) => p,
            Err(e) => {
                error!(error = %e, "Failed to serialize response");
                return;
            }
        };

        let frame = IpcFrame::new(payload);
        let bytes = frame.encode();

        if let Err(e) = self.stdout.write_all(&bytes) {
            error!(error = %e, "Failed to write response");
            return;
        }

        if let Err(e) = self.stdout.flush() {
            error!(error = %e, "Failed to flush stdout");
        }
    }

    /// Handle an incoming IPC message
    fn handle_message(&mut self, message: IpcMessage) {
        match message {
            IpcMessage::Init { config: _ } => {
                // Already initialized, just acknowledge
                self.send_response(IpcResponse::Ready {
                    metadata: self.metadata.clone(),
                });
            }

            IpcMessage::ExecuteCommand { command, args, request_id } => {
                self.handle_execute_command(command, args, request_id);
            }

            IpcMessage::ProduceMetrics { request_id } => {
                self.handle_produce_metrics(request_id);
            }

            IpcMessage::HealthCheck { request_id } => {
                self.handle_health_check(request_id);
            }

            IpcMessage::GetMetadata { request_id } => {
                self.send_response(IpcResponse::Metadata {
                    request_id,
                    metadata: self.metadata.clone(),
                });
            }

            IpcMessage::Shutdown => {
                info!("Received shutdown command");
                self.running = false;
            }

            IpcMessage::Ping { timestamp } => {
                self.send_response(IpcResponse::Pong { timestamp });
            }
        }
    }

    /// Handle execute command
    fn handle_execute_command(&mut self, command: String, args: serde_json::Value, request_id: u64) {
        debug!(command = %command, request_id, "Executing command");

        // Use tokio runtime for async execution
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                self.send_response(IpcResponse::Error {
                    request_id,
                    error: format!("Failed to create runtime: {}", e),
                    kind: ErrorKind::Internal,
                });
                return;
            }
        };

        let ext_clone = Arc::clone(&self.extension);
        let command_clone = command.clone();

        let result = rt.block_on(async {
            let ext_guard = ext_clone.read().await;
            ext_guard.execute_command(&command_clone, &args).await
        });

        match result {
            Ok(value) => {
                self.send_response(IpcResponse::Success {
                    request_id,
                    data: value,
                });
            }
            Err(e) => {
                let kind = match &e {
                    neomind_core::extension::system::ExtensionError::CommandNotFound(_) => {
                        ErrorKind::CommandNotFound
                    }
                    neomind_core::extension::system::ExtensionError::InvalidArguments(_) => {
                        ErrorKind::InvalidArguments
                    }
                    neomind_core::extension::system::ExtensionError::Timeout(_) => ErrorKind::Timeout,
                    _ => ErrorKind::ExecutionFailed,
                };

                self.send_response(IpcResponse::Error {
                    request_id,
                    error: e.to_string(),
                    kind,
                });
            }
        }
    }

    /// Handle produce metrics
    fn handle_produce_metrics(&mut self, request_id: u64) {
        debug!(request_id, "Producing metrics");

        // Use tokio runtime
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                self.send_response(IpcResponse::Error {
                    request_id,
                    error: format!("Failed to create runtime: {}", e),
                    kind: ErrorKind::Internal,
                });
                return;
            }
        };

        let ext_clone = Arc::clone(&self.extension);

        let result = rt.block_on(async {
            let ext_guard = ext_clone.read().await;
            ext_guard.produce_metrics()
        });

        match result {
            Ok(metrics) => {
                self.send_response(IpcResponse::Metrics {
                    request_id,
                    metrics,
                });
            }
            Err(e) => {
                self.send_response(IpcResponse::Error {
                    request_id,
                    error: e.to_string(),
                    kind: ErrorKind::Internal,
                });
            }
        }
    }

    /// Handle health check
    fn handle_health_check(&mut self, request_id: u64) {
        debug!(request_id, "Health check");

        // Use tokio runtime
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(_) => {
                self.send_response(IpcResponse::Health {
                    request_id,
                    healthy: false,
                });
                return;
            }
        };

        let ext_clone = Arc::clone(&self.extension);

        let healthy = rt.block_on(async {
            let ext_guard = ext_clone.read().await;
            ext_guard.health_check().await.unwrap_or(false)
        });

        self.send_response(IpcResponse::Health {
            request_id,
            healthy,
        });
    }
}

fn main() {
    let args = Args::parse();

    // Initialize logging to stderr (stdout is used for IPC)
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .with_writer(std::io::stderr) // Important: log to stderr, not stdout
        .with_ansi(false)
        .compact()
        .init();

    info!("NeoMind Extension Runner starting");
    debug!(extension_path = %args.extension_path.display(), "Extension path");

    // Load the extension
    let mut runner = match Runner::load(&args.extension_path) {
        Ok(r) => r,
        Err(e) => {
            error!(error = %e, "Failed to load extension");
            std::process::exit(1);
        }
    };

    // Run the main loop
    runner.run();

    info!("Extension runner exiting normally");
}
