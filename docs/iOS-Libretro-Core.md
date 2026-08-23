# iOS Libretro Core

This guide covers iOS-specific build, signing, installation, and sandbox details.
For firmware requirements, model selection, startup, controls, save states, and
persistent device state, see the [RetroArch Core guide](RetroArch-Core.md).

## Building for iOS

### Prerequisites

1. **Xcode** — Install from Mac App Store
2. **Rust** — Install via [rustup](https://rustup.rs/)
3. **iOS targets** — Add Rust targets for iOS

```bash
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
```

### Build the Core

```bash
# For real devices (arm64)
cargo build -p wqxemu-libretro --target aarch64-apple-ios --release

# For simulator (x86_64)
cargo build -p wqxemu-libretro --target x86_64-apple-ios --release

# For simulator (arm64 - Apple Silicon)
cargo build -p wqxemu-libretro --target aarch64-apple-ios-sim --release
```

### Output Files

The compiled core will be at:
- `target/aarch64-apple-ios/release/libwqxemu_libretro.dylib`
- `target/x86_64-apple-ios/release/libwqxemu_libretro.dylib`
- `target/aarch64-apple-ios-sim/release/libwqxemu_libretro.dylib`

## Integrating the Core into RetroArch

iOS does not allow RetroArch to install or update executable cores at runtime.
The WQXEmu `.dylib` must be included in the RetroArch application before the app
is signed and installed:

1. Build `libwqxemu_libretro.dylib` for the target device architecture
2. Copy it into RetroArch's source tree at `pkg/apple/iOS/modules/`
3. Build or archive RetroArch with Xcode so the application and core are signed
   together
4. Install the resulting application through Xcode or the chosen signed-app
   distribution workflow

Copying a `.dylib` into an already installed App Store, TestFlight, or sideloaded
RetroArch build does not add a usable core. The application must be rebuilt and
resigned with WQXEmu included.

## Supported iOS Architectures

| Architecture | Rust Target | Status |
|--------------|-------------|--------|
| arm64 (real devices) | aarch64-apple-ios | ✅ Supported |
| x86_64 (Intel simulator) | x86_64-apple-ios | ✅ Supported |
| arm64 (Apple Silicon simulator) | aarch64-apple-ios-sim | ✅ Supported |

## iOS Application Sandbox

Run RetroArch once so iOS creates its application folders, then use the Files app
under **On My iPhone/iPad → RetroArch**, or Finder file sharing, to transfer the
firmware. Check **Settings → Directory → System/BIOS** inside RetroArch rather than
assuming a fixed filesystem path.

Create the `WQXEmu/<model>/` hierarchy from the
[common firmware guide](RetroArch-Core.md#firmware-file-placement) inside that
configured system directory. Firmware and save data remain inside RetroArch's app
sandbox unless exported through Files or Finder.

## iOS-Specific Troubleshooting

1. **WQXEmu does not appear in the core list** — Confirm the `.dylib` was placed in
   `pkg/apple/iOS/modules/` before building RetroArch
2. **The core fails to load** — Verify the core is built for the device architecture
   and signed as part of the application bundle
3. **Firmware copied with Files is not found** — Compare its location with
   **Settings → Directory → System/BIOS** inside RetroArch
4. **A sideloaded build stops launching** — Check its provisioning profile and
   resign or reinstall the application when required

For emulator-level errors and logging settings, use the
[common troubleshooting guide](RetroArch-Core.md#troubleshooting).

## Resources

- [RetroArch iOS Guide](https://docs.libretro.com/guides/install-ios/)
- [Rust iOS Guide](https://mozilla.github.io/book/ch20-05-rust-on-ios.html)
- [Xcode Documentation](https://developer.apple.com/xcode/)
