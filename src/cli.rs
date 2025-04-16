use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum OutputFormat {
    #[clap(name = "text")]
    Text,
    #[clap(name = "json")]
    Json,
    #[clap(name = "yaml")]
    Yaml,
    #[clap(name = "csv")]
    Csv,
    #[clap(name = "markdown")]
    Markdown,
    #[clap(name = "toml")]
    Toml,
}

#[derive(Parser, Debug)]
#[clap(
    name = "conda-env-inspect",
    version,
    author = "Dishank Chauhan <dishankchauhan@gmail.com>",
    about = "A tool for analyzing Conda environment files",
    long_about = "A Rust-based CLI tool for analyzing Conda environment files, identifying dependencies, and providing optimization recommendations."
)]
pub struct Cli {
    /// Path to the Conda environment file (environment.yml, environment.yaml, or conda-lock.yml)
    #[clap(default_value = "environment.yml")]
    pub file: PathBuf,

    /// Format for output data (text, json, yaml, csv, markdown, toml)
    #[clap(short, long, value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// Output file path (if not specified, output will be written to stdout)
    #[clap(short, long)]
    pub output: Option<PathBuf>,

    /// Check for outdated packages
    #[clap(short, long)]
    pub check_outdated: bool,

    /// Flag pinned packages in the output
    #[clap(short = 'p', long)]
    pub flag_pinned: bool,

    /// Generate a dependency graph (requires graphviz dot command)
    #[clap(short, long)]
    pub generate_graph: bool,

    /// Output path for the dependency graph (required if --generate-graph is used)
    #[clap(short = 'G', long)]
    pub graph_output: Option<PathBuf>,

    /// Generate optimization recommendations
    #[clap(short = 'r', long)]
    pub generate_recommendations: bool,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Analyze conda environment file
    Analyze {
        /// Path to the Conda environment file
        #[clap(default_value = "environment.yml")]
        file: PathBuf,

        /// Check for outdated packages
        #[clap(short, long)]
        check_outdated: bool,

        /// Flag pinned packages in the output
        #[clap(short = 'p', long)]
        flag_pinned: bool,

        /// Generate a dependency graph
        #[clap(short, long)]
        generate_graph: bool,

        /// Generate optimization recommendations
        #[clap(short = 'r', long)]
        generate_recommendations: bool,

        /// Output path for the dependency graph
        #[clap(short = 'G', long)]
        graph_output: Option<PathBuf>,
        
        /// Use interactive TUI mode
        #[clap(short, long)]
        interactive: bool,
        
        /// Generate advanced dependency graph with conflict detection
        #[clap(short = 'a', long)]
        advanced_graph: bool,
    },
    
    /// Export environment analysis in various formats
    Export {
        /// Path to the Conda environment file
        #[clap(default_value = "environment.yml")]
        file: PathBuf,
        
        /// Format for output data
        #[clap(short, long, value_enum, default_value = "text")]
        format: OutputFormat,
        
        /// Output file path (if not specified, output will be written to stdout)
        #[clap(short = 'o', long)]
        output: Option<PathBuf>,
    },
    
    /// Generate dependency graph
    Graph {
        /// Path to the Conda environment file
        #[clap(default_value = "environment.yml")]
        file: PathBuf,
        
        /// Output path for the graph
        #[clap(short = 'o', long, default_value = "dependency_graph.dot")]
        output: PathBuf,
        
        /// Use advanced graph generation with conflict detection
        #[clap(short = 'a', long)]
        advanced: bool,
    },
    
    /// Generate optimization recommendations for environment
    Recommend {
        /// Path to the Conda environment file
        #[clap(default_value = "environment.yml")]
        file: PathBuf,
        
        /// Check for outdated packages
        #[clap(short, long)]
        check_outdated: bool,
    },
    
    /// Launch interactive TUI mode
    Interactive {
        /// Path to the Conda environment file
        #[clap(default_value = "environment.yml")]
        file: PathBuf,
        
        /// Check for outdated packages
        #[clap(short, long)]
        check_outdated: bool,
        
        /// Generate advanced dependency graph with conflict detection
        #[clap(short = 'a', long)]
        advanced_graph: bool,
    },
    
    /// Check for known vulnerabilities in packages
    Vulnerabilities {
        /// Path to the Conda environment file
        #[clap(default_value = "environment.yml")]
        file: PathBuf,
    },
}
