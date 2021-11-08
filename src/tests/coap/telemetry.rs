use super::*;
use crate::{context::TestContext, tools::Auth};
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
    version: MqttVersion,
) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_coap_to_mqtt_message(
        &mut ctx,
        version,
        Default::default(),
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

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_user(
    mut ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    version: MqttVersion,
) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_coap_to_mqtt_message(
        &mut ctx,
        version,
        HttpSenderOptions {
            device: Some("device1".into()),
            ..Default::default()
        },
        TestData {
            app: app.clone(),
            device: "device1".into(),
            spec: json!({"credentials": {"credentials": [
                { "user": { "username": "foo", "password": "bar" } }
            ]}}),
            auth: Auth::UsernamePassword(format!("foo@{}", app), "bar".into()),
            ..Default::default()
        },
    )
    .await
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_user_only(
    mut ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    version: MqttVersion,
) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_coap_to_mqtt_message(
        &mut ctx,
        version,
        HttpSenderOptions {
            application: Some(app.clone()),
            device: Some("device1".into()),
            ..Default::default()
        },
        TestData {
            app: app.clone(),
            device: "device1".into(),
            spec: json!({"credentials": { "credentials": [
                { "user": {"username": "foo", "password": "bar" } }
            ]}}),
            auth: Auth::UsernamePassword("foo".into(), "bar".into()),
            ..Default::default()
        },
    )
    .await
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_user_alias(
    mut ctx: TestContext,
    #[rustfmt::skip]
    #[values(MqttVersion::V3_1_1, MqttVersion::V5(false), MqttVersion::V5(true))]
    version: MqttVersion,
) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_coap_to_mqtt_message(
        &mut ctx,
        version,
        Default::default(),
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
