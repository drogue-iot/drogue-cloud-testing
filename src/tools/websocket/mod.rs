use anyhow::{Context, Result};
use async_std::sync::Mutex;
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;
use std::fmt::Display;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::tools::assert::CloudMessage;
use crate::tools::messages::WaitForMessages;
use crate::tools::warmup::WarmupSender;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use url::Url;

pub struct WebSocketReceiver {
    messages: Arc<Mutex<Vec<Result<CloudMessage>>>>,
    _ctx: JoinHandle<()>,
}

impl WebSocketReceiver {
    pub async fn new<S>(uri: Url, token: S, app: &str) -> Result<Self>
    where
        S: Display,
    {
        let mut address = uri.join(app)?;
        address.set_query(Some(format!("token={}", token).as_str()));

        let (stream, _) = connect_async(address)
            .await
            .context("Error connecting to the Websocket endpoint:")?;
        let (_, read) = stream.split();

        log::info!("WebSocket handshake successful");

        let messages = Arc::new(Mutex::new(Vec::new()));
        let strm_messages = messages.clone();

        log::info!("Starting message stream...");
        let ctx = tokio::spawn(async move {
            read.for_each(|msg| async {
                if let Ok(m) = msg {
                    let mut msgs = strm_messages.lock().await;
                    // ignore protocol messages, only show text
                    if m.is_text() {
                        let message = m.into_text().expect("Invalid message");
                        log::info!("Raw message: {:?}", &message);
                        let json: Value =
                            serde_json::from_str(message.as_str()).expect("Parse as JSON");
                        log::info!("Json message: {:?}", &json);
                        msgs.push(Ok(CloudMessage::from(json)));
                    }
                }
            })
            .await
        });

        Ok(WebSocketReceiver {
            messages,
            _ctx: ctx,
        })
    }

    pub async fn drain_messages(&self) -> Vec<anyhow::Result<CloudMessage>> {
        self.messages.lock().await.drain(..).collect()
    }

    // Warms up the listener, to ensure we can receive data.
    pub async fn warmup<S>(self, mut sender: S, timeout: Duration) -> anyhow::Result<Self>
    where
        S: WarmupSender,
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

#[async_trait]
impl WaitForMessages for WebSocketReceiver {
    async fn num_messages(&self) -> usize {
        self.messages.lock().await.len()
    }
}

impl Drop for WebSocketReceiver {
    fn drop(&mut self) {
        log::info!("Dropping websocket receiver");
        self._ctx.abort();
    }
}
