use std::fs;
use std::path::Path;
use std::process::Command;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::{tempdir, NamedTempFile};

#[test]
fn test_cli_basic_analyze() {
    // Skip the test if we can't find the cargo binary
    if Command::new("cargo").output().is_err() {
        println!("Skipping test_cli_basic_analyze because cargo is not available");
        return;
    }
    
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
    
    let mut file = fs::File::create(&file_path).unwrap();
    file.write_all(yaml_content.as_bytes()).unwrap();
    
    // Run the CLI with basic analyze command
    let output = Command::new("cargo")
        .args(&["run", "--", "analyze", "--file", file_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute CLI command");
        
    assert!(output.status.success());
    
    // Check that the output includes expected information
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test-env"));
    assert!(stdout.contains("python"));
    assert!(stdout.contains("numpy"));
    assert!(stdout.contains("pandas"));
}

#[test]
fn test_cli_export_json() {
    // Skip the test if we can't find the cargo binary
    if Command::new("cargo").output().is_err() {
        println!("Skipping test_cli_export_json because cargo is not available");
        return;
    }
    
    // Create a temporary directory for the test
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("environment.yml");
    let output_path = dir.path().join("output.json");
    
    // Create a sample environment.yaml file
    let yaml_content = r#"name: test-env
channels:
  - conda-forge
  - defaults
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas>=1.3.0
"#;
    
    let mut file = fs::File::create(&file_path).unwrap();
    file.write_all(yaml_content.as_bytes()).unwrap();
    
    // Run the CLI with export command
    let output = Command::new("cargo")
        .args(&[
            "run", "--", 
            "export", 
            "--file", file_path.to_str().unwrap(),
            "--format", "json",
            "--output", output_path.to_str().unwrap()
        ])
        .output()
        .expect("Failed to execute CLI command");
        
    assert!(output.status.success());
    
    // Check that the JSON file exists and contains expected content
    assert!(output_path.exists());
    
    let json_content = fs::read_to_string(output_path).unwrap();
    assert!(json_content.contains("test-env"));
    assert!(json_content.contains("python"));
    assert!(json_content.contains("numpy"));
    assert!(json_content.contains("pandas"));
}

#[test]
fn test_inspect_simple_environment() {
    // Create a temporary YAML file with a simple environment
    let file = NamedTempFile::new().unwrap();
    let yaml_content = r#"
name: test-env
channels:
  - conda-forge
  - defaults
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas=1.3.0
"#;
    fs::write(file.path(), yaml_content).unwrap();
    
    // Run the inspect command
    let mut cmd = Command::cargo_bin("conda-env-inspect").unwrap();
    let assert = cmd
        .arg("inspect")
        .arg(file.path())
        .assert();
    
    // Verify command succeeds and output contains expected information
    assert
        .success()
        .stdout(predicate::str::contains("test-env"))
        .stdout(predicate::str::contains("python"))
        .stdout(predicate::str::contains("numpy"))
        .stdout(predicate::str::contains("pandas"));
}

#[test]
fn test_inspect_complex_environment() {
    // Create a temporary YAML file with a more complex environment
    let file = NamedTempFile::new().unwrap();
    let yaml_content = r#"
name: complex-env
channels:
  - conda-forge
  - defaults
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas=1.3.0
  - matplotlib>=3.5.0
  - scikit-learn
  - pip
  - pip:
    - requests==2.27.1
    - tensorflow>=2.8.0
"#;
    fs::write(file.path(), yaml_content).unwrap();
    
    // Run the inspect command
    let mut cmd = Command::cargo_bin("conda-env-inspect").unwrap();
    let assert = cmd
        .arg("inspect")
        .arg(file.path())
        .assert();
    
    // Verify command succeeds and output contains expected information
    assert
        .success()
        .stdout(predicate::str::contains("complex-env"))
        .stdout(predicate::str::contains("python"))
        .stdout(predicate::str::contains("numpy"))
        .stdout(predicate::str::contains("pip packages"))
        .stdout(predicate::str::contains("requests"))
        .stdout(predicate::str::contains("tensorflow"));
}

#[test]
fn test_inspect_invalid_environment() {
    // Create a temporary file with invalid YAML
    let file = NamedTempFile::new().unwrap();
    let invalid_yaml = r#"
name: test-env
channels:
  - conda-forge
  invalid yaml content
"#;
    fs::write(file.path(), invalid_yaml).unwrap();
    
    // Run the inspect command
    let mut cmd = Command::cargo_bin("conda-env-inspect").unwrap();
    let assert = cmd
        .arg("inspect")
        .arg(file.path())
        .assert();
    
    // Verify command fails with an error message
    assert
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn test_generate_dependency_graph() {
    // Create a temporary YAML file with a simple environment
    let file = NamedTempFile::new().unwrap();
    let yaml_content = r#"
name: test-env
channels:
  - conda-forge
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas=1.3.0
  - matplotlib=3.5.0
"#;
    fs::write(file.path(), yaml_content).unwrap();
    
    // Create a temporary directory for the output graph
    let output_dir = tempdir().unwrap();
    let output_path = output_dir.path().join("deps.dot");
    
    // Run the command to generate dependency graph
    let mut cmd = Command::cargo_bin("conda-env-inspect").unwrap();
    let assert = cmd
        .arg("graph")
        .arg(file.path())
        .arg("--output")
        .arg(&output_path)
        .assert();
    
    // Verify command succeeds and output file exists
    assert.success();
    
    assert!(output_path.exists(), "Graph file should be created");
    
    // Verify content of the graph file
    let graph_content = fs::read_to_string(output_path).unwrap();
    assert!(graph_content.contains("digraph"), "Should contain 'digraph' keyword");
    assert!(graph_content.contains("numpy"), "Should include numpy node");
    assert!(graph_content.contains("pandas"), "Should include pandas node");
}

#[test]
fn test_export_analysis_json() {
    // Create a temporary YAML file with a simple environment
    let file = NamedTempFile::new().unwrap();
    let yaml_content = r#"
name: test-env
channels:
  - conda-forge
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas=1.3.0
"#;
    fs::write(file.path(), yaml_content).unwrap();
    
    // Create a temporary directory for the output file
    let output_dir = tempdir().unwrap();
    let output_path = output_dir.path().join("analysis.json");
    
    // Run the command to export analysis as JSON
    let mut cmd = Command::cargo_bin("conda-env-inspect").unwrap();
    let assert = cmd
        .arg("inspect")
        .arg(file.path())
        .arg("--export")
        .arg("json")
        .arg("--output")
        .arg(&output_path)
        .assert();
    
    // Verify command succeeds and output file exists
    assert.success();
    
    assert!(output_path.exists(), "JSON file should be created");
    
    // Parse and verify content of the JSON file
    let json_content = fs::read_to_string(output_path).unwrap();
    assert!(json_content.contains("test-env"), "Should contain environment name");
    assert!(json_content.contains("numpy"), "Should include numpy package");
    assert!(json_content.contains("pandas"), "Should include pandas package");
}

#[test]
fn test_export_analysis_markdown() {
    // Create a temporary YAML file with a simple environment
    let file = NamedTempFile::new().unwrap();
    let yaml_content = r#"
name: test-env
channels:
  - conda-forge
dependencies:
  - python=3.9
  - numpy=1.21.0
  - pandas=1.3.0
"#;
    fs::write(file.path(), yaml_content).unwrap();
    
    // Create a temporary directory for the output file
    let output_dir = tempdir().unwrap();
    let output_path = output_dir.path().join("analysis.md");
    
    // Run the command to export analysis as Markdown
    let mut cmd = Command::cargo_bin("conda-env-inspect").unwrap();
    let assert = cmd
        .arg("inspect")
        .arg(file.path())
        .arg("--export")
        .arg("markdown")
        .arg("--output")
        .arg(&output_path)
        .assert();
    
    // Verify command succeeds and output file exists
    assert.success();
    
    assert!(output_path.exists(), "Markdown file should be created");
    
    // Verify content of the Markdown file
    let md_content = fs::read_to_string(output_path).unwrap();
    assert!(md_content.contains("# Environment Analysis"), "Should contain header");
    assert!(md_content.contains("test-env"), "Should contain environment name");
    assert!(md_content.contains("## Packages"), "Should have packages section");
    assert!(md_content.contains("numpy"), "Should include numpy package");
    assert!(md_content.contains("pandas"), "Should include pandas package");
} 