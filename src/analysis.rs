use anyhow::{Context, Result};
use log::{debug, info, warn, error};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use serde_json::Value;

use crate::models::{CondaEnvironment, Dependency, Package};

/// Dependency graph representation
#[derive(Debug)]
pub struct DependencyGraph {
    /// Nodes in the graph (packages)
    pub nodes: Vec<String>,
    /// Edges between nodes (dependencies)
    pub edges: Vec<(String, String)>,
}

/// Creates a dependency graph from environment packages by querying conda metadata
pub fn create_dependency_graph(packages: &[Package]) -> DependencyGraph {
    let mut graph = DependencyGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    
    // Add all packages as nodes
    for package in packages {
        if !graph.nodes.contains(&package.name) {
            graph.nodes.push(package.name.clone());
        }
    }
    
    // Get real dependencies using conda metadata
    let dependency_map = get_real_package_dependencies(packages);
    
    // Add real dependency edges
    for package in packages {
        if let Some(deps) = dependency_map.get(&package.name) {
            for dep in deps {
                if graph.nodes.contains(dep) {
                    debug!("Adding dependency edge: {} -> {}", package.name, dep);
                    graph.edges.push((package.name.clone(), dep.clone()));
                }
            }
        }
    }
    
    graph
}

/// Get real package dependencies using Conda and PyPI APIs
pub fn get_real_package_dependencies(packages: &[Package]) -> HashMap<String, Vec<String>> {
    info!("Getting real package dependencies for {} packages", packages.len());
    let mut dependency_map: HashMap<String, Vec<String>> = HashMap::new();
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    
    for package in packages {
        let mut dependencies = Vec::new();
        let mut success = false;
        
        // Method 1: Try conda info command directly (most accurate for conda packages)
        match get_package_depends_info(&package.name) {
            Ok(deps) => {
                debug!("Found dependencies for {} via conda info: {:?}", package.name, deps);
                dependencies = deps;
                success = true;
            },
            Err(e) => debug!("Conda info failed for {}: {}", package.name, e)
        }
        
        // Method 2: Try using Anaconda API if conda command failed
        if !success {
            match get_package_depends_api(&package.name, package.channel.as_deref()) {
                Ok(deps) => {
                    debug!("Found dependencies for {} via Anaconda API: {:?}", package.name, deps);
                    dependencies = deps;
                    success = true;
                },
                Err(e) => debug!("Anaconda API failed for {}: {}", package.name, e)
            }
        }
        
        // Method 3: Try PyPI API for pip packages
        if !success && package.channel.as_deref() == Some("pip") {
            match get_pypi_dependencies(&client, &package.name) {
                Ok(deps) => {
                    debug!("Found dependencies for {} via PyPI API: {:?}", package.name, deps);
                    dependencies = deps;
                    success = true;
                },
                Err(e) => debug!("PyPI API failed for {}: {}", package.name, e)
            }
        }
        
        // Method 4: Use conda-meta JSON files in environment (if available)
        if !success {
            match get_conda_meta_dependencies(&package.name) {
                Ok(deps) => {
                    debug!("Found dependencies for {} via conda-meta: {:?}", package.name, deps);
                    dependencies = deps;
                    success = true;
                },
                Err(e) => debug!("Conda-meta failed for {}: {}", package.name, e)
            }
        }
        
        // Method 5: Use known dependencies for common packages as fallback
        if !success {
            if let Some(deps) = get_common_package_dependencies(&package.name) {
                debug!("Using known dependencies for {}: {:?}", package.name, deps);
                dependencies = deps;
                success = true;
            }
        }
        
        // If all methods failed, log a warning
        if !success {
            warn!("Could not determine dependencies for {}", package.name);
        }
        
        // Store whatever dependencies we found (even if empty)
        dependency_map.insert(package.name.clone(), dependencies);
    }
    
    // Analyze and enhance the dependency map by checking transitive dependencies
    enhance_dependency_map(&mut dependency_map);
    
    dependency_map
}

/// Get dependencies from PyPI API for pip packages
fn get_pypi_dependencies(client: &reqwest::blocking::Client, package_name: &str) -> Result<Vec<String>> {
    info!("Getting dependencies for {} via PyPI API", package_name);
    
    let url = format!("https://pypi.org/pypi/{}/json", package_name);
    
    let response = match client.get(&url).send() {
        Ok(resp) => resp,
        Err(e) => {
            warn!("Network error querying PyPI API: {}", e);
            return Err(anyhow::anyhow!("Network error: {}", e));
        }
    };
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("PyPI API request failed with status: {}", response.status()));
    }
    
    let json: serde_json::Value = match response.json() {
        Ok(json) => json,
        Err(e) => {
            warn!("Failed to parse PyPI API response: {}", e);
            return Err(anyhow::anyhow!("Failed to parse response: {}", e));
        }
    };
    
    let mut dependencies = Vec::new();
    
    // Extract requires_dist from info section (these are the dependencies)
    if let Some(requires_dist) = json["info"]["requires_dist"].as_array() {
        for req in requires_dist {
            if let Some(req_str) = req.as_str() {
                // PyPI format is like: "numpy (>=1.14.5) ; extra == 'test'"
                // We need to extract just the package name
                if let Some(pkg_name) = extract_pypi_package_name(req_str) {
                    dependencies.push(pkg_name);
                }
            }
        }
    }
    
    Ok(dependencies)
}

/// Extract package name from PyPI dependency specification
fn extract_pypi_package_name(dep_str: &str) -> Option<String> {
    // First, split on semicolon to remove environment markers
    let parts = dep_str.split(';').next()?;
    
    // Then extract the package name (everything before parens or whitespace)
    let name_parts = parts.trim().split_whitespace().next()?;
    
    // Handle parentheses
    if let Some(paren_pos) = name_parts.find('(') {
        Some(name_parts[0..paren_pos].trim().to_string())
    } else {
        Some(name_parts.trim().to_string())
    }
}

/// Get dependencies from conda-meta JSON files
fn get_conda_meta_dependencies(package_name: &str) -> Result<Vec<String>> {
    info!("Getting dependencies for {} via conda-meta files", package_name);
    
    // First, find the active conda environment path
    let output = Command::new("conda")
        .args(["info", "--json"])
        .output()
        .with_context(|| "Failed to execute conda info command")?;
        
    if !output.status.success() {
        return Err(anyhow::anyhow!("conda info command failed"));
    }
        
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .with_context(|| "Failed to parse JSON output from conda info")?;
        
    let active_prefix = json["active_prefix"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to get active conda environment"))?;
        
    // Look for the package's meta file
    let meta_dir = format!("{}/conda-meta", active_prefix);
    let meta_files = std::fs::read_dir(&meta_dir)
        .with_context(|| format!("Failed to read conda-meta directory at {}", meta_dir))?;
        
    // Find the meta file for our package
    for file_result in meta_files {
        let file = file_result?;
        let filename = file.file_name().to_string_lossy().to_string();
        
        // Check if this file is for our package (format: name-version-build.json)
        if filename.starts_with(&format!("{}-", package_name)) && filename.ends_with(".json") {
            let file_path = file.path();
            let content = std::fs::read_to_string(&file_path)
                .with_context(|| format!("Failed to read meta file {}", file_path.display()))?;
                
            let json: serde_json::Value = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse meta file {}", file_path.display()))?;
                
            let mut depends = Vec::new();
            
            // Extract dependencies
            if let Some(deps) = json["depends"].as_array() {
                for dep in deps {
                    if let Some(dep_str) = dep.as_str() {
                        if let Some(pkg_name) = extract_package_name(dep_str) {
                            depends.push(pkg_name);
                        }
                    }
                }
            }
            
            return Ok(depends);
        }
    }
    
    Err(anyhow::anyhow!("Could not find conda-meta file for {}", package_name))
}

/// Enhance dependency map by resolving transitive dependencies
fn enhance_dependency_map(dependency_map: &mut HashMap<String, Vec<String>>) {
    debug!("Enhancing dependency map with transitive dependencies");
    
    let packages: Vec<String> = dependency_map.keys().cloned().collect();
    
    // Process each package to ensure its dependencies are properly represented
    for package in &packages {
        // Get all first-level dependencies for this package
        if let Some(deps) = dependency_map.get(package) {
            // Ensure each dependency has its dependencies populated
            for dep in deps.clone() {
                // If this dependency exists in our map but has no dependencies yet,
                // try to find them from the common packages list if we don't have them yet
                if dependency_map.get(&dep).map_or(true, |deps| deps.is_empty()) {
                    if let Some(common_deps) = get_common_package_dependencies(&dep) {
                        dependency_map.insert(dep.clone(), common_deps);
                    }
                }
            }
        }
    }
    
    debug!("Dependency map enhanced: {} total packages", dependency_map.len());
}

/// Get package dependencies using conda info command
fn get_package_depends_info(package_name: &str) -> Result<Vec<String>> {
    info!("Getting dependencies for {} via conda info", package_name);
    
    let output = Command::new("conda")
        .args(["info", package_name, "--json"])
        .output()
        .with_context(|| format!("Failed to execute conda info command for {}", package_name))?;
        
    if !output.status.success() {
        return Err(anyhow::anyhow!("conda info command failed"));
    }
        
    let json: Value = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("Failed to parse JSON output from conda info"))?;
        
    let mut depends = Vec::new();
        
    if let Some(packages) = json["packages"].as_object() {
        for (_, pkg_info) in packages {
            if let Some(deps) = pkg_info["depends"].as_array() {
                for dep in deps {
                    if let Some(dep_str) = dep.as_str() {
                        if let Some(pkg_name) = extract_package_name(dep_str) {
                            depends.push(pkg_name);
                        }
                    }
                }
            }
        }
    }
        
    Ok(depends)
}

/// Get package dependencies using Anaconda API
fn get_package_depends_api(package_name: &str, channel: Option<&str>) -> Result<Vec<String>> {
    info!("Getting dependencies for {} via API", package_name);
    
    let channel = channel.unwrap_or("conda-forge");
    // Use a timeout to avoid hanging on slow connections
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    
    let url = format!("https://api.anaconda.org/package/{}/{}", channel, package_name);
    
    let response = match client.get(&url).send() {
        Ok(resp) => resp,
        Err(e) => {
            warn!("Network error querying API for dependencies: {}", e);
            return Err(anyhow::anyhow!("Network error: {}", e));
        }
    };
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("API request failed with status: {}", response.status()));
    }
    
    let json: Value = match response.json() {
        Ok(json) => json,
        Err(e) => {
            warn!("Failed to parse API response for dependencies: {}", e);
            return Err(anyhow::anyhow!("Failed to parse response: {}", e));
        }
    };
    
    let mut depends = Vec::new();
    
    if let Some(files) = json["files"].as_array() {
        // Get the latest version's dependencies
        if let Some(latest_file) = files.iter().find(|file| {
            file["version"].as_str() == json["latest_version"].as_str()
        }) {
            if let Some(deps) = latest_file["dependencies"].as_array() {
                for dep in deps {
                    if let Some(dep_str) = dep.as_str() {
                        if let Some(pkg_name) = extract_package_name(dep_str) {
                            depends.push(pkg_name);
                        }
                    }
                }
            }
        }
    }
    
    debug!("Retrieved {} dependencies for {} via API", depends.len(), package_name);
    Ok(depends)
}

/// Extract package name from dependency specification
fn extract_package_name(dep_str: &str) -> Option<String> {
    dep_str.split_whitespace()
        .next()
        .map(|s| s.trim().to_string())
}

/// Get common dependencies for well-known packages as a fallback
fn get_common_package_dependencies(package_name: &str) -> Option<Vec<String>> {
    let common_deps: HashMap<&str, Vec<&str>> = [
        ("pandas", vec!["numpy", "python", "python-dateutil", "pytz"]),
        ("matplotlib", vec!["numpy", "python", "pillow", "cycler"]),
        ("scikit-learn", vec!["numpy", "scipy", "python", "joblib"]),
        ("tensorflow", vec!["numpy", "python", "protobuf", "absl-py"]),
        ("pytorch", vec!["python", "numpy"]),
        ("jupyterlab", vec!["python", "jupyter-core", "ipython"]),
    ].iter().cloned().collect();
    
    common_deps.get(package_name)
        .map(|deps| deps.iter().map(|&s| s.to_string()).collect())
}

/// Exports the dependency graph to DOT format for visualization
pub fn export_dependency_graph<P: AsRef<Path>>(graph: &DependencyGraph, output_path: P) -> Result<()> {
    let mut file = File::create(output_path)
        .with_context(|| "Failed to create graph file")?;
    
    // Write DOT header
    writeln!(file, "digraph conda_dependencies {{")?;
    writeln!(file, "  node [shape=box, style=filled, fillcolor=lightblue];")?;
    
    // Write nodes with attributes
    for node in &graph.nodes {
        writeln!(file, "  \"{}\" [label=\"{}\"];", node, node)?;
    }
    
    // Write edges
    for (from, to) in &graph.edges {
        writeln!(file, "  \"{}\" -> \"{}\";", from, to)?;
    }
    
    // Write DOT footer
    writeln!(file, "}}")?;
    
    Ok(())
}

/// Generate environment recommendations based on the analysis
pub fn generate_recommendations(packages: &[Package], check_outdated: bool) -> Vec<String> {
    let mut recommendations = Vec::new();
    
    // Check for outdated packages
    let outdated: Vec<&Package> = packages.iter()
        .filter(|p| p.is_outdated)
        .collect();
    
    if check_outdated && !outdated.is_empty() {
        recommendations.push(format!(
            "Found {} outdated packages. Consider updating them for security and performance improvements.",
            outdated.len()
        ));
        
        // List top 3 outdated packages
        let top_outdated: Vec<&Package> = outdated.into_iter()
            .take(3)
            .collect();
        
        for pkg in top_outdated {
            if let Some(latest) = &pkg.latest_version {
                recommendations.push(format!(
                    "Update {} from {} to {}",
                    pkg.name,
                    pkg.version.as_deref().unwrap_or("unknown"),
                    latest
                ));
            }
        }
    }
    
    // Check for pinned versions
    let pinned_count = packages.iter()
        .filter(|p| p.is_pinned)
        .count();
    
    if pinned_count > 0 {
        let percentage = (pinned_count as f64 / packages.len() as f64) * 100.0;
        
        if percentage > 70.0 {
            recommendations.push(format!(
                "{:.1}% of packages have pinned versions. This ensures reproducibility but may prevent updates.",
                percentage
            ));
        } else if percentage < 30.0 {
            recommendations.push(format!(
                "Only {:.1}% of packages have pinned versions. Consider pinning more packages for better reproducibility.",
                percentage
            ));
        }
    }
    
    // Check environment size
    let total_size: u64 = packages.iter()
        .filter_map(|p| p.size)
        .sum();
    
    if total_size > 2_000_000_000 {
        recommendations.push(
            "Environment is quite large. Consider creating a minimal environment with only required packages.".to_string()
        );
    }
    
    // Check for redundant packages
    let redundant_packages = identify_redundant_packages(packages);
    if !redundant_packages.is_empty() {
        recommendations.push(format!(
            "Found {} potentially redundant packages that might be removed to streamline your environment.",
            redundant_packages.len()
        ));
        
        for pkg in redundant_packages.iter().take(3) {
            recommendations.push(format!("Consider removing unused package: {}", pkg));
        }
    }
    
    recommendations
}

/// Identify potentially redundant packages in the environment
fn identify_redundant_packages(packages: &[Package]) -> Vec<String> {
    // Get real dependencies
    let dependency_map = get_real_package_dependencies(packages);
    
    // Find packages that are not direct dependencies of any other package
    // and have no direct Python imports (common in dev dependencies)
    let mut potentially_redundant = Vec::new();
    
    // Create a set of all packages that are dependencies
    let mut is_dependency = HashSet::new();
    for deps in dependency_map.values() {
        for dep in deps {
            is_dependency.insert(dep.clone());
        }
    }
    
    // Commonly used dev packages that should not be flagged as redundant
    let dev_packages = [
        "pytest", "black", "flake8", "mypy", "isort", "pylint", 
        "jupyter", "ipython", "notebook", "ipykernel", "jupyterlab"
    ];
    
    // Check each package
    for package in packages {
        // Skip if it's a dependency or a common dev package
        if is_dependency.contains(&package.name) || 
           dev_packages.contains(&package.name.as_str()) {
            continue;
        }
        
        // Potentially redundant
        potentially_redundant.push(package.name.clone());
    }
    
    potentially_redundant
} 