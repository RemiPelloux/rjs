# RJS (Rust JavaScript Package Manager)

**Goal:** Build a modern, fast, and secure npm alternative written in Rust.

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
