use crate::common::setup;
use crate::init::config::Config;
use crate::init::drg::Drg;
use crate::init::info::Information;
use crate::init::web::WebDriver;
use test_context::AsyncTestContext;
use tokio::runtime::Handle;

pub struct TestContext {
    web: Option<WebDriver>,
    drg: Option<Drg>,
    info: Option<Information>,
    client: Option<reqwest::Client>,
}

impl Default for TestContext {
    fn default() -> Self {
        setup();

        Self {
            web: None,
            drg: None,
            info: None,
            client: None,
        }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // this can be improved once https://github.com/la10736/rstest/issues/94 is resolved
        if let Some(mut web) = self.web.take() {
            tokio::task::block_in_place(move || {
                Handle::current()
                    .block_on(async move { web.close().await })
                    .ok()
            });
        }
    }
}

impl TestContext {
    pub fn new() -> TestContext {
        TestContext::default()
    }
}

#[async_trait::async_trait]
impl AsyncTestContext for TestContext {
    async fn setup() -> Self {
        TestContext::default()
    }

    async fn teardown(mut self) {
        if let Some(mut web) = self.web.take() {
            web.close().await.ok();
        }
    }
}

impl TestContext {
    pub async fn web(&mut self) -> anyhow::Result<WebDriver> {
        if let Some(web) = &self.web {
            Ok(web.clone())
        } else {
            let web = WebDriver::new().await?;
            self.web = Some(web.clone());
            Ok(web)
        }
    }

    pub async fn drg(&mut self) -> anyhow::Result<Drg> {
        if let Some(drg) = &self.drg {
            Ok(drg.clone())
        } else {
            let drg = Drg::auto_login(&mut self.web().await?).await?;
            self.drg = Some(drg.clone());
            Ok(drg)
        }
    }

    pub async fn client(&mut self) -> anyhow::Result<reqwest::Client> {
        if let Some(client) = &self.client {
            Ok(client.clone())
        } else {
            let client = reqwest::Client::new();
            self.client = Some(client.clone());
            Ok(client)
        }
    }

    pub async fn info(&mut self) -> anyhow::Result<Information> {
        if let Some(info) = &self.info {
            Ok(info.clone())
        } else {
            let info =
                Information::new(self.client().await?, Config::new()?, self.drg().await?).await?;
            self.info = Some(info.clone());
            Ok(info)
        }
    }
}
