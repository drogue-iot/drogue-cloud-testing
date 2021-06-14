use anyhow::Context;
use fantoccini::{Client, ClientBuilder};
use serde_json::{json, Map};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug)]
pub struct WebDriver {
    client: Client,
}

impl WebDriver {
    pub async fn new() -> anyhow::Result<Self> {
        let mut cap = Map::new();

        let mut args = vec![];

        if std::env::var("HEADLESS")
            .map(|s| s == "true")
            .unwrap_or_default()
        {
            args.push("-headless");
        }

        let opts = json!({
            "args": args,
        });

        cap.insert("moz:firefoxOptions".into(), opts);

        Ok(Self {
            client: ClientBuilder::native()
                .capabilities(cap)
                .connect("http://localhost:4444")
                .await
                .context("Failed to connect to web driver")?,
        })
    }
}

impl Deref for WebDriver {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for WebDriver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

#[cfg(test)]
mod test {
    use crate::context::TestContext;
    use std::time::Duration;
    use test_context::test_context;
    use tokio::time::sleep;

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_web_test(ctx: &mut TestContext) {
        let mut web = ctx.web().await.expect("Web Driver");

        web.goto("https://drogue.io")
            .await
            .expect("Must navigate to our homepage");

        sleep(Duration::from_millis(1000)).await;

        let current_url = web.current_url().await.expect("Failed to get current URL");

        assert_eq!("https://blog.drogue.io/", current_url.as_ref());
    }

    #[test_context(TestContext)]
    #[tokio::test]
    #[should_panic(expected = "Don't panic!")]
    async fn test_web_destroy(ctx: &mut TestContext) {
        let mut web = ctx.web().await.expect("Web Driver");

        web.goto("https://drogue.io")
            .await
            .expect("Must navigate to our homepage");

        sleep(Duration::from_millis(1000)).await;

        let _current_url = web.current_url().await.expect("Failed to get current URL");

        panic!("Don't panic!")
    }
}
