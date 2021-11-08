use crate::{
    init::{drg::Drg, info::Information, token::TokenInjector},
    resources::{apps::Application, devices::Device},
};
use serde_json::json;

pub async fn send_http_command(
    info: &Information,
    drg: &Drg,
    app: &Application,
    device: &Device<'_>,
    command: &str,
) -> anyhow::Result<reqwest::Response> {
    let client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;

    // now we can send the command back

    let mut command_url = info.command_url.clone();
    command_url.set_path(&format!(
        "/api/command/v1alpha1/apps/{appId}/devices/{deviceId}",
        appId = app.name(),
        deviceId = device.name(),
    ));
    command_url
        .query_pairs_mut()
        .append_pair("command", command);
    let command = client
        .post(command_url)
        .inject_token(drg.clone())
        .await?
        .json(&json!({
            "set-command": "foo",
        }))
        .send()
        .await?;

    Ok::<_, anyhow::Error>(command)
}
