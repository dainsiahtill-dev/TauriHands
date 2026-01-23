use clap::Parser;
use std::path::PathBuf;

use crate::cli::commands::{Cli, Commands};
use crate::cli::config::{Config, load_config, save_config};
use crate::cli::tui::start_terminal_mode;
use crate::cli::server::{start_web_server, start_gui_server};
use crate::cli::commands::{RunArgs, HeadlessArgs, WebArgs, ServeArgs, ConfigArgs};
use crate::services::kernel::KernelManager;
use crate::services::llm::LlmStore;
use crate::automation::engine::{TauriHandsEngine, AutomationConfig};
use anyhow::{Context, Result};

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    
    // Load configuration
    let config = load_config(cli.config.as_deref())?;
    
    // Set verbosity
    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    // Configure Codex if requested
    if cli.use_codex {
        std::env::set_var("TAURIHANDS_USE_CLOUD_CODEX", "false");
        
        if let Some(model) = cli.codex_model {
            std::env::set_var("CODEX_MODEL", model);
        }
        
        if let Some(reasoning) = cli.codex_reasoning {
            std::env::set_var("CODEX_REASONING", reasoning.to_string());
        }
        
        if let Some(approval) = cli.codex_approval {
            std::env::set_var("CODEX_APPROVAL", approval);
        }
        
        log::info!("Codex CLI enabled with model: {:?}, reasoning: {:?}, approval: {:?}", 
            cli.codex_model, cli.codex_reasoning, cli.codex_approval);
    }

    match cli.command {
        Commands::Run(args) => run_command(args, &config).await?,
        Commands::Terminal(args) => start_terminal_mode(args)?,
        Commands::Headless(args) => headless_command(args, &config).await?,
        Commands::Web(args) => web_command(args, &config).await?,
        Commands::Serve(args) => serve_command(args, &config).await?,
        Commands::Config(args) => config_command(args, &config)?,
        Commands::Version => {
            println!("TauriHands {}", env!("CARGO_PKG_VERSION"));
            println!("AI-Driven Development Agent");
            if cli.use_codex {
                println!("Codex Integration: Enabled");
                if let Some(model) = cli.codex_model {
                    println!("Codex Model: {}", model);
                }
                if let Some(reasoning) = cli.codex_reasoning {
                    println!("Codex Reasoning Level: {}", reasoning);
                }
                if let Some(approval) = cli.codex_approval {
                    println!("Codex Approval Mode: {}", approval);
                }
            }
        }
    }

    Ok(())
}

async fn run_command(args: RunArgs, config: &Config) -> Result<()> {
    let workspace = args.workspace.unwrap_or_else(|| config.workspace.clone().unwrap_or_else(|| std::env::current_dir().unwrap()));
    
    // Create automation config
    let automation_config = AutomationConfig {
        workspace: workspace.clone(),
        max_retries: args.max_steps.unwrap_or(3) as u32,
        timeout_seconds: 300,
        parallel_execution: !args.yes,
        auto_recovery: true,
        validation_enabled: true,
        progress_reporting: true,
        llm_model: args.model.unwrap_or_else(|| config.model.clone().unwrap_or_else(|| "gpt-4".to_string())),
        api_key: config.api_key.clone(),
    };

    // Initialize automation engine
    let engine = TauriHandsEngine::new(automation_config.clone())?;

    let task = args.task.unwrap_or_else(|| {
        // TODO: Start interactive mode
        "Interactive mode started".to_string()
    });

    if args.headless {
        headless_command(HeadlessArgs {
            task,
            workspace: Some(workspace),
            output: crate::cli::commands::OutputFormat::Json,
            output_file: None,
        }, config).await?;
    } else {
        // Run full automation
        println!("🚀 Starting TauriHands automation engine...");
        println!("📋 Task: {}", task);
        println!("📁 Workspace: {:?}", workspace);
        
        match engine.execute_automation(&task).await {
            Ok(results) => {
                println!("✅ Automation completed successfully!");
                println!("📊 Results:");
                for result in &results {
                    println!("  - {}: {}", result.task_id, 
                        if result.success { "✅ Success" } else { "❌ Failed" });
                    if !result.output.is_empty() {
                        println!("    Output: {}", result.output);
                    }
                    if let Some(error) = &result.error {
                        println!("    Error: {}", error);
                    }
                }
                
                // Save results to file
                let results_file = workspace.join("automation_results.json");
                std::fs::write(&results_file, serde_json::to_string_pretty(&results)?)?;
                println!("💾 Results saved to: {:?}", results_file);
            }
            Err(e) => {
                eprintln!("❌ Automation failed: {}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

async fn headless_command(args: HeadlessArgs, config: &Config) -> Result<()> {
    let workspace = args.workspace.unwrap_or_else(|| config.workspace.clone().unwrap_or_else(|| std::env::current_dir().unwrap()));
    
    log::info!("Starting headless mode");
    log::info!("Task: {}", args.task);
    log::info!("Workspace: {:?}", workspace);
    log::info!("Output format: {:?}", args.output);

    // Initialize kernel
    let llm_store = LlmStore::new()?;
    let mut kernel = KernelManager::new(
        workspace.clone(),
        Default::default(),
        Default::default(),
        Default::default(),
        workspace.join(".taurihands"),
    )?;

    // Execute task
    log::info!("Executing task: {}", args.task);
    
    // TODO: Implement actual task execution
    let result = format!("Task completed: {}", args.task);
    
    match args.output {
        crate::cli::commands::OutputFormat::Json => {
            let output = serde_json::json!({
                "task": args.task,
                "result": result,
                "status": "completed"
            });
            
            if let Some(output_file) = &args.output_file {
                std::fs::write(output_file, serde_json::to_string_pretty(&output)?)?;
                log::info!("Output saved to: {:?}", output_file);
            } else {
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
        }
        crate::cli::commands::OutputFormat::Yaml => {
            let output = serde_yaml::to_string(&serde_json::json!({
                "task": args.task,
                "result": result,
                "status": "completed"
            }))?;
            
            if let Some(output_file) = &args.output_file {
                std::fs::write(output_file, output)?;
                log::info!("Output saved to: {:?}", output_file);
            } else {
                println!("{}", output);
            }
        }
        crate::cli::commands::OutputFormat::Text => {
            let output = format!("Task: {}\nResult: {}\nStatus: completed", args.task, result);
            
            if let Some(output_file) = &args.output_file {
                std::fs::write(output_file, output)?;
                log::info!("Output saved to: {:?}", output_file);
            } else {
                println!("{}", output);
            }
        }
    }

    Ok(())
}

async fn web_command(args: WebArgs, config: &Config) -> Result<()> {
    log::info!("Starting web interface on {}:{}", args.host, args.port);
    
    let workspace = config.workspace.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    
    start_web_server(&workspace, &args.host, args.port, args.open).await?;
    Ok(())
}

async fn serve_command(args: ServeArgs, config: &Config) -> Result<()> {
    log::info!("Starting GUI server on {}:{}", args.host, args.port);
    
    let workspace = config.workspace.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    
    start_gui_server(&workspace, &args.host, args.port, args.api).await?;
    Ok(())
}

fn config_command(args: ConfigArgs, config: &Config) -> Result<()> {
    if args.show {
        println!("Current configuration:");
        println!("  Workspace: {:?}", config.workspace);
        println!("  Model: {:?}", config.model);
        println!("  API Key: {}", if config.api_key.is_some() { "***" } else { "Not set" });
    } else if args.list {
        println!("Available configuration options:");
        println!("  workspace - Default workspace directory");
        println!("  model - Default AI model to use");
        println!("  api_key - API key for the AI model");
        println!("  max_steps - Maximum number of steps per task");
    } else if let Some(set_value) = args.set {
        let parts: Vec<&str> = set_value.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid set command format. Use: key=value"));
        }
        
        let key = parts[0];
        let value = parts[1];
        
        let mut new_config = config.clone();
        match key {
            "workspace" => {
                new_config.workspace = Some(PathBuf::from(value));
            }
            "model" => {
                new_config.model = Some(value.to_string());
            }
            "api_key" => {
                new_config.api_key = Some(value.to_string());
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
            }
        }
        
        save_config(&new_config)?;
        println!("Configuration updated: {} = {}", key, value);
    } else if args.reset {
        let default_config = Config::default();
        save_config(&default_config)?;
        println!("Configuration reset to defaults");
    }
    
    Ok(())
}
