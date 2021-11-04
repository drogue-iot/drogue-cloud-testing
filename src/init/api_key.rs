use crate::context::TestContext;
use crate::init::{config::Config, login::login, web::WebDriver};
use crate::tools::web::with_screenshot;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use fantoccini::Locator;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ApiKey {
    pub username: String,
    pub key: String,
}

impl ApiKey {
    pub fn into_provider(self) -> drogue_client::openid::ApiKeyProvider {
        drogue_client::openid::ApiKeyProvider {
            user: self.username,
            key: self.key,
        }
    }
}

impl From<ApiKey> for drogue_client::openid::ApiKeyProvider {
    fn from(key: ApiKey) -> Self {
        key.into_provider()
    }
}

#[async_trait]
pub trait ApiKeyCreator {
    async fn create_api_key_web(&mut self) -> anyhow::Result<ApiKey>;
}

pub async fn create_api_key_web(web: &mut WebDriver, config: &Config) -> anyhow::Result<String> {
    login(web, config).await?;

    web.screenshot("create_api_key_web/before-goto").await?;

    web.goto("/keys").await?;

    web.screenshot("create_api_key_web/after-goto").await?;

    let btn = web
        .wait()
        .for_element(Locator::Id("create-key"))
        .await
        .context("Failed to wait for button to create API key")?;

    log::debug!("Got button ({:?}), clicking it ...", btn);

    web.screenshot("create_api_key_web/before-click").await?;
    // FIXME: try with a delay, to see if the still needs some event listener to be attached
    tokio::time::sleep(Duration::from_secs(5)).await;
    web.screenshot("create_api_key_web/before-click-2").await?;

    btn.click().await?;

    web.screenshot("create_api_key_web/after-click").await?;

    let mut clp = web
        .wait()
        .for_element(Locator::Css(
            r#".pf-c-clipboard-copy input[name="api-key"]"#,
        ))
        .await
        .context("Failed to wait for API key value control")?;

    let key = clp.prop("value").await?;

    key.ok_or_else(|| anyhow!("Missing API key"))
}

#[async_trait]
impl ApiKeyCreator for TestContext {
    async fn create_api_key_web(&mut self) -> anyhow::Result<ApiKey> {
        let config = self.config().await?;
        let username = config.user.clone();
        let mut web = self.web().await?;
        with_screenshot(
            create_api_key_web(&mut web, &config)
                .await
                .map(|key| ApiKey { username, key }),
            "create_api_key_web",
            self,
        )
        .await
    }
}
