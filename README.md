# Disk Usage TUI Analyzer

A terminal-based disk usage analyzer that helps you visualize and explore disk space usage in a simple, interactive interface.

## Features

- 🖥️ Terminal-based user interface
- 📊 Visualize disk usage by directory
- ⚡ Fast scanning with parallel processing
- 🎨 Color-coded output
- 🔄 Sort by name or size
- 🖱️ Intuitive keyboard navigation

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/raghav-rama/disk-usage-analyzer-tui/releases) page.

### From Source

1. Install Rust from [rustup.rs](https://rustup.rs/)
2. Clone this repository
3. Build and install:
   ```bash
   cargo install --path .
   ```

## Usage

```bash
# Analyze current directory
disk-usage-tui

# Analyze specific directory
disk-usage-tui /path/to/directory

# Follow symbolic links
disk-usage-tui --follow-symlinks
```

### Keyboard Controls

| Key             | Action                 |
| --------------- | ---------------------- |
| `↑`/`k`/`↓`/`j` | Navigate items         |
| `→`/`Enter`     | Enter directory        |
| `←`/`Backspace` | Go to parent directory |
| `s`             | Toggle sort order      |
| `q`             | Quit                   |

## Building from Source

1. Clone the repository:

   ```bash
   git clone https://github.com/raghav-rama/disk-usage-analyzer-tui.git
   cd disk-usage-analyzer-tui
   ```

2. Build in release mode:

   ```bash
   cargo build --release
   ```

3. The binary will be available at `target/release/disk-usage-tui`

## License

This project is licensed under the [MIT License](LICENSE).

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.
