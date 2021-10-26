use crate::{context::TestContext, init::api_key::ApiKeyCreator};
use anyhow::Context;
use drogue_client::{meta, registry};
use serde_json::{json, Value};
use test_context::test_context;
use uuid::Uuid;

#[test_context(TestContext)]
#[tokio::test]
async fn test_registry_create_app(ctx: &mut TestContext) -> anyhow::Result<()> {
    let uuid = Uuid::new_v4().to_string();

    let key = ctx.create_api_key_web().await.context("Acquire API key")?;

    let reg = ctx
        .registry(key.into_provider())
        .await
        .context("Get registry client")?;

    reg.create_app(&new_app(&uuid, json!({})), Default::default())
        .await
        .context("App created")?;

    reg.delete_app(&uuid, Default::default())
        .await
        .context("App deleted")?;

    Ok(())
}

fn new_app<S>(name: S, spec: Value) -> registry::v1::Application
where
    S: Into<String>,
{
    let spec = match spec {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    registry::v1::Application {
        metadata: meta::v1::NonScopedMetadata {
            name: name.into(),
            ..Default::default()
        },
        spec,
        ..Default::default()
    }
}
