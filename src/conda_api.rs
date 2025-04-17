use anyhow::{Context, Result};
use log::{debug, info, warn, error};
use reqwest::blocking::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use std::process::Command;
use std::collections::HashMap;

use crate::models::Package;

const ANACONDA_API_URL: &str = "https://api.anaconda.org/package";

/// Package information structure returned by API calls
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Name of the package
    pub name: String,
    /// Latest version of the package
    pub latest_version: String,
    /// Size of the package in bytes
    pub size: Option<u64>,
    /// Available versions of the package
    pub versions: Vec<String>,
}

/// Get information about a package from the Conda API
pub fn get_package_info(package_name: &str, channel: Option<&str>) -> Result<PackageInfo> {
    let channel = channel.unwrap_or("conda-forge");
    let url = format!("{}/{}/{}", ANACONDA_API_URL, channel, package_name);
    
    debug!("Querying Anaconda API: {}", url);
    
    // Use a timeout to avoid hanging on slow connections
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    
    let response = match client.get(&url).send() {
        Ok(resp) => resp,
        Err(e) => {
            warn!("Network error querying API: {}", e);
            return Err(anyhow::anyhow!("Network error: {}", e));
        }
    };
    
    if !response.status().is_success() {
        error!("API request failed with status: {}", response.status());
        return Err(anyhow::anyhow!("Failed to get package info: HTTP status {}", response.status()));
    }
    
    let json: serde_json::Value = match response.json() {
        Ok(json) => json,
        Err(e) => {
            warn!("Failed to parse API response: {}", e);
            return Err(anyhow::anyhow!("Failed to parse response: {}", e));
        }
    };
    
    debug!("Received package info for {}", package_name);
    
    // Extract the latest version and all versions
    let latest_version = json["latest_version"].as_str()
        .unwrap_or("unknown")
        .to_string();
    
    // Extract versions
    let versions = if let Some(files) = json["files"].as_array() {
        let mut versions = Vec::new();
        for file in files {
            if let Some(version) = file["version"].as_str() {
                if !versions.contains(&version.to_string()) {
                    versions.push(version.to_string());
                }
            }
        }
        versions
    } else {
        Vec::new()
    };
    
    // Extract file size (approximate from latest version)
    let size = if let Some(files) = json["files"].as_array() {
        files.iter()
            .filter(|file| {
                file["version"].as_str() == Some(&latest_version)
            })
            .map(|file| file["size"].as_u64().unwrap_or(0))
            .max()
    } else {
        None
    };
    
    Ok(PackageInfo {
        name: package_name.to_string(),
        latest_version,
        size,
        versions,
    })
}

/// Check if a package is outdated using semantic versioning
pub fn is_outdated(package: &Package, info: &PackageInfo) -> bool {
    if let Some(version) = &package.version {
        // Use semver for proper version comparison
        match (parse_conda_version(version), parse_conda_version(&info.latest_version)) {
            (Some(current_version), Some(latest_version)) => {
                debug!("Comparing versions for {}: current={}, latest={}", 
                       package.name, current_version, latest_version);
                current_version < latest_version
            },
            _ => {
                // Fallback to string comparison if parsing fails
                warn!("Failed to parse version for {}, falling back to string comparison", package.name);
                version != &info.latest_version
            }
        }
    } else {
        false
    }
}

/// Parse a conda version string into a semver Version
fn parse_conda_version(version_str: &str) -> Option<Version> {
    // Normalize conda version for semver parsing
    let normalized = normalize_conda_version(version_str);
    match Version::parse(&normalized) {
        Ok(version) => Some(version),
        Err(e) => {
            warn!("Failed to parse version '{}': {}", version_str, e);
            None
        }
    }
}

/// Normalize conda version string to semver compatibility
fn normalize_conda_version(version: &str) -> String {
    // Handle conda specific version formats
    let version_without_build;
    
    // Remove build string if present
    if let Some(idx) = version.find('+') {
        version_without_build = &version[0..idx];
    } else if let Some(idx) = version.find('-') {
        if !version.starts_with("0-") {
            version_without_build = &version[0..idx];
        } else {
            version_without_build = version;
        }
    } else {
        version_without_build = version;
    }
    
    // Ensure there are at least major.minor.patch components
    let parts: Vec<&str> = version_without_build.split('.').collect();
    match parts.len() {
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => version_without_build.to_string(),
    }
}

/// Get the total size of an environment by querying conda and inspecting the file system
pub fn get_environment_size(env_name: &str) -> Result<Option<u64>> {
    info!("Calculating size for environment: {}", env_name);
    
    // Get the environment path
    let env_path = get_env_path(env_name)?;
    
    // If we have a valid path, calculate the total size
    if let Some(path) = env_path {
        debug!("Found environment path: {}", path);
        let size = calculate_directory_size(&path)?;
        info!("Total environment size: {} bytes", size);
        Ok(Some(size))
    } else {
        warn!("Could not determine environment path for: {}", env_name);
        Ok(None)
    }
}

/// Get the file system path for a conda environment
fn get_env_path(env_name: &str) -> Result<Option<String>> {
    debug!("Looking up environment path for: {}", env_name);
    
    let output = std::process::Command::new("conda")
        .args(["env", "list", "--json"])
        .output()
        .with_context(|| "Failed to execute conda env list command")?;

    if !output.status.success() {
        error!("conda env list command failed: {}", 
               String::from_utf8_lossy(&output.stderr));
        return Ok(None);
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .with_context(|| "Failed to parse conda env list JSON output")?;

    if let Some(envs) = json["envs"].as_array() {
        for env_path in envs {
            if let Some(path_str) = env_path.as_str() {
                // Get the environment name from the path
                let path = Path::new(path_str);
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        if name_str == env_name {
                            debug!("Found environment path: {}", path_str);
                            return Ok(Some(path_str.to_string()));
                        }
                    }
                }
            }
        }
    }

    warn!("Environment not found: {}", env_name);
    Ok(None)
}

/// Calculate the total size of a directory recursively
fn calculate_directory_size(dir_path: &str) -> Result<u64> {
    debug!("Calculating directory size for: {}", dir_path);
    
    let path = Path::new(dir_path);
    let mut total_size = 0;

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            
            if entry_path.is_file() {
                let file_size = entry.metadata()?.len();
                total_size += file_size;
                debug!("File: {}, size: {} bytes", entry_path.display(), file_size);
            } else if entry_path.is_dir() {
                if let Some(path_str) = entry_path.to_str() {
                    let dir_size = calculate_directory_size(path_str)?;
                    total_size += dir_size;
                    debug!("Directory: {}, size: {} bytes", entry_path.display(), dir_size);
                }
            }
        }
    }

    Ok(total_size)
}

/// Enriches package information with data from Conda API
pub fn enrich_packages(packages: &mut Vec<Package>) -> Result<()> {
    info!("Enriching package information for {} packages", packages.len());
    
    for package in packages {
        // Skip packages without a name or pip packages
        if package.name.is_empty() || package.name.contains('>') {
            debug!("Skipping package: {}", package.name);
            continue;
        }
        
        debug!("Enriching package: {}", package.name);
        
        // Try to get package info from API
        match get_package_info(&package.name, package.channel.as_deref()) {
            Ok(info) => {
                // Check if outdated
                package.is_outdated = is_outdated(package, &info);
                
                // Set latest version
                package.latest_version = Some(info.latest_version.clone());
                
                // Set package size
                package.size = info.size;
                
                debug!("Enriched {}: outdated={}, latest={}, size={:?}", 
                       package.name, package.is_outdated, 
                       info.latest_version, package.size);
            },
            Err(e) => {
                warn!("Failed to get info for package {}: {}", package.name, e);
            }
        }
    }
    
    info!("Package enrichment complete");
    Ok(())
}

/// Get the latest version of a package from conda-forge
pub fn get_latest_version(package_name: &str) -> Result<String> {
    // First try using conda directly
    match get_latest_version_conda(package_name) {
        Ok(version) => return Ok(version),
        Err(e) => debug!("Failed to get latest version via conda: {}", e),
    }
    
    // Fall back to Anaconda API
    get_latest_version_api(package_name)
}

/// Get the latest version using conda command
fn get_latest_version_conda(package_name: &str) -> Result<String> {
    info!("Getting latest version for {} via conda", package_name);
    
    let output = Command::new("conda")
        .args(["search", package_name, "--json"])
        .output()
        .with_context(|| format!("Failed to execute conda search for {}", package_name))?;
        
    if !output.status.success() {
        return Err(anyhow::anyhow!("conda search command failed with status: {}", output.status));
    }
        
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("Failed to parse JSON output from conda search"))?;
        
    // Find the latest version
    if let Some(packages) = json[package_name].as_array() {
        // Get all versions
        let mut versions = Vec::new();
        for pkg in packages {
            if let Some(version) = pkg["version"].as_str() {
                versions.push(version.to_string());
            }
        }
        
        // Sort versions and get latest (last in sorted array)
        versions.sort_by(|a, b| {
            // Try to use semver for comparison if possible
            match (Version::parse(a), Version::parse(b)) {
                (Ok(ver_a), Ok(ver_b)) => ver_a.cmp(&ver_b),
                _ => a.cmp(b) // Fallback to lexicographic ordering
            }
        });
        
        if let Some(latest) = versions.last() {
            return Ok(latest.clone());
        }
    }
    
    Err(anyhow::anyhow!("Failed to find latest version for {}", package_name))
}

/// Get the latest version using Anaconda API
fn get_latest_version_api(package_name: &str) -> Result<String> {
    info!("Getting latest version for {} via API", package_name);
    
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    
    // Try conda-forge first, then default channels
    for channel in &["conda-forge", "main"] {
        let url = format!("https://api.anaconda.org/package/{}/{}", channel, package_name);
        
        match client.get(&url).send() {
            Ok(response) => {
                if response.status().is_success() {
                    let json: serde_json::Value = response.json()
                        .with_context(|| format!("Failed to parse API response for {}", package_name))?;
                    
                    if let Some(latest) = json["latest_version"].as_str() {
                        return Ok(latest.to_string());
                    }
                }
            },
            Err(e) => debug!("API request to {} failed: {}", url, e),
        }
    }
    
    // Try PyPI for Python packages
    let pypi_url = format!("https://pypi.org/pypi/{}/json", package_name);
    match client.get(&pypi_url).send() {
        Ok(response) => {
            if response.status().is_success() {
                let json: serde_json::Value = response.json()
                    .with_context(|| format!("Failed to parse PyPI API response for {}", package_name))?;
                
                if let Some(version) = json["info"]["version"].as_str() {
                    return Ok(version.to_string());
                }
            }
        },
        Err(e) => debug!("PyPI API request failed: {}", e),
    }
    
    Err(anyhow::anyhow!("Could not determine latest version for {}", package_name))
}

/// Get the size of a package in bytes
pub fn get_package_size(package_name: &str) -> Result<u64> {
    // First try using conda directly
    match get_package_size_conda(package_name) {
        Ok(size) => return Ok(size),
        Err(e) => debug!("Failed to get package size via conda: {}", e),
    }
    
    // Fall back to Anaconda API
    get_package_size_api(package_name)
}

/// Get package size using conda command
fn get_package_size_conda(package_name: &str) -> Result<u64> {
    info!("Getting package size for {} via conda", package_name);
    
    let output = Command::new("conda")
        .args(["search", package_name, "--info", "--json"])
        .output()
        .with_context(|| format!("Failed to execute conda search --info for {}", package_name))?;
        
    if !output.status.success() {
        return Err(anyhow::anyhow!("conda search command failed with status: {}", output.status));
    }
        
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("Failed to parse JSON output from conda search --info"))?;
        
    // Extract size information
    if let Some(packages) = json[package_name].as_array() {
        for pkg in packages {
            if let Some(size) = pkg["size"].as_u64() {
                return Ok(size);
            }
        }
    }
    
    Err(anyhow::anyhow!("Failed to get size information for {}", package_name))
}

/// Get package size using Anaconda API
fn get_package_size_api(package_name: &str) -> Result<u64> {
    info!("Getting package size for {} via API", package_name);
    
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    
    // Try conda-forge first, then default channels
    for channel in &["conda-forge", "main"] {
        let url = format!("https://api.anaconda.org/package/{}/{}", channel, package_name);
        
        match client.get(&url).send() {
            Ok(response) => {
                if response.status().is_success() {
                    let json: serde_json::Value = response.json()
                        .with_context(|| format!("Failed to parse API response for {}", package_name))?;
                    
                    if let Some(files) = json["files"].as_array() {
                        if let Some(file) = files.first() {
                            if let Some(size) = file["size"].as_u64() {
                                return Ok(size);
                            }
                        }
                    }
                }
            },
            Err(e) => debug!("API request to {} failed: {}", url, e),
        }
    }
    
    Err(anyhow::anyhow!("Could not determine package size for {}", package_name))
} 