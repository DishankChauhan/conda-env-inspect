use anyhow::{Context, Result};
use prettytable::{Cell, Row, Table};
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::models::EnvironmentAnalysis;
use crate::utils;

/// Export formats supported by the tool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Plain text format
    Text,
    /// JSON format
    Json,
    /// Markdown format
    Markdown,
    /// HTML format
    Html,
    /// CSV format
    Csv,
}

impl ExportFormat {
    /// Parse a string into an export format
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" | "txt" => Some(ExportFormat::Text),
            "json" => Some(ExportFormat::Json),
            "markdown" | "md" => Some(ExportFormat::Markdown),
            "html" => Some(ExportFormat::Html),
            "csv" => Some(ExportFormat::Csv),
            _ => None,
        }
    }
}

/// Export analysis data in the specified format
pub fn export_analysis<P: AsRef<Path>>(
    analysis: &EnvironmentAnalysis,
    format: ExportFormat,
    output_path: Option<P>,
) -> Result<()> {
    let content = match format {
        ExportFormat::Text => format_as_text(analysis),
        ExportFormat::Json => format_as_json(analysis)?,
        ExportFormat::Markdown => format_as_markdown(analysis),
        ExportFormat::Html => format_as_html(analysis),
        ExportFormat::Csv => format_as_csv(analysis),
    };
    
    if let Some(path) = output_path {
        let mut file = File::create(path)
            .with_context(|| "Failed to create output file")?;
        file.write_all(content.as_bytes())?;
    } else {
        // Write to stdout
        println!("{}", content);
    }
    
    Ok(())
}

/// Exports the environment analysis in a terminal-friendly format
fn export_terminal<P: AsRef<Path>>(
    analysis: &EnvironmentAnalysis,
    output_path: Option<P>,
) -> Result<()> {
    let mut table = Table::new();
    
    // Add header row
    table.add_row(Row::new(vec![
        Cell::new("Package"),
        Cell::new("Version"),
        Cell::new("Build"),
        Cell::new("Channel"),
        Cell::new("Pinned"),
        Cell::new("Outdated"),
    ]));
    
    // Add data rows
    for package in &analysis.packages {
        table.add_row(Row::new(vec![
            Cell::new(&package.name),
            Cell::new(package.version.as_deref().unwrap_or("N/A")),
            Cell::new(package.build.as_deref().unwrap_or("N/A")),
            Cell::new(package.channel.as_deref().unwrap_or("default")),
            Cell::new(if package.is_pinned { "Yes" } else { "No" }),
            Cell::new(if package.is_outdated { "Yes" } else { "No" }),
        ]));
    }
    
    // Add summary row
    table.add_row(Row::new(vec![
        Cell::new("TOTAL"),
        Cell::new(&format!("{} packages", analysis.packages.len())),
        Cell::new(""),
        Cell::new(""),
        Cell::new(&format!("{} pinned", analysis.pinned_count)),
        Cell::new(&format!("{} outdated", analysis.outdated_count)),
    ]));
    
    if let Some(size) = analysis.total_size {
        table.add_row(Row::new(vec![
            Cell::new("Size"),
            Cell::new(&utils::format_size(size)),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
            Cell::new(""),
        ]));
    }
    
    // Print the table
    let mut output = Vec::new();
    write!(output, "{}", table)?;
    
    // Add recommendations if available
    if !analysis.recommendations.is_empty() {
        writeln!(output, "\nRecommendations:")?;
        for (i, rec) in analysis.recommendations.iter().enumerate() {
            writeln!(output, "{}. {}", i + 1, rec)?;
        }
    }
    
    if let Some(path) = output_path {
        let mut file = File::create(path)?;
        file.write_all(&output)?;
    } else {
        io::stdout().write_all(&output)?;
    }
    
    Ok(())
}

/// Format analysis as plain text
fn format_as_text(analysis: &EnvironmentAnalysis) -> String {
    let mut output = String::new();
    
    // Environment info
    output.push_str(&format!("Environment: {}\n", analysis.name.as_deref().unwrap_or("unknown")));
    output.push_str(&format!("Packages: {}\n", analysis.packages.len()));
    
    if let Some(size) = analysis.total_size {
        output.push_str(&format!("Total size: {}\n", utils::format_size(size)));
    }
    
    output.push_str(&format!("Pinned packages: {}\n", analysis.pinned_count));
    output.push_str(&format!("Outdated packages: {}\n", analysis.outdated_count));
    
    // Recommendations
    if !analysis.recommendations.is_empty() {
        output.push_str("\nRecommendations:\n");
        for rec in &analysis.recommendations {
            output.push_str(&format!("- {}\n", rec));
        }
    }
    
    // Packages
    output.push_str("\nPackage list:\n");
    for package in &analysis.packages {
        let version = package.version.as_deref().unwrap_or("unknown");
        let status = if package.is_outdated {
            if let Some(latest) = &package.latest_version {
                format!("[outdated: {}]", latest)
            } else {
                "[outdated]".to_string()
            }
        } else if package.is_pinned {
            "[pinned]".to_string()
        } else {
            "".to_string()
        };
        
        output.push_str(&format!("- {} {} {}\n", package.name, version, status));
    }
    
    output
}

/// Format analysis as JSON
fn format_as_json(analysis: &EnvironmentAnalysis) -> Result<String> {
    serde_json::to_string_pretty(analysis)
        .with_context(|| "Failed to serialize analysis to JSON")
}

/// Format analysis as Markdown
fn format_as_markdown(analysis: &EnvironmentAnalysis) -> String {
    let mut output = String::new();
    
    // Environment info
    output.push_str(&format!("# Environment Analysis: {}\n\n", analysis.name.as_deref().unwrap_or("unknown")));
    output.push_str(&format!("- **Packages**: {}\n", analysis.packages.len()));
    
    if let Some(size) = analysis.total_size {
        output.push_str(&format!("- **Total size**: {}\n", utils::format_size(size)));
    }
    
    output.push_str(&format!("- **Pinned packages**: {}\n", analysis.pinned_count));
    output.push_str(&format!("- **Outdated packages**: {}\n", analysis.outdated_count));
    
    // Recommendations
    if !analysis.recommendations.is_empty() {
        output.push_str("\n## Recommendations\n\n");
        for rec in &analysis.recommendations {
            output.push_str(&format!("- {}\n", rec));
        }
    }
    
    // Packages
    output.push_str("\n## Package list\n\n");
    output.push_str("| Package | Version | Status |\n");
    output.push_str("|---------|---------|--------|\n");
    for package in &analysis.packages {
        let version = package.version.as_deref().unwrap_or("unknown");
        let status = if package.is_outdated {
            if let Some(latest) = &package.latest_version {
                format!("⚠️ Outdated (latest: {})", latest)
            } else {
                "⚠️ Outdated".to_string()
            }
        } else if package.is_pinned {
            "📌 Pinned".to_string()
        } else {
            "✅ Up-to-date".to_string()
        };
        
        output.push_str(&format!("| {} | {} | {} |\n", package.name, version, status));
    }
    
    output
}

/// Format analysis as HTML
fn format_as_html(analysis: &EnvironmentAnalysis) -> String {
    let mut output = String::new();
    
    // HTML header
    output.push_str("<!DOCTYPE html>\n");
    output.push_str("<html lang=\"en\">\n");
    output.push_str("<head>\n");
    output.push_str("  <meta charset=\"UTF-8\">\n");
    output.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    output.push_str("  <title>Conda Environment Analysis</title>\n");
    output.push_str("  <style>\n");
    output.push_str("    body { font-family: Arial, sans-serif; margin: 20px; }\n");
    output.push_str("    table { border-collapse: collapse; width: 100%; }\n");
    output.push_str("    th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
    output.push_str("    th { background-color: #f2f2f2; }\n");
    output.push_str("    tr:nth-child(even) { background-color: #f9f9f9; }\n");
    output.push_str("    .outdated { color: #e74c3c; }\n");
    output.push_str("    .pinned { color: #3498db; }\n");
    output.push_str("    .uptodate { color: #2ecc71; }\n");
    output.push_str("  </style>\n");
    output.push_str("</head>\n");
    output.push_str("<body>\n");
    
    // Environment info
    output.push_str(&format!("  <h1>Environment Analysis: {}</h1>\n", 
        analysis.name.as_deref().unwrap_or("unknown")));
    
    output.push_str("  <div class=\"summary\">\n");
    output.push_str(&format!("    <p><strong>Packages:</strong> {}</p>\n", analysis.packages.len()));
    
    if let Some(size) = analysis.total_size {
        output.push_str(&format!("    <p><strong>Total size:</strong> {}</p>\n", utils::format_size(size)));
    }
    
    output.push_str(&format!("    <p><strong>Pinned packages:</strong> {}</p>\n", analysis.pinned_count));
    output.push_str(&format!("    <p><strong>Outdated packages:</strong> {}</p>\n", analysis.outdated_count));
    output.push_str("  </div>\n");
    
    // Recommendations
    if !analysis.recommendations.is_empty() {
        output.push_str("  <h2>Recommendations</h2>\n");
        output.push_str("  <ul>\n");
        for rec in &analysis.recommendations {
            output.push_str(&format!("    <li>{}</li>\n", rec));
        }
        output.push_str("  </ul>\n");
    }
    
    // Packages
    output.push_str("  <h2>Package list</h2>\n");
    output.push_str("  <table>\n");
    output.push_str("    <tr>\n");
    output.push_str("      <th>Package</th>\n");
    output.push_str("      <th>Version</th>\n");
    output.push_str("      <th>Status</th>\n");
    output.push_str("    </tr>\n");
    
    for package in &analysis.packages {
        let version = package.version.as_deref().unwrap_or("unknown");
        let (status_class, status_text) = if package.is_outdated {
            if let Some(latest) = &package.latest_version {
                ("outdated", format!("Outdated (latest: {})", latest))
            } else {
                ("outdated", "Outdated".to_string())
            }
        } else if package.is_pinned {
            ("pinned", "Pinned".to_string())
        } else {
            ("uptodate", "Up-to-date".to_string())
        };
        
        output.push_str("    <tr>\n");
        output.push_str(&format!("      <td>{}</td>\n", package.name));
        output.push_str(&format!("      <td>{}</td>\n", version));
        output.push_str(&format!("      <td class=\"{}\">{}</td>\n", status_class, status_text));
        output.push_str("    </tr>\n");
    }
    
    output.push_str("  </table>\n");
    
    // HTML footer
    output.push_str("  <footer>\n");
    output.push_str("    <p><em>Generated by conda-env-inspect</em></p>\n");
    output.push_str("  </footer>\n");
    output.push_str("</body>\n");
    output.push_str("</html>\n");
    
    output
}

/// Format analysis as CSV
fn format_as_csv(analysis: &EnvironmentAnalysis) -> String {
    let mut output = String::new();
    
    // Header
    output.push_str("Package,Version,Channel,Size,Status,Latest Version\n");
    
    // Packages
    for package in &analysis.packages {
        let version = package.version.as_deref().unwrap_or("");
        let channel = package.channel.as_deref().unwrap_or("");
        let size = package.size.map_or("".to_string(), |s| utils::format_size(s));
        let status = if package.is_outdated {
            "outdated"
        } else if package.is_pinned {
            "pinned"
        } else {
            "up-to-date"
        };
        let latest = package.latest_version.as_deref().unwrap_or("");
        
        output.push_str(&format!("{},{},{},{},{},{}\n", 
            package.name, version, channel, size, status, latest));
    }
    
    output
}

/// Export data to yaml format
fn export_yaml<P: AsRef<Path>>(
    analysis: &EnvironmentAnalysis,
    output_path: Option<P>,
) -> Result<()> {
    let yaml_string = serde_yaml::to_string(analysis)?;
    
    match output_path {
        Some(path) => {
            let mut file = File::create(path)?;
            file.write_all(yaml_string.as_bytes())?;
        },
        None => {
            println!("{}", yaml_string);
        }
    }
    
    Ok(())
}

/// Export data to CSV format
fn export_csv<P: AsRef<Path>>(
    analysis: &EnvironmentAnalysis,
    output_path: Option<P>,
) -> Result<()> {
    match output_path {
        Some(path) => {
            // Write directly to file
            let mut wtr = csv::Writer::from_path(path)?;
            write_csv_data(&mut wtr, analysis)?;
            wtr.flush()?;
        },
        None => {
            // Write to memory buffer, then to stdout
            let mut wtr = csv::Writer::from_writer(Vec::new());
            write_csv_data(&mut wtr, analysis)?;
            let buffer = wtr.into_inner()?;
            let output = String::from_utf8(buffer)?;
            println!("{}", output);
        }
    }
    
    Ok(())
}

// Helper to write CSV data
fn write_csv_data<W: std::io::Write>(wtr: &mut csv::Writer<W>, analysis: &EnvironmentAnalysis) -> Result<()> {
    // Write header
    wtr.write_record(&["Name", "Version", "Channel", "Build", "Size", "Outdated", "Pinned"])?;
    
    // Write data
    for package in &analysis.packages {
        wtr.write_record(&[
            &package.name,
            package.version.as_deref().unwrap_or(""),
            package.channel.as_deref().unwrap_or(""),
            package.build.as_deref().unwrap_or(""),
            &package.size.map_or("".to_string(), |s| s.to_string()),
            &package.is_outdated.to_string(),
            &package.is_pinned.to_string(),
        ])?;
    }
    
    Ok(())
}

/// Export data to TOML format
fn export_toml<P: AsRef<Path>>(
    analysis: &EnvironmentAnalysis,
    output_path: Option<P>,
) -> Result<()> {
    // Convert to TOML (this is a simplified approach)
    let mut toml_string = String::new();
    
    if let Some(name) = &analysis.name {
        toml_string.push_str(&format!("name = \"{}\"\n\n", name));
    }
    
    toml_string.push_str(&format!("total_size = {}\n", analysis.total_size.unwrap_or(0)));
    toml_string.push_str(&format!("pinned_count = {}\n", analysis.pinned_count));
    toml_string.push_str(&format!("outdated_count = {}\n\n", analysis.outdated_count));
    
    toml_string.push_str("[[packages]]\n");
    for package in &analysis.packages {
        toml_string.push_str(&format!("name = \"{}\"\n", package.name));
        if let Some(version) = &package.version {
            toml_string.push_str(&format!("version = \"{}\"\n", version));
        }
        if let Some(channel) = &package.channel {
            toml_string.push_str(&format!("channel = \"{}\"\n", channel));
        }
        if let Some(build) = &package.build {
            toml_string.push_str(&format!("build = \"{}\"\n", build));
        }
        if let Some(size) = package.size {
            toml_string.push_str(&format!("size = {}\n", size));
        }
        if let Some(latest) = &package.latest_version {
            toml_string.push_str(&format!("latest_version = \"{}\"\n", latest));
        }
        toml_string.push_str(&format!("is_pinned = {}\n", package.is_pinned));
        toml_string.push_str(&format!("is_outdated = {}\n\n", package.is_outdated));
        
        toml_string.push_str("[[packages]]\n");
    }
    
    match output_path {
        Some(path) => {
            let mut file = File::create(path)?;
            file.write_all(toml_string.as_bytes())?;
        },
        None => {
            println!("{}", toml_string);
        }
    }
    
    Ok(())
}

