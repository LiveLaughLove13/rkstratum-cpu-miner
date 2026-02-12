use anyhow::{Context, Result};
use kaspa_addresses::Address;
use kaspa_grpc_client::GrpcClient;
use kaspa_rpc_core::{
    GetBlockTemplateRequest, RpcRawBlock, SubmitBlockRequest, SubmitBlockResponse, api::rpc::RpcApi,
    notify::mode::NotificationMode,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Simplified Kaspa API client for standalone miner
pub struct KaspaApi {
    client: Arc<GrpcClient>,
}

impl KaspaApi {
    /// Create a new Kaspa API client
    pub async fn new(address: String) -> Result<Arc<Self>> {
        // Add grpc:// prefix if not present
        let grpc_address = if address.starts_with("grpc://") {
            address.clone()
        } else {
            format!("grpc://{}", address)
        };

        debug!("Connecting to Kaspa node at {}", grpc_address);

        let mut attempt = 0;
        let mut backoff_ms = 250u64;

        let client = loop {
            attempt += 1;
            let connect_fut = GrpcClient::connect_with_args(
                NotificationMode::Direct,
                grpc_address.clone(),
                None,
                true,
                None,
                false,
                Some(500_000),
                Default::default(),
            );

            match connect_fut.await {
                Ok(client) => break Arc::new(client),
                Err(e) => {
                    warn!(
                        "Failed to connect to kaspa node (attempt {}): {}, retrying in {:.2}s",
                        attempt,
                        e,
                        Duration::from_millis(backoff_ms).as_secs_f64()
                    );

                    sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms.saturating_mul(2)).min(5_000);
                }
            }
        };

        // Start the client
        client.start(None).await;

        debug!("Connected to Kaspa node successfully");

        Ok(Arc::new(Self { client }))
    }

    /// Wait for node to sync
    pub async fn wait_for_sync(&self) -> Result<()> {
        loop {
            match self.client.get_info().await {
                Ok(info) => {
                    if info.is_synced {
                        debug!("Node is synced");
                        return Ok(());
                    }
                    debug!("Node not synced yet, waiting...");
                }
                Err(e) => {
                    warn!("Error checking sync status: {}", e);
                }
            }
            sleep(Duration::from_secs(2)).await;
        }
    }

    /// Get block template for mining (with retry logic matching rkstratum_cpu_miner.rs)
    pub async fn get_block_template_rpc(
        &self,
        mining_address: &str,
    ) -> Result<(kaspa_consensus_core::block::Block, RpcRawBlock)> {
        // Retry up to 3 times if we get "Odd number of digits" error
        // This error can occur if the block template has malformed hash fields
        let max_retries = 3;
        let mut last_error: Option<String> = None;

        for attempt in 0..max_retries {
            // Parse wallet address each time (in case Address doesn't implement Clone)
            let address = Address::try_from(mining_address)
                .map_err(|e| anyhow::anyhow!("Could not decode address {}: {}", mining_address, e))?;

            // Request block template using RPC client wrapper
            let response = match self
                .client
                .get_block_template_call(None, GetBlockTemplateRequest::new(address, b"internal".to_vec()))
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    if attempt < max_retries - 1 {
                        warn!("Failed to get block template (attempt {}/{}): {}, retrying...", attempt + 1, max_retries, e);
                        sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                        continue;
                    }
                    return Err(anyhow::anyhow!("Failed to get block template after {} attempts: {}", max_retries, e));
                }
            };

            // Get RPC block from response (preserve original with covenant data)
            let rpc_block = response.block.clone();

            // Convert RpcRawBlock to Block for PoW validation
            // The "Odd number of digits" error can occur here if hash fields have malformed hex strings
            match kaspa_consensus_core::block::Block::try_from(rpc_block.clone()) {
                Ok(block) => {
                    return Ok((block, rpc_block));
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    last_error = Some(error_msg.clone());
                    if attempt < max_retries - 1 {
                        warn!("Failed to convert RPC block to Block (attempt {}/{}): {}, retrying...", attempt + 1, max_retries, error_msg);
                        sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                        continue;
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Failed to get block template after {} attempts: {}",
            max_retries,
            last_error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }

    /// Submit a mined block
    pub async fn submit_rpc_block(&self, rpc_block: RpcRawBlock) -> Result<SubmitBlockResponse> {
        let request = SubmitBlockRequest::new(rpc_block, false);
        self.client
            .submit_block_call(None, request)
            .await
            .context("Failed to submit block")
    }
}

