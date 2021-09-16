use async_trait::async_trait;
use std::time::{Duration, Instant};

/// Helps waiting for messages
#[async_trait]
pub trait WaitForMessages {
    /// Get the current number of messages.
    async fn num_messages(&self) -> usize;

    /// Wait until the number of messages exceeds the requirement, or the operation times out.
    async fn wait_for_messages(&self, num: usize, timeout: Duration) -> anyhow::Result<()> {
        let start = Instant::now();
        while self.num_messages().await < num {
            tokio::time::sleep(Duration::from_millis(250)).await;
            if start.elapsed() > timeout {
                anyhow::bail!("Time elapsed")
            }
        }
        Ok(())
    }
}
