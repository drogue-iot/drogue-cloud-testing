use super::*;
use crate::{
    context::TestContext,
    resources::devices::Device,
    tools::{command::send_http_command, messages::WaitForMessages, mqtt::MqttVersion},
};
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

#[fixture]
fn ctx() -> TestContext {
    TestContext::new()
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simple(
    ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    version: MqttVersion,
    #[values(true, false)] ws: bool,
) -> anyhow::Result<()> {
    perform_simple(
        ctx,
        (version, ws).into(),
        CommandOptions {
            command: "SET",
            filter: "command/inbox/#",
            expected_topic: "command/inbox//SET",
        },
    )
    .await
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simple_direct(
    ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    version: MqttVersion,
    #[values(true, false)] ws: bool,
) -> anyhow::Result<()> {
    perform_simple(
        ctx,
        (version, ws).into(),
        CommandOptions {
            command: "SET",
            filter: "command/inbox/device1/#",
            expected_topic: "command/inbox/device1/SET",
        },
    )
    .await
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn simple_me(
    ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    version: MqttVersion,
    #[values(true, false)] ws: bool,
) -> anyhow::Result<()> {
    perform_simple(
        ctx,
        (version, ws).into(),
        CommandOptions {
            command: "SET",
            filter: "command/inbox//#",
            expected_topic: "command/inbox//SET",
        },
    )
    .await
}

async fn perform_simple(
    mut ctx: TestContext,
    variant: MqttVariant,
    options: CommandOptions<'_>,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;

    let app_name = Uuid::new_v4().to_string();

    let app = Application::new(drg, app_name.clone())
        .expect("Create a new application")
        .expect_ready();

    test_single_mqtt_command(
        &mut ctx,
        &app,
        variant,
        options,
        TestData {
            app: app_name.clone(),
            device: "device1".into(),
            spec: json!({"credentials": {"credentials": [
                { "pass": "foo" }
            ]}}),
            auth: Auth::UsernamePassword(format!("device1@{}", app_name), "foo".into()),
            ..Default::default()
        },
    )
    .await
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gateway(
    ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    version: MqttVersion,
    #[values(true, false)] ws: bool,
) -> anyhow::Result<()> {
    perform_gateway(
        ctx,
        (version, ws).into(),
        CommandOptions {
            command: "SET",
            filter: "command/inbox/#",
            expected_topic: "command/inbox/device1/SET",
        },
    )
    .await
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gateway_direct(
    ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    version: MqttVersion,
    #[values(true, false)] ws: bool,
) -> anyhow::Result<()> {
    perform_gateway(
        ctx,
        (version, ws).into(),
        CommandOptions {
            command: "SET",
            filter: "command/inbox/device1/#",
            expected_topic: "command/inbox/device1/SET",
        },
    )
    .await
}

async fn perform_gateway(
    mut ctx: TestContext,
    variant: MqttVariant,
    options: CommandOptions<'_>,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;

    let app_name = Uuid::new_v4().to_string();

    let app = Application::new(drg, app_name.clone())
        .expect("Create a new application")
        .expect_ready();

    let _gateway = Device::new(
        &app,
        "gateway1",
        &json!({"credentials": {"credentials": [
            { "pass": "foo" }
        ]}}),
    )
    .expect("Gateway to be created");

    test_single_mqtt_command(
        &mut ctx,
        &app,
        variant,
        options,
        TestData {
            app: app_name.clone(),
            device: "device1".into(),
            spec: json!({ "gatewaySelector": {"matchNames": ["gateway1"]} }),
            auth: Auth::UsernamePassword(format!("gateway1@{}", app_name), "foo".into()),
            send_as: SendAs::Gateway {
                device: "device1".into(),
            },
            expected: ExpectedOutcome {
                device: Some("device1".into()),
            },
            ..Default::default()
        },
    )
    .await
}

struct CommandOptions<'o> {
    pub command: &'o str,
    pub filter: &'o str,
    pub expected_topic: &'o str,
}

async fn test_single_mqtt_command(
    ctx: &mut TestContext,
    app: &Application,
    variant: MqttVariant,
    options: CommandOptions<'_>,
    data: TestData,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let info = ctx.info().await?;

    let device = app
        .create_device(data.device, &data.spec)
        .expect("Create new device");

    log::info!("Sending payload");

    // subscribe to commands, we don't need to send telemetry here

    let mut mqtt = MqttDevice::new(&info, data.auth, variant, ctx).await?;
    mqtt.subscribe_commands(options.filter)
        .await
        .expect("MQTT subscribe to commands to succeed");

    // send the http command

    let command = send_http_command(&info, &drg, app, &device, options.command).await?;

    // assert command response

    assert!(command.status().is_success());
    let command = command.text().await;
    assert!(command.is_ok());
    // command call should not return payload
    assert_eq!(command.unwrap(), "");

    // we sent the command, now fetch the msg from the buffer

    mqtt.wait_for_messages(1, Duration::from_secs(5)).await?;

    // assert the command

    let commands = mqtt.fetch_messages().expect("Get received commands");

    log::debug!("Received commands: {:#?}", commands);

    assert_eq!(commands.len(), 1, "Expect a single topic");
    let commands = commands
        .get(options.expected_topic)
        .unwrap_or_else(|| panic!("Command missing. Have: {:?}", commands));
    assert_eq!(commands.len(), 1, "Expect a single command on that topic");
    let command = &commands[0];
    log::info!("Command: {:#?}", command);
    assert_eq!(command.topic, options.expected_topic);
    let payload = command.payload.as_slice();
    log::debug!("Payload: {:?}", String::from_utf8_lossy(payload));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(payload)?,
        json!({"set-command": "foo"})
    );

    // done

    Ok(())
}
