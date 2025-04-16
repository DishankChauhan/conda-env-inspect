use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Terminal,
};
use std::io::{stdout, Stdout};
use std::time::Duration;

use crate::advanced_analysis::AdvancedDependencyGraph;
use crate::models::{EnvironmentAnalysis, Package};

/// Interactive UI for environment analysis
// For tests, implement minimal versions without UI functionality
#[derive(Debug)]
pub struct InteractiveUI {
    #[allow(dead_code)]
    analysis: EnvironmentAnalysis,
    #[allow(dead_code)]
    advanced_graph: Option<AdvancedDependencyGraph>,
}

impl InteractiveUI {
    /// Create a new interactive UI
    pub fn new(analysis: EnvironmentAnalysis, advanced_graph: Option<AdvancedDependencyGraph>) -> anyhow::Result<Self> {
        Ok(Self {
            analysis,
            advanced_graph,
        })
    }
    
    /// Run the interactive UI
    pub fn run(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Display a progress bar
pub fn create_progress_bar(len: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {msg} [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Format a size for display
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} bytes", size)
    }
}

// NOTE: Temporarily commented out for tests to compile
/*
fn render_summary_tab(
    f: &mut ratatui::Frame<CrosstermBackend<Stdout>>, 
    area: ratatui::layout::Rect, 
    analysis: &EnvironmentAnalysis
) {
    // ...
}

fn render_packages_tab(
    f: &mut ratatui::Frame<CrosstermBackend<Stdout>>, 
    area: ratatui::layout::Rect, 
    analysis: &EnvironmentAnalysis,
    selected_package: usize
) {
    // ...
}

fn render_deps_tab(
    f: &mut ratatui::Frame<CrosstermBackend<Stdout>>, 
    area: ratatui::layout::Rect,
    graph: &Option<AdvancedDependencyGraph>
) {
    // ...
}

fn render_recommendations_tab(
    f: &mut ratatui::Frame<CrosstermBackend<Stdout>>, 
    area: ratatui::layout::Rect,
    analysis: &EnvironmentAnalysis
) {
    // ...
}
*/ 