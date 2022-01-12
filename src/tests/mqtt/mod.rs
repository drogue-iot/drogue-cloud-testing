use crate::tools::mqtt::{scrub_uri, MqttVariant};
use crate::tools::SendAs;
use crate::{
    context::TestContext,
    init::token::TokenProvider,
    resources::apps::Application,
    tools::{
        assert::{assert_msgs, CloudMessage},
        messages::WaitForMessages,
        mqtt::{MqttDevice, MqttQoS, MqttReceiver},
        warmup::HttpWarmup,
        Auth,
    },
};
use serde_json::Value;
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
    pub sender: Option<String>,
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

    mqtt.wait_for_messages(1, Duration::from_secs(5))
        .await
        .expect("Message should not time out");

    log::info!("Check messages");

    // test for messages

    let messages = mqtt.close().await;

    assert_msgs(
        &messages,
        &vec![CloudMessage {
            subject: channel,
            r#type: "io.drogue.event.v1".into(),
            instance: "drogue".into(),
            app: app.name().into(),
            device: device.name().into(),
            sender: data.expected.sender.unwrap_or_else(|| device.name().into()),
            content_type: Some("application/octet-stream".into()),
            payload: data.payload.unwrap_or_default(),
        }],
    );

    Ok(())
}
