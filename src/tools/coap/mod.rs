use crate::init::info::Information;
use crate::tools::http::HttpSenderOptions;
use crate::tools::Auth;
use coap_lite::CoapResponse;
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

    pub async fn send(
        &self,
        channel: String,
        auth: Auth,
        content_type: String,
        options: &HttpSenderOptions,
        payload: Option<Vec<u8>>,
    ) -> anyhow::Result<CoapResponse> {
        let url = self.coap_url.to_string();

        log::debug!("Client request: {}", url);

        Ok(helper::get(url, channel, content_type, options, payload, auth).await?)
    }
}
