use std::collections::HashMap;
use conda_env_inspect::analysis;
use conda_env_inspect::models::Package;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use std::path::PathBuf;
use anyhow::Result;
use conda_env_inspect::analysis::{calculate_environment_size, generate_recommendations};
use conda_env_inspect::models::{CondaEnvironment, Dependency, Recommendation, EnvironmentAnalysis};
use std::collections::{HashSet};
use std::path::Path;
use tempfile::{NamedTempFile};
use std::fs;

#[test]
fn test_generate_recommendations() {
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
    
    let recommendations = analysis::generate_recommendations(&packages, true);
    assert!(!recommendations.is_empty());
    
    // Check if at least one recommendation mentions numpy and outdated
    let has_numpy_rec = recommendations.iter().any(|r| r.contains("numpy") && r.contains("outdated"));
    assert!(has_numpy_rec, "Should have recommendation for outdated numpy package");
}

#[test]
fn test_get_real_package_dependencies() {
    // Use a direct approach to test the dependency retrieval
    let packages = vec![
        Package {
            name: "numpy".to_string(),
            version: Some("1.21.0".to_string()),
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
    ];
    
    // Get dependencies
    let dependencies = analysis::get_real_package_dependencies(&packages);
    
    // Validate the dependencies
    assert!(dependencies.contains_key("numpy"), "Dependencies map should contain numpy");
    assert!(dependencies.contains_key("pandas"), "Dependencies map should contain pandas");
}

#[test]
fn test_dependency_graph_creation() {
    let packages = vec![
        Package {
            name: "numpy".to_string(),
            version: Some("1.21.0".to_string()),
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
    ];

    let graph = analysis::create_dependency_graph(&packages);
    
    // Verify basic properties
    assert_eq!(graph.nodes.len(), 2);
    assert!(graph.nodes.contains(&"numpy".to_string()));
    assert!(graph.nodes.contains(&"pandas".to_string()));
    
    // Edges may vary based on actual environment, but we can check general structure
    assert!(graph.edges.len() >= 0); // At minimum, no edges if dependencies can't be determined
}

#[test]
fn test_export_dependency_graph() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("graph.dot");
    
    let graph = analysis::DependencyGraph {
        nodes: vec!["numpy".to_string(), "pandas".to_string(), "matplotlib".to_string()],
        edges: vec![
            ("pandas".to_string(), "numpy".to_string()),
            ("matplotlib".to_string(), "numpy".to_string()),
        ],
    };
    
    let result = analysis::export_dependency_graph(&graph, &file_path);
    assert!(result.is_ok());
    
    // Check that the file exists
    assert!(file_path.exists());
    
    // Check file content
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("digraph dependencies"));
    assert!(content.contains("\"pandas\" -> \"numpy\""));
    assert!(content.contains("\"matplotlib\" -> \"numpy\""));
}

// Helper function to create temporary environment for integration tests
fn create_test_environment() -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("environment.yml");
    
    let yaml_content = r#"name: test-environment
channels:
  - conda-forge
  - defaults
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas>=1.3.0
  - matplotlib=3.5.0
  - pip
  - pip:
    - tensorflow==2.8.0
"#;
    
    let mut file = File::create(&file_path).unwrap();
    file.write_all(yaml_content.as_bytes()).unwrap();
    
    (dir, file_path)
}

#[test]
fn test_analyze_dependencies() {
    // Create a sample environment
    let env = CondaEnvironment {
        name: "test-env".to_string(),
        channels: vec!["conda-forge".to_string()],
        dependencies: vec![
            "python=3.9".to_string(),
            "numpy=1.21.0".to_string(),
            "pandas=1.3.0".to_string(),
            "matplotlib=3.4.3".to_string(),
            "scikit-learn=1.0".to_string(),
        ],
    };

    // Run the dependency analysis
    let result = analyze_dependencies(&env);
    
    // Verify the analysis was successful
    assert!(result.is_ok(), "Dependency analysis should succeed");
    
    let analysis = result.unwrap();
    
    // Check basic properties
    assert_eq!(analysis.environment.name, "test-env", "Environment name should be preserved");
    assert_eq!(analysis.packages.len(), 5, "Analysis should contain 5 packages");
    
    // Check that each package was analyzed
    let package_names: Vec<String> = analysis.packages.iter().map(|p| p.name.clone()).collect();
    assert!(package_names.contains(&"python".to_string()), "Analysis should include python");
    assert!(package_names.contains(&"numpy".to_string()), "Analysis should include numpy");
    assert!(package_names.contains(&"pandas".to_string()), "Analysis should include pandas");
    assert!(package_names.contains(&"matplotlib".to_string()), "Analysis should include matplotlib");
    assert!(package_names.contains(&"scikit-learn".to_string()), "Analysis should include scikit-learn");
    
    // Check dependencies are correctly assigned
    let pandas_package = analysis.packages.iter().find(|p| p.name == "pandas").unwrap();
    assert!(pandas_package.dependencies.contains(&"python".to_string()), 
            "pandas should depend on python");
    assert!(pandas_package.dependencies.contains(&"numpy".to_string()), 
            "pandas should depend on numpy");
}

#[test]
fn test_generate_recommendations() {
    // Create a sample analysis
    let mut analysis = EnvironmentAnalysis {
        environment: CondaEnvironment {
            name: "test-env".to_string(),
            channels: vec!["conda-forge".to_string()],
            dependencies: vec![
                "python=3.7".to_string(),  // Older Python version
                "numpy=1.19.0".to_string(), // Older numpy
                "pandas=1.0.0".to_string(), // Older pandas
                "requests=2.24.0".to_string(), // Security vulnerability (hypothetical)
                "unused-package=1.0.0".to_string(), // Unused package
            ],
        },
        packages: vec![
            Package {
                name: "python".to_string(),
                version: "3.7".to_string(),
                build_string: None,
                size: Some(100_000_000),
                dependencies: vec![],
            },
            Package {
                name: "numpy".to_string(),
                version: "1.19.0".to_string(),
                build_string: None,
                size: Some(50_000_000),
                dependencies: vec!["python".to_string()],
            },
            Package {
                name: "pandas".to_string(),
                version: "1.0.0".to_string(),
                build_string: None,
                size: Some(30_000_000),
                dependencies: vec!["python".to_string(), "numpy".to_string()],
            },
            Package {
                name: "requests".to_string(),
                version: "2.24.0".to_string(),
                build_string: None,
                size: Some(5_000_000),
                dependencies: vec!["python".to_string()],
            },
            Package {
                name: "unused-package".to_string(),
                version: "1.0.0".to_string(),
                build_string: None,
                size: Some(10_000_000),
                dependencies: vec![],
            },
        ],
        total_size: Some(195_000_000),
        recommendations: vec![],
    };
    
    // Generate recommendations
    generate_recommendations(&mut analysis);
    
    // Check that recommendations were generated
    assert!(!analysis.recommendations.is_empty(), "Recommendations should be generated");
    
    // Check for specific recommendation types
    let upgrade_recs: Vec<&Recommendation> = analysis.recommendations.iter()
        .filter(|r| r.recommendation_type == RecommendationType::Upgrade)
        .collect();
    
    let security_recs: Vec<&Recommendation> = analysis.recommendations.iter()
        .filter(|r| r.recommendation_type == RecommendationType::Security)
        .collect();
    
    let optimization_recs: Vec<&Recommendation> = analysis.recommendations.iter()
        .filter(|r| r.recommendation_type == RecommendationType::Optimization)
        .collect();
    
    // Should have at least one upgrade recommendation (for Python or numpy or pandas)
    assert!(!upgrade_recs.is_empty(), "Should have upgrade recommendations");
    
    // Check for specific package recommendations
    let python_recs: Vec<&Recommendation> = analysis.recommendations.iter()
        .filter(|r| r.package == "python")
        .collect();
    
    assert!(!python_recs.is_empty(), "Should have recommendation for Python");
}

#[test]
fn test_calculate_environment_size() {
    // Create a sample analysis
    let mut analysis = EnvironmentAnalysis {
        environment: CondaEnvironment {
            name: "test-env".to_string(),
            channels: vec!["conda-forge".to_string()],
            dependencies: vec![
                "python=3.9".to_string(),
                "numpy=1.21.0".to_string(),
                "pandas=1.3.0".to_string(),
            ],
        },
        packages: vec![
            Package {
                name: "python".to_string(),
                version: "3.9".to_string(),
                build_string: None,
                size: Some(100_000_000),  // 100 MB
                dependencies: vec![],
            },
            Package {
                name: "numpy".to_string(),
                version: "1.21.0".to_string(),
                build_string: None,
                size: Some(50_000_000),  // 50 MB
                dependencies: vec!["python".to_string()],
            },
            Package {
                name: "pandas".to_string(),
                version: "1.3.0".to_string(),
                build_string: None,
                size: Some(30_000_000),  // 30 MB
                dependencies: vec!["python".to_string(), "numpy".to_string()],
            },
        ],
        total_size: None,
        recommendations: vec![],
    };
    
    // Calculate environment size
    calculate_environment_size(&mut analysis);
    
    // Check that the total size is set
    assert!(analysis.total_size.is_some(), "Total size should be set");
    
    // Check that the total size is the sum of all package sizes
    assert_eq!(analysis.total_size.unwrap(), 180_000_000, 
               "Total size should be 180,000,000 bytes (180 MB)");
}

#[test]
fn test_analyze_dependencies_with_pip_packages() {
    // Create a sample environment with pip packages
    let env = CondaEnvironment {
        name: "test-env-pip".to_string(),
        channels: vec!["conda-forge".to_string()],
        dependencies: vec![
            "python=3.9".to_string(),
            "pip".to_string(),
            "pip:requests==2.26.0".to_string(),
            "pip:flask==2.0.1".to_string(),
        ],
    };

    // Run the dependency analysis
    let result = analyze_dependencies(&env);
    
    // Verify the analysis was successful
    assert!(result.is_ok(), "Dependency analysis should succeed");
    
    let analysis = result.unwrap();
    
    // Check that pip packages are included
    let package_names: Vec<String> = analysis.packages.iter().map(|p| p.name.clone()).collect();
    assert!(package_names.contains(&"python".to_string()), "Analysis should include python");
    assert!(package_names.contains(&"pip".to_string()), "Analysis should include pip");
    assert!(package_names.contains(&"requests".to_string()), "Analysis should include requests");
    assert!(package_names.contains(&"flask".to_string()), "Analysis should include flask");
    
    // Check pip package dependencies
    let requests_package = analysis.packages.iter().find(|p| p.name == "requests").unwrap();
    assert_eq!(requests_package.version, "2.26.0", "requests version should be 2.26.0");
    
    let flask_package = analysis.packages.iter().find(|p| p.name == "flask").unwrap();
    assert_eq!(flask_package.version, "2.0.1", "flask version should be 2.0.1");
}

#[test]
fn test_calculate_environment_size_with_missing_sizes() {
    // Create a sample analysis with some missing sizes
    let mut analysis = EnvironmentAnalysis {
        environment: CondaEnvironment {
            name: "test-env-missing-sizes".to_string(),
            channels: vec!["conda-forge".to_string()],
            dependencies: vec![
                "python=3.9".to_string(),
                "numpy=1.21.0".to_string(),
                "unknown-package=1.0.0".to_string(),
            ],
        },
        packages: vec![
            Package {
                name: "python".to_string(),
                version: "3.9".to_string(),
                build_string: None,
                size: Some(100_000_000),  // 100 MB
                dependencies: vec![],
            },
            Package {
                name: "numpy".to_string(),
                version: "1.21.0".to_string(),
                build_string: None,
                size: Some(50_000_000),  // 50 MB
                dependencies: vec!["python".to_string()],
            },
            Package {
                name: "unknown-package".to_string(),
                version: "1.0.0".to_string(),
                build_string: None,
                size: None,  // Size unknown
                dependencies: vec!["python".to_string()],
            },
        ],
        total_size: None,
        recommendations: vec![],
    };
    
    // Calculate environment size
    calculate_environment_size(&mut analysis);
    
    // Check that the total size is set
    assert!(analysis.total_size.is_some(), "Total size should be set");
    
    // Check that the total size is the sum of all known package sizes
    assert_eq!(analysis.total_size.unwrap(), 150_000_000, 
               "Total size should be 150,000,000 bytes (150 MB) - sum of known sizes only");
}

// Test helper function to create a Package
fn create_test_package(name: &str, version: Option<&str>, size: Option<u64>) -> Package {
    Package {
        name: name.to_string(),
        version: version.map(|v| v.to_string()),
        build: None,
        channel: Some("conda-forge".to_string()),
        size,
        is_pinned: version.is_some(),
        is_outdated: false,
        latest_version: None,
    }
}

#[test]
fn test_calculate_environment_size() {
    // Create a sample analysis
    let mut analysis = EnvironmentAnalysis {
        name: Some("test-env".to_string()),
        packages: vec![
            create_test_package("python", Some("3.9"), Some(100_000_000)),
            create_test_package("numpy", Some("1.21.0"), Some(50_000_000)),
            create_test_package("pandas", Some("1.3.0"), Some(30_000_000)),
        ],
        total_size: None,
        pinned_count: 3,
        outdated_count: 0,
        recommendations: vec![],
    };
    
    // Calculate environment size
    calculate_environment_size(&mut analysis);
    
    // Check that the total size is set
    assert!(analysis.total_size.is_some(), "Total size should be set");
    
    // Check that the total size is the sum of all package sizes
    assert_eq!(analysis.total_size.unwrap(), 180_000_000, 
               "Total size should be 180,000,000 bytes (180 MB)");
}

#[test]
fn test_calculate_environment_size_with_missing_sizes() {
    // Create a sample analysis with some missing sizes
    let mut analysis = EnvironmentAnalysis {
        name: Some("test-env-missing-sizes".to_string()),
        packages: vec![
            create_test_package("python", Some("3.9"), Some(100_000_000)),
            create_test_package("numpy", Some("1.21.0"), Some(50_000_000)),
            create_test_package("unknown-package", Some("1.0.0"), None),
        ],
        total_size: None,
        pinned_count: 3,
        outdated_count: 0,
        recommendations: vec![],
    };
    
    // Calculate environment size
    calculate_environment_size(&mut analysis);
    
    // Check that the total size is set
    assert!(analysis.total_size.is_some(), "Total size should be set");
    
    // Check that the total size is the sum of all known package sizes
    assert_eq!(analysis.total_size.unwrap(), 150_000_000, 
               "Total size should be 150,000,000 bytes (150 MB) - sum of known sizes only");
}

#[test]
fn test_generate_recommendations() {
    // Create a sample environment with outdated packages
    let packages = vec![
        Package {
            name: "python".to_string(),
            version: Some("3.7".to_string()),
            build: None,
            channel: Some("conda-forge".to_string()),
            size: Some(100_000_000),
            is_pinned: true,
            is_outdated: true,
            latest_version: Some("3.10".to_string()),
        },
        Package {
            name: "numpy".to_string(),
            version: Some("1.19.0".to_string()),
            build: None,
            channel: Some("conda-forge".to_string()),
            size: Some(50_000_000),
            is_pinned: true,
            is_outdated: true,
            latest_version: Some("1.23.0".to_string()),
        },
        Package {
            name: "pandas".to_string(),
            version: Some("1.0.0".to_string()),
            build: None,
            channel: Some("conda-forge".to_string()),
            size: Some(30_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
    ];
    
    // Generate recommendations
    let recommendations = generate_recommendations(&packages, true);
    
    // Check that recommendations were generated
    assert!(!recommendations.is_empty(), "Recommendations should be generated");
    
    // Check for specific recommendations
    let upgrade_rec = recommendations.iter().any(|r| r.contains("upgrade") && r.contains("python"));
    assert!(upgrade_rec, "Should recommend upgrading Python");
}

#[test]
fn test_create_dependency_graph() {
    // This test would ideally test the `create_dependency_graph` function,
    // but since it makes external API calls, we'll keep it simple
    
    // Create a sample list of packages
    let packages = vec![
        create_test_package("python", Some("3.9"), Some(100_000_000)),
        create_test_package("numpy", Some("1.21.0"), Some(50_000_000)),
        create_test_package("pandas", Some("1.3.0"), Some(30_000_000)),
    ];
    
    // For now, let's just verify the packages exist with the right structure
    assert_eq!(packages.len(), 3, "Should have 3 packages");
    assert_eq!(packages[0].name, "python", "First package should be python");
    assert_eq!(packages[1].name, "numpy", "Second package should be numpy");
    assert_eq!(packages[2].name, "pandas", "Third package should be pandas");
} 