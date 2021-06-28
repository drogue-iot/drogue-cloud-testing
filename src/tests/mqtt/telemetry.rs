use super::*;
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
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    integration_version: MqttVersion,
) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_mqtt_to_mqtt_message(
        &mut ctx,
        MqttQoS::QoS0,
        endpoint_version,
        integration_version,
        TestData {
            app: app.clone(),
            device: "device1".into(),
            spec: json!({"credentials": {"credentials": [
                { "pass": "foo" }
            ]}}),
            auth: Auth::UsernamePassword(format!("device1@{}", app), "foo".into()),
            ..Default::default()
        },
    )
    .await
}
