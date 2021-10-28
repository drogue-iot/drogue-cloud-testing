pub mod assert;
pub mod coap;
pub mod command;
pub mod http;
pub mod messages;
pub mod mqtt;
pub mod tls;
pub mod websocket;
pub mod warmup;
pub mod web;

#[derive(Clone, Debug)]
pub enum Auth {
    None,
    UsernamePassword(String, String),
    X509Certificate(Vec<u8>),
}

impl Default for Auth {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug)]
pub enum SendAs {
    Device,
    Gateway { device: String },
}

impl Default for SendAs {
    fn default() -> Self {
        Self::Device
    }
}
