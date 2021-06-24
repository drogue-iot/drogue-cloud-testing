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
use rstest::{fixture, rstest};
use rstest_reuse::{self, *};
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;

#[fixture]
fn ctx() -> TestContext {
    TestContext::new()
}

#[template]
#[rstest(
    version,
    case::mqtt3(MqttVersion::V3_1_1),
    case::mqtt5_structured(MqttVersion::V5(false)),
    case::mqtt5_binary(MqttVersion::V5(true))
)]
fn mqtt_versions(version: MqttVersion) {}

#[apply(mqtt_versions)]
#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_pass(
    mut ctx: TestContext,
    version: MqttVersion,
) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_mqtt_message(
        &mut ctx,
        version,
        TestData {
            app: app.clone(),
            device: "device1".into(),
            spec: json!({"credentials": {"credentials": [
                { "pass": "foo" }
            ]}}),
            auth: Auth::UsernamePassword(format!("device1@{}", app), "foo".into()),
            params: Default::default(),
            ..Default::default()
        },
    )
    .await
}

#[apply(mqtt_versions)]
#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_user(
    mut ctx: TestContext,
    version: MqttVersion,
) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_mqtt_message(
        &mut ctx,
        version,
        TestData {
            app: app.clone(),
            device: "device1".into(),
            spec: json!({"credentials": {"credentials": [
                { "user": { "username": "foo", "password": "bar" } }
            ]}}),
            auth: Auth::UsernamePassword(format!("foo@{}", app), "bar".into()),
            params: convert_args!(hashmap! (
                "device" => "device1",
            )),
            ..Default::default()
        },
    )
    .await
}

#[apply(mqtt_versions)]
#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_user_only(
    mut ctx: TestContext,
    version: MqttVersion,
) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_mqtt_message(
        &mut ctx,
        version,
        TestData {
            app: app.clone(),
            device: "device1".into(),
            spec: json!({"credentials": { "credentials": [
                { "user": {"username": "foo", "password": "bar" } }
            ]}}),
            auth: Auth::UsernamePassword("foo".into(), "bar".into()),
            params: convert_args!(hashmap! (
                "application" => app,
                "device" => "device1",
            )),
            ..Default::default()
        },
    )
    .await
}

#[apply(mqtt_versions)]
#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_user_alias(
    mut ctx: TestContext,
    version: MqttVersion,
) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_mqtt_message(
        &mut ctx,
        version,
        TestData {
            app: app.clone(),
            device: "device1".into(),
            spec: json!({"credentials": { "credentials": [
                { "user": {"username": "foo", "password": "bar", "unique": true } }
            ]}}),
            auth: Auth::UsernamePassword(format!("foo@{}", app), "bar".into()),
            ..Default::default()
        },
    )
    .await
}

#[derive(Clone, Debug, Default)]
pub struct TestData {
    app: String,
    device: String,
    spec: Value,
    auth: Auth,
    params: HashMap<String, String>,
    payload: Option<Vec<u8>>,
    content_type: Option<String>,
    channel: Option<String>,
}

impl TestData {
    pub fn channel(&self) -> String {
        self.channel
            .as_ref()
            .map_or_else(|| "telemetry".into(), |s| s.into())
    }
}

async fn test_single_mqtt_message(
    ctx: &mut TestContext,
    version: MqttVersion,
    data: TestData,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let info = ctx.info().await?;

    let channel = data.channel();
    let app = Application::new(drg.clone(), data.app).expect("Create a new application");
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

    // FIXME: instead of just sleeping, we should try to warm up the channel with different events

    tokio::time::sleep(Duration::from_secs(5)).await;

    // do some work

    log::info!("Sending payload");

    let response = HttpSender::new(&info)?
        .send(
            channel,
            data.auth,
            "application/octet-stream".into(),
            data.params,
            data.payload,
        )
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
            device: device.name().into(),
            content_type: Some("application/octet-stream".into()),
            payload: vec![],
        }],
    );

    Ok(())
}
