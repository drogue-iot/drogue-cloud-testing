use crate::{
    init::token::TokenProvider,
    tools::{
        assert::{assert_mqtt_msgs, Message},
        http::{Auth, HttpSender},
        mqtt::{MqttQoS, MqttReceiver, MqttVersion},
    },
    {context::TestContext, resources::apps::Application},
};
use serde_json::{json, Value};
use std::time::Duration;
use test_context::test_context;

#[test_context(TestContext)]
#[tokio::test]
async fn test_send_telemetry_pass(ctx: &mut TestContext) -> anyhow::Result<()> {
    test_single_mqtt_message(
        ctx,
        "device1",
        json!({
            "credentials": {
                "credentials": [
                    { "pass": "foo" }
                ]
            }
        }),
        Auth::Password("foo"),
    )
    .await
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_send_telemetry_user(ctx: &mut TestContext) -> anyhow::Result<()> {
    test_single_mqtt_message(
        ctx,
        "device1",
        json!({
            "credentials": {
                "credentials": [
                    { "user": {"username": "foo", "password": "bar", "unique": true } }
                ]
            }
        }),
        Auth::UsernamePassword("foo", "bar"),
    )
    .await
}

async fn test_single_mqtt_message(
    ctx: &mut TestContext,
    device_name: &str,
    spec: Value,
    auth: Auth<'_>,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let info = ctx.info().await?;

    let app = Application::new_random(drg.clone()).expect("Create a new application");
    let device = app
        .create_device(device_name, &spec)
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

    log::info!("Sending payload");

    let response = HttpSender::new(&info)?
        .send(&device, auth, "telemetry")
        .await
        .expect("HTTP call to succeed");

    assert!(response.status().is_success());

    log::info!("Payload sent, waiting for messages");

    mqtt.wait_for_messages(1, Duration::from_secs(5))
        .await
        .expect("No timeout");

    log::info!("Check messages");

    // test for messages

    let messages = mqtt.close();

    assert_mqtt_msgs(
        &messages,
        &vec![Message {
            subject: "telemetry".into(),
            r#type: "io.drogue.event.v1".into(),
            instance: "drogue".into(),
            app: app.name().into(),
            device: device_name.to_string(),
            content_type: Some("application/octet-stream".into()),
        }],
    );

    Ok(())
}
