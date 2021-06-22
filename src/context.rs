use crate::common::setup;
use crate::init::config::Config;
use crate::init::drg::Drg;
use crate::init::info::Information;
use crate::init::web::WebDriver;
use test_context::AsyncTestContext;

pub struct TestContext {
    web: Option<WebDriver>,
    drg: Option<Drg>,
    info: Option<Information>,
    client: Option<reqwest::Client>,
}

#[async_trait::async_trait]
impl AsyncTestContext for TestContext {
    async fn setup() -> Self {
        setup();

        Self {
            web: None,
            drg: None,
            info: None,
            client: None,
        }
    }

    async fn teardown(self) {
        if let Some(mut web) = self.web {
            match web.close().await {
                Ok(_) => {}
                Err(err) => {
                    log::error!("Failed to close web driver: {}", err);
                }
            }
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
