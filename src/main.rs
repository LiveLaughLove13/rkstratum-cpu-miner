use kaspa_cpu_miner_gui::AppState;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    // Create log collector
    let logs = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let logs_clone = Arc::clone(&logs);

    // Setup tracing subscriber that captures logs
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // Create a runtime handle for the log writer
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    let rt_handle = rt.handle().clone();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(move || LogWriter {
            logs: Arc::clone(&logs_clone),
            rt_handle: rt_handle.clone(),
        })
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Kaspa CPU Miner"),
        ..Default::default()
    };

    let mut app = kaspa_cpu_miner_gui::gui::MinerApp::default();
    app.logs = logs;

    eframe::run_native(
        "Kaspa CPU Miner",
        options,
        Box::new(move |_cc| Box::new(app)),
    )
}

// Custom writer that captures logs
struct LogWriter {
    logs: Arc<tokio::sync::Mutex<Vec<String>>>,
    rt_handle: tokio::runtime::Handle,
}

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(s) = std::str::from_utf8(buf) {
            let logs = Arc::clone(&self.logs);
            let lines: Vec<String> = s.lines().map(|l| l.to_string()).collect();

            if !lines.is_empty() {
                self.rt_handle.spawn(async move {
                    let mut logs_guard = logs.lock().await;
                    for line in lines {
                        if !line.trim().is_empty() {
                            logs_guard.push(line);
                        }
                    }
                    // Keep only last 1000 lines
                    while logs_guard.len() > 1000 {
                        logs_guard.remove(0);
                    }
                });
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
