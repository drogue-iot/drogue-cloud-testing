use anyhow::Context;
use url::Url;

#[derive(Clone, Debug)]
pub struct Config {
    pub user: String,
    pub password: String,
    pub api: Url,
}

impl Config {
    pub fn new() -> anyhow::Result<Self> {
        log::debug!("Create new config");

        let user = std::env::var("TEST_USER").context("Missing 'TEST_USER' variable")?;
        let password =
            std::env::var("TEST_PASSWORD").context("Missing 'TEST_PASSWORD' variable")?;

        let api = std::env::var("API_URL")
            .context("Missing 'API_URL' variable")?
            .parse()?;

        Ok(Self {
            user,
            password,
            api,
        })
    }

    pub async fn console(&self) -> anyhow::Result<String> {
        // get endpoints

        let endpoints: serde_json::Value =
            reqwest::get(format!("{}.well-known/drogue-endpoints", self.api))
                .await
                .context("Failed to fetch endpoints")?
                .json()
                .await
                .context("Failed to parse endpoints")?;
        log::info!("Endpoints: {:#?}", endpoints);

        let console = endpoints["console"]
            .as_str()
            .expect("Missing console endpoint");

        Ok(console.to_string())
    }
}
