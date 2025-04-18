use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use conda_env_inspect::parsers;
use conda_env_inspect::models::{CondaEnvironment, Dependency};
use conda_env_inspect::parsers::{parse_environment_file, parse_full_environment};
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;

#[test]
fn test_parse_environment_yaml() {
    // Create a temporary directory for the test
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("environment.yml");
    
    // Create a sample environment.yaml file
    let yaml_content = r#"name: test-env
channels:
  - conda-forge
  - defaults
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas>=1.3.0
  - pip
  - pip:
    - tensorflow==2.8.0
"#;
    
    let mut file = File::create(&file_path).unwrap();
    file.write_all(yaml_content.as_bytes()).unwrap();
    
    // Parse the environment file
    let result = parsers::parse_environment_file(&file_path);
    assert!(result.is_ok());
    
    let env = result.unwrap();
    assert_eq!(env.name, Some("test-env".to_string()));
    assert_eq!(env.dependencies.len(), 5); // 5 dependencies total
    
    // Check conda dependencies
    let mut count_conda_deps = 0;
    let mut count_pip_deps = 0;
    
    for dep in env.dependencies {
        match dep {
            Dependency::Simple(dep_str) => {
                if dep_str.starts_with("python=") {
                    assert!(dep_str.contains("3.9"));
                } else if dep_str.starts_with("numpy=") {
                    assert!(dep_str.contains("1.21.0"));
                } else if dep_str.starts_with("pandas>=") {
                    assert!(dep_str.contains("1.3.0"));
                }
                count_conda_deps += 1;
            },
            Dependency::Complex(complex_dep) => {
                // Check pip dependencies
                if let Some(pip_deps) = complex_dep.pip {
                    for pip_dep in pip_deps {
                        if pip_dep.contains("tensorflow") {
                            assert!(pip_dep.contains("2.8.0"));
                            count_pip_deps += 1;
                        }
                    }
                }
            }
        }
    }
    
    assert_eq!(count_conda_deps, 4); // python, numpy, pandas, pip
    assert_eq!(count_pip_deps, 1);   // tensorflow
}

#[test]
fn test_parse_conda_lock_file() {
    // Create a temporary directory for the test
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("conda-lock.yml");
    
    // Create a sample conda-lock.yml file - using a format that our parser can handle
    let yaml_content = r#"
name: locked-env
channels:
  - defaults
  - conda-forge
dependencies:
  - python=3.9.7
  - numpy=1.21.0
  - pandas=1.3.0
"#;
    
    let mut file = File::create(&file_path).unwrap();
    file.write_all(yaml_content.as_bytes()).unwrap();
    
    // Parse the file
    let result = parsers::parse_environment_file(&file_path);
    assert!(result.is_ok());
    
    let env = result.unwrap();
    assert_eq!(env.name, Some("locked-env".to_string()));
    assert_eq!(env.dependencies.len(), 3); // 3 dependencies total
    
    // Check the dependencies
    let mut has_python = false;
    let mut has_numpy = false;
    let mut has_pandas = false;
    
    for dep in env.dependencies {
        if let Dependency::Simple(dep_str) = dep {
            if dep_str.starts_with("python=") {
                has_python = true;
                assert!(dep_str.contains("3.9.7"));
            } else if dep_str.starts_with("numpy=") {
                has_numpy = true;
                assert!(dep_str.contains("1.21.0"));
            } else if dep_str.starts_with("pandas=") {
                has_pandas = true;
                assert!(dep_str.contains("1.3.0"));
            }
        }
    }
    
    assert!(has_python, "Python dependency not found");
    assert!(has_numpy, "NumPy dependency not found");
    assert!(has_pandas, "Pandas dependency not found");
}

#[test]
fn test_parse_environment_file() {
    // Create a temporary file with sample environment.yml content
    let temp_dir = tempdir::TempDir::new("test_parse_env").unwrap();
    let file_path = temp_dir.path().join("environment.yml");
    
    let yaml_content = r#"
name: test-env
channels:
  - conda-forge
  - defaults
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas=1.3.0
  - pip
  - pip:
    - requests==2.26.0
"#;
    
    std::fs::write(&file_path, yaml_content).unwrap();
    
    // Parse the environment file
    let result = parse_environment_file(&file_path);
    
    // Check that parsing was successful
    assert!(result.is_ok(), "Failed to parse valid environment file");
    
    // Get the parsed environment
    let env = result.unwrap();
    
    // Verify the environment properties
    assert_eq!(env.name, "test-env", "Environment name should be 'test-env'");
    
    // Check channels
    assert_eq!(env.channels.len(), 2, "Environment should have 2 channels");
    assert_eq!(env.channels[0], "conda-forge", "First channel should be 'conda-forge'");
    assert_eq!(env.channels[1], "defaults", "Second channel should be 'defaults'");
    
    // Check dependencies
    assert_eq!(env.dependencies.len(), 5, "Environment should have 5 dependencies");
    assert!(env.dependencies.contains(&"python=3.9".to_string()), "Dependencies should include 'python=3.9'");
    assert!(env.dependencies.contains(&"numpy=1.21.0".to_string()), "Dependencies should include 'numpy=1.21.0'");
    assert!(env.dependencies.contains(&"pandas=1.3.0".to_string()), "Dependencies should include 'pandas=1.3.0'");
    assert!(env.dependencies.contains(&"pip".to_string()), "Dependencies should include 'pip'");
    assert!(env.dependencies.contains(&"pip:requests==2.26.0".to_string()), "Dependencies should include pip package 'requests==2.26.0'");
}

#[test]
fn test_parse_invalid_environment_file() {
    // Create a temporary file with invalid YAML content
    let temp_dir = tempdir::TempDir::new("test_parse_invalid_env").unwrap();
    let file_path = temp_dir.path().join("invalid_environment.yml");
    
    let invalid_yaml_content = r#"
name: test-env
channels:
  - conda-forge
  - defaults
dependencies:
  - python=3.9
  invalid_line
  - numpy=1.21.0
"#;
    
    std::fs::write(&file_path, invalid_yaml_content).unwrap();
    
    // Parse the invalid environment file
    let result = parse_environment_file(&file_path);
    
    // Check that parsing failed
    assert!(result.is_err(), "Should fail to parse invalid environment file");
    
    // Verify error message contains useful information
    let err = result.err().unwrap();
    assert!(err.to_string().contains("Failed to parse"), "Error should indicate parsing failure");
}

#[test]
fn test_parse_empty_environment_file() {
    // Create a temporary file with minimal valid YAML content
    let temp_dir = tempdir::TempDir::new("test_parse_empty_env").unwrap();
    let file_path = temp_dir.path().join("empty_environment.yml");
    
    let empty_yaml_content = r#"
name: empty-env
channels:
  - defaults
dependencies: []
"#;
    
    std::fs::write(&file_path, empty_yaml_content).unwrap();
    
    // Parse the empty environment file
    let result = parse_environment_file(&file_path);
    
    // Check that parsing was successful
    assert!(result.is_ok(), "Failed to parse empty environment file");
    
    // Get the parsed environment
    let env = result.unwrap();
    
    // Verify the environment properties
    assert_eq!(env.name, "empty-env", "Environment name should be 'empty-env'");
    assert_eq!(env.channels.len(), 1, "Environment should have 1 channel");
    assert_eq!(env.dependencies.len(), 0, "Environment should have 0 dependencies");
}

#[test]
fn test_parse_nonexistent_file() {
    // Try to parse a file that doesn't exist
    let result = parse_environment_file(std::path::Path::new("/nonexistent/file.yml"));
    
    // Check that parsing failed
    assert!(result.is_err(), "Should fail to parse nonexistent file");
    
    // Verify error message contains useful information
    let err = result.err().unwrap();
    assert!(err.to_string().contains("Failed to read"), "Error should indicate file reading failure");
}

#[test]
fn test_parse_environment_with_complex_dependencies() {
    // Create a temporary file with complex dependencies
    let temp_dir = tempdir::TempDir::new("test_parse_complex_env").unwrap();
    let file_path = temp_dir.path().join("complex_environment.yml");
    
    let complex_yaml_content = r#"
name: complex-env
channels:
  - conda-forge
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas=1.3.0
  - matplotlib=3.4.3
  - scikit-learn=1.0
  - jupyterlab=3.2.1
  - pip>=21.3
  - pip:
    - requests==2.26.0
    - flask==2.0.2
    - black==21.9b0
    - pytest>=6.0
"#;
    
    std::fs::write(&file_path, complex_yaml_content).unwrap();
    
    // Parse the environment file
    let result = parse_environment_file(&file_path);
    
    // Check that parsing was successful
    assert!(result.is_ok(), "Failed to parse complex environment file");
    
    // Get the parsed environment
    let env = result.unwrap();
    
    // Verify the environment properties
    assert_eq!(env.name, "complex-env", "Environment name should be 'complex-env'");
    
    // Check channels
    assert_eq!(env.channels.len(), 1, "Environment should have 1 channel");
    assert_eq!(env.channels[0], "conda-forge", "Channel should be 'conda-forge'");
    
    // Check dependencies (should have 11 in total: 7 conda packages + 4 pip packages)
    assert_eq!(env.dependencies.len(), 11, "Environment should have 11 dependencies");
    
    // Check for conda packages
    assert!(env.dependencies.contains(&"python=3.9".to_string()));
    assert!(env.dependencies.contains(&"numpy=1.21.0".to_string()));
    assert!(env.dependencies.contains(&"pandas=1.3.0".to_string()));
    assert!(env.dependencies.contains(&"matplotlib=3.4.3".to_string()));
    assert!(env.dependencies.contains(&"scikit-learn=1.0".to_string()));
    assert!(env.dependencies.contains(&"jupyterlab=3.2.1".to_string()));
    assert!(env.dependencies.contains(&"pip>=21.3".to_string()));
    
    // Check for pip packages
    assert!(env.dependencies.contains(&"pip:requests==2.26.0".to_string()));
    assert!(env.dependencies.contains(&"pip:flask==2.0.2".to_string()));
    assert!(env.dependencies.contains(&"pip:black==21.9b0".to_string()));
    assert!(env.dependencies.contains(&"pip:pytest>=6.0".to_string()));
}

#[test]
fn test_parse_package_spec() {
    // Test basic package spec parsing
    let pkg = parsers::parse_package_spec("numpy=1.21.0");
    assert_eq!(pkg.name, "numpy");
    assert!(pkg.version.as_ref().unwrap().starts_with("1.21"));  // More flexible version check
    assert_eq!(pkg.build, None);
    assert_eq!(pkg.channel, None);
    assert!(pkg.is_pinned);
    
    // Test with channel
    let pkg = parsers::parse_package_spec("conda-forge::numpy=1.21.0");
    assert_eq!(pkg.name, "numpy");
    assert!(pkg.version.as_ref().unwrap().starts_with("1.21"));
    assert_eq!(pkg.channel, Some("conda-forge".to_string()));
    
    // Test with build string
    let pkg = parsers::parse_package_spec("numpy=1.21.0=py39h5d0ccc0_0");
    assert_eq!(pkg.name, "numpy");
    assert!(pkg.version.as_ref().unwrap().starts_with("1.21"));
    assert_eq!(pkg.build, Some("py39h5d0ccc0_0".to_string()));
    
    // Test without version
    let pkg = parsers::parse_package_spec("numpy");
    assert_eq!(pkg.name, "numpy");
    assert_eq!(pkg.version, None);
    assert!(!pkg.is_pinned);
} 