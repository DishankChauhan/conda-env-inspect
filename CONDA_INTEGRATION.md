# Conda Integration Guide

This document explains how `conda-env-inspect` integrates with the Conda ecosystem to provide detailed environment analysis.

## Real Dependency Resolution

The tool now uses actual conda metadata to resolve package dependencies rather than mock data. This provides a more accurate representation of your environment's structure.

### How It Works

1. The tool runs `conda list --json` to retrieve detailed information about installed packages
2. It parses the dependencies from each package's metadata
3. For packages that are missing dependency info, it falls back to `conda list <package> --json`
4. The gathered dependency information is used to build a complete dependency graph

### Using It In Your Environment

For the dependency resolution to work correctly, the tool must be run from within an activated conda environment or with proper environment name specification:

```bash
# Activate your environment first
conda activate my-environment

# Then run conda-env-inspect
conda-env-inspect examples/environment.yml -g --graph-output deps.dot
```

## Real Environment Size Calculation

The tool calculates actual environment sizes by inspecting the conda environment directory on your file system.

### How It Works

1. Uses `conda env list --json` to locate the environment directory
2. Recursively scans all files in the environment directory
3. Sums up the sizes of all files to get the total environment size

### Using It In Your Environment

For accurate size calculation, make sure to specify the correct environment name:

```bash
# Check environment size
conda-env-inspect examples/environment.yml -c

# The environment name from the .yml file will be used to locate the directory
```

## Package Outdated Status

The tool checks for outdated packages by querying the Anaconda API for the latest available versions.

### How It Works

1. Parses version information from your environment file
2. Queries the Anaconda API for each package to get the latest version
3. Compares versions using semantic versioning rules
4. Identifies packages that have newer versions available

### Using It In Your Environment

To check for outdated packages:

```bash
# Check outdated packages
conda-env-inspect examples/environment.yml -c
```

## Redundant Package Detection

The tool can identify potentially redundant packages in your environment that aren't required as dependencies by other packages.

### How It Works

1. Builds a complete dependency graph of your environment
2. Identifies packages that are not dependencies of any other package
3. Excludes common development packages (like pytest, jupyter, etc.)
4. Reports the potentially redundant packages

### Using It In Your Environment

To detect redundant packages:

```bash
# Check for redundant packages (included in recommendations)
conda-env-inspect examples/environment.yml -r
```

## Troubleshooting

If you encounter issues with the conda integration:

1. **Missing Dependency Information**: Make sure conda is properly installed and in your PATH
2. **Environment Size Calculation Fails**: Verify that you have read access to the environment directory
3. **API Connectivity Issues**: Check your internet connection for package outdated status

For advanced debugging, you can run with more verbose output:

```bash
# Enable verbose output (for development)
RUST_LOG=debug cargo run -- examples/environment.yml -c -g --graph-output deps.dot
``` 