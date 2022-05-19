use super::*;
use crate::tools::{assert::CloudMessage, messages::WaitForMessages, tls};
use async_std::sync::Mutex;
use async_trait::async_trait;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, Transport};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{fs, sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use uuid::Uuid;

pub struct MqttReceiver {
    messages: Arc<Mutex<Vec<anyhow::Result<CloudMessage>>>>,
    do_ack: Arc<AtomicBool>,
    client: AsyncClient,
    ctx: JoinHandle<()>,
}

impl MqttReceiver {
    pub async fn new<H, S>(
        host: H,
        port: u16,
        username: Option<String>,
        password: Option<String>,
        topic: S,
        qos: MqttQoS,
    ) -> anyhow::Result<Self>
    where
        H: Into<String>,
        S: Into<String>,
    {
        let client_id = Uuid::new_v4().to_string();

        let mut opts = MqttOptions::new(client_id, host, port);

        opts.set_credentials(username.unwrap_or_default(), password.unwrap_or_default());
        opts.set_keep_alive(Duration::from_secs(30));

        let ca = fs::read(tls::default_ca_certs_path()?)?;
        opts.set_transport(Transport::tls(ca, None, None));

        // we always ack manually
        opts.set_manual_acks(true);

        // create client

        let (client, mut events) = AsyncClient::new(opts, 100);
        let strm_client = client.clone();

        client.subscribe(topic, qos.into()).await?;

        let do_ack = Arc::new(AtomicBool::new(true));
        let strm_do_ack = do_ack.clone();

        let messages = Arc::new(Mutex::new(Vec::new()));
        let strm_messages = messages.clone();

        let ctx = tokio::spawn(async move {
            log::info!("Starting message stream...");

            loop {
                let evt = events.poll().await;
                log::debug!("MQTT event: {evt:?}");
                match evt {
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        if strm_do_ack.load(Ordering::Acquire) {
                            log::debug!("Auto-ack'ed message");
                            strm_client.ack(&publish).await.ok();
                        }

                        let msg = MqttMessage::from(publish);
                        log::info!("Received: {:?}", msg);

                        let mut msgs = strm_messages.lock().await;
                        msgs.push(msg.into_message(false));
                    }
                    _ => {}
                }
            }
        });

        Ok(Self {
            messages,
            client,
            ctx,
            do_ack,
        })
    }

    pub async fn close(self) -> Vec<anyhow::Result<CloudMessage>> {
        self.drain_messages().await
    }

    pub fn set_ack(&self, ack: bool) {
        self.do_ack.store(ack, Ordering::Release);
    }
}

#[async_trait]
impl WaitForMessages for MqttReceiver {
    async fn num_messages(&self) -> usize {
        self.messages.lock().await.len()
    }
}

#[async_trait]
impl WarmupReceiver for MqttReceiver {
    async fn drain_messages(&self) -> Vec<anyhow::Result<CloudMessage>> {
        self.messages.lock().await.drain(..).collect()
    }
}

impl Drop for MqttReceiver {
    fn drop(&mut self) {
        log::info!("Dropping MQTT receiver");
        self.ctx.abort();
    }
}
