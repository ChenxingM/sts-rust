# STS 3.0

A timesheet editor rebuilt in Rust

## Features

- **Fast & Lightweight**: Optimized executable ~3MB, minimal memory footprint
- **Native STS Format**: Full support for reading/writing `.sts` files with Shift-JIS encoding
- **Theme Settings**: Light/Dark theme switching
- **AE Keyframe Export**: Copy keyframe data to clipboard for pasting into After Effects
- **Flexible Input**: Configurable Enter key jump step with auto-fill for skipped cells

## Usage

### Create New Timesheet

1. Click **File → New** or press **Ctrl+N**
2. Configure parameters:
   - **Name**: Timesheet name
   - **Layers**: Number of layers (1-1000)
   - **Frame Rate**: FPS (24 or 30)
   - **Frames per Page**: Frames displayed per page (12-288)
   - **Duration**: Total duration in seconds+frames (e.g., 6s + 0k)
3. Click **OK** to create

### Edit Cells

- **Click** a cell to start editing
- **Enter** a number and press **Enter** to move down
- **Arrow keys** to navigate while editing
- **Empty input** + **Enter** copies the value from the cell above
- **Esc** to cancel editing

### Selection & Clipboard

- **Drag** to select multiple cells
- **Right-click** to open context menu
- **Copy/Cut/Paste** selected regions
- System clipboard support (Excel-compatible TSV format)

## File Format

ShiraheiTimeSheet binary format

Supports:
- Up to **255 layers**
- Up to **65535 frames**

## Build

```bash
# Debug
cargo build

# Release
cargo build --release
```

Release executable will be at `target/release/sts.exe`.

## Dependencies

- **egui** 0.29 - GUI framework
- **eframe** 0.29 - Native window wrapper
- **encoding_rs** - Shift-JIS encoding/decoding
- **rfd** - Native file dialogs
- **anyhow** - Error handling

## System Requirements

- **Windows**: Windows 7 or later
- **macOS**: macOS 10.13 or later
- **Linux**: Modern Linux distributions (X11 or Wayland required)

## License
Apache-2.0

### Encoding Handling

Uses `encoding_rs` library for Shift-JIS encoding:
- Automatically decodes to UTF-8 when reading
- Converts back to Shift-JIS when saving
- Compatible with Japanese layer names

## Development Roadmap

### Completed ✅
- [x] Basic table editing
- [x] STS file read/write
- [x] Undo/Redo
- [x] Clipboard operations
- [x] Keyboard navigation


## Contributing

Issues and pull requests are welcome!

### Development Setup

```bash
# Clone repository
git clone https://github.com/ChenxingM/sts-rust.git
cd sts-rust

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Run tests
cargo test

# Start development build
cargo run
```

### Code Style

- Format code with `rustfmt`
- Check code quality with `clippy`
- Write tests for key functionality

```bash
cargo fmt
cargo clippy
cargo test
```


## Acknowledgments

- Original ShiraheiTimeSheet author
- egui GUI framework

## Contact

For questions or suggestions, please submit an Issue or Pull Request.

---
