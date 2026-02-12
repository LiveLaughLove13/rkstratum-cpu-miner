use anyhow::{Context, Result};
use kaspa_addresses::Address;
use kaspa_grpc_client::GrpcClient;
use kaspa_rpc_core::{
    api::rpc::RpcApi, notify::mode::NotificationMode, GetBlockTemplateRequest, RpcRawBlock,
    SubmitBlockRequest, SubmitBlockResponse,
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

    /// Get block template for mining
    pub async fn get_block_template_rpc(
        &self,
        mining_address: &str,
    ) -> Result<(kaspa_consensus_core::block::Block, RpcRawBlock)> {
        // Parse address string to Address type
        let address = Address::try_from(mining_address)
            .map_err(|e| anyhow::anyhow!("Invalid mining address {}: {}", mining_address, e))?;

        // Convert extra_data string to Vec<u8>
        let extra_data = b"internal".to_vec();

        let request = GetBlockTemplateRequest::new(address, extra_data);

        let response = self
            .client
            .get_block_template_call(None, request)
            .await
            .context("Failed to get block template")?;

        // Convert RpcRawBlock to Block
        let block = kaspa_consensus_core::block::Block::try_from(response.block.clone())
            .context("Failed to convert RPC block to Block")?;

        Ok((block, response.block))
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
