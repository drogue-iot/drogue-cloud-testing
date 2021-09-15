use super::*;
use crate::{
    context::TestContext,
    init::token::TokenInjector,
    tools::{messages::WaitForMessages, mqtt::MqttVersion},
};
use anyhow::Context;
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
    let app = Uuid::new_v4().to_string();
    test_single_mqtt_command(&mut ctx, version, TestData::simple(&app)).await
}

async fn test_single_mqtt_command(
    ctx: &mut TestContext,
    version: MqttVersion,
    data: TestData,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let info = ctx.info().await?;

    let app = Application::new(drg.clone(), data.app)
        .expect("Create a new application")
        .expect_ready();
    let device = app
        .create_device(data.device, &data.spec)
        .expect("Create new device");

    // send telemetry (with command time out)

    log::info!("Sending payload");

    // send the telemetry message

    let mut mqtt = MqttSender::new(&info, data.auth, version, ctx).await?;
    mqtt.subscribe_commands()
        .await
        .expect("MQTT publish to succeed");

    let client = ctx.client().await.context("Get HTTP client")?;

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

    // we sent the command, now fetch the msg from the buffer

    mqtt.wait_for_messages(1, Duration::from_secs(5)).await?;

    // assert command response

    assert!(command.status().is_success());
    let command = command.text().await;
    assert!(command.is_ok());
    assert_eq!(command.unwrap(), "");

    // assert the command

    let commands = mqtt.fetch_messages().expect("Get received commands");

    log::debug!("Received commands: {:#?}", commands);

    assert_eq!(commands.len(), 1, "Expect a single topic");
    let commands = commands.get("command/inbox/SET").expect("Command missing");
    assert_eq!(commands.len(), 1, "Expect a single command on that topic");
    let command = &commands[0];
    log::info!("Command: {:#?}", command);
    assert_eq!(command.topic, "command/inbox/SET");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(command.payload.as_slice())?,
        json!({"set-command": "foo"})
    );

    // done

    Ok(())
}
