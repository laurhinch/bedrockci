use anyhow::Result;
use bedrockci_lib::server::list_servers;

pub async fn handle_list_servers() -> Result<()> {
    let versions = list_servers()?;

    println!("Downloaded server versions:");
    for version in versions {
        println!("{}", version);
    }

    Ok(())
}
