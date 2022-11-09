use super::*;
use crate::{
    context::TestContext,
    init::info::Information,
    tools::{messages::WaitForMessages, tls, Auth},
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use uuid::Uuid;

struct MqttMessageDispatcher {
    receiver_map: HashMap<String, Vec<MqttMessage>>,
}

pub struct MqttDevice {
    client: paho_mqtt::AsyncClient,
    dispatcher: Arc<Mutex<MqttMessageDispatcher>>,
}

impl MqttDevice {
    pub async fn new(
        info: &Information,
        auth: Auth,
        variant: MqttVariant,
        ctx: &mut TestContext,
    ) -> anyhow::Result<Self> {
        let client_id = Uuid::new_v4().to_string();

        let uri = match variant {
            MqttVariant::Plain(_) => format!("ssl://{}:{}", info.mqtt.host, info.mqtt.port),
            MqttVariant::WebSocket(_) => scrub_uri(&info.mqtt_ws.url),
        };

        let create_opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(uri)
            .client_id(client_id)
            .persistence(paho_mqtt::PersistenceType::None);

        let create_opts = match variant.version() {
            MqttVersion::V3_1_1 => create_opts.mqtt_version(paho_mqtt::MQTT_VERSION_3_1_1),
            MqttVersion::V5(_) => create_opts.mqtt_version(paho_mqtt::MQTT_VERSION_5),
        };

        let client = paho_mqtt::AsyncClient::new(create_opts.finalize())
            .context("Failed to create client")?;

        let mut ssl_opts = paho_mqtt::SslOptionsBuilder::new();

        ssl_opts.enable_server_cert_auth(true).verify(true);

        ssl_opts
            .trust_store(tls::default_ca_certs_path()?)
            .context("Failed to load CA bundle for MQTT sender client")?;

        let mut conn_opts = paho_mqtt::ConnectOptionsBuilder::new();

        match auth {
            Auth::None => {}
            Auth::UsernamePassword(username, password) => {
                conn_opts.user_name(username).password(password);
            }
            Auth::X509Certificate(cert) => {
                let file = ctx.create_temp_file(cert.as_slice())?;
                ssl_opts.key_store(file)?;
                // FIXME: need to actually test once `drg` is ready for this
            }
        }

        conn_opts
            .ssl_options(ssl_opts.finalize())
            .keep_alive_interval(Duration::from_secs(30))
            .automatic_reconnect(Duration::from_millis(100), Duration::from_secs(5));

        variant.version().apply(&mut conn_opts);

        let dispatcher = Arc::new(Mutex::new(MqttMessageDispatcher {
            receiver_map: HashMap::new(),
        }));

        let receiver_messages = dispatcher.clone();
        client.set_message_callback(move |_, msg| {
            log::debug!("Got message: {:?}", msg);
            if let Some(msg) = msg {
                if let Ok(mut messages) = receiver_messages.lock() {
                    let buffer = messages
                        .receiver_map
                        .entry(msg.topic().to_string())
                        .or_insert_with(std::vec::Vec::new);
                    let msg = msg.into();
                    log::info!("Received on device: {:#?}", msg);
                    buffer.push(msg);
                }
            }
        });

        client
            .connect(conn_opts.finalize())
            .await
            .context("Failed to connect")?;

        Ok(Self { client, dispatcher })
    }

    pub async fn send(
        &self,
        topic: String,
        qos: MqttQoS,
        content_type: String,
        payload: Option<Vec<u8>>,
    ) -> anyhow::Result<()> {
        let mut props = paho_mqtt::Properties::new();
        props.push_string(paho_mqtt::PropertyCode::ContentType, &content_type)?;

        let msg = paho_mqtt::MessageBuilder::new()
            .topic(topic)
            .payload(payload.unwrap_or_default())
            .qos(qos.into())
            .properties(props);

        Ok(self.client.try_publish(msg.finalize())?.await?)
    }

    /// Subscribe to the command topic
    pub async fn subscribe_commands(&mut self, filter: &str) -> anyhow::Result<()> {
        self.client.subscribe(filter, MqttQoS::QoS0.into()).await?;
        Ok(())
    }

    pub fn fetch_messages(&self) -> anyhow::Result<HashMap<String, Vec<MqttMessage>>> {
        if let Ok(dispatcher) = self.dispatcher.lock() {
            Ok(dispatcher.receiver_map.clone())
        } else {
            Err(anyhow!("Failed to lock dispatcher"))
        }
    }
}

#[async_trait]
impl WaitForMessages for MqttDevice {
    async fn num_messages(&self) -> usize {
        self.dispatcher
            .lock()
            .map_or(0, |m| m.receiver_map.iter().map(|(_, v)| v.len()).sum())
    }
}
