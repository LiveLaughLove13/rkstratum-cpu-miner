#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod miner;

use api::KaspaApi;
use miner::{start_cpu_miner, CpuMinerConfig, CpuMinerMetrics};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{
    fmt::{format::Writer, time::FormatTime, MakeWriter},
    layer::Layer,
    EnvFilter,
};

struct MinerState {
    api: Arc<Mutex<Option<Arc<KaspaApi>>>>,
    metrics: Arc<Mutex<Option<Arc<CpuMinerMetrics>>>>,
    shutdown: Arc<Mutex<Option<tokio::sync::watch::Sender<bool>>>>,
}

// Global app handle for log emission (set during setup)
use std::sync::OnceLock;
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

// Custom time formatter for logs
struct LocalTimer;

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        use chrono::Local;
        write!(w, "{}", Local::now().format("%-I:%M:%S %p"))
    }
}

#[tauri::command]
async fn connect_node(address: String, state: State<'_, MinerState>) -> Result<String, String> {
    let api = KaspaApi::new(address.clone())
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    api.wait_for_sync()
        .await
        .map_err(|e| format!("Failed to sync: {}", e))?;

    *state.api.lock().await = Some(api);
    Ok("Connected and synced".to_string())
}

#[tauri::command]
async fn start_mining(
    mining_address: String,
    threads: usize,
    throttle_ms: Option<u64>,
    state: State<'_, MinerState>,
) -> Result<String, String> {
    let api = {
        let api_guard = state.api.lock().await;
        api_guard
            .as_ref()
            .ok_or_else(|| "Not connected to node".to_string())?
            .clone()
    };

    let config = CpuMinerConfig {
        mining_address,
        threads: threads.max(1),
        throttle: throttle_ms.map(Duration::from_millis),
        // Optimization: Use 50ms poll interval for high BPS networks like TN12 (10 BPS)
        // This ensures we get new work quickly when blocks are found
        template_poll_interval: Duration::from_millis(50),
    };

    let (metrics, shutdown) = start_cpu_miner(api, config)
        .await
        .map_err(|e| format!("Failed to start miner: {}", e))?;

    *state.metrics.lock().await = Some(metrics);
    *state.shutdown.lock().await = Some(shutdown);

    Ok("Mining started".to_string())
}

#[tauri::command]
async fn disconnect_node(state: State<'_, MinerState>) -> Result<String, String> {
    // Stop mining first if running
    {
        let shutdown = {
            let mut shutdown_guard = state.shutdown.lock().await;
            shutdown_guard.take()
        };
        if let Some(shutdown) = shutdown {
            let _ = shutdown.send(true);
            *state.metrics.lock().await = None;
        }
    }

    // Clear API connection
    *state.api.lock().await = None;
    Ok("Disconnected".to_string())
}

#[tauri::command]
async fn stop_mining(state: State<'_, MinerState>) -> Result<String, String> {
    let shutdown = {
        let mut shutdown_guard = state.shutdown.lock().await;
        shutdown_guard.take()
    };

    if let Some(shutdown) = shutdown {
        let _ = shutdown.send(true);
        *state.metrics.lock().await = None;
        Ok("Mining stopped".to_string())
    } else {
        Err("Miner not running".to_string())
    }
}

#[tauri::command]
async fn get_metrics(state: State<'_, MinerState>) -> Result<serde_json::Value, String> {
    let metrics_guard = state.metrics.lock().await;
    if let Some(metrics) = metrics_guard.as_ref() {
        Ok(serde_json::json!({
            "hashes_tried": metrics.hashes_tried.load(std::sync::atomic::Ordering::Relaxed),
            "blocks_submitted": metrics.blocks_submitted.load(std::sync::atomic::Ordering::Relaxed),
            "blocks_accepted": metrics.blocks_accepted.load(std::sync::atomic::Ordering::Relaxed),
        }))
    } else {
        Err("Miner not running".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            // Store app handle globally for log emission
            let app_handle = app.handle().clone();
            APP_HANDLE.set(app_handle.clone()).ok();

            // Initialize tracing with custom layer that emits Tauri events
            let filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

            // Create a custom layer that emits to Tauri events
            let tauri_layer = tracing_subscriber::fmt::layer()
                .with_timer(LocalTimer)
                .with_writer(TauriLogWriter)
                .with_filter(filter.clone());

            // Also log to stdout for debugging
            let stdout_layer = tracing_subscriber::fmt::layer()
                .with_timer(LocalTimer)
                .with_filter(filter);

            tracing_subscriber::registry()
                .with(tauri_layer)
                .with(stdout_layer)
                .init();

            Ok(())
        })
        .manage(MinerState {
            api: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(None)),
            shutdown: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            connect_node,
            start_mining,
            stop_mining,
            get_metrics,
            disconnect_node
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Custom writer that emits logs to Tauri frontend
struct TauriLogWriter;

impl<'a> MakeWriter<'a> for TauriLogWriter {
    type Writer = TauriLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        TauriLogWriter
    }
}

impl std::io::Write for TauriLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let message = String::from_utf8_lossy(buf);
        let trimmed = message.trim();
        if !trimmed.is_empty() {
            // Emit log event to frontend using global app handle
            if let Some(app_handle) = APP_HANDLE.get() {
                let _ = app_handle.emit("log", trimmed);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
