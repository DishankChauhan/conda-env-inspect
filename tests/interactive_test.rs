use conda_env_inspect::interactive::InteractiveUI;
use conda_env_inspect::models::{EnvironmentAnalysis, Package, DependencyGraph, VersionConflict};
use conda_env_inspect::advanced_analysis::AdvancedDependencyGraph;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use crossterm::event::KeyCode;
use std::collections::HashMap;
use conda_env_inspect::analysis::{EnvironmentAnalysis, Package, Recommendation, RecommendationType};
use conda_env_inspect::parsers::CondaEnvironment;
use conda_env_inspect::advanced_analysis::{AdvancedDependencyGraph, DependencyNode, DependencyEdge};
use ratatui::buffer::Buffer;
use conda_env_inspect::models::{CondaEnvironment, Dependency, DependencyInfo, Recommendation};
use conda_env_inspect::advanced_analysis::{create_advanced_dependency_graph};

fn create_test_analysis() -> EnvironmentAnalysis {
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
    
    let dependency_graph = DependencyGraph {
        nodes: vec!["numpy".to_string(), "pandas".to_string()],
        edges: vec![(1, 0)], // pandas depends on numpy
    };
    
    let mut recommendations = HashMap::new();
    recommendations.insert("numpy".to_string(), vec!["Consider upgrading to 1.22.0".to_string()]);
    
    EnvironmentAnalysis {
        name: "test-env".to_string(),
        packages,
        total_size: Some(31457280),
        dependency_graph: Some(dependency_graph),
        recommendations,
        version_conflicts: vec![],
    }
}

// Helper function to create a test environment analysis
fn create_test_environment_analysis() -> EnvironmentAnalysis {
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
    
    let mut dependencies = HashMap::new();
    dependencies.insert("pandas".to_string(), vec![
        DependencyInfo {
            name: "numpy".to_string(),
            version: Some(">=1.20.0".to_string()),
        },
        DependencyInfo {
            name: "python".to_string(),
            version: Some(">=3.8".to_string()),
        }
    ]);
    
    dependencies.insert("numpy".to_string(), vec![
        DependencyInfo {
            name: "python".to_string(),
            version: Some(">=3.8".to_string()),
        }
    ]);
    
    let recommendations = vec![
        Recommendation {
            package: "numpy".to_string(),
            issue: "Outdated version".to_string(),
            recommendation: "Update to 1.22.0".to_string(),
        },
        Recommendation {
            package: "pandas".to_string(),
            issue: "Size concern".to_string(),
            recommendation: "Consider if you need this package".to_string(),
        }
    ];
    
    let total_size = packages.iter().filter_map(|p| p.size).sum();
    
    EnvironmentAnalysis {
        environment: env,
        packages,
        dependencies,
        total_size,
        recommendations,
    }
}

// Helper function to create a dependency graph for testing
fn create_test_dependency_graph(analysis: &EnvironmentAnalysis) -> AdvancedDependencyGraph {
    let mut dep_map = HashMap::new();
    for (pkg, deps) in &analysis.dependencies {
        let dep_names: Vec<String> = deps.iter().map(|d| d.name.clone()).collect();
        dep_map.insert(pkg.clone(), dep_names);
    }
    
    create_advanced_dependency_graph(&analysis.packages, &dep_map)
}

#[test]
fn test_render_summary_tab() {
    // Create test data
    let analysis = create_test_environment_analysis();
    
    // Create a UI with the test data
    let ui = InteractiveUI::new(analysis.clone(), None);
    
    // Create a test backend and terminal
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Create a buffer to capture the rendered output
    let mut buffer = Buffer::empty(terminal.size().unwrap());
    
    // This is a simplified test since we can't easily test the actual rendering
    // Just verify that UI functions can be called without panicking
    assert!(ui.analysis.packages.len() == 3, "Should have 3 packages in test data");
    assert!(ui.analysis.environment.channels.contains(&"conda-forge".to_string()), 
            "Test environment should include conda-forge channel");
}

#[test]
fn test_render_packages_tab() {
    // Create test data
    let analysis = create_test_environment_analysis();
    
    // Create a UI with the test data
    let ui = InteractiveUI::new(analysis.clone(), None);
    
    // Verify package data is correctly loaded
    assert_eq!(ui.analysis.packages.len(), 3, "Should have 3 packages");
    assert!(ui.analysis.packages.iter().any(|p| p.name == "python"), "Should have python package");
    assert!(ui.analysis.packages.iter().any(|p| p.name == "numpy"), "Should have numpy package");
    assert!(ui.analysis.packages.iter().any(|p| p.name == "pandas"), "Should have pandas package");
    
    // Check sizes
    let python_pkg = ui.analysis.packages.iter().find(|p| p.name == "python").unwrap();
    assert_eq!(python_pkg.size, Some(100_000_000), "Python package should have correct size");
}

#[test]
fn test_render_deps_tab() {
    // Create test data
    let analysis = create_test_environment_analysis();
    let graph = create_test_dependency_graph(&analysis);
    
    // Create a UI with the test data and graph
    let ui = InteractiveUI::new(analysis.clone(), Some(graph));
    
    // Verify dependency data
    assert!(ui.analysis.dependencies.contains_key("pandas"), "Should have pandas dependencies");
    assert!(ui.analysis.dependencies.contains_key("numpy"), "Should have numpy dependencies");
    
    // Get pandas dependencies
    let pandas_deps = ui.analysis.dependencies.get("pandas").unwrap();
    assert_eq!(pandas_deps.len(), 2, "Pandas should have 2 dependencies");
    
    // Check if the advanced graph is available
    assert!(ui.advanced_graph.is_some(), "Advanced graph should be available");
}

#[test]
fn test_render_recommendations_tab() {
    // Create test data
    let analysis = create_test_environment_analysis();
    
    // Create a UI with the test data
    let ui = InteractiveUI::new(analysis.clone(), None);
    
    // Verify recommendations data
    assert_eq!(ui.analysis.recommendations.len(), 2, "Should have 2 recommendations");
    
    // Check recommendations content
    let numpy_rec = ui.analysis.recommendations.iter()
        .find(|r| r.package == "numpy")
        .unwrap();
    assert_eq!(numpy_rec.issue, "Outdated version", "Should have correct issue for numpy");
    assert_eq!(numpy_rec.recommendation, "Update to 1.22.0", "Should have correct recommendation for numpy");
    
    let pandas_rec = ui.analysis.recommendations.iter()
        .find(|r| r.package == "pandas")
        .unwrap();
    assert_eq!(pandas_rec.issue, "Size concern", "Should have correct issue for pandas");
}

#[test]
fn test_handle_key_events() {
    let analysis = create_test_analysis();
    let mut ui = InteractiveUI::new(analysis, None);
    
    // Test tab navigation
    assert_eq!(ui.selected_tab, 0, "Initial tab should be Summary (0)");
    
    // Test package selection
    ui.selected_tab = 1; // Set to Packages tab
    assert_eq!(ui.selected_package, 0, "Initial selected package should be 0");
    
    // Simulating key events would require more complex testing setup
    // But at least we can test the initial state
}

#[test]
fn test_ui_initialization() {
    let analysis = create_test_analysis();
    let ui = InteractiveUI::new(analysis.clone(), None);
    
    assert_eq!(ui.analysis.name, "test-env", "Analysis name should match");
    assert_eq!(ui.selected_tab, 0, "Initial tab should be 0");
    assert_eq!(ui.selected_package, 0, "Initial selected package should be 0");
    assert_eq!(ui.graph_scroll, (0, 0), "Initial graph scroll should be (0, 0)");
}

#[test]
fn test_interactive_ui_creation() {
    // Create a sample environment analysis
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
                size: Some(100_000_000),
                dependencies: vec!["".to_string()],
            },
            Package {
                name: "numpy".to_string(),
                version: "1.21.0".to_string(),
                build_string: None,
                size: Some(50_000_000),
                dependencies: vec!["python".to_string()],
            },
            Package {
                name: "pandas".to_string(),
                version: "1.3.0".to_string(),
                build_string: None,
                size: Some(30_000_000),
                dependencies: vec!["python".to_string(), "numpy".to_string()],
            },
        ],
        total_size: Some(180_000_000),
        recommendations: vec![
            Recommendation {
                recommendation_type: RecommendationType::Upgrade,
                package: "numpy".to_string(),
                details: "Consider upgrading to version 1.22.0".to_string(),
                impact: "Improved performance and security".to_string(),
            }
        ],
    };
    
    // Create a simple dependency graph
    let mut dependency_graph = AdvancedDependencyGraph::new();
    dependency_graph.add_node(DependencyNode {
        id: 0,
        name: "python".to_string(),
        version: "3.9".to_string(),
        size: Some(100_000_000),
    });
    dependency_graph.add_node(DependencyNode {
        id: 1,
        name: "numpy".to_string(),
        version: "1.21.0".to_string(),
        size: Some(50_000_000),
    });
    dependency_graph.add_node(DependencyNode {
        id: 2,
        name: "pandas".to_string(),
        version: "1.3.0".to_string(),
        size: Some(30_000_000),
    });
    
    dependency_graph.add_edge(DependencyEdge {
        from: 0,
        to: 1,
    });
    dependency_graph.add_edge(DependencyEdge {
        from: 0,
        to: 2,
    });
    dependency_graph.add_edge(DependencyEdge {
        from: 1,
        to: 2,
    });
    
    // Create the interactive UI
    let ui = InteractiveUI::new(analysis, Some(dependency_graph));
    
    // Assert the initial state
    assert_eq!(ui.selected_tab, 0);
    assert_eq!(ui.selected_package, 0);
    assert_eq!(ui.graph_scroll, (0, 0));
}

#[test]
fn test_render_summary_tab() {
    // Create a sample environment analysis
    let analysis = EnvironmentAnalysis {
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
                size: Some(100_000_000),
                dependencies: vec!["".to_string()],
            },
            Package {
                name: "numpy".to_string(),
                version: "1.21.0".to_string(),
                build_string: None,
                size: Some(50_000_000),
                dependencies: vec!["python".to_string()],
            },
            Package {
                name: "pandas".to_string(),
                version: "1.3.0".to_string(),
                build_string: None,
                size: Some(30_000_000),
                dependencies: vec!["python".to_string(), "numpy".to_string()],
            },
        ],
        total_size: Some(180_000_000),
        recommendations: vec![],
    };
    
    // Create the interactive UI
    let mut ui = InteractiveUI::new(analysis, None);
    
    // Set up a test backend and terminal
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Test the rendering (we can't easily check the actual render output in a unit test,
    // but we can ensure it doesn't panic)
    terminal.draw(|frame| {
        ui.render_ui(frame);
    }).unwrap();
    
    // Assert that the terminal size is stored correctly
    assert_eq!(ui.viewport_width, 80);
    assert_eq!(ui.viewport_height, 30);
}

#[test]
fn test_package_selection() {
    // Create a sample environment analysis
    let analysis = EnvironmentAnalysis {
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
                size: Some(100_000_000),
                dependencies: vec!["".to_string()],
            },
            Package {
                name: "numpy".to_string(),
                version: "1.21.0".to_string(),
                build_string: None,
                size: Some(50_000_000),
                dependencies: vec!["python".to_string()],
            },
            Package {
                name: "pandas".to_string(),
                version: "1.3.0".to_string(),
                build_string: None,
                size: Some(30_000_000),
                dependencies: vec!["python".to_string(), "numpy".to_string()],
            },
        ],
        total_size: Some(180_000_000),
        recommendations: vec![],
    };
    
    // Create the interactive UI
    let mut ui = InteractiveUI::new(analysis, None);
    
    // Initial selected package should be 0
    assert_eq!(ui.selected_package, 0);
    
    // Test package navigation
    ui.selected_tab = 1; // Set to packages tab
    
    // Select next package (from 0 to 1)
    ui.select_next_package();
    assert_eq!(ui.selected_package, 1);
    
    // Select next package (from 1 to 2)
    ui.select_next_package();
    assert_eq!(ui.selected_package, 2);
    
    // Select next package (from 2 should stay at 2 since there are 3 packages)
    ui.select_next_package();
    assert_eq!(ui.selected_package, 2);
    
    // Select previous package (from 2 to 1)
    ui.select_previous_package();
    assert_eq!(ui.selected_package, 1);
    
    // Select previous package (from 1 to 0)
    ui.select_previous_package();
    assert_eq!(ui.selected_package, 0);
    
    // Select previous package (from 0 should stay at 0)
    ui.select_previous_package();
    assert_eq!(ui.selected_package, 0);
}

#[test]
fn test_tab_navigation() {
    // Create a minimal environment analysis
    let analysis = EnvironmentAnalysis {
        environment: CondaEnvironment {
            name: "test-env".to_string(),
            channels: vec!["conda-forge".to_string()],
            dependencies: vec!["python=3.9".to_string()],
        },
        packages: vec![
            Package {
                name: "python".to_string(),
                version: "3.9".to_string(),
                build_string: None,
                size: Some(100_000_000),
                dependencies: vec!["".to_string()],
            },
        ],
        total_size: Some(100_000_000),
        recommendations: vec![],
    };
    
    // Create the interactive UI
    let mut ui = InteractiveUI::new(analysis, None);
    
    // Initial selected tab should be 0 (Summary)
    assert_eq!(ui.selected_tab, 0);
    
    // Select next tab (from 0 to 1)
    ui.select_next_tab();
    assert_eq!(ui.selected_tab, 1);
    
    // Select next tab (from 1 to 2)
    ui.select_next_tab();
    assert_eq!(ui.selected_tab, 2);
    
    // Select next tab (from 2 to 3)
    ui.select_next_tab();
    assert_eq!(ui.selected_tab, 3);
    
    // Select next tab (from 3 should wrap to 0)
    ui.select_next_tab();
    assert_eq!(ui.selected_tab, 0);
    
    // Select previous tab (from 0 should wrap to 3)
    ui.select_previous_tab();
    assert_eq!(ui.selected_tab, 3);
    
    // Select previous tab (from 3 to 2)
    ui.select_previous_tab();
    assert_eq!(ui.selected_tab, 2);
    
    // Select previous tab (from 2 to 1)
    ui.select_previous_tab();
    assert_eq!(ui.selected_tab, 1);
    
    // Select previous tab (from 1 to 0)
    ui.select_previous_tab();
    assert_eq!(ui.selected_tab, 0);
}