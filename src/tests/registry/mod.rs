use crate::common::setup;
use crate::context::TestContext;
use crate::init::drg::Drg;
use test_context::test_context;
use uuid::Uuid;

pub struct Application {
    drg: Drg,
    name: String,
    deleted: bool,
}

impl Application {
    pub fn new<S: Into<String>>(drg: Drg, name: S) -> anyhow::Result<Self> {
        log::info!("Create application");

        let name = name.into();

        drg.create_app(&name)?;

        Ok(Self {
            drg,
            name: name.into(),
            deleted: false,
        })
    }

    pub fn mark_deleted(&mut self) {
        self.deleted = true;
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        if self.deleted {
            log::info!("Skipping deletion of '{}'", self.name);
        } else {
            log::info!("Destroy application '{}'", self.name);
            match self.drg.delete_app(&self.name) {
                Ok(_) => {}
                Err(err) => {
                    log::error!("Failed to delete: {}", err);
                }
            }
        }
    }
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_registry_create_app(ctx: &mut TestContext) {
    setup();

    let uuid = Uuid::new_v4().to_string();

    Application::new(ctx.drg().await.unwrap(), &uuid).expect("Created application");
}

#[test_context(TestContext)]
#[tokio::test]
async fn test_registry_create_app_twice(ctx: &mut TestContext) {
    setup();

    let uuid = Uuid::new_v4().to_string();

    // first attempt must succeed

    let _app1 = Application::new(ctx.drg().await.unwrap(), &uuid).expect("Created application");

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

    let mut app1 = Application::new(drg.clone(), &uuid).expect("Created application");
    drg.delete_app(&uuid).expect("Deleted application");

    // currently deleting a non-existent app returns an error
    let r = drg.delete_app(&uuid);
    assert!(r.is_err());

    // mark deleted to not fail when dropping
    app1.mark_deleted()
}
