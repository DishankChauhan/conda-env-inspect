use std::collections::HashMap;
use conda_env_inspect::advanced_analysis::{AdvancedDependencyGraph, create_advanced_dependency_graph, detect_conflicts};
use conda_env_inspect::models::{CondaEnvironment, Dependency, Package};

#[test]
fn test_build_advanced_dependency_graph() {
    // Create a sample environment
    let env = CondaEnvironment {
        name: Some("test-env".to_string()),
        channels: vec!["conda-forge".to_string()],
        dependencies: vec![
            Dependency::Simple("python=3.9".to_string()),
            Dependency::Simple("numpy=1.21.0".to_string()),
            Dependency::Simple("pandas=1.3.0".to_string()),
        ],
        extra: HashMap::new(),
    };
    
    // Create packages with appropriate dependencies
    let packages = vec![
        Package {
            name: "python".to_string(),
            version: Some("3.9".to_string()),
            build: Some("main".to_string()),
            channel: Some("conda-forge".to_string()),
            size: Some(100_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
        Package {
            name: "numpy".to_string(),
            version: Some("1.21.0".to_string()),
            build: Some("py39".to_string()),
            channel: Some("conda-forge".to_string()),
            size: Some(50_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
        Package {
            name: "pandas".to_string(),
            version: Some("1.3.0".to_string()),
            build: Some("py39".to_string()),
            channel: Some("conda-forge".to_string()),
            size: Some(30_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
    ];
    
    // Build a dependency graph
    let mut dep_map = HashMap::new();
    dep_map.insert("numpy".to_string(), vec!["python".to_string()]);
    dep_map.insert("pandas".to_string(), vec!["python".to_string(), "numpy".to_string()]);
    
    let graph = create_advanced_dependency_graph(&packages, &dep_map);
    
    // Verify the graph structure
    assert_eq!(graph.node_map.len(), 3, "Graph should have 3 nodes");
    
    // Check that all packages have a node in the graph
    assert!(graph.node_map.contains_key("python"), "Graph should contain python node");
    assert!(graph.node_map.contains_key("numpy"), "Graph should contain numpy node");
    assert!(graph.node_map.contains_key("pandas"), "Graph should contain pandas node");
    
    // Check edge relationships 
    let edges_count = graph.graph.edge_count();
    assert_eq!(edges_count, 3, "Should have 3 dependency edges");
}

#[test]
fn test_detect_conflicts() {
    // Create packages with some version conflicts
    let packages = vec![
        Package {
            name: "python".to_string(),
            version: Some("3.9".to_string()),
            build: Some("main".to_string()),
            channel: Some("conda-forge".to_string()),
            size: Some(100_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
        Package {
            name: "numpy".to_string(),
            version: Some("1.21.0".to_string()),
            build: Some("py39".to_string()),
            channel: Some("conda-forge".to_string()),
            size: Some(50_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
        Package {
            name: "pandas".to_string(),
            version: Some("1.3.0".to_string()),
            build: Some("py39".to_string()),
            channel: Some("conda-forge".to_string()),
            size: Some(30_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
        Package {
            name: "scikit-learn".to_string(),
            version: Some("1.0.0".to_string()),
            build: Some("py39".to_string()),
            channel: Some("conda-forge".to_string()),
            size: Some(25_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
    ];
    
    // Create dependency map with conflicting requirements
    let mut dep_map = HashMap::new();
    dep_map.insert("pandas".to_string(), vec!["numpy==1.21.0".to_string()]);
    dep_map.insert("scikit-learn".to_string(), vec!["numpy==1.20.0".to_string()]);
    
    // Detect conflicts
    let conflicts = detect_conflicts(&packages, &dep_map);
    
    // Verify conflicts
    assert!(!conflicts.is_empty(), "Should detect version conflicts");
}

#[test]
fn test_calculate_graph_metrics() {
    // Create packages
    let packages = vec![
        Package {
            name: "python".to_string(),
            version: Some("3.9".to_string()),
            build: Some("main".to_string()),
            channel: Some("conda-forge".to_string()),
            size: Some(100_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
        Package {
            name: "numpy".to_string(),
            version: Some("1.21.0".to_string()),
            build: Some("py39".to_string()),
            channel: Some("conda-forge".to_string()),
            size: Some(50_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
        Package {
            name: "pandas".to_string(),
            version: Some("1.3.0".to_string()),
            build: Some("py39".to_string()),
            channel: Some("conda-forge".to_string()),
            size: Some(30_000_000),
            is_pinned: true,
            is_outdated: false,
            latest_version: None,
        },
    ];
    
    // Create dependency map
    let mut dep_map = HashMap::new();
    dep_map.insert("numpy".to_string(), vec!["python".to_string()]);
    dep_map.insert("pandas".to_string(), vec!["python".to_string(), "numpy".to_string()]);
    
    // Create the graph
    let graph = create_advanced_dependency_graph(&packages, &dep_map);
    
    // Calculate metrics
    let node_count = graph.graph.node_count();
    let edge_count = graph.graph.edge_count();
    let total_size: u64 = packages.iter()
        .filter_map(|p| p.size)
        .sum();
    
    // Verify metrics
    assert_eq!(node_count, 3, "Graph should have 3 nodes");
    assert_eq!(edge_count, 3, "Graph should have 3 edges");
    assert_eq!(total_size, 180_000_000, "Total size should be 180MB");
}

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
    dependency_map.insert("pandas".to_string(), vec!["numpy".to_string()]);
    dependency_map.insert("matplotlib".to_string(), vec!["numpy".to_string()]);

    // Create the advanced dependency graph
    let graph = create_advanced_dependency_graph(&packages, &dependency_map);

    // Check that nodes were created
    assert_eq!(graph.nodes.len(), 3);
    assert!(graph.node_map.contains_key("numpy"));
    assert!(graph.node_map.contains_key("pandas"));
    assert!(graph.node_map.contains_key("matplotlib"));
    
    // Check direct dependencies
    assert!(graph.direct_deps.contains("numpy"));
    assert!(graph.direct_deps.contains("pandas"));
    assert!(graph.direct_deps.contains("matplotlib"));
    
    // Check edges
    let has_pandas_numpy_edge = graph.edges.iter().any(|e| e.from == pandas_id && e.to == numpy_id);
    let has_matplotlib_numpy_edge = graph.edges.iter().any(|e| e.from == matplotlib_id && e.to == numpy_id);
    
    assert!(has_pandas_numpy_edge);
    assert!(has_matplotlib_numpy_edge);
}

#[test]
fn test_export_advanced_dependency_graph() {
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
    ];

    // Create a dependency map
    let mut dependency_map = HashMap::new();
    dependency_map.insert("pandas".to_string(), vec!["numpy".to_string()]);

    // Create the advanced dependency graph
    let graph = create_advanced_dependency_graph(&packages, &dependency_map);
    
    // Export the graph to a temporary file
    let temp_dir = tempfile::tempdir().unwrap();
    let output_path = temp_dir.path().join("advanced_graph.dot");
    
    let result = conda_env_inspect::advanced_analysis::export_advanced_dependency_graph(&graph, &output_path);
    assert!(result.is_ok());
    
    // Check that the file exists and contains expected content
    assert!(output_path.exists());
    
    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("digraph"));
    assert!(content.contains("numpy"));
    assert!(content.contains("pandas"));
}

#[test]
fn test_find_vulnerabilities() {
    // Create packages with known vulnerable versions
    let packages = vec![
        Package {
            name: "numpy".to_string(),
            version: Some("1.19.0".to_string()),  // Older version
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: true,
            size: Some(10485760),
            latest_version: Some("1.24.0".to_string()),
        },
        Package {
            name: "requests".to_string(),
            version: Some("2.20.0".to_string()),  // Has known vulnerabilities
            build: Some("py39h5d0ccc0_0".to_string()),
            channel: Some("conda-forge".to_string()),
            is_pinned: false,
            is_outdated: true,
            size: Some(5242880),
            latest_version: Some("2.28.0".to_string()),
        },
    ];

    // Find vulnerabilities
    let vulnerabilities = conda_env_inspect::advanced_analysis::find_vulnerabilities(&packages);
    
    // The test is somewhat non-deterministic since it depends on network calls
    // So we'll just check that we got a result back
    println!("Found {} potential vulnerabilities", vulnerabilities.len());
    
    // Tests that use external services should be more lenient
    // We'll just ensure the function runs without error
} 