use anyhow::Result;
use bedrockci::download::server::{ServerDownloadError, download_server};
use bedrockci::server_path::get_server_path;

pub async fn handle_download(
    version: Option<String>,
    accepted_eula_and_privacy_policy: bool,
    force_reinstall: bool,
) -> Result<()> {
    let path = get_server_path(true)?;
    let version = version.unwrap_or_else(|| "1.21.84.1".to_string());

    match download_server(
        &version,
        path,
        accepted_eula_and_privacy_policy,
        force_reinstall,
    )
    .await
    {
        Ok(_) => println!("Server downloaded successfully!"),
        Err(ServerDownloadError::EulaAndPrivacyPolicyNotAccepted) => {
            eprintln!(
                "Please run the command with the --accept-eula flag to accept the EULA and Privacy Policy."
            );
            eprintln!("You must accept the EULA and Privacy Policy to download the server.");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error downloading server: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
