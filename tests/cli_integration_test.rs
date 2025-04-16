use std::fs::File;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

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
    
    let mut file = File::create(&file_path).unwrap();
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
    
    let mut file = File::create(&file_path).unwrap();
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
    
    let json_content = std::fs::read_to_string(output_path).unwrap();
    assert!(json_content.contains("test-env"));
    assert!(json_content.contains("python"));
    assert!(json_content.contains("numpy"));
    assert!(json_content.contains("pandas"));
} 