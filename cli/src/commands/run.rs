use anyhow::Result;
use bedrockci::server::list_servers;
use bedrockci::server_path::get_server_path;
use bedrockci::validate::symlink_test_packs;
use colored::*;
use std::path::Path;
use std::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::signal;

/// Handles the run command for spinning up a Bedrock server with specified packs.
///
/// This command sets up symlinks for the provided behavior and resource packs,
/// then starts a Bedrock server that will keep running until manually stopped.
/// Unlike the validate command, this is designed for interactive testing and
/// development workflows.
///
/// # Arguments
///
/// * `resource_pack` - Path to the resource pack directory
/// * `behavior_pack` - Path to the behavior pack directory
/// * `version` - Optional server version to use (defaults to latest installed)
/// * `verbose` - Whether to show verbose server output
///
/// # Returns
///
/// * `Ok(())` - If the server started and ran successfully
/// * `Err(anyhow::Error)` - If there was an error during setup or execution
pub async fn handle_run(
    resource_pack: String,
    behavior_pack: String,
    version: Option<String>,
    verbose: bool,
) -> Result<()> {
    let resource_path = Path::new(&resource_pack);
    let behavior_path = Path::new(&behavior_pack);

    // Validate pack paths exist and are directories
    if !resource_path.exists() {
        anyhow::bail!("Resource pack not found at: {}", resource_pack);
    }
    if !resource_path.is_dir() {
        anyhow::bail!("Resource pack path is not a directory: {}", resource_pack);
    }
    if !behavior_path.exists() {
        anyhow::bail!("Behavior pack not found at: {}", behavior_pack);
    }
    if !behavior_path.is_dir() {
        anyhow::bail!("Behavior pack path is not a directory: {}", behavior_pack);
    }

    let version = match version {
        Some(v) => v,
        None => {
            let versions = list_servers()?;
            if versions.is_empty() {
                anyhow::bail!(
                    "No server versions found. Please download a server version first using: bedrockci download"
                );
            }
            println!(
                "No version specified, using latest: {}",
                versions.last().unwrap()
            );
            versions.last().unwrap().clone()
        }
    };

    // Get server path from environment or use the specified version
    let server_path = get_server_path(false)?.join(&version);

    if !server_path.exists() {
        anyhow::bail!(
            "Server version {} not found. Please download it first using: bedrockci download --version {}",
            version,
            version
        );
    }

    println!(
        "{}",
        format!("Using server version: {}", version).cyan().bold()
    );

    println!("{}", "Symlinking test packs to server directory...".cyan());
    symlink_test_packs(&server_path, behavior_path, resource_path)?;
    println!("{}", "Packs successfully linked to server".green());

    println!("{}", "Starting server...".cyan());
    start_and_run_server(&server_path, verbose).await?;

    Ok(())
}

/// Starts and runs the Bedrock server, monitoring its output until interrupted.
///
/// This function handles the server lifecycle including:
/// - Making the server executable
/// - Starting the server process
/// - Monitoring stdout/stderr output
/// - Graceful shutdown on Ctrl+C
///
/// # Arguments
///
/// * `server_path` - Path to the server directory
/// * `verbose` - Whether to show all server output or just important messages
///
/// # Returns
///
/// * `Ok(())` - If the server ran and was stopped successfully
/// * `Err(anyhow::Error)` - If there was an error during server execution
async fn start_and_run_server(server_path: &Path, verbose: bool) -> Result<()> {
    let server_exe = server_path.join("bedrock_server");
    if !server_exe.exists() {
        anyhow::bail!(
            "bedrock_server executable not found in {}",
            server_path.display()
        );
    }

    println!("{}", "Making server executable...".cyan());
    Command::new("chmod")
        .arg("+x")
        .arg(&server_exe)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to make server executable: {}", e))?;

    println!("{}", "Starting server process...".cyan());
    let mut child = TokioCommand::new(&server_exe)
        .current_dir(server_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start server process: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture server stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture server stderr"))?;

    println!();
    println!(
        "{}",
        "Server is running! Press Ctrl+C to stop.".green().bold()
    );
    println!("{}", "Monitoring server output...".cyan());
    if !verbose {
        println!("{}", "Use --verbose to see all server output".dimmed());
    }
    println!();

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    let mut server_started = false;

    loop {
        tokio::select! {
            Ok(Some(line)) = stdout_reader.next_line() => {
                let line = line.trim();
                if !line.is_empty() {
                    process_server_line(line, &mut server_started, verbose);
                }
            }
            Ok(Some(line)) = stderr_reader.next_line() => {
                let line = line.trim();
                if !line.is_empty() {
                    process_server_line(line, &mut server_started, verbose);
                }
            }
            _ = signal::ctrl_c() => {
                println!("\n{}", "Received Ctrl+C, stopping server...".yellow());
                break;
            }
            else => break
        }
    }

    println!("{}", "Stopping server...".cyan());
    child
        .kill()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to kill server process: {}", e))?;
    child
        .wait()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to wait for server to stop: {}", e))?;
    println!("{}", "Server stopped successfully.".green());

    Ok(())
}

/// Processes a single line of server output and displays it appropriately.
///
/// This function categorizes server output and applies appropriate formatting
/// and filtering based on the verbose setting and message importance.
///
/// # Arguments
///
/// * `line` - The raw output line from the server
/// * `server_started` - Mutable reference to track if server has fully started
/// * `verbose` - Whether to show all output or just important messages
fn process_server_line(line: &str, server_started: &mut bool, verbose: bool) {
    // Check if server has started
    if line.contains("Server started.") {
        *server_started = true;
        println!(
            "{}",
            "Server has started successfully! Ready for connections."
                .green()
                .bold()
        );
        return;
    }

    // Show important startup messages even in non-verbose mode
    if !*server_started {
        if line.contains("Starting Server")
            || line.contains("IPv4 supported")
            || line.contains("IPv6 supported")
            || line.contains("Level Name:")
            || line.contains("Game mode:")
            || line.contains("Difficulty:")
            || line.contains("opening worlds")
        {
            println!("{}", format!("{}", line).blue());
            return;
        }
    }

    // In verbose mode, show all output
    if verbose {
        if line.contains("ERROR") {
            println!("{}", format!("{}", line).red());
        } else if line.contains("WARN") {
            println!("{}", format!("{}", line).yellow());
        } else if line.contains("INFO") {
            println!("{}", format!("{}", line).blue());
        } else {
            println!("{}", format!("{}", line).dimmed());
        }
    } else {
        // In non-verbose mode, only show errors, warnings, and important info
        if line.contains("ERROR") {
            println!("{}", format!("{}", line).red());
        } else if line.contains("WARN") {
            println!("{}", format!("{}", line).yellow());
        } else if line.contains("Player connected:")
            || line.contains("Player disconnected:")
            || line.contains("Player Spawned:")
            || line.contains("[Chat]")
            || (line.contains("INFO")
                && (line.contains("Running AutoCompaction")
                    || line.contains("AutoCompaction took")
                    || line.contains("Saving...")
                    || line.contains("Changes to the level are resumed")))
        {
            println!("{}", format!("{}", line).blue());
        }
    }
}
