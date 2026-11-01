# EmulatorJS Core (Browser)

WQXEmu can run in the browser as a standard [EmulatorJS](https://emulatorjs.org)
core. The build follows EmulatorJS's own pipeline: the Rust libretro core is
compiled for `wasm32-unknown-emscripten`, linked against the
`EmulatorJS/RetroArch` fork, and packaged as the `<core>-wasm.data` bundles that
an EmulatorJS page loads at runtime.

The project site ships a ready-to-use player at
<https://AloysHF.github.io/WQXEmu/play.html>: visitors pick a firmware ZIP from
their own machine and it runs from a blob URL, so nothing is uploaded and the
site does not host any firmware.

## What you get

Every nightly publishes a single asset:

| File | Contents |
|------|----------|
| `wqxemu-emulatorjs.zip` | Both core variants, the build report `cores/reports/wqxemu.json`, `core.json`, `build.json`, `license.txt` and a README |

The two `.data` files stay separate inside the ZIP: EmulatorJS requests
`<core>-wasm.data` or `<core>-legacy-wasm.data` by name depending on WebGL2
support, so one bundle cannot serve both cases.

The bundle is laid out like this:

```
wqxemu-emulatorjs.zip
├── cores/
│   ├── wqxemu-wasm.data
│   ├── wqxemu-legacy-wasm.data
│   └── reports/wqxemu.json
├── core.json
├── build.json
├── license.txt
└── README.md
```

Each `.data` file is a 7z archive containing `<core>_libretro.js`,
`<core>_libretro.wasm`, `core.json`, `build.json` and `license.txt`.

## Install into a site

EmulatorJS always fetches cores from `<EJS_pathtodata>cores/`, so host the
runtime yourself (or use the pinned CDN files) and drop the core beside it:

```
data/
├── loader.js
├── emulator.min.js
├── emulator.min.css
├── compression/
│   ├── extract7z.js       # extracts the core .data archive
│   └── extractzip.js      # extracts ZIP content
└── cores/
    ├── wqxemu-wasm.data
    ├── wqxemu-legacy-wasm.data
    └── reports/wqxemu.json
```

`emulator.min.zip` only contains `emulator.min.js` and `emulator.min.css`;
the `compression/` files are loaded on demand from the same EmulatorJS data
directory. Without `extract7z.js` the core download ends in a "Network Error".

To upgrade the runtime, replace the runtime files with a newer EmulatorJS
release (≥ 4.2.3) and keep this `cores/` directory.

## Use

```html
<div id="game"></div>
<script>
  EJS_player = "#game";
  EJS_core = "wqxemu";
  EJS_gameUrl = URL.createObjectURL(firmwareZip);
  EJS_gameName = firmwareZip.name;
  EJS_pathtodata = "data/";
  EJS_startOnLoaded = false;
</script>
<script src="data/loader.js"></script>
```

`site/play.html` in this repository is a complete example.

Like the RetroArch core, the browser build boots firmware from a system
directory rather than from game content. EmulatorJS extracts the content ZIP
into the root of the wasm filesystem, and the core reads
`/wqxemu/<model>/` from there. The page only exposes a firmware ZIP picker and
never ships dumps of its own.

## Firmware archive layout

The ZIP must contain exactly one model folder below `wqxemu/`:

```
nc1020.zip
  wqxemu/
    nc1020/
      nc1020.rom
      nc1020.fls
```

The folder name selects the model (`nc1020`, `pc1000`, `cc800`, `nc2000`,
`nc3000`). EmulatorJS exposes no core-option UI, so the archive itself names
the model. An archive with zero or several model folders is rejected rather
than guessed, and the single-file content that a game-oriented frontend would
provide cannot boot these machines.

File names are case-sensitive and must match the table below exactly.

| Model  | Required files |
|--------|----------------|
| NC1020 | `nc1020.rom`, `nc1020.fls` |
| PC1000 | `pc1000.rom`, `pc1000.nor` |
| CC800  | `cc800.rom`, `cc800.fls` |
| NC2000 | `nc2000.nor`, `nc2000.nand`, `nc2000.nand0` |
| NC3000 | `nc3000.nor`, `nc3000.nand` (`nc3000.nand0` optional) |

Firmware dumps are copyrighted; this bundle ships none. Guest writes to
NOR/NAND stay in the wasm filesystem and are lost when the page is reloaded.

## Rendering

The core renders its framebuffer through libretro's software path as XRGB8888
and reports a fixed 160×80 LCD at startup. EmulatorJS scales the output to fit
the play stage; the play page uses nearest-neighbor scaling so the 4-level
grayscale LCD stays crisp.

## Controls

The core registers both a keyboard device and a RetroPad device:

| Input | Guest key |
|-------|-----------|
| Typed letters, digits, punctuation | Matching QWERTY key |
| Arrow keys | Direction keys |
| Enter / Backspace / Escape / Space | Same-named keys |
| Page Up / Page Down | Page keys |
| F1–F12 | Function keys |

EmulatorJS maps its RetroPad to the same key set:

| RetroPad | Guest key |
|----------|-----------|
| D-pad | Direction keys |
| A / B | Enter / Escape |
| X / Y | F1 / F4 |
| L / R | Page Up / Page Down |
| Start / Select | F10 / F11 |

Touch controls and physical gamepads are provided by EmulatorJS and arrive
through the libretro devices above.

## Saves

Firmware writes have no battery-backed save file in this core, so `core.json`
sets `"save": false` to hide EmulatorJS's save-file buttons. Libretro save
states are implemented by the core and handled by EmulatorJS; they stay in the
browser's local storage. The desktop persistent-session file is disabled on
Emscripten because it relies on the host filesystem.

## Building the bundle

The nightly workflow runs this automatically. To build locally you need an
activated [emsdk](https://emscripten.org/docs/getting_started/downloads.html)
(`emmake` in `PATH`), `p7zip`, `zip`, and the pinned nightly toolchain:

```bash
# Rust's prebuilt emscripten stdlib carries wasm exception handling, which
# Binaryen's Asyncify pass rejects. Rebuild std with panic=abort instead.
cargo +nightly build -p wqxemu-libretro \
  --target wasm32-unknown-emscripten --profile wasm-release \
  -Zbuild-std=std,panic_abort

scripts/build-emulatorjs-core.sh
```

The script clones the pinned `EmulatorJS/RetroArch` revision, links the
staticlib with `Makefile.emulatorjs`, builds the WebGL2 and legacy variants, and
writes `out/wqxemu-emulatorjs.zip`. `--work-dir` moves the RetroArch checkout
out of `target/`, which matters on CI because `rust-cache` restores `target/`
without the checkout's `.git`.

The Pages deployment fetches the pinned EmulatorJS runtime, downloads the
`wqxemu-emulatorjs.zip` asset from the rolling `nightly-build` release, and
redeploys after every successful Nightly Build.
