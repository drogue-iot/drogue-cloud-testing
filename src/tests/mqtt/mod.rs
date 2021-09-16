use crate::{
    context::TestContext,
    init::token::TokenProvider,
    resources::apps::Application,
    tools::{
        assert::{assert_msgs, CloudMessage},
        messages::WaitForMessages,
        mqtt::{MqttDevice, MqttQoS, MqttReceiver, MqttVersion},
        warmup::HttpWarmup,
        Auth,
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
    payload: Option<Vec<u8>>,
    content_type: Option<String>,
    channel: Option<String>,
}

impl TestData {
    pub fn channel(&self) -> String {
        self.channel.clone().unwrap_or_else(|| "telemetry".into())
    }

    pub fn simple(app: &str) -> Self {
        Self {
            app: app.into(),
            device: "device1".into(),
            spec: json!({"credentials": {"credentials": [
                { "pass": "foo" }
            ]}}),
            auth: Auth::UsernamePassword(format!("device1@{}", app), "foo".into()),
            ..Default::default()
        }
    }
}

/// Test a message sent to the MQTT endpoint and received by the MQTT integration
async fn test_single_mqtt_to_mqtt_message(
    ctx: &mut TestContext,
    qos: MqttQoS,
    endpoint_version: MqttVersion,
    integration_version: MqttVersion,
    data: TestData,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let info = ctx.info().await?;

    let channel = data.channel();
    let app = Application::new(drg.clone(), data.app)
        .expect("Create a new application")
        .expect_ready();
    let device = app
        .create_device(data.device, &data.spec)
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
        integration_version,
        format!("app/{}", app.name()),
        MqttQoS::QoS0,
    )
    .await
    .unwrap();

    log::info!("Receiver created");

    let mqtt = mqtt
        .warmup(
            HttpWarmup::new(ctx, &device, &data.auth).await?,
            Duration::from_secs(30),
        )
        .await?;

    // do some work

    log::info!("Sending payload");

    MqttDevice::new(&info, data.auth, endpoint_version, ctx)
        .await?
        .send(
            channel,
            qos,
            "application/octet-stream".into(),
            data.payload,
        )
        .await
        .expect("MQTT publish to succeed");

    log::info!("Payload sent, waiting for messages");

    mqtt.wait_for_messages(1, Duration::from_secs(5))
        .await
        .expect("Message should not time out");

    log::info!("Check messages");

    // test for messages

    let messages = mqtt.close().await;

    assert_msgs(
        &messages,
        &vec![CloudMessage {
            subject: "telemetry".into(),
            r#type: "io.drogue.event.v1".into(),
            instance: "drogue".into(),
            app: app.name().into(),
            device: device.name().into(),
            content_type: Some("application/octet-stream".into()),
            payload: vec![],
        }],
    );

    Ok(())
}
