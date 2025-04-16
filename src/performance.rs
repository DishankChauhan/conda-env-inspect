use cached::proc_macro::cached;
use log::{debug, info};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::conda_api::PackageInfo;
use crate::models::Package;

/// Enriches package information in parallel using rayon
pub fn enrich_packages_parallel(packages: &mut Vec<Package>) -> anyhow::Result<()> {
    info!("Enriching {} packages in parallel", packages.len());
    
    // Store package information for parallel iteration
    let package_names: Vec<(usize, String, Option<String>)> = packages.iter().enumerate()
        .map(|(i, p)| (i, p.name.clone(), p.channel.clone()))
        .collect();
    
    // Process packages in parallel, using a lock to update the original packages
    let packages_ref = Arc::new(Mutex::new(packages));
    
    package_names.par_iter()
        .for_each(|(i, name, channel)| {
            // Skip packages without a name or pip packages
            if name.is_empty() || name.contains('>') {
                debug!("Skipping package: {}", name);
                return;
            }
            
            debug!("Enriching package {}/{}: {}", i + 1, package_names.len(), name);
            
            // Get package info using cached function
            match get_package_info_cached(name, channel.as_deref()) {
                Ok(info) => {
                    // Lock the packages for mutation
                    if let Ok(mut packages_guard) = packages_ref.lock() {
                        if let Some(pkg) = (**packages_guard).get_mut(*i) {
                            // Update the package with the retrieved information
                            update_package_with_info(pkg, &info);
                            debug!("Successfully enriched {}", name);
                        }
                    }
                },
                Err(e) => {
                    debug!("Failed to enrich {}: {}", name, e);
                }
            }
        });
    
    info!("Parallel package enrichment complete");
    Ok(())
}

/// Updates a package with information from PackageInfo
fn update_package_with_info(package: &mut Package, info: &PackageInfo) {
    // Check if outdated using semantic versioning
    if let Some(version) = &package.version {
        if let (Some(current), Some(latest)) = (
            parse_version_cached(version),
            parse_version_cached(&info.latest_version)
        ) {
            package.is_outdated = current < latest;
        } else {
            // Fallback to string comparison
            package.is_outdated = version != &info.latest_version;
        }
    }
    
    // Set latest version
    package.latest_version = Some(info.latest_version.clone());
    
    // Set package size
    package.size = info.size;
}

/// Cached version of the package info retrieval
#[cached(
    time = 3600, // Cache for 1 hour
    key = "String",
    convert = r#"{ format!("{}:{}", name, channel.unwrap_or("conda-forge")) }"#,
    result = true
)]
fn get_package_info_cached(name: &str, channel: Option<&str>) -> anyhow::Result<PackageInfo> {
    crate::conda_api::get_package_info(name, channel)
}

/// Parse a version string
fn parse_version_cached(version_str: &str) -> Option<semver::Version> {
    let normalized = normalize_conda_version(version_str);
    match semver::Version::parse(&normalized) {
        Ok(version) => Some(version),
        Err(_) => None
    }
}

/// Normalize conda version string to semver compatibility (cached version)
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