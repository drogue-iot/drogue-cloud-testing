use crate::resources::apps::Application;
use serde_json::Value;

pub struct Device<'a> {
    app: &'a Application,
    name: String,
    deleted: bool,
}

impl<'a> Device<'a> {
    pub fn new<S: Into<String>>(
        app: &'a Application,
        name: S,
        spec: &Value,
    ) -> anyhow::Result<Self> {
        let name = name.into();
        log::info!("Create device: {}", name);

        app.drg().create_device(app.name(), &name, spec)?;

        Ok(Self {
            app,
            name,
            deleted: false,
        })
    }

    pub fn mark_deleted(&mut self) {
        self.deleted = true;
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<'a> Drop for Device<'a> {
    fn drop(&mut self) {
        if self.deleted {
            log::info!("Skipping deletion of '{}'", self.name);
        } else {
            log::info!("Destroy application '{}'", self.name);
            match self.app.drg().delete_device(self.app.name(), &self.name) {
                Ok(_) => {}
                Err(err) => {
                    log::error!("Failed to delete: {}", err);
                }
            }
        }
    }
}
