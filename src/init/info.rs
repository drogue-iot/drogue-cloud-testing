use crate::init::{
    config::Config,
    token::{TokenInjector, TokenProvider},
};
use reqwest::Url;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Information {
    pub api: Url,
    pub http: UrlInformation,
    pub coap: UrlInformation,
    pub mqtt: HostPortInformation,
    pub mqtt_integration: HostPortInformation,
    pub command_url: Url,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UrlInformation {
    pub url: Url,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HostPortInformation {
    pub host: String,
    pub port: u16,
}

impl Information {
    pub async fn new<T>(client: reqwest::Client, config: Config, token: T) -> anyhow::Result<Self>
    where
        T: TokenProvider,
    {
        Ok(client
            .get(config.api.join("/api/console/v1alpha1/info")?)
            .inject_token(token)
            .await?
            .send()
            .await?
            .json()
            .await?)
    }
}
