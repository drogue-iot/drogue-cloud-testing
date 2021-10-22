use crate::{context::TestContext, init::api_key::ApiKeyCreator};
use test_context::test_context;

#[test_context(TestContext)]
#[tokio::test]
async fn test_create_api_key_web(ctx: &mut TestContext) {
    let key = with_screenshot(
        ctx.create_api_key_web().await,
        "test_create_api_key_web",
        ctx,
    )
    .await
    .expect("Get API key");

    assert!(key.key.starts_with("drg_"));
}

async fn with_screenshot<T, E>(
    result: Result<T, E>,
    name: &str,
    ctx: &mut TestContext,
) -> Result<T, E> {
    match result {
        Ok(result) => Ok(result),
        Err(err) => {
            ctx.web()
                .await
                .expect("Web Driver")
                .screenshot(format!("{}.png", name))
                .await
                .ok();
            Err(err)
        }
    }
}
