# RJS - Rust JavaScript Package Manager

A lightweight JavaScript package manager built with Rust.

## Features

- Initialize new projects
- Install packages
- List installed dependencies
- Manage development dependencies

## Installation

```
cargo install --path .
```

## Usage

### Initialize a new project

```
rjs init [--yes/-y]
```

### Install a package

```
rjs install <package-name> [--dev/-D]
```

### List installed packages

```
rjs list
```

## Project Structure

```
.
├── src/
│   ├── cli/
│   │   ├── commands/
│   │   │   ├── init.rs
│   │   │   ├── install.rs
│   │   │   ├── list.rs
│   │   │   └── mod.rs
│   │   └── mod.rs
│   ├── dependency/
│   │   └── mod.rs
│   ├── package/
│   │   └── mod.rs
│   ├── registry/
│   │   └── mod.rs
│   ├── utils/
│   │   └── mod.rs
│   └── main.rs
├── tests/
│   ├── functional.rs
│   └── performance.rs
└── scripts/
    ├── dev/
    │   ├── build.sh
    │   └── setup.sh
    ├── git/
    │   └── push.sh
    ├── tests/
    │   ├── run_tests.sh
    │   └── run_performance_tests.sh
    └── utils/
        └── common.sh
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

## Features (Planned)
- ⚡ Ultra-fast install using Rust async I/O
- 🔒 Secure architecture, minimal memory bugs
- 🧠 Deterministic lockfile generation
- 🎨 Beautiful and responsive CLI UX

---

## Project Structure

```
src/
├── cli/            # CLI parsing & command dispatch
│   └── commands/   # Subcommand handlers: install, init
├── dependency/     # Dependency resolution, lockfile generation
├── registry/       # Handles communication with npm registry
├── utils/          # Shared FS and HTTP utilities
└── main.rs         # Entrypoint
```

---

## Getting Started

1. Clone this repo
2. Run the setup script
3. Build the CLI:
```bash
cargo build
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
