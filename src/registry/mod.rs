use crate::common::setup;
use crate::context::TestContext;
use crate::init::drg::Drg;
use test_context::test_context;
use uuid::Uuid;

pub struct Application {
    drg: Drg,
    name: String,
}

impl Application {
    pub fn new<S: Into<String>>(drg: Drg, name: S) -> anyhow::Result<Self> {
        log::info!("Create application");

        let name = name.into();

        drg.create_app(&name)?;

        Ok(Self {
            drg,
            name: name.into(),
        })
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        log::info!("Destroy application");
        match self.drg.delete_app(&self.name) {
            Ok(_) => {}
            Err(err) => {
                log::error!("Failed to delete: {}", err);
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
    let app1 = Application::new(ctx.drg().await.unwrap(), &uuid).expect("Created application");
    let app2 = Application::new(ctx.drg().await.unwrap(), &uuid);
    assert!(app2.is_err());
}
