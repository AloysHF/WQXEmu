# Standalone Emulator

This guide covers the standalone WQXEmu desktop application, including installation, keyboard controls, headless mode, screenshots, and all command-line options.

## Installation

### Download Pre-built Binaries

Download the latest release from the [Releases](https://github.com/AloysHF/WQXEmu/releases) page. Binaries are available for:

- Windows (x86_64)
- macOS (x86_64, aarch64)
- Linux (x86_64)

### Build from Source

Requires [Rust](https://www.rust-lang.org/tools/install) (stable).

```bash
cargo build --release
```

The binary will be at `target/release/wqx-emu` (or `wqx-emu.exe` on Windows).

## Quick Start

### Basic Usage

```bash
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020
```

### Supported Models

| Model | Required firmware | Optional firmware |
|-------|-------------------|-------------------|
| NC1020 | ROM + NOR | — |
| PC1000 | ROM + NOR | — |
| CC800 | ROM + NOR | — |
| NC2000 | NOR + NAND + NAND0 | — |
| NC3000 | NOR + NAND | NAND0 |

### Firmware Files

Point `--rom-dir` at a directory containing the firmware dumps for one device.
Files are discovered by extension (case-insensitive); any other files are
ignored:

- `.rom` / `.bin` — System ROM (NC1020, PC1000, CC800)
- `.fls` / `.nor` — NOR Flash dump
- `.nand` — NAND Flash dump (NC2000, NC3000)
- `.nand0` — First NAND plane dump (NC2000, NC3000)

The required combination depends on the selected model (see the table above).

## Command-Line Options

### Usage

```bash
wqx-emu [OPTIONS] [COMMAND]
```

### Model Selection

- `--model <MODEL>` — Select the model to emulate
  - `nc1020` (default)
  - `pc1000`
  - `cc800`
  - `nc2000`
  - `nc3000`

### Firmware Files

- `--rom-dir <DIR>` — Directory containing the firmware dumps for one device
  (required). Files are discovered by extension: `.rom`/`.bin` (system ROM),
  `.fls`/`.nor` (NOR Flash), `.nand` (NAND Flash), `.nand0` (first NAND plane)

### Display Options

- `--scale <N>` — Window scale factor (default: 4)
  - Range: 1-8
- `--skin <MODE>` — Device skin renderer (default: `image`)
  - `image` uses the original embedded model image
  - `code` draws the shell, controls, labels, speaker, and keyboard without reading a skin image
- `--fullscreen` — Start in fullscreen mode

For example, start the NC1020 with the code-drawn skin:

```bash
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020 --skin code
```

Both modes use the same LCD rectangle and clickable key geometry. The image
mode provides the original photographic appearance; the code mode recreates
the device layout with model-specific colors and raster-drawn controls.

### State Management

- `--state-file <PATH>` — Path to save/load compressed session state
  - Creates a gzip-compressed session state file
  - Restores state on subsequent runs

### Headless Mode

- `--headless` — Run without a window
  - Useful for testing and batch processing
- `--frames <N>` — Run for N frames and exit
  - Only works with `--headless`
- `--screenshot <PATH>` — Save screenshot to file
  - Only works with `--headless`

### Audio Options

- `--no-audio` — Disable audio output
  - Useful for headless mode or testing

### Debug Options

- `--debug` — Enable debug logging
  - Shows detailed information about emulator operation
- `--trace-cpu` — Enable CPU instruction tracing
  - Shows each CPU instruction executed
- `--trace-io` — Enable IO register tracing
  - Shows IO register reads and writes
- `--trace-bank` — Enable bank switching tracing
  - Shows bank switch operations

### Help

- `-h, --help` — Print help information
- `-V, --version` — Print version information

## Keyboard Controls

The standalone frontend uses the same physical keyboard mapping as the
RetroArch core. It also displays the complete device and accepts mouse clicks on
the pictured keys.

See [Physical Keyboard Controls](Keyboard-Controls.md) for the complete common
and model-specific mapping.

Escape operates the Wenquxing Escape/Back key. Close the window or use the
operating system's standard close shortcut to exit the emulator.

## Headless Mode

Headless mode runs the emulator without a window, useful for testing and batch processing.

### Basic Headless Usage

```bash
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020 --headless --frames 300
```

### Taking Screenshots

```bash
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020 --screenshot screenshot.png --frames 300
```

### Run Headless Without Audio

```bash
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020 --headless --no-audio
```

## Persistent Sessions

The `--state-file` option enables persistent sessions that save and restore emulator state.

### First Run

```bash
wqx-emu --model nc2000 --rom-dir tmp/roms/nc2000 --state-file nc2000.wqxs
```

On exit, the emulator saves a compressed session state to `nc2000.wqxs`.

### Subsequent Runs

```bash
wqx-emu --model nc2000 --state-file nc2000.wqxs
```

The emulator restores the saved state without re-running first-boot recovery.

### Important Notes

- Use a separate state file for each machine and firmware configuration
- States from another model are rejected
- ROM, NOR, NAND, and NAND0 source dumps remain read-only

## Examples

### Basic Usage

```bash
# NC1020 with ROM and NOR
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020

# PC1000 with ROM and NOR
wqx-emu --model pc1000 --rom-dir tmp/roms/pc1000

# CC800 with ROM and NOR
wqx-emu --model cc800 --rom-dir tmp/roms/cc800

# NC2000 with NOR, NAND, and NAND0
wqx-emu --model nc2000 --rom-dir tmp/roms/nc2000

# NC3000 with NOR and NAND
wqx-emu --model nc3000 --rom-dir tmp/roms/nc3000
```

### Display Options

```bash
# 2x scale
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020 --scale 2

# Fullscreen mode
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020 --fullscreen
```

### State Management

```bash
# First run with state file
wqx-emu --model nc2000 --rom-dir tmp/roms/nc2000 --state-file nc2000.wqxs

# Subsequent runs (firmware files are not needed)
wqx-emu --model nc2000 --state-file nc2000.wqxs
```

### Debug Options

```bash
# Enable debug logging
RUST_LOG=wqxemu=debug wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020

# Enable CPU tracing
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020 --trace-cpu

# Enable IO tracing
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020 --trace-io

# Enable bank switching tracing
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020 --trace-bank
```

## Environment Variables

- `RUST_LOG` — Set logging level
  - `wqxemu=debug` — Enable debug logging
  - `wqxemu=trace` — Enable trace logging
  - `wqxemu=info` — Enable info logging (default)
  - `wqxemu=warn` — Enable warning logging
  - `wqxemu=error` — Enable error logging

## Troubleshooting

### Common Issues

1. **"Firmware directory not found" / "No firmware dumps found"** — Ensure `--rom-dir` points at the directory holding the dumps
2. **"Invalid model"** — Check the `--model` parameter
3. **"State file mismatch"** — Use a state file created with the same model and firmware

### Debug Logging

Enable debug logging to see detailed information:

```bash
RUST_LOG=wqxemu=debug wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020
```

### CPU Tracing

Trace CPU instructions for debugging:

```bash
wqx-emu --model nc1020 --rom-dir tmp/roms/nc1020 --trace-cpu
```

## Notes

1. **Firmware files are required** — The emulator cannot run without firmware files
2. **State files are optional** — You can run without a state file
3. **Headless mode is for testing** — Use `--headless` for automated testing
4. **Debug options are verbose** — Use with caution in production
5. **Scale factor affects performance** — Higher scale factors may be slower
