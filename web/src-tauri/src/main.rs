// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;

// Global state for the Axum server
// We use Arc<Runtime> to keep it alive for the app's lifetime
struct ServerState {
    runtime: Arc<Mutex<Option<Runtime>>>,
    server_thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

// Global state for the tray icon
// We need to keep the tray icon alive for the app's lifetime
struct TrayState {
    _tray: Option<tauri::tray::TrayIcon>,
}

impl ServerState {
    // Wait for server to be ready by polling the TCP port
    fn wait_for_server_ready(&self, timeout_secs: u64) -> bool {
        let max_attempts = timeout_secs * 20; // Check every 50ms

        for _ in 0..max_attempts {
            if self.check_server_health() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn check_server_health(&self) -> bool {
        // Try to connect to the server's port to verify it's listening
        match std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], 3000)),
            Duration::from_millis(100),
        ) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

// Implement safe shutdown for ServerState
impl Drop for ServerState {
    fn drop(&mut self) {
        // Shutdown the runtime gracefully
        if let Some(rt) = self.runtime.lock().unwrap().take() {
            rt.shutdown_background();
        }
    }
}

// Tauri commands for frontend communication
#[tauri::command]
fn show_window(window: tauri::Window) {
    if let Some(webview) = window.get_webview_window("main") {
        let _ = webview.show();
        let _ = webview.set_focus();
    }
}

#[tauri::command]
fn hide_window(window: tauri::Window) {
    if let Some(webview) = window.get_webview_window("main") {
        let _ = webview.hide();
    }
}

#[tauri::command]
fn is_window_visible(window: tauri::Window) -> bool {
    window
        .get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false)
}

#[tauri::command]
async fn get_server_port() -> Result<usize, String> {
    // Return the port where Axum server is running
    // In production, this should be read from config or state
    Ok(3000)
}

#[tauri::command]
fn open_devtools(window: tauri::Window) {
    if let Some(webview) = window.get_webview_window("main") {
        webview.open_devtools();
    }
}

fn create_tray_menu(app: &tauri::App) -> Result<tauri::tray::TrayIcon, Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let show = MenuItem::with_id(app, "show", "Show", true, None::<String>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide", true, None::<String>)?;
    let dev = MenuItem::with_id(app, "devtools", "DevTools", true, None::<String>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<String>)?;

    let menu = Menu::with_items(app, &[&show, &hide, &dev, &quit])?;

    let tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "devtools" => {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(tray)
}

fn start_axum_server(state: tauri::State<ServerState>) -> Result<(), String> {
    // Clone the Arc so we can move it into the thread
    let runtime_arc = Arc::clone(&state.runtime);

    // Start the Axum server in a background thread
    // This thread owns the runtime and keeps it alive
    let thread_handle = std::thread::spawn(move || {
        // Take ownership of the runtime
        let rt = runtime_arc.lock().unwrap()
            .take()
            .expect("Runtime not available");

        // Run the server - this blocks until the server stops
        rt.block_on(async {
            if let Err(e) = edge_api::start_server().await {
                eprintln!("Failed to start server: {}", e);
            }
        });
    });

    *state.server_thread.lock().unwrap() = Some(thread_handle);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let rt = Runtime::new().map_err(|e| format!("Failed to create runtime: {}", e)).unwrap();
    let server_state = ServerState {
        runtime: Arc::new(Mutex::new(Some(rt))),
        server_thread: Mutex::new(None),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .manage(server_state)
        .invoke_handler(tauri::generate_handler![
            show_window,
            hide_window,
            is_window_visible,
            get_server_port,
            open_devtools,
        ])
        .setup(|app| {
            // Create system tray and store it in global state to keep it alive
            let tray = match create_tray_menu(app) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Failed to create tray: {}", e);
                    return Ok(());
                }
            };
            // Store tray icon in app state so it lives for the app's lifetime
            app.manage(TrayState { _tray: Some(tray) });

            // Start Axum server in background
            let state = app.state::<ServerState>();
            if let Err(e) = start_axum_server(state) {
                eprintln!("Failed to start Axum server: {}", e);
            }

            // Wait for server to be ready (up to 10 seconds)
            let state = app.state::<ServerState>();
            if !state.wait_for_server_ready(10) {
                eprintln!("Server did not become ready in time");
            }

            // Handle window close event - minimize to tray instead of quitting
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            window.on_window_event(move |event| match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    // Prevent the window from closing
                    api.prevent_close();
                    // Hide the window instead
                    let _ = window_clone.hide();
                }
                _ => {}
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run()
}
