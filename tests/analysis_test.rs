use std::collections::HashMap;
use conda_env_inspect::analysis;
use conda_env_inspect::models::{Package, CondaEnvironment};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_generate_recommendations() {
    // Create a set of packages for testing
    let packages = vec![
        Package {
            name: "numpy".to_string(),
            version: Some("1.19.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: true,
            is_outdated: true,
            size: Some(10485760),
            latest_version: Some("1.23.5".to_string()),
        },
        Package {
            name: "pandas".to_string(),
            version: Some("1.3.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: Some(20971520),
            latest_version: Some("1.3.0".to_string()),
        },
        Package {
            name: "tensorflow".to_string(),
            version: Some("2.4.0".to_string()),
            build: None,
            channel: Some("pip".to_string()),
            is_pinned: false,
            is_outdated: true,
            size: Some(157286400),
            latest_version: Some("2.9.0".to_string()),
        },
    ];
    
    // Get recommendations
    let recommendations = analysis::generate_recommendations(&packages, true);
    
    // Validate recommendations
    assert!(!recommendations.is_empty());
    assert!(recommendations.iter().any(|r| r.contains("numpy")));
    assert!(recommendations.iter().any(|r| r.contains("tensorflow")));
    assert!(recommendations.iter().any(|r| r.contains("outdated")));
}

#[test]
fn test_get_real_package_dependencies() {
    // Create a temporary conda environment file to test real parsing
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("environment.yml");
    
    // Create environment with known dependencies
    let yaml_content = r#"name: test-env
channels:
  - conda-forge
  - defaults
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas>=1.3.0
  - matplotlib=3.5.0
  - scipy>=1.7.0
  - scikit-learn
"#;
    
    let mut file = File::create(&file_path).unwrap();
    file.write_all(yaml_content.as_bytes()).unwrap();
    
    // Parse the environment file to get packages
    let env = conda_env_inspect::parsers::parse_environment_file(&file_path).unwrap();
    
    // Now get the real dependencies
    let dependencies = analysis::get_real_package_dependencies(&env.packages);
    
    // Validate the dependencies
    assert!(!dependencies.is_empty(), "Dependencies map should not be empty");
    
    // Check for some typical dependencies
    if let Some(numpy_deps) = dependencies.get("numpy") {
        // NumPy typically depends on Python
        assert!(numpy_deps.iter().any(|dep| dep.contains("python")), 
                "NumPy should depend on Python");
    }
    
    if let Some(pandas_deps) = dependencies.get("pandas") {
        // Pandas typically depends on NumPy
        assert!(pandas_deps.iter().any(|dep| dep.contains("numpy")), 
                "Pandas should depend on NumPy");
    }
    
    if let Some(matplotlib_deps) = dependencies.get("matplotlib") {
        // Matplotlib typically has multiple dependencies
        assert!(matplotlib_deps.len() > 1, 
                "Matplotlib should have multiple dependencies");
    }
}

#[test]
fn test_analyze_packages() {
    // Create a set of packages for testing
    let packages = vec![
        Package {
            name: "numpy".to_string(),
            version: Some("1.19.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: true,
            is_outdated: true,
            size: Some(10485760),
            latest_version: Some("1.23.5".to_string()),
        },
        Package {
            name: "pandas".to_string(),
            version: Some("1.3.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: Some(20971520),
            latest_version: Some("1.3.0".to_string()),
        },
    ];
    
    // Analyze packages
    let (pinned_count, outdated_count, total_size) = analysis::analyze_packages(&packages);
    
    // Validate analysis
    assert_eq!(pinned_count, 1);
    assert_eq!(outdated_count, 1);
    assert_eq!(total_size, Some(31457280)); // 10MB + 20MB
} 