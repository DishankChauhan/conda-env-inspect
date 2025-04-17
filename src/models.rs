use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Represents a complete Conda environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondaEnvironment {
    /// Name of the environment
    pub name: Option<String>,
    /// Conda channels to use
    #[serde(default)]
    pub channels: Vec<String>,
    /// Dependencies (packages) in the environment
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    /// Additional properties not explicitly modeled
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

/// Represents a dependency in a Conda environment.
/// Can be a simple string like "numpy=1.19.2" or a complex specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    /// Simple package spec as a string (e.g., "numpy=1.19.2")
    Simple(String),
    /// Complex dependency specification
    Complex(ComplexDependency),
}

/// Represents a complex dependency specification with pip packages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexDependency {
    /// Name of the package, like "pip"
    pub name: Option<String>,
    /// The pip packages to install
    pub pip: Option<Vec<String>>,
    /// Additional properties not explicitly modeled
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

/// Represents a parsed package with its details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Name of the package
    pub name: String,
    /// Version of the package
    pub version: Option<String>,
    /// Build string (if available)
    pub build: Option<String>,
    /// Channel the package comes from
    pub channel: Option<String>,
    /// Size of the package (if available)
    pub size: Option<u64>,
    /// Whether the package version is pinned
    pub is_pinned: bool,
    /// Whether the package is outdated
    pub is_outdated: bool,
    /// Latest available version (if known)
    pub latest_version: Option<String>,
}

/// Represents a recommendation for environment optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Description of the recommendation
    pub description: String,
    /// Numerical value associated with the recommendation (e.g., potential size reduction)
    pub value: String,
    /// Optional detailed explanation
    pub details: Option<String>,
}

impl fmt::Display for Recommendation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (Value: {})", self.description, self.value)
    }
}

/// Represents the analysis results for an environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentAnalysis {
    /// Name of the environment
    pub name: Option<String>,
    /// Parsed packages in the environment
    pub packages: Vec<Package>,
    /// Total size of all packages combined
    pub total_size: Option<u64>,
    /// Count of pinned packages
    pub pinned_count: usize,
    /// Count of outdated packages
    pub outdated_count: usize,
    /// Recommendations for environment optimization
    #[serde(default)]
    pub recommendations: Vec<Recommendation>,
}
