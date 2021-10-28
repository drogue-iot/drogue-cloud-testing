use std::fmt::Display;
use std::sync::Arc;
use async_std::sync::Mutex;
use async_trait::async_trait;
use anyhow::{Context, Result};
use serde_json::Value;

use tokio::task::JoinHandle;
use url::Url;
use tungstenite::connect;
use crate::tools::assert::CloudMessage;
use crate::tools::messages::WaitForMessages;


pub struct WebSocketReceiver {
    messages: Arc<Mutex<Vec<Result<CloudMessage>>>>,
    _ctx: JoinHandle<()>,
}


impl WebSocketReceiver {
    pub fn new<S>(uri: Url, token: S, app: &str) -> Result<Self>
        where
            S: Display {
        let mut address = uri.join(app)?;
        address.set_query(Some(format!("token={}", token).as_str()));

        let (mut socket, _) =
            connect(address).context("Error connecting to the Websocket endpoint:")?;

        log::info!("WebSocket handshake successful");

        let messages = Arc::new(Mutex::new(Vec::new()));
        let strm_messages = messages.clone();

        let ctx = tokio::spawn(async move {
            log::info!("Starting message stream...");

            loop {
                let msg = socket.read_message();
                if let Ok(m) = msg {
                    let mut msgs = strm_messages.lock().await;
                    // ignore protocol messages, only show text
                    if m.is_text() {
                        let message = m.into_text().expect("Invalid message");
                        log::info!("Raw message: {:?}", &message);
                        let json: Value = serde_json::from_str(message.as_str()).expect("Parse as JSON");
                        log::info!("Json message: {:?}", &json);
                        msgs.push(Ok(CloudMessage::from(json)));
                    }
                }
            }
        });

        Ok(WebSocketReceiver {
            messages,
            _ctx: ctx,
        })
    }

    pub async fn drain_messages(&self) -> Vec<anyhow::Result<CloudMessage>> {
        self.messages.lock().await.drain(..).collect()
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