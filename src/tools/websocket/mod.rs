use crate::tools::{assert::CloudMessage, messages::WaitForMessages, mqtt::WarmupReceiver};
use anyhow::{Context, Result};
use async_std::sync::Mutex;
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;
use std::fmt::Display;
use std::sync::Arc;
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
}

#[async_trait]
impl WaitForMessages for WebSocketReceiver {
    async fn num_messages(&self) -> usize {
        self.messages.lock().await.len()
    }
}

#[async_trait]
impl WarmupReceiver for WebSocketReceiver {
    async fn drain_messages(&self) -> Vec<anyhow::Result<CloudMessage>> {
        self.messages.lock().await.drain(..).collect()
    }
}

impl Drop for WebSocketReceiver {
    fn drop(&mut self) {
        log::info!("Dropping websocket receiver");
        self._ctx.abort();
    }
}
