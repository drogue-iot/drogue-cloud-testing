use crate::init::config::Config;
use crate::init::web::WebDriver;
use fantoccini::error::CmdError;
use fantoccini::Locator;
use std::time::Duration;
use url::Url;

/// ensure that the web driver is logged in to the console
pub async fn login(web: &mut WebDriver, config: &Config) -> anyhow::Result<()> {
    let url = config.console().await?;

    // go to the login page
    web.goto(&url).await?;

    match web
        .wait()
        .at_most(Duration::from_secs(2))
        .for_element(Locator::Id("user-dropdown"))
        .await
    {
        Ok(_) => {
            log::info!("Early return, we are already logged in");
            return Ok(());
        }
        Err(CmdError::WaitTimeout) | Err(CmdError::NoSuchElement(_)) => {
            log::info!("Performing console login");
            // continue
        }
        Err(err) => return Err(err.into()),
    }

    web.wait()
        .for_element(Locator::Css("button.pf-c-button.pf-m-primary"))
        .await?
        .click()
        .await?;

    let mut form = web.form(Locator::Id("kc-form-login")).await?;
    form.set_by_name("username", &config.user)
        .await?
        .set_by_name("password", &config.password)
        .await?
        .submit()
        .await?;

    web.wait().for_element(Locator::Id("user-dropdown")).await?;

    Ok(())
}
