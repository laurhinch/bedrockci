use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::select;
use tokio::time::sleep;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Failed to copy pack: {0}")]
    PackCopyFailed(String),
    #[error("Invalid pack path: {0}")]
    InvalidPackPath(String),
    #[error("Invalid server path: {0}")]
    InvalidServerPath(String),
    #[error("Failed to start server: {0}")]
    ServerStartFailed(String),
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

#[derive(Debug)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

const TESTING_BP_NAME: &str = "TESTING_PACK_BP";
const TESTING_RP_NAME: &str = "TESTING_PACK_RP";

#[derive(Debug, Deserialize)]
struct Manifest {
    header: Header,
}

#[derive(Debug, Deserialize)]
struct Header {
    uuid: String,
    version: Vec<u32>,
}

#[derive(Debug, Serialize)]
struct WorldPack {
    pack_id: String,
    version: Vec<u32>,
}

/// Reads a pack's manifest file and returns its header information.
///
/// # Arguments
///
/// * `pack_path` - Path to the pack directory
///
/// # Returns
///
/// * `Ok(Header)` - The pack's header information
/// * `Err(ValidationError)` - If there was an error reading the manifest
fn read_pack_manifest(pack_path: &Path) -> Result<Header, ValidationError> {
    let manifest_path = pack_path.join("manifest.json");
    if !manifest_path.exists() {
        return Err(ValidationError::InvalidPackPath(
            "manifest.json not found".to_string(),
        ));
    }

    let manifest_content = fs::read_to_string(manifest_path).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to read manifest.json: {}", e))
    })?;

    let manifest: Manifest = serde_json::from_str(&manifest_content).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to parse manifest.json: {}", e))
    })?;

    Ok(manifest.header)
}

/// Creates world pack configuration files for the server.
///
/// # Arguments
///
/// * `server_path` - Path to the server directory
/// * `bp_header` - Behavior pack header information
/// * `rp_header` - Resource pack header information
///
/// # Returns
///
/// * `Ok(())` - If the configuration files were created successfully
/// * `Err(ValidationError)` - If there was an error creating the files
fn create_world_pack_configs(
    server_path: &Path,
    bp_header: Header,
    rp_header: Header,
) -> Result<(), ValidationError> {
    let world_path = server_path.join("worlds/Bedrock level");
    fs::create_dir_all(&world_path).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to create world directory: {}", e))
    })?;

    // Create behavior pack config
    let bp_config = vec![WorldPack {
        pack_id: bp_header.uuid,
        version: bp_header.version,
    }];
    let bp_config_path = world_path.join("world_behavior_packs.json");
    fs::write(
        &bp_config_path,
        serde_json::to_string_pretty(&bp_config).map_err(|e| {
            ValidationError::PackCopyFailed(format!("Failed to serialize BP config: {}", e))
        })?,
    )
    .map_err(|e| ValidationError::PackCopyFailed(format!("Failed to write BP config: {}", e)))?;

    // Create resource pack config
    let rp_config = vec![WorldPack {
        pack_id: rp_header.uuid,
        version: rp_header.version,
    }];
    let rp_config_path = world_path.join("world_resource_packs.json");
    fs::write(
        &rp_config_path,
        serde_json::to_string_pretty(&rp_config).map_err(|e| {
            ValidationError::PackCopyFailed(format!("Failed to serialize RP config: {}", e))
        })?,
    )
    .map_err(|e| ValidationError::PackCopyFailed(format!("Failed to write RP config: {}", e)))?;

    Ok(())
}

/// Copies behavior and resource packs to the server directory, clearing any existing test packs first.
///
/// # Arguments
///
/// * `server_path` - Path to the server directory
/// * `bp_path` - Path to the behavior pack
/// * `rp_path` - Path to the resource pack
///
/// # Returns
///
/// * `Ok(())` - If the packs were copied successfully
/// * `Err(ValidationError)` - If there was an error copying the packs
pub fn copy_test_packs(
    server_path: &Path,
    bp_path: &Path,
    rp_path: &Path,
) -> Result<(), ValidationError> {
    // Validate paths
    if !server_path.exists() || !server_path.is_dir() {
        return Err(ValidationError::InvalidServerPath(
            "Server path does not exist or is not a directory".to_string(),
        ));
    }
    if !bp_path.exists() || !bp_path.is_dir() {
        return Err(ValidationError::InvalidPackPath(
            "Behavior pack path does not exist or is not a directory".to_string(),
        ));
    }
    if !rp_path.exists() || !rp_path.is_dir() {
        return Err(ValidationError::InvalidPackPath(
            "Resource pack path does not exist or is not a directory".to_string(),
        ));
    }

    // Read pack manifests
    let bp_header = read_pack_manifest(bp_path)?;
    let rp_header = read_pack_manifest(rp_path)?;

    // Setup pack directories
    let bp_dir = server_path.join("behavior_packs").join(TESTING_BP_NAME);
    let rp_dir = server_path.join("resource_packs").join(TESTING_RP_NAME);

    // Clear existing test packs
    if bp_dir.exists() {
        fs::remove_dir_all(&bp_dir).map_err(|e| {
            ValidationError::PackCopyFailed(format!("Failed to remove existing BP: {}", e))
        })?;
    }
    if rp_dir.exists() {
        fs::remove_dir_all(&rp_dir).map_err(|e| {
            ValidationError::PackCopyFailed(format!("Failed to remove existing RP: {}", e))
        })?;
    }

    // Copy new packs
    fs::create_dir_all(bp_dir.parent().unwrap()).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to create BP directory: {}", e))
    })?;
    fs::create_dir_all(rp_dir.parent().unwrap()).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to create RP directory: {}", e))
    })?;

    copy_dir(bp_path, &bp_dir)?;
    copy_dir(rp_path, &rp_dir)?;

    // Create world pack configurations
    create_world_pack_configs(server_path, bp_header, rp_header)?;

    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), ValidationError> {
    fs::create_dir_all(dst).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to create destination directory: {}", e))
    })?;

    for entry in fs::read_dir(src).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to read source directory: {}", e))
    })? {
        let entry = entry.map_err(|e| {
            ValidationError::PackCopyFailed(format!("Failed to read directory entry: {}", e))
        })?;
        let path = entry.path();
        let target = dst.join(path.file_name().unwrap());

        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|e| {
                ValidationError::PackCopyFailed(format!("Failed to copy file: {}", e))
            })?;
        }
    }

    Ok(())
}

/// Creates symlinks to behavior and resource packs in the server directory, removing any existing test packs first.
///
/// # Arguments
///
/// * `server_path` - Path to the server directory
/// * `bp_path` - Path to the behavior pack
/// * `rp_path` - Path to the resource pack
///
/// # Returns
///
/// * `Ok(())` - If the symlinks were created successfully
/// * `Err(ValidationError)` - If there was an error creating the symlinks
pub fn symlink_test_packs(
    server_path: &Path,
    bp_path: &Path,
    rp_path: &Path,
) -> Result<(), ValidationError> {
    // Validate paths
    if !server_path.exists() || !server_path.is_dir() {
        return Err(ValidationError::InvalidServerPath(
            "Server path does not exist or is not a directory".to_string(),
        ));
    }
    if !bp_path.exists() || !bp_path.is_dir() {
        return Err(ValidationError::InvalidPackPath(
            "Behavior pack path does not exist or is not a directory".to_string(),
        ));
    }
    if !rp_path.exists() || !rp_path.is_dir() {
        return Err(ValidationError::InvalidPackPath(
            "Resource pack path does not exist or is not a directory".to_string(),
        ));
    }

    // Read pack manifests
    let bp_header = read_pack_manifest(bp_path)?;
    let rp_header = read_pack_manifest(rp_path)?;

    // Setup pack directories
    let bp_dir = server_path.join("behavior_packs").join(TESTING_BP_NAME);
    let rp_dir = server_path.join("resource_packs").join(TESTING_RP_NAME);

    let cleanup_path = |path: &Path| -> Result<(), ValidationError> {
        match fs::metadata(path) {
            Ok(_) => {
                if let Err(e) = fs::remove_file(path) {
                    fs::remove_dir_all(path).map_err(|e| {
                        ValidationError::PackCopyFailed(format!("Failed to remove existing path: {}", e))
                    })?;
                }
            }
            Err(_) => {}
        }
        
        Ok(())
    };

    cleanup_path(&bp_dir)?;
    cleanup_path(&rp_dir)?;

    // Create parent directories if they don't exist
    fs::create_dir_all(bp_dir.parent().unwrap()).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to create BP directory: {}", e))
    })?;
    fs::create_dir_all(rp_dir.parent().unwrap()).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to create RP directory: {}", e))
    })?;

    // Create symlinks using absolute paths to avoid any relative path issues
    let bp_abs = fs::canonicalize(bp_path).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to get absolute BP path: {}", e))
    })?;
    let rp_abs = fs::canonicalize(rp_path).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to get absolute RP path: {}", e))
    })?;

    std::os::unix::fs::symlink(&bp_abs, &bp_dir).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to create BP symlink: {}", e))
    })?;

    std::os::unix::fs::symlink(&rp_abs, &rp_dir).map_err(|e| {
        ValidationError::PackCopyFailed(format!("Failed to create RP symlink: {}", e))
    })?;

    // Create world pack configurations
    create_world_pack_configs(server_path, bp_header, rp_header)?;

    Ok(())
}

/// Starts the Bedrock server from the specified directory and monitors its output.
///
/// # Arguments
///
/// * `server_path` - Path to the server directory containing bedrock_server
///
/// # Returns
///
/// * `Ok(ValidationResult)` - The validation results from the server output
/// * `Err(ValidationError)` - If there was an error starting or monitoring the server
pub async fn start_server(server_path: &Path) -> Result<ValidationResult, ValidationError> {
    if !server_path.exists() || !server_path.is_dir() {
        return Err(ValidationError::InvalidServerPath(
            "Server path does not exist or is not a directory".to_string(),
        ));
    }

    let server_exe = server_path.join("bedrock_server");
    if !server_exe.exists() {
        return Err(ValidationError::InvalidServerPath(
            "bedrock_server executable not found".to_string(),
        ));
    }

    println!("{}", "Making server executable...".cyan());
    Command::new("chmod")
        .arg("+x")
        .arg(&server_exe)
        .output()
        .map_err(|e| {
            ValidationError::ServerStartFailed(format!("Failed to chmod server: {}", e))
        })?;

    println!("{}", "Starting server process...".cyan());
    let mut child = TokioCommand::new(&server_exe)
        .current_dir(server_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            ValidationError::ServerStartFailed(format!("Failed to start server: {}", e))
        })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        ValidationError::ServerStartFailed("Failed to capture server stdout".to_string())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        ValidationError::ServerStartFailed("Failed to capture server stderr".to_string())
    })?;

    let mut validation_result = ValidationResult {
        errors: Vec::new(),
        warnings: Vec::new(),
        info: Vec::new(),
    };

    println!("{}", "Monitoring server output...".cyan());
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    let mut last_log_time = Instant::now();
    let mut telemetry_seen = false;
    let mut server_started = false;
    let mut telemetry_complete = false;

    loop {
        let timeout_future = if telemetry_complete {
            Box::pin(sleep(Duration::from_secs(5)))
        } else {
            Box::pin(sleep(Duration::from_secs(0)))
        };

        select! {
            Ok(Some(line)) = stdout_reader.next_line() => {
                let line = line.trim();
                if !line.is_empty() {
                    process_line(line, &mut validation_result, &mut last_log_time, &mut telemetry_seen, &mut server_started, &mut telemetry_complete)?;
                }
            }
            Ok(Some(line)) = stderr_reader.next_line() => {
                let line = line.trim();
                if !line.is_empty() {
                    process_line(line, &mut validation_result, &mut last_log_time, &mut telemetry_seen, &mut server_started, &mut telemetry_complete)?;
                }
            }
            _ = timeout_future => {
                if telemetry_complete {
                    println!("{}", "\nNo new logs for 5 seconds, stopping server...".yellow());
                    break;
                }
            }
            else => break
        }
    }

    println!("{}", "Stopping server...".cyan());
    child
        .kill()
        .await
        .map_err(|e| ValidationError::ServerStartFailed(format!("Failed to stop server: {}", e)))?;
    child.wait().await.map_err(|e| {
        ValidationError::ServerStartFailed(format!("Failed to wait for server to stop: {}", e))
    })?;
    println!("{}", "Server stopped.".green());

    Ok(validation_result)
}

fn process_line(
    line: &str,
    validation_result: &mut ValidationResult,
    last_log_time: &mut Instant,
    telemetry_seen: &mut bool,
    server_started: &mut bool,
    telemetry_complete: &mut bool,
) -> Result<(), ValidationError> {
    // Check if server has started
    if line.contains("Server started.") {
        *server_started = true;
        println!("{}", "Server has started successfully".green());
        return Ok(());
    }

    // Check if we've seen the telemetry message
    if line.contains("TELEMETRY MESSAGE") {
        *telemetry_seen = true;
        println!("{}", "Starting validation...".cyan());
        *last_log_time = Instant::now();
        return Ok(());
    }

    // Skip all logs between telemetry message and separator
    if *telemetry_seen && line.contains("======================================================") {
        *telemetry_seen = false;
        *telemetry_complete = true;
        *last_log_time = Instant::now();
        return Ok(());
    }

    // Skip all logs before telemetry block is complete
    if !*server_started || *telemetry_seen {
        return Ok(());
    }

    // Update last log time for any log message
    if line.contains("ERROR") || line.contains("WARN") || line.contains("INFO") {
        *last_log_time = Instant::now();
    }

    // Categorize and print the log message
    if line.contains("ERROR") {
        validation_result.errors.push(line.to_string());
    } else if line.contains("WARN") {
        validation_result.warnings.push(line.to_string());
    } else if line.contains("INFO") {
        validation_result.info.push(line.to_string());
        println!("{}", format!("{}", line).blue());
    }

    Ok(())
}
