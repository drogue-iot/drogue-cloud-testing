use crate::{
    init::token::TokenProvider,
    tools::{
        assert::{assert_mqtt_msgs, Message},
        http::{Auth, HttpSender},
        mqtt::{MqttQoS, MqttReceiver, MqttVersion},
    },
    {context::TestContext, resources::apps::Application},
};
use maplit::{convert_args, hashmap};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use test_context::test_context;
use uuid::Uuid;

#[test_context(TestContext)]
#[tokio::test]
async fn test_send_telemetry_pass(ctx: &mut TestContext) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_mqtt_message(
        ctx,
        app.clone(),
        "device1",
        json!({
            "credentials": {
                "credentials": [
                    { "pass": "foo" }
                ]
            }
        }),
        Auth::UsernamePassword(format!("device1@{}", app), "foo".into()),
        Default::default(),
    )
    .await
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_send_telemetry_user(ctx: &mut TestContext) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_mqtt_message(
        ctx,
        app.clone(),
        "device1",
        json!({
            "credentials": {
                "credentials": [
                    { "user": {"username": "foo", "password": "bar" } }
                ]
            }
        }),
        Auth::UsernamePassword(format!("foo@{}", app), "bar".into()),
        convert_args!(hashmap! (
            "device" => "device1",
        )),
    )
    .await
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_send_telemetry_user_only(ctx: &mut TestContext) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_mqtt_message(
        ctx,
        app.clone(),
        "device1",
        json!({
            "credentials": {
                "credentials": [
                    { "user": {"username": "foo", "password": "bar" } }
                ]
            }
        }),
        Auth::UsernamePassword("foo".into(), "bar".into()),
        convert_args!(hashmap! (
            "application" => app,
            "device" => "device1",
        )),
    )
    .await
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_send_telemetry_user_alias(ctx: &mut TestContext) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_mqtt_message(
        ctx,
        app.clone(),
        "device1",
        json!({
            "credentials": {
                "credentials": [
                    { "user": {"username": "foo", "password": "bar", "unique": true } }
                ]
            }
        }),
        Auth::UsernamePassword(format!("foo@{}", app), "bar".into()),
        Default::default(),
    )
    .await
}

async fn test_single_mqtt_message<S>(
    ctx: &mut TestContext,
    app_name: S,
    device_name: &str,
    spec: Value,
    auth: Auth,
    params: HashMap<String, String>,
) -> anyhow::Result<()>
where
    S: Into<String>,
{
    let drg = ctx.drg().await?;
    let info = ctx.info().await?;

    let app = Application::new(drg.clone(), app_name).expect("Create a new application");
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
        .send(&device, auth, "telemetry", params)
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
