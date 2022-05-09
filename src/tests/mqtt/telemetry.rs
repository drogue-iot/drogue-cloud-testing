use super::*;
use crate::resources::devices::Device;
use crate::{
    context::TestContext,
    tools::{
        mqtt::{MqttQoS, MqttVersion},
        Auth,
    },
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
async fn test_send_telemetry_pass(
    mut ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    endpoint_version: MqttVersion,
    #[values(false, true)] endpoint_ws: bool,
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    integration_version: MqttVersion,
    #[values(false, true)] integration_ws: bool,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let app_name = Uuid::new_v4().to_string();

    let app = Application::new(drg.clone(), app_name.clone())
        .expect("Create a new application")
        .expect_ready();

    test_single_mqtt_to_mqtt_message(
        &mut ctx,
        MqttQoS::QoS0,
        (endpoint_version, endpoint_ws).into(),
        (integration_version, integration_ws).into(),
        &app,
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
async fn test_send_telemetry_gateway_pass(
    mut ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    endpoint_version: MqttVersion,
    #[values(false, true)] endpoint_ws: bool,
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    integration_version: MqttVersion,
    #[values(false, true)] integration_ws: bool,
) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;
    let app_name = Uuid::new_v4().to_string();

    let app = Application::new(drg.clone(), app_name.clone())
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

    test_single_mqtt_to_mqtt_message(
        &mut ctx,
        MqttQoS::QoS0,
        (endpoint_version, endpoint_ws).into(),
        (integration_version, integration_ws).into(),
        &app,
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
