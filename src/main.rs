use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use env_logger::Env;
use indicatif::ProgressBar;
use log::{debug, info, warn};
use std::collections::HashSet;
use std::path::PathBuf;
use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

use conda_env_inspect::{
    advanced_analysis,
    cli::{Cli, Commands},
    interactive::{self, create_progress_bar},
    utils,
};
use conda_env_inspect::exporters::{self, ExportFormat};
use conda_env_inspect::models::EnvironmentAnalysis;

fn main() -> Result<()> {
    let start_time = Instant::now();
    
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();
    
    info!("Starting conda-env-inspect v{}", env!("CARGO_PKG_VERSION"));
    
    // Check if conda is available and log warning if not
    check_conda_availability();
    
    // Parse command line arguments
    let cli = Cli::parse();
    debug!("Parsed command-line arguments: {:?}", cli);

    // Create progress bar for long operations
    let pb = create_progress_bar(100, "Analyzing environment...");
    pb.set_position(0);

    // Handle commands
    match &cli.command {
        Some(Commands::Analyze {
            file,
            check_outdated,
            flag_pinned,
            generate_graph,
            generate_recommendations: _,
            graph_output,
            interactive,
            advanced_graph,
        }) => {
            info!("Analyzing environment file: {:?}", file);
            pb.set_position(10);
            
            let mut analysis = if *check_outdated {
                pb.set_message("Enriching package information...");
                utils::analyze_environment_parallel(file, *check_outdated, *flag_pinned)
                    .with_context(|| format!("Failed to analyze environment file: {:?}", file))?
            } else {
                utils::analyze_environment(file, *check_outdated, *flag_pinned)
                    .with_context(|| format!("Failed to analyze environment file: {:?}", file))?
            };
            
            pb.set_position(50);
            pb.set_message("Processing dependencies...");
            
            let advanced_deps = if *advanced_graph {
                Some(create_advanced_dependency_graph(&analysis, pb.clone())?)
            } else {
                None
            };
            
            pb.set_position(80);
            
            // Generate dependency graph if requested
            if *generate_graph {
                if let Some(graph_path) = graph_output {
                    info!("Generating dependency graph: {:?}", graph_path);
                    if let Err(e) = utils::generate_dependency_graph(file, graph_path) {
                        warn!("Failed to generate full dependency graph: {}", e);
                        println!("Note: Generated a basic dependency graph without all relationships. For complete dependency analysis, please run in an environment with conda installed.");
                    } else {
                        println!("Dependency graph saved to: {:?}", graph_path);
                    }
                } else {
                    warn!("No output path specified for dependency graph");
                    return Err(anyhow::anyhow!("No output path specified for dependency graph"));
                }
            }
            
            pb.set_position(90);
            
            // If interactive mode is enabled, launch the TUI
            if *interactive {
                pb.finish_and_clear();
                info!("Starting interactive UI");
                let mut ui = interactive::InteractiveUI::new(analysis, advanced_deps)?;
                ui.run()?;
            } else {
                pb.set_message("Exporting results...");
                exporters::export_analysis(&analysis, convert_format(cli.format), cli.output.as_ref())
                    .with_context(|| "Failed to export analysis")?;
                pb.finish_with_message("Analysis complete!");
            }
        }
        Some(Commands::Export { file, format, output }) => {
            info!("Exporting environment file: {:?}", file);
            pb.set_message("Analyzing environment...");
            
            let analysis = utils::analyze_environment(file, false, false)
                .with_context(|| format!("Failed to analyze environment file: {:?}", file))?;
            
            pb.set_position(80);
            pb.set_message("Exporting results...");
            
            info!("Exporting in format: {:?}", format);
            exporters::export_analysis(&analysis, convert_format(*format), output.as_ref())
                .with_context(|| "Failed to export analysis")?;
            
            pb.finish_with_message("Export complete!");
        }
        Some(Commands::Graph { file, output, advanced }) => {
            info!("Generating dependency graph for: {:?}", file);
            pb.set_message("Analyzing environment...");
            
            let analysis = utils::analyze_environment(file, false, false)
                .with_context(|| format!("Failed to analyze environment file: {:?}", file))?;
            
            pb.set_position(50);
            pb.set_message("Generating graph...");
            
            if *advanced {
                let advanced_deps = create_advanced_dependency_graph(&analysis, pb.clone())?;
                advanced_analysis::export_advanced_dependency_graph(&advanced_deps, output)
                    .with_context(|| "Failed to generate advanced dependency graph")?;
                println!("Advanced dependency graph saved to: {:?}", output);
            } else {
                if let Err(e) = utils::generate_dependency_graph(file, output) {
                    warn!("Failed to generate full dependency graph: {}", e);
                    println!("Note: Generated a basic dependency graph without all relationships. For complete dependency analysis, please run in an environment with conda installed.");
                } else {
                    println!("Dependency graph saved to: {:?}", output);
                }
            }
            
            pb.finish_with_message("Graph generation complete!");
        }
        Some(Commands::Recommend { file, check_outdated }) => {
            info!("Generating recommendations for: {:?}", file);
            pb.set_message("Analyzing environment...");
            
            let analysis = utils::analyze_environment(file, *check_outdated, true)
                .with_context(|| format!("Failed to analyze environment file: {:?}", file))?;
            
            pb.finish_and_clear();
            
            if analysis.recommendations.is_empty() {
                println!("No recommendations available for this environment.");
            } else {
                println!("Recommendations for environment: {:?}", file);
                for (i, rec) in analysis.recommendations.iter().enumerate() {
                    println!("{}. {}", i + 1, rec);
                }
            }
        }
        Some(Commands::Interactive { file, check_outdated, advanced_graph }) => {
            info!("Starting interactive analysis for: {:?}", file);
            pb.set_message("Analyzing environment...");
            
            let analysis = if *check_outdated {
                utils::analyze_environment_parallel(file, *check_outdated, true)
                    .with_context(|| format!("Failed to analyze environment file: {:?}", file))?
            } else {
                utils::analyze_environment(file, *check_outdated, true)
                    .with_context(|| format!("Failed to analyze environment file: {:?}", file))?
            };
            
            pb.set_position(60);
            pb.set_message("Processing dependencies...");
            
            let advanced_deps = if *advanced_graph {
                Some(create_advanced_dependency_graph(&analysis, pb.clone())?)
            } else {
                None
            };
            
            pb.finish_and_clear();
            
            info!("Starting interactive UI");
            let mut ui = interactive::InteractiveUI::new(analysis, advanced_deps)?;
            ui.run()?;
        }
        Some(Commands::Vulnerabilities { file }) => {
            info!("Checking for vulnerabilities in: {:?}", file);
            pb.set_message("Analyzing environment...");
            
            let analysis = utils::analyze_environment(file, true, false)
                .with_context(|| format!("Failed to analyze environment file: {:?}", file))?;
            
            pb.set_position(50);
            pb.set_message("Checking vulnerabilities...");
            
            let vulnerabilities = advanced_analysis::find_vulnerabilities(&analysis.packages);
            
            pb.finish_and_clear();
            
            if vulnerabilities.is_empty() {
                println!("No known vulnerabilities found in the environment.");
            } else {
                println!("Found {} potential security vulnerabilities:", vulnerabilities.len());
                for (i, (pkg, ver, desc)) in vulnerabilities.iter().enumerate() {
                    println!("{}. {} {} - {}", i + 1, pkg, ver, desc);
                }
            }
        }
        None => {
            // Default behavior when no subcommand is specified
            info!("Using default behavior for file: {:?}", cli.file);
            pb.set_message("Analyzing environment...");
            
            let analysis = if cli.check_outdated {
                pb.set_message("Enriching package information...");
                utils::analyze_environment_parallel(&cli.file, cli.check_outdated, cli.flag_pinned)
                    .with_context(|| format!("Failed to analyze environment file: {:?}", cli.file))?
            } else {
                utils::analyze_environment(&cli.file, cli.check_outdated, cli.flag_pinned)
                    .with_context(|| format!("Failed to analyze environment file: {:?}", cli.file))?
            };
            
            pb.set_position(50);
            
            // Generate dependency graph if requested
            if cli.generate_graph {
                pb.set_message("Generating dependency graph...");
                if let Some(graph_path) = &cli.graph_output {
                    info!("Generating dependency graph: {:?}", graph_path);
                    if let Err(e) = utils::generate_dependency_graph(&cli.file, graph_path) {
                        warn!("Failed to generate full dependency graph: {}", e);
                        println!("Note: Generated a basic dependency graph without all relationships. For complete dependency analysis, please run in an environment with conda installed.");
                    } else {
                        println!("Dependency graph saved to: {:?}", graph_path);
                    }
                } else {
                    warn!("No output path specified for dependency graph");
                    return Err(anyhow::anyhow!("No output path specified for dependency graph"));
                }
            }
            
            pb.set_position(80);
            pb.set_message("Exporting results...");
            
            info!("Exporting analysis results");
            exporters::export_analysis(&analysis, convert_format(cli.format), cli.output.as_ref())
                .with_context(|| "Failed to export analysis")?;
            
            pb.finish_with_message("Analysis complete!");
        }
    }

    info!("Completed successfully in {:.2?}", start_time.elapsed());
    Ok(())
}

/// Check if conda is available in the system and log warning if not
fn check_conda_availability() {
    match Command::new("conda").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                info!("Found conda: {}", version.trim());
            } else {
                warn!("Conda is installed but returned an error: {}", 
                      String::from_utf8_lossy(&output.stderr));
            }
        },
        Err(_) => {
            warn!("Conda is not available in the system PATH. Some features will use fallback mechanisms.");
            warn!("For complete functionality, please install conda and ensure it's in your PATH.");
        }
    }
}

/// Create advanced dependency graph with progress bar
fn create_advanced_dependency_graph(
    analysis: &conda_env_inspect::models::EnvironmentAnalysis,
    pb: ProgressBar,
) -> Result<conda_env_inspect::advanced_analysis::AdvancedDependencyGraph> {
    // First get the dependency map
    let deps = conda_env_inspect::analysis::get_real_package_dependencies(&analysis.packages);
    
    pb.set_position(70);
    pb.set_message("Creating advanced dependency graph...");
    
    // Create the advanced graph
    let graph = conda_env_inspect::advanced_analysis::create_advanced_dependency_graph(&analysis.packages, &deps);
    
    pb.set_position(80);
    
    Ok(graph)
}

/// Convert CLI OutputFormat to exporters ExportFormat
fn convert_format(format: conda_env_inspect::cli::OutputFormat) -> ExportFormat {
    match format {
        conda_env_inspect::cli::OutputFormat::Text => ExportFormat::Text,
        conda_env_inspect::cli::OutputFormat::Json => ExportFormat::Json,
        conda_env_inspect::cli::OutputFormat::Markdown => ExportFormat::Markdown,
        conda_env_inspect::cli::OutputFormat::Csv => ExportFormat::Csv,
        // For formats not directly supported, fall back to text
        _ => ExportFormat::Text,
    }
}
