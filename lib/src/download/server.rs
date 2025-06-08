use futures::StreamExt;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use zip::ZipArchive;
use regex::Regex;

#[derive(Debug, thiserror::Error)]
pub enum ServerDownloadError {
    #[error("EULA and Privacy Policy not accepted")]
    EulaAndPrivacyPolicyNotAccepted,
    #[error("Failed to download server: {0}")]
    DownloadFailed(String),
    #[error("Failed to read server zip: {0}")]
    ZipReadFailed(String),
    #[error("Failed to create temporary file: {0}")]
    TempFileCreationFailed(String),
    #[error("Failed to extract server files: {0}")]
    ExtractionFailed(String),
    #[error("Invalid download path: {0}")]
    InvalidPath(String),
    #[error("Server version {0} already installed")]
    ServerAlreadyInstalled(String),
}

const EULA_NOT_ACCEPTED_TEXT: &str = r#"
By proceeding, you agree to the Minecraft End User License Agreement:
https://minecraft.net/eula
and the Privacy Policy:
https://go.microsoft.com/fwlink/?LinkId=521839

If you do not agree, you must not use this software.
"#;

/// Downloads the Bedrock Dedicated Server from the Minecraft website.
///
/// # Arguments
///
/// * `version` - The version of the Bedrock Dedicated Server to download.
/// * `download_path` - The base path to download the server to. The server will be installed in a subdirectory named after the version.
/// * `accepted_eula_and_privacy_policy` - Whether the EULA and Privacy Policy have been accepted. Must be true to download the server.
/// * `force_reinstall` - Whether to force reinstallation if the server is already installed.
///
/// # Returns
///
/// * `Ok(())` - If the server was downloaded successfully.
/// * `Err(ServerDownloadError)` - If the server was not downloaded successfully.
pub async fn download_server(
    version: &str,
    download_path: PathBuf,
    accepted_eula_and_privacy_policy: bool,
    force_reinstall: bool,
) -> Result<(), ServerDownloadError> {
    if !accepted_eula_and_privacy_policy {
        println!("{}", EULA_NOT_ACCEPTED_TEXT);
        return Err(ServerDownloadError::EulaAndPrivacyPolicyNotAccepted);
    }

    // Create version-specific directory
    let version_path = download_path.join(version);

    if !force_reinstall {
        if version_path.exists() {
            return Err(ServerDownloadError::ServerAlreadyInstalled(
                version.to_string(),
            ));
        }
    }

    // Validate download path
    if !download_path.exists() {
        std::fs::create_dir_all(&download_path).map_err(|e| {
            ServerDownloadError::InvalidPath(format!("Failed to create directory: {}", e))
        })?;
    }
    if !download_path.is_dir() {
        return Err(ServerDownloadError::InvalidPath(
            "Path must be a directory".to_string(),
        ));
    }

    println!("Downloading Bedrock Server version {}...", version);
    let download_url = get_download_url(version);

    // Download with progress feedback
    let response = reqwest::get(&download_url).await.map_err(|e| {
        ServerDownloadError::DownloadFailed(format!("Failed to connect to server: {}", e))
    })?;

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded = 0;
    let mut content = Vec::new();

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            ServerDownloadError::DownloadFailed(format!("Failed to read chunk: {}", e))
        })?;
        content.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let percentage = (downloaded as f64 / total_size as f64 * 100.0) as u32;
            print!("\rDownloading: {}%", percentage);
            std::io::stdout().flush().ok();
        }
    }
    println!("\nDownload complete!");

    println!("Extracting server files...");
    // Create a temporary file for the zip
    let temp_zip = tempfile::NamedTempFile::new()
        .map_err(|e| ServerDownloadError::TempFileCreationFailed(e.to_string()))?;

    temp_zip.as_file().write_all(&content).map_err(|e| {
        ServerDownloadError::ZipReadFailed(format!("Failed to write zip file: {}", e))
    })?;

    // Extract the zip file
    let mut archive = ZipArchive::new(temp_zip.as_file()).map_err(|e| {
        ServerDownloadError::ExtractionFailed(format!("Failed to open zip archive: {}", e))
    })?;

    let total_files = archive.len();
    for i in 0..total_files {
        if let Ok(mut file) = archive.by_index(i) {
            let outpath = version_path.join(file.name());
            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath).map_err(|e| {
                    ServerDownloadError::ExtractionFailed(format!(
                        "Failed to create directory: {}",
                        e
                    ))
                })?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        ServerDownloadError::ExtractionFailed(format!(
                            "Failed to create parent directory: {}",
                            e
                        ))
                    })?;
                }
                let mut outfile = File::create(&outpath).map_err(|e| {
                    ServerDownloadError::ExtractionFailed(format!("Failed to create file: {}", e))
                })?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| {
                    ServerDownloadError::ExtractionFailed(format!("Failed to extract file: {}", e))
                })?;
            }
            print!("\rExtracting: {}/{} files", i + 1, total_files);
            std::io::stdout().flush().ok();
        }
    }
    println!("\nExtraction complete!");

    Ok(())
}

fn get_download_url(version: &str) -> String {
    format!(
        "https://www.minecraft.net/bedrockdedicatedserver/bin-linux/bedrock-server-{}.zip",
        version
    )
}

/// Gets the latest version of the Bedrock Dedicated Server by parsing the Minecraft download page.
///
/// # Returns
///
/// * `Ok(String)` - The latest version string if successful
/// * `Err(ServerDownloadError)` - If the version could not be retrieved
pub async fn get_latest_version() -> Result<String, ServerDownloadError> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.33 (KHTML, like Gecko) Chrome/90.0.0.0 Safari/537.33")
        .build()
        .map_err(|e| ServerDownloadError::DownloadFailed(format!("Failed to create HTTP client: {}", e)))?;

    let response = client
        .get("https://minecraft.net/en-us/download/server/bedrock/")
        .header("Accept-Encoding", "identity")
        .header("Accept-Language", "en")
        .send()
        .await
        .map_err(|e| ServerDownloadError::DownloadFailed(format!("Failed to fetch download page: {}", e)))?;

    let html = response
        .text()
        .await
        .map_err(|e| ServerDownloadError::DownloadFailed(format!("Failed to read response: {}", e)))?;

    let re = Regex::new(r"https://www\.minecraft\.net/bedrockdedicatedserver/bin-linux/bedrock-server-([\d\.]+)\.zip")
        .map_err(|e| ServerDownloadError::DownloadFailed(format!("Failed to create regex: {}", e)))?;

    if let Some(captures) = re.captures(&html) {
        if let Some(version) = captures.get(1) {
            return Ok(version.as_str().to_string());
        }
    }

    Err(ServerDownloadError::DownloadFailed("Could not find version in download page".to_string()))
}
