use crate::init::info::Information;
use crate::tools::Auth;
use coap_lite::CoapResponse;
use std::collections::HashMap;
use url::Url;

mod helper;

pub struct CoapSender {
    coap_url: Url,
}

impl CoapSender {
    pub fn new(info: &Information) -> Self {
        CoapSender {
            coap_url: info.coap.url.clone(),
        }
    }

    pub fn send(
        &self,
        channel: String,
        auth: Auth,
        content_type: String,
        params: HashMap<String, String>,
        payload: Option<Vec<u8>>,
    ) -> anyhow::Result<CoapResponse> {
        let mut url = self.coap_url.to_string();

        println!("Client request: {}", url);

        Ok(helper::get(url.as_str(), auth).unwrap())
    }
}
