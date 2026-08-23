# RetroArch Core

This guide covers using WQXEmu as a libretro core with RetroArch, including installation, supported platforms, RetroPad mapping, and features.

## Installation

### Online Updater (Recommended)

1. Open RetroArch
2. Go to **Main Menu → Online Updater → Core Downloader**
3. Select **WQXEmu** from the list

### Manual Installation

1. Download the core from the [Releases](https://github.com/AloysHF/WQXEmu/releases) page
2. Copy the core file to RetroArch's `cores/` directory:
   - Windows: `wqxemu_libretro.dll`
   - Linux: `libwqxemu_libretro.so`
   - macOS: `libwqxemu_libretro.dylib`
3. Copy the info file to RetroArch's `info/` directory:
   - `wqxemu_libretro.info`

### Build from Source

```bash
cargo build -p wqxemu-libretro --release
```

The compiled core will be at:
- Windows: `target/release/wqxemu_libretro.dll`
- Linux: `target/release/libwqxemu_libretro.so`
- macOS: `target/release/libwqxemu_libretro.dylib`

## Supported Platforms

- Windows (x86_64)
- Linux (x86_64)
- macOS (x86_64, aarch64)
- Android (arm64-v8a, armeabi-v7a)
- iOS (arm64)
- webOS

## Loading Content

### Automatic Model Detection

The core automatically detects the model based on the loaded firmware files:

1. Open RetroArch
2. Select **Load Core** → **WQXEmu**
3. Select **Load Content** and choose a firmware file
4. The core will automatically detect the model and load required firmware files

### Firmware File Requirements

| Model | Required firmware | File types |
|-------|-------------------|------------|
| NC1020 | ROM + NOR | `.bin`, `.fls` |
| PC1000 | ROM + NOR | `.rom`, `.fls` |
| CC800 | ROM + NOR | `.bin`, `.fls` |
| NC2000 | NOR + NAND + NAND0 | `.nor`, `.nand`, `.nand0` |
| NC3000 | NOR + NAND | `.nor`, `.nand` |

### Firmware File Placement

Keep each model's firmware files together in its own directory. The directory can be anywhere RetroArch can access; placing it under RetroArch's `system/` directory is recommended:

```
system/
└── WQXEmu/
    ├── nc1020/
    │   ├── obj_lu.bin
    │   └── nc1020.fls
    ├── pc1000/
    │   ├── pc1000.rom
    │   └── pc1000.fls
    ├── cc800/
    │   ├── obj.bin
    │   └── cc800.fls
    ├── nc2000/
    │   ├── nc2000.nor
    │   ├── nc2000.nand
    │   └── nc2000.nand0
    └── nc3000/
        ├── nc3000.nor
        └── nc3000.nand
```

Load any one file from the model's directory. The core first looks for companion files with the same stem, then accepts a uniquely matching firmware extension in that directory. This supports both pairs such as `pc1000.rom` + `pc1000.fls` and differently named pairs such as `obj_lu.bin` + `nc1020.fls`.

## RetroPad Button Mapping

| RetroPad Button | WQX Key | Action |
|-----------------|---------|--------|
| D-Pad Up | Up | Navigate up |
| D-Pad Down | Down | Navigate down |
| D-Pad Left | Left | Navigate left |
| D-Pad Right | Right | Navigate right |
| A | Enter | Confirm |
| B | Escape | Back / Cancel |
| X | F1 | Function key 1 |
| Y | F4 | Function key 4 |
| L1 | Page Up | Previous page |
| R1 | Page Down | Next page |
| L2 | — | — |
| R2 | — | — |
| Select | F11 | Model-specific hotkey |
| Start | F10 | Model-specific hotkey |

## Core Options

The current core does not expose core-specific options. Display scaling, audio volume, input bindings, and logging use RetroArch's frontend settings.

## Save States

Save states are supported through RetroArch's save state system.

### Save State

- Press **F2** or use **Quick Menu → Save State**

### Load State

- Press **F4** or use **Quick Menu → Load State**

### Important Notes

- The core rejects save states created for a different model
- Use the same firmware set when loading a save state
- Save states are separate from persistent sessions
- Source firmware dumps remain read-only; flash changes are not persisted after unloading content

## Screenshots

Screenshots can be taken through RetroArch:

- Press **F8** or use **Quick Menu → Take Screenshot**

## Troubleshooting

### Common Issues

1. **"No firmware found"** — Ensure firmware files are in the correct location
2. **"Model detection failed"** — Keep only one model's firmware set in the selected file's directory and verify all required files are present
3. **"Black screen"** — Check firmware file integrity

### Debug Logging

Enable debug logging in RetroArch:

1. Go to **Settings → Logging**
2. Set **Logging Verbosity** to **Debug**
3. Restart RetroArch

### Core Information

View core information:

1. Go to **Quick Menu → Information**
2. Check **Core Name**, **Core Version**, and **System Name**

## Android

For Android-specific instructions, see [Android Libretro Core](Android-Libretro-Core.md).

## iOS

For iOS-specific instructions, see [iOS Libretro Core](iOS-Libretro-Core.md).
