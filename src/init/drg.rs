use crate::init::config::Config;
use crate::init::web::WebDriver;
use anyhow::{bail, Context};
use fantoccini::Locator;
use serde_json::Value;
use std::io::Write;
use std::process::Stdio;
use std::{ffi::OsStr, process::Command};

const CONTEXT: &str = "system-tests";

#[derive(Clone)]
pub struct Drg {}

impl Drg {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn auto_login(c: &mut WebDriver) -> anyhow::Result<Self> {
        log::debug!("auto login");

        let config = Config::new()?;

        // get endpoints

        let endpoints: serde_json::Value =
            reqwest::get(format!("{}.well-known/drogue-endpoints", config.api))
                .await
                .context("Failed to fetch endpoints")?
                .json()
                .await
                .context("Failed to parse endpoints")?;
        log::info!("Endpoints: {:#?}", endpoints);

        let console = endpoints["console"]
            .as_str()
            .expect("Missing console endpoint");

        // get access token

        // go to the login page
        c.goto(console).await?;

        c.wait()
            .for_element(Locator::Css("button.pf-c-button.pf-m-primary"))
            .await?
            .click()
            .await?;

        let mut form = c.form(Locator::Id("kc-form-login")).await?;
        form.set_by_name("username", &config.user)
            .await?
            .set_by_name("password", &config.password)
            .await?
            .submit()
            .await?;

        c.wait().for_element(Locator::Id("user-dropdown")).await?;

        // go to the token page
        c.goto(&format!("{}/token", console)).await?;

        let refresh_token = c
            .wait()
            .for_element(Locator::Css("input.pf-c-form-control"))
            .await?
            .prop("value")
            .await?
            .unwrap_or_default();

        log::info!("Refresh token: {}", refresh_token);

        // do the login

        let drg = Self::new();
        drg.delete_context().ok();
        drg.login(config.api.to_string(), &refresh_token)?;

        // close window

        c.close_window().await?;

        // return result

        Ok(drg)
    }

    pub fn run<I, S>(&self, args: I) -> anyhow::Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.run_with_input(args, Option::<&[u8]>::None)
    }

    pub fn run_with_input<I, S, P>(&self, args: I, input: Option<P>) -> anyhow::Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        P: AsRef<[u8]>,
    {
        let mut cmd = Command::new("drg");
        cmd.args(args)
            .env("DRG_CONTEXT", CONTEXT)
            .stdin(Stdio::piped());

        log::info!("Running: {:?}", cmd);
        let output = if let Some(input) = input {
            let mut child = cmd.spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(input.as_ref())?;
            }

            child.wait_with_output()
        } else {
            cmd.output()
        };

        log::info!("Output: {:?}", output);
        let output = output?;

        if !output.status.success() {
            bail!("Command was not successful - rc = {}", output.status);
        }

        let out = output.stdout;

        Ok(String::from_utf8(out)?)
    }

    pub fn delete_context(&self) -> anyhow::Result<()> {
        self.run(&["context", "delete", CONTEXT])?;
        Ok(())
    }

    pub fn login<S: AsRef<str>>(&self, url: String, refresh_token: S) -> anyhow::Result<()> {
        self.run(&[
            "login",
            &url,
            "-t",
            &refresh_token.as_ref(),
            "--context",
            CONTEXT,
        ])?;
        Ok(())
    }

    pub fn version_str(&self) -> anyhow::Result<String> {
        self.run(&["version"])
    }

    pub fn contexts_str(&self) -> anyhow::Result<String> {
        self.run(&["context", "list"])
    }

    pub fn create_app(&self, name: &str) -> anyhow::Result<()> {
        self.run(&["create", "app", name])?;
        Ok(())
    }

    pub fn delete_app(&self, name: &str) -> anyhow::Result<()> {
        self.run(&["delete", "app", name])?;
        Ok(())
    }

    pub fn get_app(&self, name: &str) -> anyhow::Result<serde_json::Value> {
        let json = self.run(&["get", "app", name])?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn create_device(&self, app: &str, name: &str, spec: &Value) -> anyhow::Result<()> {
        let mut args = vec!["create", "device", "--app", app, name];

        let spec = match spec {
            Value::Object(_) => Some(serde_json::to_string(spec)?),
            Value::Null => None,
            _ => anyhow::bail!("Invalid device spec, but be null or object"),
        };

        if let Some(spec) = &spec {
            args.push("--spec");
            args.push(&spec);
        }

        self.run(&args)?;
        Ok(())
    }

    pub fn delete_device(&self, app: &str, name: &str) -> anyhow::Result<()> {
        self.run(&["delete", "device", "--app", app, name])?;
        Ok(())
    }

    pub fn get_device(&self, app: &str, name: &str) -> anyhow::Result<serde_json::Value> {
        let json = self.run(&["get", "device", "--app", app, name])?;
        Ok(serde_json::from_str(&json)?)
    }
}
