pub mod paho;
pub mod rumqtt;
mod warmup;

pub use warmup::*;

use super::*;
use crate::tools::assert::CloudMessage;
use anyhow::Context;
use rumqttc::Publish;
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
};

#[derive(Clone, Eq, PartialEq)]
pub struct MqttMessage {
    pub topic: String,
    pub user_properties: HashMap<String, String>,
    pub content_type: Option<String>,
    pub payload: Vec<u8>,
}

impl Debug for MqttMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("MqttMessage");

        d.field("topic", &self.topic)
            .field("content_type", &self.content_type)
            .field("user_properties", &self.user_properties);

        match String::from_utf8(self.payload.clone()) {
            Ok(payload) => d.field("payload", &payload),
            Err(_) => d.field("payload", &self.payload),
        };

        d.finish()
    }
}

impl MqttMessage {
    pub fn into_message(self, binary: bool) -> anyhow::Result<CloudMessage> {
        match binary {
            true => self.into_message_binary(),
            false => self.into_message_structured(),
        }
    }

    pub fn into_message_structured(self) -> anyhow::Result<CloudMessage> {
        if let Some(content_type) = self.content_type {
            if content_type.as_str() != "application/cloudevents+json; charset=utf-8" {
                anyhow::bail!("Wrong content type: {}", content_type);
            }
        }

        let json: Value = serde_json::from_slice(&self.payload).context("Parse as JSON")?;

        Ok(CloudMessage::from(json))
    }

    pub fn into_message_binary(self) -> anyhow::Result<CloudMessage> {
        Ok(CloudMessage {
            subject: self
                .user_properties
                .get("subject")
                .map(Into::into)
                .unwrap_or_default(),
            r#type: self
                .user_properties
                .get("type")
                .map(Into::into)
                .unwrap_or_default(),
            instance: self
                .user_properties
                .get("instance")
                .map(Into::into)
                .unwrap_or_default(),
            app: self
                .user_properties
                .get("application")
                .map(Into::into)
                .unwrap_or_default(),
            device: self
                .user_properties
                .get("device")
                .map(Into::into)
                .unwrap_or_default(),
            sender: self
                .user_properties
                .get("sender")
                .map(Into::into)
                .unwrap_or_default(),
            content_type: self.content_type,
            payload: self.payload,
        })
    }
}

impl From<paho_mqtt::Message> for MqttMessage {
    fn from(msg: paho_mqtt::Message) -> Self {
        Self {
            topic: msg.topic().into(),
            user_properties: msg.properties().user_iter().collect::<HashMap<_, _>>(),
            content_type: msg
                .properties()
                .get_string(paho_mqtt::PropertyCode::ContentType),
            payload: msg.payload().into(),
        }
    }
}

impl From<Publish> for MqttMessage {
    fn from(msg: Publish) -> Self {
        Self {
            topic: msg.topic,
            user_properties: Default::default(),
            content_type: Default::default(),
            payload: msg.payload.to_vec(),
        }
    }
}
