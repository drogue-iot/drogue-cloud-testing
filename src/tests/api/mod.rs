use crate::init::api_key::ApiKeyCreator;
use crate::{common::setup, context::TestContext};
use test_context::test_context;

#[test_context(TestContext)]
#[tokio::test]
async fn test_create_api_key_web(ctx: &mut TestContext) {
    setup();

    let key = ctx.create_api_key_web().await.expect("Create API key");

    assert!(key.starts_with("drg_"));
}
