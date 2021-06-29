use crate::init::info::Information;
use crate::tools::Auth;
use std::collections::HashMap;
use url::Url;

pub trait ClientBuilderProvider {
    fn new_client_builder(&self) -> anyhow::Result<reqwest::ClientBuilder>;
}

#[derive(Clone, Debug)]
pub struct HttpSender<'cb, CB>
where
    CB: ClientBuilderProvider,
{
    http_url: Url,
    client_builder: &'cb CB,
}

impl<'cb, CB> HttpSender<'cb, CB>
where
    CB: ClientBuilderProvider,
{
    pub fn new(info: &Information, client_builder: &'cb CB) -> Self {
        let http_url = info.http.url.clone();

        Self {
            http_url,
            client_builder,
        }
    }

    pub async fn send(
        &self,
        channel: String,
        auth: Auth,
        content_type: String,
        params: HashMap<String, String>,
        payload: Option<Vec<u8>>,
    ) -> anyhow::Result<reqwest::Response> {
        let builder = self.client_builder.new_client_builder()?;

        let builder = match &auth {
            Auth::X509Certificate(cert) => {
                let id = reqwest::Identity::from_pem(&cert)?;
                builder.identity(id)
            }
            _ => builder,
        };

        let client = builder.build()?;

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
