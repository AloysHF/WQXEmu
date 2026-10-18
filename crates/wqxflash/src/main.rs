// wqxflash — offline GFFS flash-injection CLI for WQXEmu.
//
// Injects `.bin` / `.bas` game files into a Wenquxing firmware image by
// driving the firmware's own file-system syscalls through the same 0x3FFF
// pump protocol that wangyu's `put` command uses, but entirely offline and
// without ever writing to the source firmware directory (`tmp/roms`).
//
// The modified flash is written only to the user-supplied `--out` path.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use wqxemu_core::{Emulator, MachineModel, RomFiles};

#[derive(Parser)]
#[command(
    name = "wqxflash",
    version,
    about = "Offline GFFS flash injection for WQXEmu (adds .bin/.bas games to firmware images)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Inject one or more `.bin`/`.bas` files into the firmware flash file
    /// system. The source firmware directory is never modified; output goes to
    /// `--out`.
    Put {
        /// Target model: nc1020, pc1000, cc800, nc2000, nc3000.
        #[arg(long)]
        model: String,
        /// Directory containing the firmware ROM/NOR/NAND dumps (read-only).
        #[arg(long)]
        rom_dir: PathBuf,
        /// Path to write the modified flash image to (must not be inside
        /// `rom_dir`).
        #[arg(long)]
        out: PathBuf,
        /// Override the stored file name (only meaningful with a single file).
        #[arg(long)]
        name: Option<String>,
        /// Source file(s) to inject.
        files: Vec<PathBuf>,
    },
    /// List the user-file directory entries of a firmware image without
    /// modifying anything.
    Ls {
        /// Target model.
        #[arg(long)]
        model: String,
        /// Directory containing the firmware dumps (read-only).
        #[arg(long)]
        rom_dir: PathBuf,
    },
    /// Boot, then try each key id and report which ones cause the LCD to draw
    /// (debug helper for discovering the UI navigation keys).
    Keysweep {
        #[arg(long)]
        model: String,
        #[arg(long)]
        rom_dir: PathBuf,
        #[arg(long, default_value_t = 600)]
        frames: u32,
    },
    /// Boot and record the BRK syscalls (id, signature, handler) the firmware
    /// itself issues (debug helper).
    Syscalls {
        #[arg(long)]
        model: String,
        #[arg(long)]
        rom_dir: PathBuf,
        /// Total CPU cycles to trace (ignored when --keys is given).
        #[arg(long, default_value_t = 20_000_000)]
        cycles: u64,
        /// Boot frames before applying --keys.
        #[arg(long, default_value_t = 200)]
        boot_frames: u32,
        /// Frames to trace after the last key.
        #[arg(long, default_value_t = 120)]
        post_frames: u32,
        /// Comma-separated hex key ids to press while tracing.
        #[arg(long)]
        keys: Option<String>,
    },
    /// Boot the firmware for N frames and report any NOR mutations (debug
    /// helper for reverse-engineering the firmware's own flash writes).
    Trace {
        #[arg(long)]
        model: String,
        #[arg(long)]
        rom_dir: PathBuf,
        /// Number of frames to boot before sampling.
        #[arg(long, default_value_t = 400)]
        frames: u32,
        /// Fill the NOR image with 0xFF before booting (simulate a blank,
        /// unformatted flash to trigger the firmware formatter).
        #[arg(long, default_value_t = false)]
        blank_nor: bool,
        /// Comma-separated hex key ids to press after boot (e.g. "0E,1A,1D").
        #[arg(long)]
        keys: Option<String>,
        /// Print the LCD framebuffer as ASCII after boot + keys.
        #[arg(long, default_value_t = false)]
        dump_lcd: bool,
        /// Save the LCD framebuffer as a PNG after boot + keys.
        #[arg(long)]
        png: Option<String>,
        /// Save the (possibly modified) NOR image after the session.
        #[arg(long)]
        save_out: Option<String>,
    },
}

fn resolve_model(s: &str) -> Result<MachineModel> {
    MachineModel::from_name(s)
        .with_context(|| format!("Unknown model `{s}` (use nc1020/pc1000/cc800/nc2000/nc3000)"))
}

fn build_emulator(model: MachineModel, rom_dir: &Path) -> Result<Emulator> {
    let files = RomFiles::from_dir(rom_dir)
        .with_context(|| format!("Failed to discover firmware in {}", rom_dir.display()))?;
    files
        .validate_for_model(model)
        .with_context(|| format!("Firmware files do not match model `{}`", model.name()))?;
    let mut emu = Emulator::new(model, &files)
        .with_context(|| format!("Failed to create {} emulator", model.name()))?;
    // Match the desktop frontend, which resets the CPU after construction.
    emu.reset();
    Ok(emu)
}

/// Ensure `out` is not inside `rom_dir` (we must never write back to the
/// read-only source firmware).
fn guard_outside_rom_dir(rom_dir: &Path, out: &Path) -> Result<()> {
    let rom_dir = rom_dir
        .canonicalize()
        .unwrap_or_else(|_| rom_dir.to_path_buf());
    let out = out.canonicalize().unwrap_or_else(|_| out.to_path_buf());
    if out.starts_with(&rom_dir) {
        bail!(
            "`--out` ({}) must not be inside the read-only firmware directory ({})",
            out.display(),
            rom_dir.display()
        );
    }
    Ok(())
}

/// Save an XRGB8888 LCD framebuffer as a PNG.
fn save_lcd_png(pixels: &[u32], width: usize, height: usize, path: &str) -> Result<()> {
    use image::{ImageBuffer, Rgba};
    const SCALE: usize = 4;
    let img = ImageBuffer::from_fn((width * SCALE) as u32, (height * SCALE) as u32, |x, y| {
        let sx = (x as usize) / SCALE;
        let sy = (y as usize) / SCALE;
        let p = pixels[sy * width + sx];
        Rgba([
            ((p >> 16) & 0xFF) as u8,
            ((p >> 8) & 0xFF) as u8,
            (p & 0xFF) as u8,
            ((p >> 24) & 0xFF) as u8,
        ])
    });
    img.save(path)?;
    Ok(())
}

fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Put {
            model,
            rom_dir,
            out,
            name,
            files,
        } => {
            if files.is_empty() {
                bail!("No input file given");
            }
            guard_outside_rom_dir(&rom_dir, &out)?;

            let model = resolve_model(&model)?;
            let mut emu = build_emulator(model, &rom_dir)?;

            for file in &files {
                let data = std::fs::read(file)
                    .with_context(|| format!("Cannot read input file {}", file.display()))?;
                let default_name = file
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "GAME.BIN".to_string());
                let target_name = if files.len() == 1 {
                    name.clone().unwrap_or(default_name)
                } else {
                    default_name
                };

                let report = emu
                    .inject_file(&target_name, &data)
                    .with_context(|| format!("Injection of `{}` failed", target_name))?;

                if report.recognized {
                    println!(
                        "[ok] `{}` injected ({} / {} bytes) and recognised by firmware GFFS",
                        report.name, report.bytes_sent, report.file_size
                    );
                } else {
                    println!(
                        "[warn] `{}` injected ({} / {} bytes) but NOT found in GFFS after re-mount; \
                         the firmware may use a different syscall interface for this image",
                        report.name, report.bytes_sent, report.file_size
                    );
                }
            }

            // Persist the modified flash. NOR-based models write the NOR image;
            // models that keep the GFFS in NAND write the NAND image.
            match model {
                MachineModel::Nc1020 | MachineModel::Pc1000 | MachineModel::Cc800 => {
                    emu.save_nor(out.to_str().unwrap())
                        .context("Failed to save NOR flash")?;
                }
                MachineModel::Nc2000 | MachineModel::Nc3000 => {
                    emu.save_nand(out.to_str().unwrap())
                        .context("Failed to save NAND flash")?;
                }
            }
            println!("Saved modified flash to {}", out.display());
            Ok(())
        }
        Command::Ls { model, rom_dir } => {
            let model = resolve_model(&model)?;
            let emu = build_emulator(model, &rom_dir)?;
            let entries = emu.gffs_list();
            println!(
                "GFFS user-directory entries for {} ({}):",
                model.name(),
                entries.len()
            );
            for e in &entries {
                println!(
                    "  {:<16} len={:5} start=0x{:04X}",
                    e.name, e.length, e.start
                );
            }
            Ok(())
        }
        Command::Keysweep {
            model,
            rom_dir,
            frames,
        } => {
            let model = resolve_model(&model)?;
            let mut emu = build_emulator(model, &rom_dir)?;
            for _ in 0..frames {
                emu.run_frame();
            }
            // XRGB: ink (black) = 0xFF000000, background (white) = 0xFFFFFFFF.
            let ink = |emu: &Emulator| {
                emu.framebuffer()
                    .iter()
                    .filter(|&&p| p == 0xFF00_0000)
                    .count()
            };
            let mut prev = ink(&emu);
            println!("baseline ink pixels: {prev}");
            for id in 0u8..0x40 {
                emu.debug_press_key(id, 4);
                for _ in 0..4 {
                    emu.run_frame();
                }
                let now = ink(&emu);
                if now != prev {
                    println!(
                        "key 0x{id:02X}: ink {prev} -> {now}  (delta {:+})",
                        now as i64 - prev as i64
                    );
                    prev = now;
                }
            }
            Ok(())
        }
        Command::Syscalls {
            model,
            rom_dir,
            cycles,
            boot_frames,
            post_frames,
            keys,
        } => {
            let model = resolve_model(&model)?;
            let mut emu = build_emulator(model, &rom_dir)?;
            let seen = if let Some(keys) = keys {
                let ids: Vec<u8> = keys
                    .split(',')
                    .filter_map(|p| u8::from_str_radix(p.trim(), 16).ok())
                    .collect();
                let seen = emu.trace_key_session(boot_frames, &ids, post_frames);
                println!(
                    "firmware BRK syscalls for keys {:02X?} ({} distinct):",
                    ids,
                    seen.len()
                );
                seen
            } else {
                let seen = emu.trace_syscalls(cycles);
                println!(
                    "firmware BRK syscalls during {} cycles ({} distinct):",
                    cycles,
                    seen.len()
                );
                seen
            };
            for (id, sig, ptr) in &seen {
                println!("  id=0x{id:02X} sig=0x{sig:02X} handler=0x{ptr:04X}");
            }
            Ok(())
        }
        Command::Trace {
            model,
            rom_dir,
            frames,
            blank_nor,
            keys,
            dump_lcd,
            png,
            save_out,
        } => {
            let model = resolve_model(&model)?;
            let mut emu = build_emulator(model, &rom_dir)?;
            if blank_nor {
                emu.debug_fill_nor(0xFF);
            }
            for _ in 0..frames {
                emu.run_frame();
            }
            let before = emu.debug_nor_bytes(0, 0x100000);
            if let Some(keys) = keys {
                for part in keys.split(',') {
                    if let Ok(id) = u8::from_str_radix(part.trim(), 16) {
                        emu.debug_press_key(id, 6);
                    }
                }
            }
            for _ in 0..200 {
                emu.run_frame();
            }
            let after = emu.debug_nor_bytes(0, 0x100000);
            let mut changed = 0usize;
            let mut samples = Vec::new();
            for (i, (a, b)) in before.iter().zip(after.iter()).enumerate() {
                if a != b {
                    changed += 1;
                    if samples.len() < 48 {
                        samples.push((i, *a, *b));
                    }
                }
            }
            let ram9c0: String = (0u16..16)
                .map(|i| format!("{:02X}", emu.peek(0x9C0 + i)))
                .collect::<Vec<_>>()
                .join(" ");
            let (wc, po, eo) = emu.flash_stats();
            println!(
                "boot {} frames: {} NOR bytes changed (of {}); PC=0x{:04X} sleeping={} io06=0x{:02X} io0C=0x{:02X} flash(w={} prog={} erase={}) RAM[0x9C0..]=[{}]",
                frames,
                changed,
                before.len(),
                emu.pc(),
                emu.is_sleeping(),
                emu.peek(0x06),
                emu.peek(0x0C),
                wc,
                po,
                eo,
                ram9c0
            );
            for (i, a, b) in samples {
                println!("  [0x{i:05X}] {a:02X} -> {b:02X}");
            }
            if dump_lcd {
                let fb = emu.framebuffer();
                let (w, h) = (emu.lcd_width(), emu.lcd_height());
                println!("--- LCD {w}x{h} ---");
                for y in 0..h {
                    let mut line = String::with_capacity(w);
                    for x in 0..w {
                        // ink (black) is 0xFF000000; off (white) is 0xFFFFFFFF.
                        line.push(if fb[y * w + x] == 0xFF00_0000 {
                            '#'
                        } else {
                            '.'
                        });
                    }
                    println!("{line}");
                }
            }
            if let Some(path) = png {
                save_lcd_png(&emu.framebuffer(), emu.lcd_width(), emu.lcd_height(), &path)?;
                println!("LCD PNG saved to {path}");
            }
            if let Some(path) = save_out {
                emu.save_nor(&path)
                    .with_context(|| format!("Failed to save NOR to {path}"))?;
                println!("modified NOR saved to {path}");
            }
            Ok(())
        }
    }
}
