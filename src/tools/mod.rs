pub mod assert;
pub mod http;
pub mod mqtt;

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
