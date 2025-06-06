/// Gets the server path, either the ENV variable BEDROCK_SERVER_PATH or the default path
pub fn get_server_path() -> PathBuf {
    let server_path = std::env::var("BEDROCK_SERVER_PATH").unwrap_or_else(|_| {
        let home = home_dir().ok_or_else(|| {
            anyhow::anyhow!(
                "Could not find home directory, and no BEDROCK_SERVER_PATH environment variable set"
            )
        })?;
        home.join(".bedrockci/server")
    });
    server_path
}
