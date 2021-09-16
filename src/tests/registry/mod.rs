use crate::{common::setup, context::TestContext, resources::apps::Application};
use serde_json::{json, Value};
use test_context::test_context;
use uuid::Uuid;

#[test_context(TestContext)]
#[tokio::test]
async fn test_registry_create_app(ctx: &mut TestContext) {
    setup();

    let uuid = Uuid::new_v4().to_string();

    Application::new(ctx.drg().await.unwrap(), &uuid)
        .expect("Created application")
        .expect_ready();
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_registry_create_app_twice(ctx: &mut TestContext) {
    setup();

    let uuid = Uuid::new_v4().to_string();

    // first attempt must succeed

    let _app1 = Application::new(ctx.drg().await.unwrap(), &uuid)
        .expect("Created application")
        .expect_ready();

    // second attempt needs to fail
    let app2 = Application::new(ctx.drg().await.unwrap(), &uuid);
    assert!(app2.is_err());
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_registry_create_and_delete(ctx: &mut TestContext) {
    setup();

    let drg = ctx.drg().await.unwrap();

    let uuid = Uuid::new_v4().to_string();

    // first attempt must succeed

    let mut app1 = Application::new(drg.clone(), &uuid)
        .expect("Created application")
        .expect_ready();
    drg.delete_app(&uuid).expect("Deleted application");

    let r = drg.delete_app(&uuid);
    assert!(r.is_err());

    // mark deleted to not fail when dropping
    app1.mark_deleted();
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_registry_create_app_and_device(ctx: &mut TestContext) {
    setup();

    let uuid = Uuid::new_v4().to_string();

    let app = Application::new(ctx.drg().await.unwrap(), &uuid)
        .expect("Created application")
        .expect_ready();
    let _device = app
        .create_device(
            "id1",
            &json!({
                "foo": "bar,"
            }),
        )
        .expect("Created device");
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_registry_create_app_and_device_twice(ctx: &mut TestContext) {
    setup();

    let uuid = Uuid::new_v4().to_string();

    let app = Application::new(ctx.drg().await.unwrap(), &uuid)
        .expect("Created application")
        .expect_ready();
    let _device = app
        .create_device(
            "id1",
            &json!({
                "foo": "bar,"
            }),
        )
        .expect("Created device");

    let err = app.create_device("id1", &Value::Null);

    assert!(err.is_err());
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_registry_device_create_and_delete(ctx: &mut TestContext) {
    setup();

    let drg = ctx.drg().await.unwrap();

    let uuid = Uuid::new_v4().to_string();

    // first attempt must succeed

    let app1 = Application::new(drg.clone(), &uuid)
        .expect("Created application")
        .expect_ready();
    let mut device1 = app1
        .create_device("id1", &Value::Null)
        .expect("Created device");

    drg.delete_device(app1.name(), "id1")
        .expect("Deleted device");

    // currently deleting a non-existent device returns an error
    let r = drg.delete_device(app1.name(), "id1");
    assert!(r.is_err());

    // mark deleted to not fail when dropping
    device1.mark_deleted();
}
