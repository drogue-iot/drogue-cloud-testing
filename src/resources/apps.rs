use crate::init::drg::Drg;
use crate::resources::devices::Device;
use serde_json::Value;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

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

    pub fn wait_ready(&self) -> anyhow::Result<()> {
        // we first wait for KafkaReady, as that gets added shortly after the application was created
        self.wait_condition("KafkaReady", Duration::from_secs(5 * 60))?;
        // then we wait for the global "ready", which now includes kafka
        self.wait_condition("Ready", Duration::from_secs(5 * 60))?;
        // all good
        Ok(())
    }

    pub fn expect_ready(self) -> Self {
        self.wait_ready().unwrap_or_else(|err| {
            panic!(
                "Expect application '{}' to become ready: {}",
                self.name, err
            )
        });
        self
    }

    pub fn wait_condition(&self, condition: &str, duration: Duration) -> anyhow::Result<()> {
        let start = SystemTime::now();

        loop {
            let json = self.drg.get_app(&self.name)?;
            log::debug!("Application: {:?}", json);
            let state = json["status"]["conditions"]
                .as_array()
                .and_then(|conditions| {
                    conditions
                        .iter()
                        .find(|c| c["type"].as_str() == Some(condition))
                })
                .map(|c| c["status"].as_str() == Some("True"))
                .unwrap_or_default();

            log::debug!("Application - Condition: {} = {}", condition, state);

            if state {
                return Ok(());
            }

            if SystemTime::now().duration_since(start)? > duration {
                anyhow::bail!(
                    "Timeout expired. Condition '{}' still not ready.",
                    condition
                );
            }

            sleep(Duration::from_secs(1));
        }
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
