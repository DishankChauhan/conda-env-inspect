use anyhow::{Context, Result};
use log::{debug, warn};
use petgraph::Direction;
use rayon::prelude::*;
use regex::Regex;
use std::path::Path;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::analysis;
use crate::conda_api;
use crate::models::{EnvironmentAnalysis, Package, Recommendation};
use crate::parsers;
use crate::advanced_analysis::AdvancedDependencyGraph;

/// Analyzes a Conda environment file and returns the analysis results
pub fn analyze_environment<P: AsRef<Path>>(
    file_path: P,
    should_check_outdated: bool,
    flag_pinned: bool,
) -> Result<EnvironmentAnalysis> {
    // Parse the environment file
    let env = parsers::parse_environment_file(&file_path)?;
    
    // Process and enrich all packages
    let mut packages = extract_packages_from_environment(&env)?;
    
    // Flag pinned packages if requested
    if flag_pinned {
        for package in &mut packages {
            package.is_pinned = is_pinned_package(&package.name, &env)?;
        }
    }
    
    // Check for outdated packages if requested
    if should_check_outdated {
        for package in &mut packages {
            if let Some((is_outdated, latest)) = check_outdated(&package.name, package.version.as_deref()) {
                package.is_outdated = is_outdated;
                package.latest_version = latest;
            }
        }
    }
    
    // Get package sizes
    let total_size = get_packages_sizes(&mut packages);
    
    // Count pinned and outdated packages
    let pinned_count = packages.iter().filter(|p| p.is_pinned).count();
    let outdated_count = packages.iter().filter(|p| p.is_outdated).count();
    
    // Generate simple dependency graph
    let dependency_graph = analysis::create_dependency_graph(&packages);
    
    // Generate recommendations
    let recommendations = generate_simple_recommendations(&packages, pinned_count, outdated_count);
    
    Ok(EnvironmentAnalysis {
        name: env.name.clone(),
        packages,
        total_size,
        pinned_count,
        outdated_count,
        recommendations,
    })
}

/// Analyzes a Conda environment file using parallel processing for better performance
pub fn analyze_environment_parallel<P: AsRef<Path>>(
    file_path: P,
    should_check_outdated: bool,
    flag_pinned: bool,
) -> Result<EnvironmentAnalysis> {
    // Parse the environment file
    let env = parsers::parse_environment_file(&file_path)?;
    
    // Process and enrich all packages
    let mut packages = extract_packages_from_environment(&env)?;
    
    // Flag pinned packages if requested
    if flag_pinned {
        packages.par_iter_mut().for_each(|package| {
            package.is_pinned = is_pinned_package(&package.name, &env).unwrap_or(false);
        });
    }
    
    // Check for outdated packages if requested
    if should_check_outdated {
        packages.par_iter_mut().for_each(|package| {
            if let Some((is_outdated, latest)) = check_outdated(&package.name, package.version.as_deref()) {
                package.is_outdated = is_outdated;
                package.latest_version = latest;
            }
        });
    }
    
    // Get package sizes
    let total_size = get_packages_sizes(&mut packages);
    
    // Count pinned and outdated packages
    let pinned_count = packages.iter().filter(|p| p.is_pinned).count();
    let outdated_count = packages.iter().filter(|p| p.is_outdated).count();
    
    // Generate simple dependency graph
    let dependency_graph = analysis::create_dependency_graph(&packages);
    
    // Generate recommendations
    let recommendations = generate_simple_recommendations(&packages, pinned_count, outdated_count);
    
    Ok(EnvironmentAnalysis {
        name: env.name.clone(),
        packages,
        total_size,
        pinned_count,
        outdated_count,
        recommendations,
    })
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

pub fn generate_recommendations(packages: &[Package], dependency_graph: &AdvancedDependencyGraph) -> Vec<Recommendation> {
    let mut recommendations = Vec::new();

    for package in packages {
        // Check for outdated versions
        if package.is_outdated {
            recommendations.push(Recommendation {
                description: format!("Package {} is outdated", package.name),
                details: Some(format!("Current version: {}, Latest version: {}", 
                    package.version.as_deref().unwrap_or("unknown"), 
                    package.latest_version.as_deref().unwrap_or("unknown"))),
                value: "1.0".to_string(),
            });
        }

        // Check for security vulnerabilities
        // For now, just flag significantly outdated packages as potentially vulnerable
        if package.is_outdated && package.latest_version.is_some() {
            recommendations.push(Recommendation {
                description: format!("Potential security vulnerabilities in {}", package.name),
                details: Some("Significantly outdated packages may contain security vulnerabilities".to_string()),
                value: "2.0".to_string(),
            });
        }

        // Check for deprecated packages
        if is_deprecated(&package.name) {
            recommendations.push(Recommendation {
                description: format!("Package {} is deprecated", package.name),
                details: Some("Consider finding an alternative package".to_string()),
                value: "1.0".to_string(),
            });
        }
    }

    // Analyze dependency graph for unused dependencies
    let unused = find_unused_dependencies(dependency_graph);
    if !unused.is_empty() {
        recommendations.push(Recommendation {
            description: "Unused dependencies detected".to_string(),
            details: Some(format!("Consider removing: {}", unused.join(", "))),
            value: format!("{}.0", unused.len()),
        });
    }

    recommendations
}

fn check_latest_version(package_name: &str) -> Option<String> {
    // Mock implementation since the real API call function is missing
    None
}

fn is_deprecated(package_name: &str) -> bool {
    // Check if the package is in a list of known deprecated packages 
    let deprecated_packages = vec!["deprecated_pkg1", "deprecated_pkg2"];
    deprecated_packages.contains(&package_name)
}

fn find_unused_dependencies(graph: &AdvancedDependencyGraph) -> Vec<String> {
    let mut unused = Vec::new();
    
    // Find packages that are not depended on by any other package
    for pkg in graph.graph.node_indices() {
        let pkg_name = &graph.graph[pkg];
        if graph.graph.neighbors_directed(pkg, Direction::Incoming).count() == 0 {
            unused.push(pkg_name.clone());
        }
    }
    
    unused
}

// Generate simple text recommendations instead of structured Recommendation objects
fn generate_simple_recommendations(
    packages: &[Package], 
    pinned_count: usize, 
    outdated_count: usize
) -> Vec<Recommendation> {
    let mut recommendations = Vec::new();
    
    // Add recommendations for outdated packages
    if outdated_count > 0 {
        let percent = (outdated_count as f64 / packages.len() as f64) * 100.0;
        recommendations.push(Recommendation {
            description: format!("Found {} outdated packages ({}%). Consider updating them for security and performance improvements.", 
                outdated_count, percent as u32),
            value: format!("{}", outdated_count),
            details: None,
        });
        
        // Add specific update recommendations for each outdated package
        for package in packages.iter().filter(|p| p.is_outdated) {
            if let (Some(version), Some(latest)) = (&package.version, &package.latest_version) {
                recommendations.push(Recommendation {
                    description: format!("Update {} from {} to {}", package.name, version, latest),
                    value: "1.0".to_string(),
                    details: None,
                });
            }
        }
    }
    
    // Add recommendation about pinned packages
    if pinned_count > 0 {
        let percent = (pinned_count as f64 / packages.len() as f64) * 100.0;
        recommendations.push(Recommendation {
            description: format!("{}% of packages have pinned versions. This ensures reproducibility but may prevent updates.", 
                percent as u32),
            value: format!("{}", pinned_count),
            details: None,
        });
    }
    
    recommendations
}

/// Extracts packages from a conda environment
fn extract_packages_from_environment(env: &crate::models::CondaEnvironment) -> Result<Vec<Package>> {
    let mut packages = Vec::new();
    
    // Extract normal dependencies
    for dep in &env.dependencies {
        match dep {
            crate::models::Dependency::Simple(spec) => {
                let parts: Vec<&str> = spec.split('=').collect();
                let name = parts[0].trim().to_string();
                let version = if parts.len() > 1 { Some(parts[1].trim().to_string()) } else { None };
                let is_pinned = version.is_some();
                
                packages.push(Package {
                    name,
                    version,
                    build: None,
                    channel: None,
                    size: None,
                    is_pinned,
                    is_outdated: false,
                    latest_version: None,
                });
            },
            crate::models::Dependency::Complex(complex) => {
                // Handle pip packages
                if let Some(pip_pkgs) = &complex.pip {
                    for pip_spec in pip_pkgs {
                        let parts: Vec<&str> = pip_spec.split('=').collect();
                        let name = parts[0].trim().to_string();
                        let version = if parts.len() > 1 { 
                            Some(parts[1].trim().to_string()) 
                        } else { 
                            None 
                        };
                        let is_pinned = version.is_some();
                        
                        packages.push(Package {
                            name,
                            version,
                            build: None,
                            channel: Some("pip".to_string()),
                            size: None,
                            is_pinned,
                            is_outdated: false,
                            latest_version: None,
                        });
                    }
                }
            }
        }
    }
    
    Ok(packages)
}

/// Checks if a package is pinned in the environment
fn is_pinned_package(pkg_name: &str, env: &crate::models::CondaEnvironment) -> Result<bool> {
    for dep in &env.dependencies {
        match dep {
            crate::models::Dependency::Simple(spec) => {
                let parts: Vec<&str> = spec.split('=').collect();
                if parts[0].trim() == pkg_name {
                    return Ok(parts.len() > 1);
                }
            },
            crate::models::Dependency::Complex(complex) => {
                if let Some(pip_pkgs) = &complex.pip {
                    for pip_spec in pip_pkgs {
                        let parts: Vec<&str> = pip_spec.split('=').collect();
                        if parts[0].trim() == pkg_name {
                            return Ok(parts.len() > 1);
                        }
                    }
                }
            }
        }
    }
    
    Ok(false)
}

/// Checks if a package is outdated by querying the conda API
fn check_outdated(pkg_name: &str, current_version: Option<&str>) -> Option<(bool, Option<String>)> {
    if let Some(current) = current_version {
        // Query the conda API for the latest version
        match conda_api::get_latest_version(pkg_name) {
            Ok(latest) => {
                // Compare versions using semver if possible
                let is_outdated = match (semver::Version::parse(current), semver::Version::parse(&latest)) {
                    (Ok(curr_ver), Ok(latest_ver)) => latest_ver > curr_ver,
                    _ => latest != current.to_string() // Fallback to string comparison if parsing fails
                };
                
                Some((is_outdated, Some(latest)))
            },
            Err(_) => Some((false, None)) // Couldn't determine, assume not outdated
        }
    } else {
        Some((false, None)) // No current version, can't determine
    }
}

/// Get package sizes by reading package metadata
fn get_packages_sizes(packages: &mut [Package]) -> Option<u64> {
    let mut total_size = 0;
    
    let active_env = std::env::var("CONDA_PREFIX").ok();
    
    if let Some(env_path) = active_env {
        // Get sizes from actual conda packages in the environment
        for package in packages {
            // Look for package in pkgs directory
            let pkg_paths = glob::glob(&format!("{}/pkgs/{}*", env_path, package.name))
                .ok()?
                .filter_map(Result::ok);
            
            for path in pkg_paths {
                if path.is_dir() && path.file_name().unwrap().to_string_lossy().contains(&package.name) {
                    // Walk the directory and calculate size
                    let pkg_size = walkdir::WalkDir::new(&path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter_map(|e| e.metadata().ok())
                        .filter(|m| m.is_file())
                        .fold(0, |acc, m| acc + m.len());
                    
                    package.size = Some(pkg_size);
                    total_size += pkg_size;
                    break;
                }
            }
            
            // If size still not determined, query conda API
            if package.size.is_none() {
                if let Ok(size) = conda_api::get_package_size(&package.name) {
                    package.size = Some(size);
                    total_size += size;
                }
            }
        }
    } else {
        // Fallback to conda API if no active environment
        for package in packages {
            if let Ok(size) = conda_api::get_package_size(&package.name) {
                package.size = Some(size);
                total_size += size;
            } else {
                // Estimate size if API fails (better than having nothing)
                package.size = Some(5_000_000); // Default guess 5MB
                total_size += 5_000_000;
            }
        }
    }
    
    if total_size > 0 {
        Some(total_size)
    } else {
        None
    }
}
