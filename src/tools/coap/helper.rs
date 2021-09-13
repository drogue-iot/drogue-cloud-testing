use coap::CoAPClient;
use coap_lite::{CoapOption, CoapRequest, CoapResponse};
use regex::Regex;
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;
use std::time::Duration;
use url::Url;

use crate::tools::Auth;

/// Execute a single get request with a coap url and a specific timeout.
pub fn get(
    url: String,
    channel: String,
    _content_type: String,
    params: HashMap<String, String>,
    payload: Option<Vec<u8>>,
    auth: Auth,
) -> Result<CoapResponse> {
    let (domain, port, path, queries) = parse_coap_url(format!("{}/v1/{}",url,channel))?;

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
    
    if let Some(p) = payload {
        packet.message.payload = p;
    } 

    let mut p = serde_urlencoded::to_string(params).unwrap().as_bytes().to_vec();
    packet.message.add_option(CoapOption::UriQuery, p.clone());

    if let Some(mut q) = queries {
        p.append(&mut "&".as_bytes().to_vec());
        p.append(&mut q);
        packet.message.add_option(CoapOption::UriQuery, q);
    }

    println!("{:#?}", packet);

    let client = CoAPClient::new((domain.as_str(), port))?;
    client.send(&packet)?;

    client.set_receive_timeout(Some(Duration::new(35, 0)))?;
    match client.receive() {
        Ok(receive_packet) => Ok(receive_packet),
        Err(e) => Err(e),
    }
}

fn parse_coap_url(url: String) -> Result<(String, u16, String, Option<Vec<u8>>)> {
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
        .replace(&host, "$1")
        .to_string();

    let port = match url_params.port() {
        Some(p) => p,
        None => 5683,
    };

    let path = url_params.path().to_string();

    let queries = url_params.query().map(|q| q.as_bytes().to_vec());

    return Ok((host.to_string(), port, path, queries));
}
