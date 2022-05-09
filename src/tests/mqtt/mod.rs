use crate::{
    context::TestContext,
    init::token::TokenProvider,
    resources::apps::Application,
    tools::{
        assert::{assert_msgs, CloudMessage},
        messages::WaitForMessages,
        mqtt::{scrub_uri, MqttVariant},
        mqtt::{MqttDevice, MqttQoS, MqttReceiver},
        warmup::HttpWarmup,
        Auth, SendAs,
    },
};
use serde_json::{json, Value};
use std::time::Duration;

pub mod command;
pub mod telemetry;

#[derive(Clone, Debug, Default)]
pub struct TestData {
    app: String,
    device: String,
    spec: Value,
    auth: Auth,
    send_as: SendAs,
    payload: Option<Vec<u8>>,
    content_type: Option<String>,
    channel: Option<String>,
    expected: ExpectedOutcome,
}

#[derive(Clone, Debug, Default)]
pub struct ExpectedOutcome {
    pub device: Option<String>,
}

impl TestData {
    pub fn channel(&self) -> String {
        self.channel.clone().unwrap_or_else(|| "telemetry".into())
    }
}

/// Test a message sent to the MQTT endpoint and received by the MQTT integration
async fn test_single_mqtt_to_mqtt_message(
    ctx: &mut TestContext,
    qos: MqttQoS,
    endpoint_variant: MqttVariant,
    integration_variant: MqttVariant,
    app: &Application,
    data: TestData,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let info = ctx.info().await?;

    let channel = data.channel();

    let device = app
        .create_device(data.device, &data.spec)
        .expect("Create new device");

    let uri = match integration_variant {
        MqttVariant::Plain(_) => format!(
            "ssl://{}:{}",
            info.mqtt_integration.host, info.mqtt_integration.port
        ),
        MqttVariant::WebSocket(_) => scrub_uri(&info.mqtt_integration_ws.url),
    };

    log::info!("MQTT integration URL: {}", uri);

    let mqtt = MqttReceiver::new(
        uri,
        None,
        Some(drg.current_token().await?),
        integration_variant.version(),
        format!("app/{}", app.name()),
        MqttQoS::QoS0,
    )
    .await
    .unwrap();

    log::info!("Receiver created");

    let mqtt = mqtt
        .warmup(
            HttpWarmup::with_params(ctx, &device, &data.auth, &(&data.send_as).into()).await?,
            Duration::from_secs(30),
        )
        .await?;

    // do some work

    let topic = match &data.send_as {
        SendAs::Gateway { device } => format!("{}/{}", channel, device),
        SendAs::Device => channel.clone(),
    };

    log::info!("Sending payload: {}", topic);

    MqttDevice::new(&info, data.auth, endpoint_variant, ctx)
        .await?
        .send(
            topic,
            qos,
            "application/octet-stream".into(),
            data.payload.clone(),
        )
        .await
        .expect("MQTT publish to succeed");

    log::info!("Payload sent, waiting for messages");

    // wait for 3 messages, 2 connection events, one data message
    mqtt.wait_for_messages(3, Duration::from_secs(5))
        .await
        .expect("Message should not time out");

    log::info!("Check messages");

    // test for messages

    let messages = mqtt.close().await;

    let mf = MessageFactory::new(app, device.name());

    assert_msgs(
        &messages,
        &vec![
            mf.connection(true),
            mf.data(&channel, data.expected.device.as_deref(), data.payload),
            mf.connection(false),
        ],
    );

    Ok(())
}

pub struct MessageFactory<'t> {
    app: &'t Application,
    connecting_device: &'t str,
}

impl<'t> MessageFactory<'t> {
    pub fn new(app: &'t Application, connecting_device: &'t str) -> Self {
        Self {
            app,
            connecting_device,
        }
    }

    pub fn connection(&self, state: bool) -> CloudMessage {
        connection_message(self.app, self.connecting_device, state)
    }

    pub fn data(
        &self,
        channel: &str,
        device: Option<&str>,
        payload: Option<Vec<u8>>,
    ) -> CloudMessage {
        data_message(channel, self.app, self.connecting_device, device, payload)
    }
}

fn connection_message(app: &Application, device: &str, state: bool) -> CloudMessage {
    message(
        "connection",
        "io.drogue.connection.v1",
        app,
        device,
        None,
        Some("application/json"),
        Some(serde_json::to_vec(&json!({ "connected": state })).unwrap()),
    )
}

fn data_message(
    channel: &str,
    app: &Application,
    sender: &str,
    device: Option<&str>,
    payload: Option<Vec<u8>>,
) -> CloudMessage {
    message(
        channel,
        "io.drogue.event.v1",
        app,
        sender,
        device,
        Some("application/octet-stream"),
        payload,
    )
}

fn message(
    channel: &str,
    r#type: &str,
    app: &Application,
    sender: &str,
    device: Option<&str>,
    content_type: Option<&str>,
    payload: Option<Vec<u8>>,
) -> CloudMessage {
    CloudMessage {
        subject: channel.into(),
        r#type: r#type.into(),
        instance: "drogue".into(),
        app: app.name().into(),
        device: device.unwrap_or_else(|| sender).into(),
        sender: sender.into(),
        content_type: content_type.map(|s| s.into()),
        payload: payload.unwrap_or_default(),
    }
}
