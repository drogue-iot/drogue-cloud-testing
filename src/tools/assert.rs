use std::cmp::max;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub subject: String,
    pub r#type: String,
    pub instance: String,
    pub app: String,
    pub device: String,
    pub content_type: Option<String>,
    pub payload: Vec<u8>,
}

pub fn assert_mqtt_msgs(actual: &Vec<anyhow::Result<Message>>, expected: &Vec<Message>) {
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
