use crate::tools::http::HttpSenderOptions;
use crate::{
    context::TestContext,
    resources::devices::Device,
    tools::{
        assert::{CloudMessage, DeviceMessage},
        http::{ClientBuilderProvider, HttpSender},
        Auth,
    },
};
use async_trait::async_trait;
use serde_json::{json, Value};

#[async_trait]
pub trait WarmupSender {
    // Send a message to warm up the receiver
    async fn send(&mut self, index: u32) -> anyhow::Result<()>;
    // Identify a warmup message
    fn identify(&self, message: &CloudMessage, index: u32) -> bool;
}

#[async_trait]
pub trait DefaultWarmupSender {
    async fn send(&mut self, message: DeviceMessage) -> anyhow::Result<()>;

    fn application(&self) -> &str;
    fn device(&self) -> &str;
}

#[async_trait]
impl<W> WarmupSender for W
where
    W: DefaultWarmupSender + Send,
{
    async fn send(&mut self, index: u32) -> anyhow::Result<()> {
        let msg = DeviceMessage {
            subject: "warmup".into(),
            app: self.application().into(),
            device: self.device().into(),
            content_type: Some("application/json".into()),
            payload: Some(serde_json::to_vec(&json!({ "index": index }))?),
        };
        self.send(msg).await
    }

    fn identify(&self, message: &CloudMessage, expected_index: u32) -> bool {
        if message.app != self.application() {
            log::debug!(
                "Application does not match (expected: {}, actual: {})",
                self.application(),
                message.app
            );
            return false;
        }

        if message.device != self.device() {
            log::debug!(
                "Device does not match (expected: {}, actual: {})",
                self.device(),
                message.device
            );
            return false;
        }

        if message.subject != "warmup" {
            log::debug!("Subject is not 'warmup'");
            return false;
        }

        let index = serde_json::from_slice(&message.payload)
            .ok()
            .and_then(|json: Value| json["index"].as_i64());

        match index {
            Some(index) => index == expected_index as i64,
            None => {
                log::info!(
                    "Invalid warmup payload: '{}' ({:?})",
                    String::from_utf8_lossy(&message.payload),
                    &message.payload
                );
                false
            }
        }
    }
}

pub struct HttpWarmup<'w, CB>
where
    CB: ClientBuilderProvider,
{
    sender: HttpSender<'w, CB>,
    device: &'w Device<'w>,
    auth: &'w Auth,
    options: &'w HttpSenderOptions,
}

impl<'w, CB> HttpWarmup<'w, CB>
where
    CB: ClientBuilderProvider,
{
    pub fn with_sender(
        sender: HttpSender<'w, CB>,
        device: &'w Device<'w>,
        auth: &'w Auth,
        options: &'w HttpSenderOptions,
    ) -> Self {
        HttpWarmup {
            device,
            auth,
            options,
            sender,
        }
    }
}

impl<'w> HttpWarmup<'w, TestContext> {
    pub async fn with_params(
        ctx: &'w mut TestContext,
        device: &'w Device<'w>,
        auth: &'w Auth,
        options: &'w HttpSenderOptions,
    ) -> anyhow::Result<HttpWarmup<'w, TestContext>> {
        Ok(Self::with_sender(
            HttpSender::new(&ctx.info().await?, ctx),
            device,
            auth,
            options,
        ))
    }
}

#[async_trait]
impl<'w, CB> DefaultWarmupSender for HttpWarmup<'w, CB>
where
    CB: ClientBuilderProvider + Sync,
{
    async fn send(&mut self, message: DeviceMessage) -> anyhow::Result<()> {
        self.sender
            .send(
                message.subject,
                self.auth,
                message.content_type,
                self.options,
                message.payload,
            )
            .await?;

        Ok(())
    }

    fn application(&self) -> &str {
        self.device.app().name()
    }

    fn device(&self) -> &str {
        self.device.name()
    }
}
