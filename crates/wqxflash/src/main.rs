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
use wqxemu_core::injection::{
    block_entries, block_inject, nand_directories, nand_fat_entries, nand_name_to_string,
};
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
    /// Directly add/update a game file in the GFFS directory of a flash image
    /// (works offline on the NOR dump; no firmware execution needed).
    Inject {
        /// Target model (currently nc1020; others use the same GFFS layout).
        #[arg(long)]
        model: String,
        /// Directory containing the firmware dumps (read-only).
        #[arg(long)]
        rom_dir: PathBuf,
        /// Output path for the modified NOR image.
        #[arg(long)]
        out: PathBuf,
        /// Game file to inject (.bin or .bas).
        file: PathBuf,
        /// Stored file name (default: the input file name).
        #[arg(long)]
        name: Option<String>,
        /// Entry type header (hex). Default: 0xDFC8 for .bin, 0xFF09 for .bas.
        #[arg(long)]
        header: Option<String>,
        /// Replace/update an existing file with this name (its banks are
        /// reused). Required to update a game that is already installed.
        #[arg(long)]
        replace: Option<String>,
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
        /// Append `+` to hold a key down and `-` to release it, e.g.
        /// "2E,39+,09,39-" taps F9 while SHIFT (中英数) is held.
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
        /// Print which 256-byte NOR blocks the firmware read during the
        /// session (debug helper; requires WQX_NORTRACE=1 for the model).
        #[arg(long, default_value_t = false)]
        nor_reads: bool,
        /// Print which 64-byte NAND blocks the firmware read during the
        /// session (debug helper; requires WQX_NANDTRACE=1 for the model).
        #[arg(long, default_value_t = false)]
        nand_reads: bool,
        /// Save the (possibly modified) NAND image after the session.
        #[arg(long)]
        save_nand_out: Option<String>,
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

/// NC1020 NOR on-disk <-> logical conversion (16KB half-swap per 32KB bank,
/// an involution so it works for both directions).
fn nc1020_swap(data: &[u8]) -> Vec<u8> {
    const BANK: usize = 0x8000;
    const HALF: usize = 0x4000;
    let mut out = vec![0u8; data.len()];
    let n = data.len();
    let mut off = 0;
    while off < n {
        if off + BANK > n {
            out[off..].copy_from_slice(&data[off..]);
            break;
        }
        let s = &data[off..off + BANK];
        out[off + HALF..off + BANK].copy_from_slice(&s[..HALF]);
        out[off..off + HALF].copy_from_slice(&s[HALF..]);
        off += BANK;
    }
    out
}

/// Return the stored name if `off` holds a valid GFFS directory entry.
fn gffs_entry_name(nor: &[u8], off: usize) -> Option<String> {
    if off + 32 > nor.len() {
        return None;
    }
    let e = &nor[off..off + 32];
    if e[30] != 0xFF || e[31] != 0xFF {
        return None;
    }
    if e[2..18].iter().all(|&b| b == 0xFF) {
        return None;
    }
    // Names are ASCII or GB2312.  Keep ASCII verbatim and escape anything
    // else as \xNN so a name stays unambiguous (and round-trips for --replace).
    let mut s = String::new();
    for &b in &e[2..18] {
        if b == 0x20 || b == 0x00 {
            if s.ends_with(' ') || s.is_empty() {
                continue;
            }
            s.push(' ');
        } else if (0x21..0x7F).contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{b:02X}"));
        }
    }
    let s = s.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// List every user file of an NC1020 GFFS image by scanning all directory
/// blocks (the app list lives in 0x21000, saves in 0x27000, ...).
fn gffs_list_nc1020(nor: &[u8]) -> Vec<(usize, String, u32, u16)> {
    let mut out = Vec::new();
    for blk in [0x21000usize, 0x22000, 0x23000, 0x24000, 0x27000] {
        let end = (blk + 0x1000).min(nor.len());
        let mut o = blk;
        while o + 32 <= end {
            if let Some(n) = gffs_entry_name(nor, o) {
                let e = &nor[o..o + 32];
                let len = u32::from(e[24]) | (u32::from(e[25]) << 8);
                let start = u16::from(e[26]) | (u16::from(e[27]) << 8);
                if start != 0xFFFF && len != 0xFFFF {
                    out.push((o, n, len, start));
                }
            }
            o += 32;
        }
    }
    out
}

/// Plan/execute a direct GFFS injection into a nc1020 logical NOR image.
fn gffs_inject_nc1020(nor: &mut [u8], name: &str, data: &[u8], header: u16) -> anyhow::Result<()> {
    const BLOCK: usize = 0x21000;
    if data.len() > 0xFFFF {
        anyhow::bail!("file too large for GFFS (max 65535 bytes)");
    }
    let len = data.len();

    // Collect used data ranges from all directory blocks so we don't clobber
    // an existing file's allocation (including its FF padding tail).
    let mut used: Vec<(usize, usize)> = Vec::new();
    for blk in [0x21000usize, 0x22000, 0x23000, 0x24000, 0x27000] {
        let end = (blk + 0x1000).min(nor.len());
        let mut o = blk;
        while o + 32 <= end {
            let e = &nor[o..o + 32];
            if e[30] == 0xFF && e[31] == 0xFF && !e[2..18].iter().all(|&b| b == 0xFF) {
                let ln = e[24] as usize | (e[25] as usize) << 8;
                let st = e[26] as usize | (e[27] as usize) << 8;
                if st != 0xFFFF && ln != 0xFFFF && ln > 0 {
                    used.push((st << 8, ln));
                }
            }
            o += 32;
        }
    }

    // Find an existing entry with the same name (update) in the app block.
    let mut existing_slot: Option<(usize, usize, usize)> = None; // (off, start_off, old_len)
    {
        let end = (BLOCK + 0x1000).min(nor.len());
        let mut o = BLOCK;
        while o + 32 <= end {
            if let Some(n) = gffs_entry_name(nor, o) {
                if n.eq_ignore_ascii_case(name.trim()) {
                    let e = &nor[o..o + 32];
                    let st = e[26] as usize | (e[27] as usize) << 8;
                    let ln = e[24] as usize | (e[25] as usize) << 8;
                    existing_slot = Some((o, st << 8, ln));
                    break;
                }
            }
            o += 32;
        }
    }

    let (slot_off, data_off, start) = if let Some((slot, old_off, old_len)) = existing_slot {
        if len <= old_len && old_off + len <= nor.len() {
            // Reuse the existing allocation in place.
            nor[old_off..old_off + len].copy_from_slice(data);
            (slot, old_off, (old_off >> 8) as u16)
        } else {
            let data_off = find_free_data(nor, len, &used)?;
            nor[data_off..data_off + len].copy_from_slice(data);
            (slot, data_off, (data_off >> 8) as u16)
        }
    } else {
        // Allocate a free directory slot (skip +0x000) and free data.
        let end = (BLOCK + 0x1000).min(nor.len());
        let mut free_slot = None;
        let mut o = BLOCK + 0x20;
        while o + 32 <= end {
            if nor[o..o + 32].iter().all(|&b| b == 0xFF) {
                free_slot = Some(o);
                break;
            }
            o += 32;
        }
        let slot = free_slot.context("no free GFFS directory slot in block 0x21000")?;
        let data_off = find_free_data(nor, len, &used)?;
        nor[data_off..data_off + len].copy_from_slice(data);
        (slot, data_off, (data_off >> 8) as u16)
    };

    // Build the 32-byte directory entry.
    let mut e = [0u8; 32];
    e[0] = (header & 0xFF) as u8;
    e[1] = (header >> 8) as u8;
    let mut nm = [b' '; 16];
    for (i, &b) in name.as_bytes().iter().take(16).enumerate() {
        nm[i] = b;
    }
    e[2..18].copy_from_slice(&nm);
    e[18..24].copy_from_slice(&[0x79, 0x00, 0x00, 0xFF, 0xFF, 0xFF]);
    e[24] = (len & 0xFF) as u8;
    e[25] = ((len >> 8) & 0xFF) as u8;
    e[26] = (start & 0xFF) as u8;
    e[27] = ((start >> 8) & 0xFF) as u8;
    e[28] = 0x00;
    e[29] = 0xFF;
    e[30] = 0xFF;
    e[31] = 0xFF;
    nor[slot_off..slot_off + 32].copy_from_slice(&e);
    log::info!(
        "GFFS inject: slot=0x{slot_off:05X} data=0x{data_off:05X} len={len} start=0x{start:04X} hdr=0x{header:04X}"
    );
    Ok(())
}

fn find_free_data(nor: &[u8], len: usize, used: &[(usize, usize)]) -> anyhow::Result<usize> {
    // Start high (above the system program and existing data) to avoid
    // clobbering the firmware's system-program area (which triggers the
    // "升级系统程序" recovery screen).
    let mut off = 0xA0000usize;
    while off + len <= nor.len() {
        // Skip the directory/superblock window.
        if (0x20000..0x29000).contains(&off) {
            off = 0x29000;
            continue;
        }
        let overlaps = used.iter().any(|&(u, ul)| off < u + ul && u < off + len);
        if !overlaps && nor[off..off + len].iter().all(|&b| b == 0xFF) {
            return Ok(off);
        }
        off += 0x100;
    }
    anyhow::bail!("no free data region of {len} bytes found");
}

/// PC1000/CC800 dumps may be stored half-swapped inside each 32 KB bank. The
/// firmware's info-block signature sits at bank 0 + 0x4000 in a swapped dump
/// and at bank 0 + 0x0000 once normalised (same rule as the machine loader).
fn block_fs_swapped(data: &[u8]) -> bool {
    const SIG: [u8; 7] = [0x2A, 0xBD, 0xF0, 0xD4, 0xB6, 0xBC, 0xFB];
    if data.len() < 0x4007 {
        return false;
    }
    let at_zero = data[0x0000..0x0007] == SIG;
    let at_half = data[0x4000..0x4007] == SIG;
    at_half && !at_zero
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
        Command::Inject {
            model,
            rom_dir,
            out: out_path,
            file,
            name,
            header,
            replace,
        } => {
            let model = resolve_model(&model)?;
            guard_outside_rom_dir(&rom_dir, &out_path)?;
            let rom_files = RomFiles::from_dir(&rom_dir)?;
            let nor_path = rom_files.nor.context("no NOR dump found in --rom-dir")?;
            let raw = std::fs::read(&nor_path)
                .with_context(|| format!("cannot read {}", nor_path.display()))?;

            let data =
                std::fs::read(&file).with_context(|| format!("cannot read {}", file.display()))?;
            let stored_name = name.unwrap_or_else(|| {
                file.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "GAME.BIN".to_string())
            });

            match model {
                MachineModel::Nc1020 => {
                    // On-disk <-> logical is a 16KB half-swap per 32KB bank.
                    let mut logical = nc1020_swap(&raw);
                    let hdr = match header {
                        Some(h) => u16::from_str_radix(h.trim_start_matches("0x"), 16)
                            .context("invalid --header")?,
                        None if stored_name.to_ascii_lowercase().ends_with(".bas") => 0xFF09,
                        None => 0xDFC8,
                    };
                    gffs_inject_nc1020(&mut logical, &stored_name, &data, hdr)
                        .with_context(|| format!("inject `{stored_name}` failed"))?;
                    std::fs::write(&out_path, nc1020_swap(&logical))
                        .with_context(|| format!("cannot write {}", out_path.display()))?;
                    println!(
                        "[ok] injected `{stored_name}` ({} bytes, hdr=0x{hdr:04X}) -> {}",
                        data.len(),
                        out_path.display()
                    );
                }
                MachineModel::Pc1000 | MachineModel::Cc800 => {
                    // PC1000/CC800 dumps are stored half-swapped like the
                    // NC1020; detect the orientation the same way the machine
                    // loader does so we always edit in logical order.
                    let swapped = block_fs_swapped(&raw);
                    let mut logical = if swapped {
                        nc1020_swap(&raw)
                    } else {
                        raw.clone()
                    };
                    let info = block_inject(&mut logical, &stored_name, &data, replace.as_deref())
                        .with_context(|| format!("inject `{stored_name}` failed"))?;
                    let out = if swapped {
                        nc1020_swap(&logical)
                    } else {
                        logical
                    };
                    std::fs::write(&out_path, out)
                        .with_context(|| format!("cannot write {}", out_path.display()))?;
                    println!(
                        "[ok] {} `{stored_name}` ({} bytes) -> bank {}{} -> {}",
                        if info.replaced { "updated" } else { "added" },
                        data.len(),
                        info.bank,
                        if info.banks > 1 {
                            format!("..{}", info.bank + info.banks - 1)
                        } else {
                            String::new()
                        },
                        out_path.display()
                    );
                    if !info.replaced {
                        println!(
                            "     note: the 应用程序 list is built by scanning banks 2..=13, so \
                             the new entry shows up there."
                        );
                    }
                }
                other => bail!(
                    "flash injection is implemented for nc1020/pc1000/cc800; \
                     {} stores its file system in NAND (not supported yet)",
                    other.name()
                ),
            }
            Ok(())
        }
        Command::Ls { model, rom_dir } => {
            let model = resolve_model(&model)?;
            match model {
                MachineModel::Pc1000 | MachineModel::Cc800 => {
                    let files = RomFiles::from_dir(&rom_dir)?;
                    let nor_path = files.nor.context("no NOR dump found in --rom-dir")?;
                    let raw = std::fs::read(&nor_path)?;
                    let logical = if block_fs_swapped(&raw) {
                        nc1020_swap(&raw)
                    } else {
                        raw
                    };
                    let entries = block_entries(&logical);
                    println!(
                        "block file system entries for {} ({}):",
                        model.name(),
                        entries.len()
                    );
                    for e in &entries {
                        println!(
                            "  bank {:2}  banks={} len={:5} attr=0x{:02X}  {}",
                            e.bank, e.banks, e.length, e.attr, e.name
                        );
                    }
                    let used: usize = entries.iter().map(|e| e.banks).sum();
                    println!(
                        "  file area: {} of {} banks used (2..=13)",
                        used,
                        wqxemu_core::injection::BLOCK_BANK_LAST
                            - wqxemu_core::injection::BLOCK_BANK_FIRST
                            + 1
                    );
                    Ok(())
                }
                MachineModel::Nc1020 => {
                    let files = RomFiles::from_dir(&rom_dir)?;
                    let nor_path = files.nor.context("no NOR dump found in --rom-dir")?;
                    let raw = std::fs::read(&nor_path)?;
                    let logical = nc1020_swap(&raw);
                    let entries = gffs_list_nc1020(&logical);
                    println!("GFFS entries for nc1020 ({}):", entries.len());
                    for (off, name, len, start) in &entries {
                        println!(
                            "  {:<16} len={:5} start=0x{:04X} data=0x{:05X} dir=0x{:05X}",
                            name,
                            len,
                            start,
                            (*start as usize) << 8,
                            off
                        );
                    }
                    Ok(())
                }
                MachineModel::Nc2000 | MachineModel::Nc3000 => {
                    let files = RomFiles::from_dir(&rom_dir)?;
                    let nand_path = files.nand.context("no NAND dump found in --rom-dir")?;
                    let raw = std::fs::read(&nand_path)?;
                    let entries = nand_fat_entries(&raw);
                    println!(
                        "NAND allocation table for {} ({} records):",
                        model.name(),
                        entries.len()
                    );
                    for e in &entries {
                        println!(
                            "  slot {:3}  pages={:5}  flags=0x{:04X}  first pages: {}",
                            e.slot,
                            e.blocks,
                            e.flags,
                            e.pages
                                .iter()
                                .map(|p| format!("{p}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                    // Directory entries name the allocation-table slots, so
                    // join the two to print real file names.
                    let dirs = nand_directories(&raw, 2);
                    let mut names: std::collections::HashMap<u16, String> =
                        std::collections::HashMap::new();
                    for d in &dirs {
                        names.insert(d.slot, nand_name_to_string(&d.name));
                    }
                    if !dirs.is_empty() {
                        println!();
                        for e in &entries {
                            let name = names
                                .get(&e.slot)
                                .cloned()
                                .unwrap_or_else(|| "?".to_string());
                            println!(
                                "  slot {:3}  pages={:5}  {:<16}  first pages: {}",
                                e.slot,
                                e.blocks,
                                name,
                                e.pages
                                    .iter()
                                    .map(|p| format!("{p}"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                        }
                        println!(
                            "\n  directory pages found: {}  (names without a slot are unused)",
                            dirs.iter()
                                .map(|d| d.page)
                                .collect::<std::collections::BTreeSet<_>>()
                                .len()
                        );
                    }
                    Ok(())
                }
            }
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
            // Compare a hash of the framebuffer so that screens with the same
            // amount of ink but different content are still detected.
            let fb_hash = |emu: &Emulator| {
                let mut h: u64 = 0xcbf2_9ce4_8422_2325;
                for p in emu.framebuffer() {
                    h ^= u64::from(p);
                    h = h.wrapping_mul(0x0100_0000_01b3);
                }
                h
            };
            let baseline = fb_hash(&emu);
            println!("baseline framebuffer hash: 0x{baseline:016X}");
            for id in 0u8..0x40 {
                // Fresh-ish state: press, hold long enough for a slow scan, then
                // let the UI settle before sampling.
                emu.debug_press_key(id, 8);
                for _ in 0..12 {
                    emu.run_frame();
                }
                let now = fb_hash(&emu);
                if now != baseline {
                    println!("key 0x{id:02X}: screen CHANGED");
                }
                // Return to the desktop between probes.
                emu.debug_press_key(0x2E, 4);
                for _ in 0..8 {
                    emu.run_frame();
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
            nor_reads,
            nand_reads,
            save_nand_out,
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
                // `0E` taps a key, `39+` holds it down and `39-` releases it,
                // so shifted hotkeys (e.g. NC2000 "游戏" = SHIFT + F9) work.
                for part in keys.split(',') {
                    let tok = part.trim();
                    let (tok, hold) = match tok.strip_suffix('+') {
                        Some(t) => (t, Some(true)),
                        None => match tok.strip_suffix('-') {
                            Some(t) => (t, Some(false)),
                            None => (tok, None),
                        },
                    };
                    if let Ok(id) = u8::from_str_radix(tok, 16) {
                        match hold {
                            Some(pressed) => {
                                emu.set_key(id, pressed);
                                for _ in 0..4 {
                                    emu.run_frame();
                                }
                            }
                            None => emu.debug_press_key(id, 6),
                        }
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
                "boot {} frames: {} NOR bytes changed (of {}); PC=0x{:04X} sleeping={} io06=0x{:02X} io0C=0x{:02X} flash(w={} prog={} erase={}) nand_prog={} nand_w={} RAM[0x9C0..]=[{}]",
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
                emu.nand_program_ops(),
                emu.nand_write_calls(),
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
            if nor_reads {
                let map = emu.debug_nor_read_map();
                if map.is_empty() {
                    println!("nor-reads: model did not collect a read map");
                } else {
                    println!("nor-reads: {} blocks of 64B traced", map.len());
                    let mut start: Option<usize> = None;
                    for (i, &v) in map.iter().enumerate() {
                        match (v != 0, start) {
                            (true, None) => start = Some(i),
                            (false, Some(s0)) => {
                                println!("  read 0x{:05X}..0x{:05X}", s0 * 64, i * 64);
                                start = None;
                            }
                            _ => {}
                        }
                    }
                    if let Some(s0) = start {
                        println!("  read 0x{:05X}..0x{:05X}", s0 * 64, map.len() * 64);
                    }
                }
            }
            if nand_reads {
                let map = emu.debug_nand_read_map();
                if map.is_empty() {
                    println!("nand-reads: model did not collect a read map");
                } else {
                    println!("nand-reads: {} blocks of 64B traced", map.len());
                    let mut start: Option<usize> = None;
                    for (i, &v) in map.iter().enumerate() {
                        match (v != 0, start) {
                            (true, None) => start = Some(i),
                            (false, Some(s0)) => {
                                println!(
                                    "  read 0x{:06X}..0x{:06X} (page {}..{})",
                                    s0 * 64,
                                    i * 64,
                                    s0 * 64 / 528,
                                    i * 64 / 528
                                );
                                start = None;
                            }
                            _ => {}
                        }
                    }
                    if let Some(s0) = start {
                        println!(
                            "  read 0x{:06X}..0x{:06X} (page {}..{})",
                            s0 * 64,
                            map.len() * 64,
                            s0 * 64 / 528,
                            map.len() * 64 / 528
                        );
                    }
                }
            }
            if let Some(path) = save_nand_out {
                emu.save_nand(&path)
                    .with_context(|| format!("Failed to save NAND to {path}"))?;
                println!("modified NAND saved to {path}");
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
