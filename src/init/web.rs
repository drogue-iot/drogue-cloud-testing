use anyhow::Context;
use fantoccini::{Client, ClientBuilder};
use serde_json::{json, Map};
use std::fs::{create_dir_all, write};
use std::ops::{Deref, DerefMut};
use std::path::Path;

use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

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

    pub async fn screenshot<S: AsRef<str>>(&mut self, name: S) -> anyhow::Result<()> {
        let data = self.client.screenshot().await?;

        let name = format!(
            "screenshots/{}/{}.png",
            name.as_ref(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let name = Path::new(&name);

        if let Some(parent) = name.parent() {
            create_dir_all(parent)?;
        }

        write(&name, &data)?;

        Ok(())
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
    use url::Url;

    #[test_context(TestContext)]
    #[tokio::test]
    async fn test_web_test(ctx: &mut TestContext) {
        let mut web = ctx.web().await.expect("Web Driver");

        web.goto("https://drogue.io")
            .await
            .expect("Must navigate to our homepage");

        sleep(Duration::from_millis(2000)).await;

        /*
        web.wait()
            .until(Duration::from_secs(5))
            .every(Duration::from_millis(250))
            .on_predicate(|client| {
                Box::pin(async move {
                    let current = client.current_url().await?;
                    Ok(current.as_str() == "https://blog.drogue.io/")
                })
            })
            .await
            .unwrap();
         */

        let current_url = web.current_url().await.expect("Failed to get current URL");

        web.screenshot("test_web_test/1").await.ok();

        assert_eq!("https://www.drogue.io/", current_url.as_ref());

        /*
        let mut element = web
            .wait()
            .until(Duration::from_secs(5))
            .every(Duration::from_millis(250))
            .on_element(Locator::Css(".hero"))
            .await
            .unwrap();

        assert_eq!(element.text().await.unwrap(), "");
        */
    }

    #[test_context(TestContext)]
    #[tokio::test]
    #[should_panic(expected = "Don't panic!")]
    async fn test_web_destroy(ctx: &mut TestContext) {
        let mut web = ctx.web().await.expect("Web Driver");

        web.goto("https://drogue.io")
            .await
            .expect("Must navigate to our homepage");

        web.wait()
            .for_url(Url::parse("https://www.drogue.io").unwrap())
            .await
            .unwrap();

        let _current_url = web.current_url().await.expect("Failed to get current URL");

        panic!("Don't panic!")
    }
}
