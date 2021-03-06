use super::*;
use crate::{context::TestContext, tools::Auth};
use maplit::{convert_args, hashmap};
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

#[fixture]
fn ctx() -> TestContext {
    TestContext::new()
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_pass(mut ctx: TestContext) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_http_to_websocket_message(
        &mut ctx,
        TestData::simple(&app),
        HttpSenderOptions {
            device: Some("device1".into()),
            ..Default::default()
        },
    )
    .await
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_user(mut ctx: TestContext) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_http_to_websocket_message(
        &mut ctx,
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
        HttpSenderOptions {
            device: Some("device1".into()),
            ..Default::default()
        },
    )
    .await
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_user_only(mut ctx: TestContext) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_http_to_websocket_message(
        &mut ctx,
        TestData {
            app: app.clone(),
            device: "device1".into(),
            spec: json!({"credentials": { "credentials": [
                { "user": {"username": "foo", "password": "bar" } }
            ]}}),
            auth: Auth::UsernamePassword("foo".into(), "bar".into()),
            params: convert_args!(hashmap! (
                "application" => app.clone(),
                "device" => "device1",
            )),
            ..Default::default()
        },
        HttpSenderOptions {
            application: Some(app.clone()),
            device: Some("device1".into()),
            ..Default::default()
        },
    )
    .await
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_send_telemetry_user_alias(mut ctx: TestContext) -> anyhow::Result<()> {
    let app = Uuid::new_v4().to_string();
    test_single_http_to_websocket_message(
        &mut ctx,
        TestData {
            app: app.clone(),
            device: "device1".into(),
            spec: json!({"credentials": { "credentials": [
                { "user": {"username": "foo", "password": "bar", "unique": true } }
            ]}}),
            auth: Auth::UsernamePassword(format!("foo@{}", app), "bar".into()),
            ..Default::default()
        },
        HttpSenderOptions {
            device: Some("device1".into()),
            ..Default::default()
        },
    )
    .await
}
