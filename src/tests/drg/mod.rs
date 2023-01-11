use crate::context::TestContext;
use test_context::test_context;

#[test_context(TestContext)]
#[tokio::test]
async fn test_drg_version(ctx: &mut TestContext) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;

    let version = drg.version()?;
    assert_eq!("0.11.0", version.get("drg").unwrap().as_str().unwrap());

    Ok(())
}
