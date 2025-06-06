use dirs::home_dir;
use std::path::PathBuf;

/// Gets the server path, either the ENV variable BEDROCK_SERVER_PATH or the default path
pub fn get_server_path() -> Result<PathBuf, anyhow::Error> {
    let server_path = match std::env::var("BEDROCK_SERVER_PATH") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            let home = home_dir().ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not find home directory, and no BEDROCK_SERVER_PATH environment variable set"
                )
            })?;
            home.join(".bedrockci/server")
        }
    };

    if !server_path.exists() {
        return Err(anyhow::anyhow!(
            "Server directory does not exist at {}. Please run 'bedrockci download' first.",
            server_path.display()
        ));
    }

    Ok(server_path)
}
