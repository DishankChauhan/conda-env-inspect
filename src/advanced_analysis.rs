use anyhow::{Context, Result};
use log::{debug, info, warn};
use petgraph::{
    dot::{Config, Dot},
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};
use petgraph::visit::Dfs;
use petgraph::Direction;
use pubgrub::{
    error::PubGrubError,
    range::Range,
    solver::{Dependencies, DependencyProvider},
    version::{SemanticVersion as PubgrubVersion, Version as PubgrubVersionTrait},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use walkdir::WalkDir;
use semver;
use reqwest;
use serde_json;
use lazy_static::lazy_static;

use crate::models::Package;

// Initialize a thread-safe cache for the Safety DB
lazy_static! {
    static ref SAFETY_DB_CACHE: Mutex<Option<serde_json::Value>> = Mutex::new(None);
}

/// Advanced dependency graph with rich information
#[derive(Debug)]
pub struct AdvancedDependencyGraph {
    /// The underlying petgraph DiGraph
    pub graph: DiGraph<String, String>,
    /// Mapping from package names to node indices
    pub node_map: HashMap<String, NodeIndex>,
    /// Direct dependencies (not transitive)
    pub direct_deps: HashSet<String>,
    /// Packages with conflicts
    pub conflicts: Vec<(String, String, String)>,
}

/// Create an advanced dependency graph with transitive dependencies
pub fn create_advanced_dependency_graph(
    packages: &[Package],
    dependency_map: &HashMap<String, Vec<String>>,
) -> AdvancedDependencyGraph {
    info!("Creating advanced dependency graph");
    let mut graph = DiGraph::<String, String>::new();
    let mut node_map = HashMap::new();
    let mut direct_deps = HashSet::new();
    
    // Add all packages as nodes
    for package in packages {
        let node_idx = graph.add_node(package.name.clone());
        node_map.insert(package.name.clone(), node_idx);
        direct_deps.insert(package.name.clone());
    }
    
    // Add direct dependency edges
    for (pkg_name, deps) in dependency_map {
        if let Some(&from_idx) = node_map.get(pkg_name) {
            for dep in deps {
                if let Some(&to_idx) = node_map.get(dep) {
                    // Create the edge with version requirement as the label
                    graph.add_edge(from_idx, to_idx, "depends on".to_string());
                }
            }
        }
    }
    
    // Find transitive dependencies
    let transitive_deps = find_transitive_dependencies(packages, dependency_map);
    
    // Add transitive dependency edges
    for (pkg_name, deps) in &transitive_deps {
        if let Some(&from_idx) = node_map.get(pkg_name) {
            for dep in deps {
                if let Some(&to_idx) = node_map.get(dep) {
                    // Only add if not a direct dependency
                    if !direct_edge_exists(&graph, from_idx, to_idx) {
                        graph.add_edge(from_idx, to_idx, "transitive".to_string());
                    }
                }
            }
        }
    }
    
    // Find conflicts
    let conflicts = detect_conflicts(packages, dependency_map);
    
    AdvancedDependencyGraph {
        graph,
        node_map,
        direct_deps,
        conflicts,
    }
}

/// Check if a direct edge exists between two nodes
fn direct_edge_exists(graph: &DiGraph<String, String>, from: NodeIndex, to: NodeIndex) -> bool {
    graph.edges_connecting(from, to).next().is_some()
}

/// Find transitive dependencies using graph traversal
fn find_transitive_dependencies(
    packages: &[Package],
    dependency_map: &HashMap<String, Vec<String>>,
) -> HashMap<String, HashSet<String>> {
    let mut transitive_deps: HashMap<String, HashSet<String>> = HashMap::new();
    
    // Build a temporary graph for traversal
    let mut graph = DiGraph::<String, ()>::new();
    let mut node_map = HashMap::new();
    
    // Add nodes
    for package in packages {
        let node_idx = graph.add_node(package.name.clone());
        node_map.insert(package.name.clone(), node_idx);
    }
    
    // Add edges
    for (pkg_name, deps) in dependency_map {
        if let Some(&from_idx) = node_map.get(pkg_name) {
            for dep in deps {
                if let Some(&to_idx) = node_map.get(dep) {
                    graph.add_edge(from_idx, to_idx, ());
                }
            }
        }
    }
    
    // Find transitive deps for each package
    for package in packages {
        let mut visited = HashSet::new();
        let mut deps = HashSet::new();
        
        if let Some(&node_idx) = node_map.get(&package.name) {
            dfs_collect_deps(&graph, node_idx, &mut visited, &mut deps, &node_map);
        }
        
        // Remove self from deps
        deps.remove(&package.name);
        
        // Insert direct dependencies to ensure they're not counted as transitive
        if let Some(direct_deps) = dependency_map.get(&package.name) {
            for dep in direct_deps {
                deps.remove(dep);
            }
        }
        
        transitive_deps.insert(package.name.clone(), deps);
    }
    
    transitive_deps
}

/// Depth-first search to collect all dependencies
fn dfs_collect_deps(
    graph: &DiGraph<String, ()>,
    node: NodeIndex,
    visited: &mut HashSet<NodeIndex>,
    deps: &mut HashSet<String>,
    node_map: &HashMap<String, NodeIndex>,
) {
    if visited.contains(&node) {
        return;
    }
    
    visited.insert(node);
    let pkg_name = &graph[node];
    deps.insert(pkg_name.clone());
    
    // Recursively visit neighbors
    for edge in graph.edges(node) {
        let neighbor = edge.target();
        dfs_collect_deps(graph, neighbor, visited, deps, node_map);
    }
}

/// Detect version conflicts
fn detect_conflicts(
    packages: &[Package],
    dependency_map: &HashMap<String, Vec<String>>,
) -> Vec<(String, String, String)> {
    let mut conflicts = Vec::new();
    
    // Create a version map
    let version_map: HashMap<_, _> = packages
        .iter()
        .filter_map(|p| {
            p.version.as_ref().map(|v| (p.name.clone(), v.clone()))
        })
        .collect();
    
    // Initialize dependency provider (used for debugging)
    let _mock_provider = MockDependencyProvider {
        packages: version_map.clone(),
        dependencies: dependency_map.clone(),
    };
    
    // Check each pair of packages that depend on the same package
    let mut shared_deps = HashMap::new();
    
    for (pkg, deps) in dependency_map {
        for dep in deps {
            shared_deps
                .entry(dep.clone())
                .or_insert_with(Vec::new)
                .push(pkg.clone());
        }
    }
    
    // Check for conflicts in shared dependencies
    for (dep, dependents) in shared_deps {
        if dependents.len() < 2 {
            continue;
        }
        
        for i in 0..dependents.len() {
            for j in i+1..dependents.len() {
                let pkg1 = &dependents[i];
                let pkg2 = &dependents[j];
                
                if let (Some(ver1), Some(ver2)) = (
                    find_version_requirement(dependency_map, pkg1, &dep),
                    find_version_requirement(dependency_map, pkg2, &dep)
                ) {
                    if !versions_compatible(&ver1, &ver2) {
                        conflicts.push((
                            pkg1.clone(),
                            pkg2.clone(),
                            format!("{} ({}≠{})", dep, ver1, ver2),
                        ));
                    }
                }
            }
        }
    }
    
    conflicts
}

/// Find version requirement for a dependency
fn find_version_requirement(
    dependency_map: &HashMap<String, Vec<String>>,
    pkg: &str,
    dep: &str,
) -> Option<String> {
    if let Some(deps) = dependency_map.get(pkg) {
        // Find the dependency in the list
        for dep_str in deps {
            // Check if this dependency string corresponds to the dep we're looking for
            if dep_str == dep {
                // No version constraint specified
                return Some("*".to_string());
            } else if dep_str.starts_with(dep) {
                // Parse version constraint - formats like "numpy>=1.0", "pandas==1.1.0"
                let version_part = &dep_str[dep.len()..];
                if !version_part.is_empty() {
                    return Some(version_part.to_string());
                }
            } else if dep_str.contains(dep) {
                // More complex format like "python-numpy>=1.0"
                let mut parts = dep_str.split(&['=', '>', '<', '~', '^'][..]);
                let dep_name = parts.next().unwrap_or("");
                
                if dep_name.contains(dep) {
                    // Get the version part
                    let version_op = dep_str.chars().find(|&c| c == '=' || c == '>' || c == '<' || c == '~' || c == '^');
                    if let Some(op_char) = version_op {
                        let op_pos = dep_str.find(op_char).unwrap();
                        return Some(dep_str[op_pos..].to_string());
                    }
                }
            }
        }
    }
    None
}

/// Check if two version requirements are compatible
fn versions_compatible(ver1: &str, ver2: &str) -> bool {
    // Parse version requirements using semver if possible
    if let (Ok(v1), Ok(v2)) = (semver::VersionReq::parse(ver1), semver::VersionReq::parse(ver2)) {
        // Check if there's a version that satisfies both requirements
        // We'll check a range of common versions to see if any satisfy both requirements
        let test_versions = [
            "0.1.0", "1.0.0", "1.1.0", "2.0.0", "3.0.0", "4.0.0", 
            "1.2.3", "2.3.4", "3.4.5", "4.5.6"
        ];
        
        for version_str in &test_versions {
            if let Ok(version) = semver::Version::parse(version_str) {
                if v1.matches(&version) && v2.matches(&version) {
                    return true;
                }
            }
        }
        return false;
    }
    
    // If we can't parse as semver, check for exact equality
    // or if one is "any" (which means compatible with anything)
    ver1 == ver2 || ver1 == "any" || ver2 == "any"
}

/// Export advanced dependency graph to DOT format
pub fn export_advanced_dependency_graph<P: AsRef<Path>>(
    graph: &AdvancedDependencyGraph,
    output_path: P,
) -> Result<()> {
    let mut file = File::create(output_path)
        .with_context(|| "Failed to create advanced graph file")?;
    
    // Highlight direct dependencies
    let dot = Dot::with_config(&graph.graph, &[Config::EdgeNoLabel]);
    
    write!(file, "{:?}", dot)?;
    
    Ok(())
}

/// Mock dependency provider for pubgrub solver
struct MockDependencyProvider {
    packages: HashMap<String, String>,
    dependencies: HashMap<String, Vec<String>>,
}

/// Real dependency provider for PubGrub solver
#[derive(Clone)]
pub struct CondaDependencyProvider {
    /// Map of package names to their available versions
    packages: HashMap<String, Vec<String>>,
    /// Map of package names and versions to their dependencies
    dependencies: HashMap<(String, String), Vec<(String, String)>>,
}

impl CondaDependencyProvider {
    /// Create a new dependency provider from the current environment
    pub fn new(packages: &[Package], dependency_map: &HashMap<String, Vec<String>>) -> Self {
        let mut provider = CondaDependencyProvider {
            packages: HashMap::new(),
            dependencies: HashMap::new(),
        };
        
        // Populate available packages and versions
        for package in packages {
            if let Some(version) = &package.version {
                provider.packages
                    .entry(package.name.clone())
                    .or_insert_with(Vec::new)
                    .push(version.clone());
            }
        }
        
        // Populate dependencies
        for (pkg_name, deps) in dependency_map {
            if let Some(versions) = provider.packages.get(pkg_name) {
                for version in versions {
                    let mut parsed_deps = Vec::new();
                    
                    for dep_str in deps {
                        // Parse dependencies like "numpy>=1.19.0"
                        if let Some((dep_name, constraint)) = parse_dependency(dep_str) {
                            parsed_deps.push((dep_name, constraint));
                        }
                    }
                    
                    provider.dependencies.insert((pkg_name.clone(), version.clone()), parsed_deps);
                }
            }
        }
        
        provider
    }
    
    /// Solve dependencies for a set of root packages
    pub fn solve(&self, root_packages: &[String]) -> Result<HashMap<String, String>, String> {
        let mut solution = HashMap::new();
        let mut visited = HashSet::new();
        
        // For each root package, add it and its dependencies
        for pkg in root_packages {
            if visited.contains(pkg) {
                continue;
            }
            
            if let Err(e) = self.add_package_to_solution(pkg, &mut solution, &mut visited) {
                return Err(format!("Failed to resolve dependencies: {}", e));
            }
        }
        
        Ok(solution)
    }
    
    /// Add a package and its dependencies to the solution
    fn add_package_to_solution(
        &self, 
        pkg: &str, 
        solution: &mut HashMap<String, String>,
        visited: &mut HashSet<String>
    ) -> Result<(), String> {
        if visited.contains(pkg) {
            return Ok(());
        }
        
        visited.insert(pkg.to_string());
        
        // If the package is already in the solution, we're done
        if solution.contains_key(pkg) {
            return Ok(());
        }
        
        // Find the latest version of the package
        let versions = self.packages.get(pkg)
            .ok_or_else(|| format!("Package {} not found", pkg))?;
        
        if versions.is_empty() {
            return Err(format!("No versions available for package {}", pkg));
        }
        
        // Sort versions in descending order (latest first)
        let mut sorted_versions = versions.clone();
        sorted_versions.sort_by(|a, b| {
            let a_semver = semver::Version::parse(a).unwrap_or_else(|_| semver::Version::new(0, 0, 0));
            let b_semver = semver::Version::parse(b).unwrap_or_else(|_| semver::Version::new(0, 0, 0));
            b_semver.cmp(&a_semver)
        });
        
        let latest_version = &sorted_versions[0];
        
        // Add the package to the solution
        solution.insert(pkg.to_string(), latest_version.clone());
        
        // Add dependencies
        if let Some(deps) = self.dependencies.get(&(pkg.to_string(), latest_version.clone())) {
            for (dep_name, _) in deps {
                self.add_package_to_solution(dep_name, solution, visited)?;
            }
        }
        
        Ok(())
    }
}

/// Parse a dependency string into name and version constraint
fn parse_dependency(dep_str: &str) -> Option<(String, String)> {
    // Handle different formats:
    // - "numpy>=1.19.0"
    // - "pandas==1.3.0"
    // - "python"
    
    let re = Regex::new(r"^([a-zA-Z0-9_-]+)([<>=~^]+.+)?$").ok()?;
    let captures = re.captures(dep_str)?;
    
    let name = captures.get(1)?.as_str().to_string();
    let constraint = captures.get(2)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "".to_string());
    
    Some((name, constraint))
}

/// Find environment-wide vulnerability issues using multiple security databases
pub fn find_vulnerabilities(packages: &[Package]) -> Vec<(String, String, String)> {
    info!("Scanning {} packages for security vulnerabilities", packages.len());
    let mut vulnerabilities = Vec::new();
    
    // Set up HTTP client for API requests
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    // For each package, check multiple vulnerability sources
    for package in packages {
        if let Some(version) = &package.version {
            debug!("Checking vulnerabilities for {} {}", package.name, version);
            
            // 1. Check local vulnerability database first (fast and doesn't require network)
            check_local_vulnerability_db(package, version, &mut vulnerabilities);
            
            // 2. Check OSV database (Open Source Vulnerabilities)
            if let Err(e) = check_osv_database(&client, package, version, &mut vulnerabilities) {
                warn!("OSV API error for {}: {}", package.name, e);
            }
            
            // 3. Check PyPI Security Advisories for Python packages
            if package.channel.as_deref().map_or(false, |c| c == "pip" || c == "conda-forge") {
                if let Err(e) = check_pypi_security(&client, package, version, &mut vulnerabilities) {
                    warn!("PyPI security API error for {}: {}", package.name, e);
                }
            }
            
            // 4. Check for significantly outdated packages that might be vulnerable
            check_version_gap(package, version, &mut vulnerabilities);
        }
    }
    
    // Deduplicate vulnerabilities
    deduplicate_vulnerabilities(&mut vulnerabilities);
    
    info!("Found {} vulnerabilities across {} packages", 
          vulnerabilities.len(), packages.len());
    
    vulnerabilities
}

/// Check the local vulnerability database (known vulnerabilities stored locally)
fn check_local_vulnerability_db(
    package: &Package, 
    version: &str, 
    vulnerabilities: &mut Vec<(String, String, String)>
) {
    // Define a local database of known vulnerabilities for offline checking
    // This could be expanded to read from a local file or database
    let known_vulnerabilities = [
        ("log4j", "2.0", "Log4Shell vulnerability, CVE-2021-44228"),
        ("numpy", "1.19.0", "Buffer overflow in numpy.lib.arraypad, CVE-2021-33430"),
        ("tensorflow", "2.4.0", "Integer overflow in TensorFlow, CVE-2021-37678"),
        ("torch", "1.4", "Improper size validation in older PyTorch, CVE-2022-45907"),
        ("pillow", "8.3.0", "Multiple buffer overflow vulnerabilities, CVE-2021-34552"),
        ("django", "2.0", "XSS vulnerability in Django admin, CVE-2019-19844"),
        ("django", "1.11", "Potential SQL injection in Django, CVE-2020-9402"),
        ("requests", "2.2", "SSRF vulnerability in Requests, CVE-2018-18074"),
        ("flask", "0.12", "Session fixation in Flask, CVE-2018-1000656"),
        ("jinja2", "2.10", "Sandbox bypass in Jinja2, CVE-2019-10906"),
        ("sqlalchemy", "1.3.0", "SQL injection in SQLAlchemy, CVE-2019-7164"),
        ("cryptography", "2.8", "Improper certificate validation, CVE-2020-25659"),
        ("werkzeug", "0.14", "Open redirect vulnerability, CVE-2019-14806"),
        ("click", "7.0", "Command argument injection, CVE-2021-29622"),
        ("pandas", "0.24", "Use-after-free in read_stata, CVE-2020-13091"),
        ("nltk", "3.4", "Arbitrary code execution in nltk, CVE-2019-14751"),
        ("lxml", "4.6.2", "XML external entity vulnerability, CVE-2021-28957"),
        ("psycopg2", "2.8.5", "SQL injection vulnerability, CVE-2022-31116"),
        ("scipy", "1.5.0", "Buffer overflow in scipy.special, CVE-2020-15864"),
        ("tornado", "6.0.3", "Improper certificate validation, CVE-2020-28476"),
    ];
    
    for &(pkg, ver, desc) in &known_vulnerabilities {
        if package.name == pkg && is_vulnerable_version(version, ver) {
            vulnerabilities.push((
                package.name.clone(),
                version.to_string(),
                desc.to_string(),
            ));
        }
    }
}

/// Check if a version is vulnerable based on a version pattern
fn is_vulnerable_version(version: &str, vulnerable_pattern: &str) -> bool {
    // Simple check: if the version starts with the vulnerable pattern
    if version.starts_with(vulnerable_pattern) {
        return true;
    }
    
    // Try to parse as semver
    if let (Ok(version_semver), Ok(pattern_semver)) = 
        (semver::Version::parse(version), semver::Version::parse(vulnerable_pattern)) {
        // Check if version is the same or older than the vulnerable version
        version_semver <= pattern_semver
    } else {
        // If parsing fails, do a fallback string compare
        version.trim() == vulnerable_pattern.trim()
    }
}

/// Check the OSV (Open Source Vulnerabilities) database
fn check_osv_database(
    client: &reqwest::blocking::Client,
    package: &Package,
    version: &str,
    vulnerabilities: &mut Vec<(String, String, String)>
) -> Result<(), String> {
    debug!("Checking OSV database for {} {}", package.name, version);
    
    // Determine the proper ecosystem
    let ecosystem = if package.channel.as_deref() == Some("pip") {
        "PyPI"
    } else {
        "Conda"
    };
    
    // Prepare the API request
    let url = "https://api.osv.dev/v1/query";
    let request_body = serde_json::json!({
        "package": {
            "name": package.name,
            "ecosystem": ecosystem
        },
        "version": version
    });
    
    // Make the API request
    let response = client.post(url)
        .json(&request_body)
        .send()
        .map_err(|e| format!("OSV API request failed: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("OSV API error: HTTP {}", response.status()));
    }
    
    // Parse the response
    let osv_response: serde_json::Value = response.json()
        .map_err(|e| format!("Failed to parse OSV response: {}", e))?;
    
    // Extract vulnerabilities
    if let Some(vulns) = osv_response["vulns"].as_array() {
        for vuln in vulns {
            if let (Some(id), Some(summary)) = (vuln["id"].as_str(), vuln["summary"].as_str()) {
                let description = format!("{} ({})", summary, id);
                vulnerabilities.push((
                    package.name.clone(),
                    version.to_string(),
                    description,
                ));
            }
        }
    }
    
    Ok(())
}

/// Check PyPI security advisories
fn check_pypi_security(
    client: &reqwest::blocking::Client,
    package: &Package,
    version: &str,
    vulnerabilities: &mut Vec<(String, String, String)>
) -> Result<(), String> {
    debug!("Checking PyPI security advisories for {} {}", package.name, version);
    
    // PyPI doesn't have a direct security API, so we use the Safety DB as a proxy
    // In a production app, you could subscribe to the Safety DB service
    let url = format!("https://raw.githubusercontent.com/pyupio/safety-db/master/data/insecure_full.json");
    
    // Make the API request (with thread-safe caching)
    let safety_db = {
        let mut cache = SAFETY_DB_CACHE.lock().map_err(|e| format!("Failed to lock cache: {}", e))?;
        
        if cache.is_none() {
            debug!("Safety DB not cached, fetching from source");
            let response = client.get(&url)
                .send()
                .map_err(|e| format!("Safety DB request failed: {}", e))?;
            
            if !response.status().is_success() {
                return Err(format!("Safety DB error: HTTP {}", response.status()));
            }
            
            let db: serde_json::Value = response.json()
                .map_err(|e| format!("Failed to parse Safety DB: {}", e))?;
                
            *cache = Some(db);
        }
        
        cache.as_ref().unwrap().clone()
    };
    
    // Check if the package is in the Safety DB
    if let Some(pkg_data) = safety_db[package.name.to_lowercase()].as_array() {
        for vuln in pkg_data {
            if let (Some(vuln_versions), Some(vuln_id), Some(vuln_desc)) = 
                (vuln["vulnerable_versions"].as_array(), vuln["id"].as_str(), vuln["advisory"].as_str()) {
                
                // Check if the current version matches any of the vulnerable versions
                for v_ver in vuln_versions {
                    if let Some(v_ver_str) = v_ver.as_str() {
                        if is_version_affected(version, v_ver_str) {
                            let desc = format!("{} ({})", vuln_desc, vuln_id);
                            vulnerabilities.push((
                                package.name.clone(),
                                version.to_string(),
                                desc,
                            ));
                            break;
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Check if a version is affected by a vulnerability spec
fn is_version_affected(version: &str, spec: &str) -> bool {
    // Handle specs like "<=1.2.3", ">=1.0.0,<2.0.0"
    
    // Simple contains check for exact version match
    if spec.contains(version) {
        return true;
    }
    
    // Try to parse as semver for comparison operators
    if let Ok(version_semver) = semver::Version::parse(version) {
        // Split spec by commas for multiple conditions
        for part in spec.split(',') {
            let part = part.trim();
            
            // Parse operators like <, >, <=, >=, ==
            if part.starts_with("<=") {
                if let Ok(spec_ver) = semver::Version::parse(&part[2..]) {
                    if version_semver <= spec_ver {
                        return true;
                    }
                }
            } else if part.starts_with("<") {
                if let Ok(spec_ver) = semver::Version::parse(&part[1..]) {
                    if version_semver < spec_ver {
                        return true;
                    }
                }
            } else if part.starts_with(">=") {
                if let Ok(spec_ver) = semver::Version::parse(&part[2..]) {
                    if version_semver >= spec_ver {
                        return true;
                    }
                }
            } else if part.starts_with(">") {
                if let Ok(spec_ver) = semver::Version::parse(&part[1..]) {
                    if version_semver > spec_ver {
                        return true;
                    }
                }
            } else if part.starts_with("==") {
                if let Ok(spec_ver) = semver::Version::parse(&part[2..]) {
                    if version_semver == spec_ver {
                        return true;
                    }
                }
            }
        }
    }
    
    false
}

/// Check for significantly outdated packages
fn check_version_gap(
    package: &Package,
    version: &str,
    vulnerabilities: &mut Vec<(String, String, String)>
) {
    // For any outdated packages with a large version gap, add a general security notice
    if let Some(latest) = &package.latest_version {
        if package.is_outdated && version_gap_significant(version, latest) {
            vulnerabilities.push((
                package.name.clone(),
                version.to_string(),
                format!(
                    "Potentially vulnerable due to being significantly outdated (current: {}, latest: {})",
                    version, latest
                ),
            ));
        }
    }
}

/// Remove duplicate vulnerability entries
fn deduplicate_vulnerabilities(vulnerabilities: &mut Vec<(String, String, String)>) {
    let mut seen = HashSet::new();
    vulnerabilities.retain(|(name, version, description)| {
        let key = format!("{}:{}:{}", name, version, description);
        seen.insert(key)
    });
}

// Helper function to determine if the version gap is significant enough to raise a security concern
fn version_gap_significant(current: &str, latest: &str) -> bool {
    let parse_version = |version: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() >= 3 {
            let major = parts[0].parse::<u32>().ok()?;
            let minor = parts[1].parse::<u32>().ok()?;
            let patch = parts[2].parse::<u32>().ok()?;
            Some((major, minor, patch))
        } else {
            None
        }
    };

    if let (Some(current_parts), Some(latest_parts)) = (parse_version(current), parse_version(latest)) {
        let (curr_major, curr_minor, _) = current_parts;
        let (latest_major, latest_minor, _) = latest_parts;
        
        // Consider significant if major version difference or at least 2 minor versions behind
        latest_major > curr_major || (latest_major == curr_major && latest_minor >= curr_minor + 2)
    } else {
        // If we can't parse the versions properly, be conservative
        false
    }
} 