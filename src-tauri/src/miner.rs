use crate::api::KaspaApi;
use kaspa_consensus_core::block::Block;
use kaspa_pow::State as PowState;
use kaspa_rpc_core::RpcRawBlock;
use parking_lot::{Condvar, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

// Performance optimizations inspired by kaspanet/cpuminer:
// 1. Batch hash counting: Update atomic counter every BATCH_SIZE hashes instead of every hash
// 2. Reduced lock contention: Check for work updates every CHECK_WORK_INTERVAL hashes
// 3. Optimized hot path: Minimize branches and checks in the inner mining loop
// 4. Better nonce distribution: Use thread count as step size for optimal coverage
// 5. Throttle optimization: Apply throttle less frequently to reduce overhead

#[derive(Clone)]
pub struct CpuMinerConfig {
    pub mining_address: String,
    pub threads: usize,
    pub throttle: Option<Duration>,
    pub template_poll_interval: Duration,
}

pub struct CpuMinerMetrics {
    pub hashes_tried: Arc<AtomicU64>,
    pub blocks_submitted: Arc<AtomicU64>,
    pub blocks_accepted: Arc<AtomicU64>,
}

impl Default for CpuMinerMetrics {
    fn default() -> Self {
        Self {
            hashes_tried: Arc::new(AtomicU64::new(0)),
            blocks_submitted: Arc::new(AtomicU64::new(0)),
            blocks_accepted: Arc::new(AtomicU64::new(0)),
        }
    }
}

struct Work {
    id: u64,
    block: Block,
    rpc_block: RpcRawBlock,
    pow_state: Arc<PowState>,
}

struct WorkSlot {
    work: Option<Work>,
    version: u64,
}

struct SharedWork {
    slot: Mutex<WorkSlot>,
    cv: Condvar,
}

impl SharedWork {
    fn new() -> Self {
        Self {
            slot: Mutex::new(WorkSlot {
                work: None,
                version: 0,
            }),
            cv: Condvar::new(),
        }
    }

    fn publish(&self, work: Work) {
        let mut slot = self.slot.lock();
        slot.version = slot.version.wrapping_add(1);
        slot.work = Some(work);
        self.cv.notify_all();
    }

    fn wait_for_update(&self, last_seen: u64, shutdown_flag: &AtomicBool) -> (u64, Option<Work>) {
        let mut slot = self.slot.lock();
        while slot.version == last_seen && !shutdown_flag.load(Ordering::Acquire) {
            self.cv.wait(&mut slot);
        }
        if shutdown_flag.load(Ordering::Acquire) && slot.version == last_seen {
            return (last_seen, None);
        }
        (
            slot.version,
            slot.work.as_ref().map(|w| Work {
                id: w.id,
                block: w.block.clone(),
                rpc_block: w.rpc_block.clone(),
                pow_state: Arc::clone(&w.pow_state),
            }),
        )
    }

    fn notify_all(&self) {
        self.cv.notify_all();
    }
}

pub async fn start_cpu_miner(
    kaspa_api: Arc<KaspaApi>,
    config: CpuMinerConfig,
) -> Result<(Arc<CpuMinerMetrics>, watch::Sender<bool>), anyhow::Error> {
    if config.mining_address.trim().is_empty() {
        return Err(anyhow::anyhow!("mining address is required"));
    }

    let work = Arc::new(SharedWork::new());
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let shutdown_flag_clone = Arc::clone(&shutdown_flag);
    let work_clone = Arc::clone(&work);
    let mut shutdown_rx_clone = shutdown_rx.clone();
    tokio::spawn(async move {
        let _ = shutdown_rx_clone.wait_for(|v| *v).await;
        shutdown_flag_clone.store(true, Ordering::Release);
        work_clone.notify_all();
    });

    let metrics = Arc::new(CpuMinerMetrics::default());
    let metrics_submit = Arc::clone(&metrics);

    let (submit_tx, mut submit_rx) = mpsc::unbounded_channel::<RpcRawBlock>();
    let kaspa_api_submit = Arc::clone(&kaspa_api);
    let shutdown_flag_submit = Arc::clone(&shutdown_flag);
    tokio::spawn(async move {
        while let Some(rpc_block) = submit_rx.recv().await {
            if shutdown_flag_submit.load(Ordering::Acquire) {
                break;
            }
            let nonce = rpc_block.header.nonce;
            let res = kaspa_api_submit.submit_rpc_block(rpc_block).await;
            match res {
                Ok(response) => {
                    if response.report.is_success() {
                        metrics_submit
                            .blocks_submitted
                            .fetch_add(1, Ordering::Relaxed);
                        metrics_submit
                            .blocks_accepted
                            .fetch_add(1, Ordering::Relaxed);
                        tracing::info!("[Miner] Block accepted by node (nonce: {})", nonce);
                    } else {
                        tracing::warn!("[Miner] Block rejected by node: {:?}", response.report);
                    }
                }
                Err(e) => {
                    tracing::warn!("[Miner] Submit block failed: {e}");
                }
            }
        }
    });

    let work_publisher = Arc::clone(&work);
    let kaspa_api_templates = Arc::clone(&kaspa_api);
    let mining_address = config.mining_address.clone();
    let poll = config.template_poll_interval;
    let shutdown_flag_templates = Arc::clone(&shutdown_flag);
    let next_id = Arc::new(AtomicU64::new(0));
    let next_id_templates = Arc::clone(&next_id);
    tokio::spawn(async move {
        // Fetch template immediately on startup
        match kaspa_api_templates
            .get_block_template_rpc(&mining_address)
            .await
        {
            Ok((block, rpc_block)) => {
                let id = next_id_templates.fetch_add(1, Ordering::Relaxed);
                let header = block.header.clone();
                let pow_state = Arc::new(PowState::new(&header));
                work_publisher.publish(Work {
                    id,
                    block,
                    rpc_block,
                    pow_state,
                });
            }
            Err(e) => {
                tracing::warn!("[Miner] Initial get_block_template failed: {e}");
            }
        }

        let mut interval = tokio::time::interval(poll);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            if shutdown_flag_templates.load(Ordering::Acquire) {
                break;
            }
            interval.tick().await;
            if shutdown_flag_templates.load(Ordering::Acquire) {
                break;
            }

            match kaspa_api_templates
                .get_block_template_rpc(&mining_address)
                .await
            {
                Ok((block, rpc_block)) => {
                    let id = next_id_templates.fetch_add(1, Ordering::Relaxed);
                    let header = block.header.clone();
                    let pow_state = Arc::new(PowState::new(&header));
                    work_publisher.publish(Work {
                        id,
                        block,
                        rpc_block,
                        pow_state,
                    });
                }
                Err(e) => {
                    tracing::warn!("[Miner] Get_block_template failed: {e}");
                }
            }
        }
    });

    let threads = config.threads.max(1);
    let throttle = config.throttle;
    let found_counter = Arc::new(AtomicU64::new(0));

    // Optimization: Batch hash counting to reduce atomic operations
    // Update metrics every BATCH_SIZE hashes instead of every single hash
    const BATCH_SIZE: u64 = 1000;

    // Optimization: Check for work updates less frequently to reduce lock contention
    // Reduced to 250 for faster work updates (critical for high BPS networks like TN12 with 10 BPS)
    // At ~0.28 MH/s per thread, 250 hashes = ~0.9ms, ensuring work updates are detected within ~1ms
    // For single-threaded mining, this ensures minimal delay between finding blocks and getting new work
    // Optimization: Reduced to 200 for faster work detection without excessive lock contention
    // At ~0.28 MH/s per thread, 200 hashes = ~0.7ms check interval
    const CHECK_WORK_INTERVAL: u64 = 200;

    for thread_idx in 0..threads {
        let work = Arc::clone(&work);
        let submit_tx = submit_tx.clone();
        let shutdown_flag = Arc::clone(&shutdown_flag);
        let found_counter = Arc::clone(&found_counter);
        let metrics_threads = Arc::clone(&metrics);

        std::thread::spawn(move || {
            let mut last_version = 0u64;
            // Optimization: Use thread index as initial nonce offset for better distribution
            // Simple offset is faster than large prime multiplication
            let nonce_step = threads as u64;
            let mut nonce = thread_idx as u64;

            // Local hash counter to batch atomic updates
            let mut local_hash_count = 0u64;

            loop {
                if shutdown_flag.load(Ordering::Acquire) {
                    break;
                }

                let (ver, maybe_work) = work.wait_for_update(last_version, &shutdown_flag);
                last_version = ver;

                let Some(w) = maybe_work else {
                    continue;
                };

                // Optimization: Reset work check counter when new work arrives
                let mut hashes_since_work_check = 0u64;

                // Mining loop for current work
                loop {
                    // Increment local counter
                    local_hash_count += 1;
                    hashes_since_work_check += 1;

                    // Check PoW - this is the hot path, optimized for speed
                    // Increment nonce BEFORE checking to optimize branch prediction
                    let current_nonce = nonce;
                    nonce = nonce.wrapping_add(nonce_step);

                    let (passed, _) = w.pow_state.check_pow(current_nonce);
                    if passed {
                        // Batch update hash count before submitting
                        if local_hash_count > 0 {
                            metrics_threads
                                .hashes_tried
                                .fetch_add(local_hash_count, Ordering::Relaxed);
                            local_hash_count = 0;
                        }

                        // Optimization: Minimize cloning - only clone header and update nonce
                        // Transactions are already Arc'd internally, so clone is cheap
                        let mined_rpc_block = RpcRawBlock {
                            header: {
                                let mut h = w.rpc_block.header.clone();
                                h.nonce = current_nonce;
                                h
                            },
                            transactions: w.rpc_block.transactions.clone(), // Preserve original transactions with covenant data
                        };
                        let _ = submit_tx.send(mined_rpc_block);
                        found_counter.fetch_add(1, Ordering::Relaxed);

                        // Optimization: Quick work check after finding block (minimal lock time)
                        // Only check version number - if changed, we'll get new work in outer loop
                        // Use try_lock for non-blocking check - if lock is busy, skip check and continue mining
                        if let Some(slot) = work.slot.try_lock() {
                            if slot.version != last_version {
                                drop(slot);
                                break; // New work available, get it immediately
                            }
                            // Lock released here automatically
                        }
                        // No new work yet - continue mining current work (still valid)
                        // Reset counter to check more frequently for new work
                        hashes_since_work_check = 0;
                    }

                    // Batch update hash count periodically to reduce atomic operations
                    if local_hash_count >= BATCH_SIZE {
                        metrics_threads
                            .hashes_tried
                            .fetch_add(BATCH_SIZE, Ordering::Relaxed);
                        local_hash_count -= BATCH_SIZE;
                    }

                    // Apply throttle if configured (optimized: use counter instead of expensive modulo)
                    if let Some(d) = throttle {
                        // Use bitwise AND for power-of-2 check (faster than modulo)
                        // Check every 128 hashes (2^7) - use hashes_since_work_check for consistent throttling
                        if (hashes_since_work_check & 127) == 0 {
                            std::thread::sleep(d);
                        }
                    }

                    // Periodically check for shutdown or work updates (reduces lock contention)
                    if hashes_since_work_check >= CHECK_WORK_INTERVAL {
                        // Check shutdown first (cheap atomic read)
                        if shutdown_flag.load(Ordering::Acquire) {
                            // Update remaining hash count before exiting
                            if local_hash_count > 0 {
                                metrics_threads
                                    .hashes_tried
                                    .fetch_add(local_hash_count, Ordering::Relaxed);
                            }
                            return;
                        }

                        // Check if work has been updated (less frequent lock acquisition)
                        let slot = work.slot.lock();
                        if slot.version != last_version {
                            drop(slot);
                            // Update remaining hash count before getting new work
                            if local_hash_count > 0 {
                                metrics_threads
                                    .hashes_tried
                                    .fetch_add(local_hash_count, Ordering::Relaxed);
                                local_hash_count = 0;
                            }
                            break; // Break to outer loop to get new work
                        }
                        drop(slot);

                        // Reset counter for next batch
                        hashes_since_work_check = 0;
                    }
                }
            }

            // Final hash count update on thread exit
            if local_hash_count > 0 {
                metrics_threads
                    .hashes_tried
                    .fetch_add(local_hash_count, Ordering::Relaxed);
            }
        });
    }

    Ok((metrics, shutdown_tx))
}
