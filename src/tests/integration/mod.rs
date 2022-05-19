use crate::context::TestContext;
use crate::init::drg::Drg;
use crate::init::info::Information;
use crate::init::token::TokenProvider;
use crate::resources::apps::Application;
use crate::tools::assert::{assert_msgs, CloudMessage};
use crate::tools::http::HttpSender;
use crate::tools::messages::WaitForMessages;
use crate::tools::mqtt::rumqtt::MqttReceiver;
use crate::tools::mqtt::{MqttQoS, WarmupReceiver};
use crate::tools::warmup::HttpWarmup;
use crate::tools::Auth;
use rstest::{fixture, rstest};
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

#[fixture]
fn ctx() -> TestContext {
    TestContext::new()
}

#[derive(Clone, Debug, Default)]
pub struct TestData {
    app: String,
    device: String,
    spec: Value,
    auth: Auth,
    payload: Option<Vec<u8>>,
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_qos_1_dropping(mut ctx: TestContext) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_http_to_mqtt_message(
        &mut ctx,
        TestData {
            app: app.clone(),
            device: "device1".into(),
            spec: json!({"credentials": { "credentials": [
                { "user": {"username": "foo", "password": "bar", "unique": true } }
            ]}}),
            auth: Auth::UsernamePassword(format!("foo@{}", app), "bar".into()),
            payload: Some("FooBar".into()),
        },
    )
    .await
}

/// Test a message sent to the HTTP endpoint and received by the MQTT integration
async fn test_single_http_to_mqtt_message(
    ctx: &mut TestContext,
    data: TestData,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let info = ctx.info().await?;

    let channel = "telemetry";
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

    let expected_messages = vec![CloudMessage {
        subject: "telemetry".into(),
        r#type: "io.drogue.event.v1".into(),
        instance: "drogue".into(),
        app: app.name().into(),
        device: device.name().into(),
        sender: device.name().into(),
        content_type: Some("application/octet-stream".into()),
        payload: data.payload.clone().unwrap_or_default(),
    }];

    log::info!("MQTT integration URL: {}", uri);

    let mqtt = create_receiver(&drg, &app, &info).await?;

    // the first one we warm up
    let mqtt = mqtt
        .warmup(
            HttpWarmup::with_params(ctx, &device, &data.auth, &Default::default()).await?,
            Duration::from_secs(30),
        )
        .await?;

    // stop ack'ing after warmup phase
    mqtt.set_ack(false);

    // do some work

    log::info!("Sending payload");

    let response = HttpSender::new(&info, ctx)
        .send(
            channel.into(),
            &data.auth,
            Some("application/octet-stream".into()),
            &Default::default(),
            data.payload.clone(),
        )
        .await
        .expect("HTTP call to succeed");

    assert!(response.status().is_success());

    mqtt.wait_for_messages(1, Duration::from_secs(5))
        .await
        .expect("No timeout");

    // test for messages

    let messages = mqtt.close().await;
    assert_msgs(&messages, &expected_messages);

    // messages are not committed yet, so consume again

    let mqtt = create_receiver(&drg, &app, &info).await?;
    mqtt.wait_for_messages(1, Duration::from_secs(5))
        .await
        .expect("No timeout");

    // and expect the same messages, once more

    let messages = mqtt.close().await;
    assert_msgs(&messages, &expected_messages);

    // let's do it again, this time, don't expect messages

    let mqtt = create_receiver(&drg, &app, &info).await?;
    let r = mqtt.wait_for_messages(1, Duration::from_secs(5)).await;
    assert!(r.is_err());
    let messages = mqtt.close().await;
    assert_msgs(&messages, &vec![]);

    // done

    Ok(())
}

async fn create_receiver(
    drg: &Drg,
    app: &Application,
    info: &Information,
) -> anyhow::Result<MqttReceiver> {
    let receiver = MqttReceiver::new(
        info.mqtt_integration.host.clone(),
        info.mqtt_integration.port,
        None,
        Some(drg.current_token().await?),
        format!("$shared/client1/app/{}", app.name()),
        MqttQoS::QoS1,
    )
    .await?;

    log::info!("Receiver created");

    Ok(receiver)
}
