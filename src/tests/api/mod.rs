use crate::{context::TestContext, init::api_key::ApiKeyCreator};
use test_context::test_context;

#[test_context(TestContext)]
#[tokio::test]
async fn test_create_api_key_web(ctx: &mut TestContext) {
    let key = ctx.create_api_key_web().await.expect("Get API key");

    assert!(key.key.starts_with("drg_"));
}
