use super::*;
use crate::tools::{assert::Message, messages::WaitForMessages, tls};
use anyhow::Context;
use futures::StreamExt;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MqttMessage {
    pub topic: String,
    pub user_properties: HashMap<String, String>,
    pub content_type: Option<String>,
    pub payload: Vec<u8>,
}

impl MqttMessage {
    pub fn into_message(self, binary: bool) -> anyhow::Result<Message> {
        match binary {
            true => self.into_message_binary(),
            false => self.into_message_structured(),
        }
    }

    pub fn into_message_structured(self) -> anyhow::Result<Message> {
        if let Some(content_type) = self.content_type {
            if content_type.as_str() != "application/cloudevents+json; charset=utf-8" {
                anyhow::bail!("Wrong content type: {}", content_type);
            }
        }

        let json: Value = serde_json::from_slice(&self.payload).context("Parse as JSON")?;

        let payload = match json["data_base64"].as_str() {
            Some(data) => base64::decode(data)?,
            None => vec![],
        };

        Ok(Message {
            subject: json["subject"].as_str().unwrap_or_default().into(),
            r#type: json["type"].as_str().unwrap_or_default().into(),
            instance: json["instance"].as_str().unwrap_or_default().into(),
            app: json["application"].as_str().unwrap_or_default().into(),
            device: json["device"].as_str().unwrap_or_default().into(),
            content_type: json["datacontenttype"].as_str().map(|s| s.into()),
            payload,
        })
    }

    pub fn into_message_binary(self) -> anyhow::Result<Message> {
        Ok(Message {
            subject: self
                .user_properties
                .get("subject")
                .map(Into::into)
                .unwrap_or_default(),
            r#type: self
                .user_properties
                .get("type")
                .map(Into::into)
                .unwrap_or_default(),
            instance: self
                .user_properties
                .get("instance")
                .map(Into::into)
                .unwrap_or_default(),
            app: self
                .user_properties
                .get("application")
                .map(Into::into)
                .unwrap_or_default(),
            device: self
                .user_properties
                .get("device")
                .map(Into::into)
                .unwrap_or_default(),
            content_type: self.content_type,
            payload: self.payload,
        })
    }
}

impl From<paho_mqtt::Message> for MqttMessage {
    fn from(msg: paho_mqtt::Message) -> Self {
        Self {
            topic: msg.topic().into(),
            user_properties: msg.properties().user_iter().collect::<HashMap<_, _>>(),
            content_type: msg
                .properties()
                .get_string(paho_mqtt::PropertyCode::ContentType),
            payload: msg.payload().into(),
        }
    }
}

pub struct MqttReceiver {
    messages: Arc<Mutex<Vec<anyhow::Result<Message>>>>,
    _client: paho_mqtt::AsyncClient,
    _ctx: JoinHandle<()>,
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
                if let Ok(mut msgs) = strm_messages.lock() {
                    log::info!("Raw message: {:?}", msg);
                    let msg = MqttMessage::from(msg);
                    log::info!("Received: {:?}", msg);

                    msgs.push(msg.into_message(binary));
                }
            }
        });

        Ok(Self {
            messages,
            _client: client,
            _ctx: ctx,
        })
    }

    pub fn close(self) -> Vec<anyhow::Result<Message>> {
        if let Ok(mut msgs) = self.messages.lock() {
            msgs.drain(..).collect()
        } else {
            log::warn!("Unable to get messages");
            vec![]
        }
    }
}

impl WaitForMessages for MqttReceiver {
    fn num_messages(&self) -> usize {
        self.messages.lock().map_or(0, |m| m.len())
    }
}

impl Drop for MqttReceiver {
    fn drop(&mut self) {
        log::info!("Dropping MQTT receiver");
        self._ctx.abort();
        self._client.disconnect(None);
    }
}
