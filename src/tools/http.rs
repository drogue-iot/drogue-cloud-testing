use crate::{
    init::info::Information,
    tools::{Auth, SendAs},
};
use url::{
    form_urlencoded::{Serializer, Target},
    Url,
};

pub trait ClientBuilderProvider {
    fn new_client_builder(&self) -> anyhow::Result<reqwest::ClientBuilder>;
}

#[derive(Clone, Debug, Default)]
pub struct HttpSenderOptions {
    pub application: Option<String>,
    pub device: Option<String>,
    // device name to send as (proxied device)
    pub r#as: Option<String>,
    pub command_timeout: Option<u32>,
}

impl From<SendAs> for HttpSenderOptions {
    fn from(send_as: SendAs) -> Self {
        match send_as {
            SendAs::Device => Default::default(),
            SendAs::Gateway { device } => HttpSenderOptions {
                r#as: Some(device),
                ..Default::default()
            },
        }
    }
}

impl From<&SendAs> for HttpSenderOptions {
    fn from(send_as: &SendAs) -> Self {
        match send_as {
            SendAs::Device => Default::default(),
            SendAs::Gateway { device } => HttpSenderOptions {
                r#as: Some(device.clone()),
                ..Default::default()
            },
        }
    }
}

impl HttpSenderOptions {
    pub fn append_query<T: Target>(&self, params: &mut Serializer<T>) {
        if let Some(ct) = self.command_timeout.as_ref() {
            params.append_pair("ct", &format!("{}", ct));
        }
        if let Some(application) = self.application.as_ref() {
            params.append_pair("application", application);
        }
        if let Some(device) = self.device.as_ref() {
            params.append_pair("device", device);
        }
        if let Some(r#as) = self.r#as.as_ref() {
            params.append_pair("as", r#as);
        }
    }
}

#[derive(Debug)]
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
        auth: &Auth,
        content_type: Option<String>,
        options: &HttpSenderOptions,
        payload: Option<Vec<u8>>,
    ) -> anyhow::Result<reqwest::Response> {
        let builder = self.client_builder.new_client_builder()?;

        let builder = match auth {
            Auth::X509Certificate(cert) => {
                let id = reqwest::Identity::from_pem(&cert)?;
                builder.identity(id)
            }
            _ => builder,
        };

        let client = builder.build()?;

        let mut url = self.http_url.clone();
        url.set_path(&format!("/v1/{}", channel));

        {
            let mut params = url.query_pairs_mut();
            params.clear();
            options.append_query(&mut params);
        }

        log::info!("Sending payload ({})", url);

        let request = client.post(url);

        let mut request = match &auth {
            Auth::None => request,
            Auth::UsernamePassword(username, password) => {
                request.basic_auth(username, Some(password))
            }
            Auth::X509Certificate(_) => request,
        };

        if let Some(content_type) = content_type {
            request = request.header(reqwest::header::CONTENT_TYPE, content_type);
        }

        if let Some(payload) = payload {
            request = request.body(payload);
        }

        Ok(request.send().await.expect("HTTP call to succeed"))
    }
}
