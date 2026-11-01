# Standalone Emulator

This guide covers the standalone WQXEmu desktop application: firmware layout,
command-line options, keyboard controls, screenshots and persistent sessions.

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

## Firmware Layout

Every frontend reads the same fixed, case-sensitive layout below the system
directory. `--model` (default `nc1020`) selects the folder that is read:

```
<system-dir>/
└── wqxemu/
    ├── nc1020/
    │   ├── obj_lu.bin
    │   └── nc1020.fls
    ├── pc1000/
    │   ├── pc1000.rom
    │   └── pc1000.nor
    ├── cc800/
    │   ├── obj.bin
    │   └── cc800.fls
    ├── nc2000/
    │   ├── nc2000.nor
    │   ├── nc2000.nand
    │   └── nc2000.nand0
    └── nc3000/
        ├── nc3000.nor
        ├── nc3000.nand
        └── nc3000.nand0    # optional
```

| Model | Required files | Optional files |
|-------|----------------|----------------|
| NC1020 | `obj_lu.bin`, `nc1020.fls` | — |
| PC1000 | `pc1000.rom`, `pc1000.nor` | — |
| CC800 | `obj.bin`, `cc800.fls` | — |
| NC2000 | `nc2000.nor`, `nc2000.nand`, `nc2000.nand0` | — |
| NC3000 | `nc3000.nor`, `nc3000.nand` | `nc3000.nand0` |

The folder names and file names are compared byte-for-byte. `WQXEmu/`,
`NC1020/` or `OBJ_LU.BIN` are not recognized; only `wqxemu/`, `nc1020/`
and `obj_lu.bin` are. Other files in the folder are ignored.

## Quick Start

```bash
wqx-emu --model nc1020 --system-dir system
```

The code-drawn skin is used by default.

## Command-Line Options

### Usage

```bash
wqx-emu [OPTIONS]
```

### Model and Firmware

- `--model <MODEL>` — Model to emulate (default: `nc1020`)
  - One of `nc1020`, `pc1000`, `cc800`, `nc2000`, `nc3000`
  - Selects `<system-dir>/wqxemu/<MODEL>/`
- `--system-dir <DIR>` — Root that contains the `wqxemu/` folder (default: `system`)

### Display Options

- `-s, --scale <N>` — Window scale factor (default: 4)
- `--skin <MODE>` — Device skin renderer (default: `code`)
  - `image` uses the original embedded model image
  - `code` draws the shell, controls, labels, speaker, and keyboard without reading a skin image
- `--dump-skin <PATH>` — Export the selected skin bitmap to a PNG file and exit
  - Honors `--model`, `--scale`, and `--skin`; useful for previewing or comparing skins
- `-S, --screenshot <PATH>` — Run some frames, save a PNG screenshot and exit
- `--screenshot-frames <N>` — Frames to run before the screenshot (default: 30)

For example, start the NC1020 with the original photograph-like image skin:

```bash
wqx-emu --model nc1020 --system-dir system --skin image
```

Both modes use the same LCD rectangle and clickable key geometry. The image
mode provides the original photographic appearance; the code mode recreates
the device layout with model-specific colors and raster-drawn controls.

### State Management

- `--state-file <PATH>` — Path to save/load a compressed session state
  - Creates a gzip-compressed session state file
  - Restores the state on subsequent runs

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

## Screenshots

```bash
wqx-emu --model nc1020 --screenshot nc1020.png --screenshot-frames 300
```

The emulator runs without opening a window, saves the LCD frame to the PNG
path and exits. `--screenshot-frames` controls how long the firmware runs
before the capture.

## Persistent Sessions

The `--state-file` option enables persistent sessions that save and restore emulator state.

### First Run

```bash
wqx-emu --model nc2000 --system-dir system --state-file nc2000.wqxs
```

On exit, the emulator saves a compressed session state to `nc2000.wqxs`.

### Subsequent Runs

```bash
wqx-emu --model nc2000 --system-dir system --state-file nc2000.wqxs
```

The emulator restores the saved state.

### Important Notes

- Use a separate state file for each machine and firmware configuration
- States from another model are rejected
- ROM, NOR, NAND, and NAND0 source dumps remain read-only and are never overwritten
- The state file must not point at one of the source dumps

## Examples

```bash
# NC1020 with ROM and NOR
wqx-emu --model nc1020 --system-dir system

# PC1000 with ROM and NOR
wqx-emu --model pc1000 --system-dir system

# CC800 with ROM and NOR
wqx-emu --model cc800 --system-dir system

# NC2000 with NOR, NAND, and NAND0
wqx-emu --model nc2000 --system-dir system

# NC3000 with NOR and NAND (NAND0 optional)
wqx-emu --model nc3000 --system-dir system

# 2x scale, original image skin
wqx-emu --model nc1020 --system-dir system --scale 2 --skin image

# Persistent session
wqx-emu --model nc2000 --system-dir system --state-file nc2000.wqxs
```

## Environment Variables

- `RUST_LOG` — Set logging level
  - `wqxemu=debug` — Enable debug logging
  - `wqxemu=trace` — Enable trace logging
  - `wqxemu=info` — Enable info logging
  - `wqxemu=warn` — Enable warning logging
  - `wqxemu=error` — Enable error logging

## Troubleshooting

1. **"Firmware directory not found: .../wqxemu/<model>"** — Check that
   `--system-dir` points at the folder that contains `wqxemu/`, and that the
   model folder name is lowercase and matches `--model`.
2. **"<model> firmware is incomplete in ..."** — The message lists every
   missing file. File names are case-sensitive; place the dumps with the
   exact names from the table above.
3. **"unknown model '...'"** — Use one of `nc1020`, `pc1000`, `cc800`,
   `nc2000`, `nc3000`.
4. **"State file mismatch"** — Use a state file created with the same model
   and firmware.

Enable debug logging to see more detail:

```bash
RUST_LOG=wqxemu=debug wqx-emu --model nc1020 --system-dir system
```

## Notes

1. **Firmware files are required** — The emulator cannot run without the
   complete set for the selected model.
2. **State files are optional** — You can run without a state file or never
   create one.
3. **Scale factor affects performance** — Higher scale factors may be slower.
