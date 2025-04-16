pub mod advanced_analysis;
pub mod analysis;
pub mod cli;
pub mod conda_api;
pub mod exporters;
pub mod interactive;
pub mod models;
pub mod parsers;
pub mod performance;
pub mod utils;

// Re-export commonly used modules and types
pub use models::{Package, EnvironmentAnalysis};
pub use parsers::parse_environment_file;

// Make these functions public in their modules
pub use analysis::generate_recommendations; 