use crate::context::TestContext;
use crate::init::{config::Config, login::login, web::WebDriver};
use crate::tools::web::with_screenshot;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use fantoccini::Locator;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct AccessToken {
    pub username: String,
    pub token: String,
}

impl AccessToken {
    pub fn into_provider(self) -> drogue_client::openid::AccessTokenProvider {
        drogue_client::openid::AccessTokenProvider {
            user: self.username,
            token: self.token,
        }
    }
}

impl From<AccessToken> for drogue_client::openid::AccessTokenProvider {
    fn from(token: AccessToken) -> Self {
        token.into_provider()
    }
}

#[async_trait]
pub trait AccessTokenCreator {
    async fn create_access_token_web(&mut self) -> anyhow::Result<AccessToken>;
}

pub async fn create_access_token_web(
    web: &mut WebDriver,
    config: &Config,
) -> anyhow::Result<String> {
    login(web, config).await?;

    web.screenshot("create_access_token_web/before-goto")
        .await?;

    web.goto("/accesstokens").await?;

    web.screenshot("create_access_token_web/after-goto").await?;

    let btn = web
        .wait()
        .for_element(Locator::Id("create-token"))
        .await
        .context("Failed to wait for button to create access token")?;

    log::debug!("Got button ({:?}), clicking it ...", btn);

    web.screenshot("create_access_token_web/before-click")
        .await?;
    // FIXME: try with a delay, to see if the still needs some event listener to be attached
    tokio::time::sleep(Duration::from_secs(5)).await;
    web.screenshot("create_access_token_web/before-click-2")
        .await?;

    btn.click().await?;

    web.screenshot("create_access_token_web/after-click")
        .await?;

    let btn_confirm = web
        .wait()
        .for_element(Locator::Id("confirm-create-token"))
        .await
        .context("Failed to wait for button to confirm create access token")?;

    log::debug!("Got button ({:?}), clicking it ...", btn_confirm);

    web.screenshot("create_access_token_web/before-click-confirm")
        .await?;

    btn_confirm.click().await?;

    web.screenshot("create_access_token_web/after-click-confirm")
        .await?;

    let clp = web
        .wait()
        .for_element(Locator::Css(
            r#".pf-c-clipboard-copy input[name="access-token"]"#,
        ))
        .await
        .context("Failed to wait for access token value control")?;

    let key = clp.prop("value").await?;

    key.ok_or_else(|| anyhow!("Missing access token"))
}

#[async_trait]
impl AccessTokenCreator for TestContext {
    async fn create_access_token_web(&mut self) -> anyhow::Result<AccessToken> {
        let config = self.config().await?;
        let username = config.user.clone();
        let mut web = self.web().await?;
        with_screenshot(
            create_access_token_web(&mut web, &config)
                .await
                .map(|token| AccessToken { username, token }),
            "create_access_token_web",
            self,
        )
        .await
    }
}
