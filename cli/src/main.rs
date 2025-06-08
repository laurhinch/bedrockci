use anyhow::Result;
use bedrockci;
use clap::{Arg, ArgAction, Command, command};

mod commands;

#[cfg(not(target_os = "linux"))]
compile_error!("This CLI only supports Linux");

fn format_version_message() -> &'static str {
    const VERSION_MESSAGE: &str = concat!(" v", env!("CARGO_PKG_VERSION"));
    VERSION_MESSAGE
}

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        // Check if running on Ubuntu
        bedrockci::check_ubuntu();
    }

    let version_message = format_version_message();
    let matches = command!()
        .name("bedrockci")
        .version(version_message)
        .about("BedrockCI CLI")
        .author("Lauren 'Yharna' Hinchcliffe <lauren@yarugames.com>")
        .display_name("BedrockCI")
        // Download command
        .subcommand(
            Command::new("download")
                .display_name("Download")
                .about("Download Minecraft Bedrock server")
                .long_about("Downloads a specific version of the Minecraft Bedrock server")
                .arg(
                    Arg::new("accept-eula")
                        .long("accept-eula")
                        .help("Accept the Minecraft EULA")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("version")
                        .long("version")
                        .short('v')
                        .help("Specific version to download (e.g., \"1.21.84.1\"). If not specified, the latest version will be used.")
                        .value_parser(clap::value_parser!(String)),
                )
                .arg(
                    Arg::new("force-reinstall")
                        .long("force-reinstall")
                        .help("Force reinstall the server, even if it already exists")
                        .action(ArgAction::SetTrue),
                ),
        )
        // List servers command
        .subcommand(
            Command::new("list")
                .display_name("List")
                .about("List downloaded server versions")
                .long_about("Lists all downloaded server versions"),
        )
        // Validate command
        .subcommand(
            Command::new("validate")
                .display_name("Validate")
                .about("Validate resource and behavior packs")
                .long_about("Validates the structure and contents of resource and behavior packs")
                .arg(
                    Arg::new("resource-pack")
                        .long("rp")
                        .help("Path to the resource pack")
                        .value_parser(clap::value_parser!(String))
                        .required(true),
                )
                .arg(
                    Arg::new("behavior-pack")
                        .long("bp")
                        .help("Path to the behavior pack")
                        .value_parser(clap::value_parser!(String))
                        .required(true),
                )
                .arg(
                    Arg::new("only-warn")
                        .long("only-warn")
                        .help("Only show warnings, don't fail CI on errors")
                        .conflicts_with("fail-on-warn")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("fail-on-warn")
                        .long("fail-on-warn")
                        .help("Fail CI on warnings")
                        .conflicts_with("only-warn")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("version")
                        .long("version")
                        .short('v')
                        .help("Specific server version to use for validation (e.g., \"1.21.84.1\"). If not specified, the latest version installed will be used.")
                        .value_parser(clap::value_parser!(String)),
                )
                .arg(
                    Arg::new("last-log-timeout")
                        .long("last-log-timeout")
                        .short('t')
                        .help("Timeout in seconds to wait after the last log message appears before wrapping up validation (default: 2)")
                        .value_parser(clap::value_parser!(u64)),
                )
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('l')
                        .help("Verbose output, print all output from the validation server")
                        .action(ArgAction::SetTrue),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("download", sub_matches)) => {
            let accept_eula = sub_matches.get_flag("accept-eula");
            let force_reinstall = sub_matches.get_flag("force-reinstall");
            let version = sub_matches
                .get_one::<String>("version")
                .map(|s| s.to_string());
            commands::download::handle_download(version, accept_eula, force_reinstall).await?;
        }
        Some(("validate", sub_matches)) => {
            let resource_pack = sub_matches
                .get_one::<String>("resource-pack")
                .unwrap()
                .to_string();
            let behavior_pack = sub_matches
                .get_one::<String>("behavior-pack")
                .unwrap()
                .to_string();
            let only_warn = sub_matches.get_flag("only-warn");
            let fail_on_warn = sub_matches.get_flag("fail-on-warn");
            let version = sub_matches
                .get_one::<String>("version")
                .map(|s| s.to_string());
            let last_log_timeout = sub_matches
                .get_one::<u64>("last-log-timeout")
                .map(|s| *s);
            let verbose = sub_matches.get_flag("verbose");
            commands::validate::handle_validate(
                resource_pack,
                behavior_pack,
                only_warn,
                fail_on_warn,
                version,
                last_log_timeout,
                verbose,
            )
            .await?;
        }
        Some(("list", _sub_matches)) => {
            commands::list_servers::handle_list_servers().await?;
        }
        _ => {
            println!("Please specify a valid subcommand. Use --help for more information.");
            std::process::exit(1);
        }
    }

    Ok(())
}
