use anyhow::Result;
use bedrockci::server::list_servers;
use bedrockci::server_path::get_server_path;
use bedrockci::validate::{start_server, symlink_test_packs, ValidationResult};
use colored::*;
use std::path::Path;

pub async fn handle_validate(
    resource_pack: String,
    behavior_pack: String,
    only_warn: bool,
    fail_on_warn: bool,
    version: Option<String>,
) -> Result<()> {
    let resource_path = Path::new(&resource_pack);
    let behavior_path = Path::new(&behavior_pack);

    if !resource_path.exists() {
        anyhow::bail!("Resource pack not found at: {}", resource_pack);
    }
    if !behavior_path.exists() {
        anyhow::bail!("Behavior pack not found at: {}", behavior_pack);
    }

    let version = match version {
        Some(v) => v,
        None => {
            let versions = list_servers()?;
            if versions.is_empty() {
                anyhow::bail!("No server versions found. Please download a server version first.");
            }
            versions.last().unwrap().clone()
        }
    };

    // Get server path from environment or use the specified version
    let server_path = get_server_path(false)?.join(&version);

    if !server_path.exists() {
        anyhow::bail!(
            "Server version {} not found. Please download it first.",
            version
        );
    }

    println!("{}", format!("Using server version: {}", version).cyan().bold());

    println!("{}", "Symlinking test packs to server directory...".cyan());
    symlink_test_packs(&server_path, behavior_path, resource_path)?;

    println!("{}", "Starting server for validation...".cyan());
    let validation_result = start_server(&server_path).await?;

    handle_validation_results(&validation_result, only_warn, fail_on_warn)
}

fn handle_validation_results(
    validation_result: &ValidationResult,
    only_warn: bool,
    fail_on_warn: bool,
) -> Result<()> {
    println!("{}", "Validation Results:".cyan().bold());

    if !validation_result.errors.is_empty() {
        println!("\n{}", "Errors:".red().bold());
        for error in &validation_result.errors {
            println!("  {}", error.red());
        }
    }

    if !validation_result.warnings.is_empty() {
        println!("\n{}", "Warnings:".yellow().bold());
        for warning in &validation_result.warnings {
            println!("  {}", warning.yellow());
        }
    }

    let errors = validation_result.errors.len();
    let warnings = validation_result.warnings.len();

    // Always print a summary message
    let summary = if errors == 0 && warnings == 0 {
        "Validation completed successfully with no errors or warnings".green()
    } else if only_warn {
        format!("Validation completed with {} errors and {} warnings", errors, warnings).yellow()
    } else if fail_on_warn {
        format!("Validation completed with {} errors and {} warnings (fail on warn mode)", errors, warnings).yellow()
    } else {
        format!("Validation completed with {} errors and {} warnings", errors, warnings).yellow()
    };
    println!("\n{}", summary);

    if only_warn {
        // Most lenient: Treat errors as warnings
        Ok(())
    } else if fail_on_warn {
        // Strictest: Fail on both errors and warnings
        if errors > 0 || warnings > 0 {
            Err(anyhow::anyhow!(
                "Validation failed with {} errors and {} warnings (fail on warn mode)",
                errors,
                warnings
            ))
        } else {
            Ok(())
        }
    } else {
        // Normal: Fail on errors only
        if errors > 0 {
            Err(anyhow::anyhow!("Validation failed with {} errors", errors))
        } else {
            Ok(())
        }
    }
}
