use crate::context::TestContext;
use crate::init::config::Config;
use crate::init::login::login;
use crate::init::web::WebDriver;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use fantoccini::Locator;

#[async_trait]
pub trait ApiKeyCreator {
    async fn create_api_key_web(&mut self) -> anyhow::Result<String>;
}

pub async fn create_api_key_web(web: &mut WebDriver, config: &Config) -> anyhow::Result<String> {
    login(web, config).await?;

    web.goto("/keys").await?;

    let btn = web
        .wait()
        .for_element(Locator::Id("create-key"))
        .await
        .context("Failed to wait for button to create API key")?;
    btn.click().await?;

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
    async fn create_api_key_web(&mut self) -> anyhow::Result<String> {
        let mut web = self.web().await?;
        create_api_key_web(&mut web, &self.config().await?).await
    }
}
