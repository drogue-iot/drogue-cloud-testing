use crate::{context::TestContext, init::access_token::AccessTokenCreator};
use test_context::test_context;

#[test_context(TestContext)]
#[tokio::test]
async fn test_create_access_token_web(ctx: &mut TestContext) {
    let token = ctx
        .create_access_token_web()
        .await
        .expect("Get access token");

    assert!(token.token.starts_with("drg_"));
}
