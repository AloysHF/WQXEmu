# Android Libretro Core

This guide covers building and using the WQXEmu libretro core on Android.

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
- `target/aarch64-linux-android/release/libwqxemu_libretro.so`
- `target/armv7-linux-androideabi/release/libwqxemu_libretro.so`
- `target/x86_64-linux-android/release/libwqxemu_libretro.so`

## Installation

### Using RetroArch on Android

1. **Install RetroArch** from Google Play Store or F-Droid
2. **Copy the core** to RetroArch's cores directory:
   - Internal storage: `RetroArch/cores/`
   - Or use RetroArch's built-in core updater
3. **Copy firmware files** to RetroArch's system directory:
   - Internal storage: `RetroArch/system/WQXEmu/`
4. **Launch RetroArch**, load the WQXEmu core, select the machine in **Core Options**, and use **Start Core**

### Manual Installation

1. Connect your Android device to your computer
2. Copy the core file to your device:
   ```bash
   adb push target/aarch64-linux-android/release/libwqxemu_libretro.so /sdcard/RetroArch/cores/
   ```
3. Copy firmware files:
   ```bash
   adb push roms/nc1020/ /sdcard/RetroArch/system/WQXEmu/nc1020/
   ```

## Supported Android Architectures

| Architecture | Rust Target | Status |
|--------------|-------------|--------|
| arm64-v8a | aarch64-linux-android | ✅ Supported |
| armeabi-v7a | armv7-linux-androideabi | ✅ Supported |
| x86_64 | x86_64-linux-android | ✅ Supported |
| x86 | i686-linux-android | ⚠️ Experimental |

## Configuration

### Firmware Placement

Place firmware files in RetroArch's system directory:

```
/sdcard/RetroArch/system/WQXEmu/
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

These paths and filenames are fixed. NC1020, PC1000, CC800, and NC2000 require
every file shown for that model. NC3000 requires its NOR and NAND files; NAND0 is
optional. Firmware is system data and cannot be selected with **Load Content**.

### Core Options

Select **Machine Model** to choose NC1020, PC1000, CC800, NC2000, or NC3000. NC1020
is the default. Restart the core after changing the model. Use RetroArch's frontend
settings for display, audio, and input configuration.

### Starting the Core

1. Select **Load Core → WQXEmu**
2. Select **Start Core**; the initial default is NC1020
3. To use another model, select **Quick Menu → Core Options → Machine Model**
4. Select **Quick Menu → Close Content**, then **Start Core** again

### Persistent Device State

Source firmware remains read-only. On a normal unload or shutdown, the core saves
a compressed session under RetroArch's configured save directory as
`<model>/<firmware fingerprint>.wqxs`. The same model and firmware set resume that
session automatically. An abnormal app termination cannot flush current changes.

## Performance Tips

1. **Use arm64-v8a** — Best performance for modern devices
2. **Close background apps** — Free up memory and CPU
3. **Use a gamepad** — Better control than touchscreen
4. **Adjust frontend latency** — Use RetroArch's audio and video latency settings if needed

## Troubleshooting

### Common Issues

1. **"Core failed to load"** — Ensure the core file is in the correct location
2. **"Missing firmware"** — Check the exact system-directory paths for the selected model and storage permissions
3. **"Black screen"** — Try a different firmware version
4. **"Audio crackling"** — Adjust RetroArch's frontend audio latency settings

### Debug Logging

Enable debug logging in RetroArch:

1. Go to **Settings → Logging**
2. Set **Logging Verbosity** to **Debug**
3. Check logs at `/sdcard/RetroArch/logs/`

### Performance Issues

If you experience performance issues:

1. Check CPU usage in RetroArch's **Quick Menu → Information**
2. Try increasing RetroArch's audio latency
3. Disable unnecessary frontend video filters and shaders
4. Close other apps running in the background

## Building with Android Studio

If you prefer using Android Studio:

1. Open the project in Android Studio
2. Build the core using Gradle
3. Copy the built core to RetroArch

## Resources

- [RetroArch Android Guide](https://docs.libretro.com/guides/install-android/)
- [Rust Android Guide](https://mozilla.github.io/book/ch20-05-rust-on-android.html)
- [Android NDK Documentation](https://developer.android.com/ndk)
