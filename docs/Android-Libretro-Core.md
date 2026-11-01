# Android Libretro Core

This guide covers Android-specific build, installation, storage, and troubleshooting
details. For firmware requirements, model selection, startup, controls, save states,
and persistent device state, see the [RetroArch Core guide](RetroArch-Core.md).

## Building for Android

### Prerequisites

1. **Android NDK** — Install via Android Studio or download directly
2. **Rust** — Install via [rustup](https://rustup.rs/)
3. **Android targets** — Add Rust targets for Android

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

### Configure Cargo

Create or edit `~/.cargo/config.toml`:

```toml
[target.aarch64-linux-android]
linker = "/path/to/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang"

[target.armv7-linux-androideabi]
linker = "/path/to/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi30-clang"

[target.x86_64-linux-android]
linker = "/path/to/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android30-clang"
```

Replace `/path/to/android-ndk` with your actual NDK path.

### Build the Core

```bash
# For arm64-v8a (most modern devices)
cargo build -p wqxemu-libretro --target aarch64-linux-android --release

# For armeabi-v7a (older devices)
cargo build -p wqxemu-libretro --target armv7-linux-androideabi --release

# For x86_64 (emulators)
cargo build -p wqxemu-libretro --target x86_64-linux-android --release
```

### Output Files

The compiled core will be at:
- `target/aarch64-linux-android/release/libwqxemu.so`
- `target/armv7-linux-androideabi/release/libwqxemu.so`
- `target/x86_64-linux-android/release/libwqxemu.so`

## Installation

### Using RetroArch on Android

1. **Install RetroArch** from Google Play Store or F-Droid
2. **Copy the core** to RetroArch's cores directory:
   - Internal storage: `RetroArch/cores/`
   - Or use RetroArch's built-in core updater
3. In **Settings → Directory**, note RetroArch's **System/BIOS** directory and make
   sure Android has granted RetroArch access to it
4. Install the firmware under that directory and start the machine as described in
   the [common guide](RetroArch-Core.md#starting-a-machine)

### Manual Installation

1. Connect your Android device to your computer
2. Copy the core file to your device:
   ```bash
   adb push target/aarch64-linux-android/release/libwqxemu.so /sdcard/RetroArch/cores/
   ```
3. Copy firmware files:
   ```bash
   adb push tmp/roms/nc1020/ /sdcard/RetroArch/system/WQXEmu/nc1020/
   ```

## Supported Android Architectures

| Architecture | Rust Target | Status |
|--------------|-------------|--------|
| arm64-v8a | aarch64-linux-android | ✅ Supported |
| armeabi-v7a | armv7-linux-androideabi | ✅ Supported |
| x86_64 | x86_64-linux-android | ✅ Supported |
| x86 | i686-linux-android | ⚠️ Experimental |

## Android Storage

The actual directory depends on the RetroArch distribution and Android storage
permissions. `/sdcard/RetroArch/` is a common manual-install location, but the
paths shown under **Settings → Directory** are authoritative.

- Place the model directories from the [firmware table](RetroArch-Core.md#firmware-file-requirements)
  below the configured **System/BIOS** directory as `WQXEmu/<model>/`
- Grant RetroArch access when Android shows the storage access framework picker
- If external storage is not visible to RetroArch, use its app-specific directory
  or another directory selected through RetroArch itself
- Android logs are commonly stored under `/sdcard/RetroArch/logs/`; confirm the
  configured log directory for the installed distribution

## Android-Specific Troubleshooting

1. **The core does not appear or load** — Install the `.so` matching the device ABI;
   prefer `arm64-v8a` on modern 64-bit devices
2. **Firmware is present but reported missing** — Recheck **Settings → Directory →
   System/BIOS** and Android's storage permission for that directory
3. **ADB copied files are not visible** — Select a directory exposed to RetroArch
   through Android's storage access framework, then copy the files there

For emulator-level errors and logging settings, use the
[common troubleshooting guide](RetroArch-Core.md#troubleshooting).

## Resources

- [RetroArch Android Guide](https://docs.libretro.com/guides/install-android/)
- [Rust Android Guide](https://mozilla.github.io/book/ch20-05-rust-on-android.html)
- [Android NDK Documentation](https://developer.android.com/ndk)

## RetroDroid

The same core ships inside [RetroDroid](https://github.com/AloysHF/RetroDroid),
the all-in-one Android app for this emulator family. RetroDroid loads
`libwqxemu.so` directly (no RetroArch required) and reads the neighbouring
`android.json` for the system profile: display names, firmware list, input
preset, orientation and pixel format.

Rules the core follows for any Android host:

- Saves and persistent session files go to the directory answered by
  `RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY`; the content path may be a throwaway
  copy in the app cache, so nothing is written next to it.
- Firmware is read from the content path the frontend hands over; if that
  cached set is unusable, the core falls back to
  `RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY` (`WQXEmu/<model>/`). A real file
  path is required and an in-memory buffer is ignored.
- Load failures are reported with `RETRO_ENVIRONMENT_SET_MESSAGE` and through
  the frontend log interface, never as a silent `false`.
- Panics are caught at the C ABI boundary (`catch_unwind` in every exported
  `retro_*` entry point), so a bad frame cannot kill the host app.
- Save states carry a `WQXS` magic header plus a format version; states from
  another version or model are rejected instead of being misread.
- Build with `RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"` so every
  `LOAD` segment is 16 KB aligned for Android 15 and newer devices.
