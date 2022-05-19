use crate::{
    context::TestContext,
    init::token::TokenProvider,
    resources::apps::Application,
    tools::{
        assert::{assert_msgs, CloudMessage},
        http::{HttpSender, HttpSenderOptions},
        messages::WaitForMessages,
        mqtt::WarmupReceiver,
        warmup::HttpWarmup,
        websocket::WebSocketReceiver,
        Auth,
    },
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

pub mod telemetry;

#[derive(Clone, Debug, Default)]
pub struct TestData {
    app: String,
    device: String,
    spec: Value,
    auth: Auth,
    payload: Option<Vec<u8>>,
    content_type: Option<String>,
    channel: Option<String>,
    params: HashMap<String, String>,
}

impl TestData {
    pub fn channel(&self) -> String {
        self.channel.clone().unwrap_or_else(|| "telemetry".into())
    }

    pub fn simple(app: &str) -> Self {
        Self {
            app: app.into(),
            device: "device1".into(),
            spec: json!({"credentials": {"credentials": [
                { "pass": "foo" }
            ]}}),
            auth: Auth::UsernamePassword(format!("device1@{}", app), "foo".into()),
            ..Default::default()
        }
    }
}

/// Test a message sent to the HTTP endpoint and received by the Websocket integration
async fn test_single_http_to_websocket_message(
    ctx: &mut TestContext,
    data: TestData,
    options: HttpSenderOptions,
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

    let uri = info.websocket_integration.url.clone();

    log::info!("WebSocket integration URL: {}", uri);

    let websocket = WebSocketReceiver::new(uri, drg.current_token().await?, app.name())
        .await
        .unwrap();

    log::info!("Receiver created");

    let websocket = websocket
        .warmup(
            HttpWarmup::with_params(ctx, &device, &data.auth, &options).await?,
            Duration::from_secs(30),
        )
        .await?;

    // do some work

    log::info!("Sending payload");

    log::info!("Sending payload");

    let response = HttpSender::new(&info, ctx)
        .send(
            channel,
            &data.auth,
            Some("application/octet-stream".into()),
            &options,
            data.payload,
        )
        .await
        .expect("HTTP call to succeed");

    assert!(response.status().is_success());

    log::info!("Payload sent, waiting for messages");

    websocket
        .wait_for_messages(1, Duration::from_secs(5))
        .await
        .expect("Message should not time out");

    log::info!("Check messages");

    // test for messages

    let messages = websocket.drain_messages().await;

    assert_msgs(
        &messages,
        &vec![CloudMessage {
            subject: "telemetry".into(),
            r#type: "io.drogue.event.v1".into(),
            instance: "drogue".into(),
            app: app.name().into(),
            device: device.name().into(),
            sender: device.name().into(),
            content_type: Some("application/octet-stream".into()),
            payload: vec![],
        }],
    );

    Ok(())
}
