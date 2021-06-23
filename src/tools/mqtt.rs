use anyhow::Context;
use futures::StreamExt;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::time::Instant;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MqttVersion {
    V3_1_1,
    V5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MqttQoS {
    QoS0,
    QoS1,
    QoS2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MqttMessage {
    pub topic: String,
    pub user_properties: HashMap<String, String>,
    pub payload: Vec<u8>,
}

pub struct MqttReceiver {
    messages: Arc<RwLock<Vec<MqttMessage>>>,
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
            .persistence(paho_mqtt::PersistenceType::None)
            .finalize();

        let mut client =
            paho_mqtt::AsyncClient::new(create_opts).context("Failed to create client")?;

        let ssl_opts = paho_mqtt::SslOptionsBuilder::new()
            .enable_server_cert_auth(false)
            .verify(false)
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
            .mqtt_version(match version {
                MqttVersion::V3_1_1 => paho_mqtt::MQTT_VERSION_3_1_1,
                MqttVersion::V5 => paho_mqtt::MQTT_VERSION_5,
            })
            .clean_session(true);

        let mut strm = client.get_stream(100);

        client
            .connect(conn_opts.finalize())
            .await
            .context("Failed to connect")?;

        client
            .subscribe(
                topic,
                match qos {
                    MqttQoS::QoS0 => 0,
                    MqttQoS::QoS1 => 1,
                    MqttQoS::QoS2 => 2,
                },
            )
            .await
            .context("Failed to subscribe")?;

        let messages = Arc::new(RwLock::new(Vec::new()));
        let strm_messages = messages.clone();

        let ctx = tokio::spawn(async move {
            log::info!("Starting message stream...");
            // we don't reconnect
            while let Some(Some(ref msg)) = strm.next().await {
                if let Ok(mut msgs) = strm_messages.write() {
                    msgs.push(MqttMessage {
                        topic: msg.topic().into(),
                        user_properties: HashMap::from_iter(msg.properties().user_iter()),
                        payload: msg.payload().into(),
                    });
                }
            }
        });

        Ok(Self {
            messages,
            _client: client,
            _ctx: ctx,
        })
    }

    fn num_messages(&self) -> usize {
        self.messages.read().map_or(0, |m| m.len())
    }

    pub async fn wait_for_messages(&self, num: usize, timeout: Duration) -> Result<(), ()> {
        let start = Instant::now();
        while self.num_messages() < num {
            tokio::time::sleep(Duration::from_millis(250)).await;
            if start.elapsed() > timeout {
                return Err(());
            }
        }
        Ok(())
    }

    pub fn close(self) -> Vec<MqttMessage> {
        if let Ok(msgs) = self.messages.read() {
            msgs.clone()
        } else {
            log::warn!("Unable to get messages");
            vec![]
        }
    }
}

impl Drop for MqttReceiver {
    fn drop(&mut self) {
        log::info!("Dropping MQTT receiver");
        self._ctx.abort();
        self._client.disconnect(None);
    }
}
