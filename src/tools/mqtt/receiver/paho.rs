use super::*;
use crate::tools::{assert::CloudMessage, messages::WaitForMessages, tls};
use anyhow::Context;
use async_std::sync::Mutex;
use async_trait::async_trait;
use futures::StreamExt;
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use uuid::Uuid;

pub struct MqttReceiver {
    messages: Arc<Mutex<Vec<anyhow::Result<CloudMessage>>>>,
    client: paho_mqtt::AsyncClient,
    ctx: JoinHandle<()>,
}

impl MqttReceiver {
    pub async fn new<U, S>(
        uri: U,
        username: Option<String>,
        password: Option<String>,
        version: MqttVersion,
        topic: S,
        qos: MqttQoS,
    ) -> anyhow::Result<Self>
    where
        U: Into<String>,
        S: Into<String>,
    {
        let client_id = Uuid::new_v4().to_string();

        let create_opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(uri)
            .client_id(client_id)
            .persistence(paho_mqtt::PersistenceType::None);

        let create_opts = match version {
            MqttVersion::V3_1_1 => create_opts.mqtt_version(paho_mqtt::MQTT_VERSION_3_1_1),
            MqttVersion::V5(_) => create_opts.mqtt_version(paho_mqtt::MQTT_VERSION_5),
        };

        let mut client = paho_mqtt::AsyncClient::new(create_opts.finalize())
            .context("Failed to create client")?;

        let ssl_opts = paho_mqtt::SslOptionsBuilder::new()
            .trust_store(tls::default_ca_certs_path()?)
            .context("Failed to load CA bundle for MQTT device client")?
            .finalize();

        let mut conn_opts = paho_mqtt::ConnectOptionsBuilder::new();
        conn_opts.ssl_options(ssl_opts);
        if let Some(username) = username {
            conn_opts.user_name(username);
        };
        if let Some(password) = password {
            conn_opts.password(password);
        };

        conn_opts
            .keep_alive_interval(Duration::from_secs(30))
            .automatic_reconnect(Duration::from_millis(100), Duration::from_secs(5));

        version.apply(&mut conn_opts);

        let mut strm = client.get_stream(100);

        client
            .connect(conn_opts.finalize())
            .await
            .context("Failed to connect")?;

        match version {
            MqttVersion::V5(true) => {
                let mut props = paho_mqtt::Properties::new();
                props.push_string_pair(
                    paho_mqtt::PropertyCode::UserProperty,
                    "content-mode",
                    "binary",
                )?;
                client
                    .subscribe_with_options(topic, qos.into(), false, Some(props))
                    .await
                    .context("Failed to subscribe")?;
            }
            _ => {
                client
                    .subscribe(topic, qos.into())
                    .await
                    .context("Failed to subscribe")?;
            }
        }

        let messages = Arc::new(Mutex::new(Vec::new()));
        let strm_messages = messages.clone();

        let binary = version.is_binary();

        let ctx = tokio::spawn(async move {
            log::info!("Starting message stream...");
            // we don't reconnect
            while let Some(Some(msg)) = strm.next().await {
                {
                    let mut msgs = strm_messages.lock().await;
                    log::info!("Raw message: {:?}", msg);
                    let msg = MqttMessage::from(msg);
                    log::info!("Received: {:?}", msg);

                    msgs.push(msg.into_message(binary));
                }
            }
        });

        Ok(Self {
            messages,
            client,
            ctx,
        })
    }

    async fn drain_messages(&self) -> Vec<anyhow::Result<CloudMessage>> {
        self.messages.lock().await.drain(..).collect()
    }

    pub async fn close(self) -> Vec<anyhow::Result<CloudMessage>> {
        self.drain_messages().await
    }
}

#[async_trait]
impl WaitForMessages for MqttReceiver {
    async fn num_messages(&self) -> usize {
        self.messages.lock().await.len()
    }
}

#[async_trait]
impl WarmupReceiver for MqttReceiver {
    async fn drain_messages(&self) -> Vec<anyhow::Result<CloudMessage>> {
        self.messages.lock().await.drain(..).collect()
    }
}

impl Drop for MqttReceiver {
    fn drop(&mut self) {
        log::info!("Dropping MQTT receiver");
        self.ctx.abort();
        self.client.disconnect(None);
    }
}
