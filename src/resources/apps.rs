use crate::init::drg::Drg;
use crate::resources::devices::Device;
use serde_json::Value;
use uuid::Uuid;

pub struct Application {
    drg: Drg,
    name: String,
    deleted: bool,
}

impl Application {
    pub fn new<S: Into<String>>(drg: Drg, name: S) -> anyhow::Result<Self> {
        let name = name.into();
        log::info!("Create application: {}", name);

        drg.create_app(&name)?;

        Ok(Self {
            drg,
            name,
            deleted: false,
        })
    }

    pub fn new_random(drg: Drg) -> anyhow::Result<Self> {
        let uuid = Uuid::new_v4();
        Self::new(drg, uuid.to_string())
    }

    pub fn mark_deleted(&mut self) {
        self.deleted = true;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn create_device<S: Into<String>>(
        &self,
        device: S,
        spec: &Value,
    ) -> anyhow::Result<Device> {
        Device::new(self, device, spec)
    }
}

impl Application {
    pub fn drg(&self) -> &Drg {
        &self.drg
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
