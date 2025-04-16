use std::collections::HashMap;
use conda_env_inspect::advanced_analysis::{self, AdvancedDependencyGraph};
use conda_env_inspect::models::Package;

#[test]
fn test_create_advanced_dependency_graph() {
    // Create packages with dependencies
    let packages = vec![
        Package {
            name: "numpy".to_string(),
            version: Some("1.19.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: Some(10485760),
            latest_version: None,
        },
        Package {
            name: "pandas".to_string(),
            version: Some("1.3.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: Some(20971520),
            latest_version: None,
        },
        Package {
            name: "matplotlib".to_string(),
            version: Some("3.5.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: Some(30485760),
            latest_version: None,
        },
    ];
    
    // Create a dependency map
    let mut dependency_map = HashMap::new();
    dependency_map.insert("pandas".to_string(), vec!["numpy>=1.18.0".to_string(), "python>=3.7".to_string()]);
    dependency_map.insert("matplotlib".to_string(), vec!["numpy>=1.19.0".to_string(), "python>=3.8".to_string()]);
    
    // Create the advanced dependency graph
    let graph = advanced_analysis::create_advanced_dependency_graph(&packages, &dependency_map);
    
    // Test the graph properties
    assert_eq!(graph.node_map.len(), 3);
    assert_eq!(graph.direct_deps.len(), 3);
    
    // Verify edges exist
    let pandas_node = graph.node_map.get("pandas").unwrap();
    let numpy_node = graph.node_map.get("numpy").unwrap();
    let matplotlib_node = graph.node_map.get("matplotlib").unwrap();
    
    // Check graph connections
    let edge_count = graph.graph.edge_count();
    assert!(edge_count >= 2, "Expected at least 2 edges for the dependencies");
}

#[test]
fn test_detect_conflicts() {
    // Create packages with conflicting dependencies
    let packages = vec![
        Package {
            name: "numpy".to_string(),
            version: Some("1.19.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: Some(10485760),
            latest_version: None,
        },
        Package {
            name: "package-a".to_string(),
            version: Some("1.0.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: Some(20971520),
            latest_version: None,
        },
        Package {
            name: "package-b".to_string(),
            version: Some("2.0.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: Some(30485760),
            latest_version: None,
        },
    ];
    
    // Create dependency map with a conflict (incompatible numpy requirements)
    let mut dependency_map = HashMap::new();
    dependency_map.insert("package-a".to_string(), vec!["numpy<1.18.0".to_string()]);
    dependency_map.insert("package-b".to_string(), vec!["numpy>=1.20.0".to_string()]);
    
    // Create the advanced dependency graph
    let graph = advanced_analysis::create_advanced_dependency_graph(&packages, &dependency_map);
    
    // Test that conflicts were detected
    assert!(!graph.conflicts.is_empty(), "Should detect version conflicts");
    
    // Verify the specific conflict
    let conflict = graph.conflicts.iter().find(|(pkg1, pkg2, _)| {
        (pkg1 == "package-a" && pkg2 == "package-b") || 
        (pkg1 == "package-b" && pkg2 == "package-a")
    });
    
    assert!(conflict.is_some(), "Should detect conflict between package-a and package-b");
    assert!(conflict.unwrap().2.contains("numpy"), "Conflict should involve numpy");
}

#[test]
fn test_find_vulnerabilities() {
    // Create packages with known vulnerabilities
    let packages = vec![
        Package {
            name: "log4j".to_string(),
            version: Some("2.0.1".to_string()),
            build: None,
            channel: Some("maven".to_string()),
            is_pinned: false,
            is_outdated: true,
            size: None,
            latest_version: Some("2.17.1".to_string()),
        },
        Package {
            name: "numpy".to_string(),
            version: Some("1.19.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: Some(10485760),
            latest_version: None,
        },
        Package {
            name: "tensorflow".to_string(),
            version: Some("2.4.0".to_string()),
            build: None,
            channel: Some("pip".to_string()),
            is_pinned: false,
            is_outdated: true,
            size: None,
            latest_version: Some("2.9.0".to_string()),
        },
    ];
    
    // Find vulnerabilities
    let vulnerabilities = advanced_analysis::find_vulnerabilities(&packages);
    
    // Check that vulnerabilities were found
    assert!(!vulnerabilities.is_empty(), "Should find vulnerabilities");
    
    // Check specific vulnerabilities
    let log4j_vuln = vulnerabilities.iter().find(|(name, _, _)| name == "log4j");
    assert!(log4j_vuln.is_some(), "Should find log4j vulnerability");
    
    let tensorflow_vuln = vulnerabilities.iter().find(|(name, _, _)| name == "tensorflow");
    assert!(tensorflow_vuln.is_some(), "Should find tensorflow vulnerability");
}

#[test]
fn test_find_vulnerabilities_with_outdated_packages() {
    // Create packages with known vulnerabilities and outdated packages
    let packages = vec![
        Package {
            name: "numpy".to_string(),
            version: Some("1.19.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: true,
            size: Some(10485760),
            latest_version: Some("1.24.3".to_string()),
        },
        Package {
            name: "pandas".to_string(),
            version: Some("1.0.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: true,
            size: Some(20971520),
            latest_version: Some("2.1.0".to_string()),
        },
        Package {
            name: "django".to_string(),
            version: Some("2.0.0".to_string()),
            build: None,
            channel: Some("pip".to_string()),
            is_pinned: false,
            is_outdated: true,
            size: None,
            latest_version: Some("4.2.0".to_string()),
        },
        Package {
            name: "requests".to_string(),
            version: Some("2.2.0".to_string()),
            build: None,
            channel: Some("pip".to_string()),
            is_pinned: false,
            is_outdated: true,
            size: None,
            latest_version: Some("2.30.0".to_string()),
        },
        Package {
            name: "safe-package".to_string(),
            version: Some("1.0.0".to_string()),
            build: None,
            channel: Some("pip".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: None,
            latest_version: Some("1.0.1".to_string()),
        },
    ];
    
    // Find vulnerabilities
    let vulnerabilities = advanced_analysis::find_vulnerabilities(&packages);
    
    // Check that vulnerabilities were found
    assert!(!vulnerabilities.is_empty(), "Should find vulnerabilities");
    
    // Check for known specific vulnerabilities
    let numpy_vuln = vulnerabilities.iter().find(|(name, _, _)| name == "numpy");
    assert!(numpy_vuln.is_some(), "Should find numpy vulnerability");
    
    let django_vuln = vulnerabilities.iter().find(|(name, _, _)| name == "django");
    assert!(django_vuln.is_some(), "Should find django vulnerability");
    
    let requests_vuln = vulnerabilities.iter().find(|(name, _, _)| name == "requests");
    assert!(requests_vuln.is_some(), "Should find requests vulnerability");
    
    // Check for version gap vulnerabilities
    let pandas_vuln = vulnerabilities.iter().find(|(name, _, desc)| 
        name == "pandas" && desc.contains("significantly outdated"));
    assert!(pandas_vuln.is_some(), "Should detect pandas as vulnerable due to significant version gap");
    
    // Verify that safe packages are not flagged
    let safe_vuln = vulnerabilities.iter().find(|(name, _, _)| name == "safe-package");
    assert!(safe_vuln.is_none(), "Should not flag up-to-date packages as vulnerable");
    
    // Verify vulnerability message format
    if let Some((_, version, desc)) = pandas_vuln {
        assert_eq!(version, "1.0.0", "Should include the correct version");
        assert!(desc.contains("current: 1.0.0"), "Should include current version in description");
        assert!(desc.contains("latest: 2.1.0"), "Should include latest version in description");
    }
} 