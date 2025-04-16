use conda_env_inspect::performance;
use conda_env_inspect::models::Package;
use conda_env_inspect::conda_api::PackageInfo;

#[test]
fn test_update_package_with_info() {
    // Create a test package
    let mut package = Package {
        name: "numpy".to_string(),
        version: Some("1.19.0".to_string()),
        build: Some("py39h5d0ccc0_0".to_string()),
        channel: Some("conda-forge".to_string()),
        is_pinned: false,
        is_outdated: false,
        size: None,
        latest_version: None,
    };
    
    // Create package info
    let info = PackageInfo {
        name: "numpy".to_string(),
        version: "1.19.0".to_string(),
        latest_version: "1.23.5".to_string(),
        description: "NumPy is the fundamental package for array computing with Python.".to_string(),
        license: "BSD-3-Clause".to_string(),
        size: Some(10485760),
    };
    
    // Update the package
    performance::update_package_with_info(&mut package, &info);
    
    // Verify the package was updated
    assert_eq!(package.latest_version, Some("1.23.5".to_string()));
    assert_eq!(package.size, Some(10485760));
    assert!(package.is_outdated, "Package should be marked as outdated");
}

#[test]
fn test_normalize_conda_version() {
    // Test various version formats
    assert_eq!(performance::normalize_conda_version("1"), "1.0.0");
    assert_eq!(performance::normalize_conda_version("1.2"), "1.2.0");
    assert_eq!(performance::normalize_conda_version("1.2.3"), "1.2.3");
    
    // Test with build string
    assert_eq!(performance::normalize_conda_version("1.2.3+build1"), "1.2.3");
    assert_eq!(performance::normalize_conda_version("1.2.3-build1"), "1.2.3");
    
    // Test edge cases
    assert_eq!(performance::normalize_conda_version("0-dev"), "0-dev");
}

#[test]
fn test_parallel_enrichment() {
    // Create a set of test packages
    let mut packages = vec![
        Package {
            name: "numpy".to_string(),
            version: Some("1.19.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: None,
            latest_version: None,
        },
        Package {
            name: "pandas".to_string(),
            version: Some("1.3.0".to_string()),
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: false,
            size: None,
            latest_version: None,
        }
    ];
    
    // Attempt to enrich the packages
    let result = performance::enrich_packages_parallel(&mut packages);
    
    // The test should compile and run without errors
    assert!(result.is_ok());
    
    // Even if we can't guarantee web API results, we can check that the 
    // function completed successfully
} 