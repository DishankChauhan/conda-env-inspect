use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use conda_env_inspect::parsers;
use conda_env_inspect::models::CondaEnvironment;

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
    assert_eq!(env.packages.len(), 5); // 5 packages total
    
    // Check if specific packages are parsed correctly
    let python_pkg = env.packages.iter().find(|p| p.name == "python").unwrap();
    let numpy_pkg = env.packages.iter().find(|p| p.name == "numpy").unwrap();
    let pandas_pkg = env.packages.iter().find(|p| p.name == "pandas").unwrap();
    
    assert_eq!(python_pkg.version, Some("3.9".to_string()));
    assert_eq!(numpy_pkg.version, Some("1.21.0".to_string()));
    assert_eq!(pandas_pkg.version, Some("1.3.0".to_string()));
    assert!(pandas_pkg.name.contains("pandas"));
    
    // Check pip package
    let pip_pkg = env.packages.iter().find(|p| p.name == "pip").unwrap();
    assert_eq!(pip_pkg.version, None);
    
    // Check tensorflow from pip section
    let tensorflow_pkg = env.packages.iter().find(|p| p.name == "tensorflow").unwrap();
    assert_eq!(tensorflow_pkg.version, Some("2.8.0".to_string()));
    assert_eq!(tensorflow_pkg.channel, Some("pip".to_string()));
}

#[test]
fn test_parse_conda_lock_file() {
    // Create a temporary directory for the test
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("conda-lock.yml");
    
    // Create a sample conda-lock.yml file
    let yaml_content = r#"
package:
  - name: python
    version: 3.9.7
    build: h6244533_1
    channel: defaults
  - name: numpy
    version: 1.21.0
    build: py39h5d0ccc0_0
    channel: conda-forge
  - name: pandas
    version: 1.3.0
    build: py39h5d0ccc0_0
    channel: conda-forge
"#;
    
    let mut file = File::create(&file_path).unwrap();
    file.write_all(yaml_content.as_bytes()).unwrap();
    
    // Parse the conda lock file
    let result = parsers::parse_environment_file(&file_path);
    assert!(result.is_ok());
    
    let env = result.unwrap();
    assert_eq!(env.name, None); // No name in conda-lock files
    assert_eq!(env.packages.len(), 3); // 3 packages total
    
    // Check if specific packages are parsed correctly
    let python_pkg = env.packages.iter().find(|p| p.name == "python").unwrap();
    let numpy_pkg = env.packages.iter().find(|p| p.name == "numpy").unwrap();
    let pandas_pkg = env.packages.iter().find(|p| p.name == "pandas").unwrap();
    
    assert_eq!(python_pkg.version, Some("3.9.7".to_string()));
    assert_eq!(python_pkg.build, Some("h6244533_1".to_string()));
    assert_eq!(python_pkg.channel, Some("defaults".to_string()));
    
    assert_eq!(numpy_pkg.version, Some("1.21.0".to_string()));
    assert_eq!(numpy_pkg.build, Some("py39h5d0ccc0_0".to_string()));
    assert_eq!(numpy_pkg.channel, Some("conda-forge".to_string()));
    
    assert_eq!(pandas_pkg.version, Some("1.3.0".to_string()));
} 