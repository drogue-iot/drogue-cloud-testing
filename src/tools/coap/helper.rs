use bytes::Bytes;
use coap_lite::{CoapOption, CoapRequest, CoapResponse, Packet, RequestType};
use futures::{SinkExt, StreamExt};
use openssl::ssl::{Ssl, SslContext, SslMethod};
use regex::Regex;
use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio_dtls_stream_sink::Client as DtlsClient;
use tokio_openssl::SslStream;
use tokio_util::codec::{BytesCodec, Decoder};
use url::{form_urlencoded, Url};

use crate::tools::http::HttpSenderOptions;
use crate::tools::{tls, Auth};

/// Execute a single get request with a coap url and a specific timeout.
pub async fn post(
    url: String,
    channel: String,
    _content_type: String,
    options: &HttpSenderOptions,
    payload: Option<Vec<u8>>,
    auth: Auth,
) -> anyhow::Result<CoapResponse> {
    let (domain, port, path) = parse_coap_url(format!("{}/v1/{}", url, channel))?;

    let mut b64: String = String::new();
    if let Auth::UsernamePassword(uname, passwd) = auth {
        b64 = base64::encode_config(format!("{}:{}", uname, passwd), base64::STANDARD_NO_PAD);
    }

    let auth_header = format!("Basic {}", b64).as_bytes().to_vec();

    let mut packet: CoapRequest<SocketAddr> = CoapRequest::new();
    packet.set_path(path.as_str());
    packet
        .message
        .add_option(CoapOption::Unknown(4209), auth_header);
    packet.set_method(RequestType::Post);

    if let Some(p) = payload {
        packet.message.payload = p;
    }

    let mut query = form_urlencoded::Serializer::new(String::new());
    options.append_query(&mut query);
    let p = query.finish();

    packet.message.add_option(CoapOption::UriQuery, p.into());

    log::debug!("{:#?}", packet);

    let dest = (domain.as_str(), port);
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let client = DtlsClient::new(socket);

    let mut ctx = SslContext::builder(SslMethod::dtls())?;
    ctx.set_ca_file(tls::default_ca_certs_path()?)?;
    let ctx = ctx.build();

    let mut session = client.connect(dest, Some(&ctx)).await?;
    let payload = packet.message.to_bytes()?;
    session.write(&payload[..]).await?;

    let mut rx_buf = [0; 2048];

    let len = session.read(&mut rx_buf[..]).await?;
    let packet = Packet::from_bytes(&rx_buf[..len])?;
    Ok(CoapResponse { message: packet });
}

fn parse_coap_url(url: String) -> Result<(String, u16, String)> {
    let url_params = match Url::parse(url.as_str()) {
        Ok(url_params) => url_params,
        Err(_) => return Err(Error::new(ErrorKind::InvalidInput, "url error")),
    };

    let host = match url_params.host_str() {
        Some("") => return Err(Error::new(ErrorKind::InvalidInput, "host error")),
        Some(h) => h,
        None => return Err(Error::new(ErrorKind::InvalidInput, "host error")),
    };
    let host = Regex::new(r"^\[(.*?)]$")
        .unwrap()
        .replace(host, "$1")
        .to_string();

    let port = url_params.port().unwrap_or(5683);
    let path = url_params.path().to_string();

    Ok((host, port, path))
}
