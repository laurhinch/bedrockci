use crate::server_path::get_server_path;
use anyhow::Result;

/// Returns a list of downloaded server versions
pub fn list_servers() -> Result<Vec<String>> {
    let path = get_server_path()?;

    let versions = std::fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            entry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect::<Vec<String>>();

    Ok(versions)
}
