use super::*;
use crate::{
    common::setup,
    context::TestContext,
    tools::{
        command::send_http_command, messages::WaitForMessages, mqtt::MqttVersion,
        warmup::HttpWarmup,
    },
};
use coap_lite::CoapOption;
use futures::join;
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

#[fixture]
fn ctx() -> TestContext {
    TestContext::new()
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_command(
    mut ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    version: MqttVersion,
) -> anyhow::Result<()> {
    setup();
    let app = Uuid::new_v4().to_string();
    test_single_coap_command(&mut ctx, version, TestData::simple(&app, "device1")).await
}

async fn test_single_coap_command(
    ctx: &mut TestContext,
    version: MqttVersion,
    data: TestData,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let info = ctx.info().await?;

    let channel = data.channel();
    let app = Application::new(drg.clone(), data.app.clone())
        .expect("Create a new application")
        .expect_ready();
    let device = app
        .create_device(data.device.clone(), &data.spec)
        .expect("Failed to create a new device");

    // send telemetry (with command time out)

    log::info!("Sending payload");

    // send the telemetry message

    let TestData { auth, payload, .. } = data;

    // set up MQTT integration receiver

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
    .expect("MQTT receiver started");

    let mqtt = mqtt
        .warmup(
            HttpWarmup::with_params(ctx, &device, &auth, &Default::default()).await?,
            Duration::from_secs(30),
        )
        .await?;

    // start telemetry

    let sender = CoapSender::new(&info);
    let opts = HttpSenderOptions {
        command_timeout: Some(6000),
        ..Default::default()
    };
    let telemetry = sender.send(
        channel,
        auth,
        "application/octet-stream".into(),
        &opts,
        payload,
    );

    let command = async {
        // wait for the MQTT message to arrive
        mqtt.wait_for_messages(1, Duration::from_secs(5)).await?;

        // then send the command
        send_http_command(&info, &drg, &app, &device, "SET").await
    };

    let (telemetry, command) = join!(telemetry, command);

    let telemetry = telemetry.expect("Failed to get telemetry response");
    let command = command.expect("Failed to get command response");

    // we must wait for the MQTT message to arrive … that is the right time to send off the command

    // assert command response

    assert!(command.status().is_success());
    let command = command.text().await;
    assert!(command.is_ok());
    assert_eq!(command.unwrap(), "");

    // assert telemetry response

    assert_eq!(telemetry.get_status().clone(), ResponseType::Content);
    assert_eq!(
        telemetry
            .message
            .get_option(CoapOption::Unknown(4210))
            .map(|v| v.front())
            .flatten(),
        Some(&"SET".as_bytes().to_vec())
    );
    let telemetry = serde_json::from_slice::<serde_json::Value>(&telemetry.message.payload);
    assert!(telemetry.is_ok());
    assert_eq!(telemetry.unwrap(), json!({"set-command": "foo"}));

    // done

    Ok(())
}
