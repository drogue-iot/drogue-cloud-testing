use crate::tools::mqtt::MqttMessage;
use serde_json::Value;
use std::cmp::max;

#[derive(Clone, Debug)]
pub struct Message {
    pub subject: String,
    pub r#type: String,
    pub instance: String,
    pub app: String,
    pub device: String,
    pub content_type: Option<String>,
}

pub fn assert_mqtt_msgs(actual: &Vec<MqttMessage>, expected: &Vec<Message>) {
    let len = max(actual.len(), expected.len());

    for i in 0..len {
        let a = actual.get(i);
        let e = expected.get(i);

        match (a, e) {
            (Some(a), Some(e)) => assert_mqtt_msg(i, a, e),
            (None, None) => {}
            (Some(_), None) => panic!("Expected no message at position #{} but found one", i),
            (None, Some(_)) => panic!("Expected message at position #{} but got none", i),
        }
    }
}

pub fn assert_mqtt_msg(pos: usize, actual: &MqttMessage, expected: &Message) {
    assert_eq!(
        actual.topic,
        format!("app/{}", expected.app),
        "Position #{}: Topic mismatch",
        pos
    );
    let json: Value = serde_json::from_slice(&actual.payload).expect("Parse as JSON");

    assert_eq!(
        json["specversion"].as_str(),
        Some("1.0"),
        "Position #{}: spec version mismatch",
        pos
    );
    assert_eq!(
        json["type"].as_str(),
        Some(expected.r#type.as_str()),
        "Position #{}: Type mismatch",
        pos
    );
    assert_eq!(
        json["datacontenttype"].as_str(),
        expected.content_type.as_deref(),
        "Position #{}: data content type mismatch",
        pos
    );
    assert_eq!(
        json["subject"].as_str(),
        Some(expected.subject.as_str()),
        "Position #{}: subject mismatch",
        pos
    );
    assert_eq!(
        json["instance"].as_str(),
        Some(expected.instance.as_str()),
        "Position #{}: instance mismatch",
        pos
    );
    assert_eq!(
        json["application"].as_str(),
        Some(expected.app.as_str()),
        "Position #{}: application mismatch",
        pos
    );
    assert_eq!(
        json["device"].as_str(),
        Some(expected.device.as_str()),
        "Position #{}: device name mismatch",
        pos
    );
}
