use conda_env_inspect::advanced_analysis;
use conda_env_inspect::models::Package;

fn main() {
    println!("Testing vulnerability detection...");
    
    // Create test data
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
    
    // Run the vulnerability detection
    let vulnerabilities = advanced_analysis::find_vulnerabilities(&packages);
    
    // Output the results
    println!("\nVulnerabilities found: {}", vulnerabilities.len());
    
    for (idx, (name, version, description)) in vulnerabilities.iter().enumerate() {
        println!("{}: {} {} - {}", idx + 1, name, version, description);
    }
    
    // Validate results
    let expected_vulnerable_packages = vec!["numpy", "django", "requests", "log4j", "pandas"];
    for pkg in &expected_vulnerable_packages {
        let found = vulnerabilities.iter().any(|(name, _, _)| name == pkg);
        println!("Expected vulnerable package '{}' found: {}", pkg, found);
        assert!(found, "Failed to find vulnerability for {}", pkg);
    }
    
    // Check safe packages are not flagged
    let safe_found = vulnerabilities.iter().any(|(name, _, _)| name == "safe-package");
    println!("Safe package incorrectly flagged: {}", safe_found);
    assert!(!safe_found, "Safe package should not be flagged as vulnerable");
    
    println!("\nAll tests passed successfully!");
} 