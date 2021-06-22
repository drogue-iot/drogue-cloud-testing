use crate::init::token::TokenProvider;
use crate::init::url::UrlExt;
use crate::{context::TestContext, resources::apps::Application};
use anyhow::Context;
use futures::StreamExt;
use paho_mqtt::{DisconnectOptionsBuilder, SslOptions};
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use test_context::test_context;
use tokio::task::JoinHandle;
use uuid::Uuid;

#[test_context(TestContext)]
#[tokio::test]
async fn test_send_telemetry(ctx: &mut TestContext) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let info = ctx.info().await?;

    let app = Application::new_random(drg.clone()).expect("Create a new application");
    let device = app
        .create_device(
            "device1",
            &json!({
                "credentials": {
                    "credentials": [
                        { "pass": "foo" }
                    ]
                }
            }),
        )
        .expect("Create new device");

    let uri = format!(
        "ssl://{}:{}",
        info.mqtt_integration.host, info.mqtt_integration.port
    );

    log::info!("MQTT integration URL: {}", uri);

    let mqtt = MqttReceiver::new(
        uri,
        None,
        Some(drg.current_token().await?),
        MqttVersion::V3_1_1,
        format!("app/{}", app.name()),
        MqttQoS::QoS0,
    )
    .await
    .unwrap();

    log::info!("Receiver created");

    tokio::time::sleep(Duration::from_secs(5)).await;

    // do some work

    let http = ctx.client().await?;
    let http = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    let mut url = info.http.url;

    url.set_path("/v1/telemetry");

    log::info!("Sending payload");

    let response = http
        .post(url)
        .basic_auth(format!("{}@{}", device.name(), app.name()), Some("foo"))
        .send()
        .await
        .expect("HTTP call to succeed");

    assert!(response.status().is_success());

    log::info!("Payload sent");

    tokio::time::sleep(Duration::from_secs(5)).await;

    log::info!("Check messages");

    // test for messages

    let messages = mqtt.close();
    assert_eq!(messages.len(), 1);

    assert_eq!(messages[0].topic, format!("app/{}", app.name()));
    let json: Value = serde_json::from_slice(&messages[0].payload).expect("Parse as JSON");
    log::info!("Message: {:#?}", json);
    assert_eq!(json["specversion"].as_str(), Some("1.0"));
    assert_eq!(json["type"].as_str(), Some("io.drogue.event.v1"));
    assert_eq!(
        json["datacontenttype"].as_str(),
        Some("application/octet-stream")
    );
    assert_eq!(json["subject"].as_str(), Some("telemetry"));
    assert_eq!(json["instance"].as_str(), Some("drogue"));
    assert_eq!(json["application"].as_str(), Some(app.name()));
    assert_eq!(json["device"].as_str(), Some(device.name()));

    Ok(())
}

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
