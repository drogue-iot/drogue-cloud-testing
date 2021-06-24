use crate::init::info::Information;
use crate::tools::Auth;
use std::collections::HashMap;
use url::Url;

#[derive(Clone, Debug)]
pub struct HttpSender {
    http_url: Url,
}

impl HttpSender {
    pub fn new(info: &Information) -> anyhow::Result<Self> {
        let http_url = info.http.url.clone();

        Ok(Self { http_url })
    }

    pub async fn send(
        &self,
        channel: String,
        auth: Auth,
        content_type: String,
        params: HashMap<String, String>,
        payload: Option<Vec<u8>>,
    ) -> anyhow::Result<reqwest::Response> {
        let client = reqwest::ClientBuilder::new().danger_accept_invalid_certs(true);

        let client = match &auth {
            Auth::X509Certificate(cert) => {
                let id = reqwest::Identity::from_pem(&cert)?;
                client.identity(id)
            }
            _ => client,
        };

        let client = client.build()?;

        let mut url = self.http_url.clone();
        url.set_path(&format!("/v1/{}", channel));

        url.query_pairs_mut().clear().extend_pairs(params.iter());

        log::info!("Sending payload");

        let request = client.post(url);

        let mut request = match &auth {
            Auth::None => request,
            Auth::UsernamePassword(username, password) => {
                request.basic_auth(username, Some(password))
            }
            Auth::X509Certificate(_) => request,
        };

        request = request.header(reqwest::header::CONTENT_TYPE, content_type);

        if let Some(payload) = payload {
            request = request.body(payload);
        }

        Ok(request.send().await.expect("HTTP call to succeed"))
    }
}
