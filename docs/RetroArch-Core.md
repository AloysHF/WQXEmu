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

## Starting a Machine

WQXEmu is a low-level emulator that boots original hardware firmware. The firmware
is system data, not game content, so the core starts without content and does not
accept ROM, NOR, or NAND files through **Load Content**.

1. Install the required files in RetroArch's system directory using the exact paths below
2. Select **Load Core → WQXEmu**
3. Select **Start Core**; the initial default is NC1020
4. To use another model, open **Quick Menu → Core Options → Machine Model** while the core is running
5. Select **Quick Menu → Close Content**, then **Start Core** again

The selected model is applied when the core starts, so changing it always requires
closing and starting the core again.

### Firmware File Requirements

| Model | Required files | Optional files |
|-------|----------------|----------------|
| NC1020 | `obj_lu.bin`, `nc1020.fls` | — |
| PC1000 | `pc1000.rom`, `pc1000.fls` | — |
| CC800 | `obj.bin`, `cc800.fls` | — |
| NC2000 | `nc2000.nor`, `nc2000.nand`, `nc2000.nand0` | — |
| NC3000 | `nc3000.nor`, `nc3000.nand` | `nc3000.nand0` |

### Firmware File Placement

The paths and filenames are fixed relative to the system directory configured in
RetroArch. Only the files for the selected model are required:

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
        ├── nc3000.nand
        └── nc3000.nand0    # Optional
```

RetroArch's core information page lists the firmware for all five models. Those
entries are conditional: a complete set is required only for the model selected in
Core Options. The core validates the selected set at startup and reports every
missing path.

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

**Machine Model** selects NC1020, PC1000, CC800, NC2000, or NC3000. NC1020 is the
default, and a running machine must be restarted after this option changes.

Display scaling, audio volume, input bindings, and logging continue to use
RetroArch's frontend settings.

## Persistent Device State

The core keeps the source firmware files in the system directory read-only. On a
normal content unload or core shutdown, it writes the complete writable device
state to:

```
<RetroArch save directory>/<model>/<firmware fingerprint>.wqxs
```

The compressed file includes writable flash/NAND, RAM, CPU, RTC, and peripheral
state. The model and a fingerprint of the complete source firmware set isolate
incompatible sessions. It is restored automatically the next time the same model
and firmware set starts. An abnormal frontend termination cannot flush changes
from the current session.

## Save States

Save states are supported through RetroArch's save state system.

### Save State

- Press **F2** or use **Quick Menu → Save State**

### Load State

- Press **F4** or use **Quick Menu → Load State**

### Important Notes

- The core rejects save states created for a different model
- Use the same firmware set when loading a save state
- Manual save states are separate from the automatically managed persistent device state
- Source firmware dumps remain read-only

## Screenshots

Screenshots can be taken through RetroArch:

- Press **F8** or use **Quick Menu → Take Screenshot**

## Troubleshooting

### Common Issues

1. **"Missing firmware"** — Verify the exact system-directory paths for the selected model
2. **The wrong machine starts** — Change **Core Options → Machine Model**, then restart the core
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
