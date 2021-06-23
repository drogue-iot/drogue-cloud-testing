use crate::init::info::Information;
use crate::resources::devices::Device;
use url::Url;

pub enum Auth<'a> {
    None,
    Password(&'a str),
    UsernamePassword(&'a str, &'a str),
    X509Certificate(&'a [u8]),
}

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
        device: &Device<'_>,
        auth: Auth<'_>,
        channel: &str,
    ) -> anyhow::Result<reqwest::Response> {
        let app = device.app().name().to_string();
        let device = device.name().to_string();

        let client = reqwest::ClientBuilder::new().danger_accept_invalid_certs(true);

        let client = match auth {
            Auth::X509Certificate(cert) => {
                let id = reqwest::Identity::from_pem(cert)?;
                client.identity(id)
            }
            _ => client,
        };

        let client = client.build()?;

        let mut url = self.http_url.clone();
        url.set_path(&format!("/v1/{}", channel));

        log::info!("Sending payload");

        let request = client.post(url);

        let request = match auth {
            Auth::None => request,
            Auth::Password(password) => {
                request.basic_auth(format!("{}@{}", device, app), Some(password))
            }
            Auth::UsernamePassword(username, password) => {
                request.basic_auth(format!("{}@{}", username, app), Some(password))
            }
            Auth::X509Certificate(_) => request,
        };

        Ok(request.send().await.expect("HTTP call to succeed"))
    }
}
