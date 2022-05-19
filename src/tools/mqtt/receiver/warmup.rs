use crate::tools::assert::CloudMessage;
use crate::tools::messages::WaitForMessages;
use crate::tools::warmup::WarmupSender;
use async_trait::async_trait;
use std::time::{Duration, SystemTime};

#[async_trait]
pub trait WarmupReceiver: WaitForMessages + Sized + Send + Sync {
    async fn drain_messages(&self) -> Vec<anyhow::Result<CloudMessage>>;

    // Warms up the listener, to ensure we can receive data.
    async fn warmup<S>(self, mut sender: S, timeout: Duration) -> anyhow::Result<Self>
    where
        S: WarmupSender + Send + Sync,
    {
        let start = SystemTime::now();

        let mut index = 0;

        // first drain messages
        self.drain_messages().await;

        loop {
            // start sending
            sender.send(index).await?;

            // sleep a bit
            tokio::time::sleep(Duration::from_secs(1)).await;

            // check if we have messages
            if self.num_messages().await > 0 {
                log::info!("Received first message after {} attempts", index);
                break;
            }

            // check if we timed out
            if SystemTime::now().duration_since(start)? > timeout {
                log::info!("Timeout waiting for first warmup message");
                anyhow::bail!("Unable to warm up listener: Timeout");
            }

            // next iteration, try again
            index += 1;
        }

        // we received messages, now read up to the most recent one sent (index)

        'for_latest: loop {
            let messages = self.drain_messages().await;

            // iterate over all "ok" messages
            for message in messages.into_iter().flatten() {
                log::debug!("Received warmup message: {:?}", index);
                if sender.identify(&message, index as u32) {
                    log::info!("Received most recent messages ... warmed up!");
                    // we identified the latest messages and can break the loop
                    break 'for_latest;
                }
            }

            // sleep a bit
            tokio::time::sleep(Duration::from_secs(1)).await;

            // check if we timed out
            if SystemTime::now().duration_since(start)? > timeout {
                log::info!("Timeout waiting for warmup cleanup");
                anyhow::bail!("Unable to warm up listener: Timeout");
            }

            // check again
        }

        Ok(self)
    }
}
