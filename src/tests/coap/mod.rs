use crate::{
    context::TestContext,
    init::token::TokenProvider,
    resources::apps::Application,
    tools::{
        assert::{assert_msgs, CloudMessage},
        coap::CoapSender,
        http::HttpSenderOptions,
        messages::WaitForMessages,
        mqtt::{paho::MqttReceiver, MqttQoS, MqttVersion, WarmupReceiver},
        warmup::HttpWarmup,
        Auth,
    },
};
use coap_lite::ResponseType;
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

    pub fn simple(app: &str, device: &str) -> TestData {
        let password = "foo";
        TestData {
            app: app.into(),
            device: device.into(),
            spec: json!({"credentials": {"credentials": [
                { "pass": password }
            ]}}),
            auth: Auth::UsernamePassword(format!("{}@{}", device, app), password.into()),
            ..Default::default()
        }
    }
}

/// Test a message sent to the HTTP endpoint and received by the MQTT integration
async fn test_single_coap_to_mqtt_message(
    ctx: &mut TestContext,
    version: MqttVersion,
    opts: HttpSenderOptions,
    data: TestData,
) -> anyhow::Result<()> {
    log::info!("entered coap_to_mqtt!");

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
        version,
        format!("app/{}", app.name()),
        MqttQoS::QoS0,
    )
    .await
    .unwrap();

    log::info!("Receiver created");

    let mqtt = mqtt
        .warmup(
            HttpWarmup::with_params(ctx, &device, &data.auth, &opts).await?,
            Duration::from_secs(30),
        )
        .await?;

    // do some work

    log::info!("Sending payload");

    let response = CoapSender::new(&info)
        .send(
            channel,
            data.auth,
            "application/octet-stream".into(),
            &opts,
            data.payload,
        )
        .await
        .expect("CoAP call to succeed");

    assert_eq!(response.get_status().clone(), ResponseType::Changed);

    log::info!("Payload sent, waiting for messages");

    mqtt.wait_for_messages(1, Duration::from_secs(5))
        .await
        .expect("No timeout");

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
            sender: device.name().into(),
            content_type: Some("application/octet-stream".into()),
            payload: vec![],
        }],
    );

    Ok(())
}
