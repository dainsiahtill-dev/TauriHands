// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;

#[cfg(feature = "cli")]
use clap::Parser;

#[cfg(feature = "cli")]
use env_logger;

#[cfg(feature = "cli")]
use log;

#[cfg(feature = "cli")]
use tauri_app_lib::cli::main::run_cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Check if we're running in CLI mode
    if std::env::args().any(|arg| arg == "--cli" || arg == "-c") {
        #[cfg(feature = "cli")]
        {
            return run_cli().await;
        }
        
        #[cfg(not(feature = "cli"))]
        {
            eprintln!("CLI support is not enabled. Please compile with --features cli");
            std::process::exit(1);
        }
    }

    // Default to GUI mode
    tauri_app_lib::run();
    Ok(())
}
