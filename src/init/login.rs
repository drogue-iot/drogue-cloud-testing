use crate::init::config::Config;
use crate::init::web::WebDriver;
use fantoccini::error::CmdError;
use fantoccini::Locator;

/// ensure that the web driver is logged in to the console
pub async fn login(web: &mut WebDriver, config: &Config) -> anyhow::Result<()> {
    // go to the login page
    web.goto(&config.console().await?).await?;

    if let Ok(_) = web.find(Locator::Id("user-dropdown")).await {
        // we are already logged in
        return Ok(());
    }

    match web.find(Locator::Id("user-dropdown")).await {
        Ok(_) => return Ok(()),
        Err(CmdError::NoSuchElement(_)) => {
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
