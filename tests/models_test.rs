use conda_env_inspect::models::{Package, EnvironmentAnalysis};

#[test]
fn test_package_creation() {
    let package = Package {
        name: "numpy".to_string(),
        version: Some("1.21.0".to_string()),
        build: Some("py39h5d0ccc0_0".to_string()),
        channel: Some("conda-forge".to_string()),
        is_pinned: false,
        is_outdated: false,
        size: Some(10485760), // 10MB
        latest_version: Some("1.23.5".to_string()),
    };

    assert_eq!(package.name, "numpy");
    assert_eq!(package.version, Some("1.21.0".to_string()));
    assert_eq!(package.build, Some("py39h5d0ccc0_0".to_string()));
    assert_eq!(package.channel, Some("conda-forge".to_string()));
    assert_eq!(package.size, Some(10485760));
    assert_eq!(package.latest_version, Some("1.23.5".to_string()));
    assert!(!package.is_pinned);
    assert!(!package.is_outdated);
}

#[test]
fn test_environment_analysis() {
    let packages = vec![
        Package {
            name: "numpy".to_string(),
            version: Some("1.21.0".to_string()),
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

    let analysis = EnvironmentAnalysis {
        name: Some("test-env".to_string()),
        packages,
        pinned_count: 1,
        outdated_count: 1,
        total_size: Some(31457280), // 30MB
        recommendations: vec!["Update numpy".to_string()],
    };

    assert_eq!(analysis.name, Some("test-env".to_string()));
    assert_eq!(analysis.packages.len(), 2);
    assert_eq!(analysis.pinned_count, 1);
    assert_eq!(analysis.outdated_count, 1);
    assert_eq!(analysis.total_size, Some(31457280));
    assert_eq!(analysis.recommendations.len(), 1);
    assert_eq!(analysis.recommendations[0], "Update numpy");
} 