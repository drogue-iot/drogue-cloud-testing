use serde_json::Value;
use std::cmp::max;

/// A message received on the cloud from a device.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudMessage {
    pub subject: String,
    pub r#type: String,
    pub instance: String,
    pub app: String,
    pub device: String,
    pub sender: String,
    pub content_type: Option<String>,
    pub payload: Vec<u8>,
}

/// A message for a device to send to the cloud.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceMessage {
    pub subject: String,
    pub app: String,
    pub device: String,
    pub content_type: Option<String>,
    pub payload: Option<Vec<u8>>,
}

pub fn assert_msgs(actual: &Vec<anyhow::Result<CloudMessage>>, expected: &Vec<CloudMessage>) {
    let len = max(actual.len(), expected.len());

    for i in 0..len {
        let a = actual.get(i);
        let e = expected.get(i);

        match (a, e) {
            (Some(Ok(a)), Some(e)) => assert_eq!(a, e, "Position #{}", i),
            (Some(Err(err)), Some(_)) => panic!("Position #{}: Failed conversion: {}", i, err),
            (None, None) => {}
            (Some(_), None) => panic!("Expected no message at position #{} but found one", i),
            (None, Some(_)) => panic!("Expected message at position #{} but got none", i),
        }
    }
}

impl From<Value> for CloudMessage {
    fn from(json: Value) -> Self {
        let payload = match (json["data_base64"].as_str(), json["data"].as_object()) {
            (Some(data), _) => base64::decode(data).expect("Base64 decode"),
            (_, Some(json)) => serde_json::to_vec(json).expect("JSON decode"),
            (None, None) => vec![],
        };

        CloudMessage {
            subject: json["subject"].as_str().unwrap_or_default().into(),
            r#type: json["type"].as_str().unwrap_or_default().into(),
            instance: json["instance"].as_str().unwrap_or_default().into(),
            app: json["application"].as_str().unwrap_or_default().into(),
            device: json["device"].as_str().unwrap_or_default().into(),
            sender: json["sender"].as_str().unwrap_or_default().into(),
            content_type: json["datacontenttype"].as_str().map(|s| s.into()),
            payload,
        }
    }
}
