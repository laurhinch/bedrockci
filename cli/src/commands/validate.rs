use anyhow::Result;
use bedrockci::server::list_servers;
use bedrockci::server_path::get_server_path;
use bedrockci::validate::{ValidationResult, copy_test_packs, start_server};
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

    println!("{}", "Using server version:".cyan().bold());
    println!("  {}", version.green());

    println!("{}", "Copying test packs to server directory...".cyan());
    copy_test_packs(&server_path, behavior_path, resource_path)?;

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

    if only_warn {
        // Most lenient: Treat errors as warnings
        println!(
            "\n{}",
            format!(
                "Validation  with {} errors and {} warnings",
                errors, warnings
            )
            .yellow()
        );
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
            println!("\n{}", "Validation completed successfully".green());
            Ok(())
        }
    } else {
        // Normal: Fail on errors only
        if errors > 0 {
            Err(anyhow::anyhow!("Validation failed with {} errors", errors))
        } else {
            println!("\n{}", "Validation completed successfully".green());
            Ok(())
        }
    }
}
