use anyhow::{anyhow, Context};
use std::{fs::File, io::BufReader, path::PathBuf};

/// Load a CA bundle
pub fn load_ca_certs(cert_file: PathBuf) -> anyhow::Result<Vec<reqwest::Certificate>> {
    Ok(vec![reqwest::Certificate::from_pem(&std::fs::read(
        cert_file,
    )?)?])
}

/// Load the default CA bundle
pub fn load_default_ca_certs() -> anyhow::Result<Vec<reqwest::Certificate>> {
    load_ca_certs(default_ca_certs_path()?).context("Failed to load default CA certificates")
}

pub fn default_ca_certs_path() -> anyhow::Result<PathBuf> {
    let cert_base: PathBuf = std::env::var_os("CERT_BASE")
        .ok_or_else(|| anyhow!("Missing 'CERT_BASE' variable"))?
        .into();

    Ok(cert_base.join("endpoints/ca-bundle.pem"))
}
