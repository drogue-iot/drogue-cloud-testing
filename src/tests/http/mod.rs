use crate::{context::TestContext, resources::apps::Application};
use serde_json::json;
use test_context::test_context;

#[test_context(TestContext)]
#[tokio::test]
async fn test_send_telemetry(ctx: &mut TestContext) -> anyhow::Result<()> {
    let drg = ctx.drg().await?;

    let app = Application::new_random(drg.clone()).expect("Create a new application");
    let device = app
        .create_device("device1", &json!({}))
        .expect("Create new device");

    Ok(())
}
