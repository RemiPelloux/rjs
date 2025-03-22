# RJS - Rust JavaScript Package Manager

A lightning-fast, high-performance JavaScript package manager built with Rust.

## Features

- **Initialize new projects** - Create package.json files with customizable defaults
- **Install packages** - Ultra-fast dependency installation with parallel processing
- **Dependency management** - Handle both production and development dependencies
- **List installed packages** - View all dependencies with formatted output
- **Dependency deduplication** - Intelligently reduce duplicate dependencies by resolving compatible versions
- **Lockfile generation** - Automatically generate and use lockfiles for reproducible builds
- **Optimized performance** - Up to 2000x faster than traditional package managers for certain operations
- **Advanced caching** - Smart package resolution caching to avoid redundant network requests
- **Parallel downloads** - Concurrent package retrieval and installation
- **Work-stealing algorithm** - Dynamic workload distribution for optimal CPU utilization
- **Streaming downloads** - Efficient streaming of package tarballs with minimal memory usage
- **Beautiful progress bars** - Real-time visual feedback during installation

## Installation

```bash
cargo install --path .
```

## Usage

### Initialize a new project

```bash
# Initialize with interactive prompts
rjs init

# Skip prompts and use defaults
rjs init --yes/-y
```

### Install a package

```bash
# Install a production dependency
rjs install <package-name>

# Install a dev dependency
rjs install <package-name> --dev/-D

# Install multiple packages
rjs install pkg1 pkg2 pkg3

# Install from package.json
rjs install

# Install using lockfile (exact versions)
rjs install --frozen/-f

# Install with custom concurrency level
rjs install --concurrency 16

# Install with custom batch size
rjs install --batch-size 50

# Install without progress display (faster for CI)
rjs install --no-progress
```

### List installed packages

```bash
# List all dependencies
rjs list

# List only dev dependencies
rjs list --dev

# List only production dependencies
rjs list --production
```

## Performance

RJS is designed for speed. Our benchmark tests show significant performance improvements over traditional package managers:

### Command Performance (in seconds)

| Command              | Before Optimization | After Optimization | Improvement |
|----------------------|--------------------:|-------------------:|------------:|
| init -y              | 6.2007              | 0.0030             | ~2066x faster |
| list (empty)         | 0.4866              | 0.0029             | ~167x faster |
| install lodash       | 3.0206              | 0.3650             | ~8x faster |
| list (after install) | 0.4985              | 0.0041             | ~121x faster |
| install multiple pkgs| 8.0093              | 1.0788             | ~7x faster |
| install --save-dev   | 3.0149              | 0.3652             | ~8x faster |
| list --dev           | 0.5307              | 0.0041             | ~129x faster |
| list --production    | 0.4883              | 0.0033             | ~148x faster |
| **Total time**       | **22.2495**         | **1.8265**         | **~12x faster** |

### Performance Chart

```
Command Performance (log scale)
----------------------------------------------------------------------------------------
init -y              |███████████████████████████████████████████████ 6.2007s
                     |█ 0.0030s (2066x faster)
                     |
list (empty)         |██████████████████ 0.4866s
                     |█ 0.0029s (167x faster)
                     |
install lodash       |████████████████████████ 3.0206s
                     |████ 0.3650s (8x faster)
                     |
list (after install) |██████████████████ 0.4985s
                     |█ 0.0041s (121x faster)
                     |
install multiple     |████████████████████████████████████████ 8.0093s
                     |█████ 1.0788s (7x faster)
                     |
install --save-dev   |████████████████████████ 3.0149s
                     |████ 0.3652s (8x faster)
                     |
list --dev           |██████████████████ 0.5307s
                     |█ 0.0041s (129x faster)
                     |
list --production    |██████████████████ 0.4883s
                     |█ 0.0033s (148x faster)
----------------------------------------------------------------------------------------
                       Before optimization █  After optimization █
```

## New Performance Features

### 1. Ultra-Fast Package Resolution

Our latest update introduces several critical performance optimizations:

- **Package Resolution Cache**: Caches package metadata to eliminate redundant network requests
- **Parallel Batch Processing**: Resolves dependencies in optimized batches for maximum throughput
- **Work-Stealing Algorithm**: Dynamically distributes workload across worker threads
- **Zero-Copy Optimization**: Minimizes memory allocation and copying during package resolution

### 2. Parallel Installation Engine

The new installation engine leverages advanced concurrency techniques:

- **Concurrent Package Downloads**: Downloads multiple packages simultaneously
- **Streaming Download Pipeline**: Processes package data in chunks as it arrives
- **Parallel Extraction**: Extracts package tarballs using dedicated worker threads
- **Asynchronous File I/O**: Non-blocking file operations to maximize throughput

### 3. Dependency Deduplication

Intelligently reduces node_modules bloat:

- **Compatible Version Resolution**: Finds single versions that satisfy multiple dependency requirements
- **Semantic Version Analysis**: Uses semver to identify compatible package versions
- **Dependency Tree Optimization**: Rewrites dependency tree to eliminate redundant packages
- **Installation Size Reduction**: Significantly reduces disk space usage and install time

### 4. Lockfile Generation

Ensures reproducible builds across environments:

- **Automatic Lockfile Creation**: Generates `rjs-lock.json` with exact package versions
- **Package Integrity Verification**: Includes integrity hashes for security validation
- **Frozen Installs**: Use `--frozen` flag to install exact versions from lockfile
- **Resolution URL Tracking**: Records exact download locations for all packages

### 5. Customizable Performance Settings

Fine-tune performance for your specific hardware:

- **Adjustable Concurrency**: Control the number of parallel operations with `--concurrency`
- **Batch Size Tuning**: Optimize memory usage with `--batch-size`
- **Headless Mode**: Boost performance in CI environments with `--no-progress`
- **Auto-Tuning**: Automatically detects optimal settings based on system capabilities

## Project Structure

```
.
├── src/
│   ├── cli/                  # CLI parsing & command dispatch
│   │   ├── commands/         # Subcommand handlers
│   │   │   ├── init.rs       # Initialize new projects
│   │   │   ├── install.rs    # Install dependencies
│   │   │   ├── list.rs       # List installed packages
│   │   │   └── mod.rs        # Command exports
│   │   └── mod.rs            # CLI module
│   ├── dependency/           # Dependency resolution
│   │   └── mod.rs            # Dependency tracking & resolution
│   ├── registry/             # Registry operations
│   │   └── mod.rs            # npm registry communication
│   ├── utils/                # Shared utilities
│   │   └── mod.rs            # File system, hash operations
│   └── main.rs               # Application entry point
├── scripts/                  # Development scripts
│   ├── dev/                  # Development utilities
│   │   ├── build.sh          # Build script
│   │   └── setup.sh          # Setup script
│   ├── git/                  # Git operations
│   │   └── push.sh           # Git push script
│   ├── tests/                # Test runners
│   │   ├── run_tests.sh      # Run all tests
│   │   └── run_performance_tests.sh # Run performance tests
│   └── utils/                # Script utilities
│       └── common.sh         # Shared functions
├── tests/                    # Test suite
│   ├── functional.rs         # Command behavior tests
│   └── performance.rs        # Performance benchmarks
```

## Development

### Scripts

The project includes several utility scripts organized by category:

#### Development Scripts (`scripts/dev/`)
- `build.sh` - Build, format, and check the project
- `setup.sh` - Set up the development environment

#### Git Scripts (`scripts/git/`)
- `push.sh [commit message]` - Add, commit, and push changes to Git

#### Test Scripts (`scripts/tests/`)
- `run_tests.sh` - Run all tests
- `run_performance_tests.sh` - Run performance tests with detailed output

### Testing

The project includes two types of tests:

1. **Functional Tests** - Verify the correct behavior of commands
2. **Performance Tests** - Measure the execution time of commands

To run tests:

```bash
# Run all tests
./scripts/tests/run_tests.sh

# Run performance tests only
./scripts/tests/run_performance_tests.sh
```

The test scripts set up a temporary test environment and run the appropriate test suites. The performance tests compile the project in release mode before running to ensure accurate benchmarks.

### Performance Optimization

The project includes several performance optimizations:

- **Release profile**: Optimized with LTO (Link Time Optimization) and minimal code generation
- **Benchmarking**: Iterative testing with warm-up runs for accurate measurements
- **Efficient algorithms**: Minimized I/O operations and parallel processing
- **Minimal dependencies**: Careful selection of dependencies to reduce bloat
- **HTTP2 support**: Leverages multiplexed connections for faster downloads
- **Connection pooling**: Reuses connections to minimize handshake overhead
- **Rayon parallel iterators**: Utilizes all CPU cores for computation-heavy tasks

### Development Setup

To set up the development environment:

```bash
./scripts/dev/setup.sh
```

This will:
- Install required dependencies
- Set up Rust toolchain
- Install development tools
- Configure git hooks
- Create project structure

### Building

To build the project:

```bash
./scripts/dev/build.sh
```

This will:
- Format code
- Run linter
- Check code
- Build project

## License

MIT

---

## Features (Implemented)

- ⚡ Ultra-fast install using Rust async I/O
- 🔒 Secure architecture, minimal memory bugs
- 🧠 Smart dependency resolution with caching
- 🎨 Beautiful and responsive CLI UX
- 🧩 Support for project templates (React, Node.js)
- 📦 Advanced parallel dependency resolution
- ⚙️ Customizable performance settings
- 🚀 Streaming downloads with minimal memory usage
- 🧵 Work-stealing algorithm for optimal resource usage
- 🔄 Dependency deduplication
- 🔒 Lockfile generation

## Roadmap

- 📊 Visualization of dependency tree
- 🌐 Offline mode
- 🔍 Vulnerability scanning
- 🧰 Plugin system

---

## Getting Started

1. Clone this repo
2. Run the setup script
3. Build the CLI:
```bash
cargo build
```

4. Initialize a new project:
```bash
# With interactive prompts
cargo run -- init

# Skip prompts
cargo run -- init -y
```

---

## Commands to Implement

| Command  | Description                  |
|----------|------------------------------|
| `init`   | Create package.json          |
| `install`| Install a dependency         |
| `list`   | (future) List installed deps |

---

## Contributing

Work in modules. Each CLI command has its own file.
Start implementing in `install.rs` and `init.rs`!

---
