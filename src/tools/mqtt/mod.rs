mod device;
mod receiver;

pub use device::*;
pub use receiver::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MqttVersion {
    V3_1_1,
    V5(bool),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MqttVariant {
    Plain(MqttVersion),
    WebSocket(MqttVersion),
}

impl From<(MqttVersion, bool)> for MqttVariant {
    fn from(value: (MqttVersion, bool)) -> Self {
        match value.1 {
            true => MqttVariant::WebSocket(value.0),
            false => MqttVariant::Plain(value.0),
        }
    }
}

impl MqttVariant {
    pub fn version(&self) -> MqttVersion {
        match self {
            Self::Plain(version) | Self::WebSocket(version) => *version,
        }
    }
}

impl MqttVersion {
    pub fn is_binary(&self) -> bool {
        match self {
            Self::V5(binary) => *binary,
            _ => false,
        }
    }

    pub fn apply(&self, conn_opts: &mut paho_mqtt::ConnectOptionsBuilder) {
        match self {
            MqttVersion::V3_1_1 => {
                conn_opts
                    .mqtt_version(paho_mqtt::MQTT_VERSION_3_1_1)
                    .clean_session(true);
            }
            MqttVersion::V5(_) => {
                conn_opts
                    .mqtt_version(paho_mqtt::MQTT_VERSION_5)
                    .clean_start(true);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MqttQoS {
    QoS0,
    QoS1,
    QoS2,
}

impl From<MqttQoS> for i32 {
    fn from(qos: MqttQoS) -> Self {
        match qos {
            MqttQoS::QoS0 => 0,
            MqttQoS::QoS1 => 1,
            MqttQoS::QoS2 => 2,
        }
    }
}

pub fn scrub_uri<S>(uri: S) -> String
where
    S: AsRef<str>,
{
    format!("{}/mqtt", uri.as_ref().trim_end_matches('/').to_string())
}
