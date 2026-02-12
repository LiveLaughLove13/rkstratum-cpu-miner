pub mod api;
pub mod gui;
pub mod miner;
pub mod ui;

pub use api::KaspaApi;
pub use miner::{CpuMinerConfig, CpuMinerMetrics};

// Re-export StatusType for UI modules
#[derive(Clone, PartialEq)]
pub enum StatusType {
    Info,
    Success,
    Error,
}

// AppState - application state structure
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub api: Arc<Mutex<Option<Arc<KaspaApi>>>>,
    pub metrics: Arc<Mutex<Option<Arc<CpuMinerMetrics>>>>,
    pub shutdown: Arc<Mutex<Option<tokio::sync::watch::Sender<bool>>>>,
    pub node_address: String,
    pub mining_address: String,
    pub threads: usize,
    pub throttle_ms: Option<u64>,
    pub status_message: String,
    pub status_type: StatusType,
    pub is_connected: bool,
    pub is_mining: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            api: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(None)),
            shutdown: Arc::new(Mutex::new(None)),
            node_address: "127.0.0.1:16210".to_string(),
            mining_address: String::new(),
            threads: 1,
            throttle_ms: None,
            status_message: String::new(),
            status_type: StatusType::Info,
            is_connected: false,
            is_mining: false,
        }
    }
}
