mod receiver;
mod sender;

pub use receiver::*;
pub use sender::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MqttVersion {
    V3_1_1,
    V5(bool),
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
