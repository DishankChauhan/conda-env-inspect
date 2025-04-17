use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::models::{CondaEnvironment, Dependency, Package};

/// Parses a Conda environment file (YAML or JSON) and returns the environment data
pub fn parse_environment_file<P: AsRef<Path>>(file_path: P) -> Result<CondaEnvironment> {
    let file_path = file_path.as_ref();
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    match extension.to_lowercase().as_str() {
        "yml" | "yaml" => parse_yaml_file(file_path),
        "conda" | "json" => parse_json_file(file_path),
        _ => Err(anyhow::anyhow!(
            "Unsupported file format: {}. Only .yml, .yaml, .conda, or .json files are supported.",
            extension
        )),
    }
}

/// Parses a YAML environment file
fn parse_yaml_file<P: AsRef<Path>>(file_path: P) -> Result<CondaEnvironment> {
    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read YAML file: {:?}", file_path.as_ref()))?;
    
    serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML content from: {:?}", file_path.as_ref()))
}

/// Parses a JSON environment file (like .conda files)
fn parse_json_file<P: AsRef<Path>>(file_path: P) -> Result<CondaEnvironment> {
    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read JSON file: {:?}", file_path.as_ref()))?;
    
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON content from: {:?}", file_path.as_ref()))
}

/// Extracts the name, version, and build string from a package specification
pub fn parse_package_spec(spec: &str) -> Package {
    let mut package = Package {
        name: String::new(),
        version: None,
        build: None,
        channel: None,
        size: None,
        is_pinned: false,
        is_outdated: false,
        latest_version: None,
    };

    // Check for channel prefix (package::channel)
    if let Some(channel_idx) = spec.find("::") {
        package.channel = Some(spec[..channel_idx].to_string());
        let spec = &spec[channel_idx + 2..];
        
        // Parse the rest of the package spec
        parse_name_version_build(spec, &mut package);
    } else {
        // No channel, just parse name, version, build
        parse_name_version_build(spec, &mut package);
    }

    // Check if version is pinned (has an exact version)
    if package.version.is_some() {
        package.is_pinned = true;
    }

    package
}

/// Helper function to parse name, version, and build from a package spec
fn parse_name_version_build(spec: &str, package: &mut Package) {
    // Check for build string
    if let Some(build_idx) = spec.find('=') {
        if let Some(second_equal) = spec[build_idx + 1..].find('=') {
            let name_ver = &spec[..build_idx + 1 + second_equal];
            let build = &spec[build_idx + 1 + second_equal + 1..];
            package.build = Some(build.to_string());
            
            // Parse name and version
            if let Some(ver_idx) = name_ver.find('=') {
                package.name = name_ver[..ver_idx].to_string();
                package.version = Some(name_ver[ver_idx + 1..name_ver.len() - 1].to_string());
            }
        } else {
            // No build string, just name and version
            if let Some(ver_idx) = spec.find('=') {
                package.name = spec[..ver_idx].to_string();
                package.version = Some(spec[ver_idx + 1..].to_string());
            }
        }
    } else {
        // No version or build, just package name
        package.name = spec.to_string();
    }
}

/// Extract packages from a parsed conda environment
pub fn extract_packages(env: &crate::models::CondaEnvironment) -> Vec<crate::models::Package> {
    let mut packages = Vec::new();
    
    // Extract normal dependencies
    for dep in &env.dependencies {
        match dep {
            crate::models::Dependency::Simple(spec) => {
                let parts: Vec<&str> = spec.split('=').collect();
                let name = parts[0].trim().to_string();
                let version = if parts.len() > 1 { Some(parts[1].trim().to_string()) } else { None };
                let is_pinned = version.is_some();
                
                packages.push(crate::models::Package {
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
                        
                        packages.push(crate::models::Package {
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
    
    packages
}
