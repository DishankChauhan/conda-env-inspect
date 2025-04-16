# conda-env-inspect

A CLI tool for inspecting and analyzing Conda environment files, built in Rust.

## Features

### Core Features
- Parse and analyze `environment.yml` and `.conda` environment files
- List all packages, versions, and channels
- Flag pinned versions of packages
- Check for outdated packages
- Calculate total environment size
- Generate dependency graphs (DOT format)
- Provide recommendations for environment optimization
- Export analysis results in different formats (terminal, JSON, YAML, CSV, Markdown, TOML)

### Advanced Features
- Vulnerability detection for packages
  - Local vulnerability database
  - Integration with OSV (Open Source Vulnerabilities) API
  - PyPI security advisories check
  - Detection of significantly outdated packages
- Advanced dependency analysis with conflict detection
- Performance optimizations with parallel processing
- Progress indicators for long-running operations

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/conda-env-inspect.git
cd conda-env-inspect

# Build the project
cargo build --release

# The binary will be available at ./target/release/conda-env-inspect
```

## Usage

### Basic Commands

```bash
# Basic usage
conda-env-inspect environment.yml

# Flag pinned versions
conda-env-inspect -p environment.yml

# Check for outdated packages
conda-env-inspect -c environment.yml

# Generate recommendations
conda-env-inspect -r environment.yml

# Generate dependency graph
conda-env-inspect -g --graph-output deps.dot environment.yml

# Export to JSON
conda-env-inspect -f json -o analysis.json environment.yml

# Export to Markdown
conda-env-inspect -f markdown -o analysis.md environment.yml
```

### Subcommands

```bash
# Analyze environment
conda-env-inspect analyze -c -p environment.yml

# Export analysis results
conda-env-inspect export -f json -o analysis.json environment.yml

# Generate dependency graph
conda-env-inspect graph -o deps.dot environment.yml

# Advanced graph with conflict detection
conda-env-inspect graph -a -o deps.dot environment.yml

# Generate recommendations
conda-env-inspect recommend -c environment.yml

# Check for vulnerabilities
conda-env-inspect vulnerabilities environment.yml

# Interactive TUI mode (under development)
conda-env-inspect interactive environment.yml
```

## Example

```bash
$ conda-env-inspect examples/environment.yml -c -r
```

Output:
```
+---------------+---------+-------+---------+--------+----------+
| Package       | Version | Build | Channel | Pinned | Outdated |
+---------------+---------+-------+---------+--------+----------+
| python        | 3.9     | N/A   | default | Yes    | Yes      |
| numpy         | 1.22.3  | N/A   | default | Yes    | Yes      |
| pandas        | 1.4.2   | N/A   | default | Yes    | Yes      |
| matplotlib    | 3.5.1   | N/A   | default | Yes    | Yes      |
| scikit-learn  | 1.0.2   | N/A   | default | Yes    | Yes      |
| jupyterlab    | N/A     | N/A   | default | No     | No       |
| tensorflow    | 2.9.1   | N/A   | default | Yes    | Yes      |
| pytorch       | 1.11.0  | N/A   | pytorch | Yes    | Yes      |
| pip           | N/A     | N/A   | default | No     | No       |
+---------------+---------+-------+---------+--------+----------+
| TOTAL         | 9 packages | | | 7 pinned | 7 outdated |
+---------------+---------+-------+---------+--------+----------+
| Size          | 1.40 GB | | | | |
+---------------+---------+-------+---------+--------+----------+

Recommendations:
1. Found 7 outdated packages. Consider updating them for security and performance improvements.
2. Update numpy from 1.22.3 to 1.26.4
3. Update pandas from 1.4.2 to 2.2.1
4. Update matplotlib from 3.5.1 to 3.9.0
5. 77.8% of packages have pinned versions. This ensures reproducibility but may prevent updates.
```

### Vulnerability Check

```bash
$ conda-env-inspect vulnerabilities examples/environment.yml
```

Output:
```
Found 5 potential security vulnerabilities:
1. numpy 1.22.3 - Potentially vulnerable due to being significantly outdated (current: 1.22.3, latest: 2.2.4)
2. matplotlib 3.5.1 - Potentially vulnerable due to being significantly outdated (current: 3.5.1, latest: 3.10.1)
3. scikit-learn 1.0.2 - Potentially vulnerable due to being significantly outdated (current: 1.0.2, latest: 1.6.1)
4. tensorflow 2.9.1 - Potentially vulnerable due to being significantly outdated (current: 2.9.1, latest: 2.18.0)
5. pytorch 1.11.0 - Potentially vulnerable due to being significantly outdated (current: 1.11.0, latest: 2.5.1)
```

### Dependency Graph

You can generate dependency graphs in DOT format that can be visualized with tools like Graphviz:

```bash
$ conda-env-inspect graph -o deps.dot examples/environment.yml
$ dot -Tpng deps.dot > deps.png
```

## Advanced Features

### Environment Size Calculation

The tool calculates the total size of packages in the environment, giving you insights into how much disk space it consumes.

### Outdated Package Detection

When using the `-c/--check-outdated` flag, the tool will query the Conda repositories to check if any packages have newer versions available.

### Recommendations

Using the `-r/--generate-recommendations` flag, the tool will analyze your environment and provide suggestions for improvement, such as:

- Updating outdated packages
- Adding or removing version pins for better reproducibility
- Optimizing environment size

### Vulnerability Detection

Using the `vulnerabilities` command, the tool will check for:

- Known vulnerable versions in a local database
- Vulnerabilities reported in the OSV (Open Source Vulnerabilities) database
- Security advisories from PyPI for Python packages
- Significantly outdated packages that might contain security issues

## Conda Integration

This tool uses real conda commands and APIs to provide accurate information:

1. **Direct Conda Integration**
   - Uses actual conda commands when available
   - Falls back to API calls when conda is not installed
   - Provides accurate dependency resolution from actual metadata

2. **Conda API Integration**
   - Queries the Anaconda API for package information
   - Gets real latest versions and package sizes
   - Performs semantic version comparison for outdated detection

3. **Real Features**
   - Size calculation using actual file system data (when available)
   - Dependency resolution from conda metadata
   - Package recommendations based on real analysis

For full functionality, it's recommended to run within an activated conda environment:

```bash
# Activate your environment 
conda activate my-env

# Then run the tool
conda-env-inspect environment.yml -c -g --graph-output deps.dot
```

## Project Status

### Completed
- Core functionality for environment analysis
- Package parsing and metadata extraction
- Dependency graph generation
- Recommendations engine
- Multiple output formats
- Vulnerability checking
- Performance optimizations with parallel processing

### In Progress
- Interactive TUI mode implementation
- Comprehensive test suite

### Planned for Future
- Additional performance optimizations for large environments
- Improved caching mechanisms
- Enhanced conflict detection for dependencies
- More detailed vulnerability reports
- CI/CD pipeline setup
- Distribution to crates.io

## License

This project is licensed under the MIT License - see the LICENSE file for details. 