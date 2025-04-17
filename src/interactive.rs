use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Table, Row, Cell, canvas::Canvas},
    Terminal,
};
use std::io::{stdout, Stdout};
use std::collections::HashMap;
use std::cmp::max;

use crate::advanced_analysis::AdvancedDependencyGraph;
use crate::models::EnvironmentAnalysis;

/// Interactive UI for environment analysis
#[derive(Debug)]
pub struct InteractiveUI {
    analysis: EnvironmentAnalysis,
    advanced_graph: Option<AdvancedDependencyGraph>,
    selected_tab: usize,
    selected_package: usize,
    graph_scroll: (u16, u16),
    viewport_width: u16,
    viewport_height: u16,
}

impl InteractiveUI {
    /// Create a new interactive UI
    pub fn new(analysis: EnvironmentAnalysis, advanced_graph: Option<AdvancedDependencyGraph>) -> Result<Self> {
        Ok(Self {
            analysis,
            advanced_graph,
            selected_tab: 0,
            selected_package: 0,
            graph_scroll: (0, 0),
            viewport_width: 0,
            viewport_height: 0,
        })
    }
    
    /// Run the interactive UI
    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        
        loop {
            terminal.draw(|f| self.render_ui(f))?;
            
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Right => {
                        if self.selected_tab == 2 && self.advanced_graph.is_some() {
                            // In graph view, scroll right
                            self.graph_scroll.0 = self.graph_scroll.0.saturating_add(5);
                        } else {
                            self.selected_tab = (self.selected_tab + 1) % 4;
                        }
                    },
                    KeyCode::Left => {
                        if self.selected_tab == 2 && self.advanced_graph.is_some() {
                            // In graph view, scroll left
                            self.graph_scroll.0 = self.graph_scroll.0.saturating_sub(5);
                        } else {
                            self.selected_tab = (self.selected_tab + 3) % 4;
                        }
                    },
                    KeyCode::Down => {
                        if self.selected_tab == 1 {
                            // In packages tab
                            self.selected_package = (self.selected_package + 1) % self.analysis.packages.len();
                        } else if self.selected_tab == 2 && self.advanced_graph.is_some() {
                            // In graph view, scroll down
                            self.graph_scroll.1 = self.graph_scroll.1.saturating_add(3);
                        }
                    },
                    KeyCode::Up => {
                        if self.selected_tab == 1 {
                            // In packages tab
                            self.selected_package = (self.selected_package + self.analysis.packages.len() - 1) % self.analysis.packages.len();
                        } else if self.selected_tab == 2 && self.advanced_graph.is_some() {
                            // In graph view, scroll up
                            self.graph_scroll.1 = self.graph_scroll.1.saturating_sub(3);
                        }
                    },
                    KeyCode::Home => {
                        if self.selected_tab == 2 && self.advanced_graph.is_some() {
                            // Reset graph scroll position
                            self.graph_scroll = (0, 0);
                        }
                    },
                    _ => {}
                }
            }
        }
        
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        
        Ok(())
    }
    
    fn render_ui(&mut self, f: &mut ratatui::Frame<CrosstermBackend<Stdout>>) {
        // Save viewport size for scrolling calculations
        self.viewport_width = f.size().width;
        self.viewport_height = f.size().height;
        
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(f.size());
        
        let tabs = ["Summary", "Packages", "Dependencies", "Recommendations"];
        let tab_titles: Vec<Line> = tabs.iter().map(|t| Line::from(vec![Span::raw(*t)])).collect();
        let tabs = Tabs::new(tab_titles)
            .block(Block::default().title("Tabs").borders(Borders::ALL))
            .select(self.selected_tab)
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow));
        f.render_widget(tabs, chunks[0]);
        
        match self.selected_tab {
            0 => render_summary_tab(f, chunks[1], &self.analysis),
            1 => render_packages_tab(f, chunks[1], &self.analysis, self.selected_package),
            2 => self.render_deps_tab(f, chunks[1]),
            3 => render_recommendations_tab(f, chunks[1], &self.analysis),
            _ => unreachable!(),
        };
    }
    
    fn render_deps_tab(&self, f: &mut ratatui::Frame<CrosstermBackend<Stdout>>, area: Rect) {
        if let Some(graph) = &self.advanced_graph {
            // Split the area into two parts: graph visualization and details
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(7)].as_ref())
                .split(area);
            
            // Create a visual graph layout
            // Calculate position for each node in the graph
            let (positions_vec, max_width, max_height) = calculate_graph_layout_vec(graph);
            
            // Adjust scroll position based on content size
            let scroll_x = self.graph_scroll.0.min(max(0, max_width.saturating_sub(chunks[0].width)));
            let scroll_y = self.graph_scroll.1.min(max(0, max_height.saturating_sub(chunks[0].height)));
            
            // Create a visual canvas with the graph
            let canvas = Canvas::default()
                .block(Block::default().title("Dependency Graph").borders(Borders::ALL))
                .marker(symbols::Marker::Braille)
                .paint(move |ctx| {
                    // Get node and edge data ready for drawing
                    let edges = graph.graph.edge_indices().filter_map(|edge_idx| {
                        if let Some((from, to)) = graph.graph.edge_endpoints(edge_idx) {
                            let from_name = graph.graph[from].clone();
                            let to_name = graph.graph[to].clone();
                            Some((from_name, to_name))
                        } else {
                            None
                        }
                    }).collect::<Vec<_>>();
                    
                    // Create a lookup map for positions
                    let mut position_map = std::collections::HashMap::new();
                    for (idx, name, x, y) in &positions_vec {
                        position_map.insert(name.clone(), (*x, *y));
                    }
                    
                    // Draw edges first
                    for (from_name, to_name) in edges {
                        if let (Some(&(from_x, from_y)), Some(&(to_x, to_y))) = 
                            (position_map.get(&from_name), position_map.get(&to_name)) {
                            // Apply scroll offset
                            let x1 = from_x as f64 - scroll_x as f64;
                            let y1 = from_y as f64 - scroll_y as f64;
                            let x2 = to_x as f64 - scroll_x as f64;
                            let y2 = to_y as f64 - scroll_y as f64;
                            
                            // Draw arrow from dependent to dependency
                            ctx.draw(&ratatui::widgets::canvas::Line {
                                x1, y1, x2, y2, 
                                color: Color::Gray,
                            });
                            
                            // Draw arrowhead
                            let dx = x2 - x1;
                            let dy = y2 - y1;
                            let len = (dx * dx + dy * dy).sqrt();
                            if len > 0.0 {
                                let normalized_dx = dx / len;
                                let normalized_dy = dy / len;
                                let arrow_size = 0.5;
                                
                                // Calculate arrowhead points
                                let ax1 = x2 - arrow_size * (normalized_dx + normalized_dy * 0.5);
                                let ay1 = y2 - arrow_size * (normalized_dy - normalized_dx * 0.5);
                                let ax2 = x2 - arrow_size * (normalized_dx - normalized_dy * 0.5);
                                let ay2 = y2 - arrow_size * (normalized_dy + normalized_dx * 0.5);
                                
                                ctx.draw(&ratatui::widgets::canvas::Line {
                                    x1: x2, y1: y2, x2: ax1, y2: ay1,
                                    color: Color::Gray,
                                });
                                ctx.draw(&ratatui::widgets::canvas::Line {
                                    x1: x2, y1: y2, x2: ax2, y2: ay2,
                                    color: Color::Gray,
                                });
                            }
                        }
                    }
                    
                    // Draw nodes
                    for (_, name, x, y) in &positions_vec {
                        // Apply scroll offset
                        let x = *x as f64 - scroll_x as f64;
                        let y = *y as f64 - scroll_y as f64;
                        
                        // Use different colors for direct deps vs transitive deps
                        let color = if graph.direct_deps.contains(name) {
                            Color::Green
                        } else {
                            Color::Blue
                        };
                        
                        // Draw node
                        ctx.print(x, y, Span::styled(name.clone(), Style::default().fg(color)));
                    }
                })
                .x_bounds([0.0, chunks[0].width as f64])
                .y_bounds([0.0, chunks[0].height as f64]);
            
            f.render_widget(canvas, chunks[0]);
            
            // Render graph information and navigation help
            let node_count = graph.graph.node_count();
            let edge_count = graph.graph.edge_count();
            let conflict_count = graph.conflicts.len();
            
            let info_text = vec![
                Line::from(vec![
                    Span::raw("Nodes: "),
                    Span::styled(node_count.to_string(), Style::default().fg(Color::Green)),
                    Span::raw("  Edges: "),
                    Span::styled(edge_count.to_string(), Style::default().fg(Color::Blue)),
                    Span::raw("  Conflicts: "),
                    Span::styled(conflict_count.to_string(), Style::default().fg(Color::Red)),
                ]),
                Line::from(Span::raw("")),
                Line::from(vec![
                    Span::styled("Navigation: ", Style::default().fg(Color::Yellow)),
                    Span::raw("Arrow keys to move, Home to reset view")
                ]),
                Line::from(vec![
                    Span::styled("Legend: ", Style::default().fg(Color::Yellow)),
                    Span::styled("Direct deps ", Style::default().fg(Color::Green)),
                    Span::raw("/ "),
                    Span::styled("Transitive deps", Style::default().fg(Color::Blue)),
                ]),
            ];
            
            let info_paragraph = Paragraph::new(info_text)
                .block(Block::default().title("Graph Information").borders(Borders::ALL))
                .alignment(ratatui::layout::Alignment::Left);
            
            f.render_widget(info_paragraph, chunks[1]);
        } else {
            // Display a message when no graph is available
            let text = vec![
                Line::from(Span::raw("Dependency graph not available.")),
                Line::from(Span::raw("Generate it with the --advanced-graph flag.")),
            ];
            
            let paragraph = Paragraph::new(text)
                .block(Block::default().title("Dependency Graph").borders(Borders::ALL))
                .alignment(ratatui::layout::Alignment::Center);
            
            f.render_widget(paragraph, area);
        }
    }
}

/// Calculate a layout for the graph visualization returning a vector of node data
/// Each entry contains (node_index, name, x, y)
fn calculate_graph_layout_vec(graph: &AdvancedDependencyGraph) -> (Vec<(petgraph::graph::NodeIndex, String, u16, u16)>, u16, u16) {
    let mut positions_vec = Vec::new();
    
    // Organize nodes into layers based on dependencies
    let mut layers = Vec::new();
    let mut assigned = std::collections::HashSet::new();
    
    // First layer: nodes with no outgoing edges (leaf nodes)
    let mut layer = Vec::new();
    for node in graph.graph.node_indices() {
        let name = &graph.graph[node];
        if graph.graph.neighbors_directed(node, petgraph::Direction::Outgoing).count() == 0 {
            layer.push((node, name.clone()));
            assigned.insert(name.clone());
        }
    }
    if !layer.is_empty() {
        layers.push(layer);
    }
    
    // Add subsequent layers
    while assigned.len() < graph.graph.node_count() {
        let mut next_layer = Vec::new();
        
        for node in graph.graph.node_indices() {
            let name = &graph.graph[node];
            if assigned.contains(name) {
                continue;
            }
            
            // Check if all dependencies are already assigned
            let mut all_deps_assigned = true;
            for neighbor in graph.graph.neighbors_directed(node, petgraph::Direction::Outgoing) {
                let neighbor_name = &graph.graph[neighbor];
                if !assigned.contains(neighbor_name) {
                    all_deps_assigned = false;
                    break;
                }
            }
            
            if all_deps_assigned {
                next_layer.push((node, name.clone()));
                assigned.insert(name.clone());
            }
        }
        
        if next_layer.is_empty() {
            // If we can't assign more nodes normally, just add remaining nodes
            for node in graph.graph.node_indices() {
                let name = &graph.graph[node];
                if !assigned.contains(name) {
                    next_layer.push((node, name.clone()));
                    assigned.insert(name.clone());
                }
            }
        }
        
        if !next_layer.is_empty() {
            layers.push(next_layer);
        } else {
            break;
        }
    }
    
    // Assign positions based on layers
    let horizontal_spacing = 15;
    let vertical_spacing = 4;
    let mut max_width = 0;
    let mut max_height = 0;
    
    for (layer_idx, layer) in layers.iter().enumerate() {
        let y = layer_idx as u16 * vertical_spacing + 2;
        
        // Center the nodes in each layer
        for (node_idx, (node, name)) in layer.iter().enumerate() {
            let x = node_idx as u16 * horizontal_spacing + 2;
            positions_vec.push((*node, name.clone(), x, y));
            max_width = max(max_width, x + name.len() as u16);
            max_height = max(max_height, y + 1);
        }
    }
    
    (positions_vec, max_width, max_height)
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

fn render_summary_tab(
    f: &mut ratatui::Frame<CrosstermBackend<Stdout>>, 
    area: ratatui::layout::Rect, 
    analysis: &EnvironmentAnalysis
) {
    let total_packages = analysis.packages.len();
    let total_size = analysis.total_size.unwrap_or(0);
    let outdated_packages = analysis.packages.iter().filter(|p| p.is_outdated).count();
    let pinned_packages = analysis.packages.iter().filter(|p| p.is_pinned).count();
    
    let summary_text = vec![
        Line::from(vec![
            Span::raw("Total packages: "),
            Span::styled(total_packages.to_string(), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::raw("Total size: "),
            Span::styled(format_size(total_size), Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::raw("Outdated packages: "),
            Span::styled(outdated_packages.to_string(), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::raw("Pinned packages: "),
            Span::styled(pinned_packages.to_string(), Style::default().fg(Color::Cyan)),
        ]),
    ];
    
    let summary_paragraph = Paragraph::new(summary_text)
        .block(Block::default().title("Summary").borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    f.render_widget(summary_paragraph, area);
}

fn render_packages_tab(
    f: &mut ratatui::Frame<CrosstermBackend<Stdout>>, 
    area: ratatui::layout::Rect, 
    analysis: &EnvironmentAnalysis,
    selected_package: usize
) {
    let packages = &analysis.packages;
    
    let header_cells = ["Name", "Version", "Channel", "Size"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Green)));
    
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::Black))
        .height(1);
    
    let rows = packages.iter().enumerate().map(|(i, pkg)| {
        let style = if i == selected_package {
            Style::default().bg(Color::Blue).fg(Color::Black)
        } else {
            Style::default()
        };
        
        Row::new(vec![
            Cell::from(pkg.name.as_str()),
            Cell::from(pkg.version.as_deref().unwrap_or("N/A")),
            Cell::from(pkg.channel.as_deref().unwrap_or("N/A")),
            Cell::from(format_size(pkg.size.unwrap_or(0))),
        ]).style(style)
    });
    
    let table = Table::new(rows)
        .header(header)
        .block(Block::default().title("Packages").borders(Borders::ALL))
        .widths(&[
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ]);
    
    f.render_widget(table, area);
}

fn render_recommendations_tab(
    f: &mut ratatui::Frame<CrosstermBackend<Stdout>>, 
    area: ratatui::layout::Rect,
    analysis: &EnvironmentAnalysis
) {
    let recommendations = &analysis.recommendations;
    
    let items: Vec<ListItem> = recommendations.iter().map(|rec| {
        let mut lines = vec![Line::from(Span::raw(&rec.description))];
        
        if let Some(ref details) = rec.details {
            lines.push(Line::from(Span::raw(details)));
        }
        
        lines.push(Line::from(vec![
            Span::raw("Value: "),
            Span::styled(&rec.value, Style::default().fg(Color::Green)),
        ]));
        
        ListItem::new(lines).style(Style::default())
    }).collect();
    
    let list = List::new(items)
        .block(Block::default().title("Recommendations").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black));
    
    f.render_widget(list, area);
}

/// The original calculate_graph_layout function is no longer used but kept for reference
fn calculate_graph_layout(graph: &AdvancedDependencyGraph) -> (HashMap<String, (u16, u16)>, u16, u16) {
    let mut positions = HashMap::new();
    
    // Function implementation is no longer used, so we leave it empty
    // to avoid duplication of logic
    
    (positions, 0, 0)
} 