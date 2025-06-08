use anyhow::Result;
use bedrockci::download::server::{ServerDownloadError, download_server, get_latest_version};
use bedrockci::server_path::get_server_path;

pub async fn handle_download(
    version: Option<String>,
    accepted_eula_and_privacy_policy: bool,
    force_reinstall: bool,
) -> Result<()> {
    let path = get_server_path(true)?;
    let version = match version {
        Some(v) => v,
        None => {
            let latest_version = get_latest_version().await?;
            println!("No version specified, using latest version: {}", latest_version);
            latest_version
        }
    };

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
        Err(ServerDownloadError::ServerAlreadyInstalled(v)) => {
            println!("Server {} already installed, use --force-reinstall to download again", v);
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Error downloading server: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
