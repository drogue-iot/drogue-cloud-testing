use crate::context::TestContext;
use test_context::test_context;

#[test_context(TestContext)]
#[tokio::test]
async fn test_drg_version(ctx: &mut TestContext) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;

    assert_eq!(
        r#"Drg Version: 0.7.0
Connected drogue-cloud service: v0.8.0
"#,
        drg.version_str().unwrap()
    );

    Ok(())
}
