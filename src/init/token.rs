use crate::init::drg::Drg;
use async_trait::async_trait;
use tokio::task;

#[async_trait]
pub trait TokenProvider: Send + Sync {
    async fn current_token(&self) -> anyhow::Result<String>;

    async fn inject_into(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        let token = self.current_token().await?;
        Ok(builder.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token)))
    }
}

#[async_trait]
impl TokenProvider for Drg {
    async fn current_token(&self) -> anyhow::Result<String> {
        let drg = self.clone();
        Ok(task::spawn_blocking(move || drg.run(&["whoami", "-t"]))
            .await??
            .trim()
            .into())
    }
}

#[async_trait]
pub trait TokenInjector: Sized {
    async fn inject_token<T>(self, token: T) -> anyhow::Result<Self>
    where
        T: TokenProvider;
}

#[async_trait]
impl TokenInjector for reqwest::RequestBuilder {
    async fn inject_token<T>(self, token: T) -> anyhow::Result<Self>
    where
        T: TokenProvider,
    {
        Ok(token.inject_into(self).await?)
    }
}
