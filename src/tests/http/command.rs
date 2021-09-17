use super::*;
use crate::{
    common::setup,
    context::TestContext,
    init::token::TokenInjector,
    tools::{messages::WaitForMessages, mqtt::MqttVersion},
};
use futures::join;
use reqwest::header::HeaderValue;
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
    test_single_http_command(&mut ctx, version, TestData::simple(&app, "device1")).await
}

async fn test_single_http_command(
    ctx: &mut TestContext,
    version: MqttVersion,
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

    // send telemetry (with command time out)

    log::info!("Sending payload");

    // add the command timeout
    let mut params = data.params.clone();
    params.insert("ct".into(), "5000".into());
    let warmup_params = data.params;

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
            HttpWarmup::with_params(ctx, &device, &auth, warmup_params).await?,
            Duration::from_secs(30),
        )
        .await?;

    // start telemetry

    let sender = HttpSender::new(&info, ctx);
    let telemetry = sender.send(
        channel,
        &auth,
        Some("application/octet-stream".into()),
        &params,
        payload,
    );

    let command = async {
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;

        // wait for the MQTT message to arrive

        mqtt.wait_for_messages(1, Duration::from_secs(5)).await?;

        // now we can send the command back

        let mut command_url = info.command_url;
        command_url.set_path(&format!(
            "/api/command/v1alpha1/apps/{appId}/devices/{deviceId}",
            appId = app.name(),
            deviceId = device.name(),
        ));
        command_url.query_pairs_mut().append_pair("command", "SET");
        let command = client
            .post(command_url)
            .inject_token(drg.clone())
            .await?
            .json(&json!({
                "set-command": "foo",
            }))
            .send()
            .await?;

        Ok::<_, anyhow::Error>(command)
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

    assert!(telemetry.status().is_success());
    assert_eq!(
        telemetry.headers().get("Command"),
        Some(&HeaderValue::from_str("SET")?)
    );
    let telemetry = telemetry.json::<serde_json::Value>().await;
    assert!(telemetry.is_ok());
    assert_eq!(telemetry.unwrap(), json!({"set-command": "foo"}));

    // done

    Ok(())
}
