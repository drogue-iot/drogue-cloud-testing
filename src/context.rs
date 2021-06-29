use crate::{
    common::setup,
    init::{config::Config, drg::Drg, info::Information, web::WebDriver},
    tools::{http::ClientBuilderProvider, tls},
};
use reqwest::ClientBuilder;
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

impl ClientBuilderProvider for TestContext {
    fn new_client_builder(&self) -> anyhow::Result<ClientBuilder> {
        Self::client_builder()
    }
}

impl TestContext {
    pub fn new() -> TestContext {
        TestContext::default()
    }
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
            let client = Self::client_builder()?.build()?;
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

    /// Create the client builder
    pub fn client_builder() -> anyhow::Result<reqwest::ClientBuilder> {
        // create basic builder

        let mut builder = reqwest::ClientBuilder::new();

        // add CA

        for cert in tls::load_default_ca_certs()? {
            log::info!("Adding root certificate");
            builder = builder.add_root_certificate(cert);
        }

        // done

        Ok(builder)
    }
}
