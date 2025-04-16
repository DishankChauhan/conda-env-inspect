use anyhow::{Context, Result};
use std::path::Path;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::analysis;
use crate::conda_api;
use crate::models::EnvironmentAnalysis;
use crate::parsers;

/// Analyzes a Conda environment file and returns the analysis results
pub fn analyze_environment<P: AsRef<Path>>(
    file_path: P,
    check_outdated: bool,
    flag_pinned: bool,
) -> Result<EnvironmentAnalysis> {
    // Parse the environment file
    let env = parsers::parse_environment_file(&file_path)?;
    
    // Extract packages
    let mut packages = parsers::extract_packages(&env);
    
    // Calculate total size and check for outdated packages if requested
    let mut total_size = None;
    let mut outdated_count = 0;
    
    if check_outdated {
        // Enrich packages with information from Conda API
        conda_api::enrich_packages(&mut packages)?;
        
        // Count outdated packages
        outdated_count = packages.iter().filter(|p| p.is_outdated).count();
        
        // Calculate total size
        let package_sizes: u64 = packages.iter()
            .filter_map(|p| p.size)
            .sum();
        
        // If we have at least some package sizes, set the total
        if package_sizes > 0 {
            total_size = Some(package_sizes);
        } else if let Some(name) = &env.name {
            // If we couldn't get sizes from packages, try to get from environment
            total_size = conda_api::get_environment_size(name)?;
        }
    }
    
    // Count pinned packages if requested
    let pinned_count = if flag_pinned {
        packages.iter().filter(|p| p.is_pinned).count()
    } else {
        0
    };
    
    // Generate recommendations if needed
    let recommendations = analysis::generate_recommendations(&packages, check_outdated);
    
    // Create analysis result
    let analysis = EnvironmentAnalysis {
        name: env.name.clone(),
        packages,
        total_size,
        pinned_count,
        outdated_count,
        recommendations,
    };
    
    Ok(analysis)
}

/// Analyzes a Conda environment file in parallel using multiple threads for better performance
pub fn analyze_environment_parallel<P: AsRef<Path>>(
    file_path: P,
    check_outdated: bool,
    flag_pinned: bool,
) -> Result<EnvironmentAnalysis> {
    let start_time = Instant::now();
    
    // Parse the environment file
    let env = parsers::parse_environment_file(&file_path)?;
    
    // Extract packages
    let mut packages = parsers::extract_packages(&env);
    
    // Calculate total size and check for outdated packages if requested
    let mut total_size = None;
    let mut outdated_count = 0;
    
    if check_outdated {
        // Use parallel processing for package enrichment
        // Split packages into chunks for multi-threading
        let chunk_size = (packages.len() / num_cpus::get()).max(1);
        let chunks: Vec<Vec<_>> = packages.chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();
            
        let enriched_chunks = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        
        // Process each chunk in parallel
        let handles: Vec<_> = chunks.into_iter().enumerate().map(|(i, mut chunk)| {
            let enriched_chunks = Arc::clone(&enriched_chunks);
            let errors = Arc::clone(&errors);
            
            thread::spawn(move || {
                println!("Processing chunk {} with {} packages", i, chunk.len());
                match conda_api::enrich_packages(&mut chunk) {
                    Ok(_) => {
                        let mut enriched = enriched_chunks.lock().unwrap();
                        enriched.push(chunk);
                    },
                    Err(e) => {
                        let mut err_list = errors.lock().unwrap();
                        err_list.push(format!("Error in chunk {}: {}", i, e));
                    }
                }
            })
        }).collect();
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread panicked");
        }
        
        // Check for errors
        let err_list = errors.lock().unwrap();
        if !err_list.is_empty() {
            return Err(anyhow::anyhow!("Errors during parallel processing: {:?}", err_list));
        }
        
        // Reassemble packages
        let enriched_chunks = enriched_chunks.lock().unwrap();
        packages = enriched_chunks.iter().flat_map(|chunk| chunk.clone()).collect();
        
        // Count outdated packages
        outdated_count = packages.iter().filter(|p| p.is_outdated).count();
        
        // Calculate total size
        let package_sizes: u64 = packages.iter()
            .filter_map(|p| p.size)
            .sum();
        
        // If we have at least some package sizes, set the total
        if package_sizes > 0 {
            total_size = Some(package_sizes);
        } else if let Some(name) = &env.name {
            // If we couldn't get sizes from packages, try to get from environment
            total_size = conda_api::get_environment_size(name)?;
        }
    }
    
    // Count pinned packages if requested
    let pinned_count = if flag_pinned {
        packages.iter().filter(|p| p.is_pinned).count()
    } else {
        0
    };
    
    // Generate recommendations
    let recommendations = analysis::generate_recommendations(&packages, check_outdated);
    
    // Create analysis result
    let analysis = EnvironmentAnalysis {
        name: env.name.clone(),
        packages,
        total_size,
        pinned_count,
        outdated_count,
        recommendations,
    };
    
    let duration = start_time.elapsed();
    println!("Parallel analysis completed in {:.2?}", duration);
    
    Ok(analysis)
}

/// Generate a dependency graph for an environment and save it to a file
pub fn generate_dependency_graph<P1: AsRef<Path>, P2: AsRef<Path>>(
    file_path: P1,
    output_path: P2,
) -> Result<()> {
    // Parse the environment file
    let env = parsers::parse_environment_file(&file_path)?;
    
    // Extract packages
    let packages = parsers::extract_packages(&env);
    
    // Create dependency graph
    let graph = analysis::create_dependency_graph(&packages);
    
    // Export graph to DOT format
    analysis::export_dependency_graph(&graph, output_path)?;
    
    Ok(())
}

/// Formats a file size to a human-readable string
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} bytes", size)
    }
}
