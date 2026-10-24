// wqxflash — offline GFFS flash-injection CLI for WQXEmu.
//
// Injects `.bin` / `.bas` game files into a Wenquxing firmware image by
// driving the firmware's own file-system syscalls through the same 0x3FFF
// pump protocol used for interactive file transfer, but entirely offline and
// without ever writing to the source firmware directory (`tmp/roms`).
//
// The modified flash is written only to the user-supplied `--out` path.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use wqxemu_core::injection::{
    block_delete, block_entries, block_inject, nand_detect_params, nand_fat_entries, nand_inject,
    nand_name_to_string, nand_root_entries, NandFsParams, NandInjection,
};
use wqxemu_core::{Emulator, MachineModel, RomFiles};

/// Pick the NAND file-system constants for `model`, preferring the ones read
/// from the dump itself.  Warns when the dump disagrees with the model default:
/// the dumps shipped by the various simulators are not always what their file
/// name says.
fn nand_params_for(model: MachineModel, nand: &[u8]) -> Option<NandFsParams> {
    let detected = nand_detect_params(nand, 0);
    if let (Some(d), Some(default)) = (detected, NandFsParams::for_raw(model)) {
        if d != default {
            eprintln!(
                "[warn] this dump uses FAT constants C0 {:02X} / version {:02X}, but {} \
                 expects C0 {:02X} / version {:02X} — the image may be a different machine",
                d.fat_sig[1],
                d.fat_ver[0],
                model.name(),
                default.fat_sig[1],
                default.fat_ver[0],
            );
        }
    }
    detected.or_else(|| NandFsParams::for_raw(model))
}

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
    /// Remove a file from the PC1000 / CC800 file area (banks 2..=13) and write
    /// the image out again.  Freeing a low bank is what makes room for a new
    /// game that the firmware's list actually shows (it only lists the first
    /// five files).
    Delete {
        /// Target model: pc1000 or cc800.
        #[arg(long)]
        model: String,
        /// Directory containing the firmware dumps (read-only).
        #[arg(long)]
        rom_dir: PathBuf,
        /// Path to write the modified image to (must not be inside `rom_dir`).
        #[arg(long)]
        out: PathBuf,
        /// File to remove (ASCII names only; use --bank otherwise).
        #[arg(long)]
        name: Option<String>,
        /// Remove by bank number — works for names that are not ASCII, since
        /// the stored names are raw Big5 / GB2312 bytes.
        #[arg(long)]
        bank: Option<usize>,
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

// --- NC1020 GFFS (Wenquxing FlashSX) physical layout -----------------------
//
// Logical NOR (after the 16 KB half-swap) is 512 KiB organised as 128 × 4 KiB
// clusters.  A 128-entry allocation table — the "WQXFlashSX" superblock —
// lives at logical 0x23000; its four-byte entry at index `i` *is* physical
// cluster `i` (physical address = i * 0x1000).  Entry format:
//     byte0 = group id          (0x04 = the factory user-data volume)
//     byte1 = 0x5E (marker)
//     byte2 = cluster id within the group
//     byte3 = status            (0xC2 = user-data cluster, 1 = system)
// A free cluster is the all-0xFF entry.  Crucially the table carries **no
// physical pointer**: the firmware finds a file's bytes by scanning for the
// (group, clusterid) pair carried in the directory entry's `start` field, then
// using that slot index as the physical cluster.  A file occupies a contiguous
// run of *slots* starting at the matched slot (the firmware reads the run by
// walking consecutive clusterids).
//
// Directory entries (32 bytes) live in the entry area from 0x22000:
//     [0:2]   file-type header (DFC8 = .bin, FF09 = .bas)
//     [2:18]  GB2312/ASCII name, space padded
//     [18:24] 79 00 00 FF FF FF  (separator)
//     [24:26] length in bytes (little endian)
//     [26:28] start = (group << 8) | clusterid (little endian)
//     [28:32] 00 FF FF FF
const GFFS_CLUSTER: usize = 0x1000;
const GFFS_NCLUST: usize = 128;
const GFFS_SUPERBLOCK: usize = 0x23010;
const GFFS_ENTRY_BLOCKS: &[usize] = &[0x22000usize, 0x24000, 0x25000, 0x26000];

/// Cluster slots 32..=38 (physical 0x20000..=0x26FFF) hold the GFFS system
/// structures: settings (0x20000), the category table (0x21000), the
/// directory entry blocks (0x22000 / 0x24000 / 0x25000 / 0x26000) and the
/// superblock (0x23000).  Some of these slots are still all-0xFF in a factory
/// image — notably slot 37 == 0x25000 — but they are directory blocks, not
/// spare space.  Allocating one for payload data makes the firmware (and `ls`)
/// re-parse the file contents as directory entries, showing phantom entries.
const GFFS_RESERVED_SLOTS: std::ops::RangeInclusive<usize> = 0x20usize..=0x26;

/// Scan the superblock and return the physical cluster index (0..128) for the
/// given (group, clusterid), or None if no such allocation exists.
fn gffs_slot_for(nor: &[u8], group: u8, cid: u8) -> Option<usize> {
    for i in 0..GFFS_NCLUST {
        let o = GFFS_SUPERBLOCK + i * 4;
        if nor[o] == group && nor[o + 1] == 0x5E && nor[o + 2] == cid {
            return Some(i);
        }
    }
    None
}

/// Indices of clusters whose superblock entry is still all-0xFF.
fn gffs_free_clusters(nor: &[u8]) -> Vec<usize> {
    let mut v = Vec::new();
    for i in 0..GFFS_NCLUST {
        if GFFS_RESERVED_SLOTS.contains(&i) {
            continue;
        }
        let o = GFFS_SUPERBLOCK + i * 4;
        if nor[o..o + 4] == [0xFF, 0xFF, 0xFF, 0xFF] {
            v.push(i);
        }
    }
    v
}

/// Largest clusterid currently used within `group` (so new runs don't collide).
fn gffs_max_cid(nor: &[u8], group: u8) -> u8 {
    let mut m = 0u8;
    for i in 0..GFFS_NCLUST {
        let o = GFFS_SUPERBLOCK + i * 4;
        if nor[o] == group && nor[o + 1] == 0x5E {
            m = m.max(nor[o + 2]);
        }
    }
    m
}

/// Mark a run of cluster slots free again in the superblock.
fn gffs_free_run(nor: &mut [u8], slots: &[usize]) {
    for &s in slots {
        let o = GFFS_SUPERBLOCK + s * 4;
        nor[o..o + 4].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    }
}

/// Allocate `n` clusters in `group`, writing superblock entries.  Returns the
/// ordered list of physical slot indices (ascending, physically contiguous) and
/// the first clusterid.  Clusterids are assigned in the same order as the slots,
/// so the run is physically contiguous *and* the cid sequence matches the slot
/// sequence — robust whether the firmware scans the superblock by (group, cid)
/// or assumes within-group physical contiguity.  We pick the lowest available
/// contiguous run.
fn gffs_alloc_run(nor: &mut [u8], group: u8, n: usize) -> anyhow::Result<(Vec<usize>, u8)> {
    let free = gffs_free_clusters(nor);
    anyhow::ensure!(
        free.len() >= n,
        "not enough free GFFS clusters (need {n}, have {})",
        free.len()
    );
    // Find the lowest run of `n` contiguous free slots.
    let mut pick: Option<Vec<usize>> = None;
    let mut run: Vec<usize> = Vec::new();
    for &s in &free {
        if let Some(&last) = run.last() {
            if s == last + 1 {
                run.push(s);
            } else {
                run = vec![s];
            }
        } else {
            run = vec![s];
        }
        if run.len() == n {
            pick = Some(run.clone());
            break;
        }
    }
    let pick = pick.context("no contiguous run of free GFFS clusters (defragment needed)")?;
    let start_cid = gffs_max_cid(nor, group).wrapping_add(1);
    for (k, &s) in pick.iter().enumerate() {
        let o = GFFS_SUPERBLOCK + s * 4;
        nor[o] = group;
        nor[o + 1] = 0x5E;
        nor[o + 2] = start_cid.wrapping_add(k as u8);
        nor[o + 3] = 0xC2;
    }
    Ok((pick, start_cid))
}

/// Find the first all-0xFF 32-byte directory slot in the entry area.
fn gffs_free_entry_slot(nor: &[u8]) -> anyhow::Result<usize> {
    for &blk in GFFS_ENTRY_BLOCKS {
        let end = (blk + 0x1000).min(nor.len());
        let mut o = blk + 0x20; // skip the header slot at the block start
        while o + 32 <= end {
            if nor[o..o + 32].iter().all(|&b| b == 0xFF) {
                return Ok(o);
            }
            o += 32;
        }
    }
    anyhow::bail!("no free GFFS directory slot in the application entry area")
}

/// List every user file of an NC1020 GFFS image and resolve its physical
/// cluster via the superblock.
fn gffs_list_nc1020(nor: &[u8]) -> Vec<(usize, String, u32, u16, usize)> {
    let mut out = Vec::new();
    for &blk in GFFS_ENTRY_BLOCKS {
        let end = (blk + 0x1000).min(nor.len());
        let mut o = blk;
        while o + 32 <= end {
            if let Some(n) = gffs_entry_name(nor, o) {
                let e = &nor[o..o + 32];
                let len = u32::from(e[24]) | (u32::from(e[25]) << 8);
                let start = u16::from(e[26]) | (u16::from(e[27]) << 8);
                if start != 0xFFFF && len != 0xFFFF && len > 0 {
                    let group = (start >> 8) as u8;
                    let cid = (start & 0xFF) as u8;
                    let phys = gffs_slot_for(nor, group, cid)
                        .map(|s| s * GFFS_CLUSTER)
                        .unwrap_or(0);
                    out.push((o, n, len, start, phys));
                }
            }
            o += 32;
        }
    }
    out
}

/// Plan/execute a direct GFFS injection into a nc1020 logical NOR image.
///
/// Writes the 32-byte directory entry into the entry area (0x22000) and the
/// payload into the correct physical cluster(s), updating the WQXFlashSX
/// superblock so the firmware can locate the data.  The cluster → physical
/// mapping is:
///   * superblock entry at index `i` (logical 0x23010 + i*4) == physical
///     cluster `i`  (physical address = i * 0x1000);
///   * the directory entry's `start` = (group << 8) | clusterid;
///   * the firmware scans the superblock for that (group, clusterid) to find
///     the physical slot, then reads a contiguous run of clusterids.
///
/// This produces structurally valid GFFS storage.
///
/// Note on launcher visibility: the firmware's built-in "应用程序" (0x0E) and
/// "游戏" (0x09) menus do *not* enumerate the GFFS directory, so an injected
/// file cannot be made to appear in either one.  Evidence:
///
///   * A NOR read trace of the 0x0E handler shows it touching only the 0x21000
///     category table (via the generic copy helper at $D104) and never the
///     0x22000 entry area.  After the key press the display cache at $3000 holds
///     exactly one record — the 应用程序 *category* itself (type 0xFFC0,
///     start=0x0300) — with the count at $1125 = 1.
///   * Tagging the injected file's data as group 0x03 instead of 0x04 produces a
///     byte-identical result, confirming the menu selects on category, not on
///     the entry's group tag.
///   * The 0x09 menu draws its ten built-in titles from a hardcoded list at
///     $7B8F and reads no GFFS structure at all.
///
/// Injected files remain reachable from a file-manager / LAVA application that
/// browses the GFFS directory.
fn gffs_inject_nc1020(nor: &mut [u8], name: &str, data: &[u8], header: u16) -> anyhow::Result<()> {
    if data.len() > 0xFFFF {
        anyhow::bail!("file too large for GFFS (max 65535 bytes)");
    }
    let len = data.len();
    let nclust = len.div_ceil(GFFS_CLUSTER);
    // File *data* lives in the factory user-data volume (group 0x04, superblock
    // slots 96..116).  The 32-byte directory entry is written separately into
    // the entry area (0x22000) and its `start` field is tagged with this same
    // group, which is how the orphan factory entry (黄金英雄坛说.bin,
    // start=0x0401) is laid out.  Group 0x03 is the entry area's own volume
    // (a single cluster) and must not be used for payload data.
    let group: u8 = 0x04;

    // Locate an existing entry with the same name (update path).
    let mut existing: Option<(usize, u8, u8)> = None; // (slot_off, group, first_cid)
    'outer: for &blk in GFFS_ENTRY_BLOCKS {
        let end = (blk + 0x1000).min(nor.len());
        let mut o = blk;
        while o + 32 <= end {
            if let Some(n) = gffs_entry_name(nor, o) {
                if n.eq_ignore_ascii_case(name.trim()) {
                    let e = &nor[o..o + 32];
                    let st = u16::from(e[26]) | (u16::from(e[27]) << 8);
                    existing = Some((o, (st >> 8) as u8, (st & 0xFF) as u8));
                    break 'outer;
                }
            }
            o += 32;
        }
    }

    // Resolve / allocate the cluster run.  Returns the entry slot and the
    // ordered physical slot indices of the file's clusters.
    let (slot_off, slots): (usize, Vec<usize>) = if let Some((so, eg, ec)) = existing {
        let base = gffs_slot_for(nor, eg, ec)
            .context("existing entry references an unallocated cluster")?;
        let mut reuse = true;
        for k in 0..nclust {
            let s = base + k;
            if s >= GFFS_NCLUST {
                reuse = false;
                break;
            }
            let o = GFFS_SUPERBLOCK + s * 4;
            if nor[o] != eg || nor[o + 1] != 0x5E || nor[o + 2] != ec.wrapping_add(k as u8) {
                reuse = false;
                break;
            }
        }
        if reuse {
            (so, (0..nclust).map(|k| base + k).collect())
        } else {
            // Free the old run, then allocate a fresh one.
            let mut old = Vec::new();
            for k in 0..GFFS_NCLUST {
                let o = GFFS_SUPERBLOCK + k * 4;
                if nor[o] == eg && nor[o + 1] == 0x5E && nor[o + 2] >= ec {
                    old.push(k);
                }
            }
            gffs_free_run(nor, &old);
            let (run, _) = gffs_alloc_run(nor, group, nclust)?;
            (so, run)
        }
    } else {
        let (run, _) = gffs_alloc_run(nor, group, nclust)?;
        let slot = gffs_free_entry_slot(nor)?;
        (slot, run)
    };

    // Write the payload, one cluster at a time, into each physical slot.
    for (k, &s) in slots.iter().enumerate() {
        let cbase = s * GFFS_CLUSTER;
        let src = k * GFFS_CLUSTER;
        let take = (len - src).min(GFFS_CLUSTER);
        nor[cbase..cbase + take].copy_from_slice(&data[src..src + take]);
    }

    // Build and write the 32-byte directory entry.
    let first_cid = {
        let o = GFFS_SUPERBLOCK + slots[0] * 4;
        nor[o + 2]
    };
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
    let start = ((group as u16) << 8) | (first_cid as u16);
    e[26] = (start & 0xFF) as u8;
    e[27] = ((start >> 8) & 0xFF) as u8;
    e[28] = 0x00;
    e[29] = 0xFF;
    e[30] = 0xFF;
    e[31] = 0xFF;
    nor[slot_off..slot_off + 32].copy_from_slice(&e);
    log::info!(
        "GFFS inject: slot=0x{slot_off:05X} group=0x{group:02X} first_cid=0x{first_cid:02X} \
         clusters={:?} len={len} hdr=0x{header:04X}",
        slots
    );
    Ok(())
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

                match model {
                    MachineModel::Nc2000 | MachineModel::Nc3000 => {
                        // The firmware-driven pump cannot register files in
                        // WQXEmu, so `put` edits the NAND image directly.
                        let info = emu
                            .inject_nand_file(&target_name, &data, None)
                            .with_context(|| format!("Injection of `{}` failed", target_name))?;
                        println!(
                            "[ok] `{}` injected ({} bytes) -> NAND slot {} pages {}+{}",
                            target_name,
                            data.len(),
                            info.slot,
                            info.start_page,
                            info.pages
                        );
                    }
                    _ => {
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
                    // Verified on the CC800 firmware: its application list shows
                    // at most the first five files (lowest banks); a 6th file is
                    // ignored.  Replacing an existing file keeps it listed.
                    if model == MachineModel::Cc800 && !info.replaced && info.bank >= 10 {
                        println!(
                            "     note: the CC800 application list only shows the first five \
                             files; a file this deep may not be listed.  Consider --replace."
                        );
                    }
                    if !info.replaced && model == MachineModel::Pc1000 {
                        println!(
                            "     note: the PC1000 application list is built by scanning banks \
                             2..=13, so the new entry shows up there (verified end to end)."
                        );
                    }
                }
                MachineModel::Nc2000 | MachineModel::Nc3000 => {
                    // NC2000 / NC3000 keep the file system in NAND.  The
                    // firmware-driven pump cannot register files in WQXEmu, so
                    // we edit the NAND image directly (offline).
                    let nand_path = rom_files.nand.context("no NAND dump found in --rom-dir")?;
                    let mut nand = std::fs::read(&nand_path)
                        .with_context(|| format!("cannot read {}", nand_path.display()))?;
                    // Prefer the constants read from the dump itself: dumps of
                    // the same model can differ (NC3000-family: C0 7A vs C0 7B).
                    let params = nand_params_for(model, &nand)
                        .context("model does not use the NC2000/NC3000 NAND layout")?;
                    let info: NandInjection =
                        nand_inject(&mut nand, &params, &stored_name, &data, replace.as_deref())
                            .with_context(|| format!("inject `{stored_name}` failed"))?;
                    std::fs::write(&out_path, &nand)
                        .with_context(|| format!("cannot write {}", out_path.display()))?;
                    println!(
                        "[ok] {} `{stored_name}` ({} bytes) -> slot {} pages {}+{} -> {}",
                        if info.replaced { "updated" } else { "added" },
                        data.len(),
                        info.slot,
                        info.start_page,
                        info.pages,
                        out_path.display()
                    );
                    if !info.replaced {
                        println!(
                            "     note: the file is registered in the NAND file system (FAT + \
                             root-directory entry).  The firmware's built-in menus are hardcoded \
                             and never list user files; the entry shows up in LAVA/file-manager \
                             applications that browse this file system."
                        );
                    }
                }
            }
            Ok(())
        }
        Command::Delete {
            model,
            rom_dir,
            out,
            name,
            bank,
        } => {
            let model = resolve_model(&model)?;
            if !matches!(model, MachineModel::Pc1000 | MachineModel::Cc800) {
                anyhow::bail!("delete is only implemented for pc1000 / cc800 (block file system)");
            }
            if name.is_none() && bank.is_none() {
                anyhow::bail!("give either --name <file> or --bank <n>");
            }
            let files = RomFiles::from_dir(&rom_dir)?;
            let nor_path = files.nor.context("no NOR dump found in --rom-dir")?;
            let raw = std::fs::read(&nor_path)
                .with_context(|| format!("cannot read {}", nor_path.display()))?;
            let swapped = block_fs_swapped(&raw);
            let mut logical = if swapped {
                nc1020_swap(&raw)
            } else {
                raw.clone()
            };
            let info = block_delete(&mut logical, name.as_deref(), bank)
                .with_context(|| format!("delete `{}` failed", name.as_deref().unwrap_or("?"),))?;
            let out_bytes = if swapped {
                nc1020_swap(&logical)
            } else {
                logical
            };
            std::fs::write(&out, out_bytes)
                .with_context(|| format!("cannot write {}", out.display()))?;
            println!(
                "[ok] deleted `{}` (banks {}..{} freed) -> {}",
                info.name,
                info.bank,
                info.bank + info.banks - 1,
                out.display()
            );
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
                    for (off, name, len, start, phys) in &entries {
                        println!(
                            "  {:<16} len={:5} start=0x{:04X} data=0x{:05X} dir=0x{:05X}",
                            name, len, start, phys, off
                        );
                    }
                    Ok(())
                }
                MachineModel::Nc2000 | MachineModel::Nc3000 => {
                    let files = RomFiles::from_dir(&rom_dir)?;
                    let nand_path = files.nand.context("no NAND dump found in --rom-dir")?;
                    let raw = std::fs::read(&nand_path)?;
                    let params = nand_params_for(model, &raw)
                        .context("model does not use the NC2000/NC3000 NAND layout")?;
                    let entries = nand_fat_entries(&raw, &params);
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
                    // The root directory (page 31) maps slots to human names.
                    let root = nand_root_entries(&raw, &params);
                    let mut names: std::collections::HashMap<u16, String> =
                        std::collections::HashMap::new();
                    for d in &root {
                        names.insert(d.slot, nand_name_to_string(&d.name));
                    }
                    if !root.is_empty() {
                        println!("\nroot directory (page {}):", params.root_dir_page);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A pristine (all-0xFF) logical NOR image with room for the GFFS structures.
    fn blank_nor() -> Vec<u8> {
        vec![0xFFu8; 0x80000]
    }

    /// Tag a superblock slot as in-use: `[group] 5E [cid] C2`.
    fn mark_used(nor: &mut [u8], slot: usize, group: u8, cid: u8) {
        let o = GFFS_SUPERBLOCK + slot * 4;
        nor[o] = group;
        nor[o + 1] = 0x5E;
        nor[o + 2] = cid;
        nor[o + 3] = 0xC2;
    }

    /// An image laid out like the factory dump: slots 0..=31 tagged 0xFF
    /// (unusable), 32..=36 / 38 holding the per-category volumes, and —
    /// crucially — slot 37 (0x25000) left all-0xFF.  Slot 37 is a directory
    /// entry block that merely *looks* free; handing it out corrupts the
    /// directory, which is exactly the bug this guards against.
    fn factory_like_nor() -> Vec<u8> {
        let mut nor = blank_nor();
        for s in 0..=31 {
            mark_used(&mut nor, s, 0xFF, 0xFF);
        }
        mark_used(&mut nor, 32, 0x01, 0x00); // settings
        mark_used(&mut nor, 33, 0x00, 0x00); // category table
        mark_used(&mut nor, 34, 0x03, 0x00); // application entry area
        mark_used(&mut nor, 36, 0x05, 0x00); // GMUD
        mark_used(&mut nor, 38, 0x02, 0x00);
        nor
    }

    /// Payload bytes chosen so they would parse as a bogus directory entry if the
    /// allocator ever placed file data inside one of the entry blocks.
    #[test]
    fn gffs_inject_never_allocates_reserved_system_clusters() {
        let mut nor = factory_like_nor();
        let payload = b"HELLO.BAS\x00\x00\x00\x00\x01\x04\x01\x00";
        gffs_inject_nc1020(&mut nor, "HELLO.BAS", payload, 0xFF09).unwrap();

        // Data must not land on any directory entry block (slots 32..=38).
        let listed = gffs_list_nc1020(&nor);
        assert_eq!(listed.len(), 1, "phantom/extra entries: {listed:?}");

        let (_o, name, _len, start, phys) = &listed[0];
        assert_eq!(name, "HELLO.BAS");
        assert_eq!(
            (*start >> 8) as u8,
            0x04,
            "entry must tag the user-data volume"
        );
        for &blk in GFFS_ENTRY_BLOCKS {
            assert!(
                *phys < blk || *phys >= blk + 0x1000,
                "payload at {phys:#X} overlaps directory entry block {blk:#X}"
            );
        }
        assert!(
            !GFFS_RESERVED_SLOTS.contains(&(*phys / GFFS_CLUSTER)),
            "payload cluster {} is a reserved system cluster",
            *phys / GFFS_CLUSTER
        );
        // First usable cluster past the reserved system area: slot 39 = 0x27000.
        assert_eq!(
            *phys, 0x27000,
            "payload should land on the first non-reserved cluster"
        );
    }

    /// The allocator must skip the reserved slots even though a factory image
    /// leaves slot 37 (0x25000) all-0xFF, i.e. "free" but a directory block.
    #[test]
    fn gffs_free_clusters_excludes_reserved_slots() {
        let nor = factory_like_nor();
        let free = gffs_free_clusters(&nor);
        for s in &free {
            assert!(
                !GFFS_RESERVED_SLOTS.contains(s),
                "slot {s} is reserved but reported free"
            );
        }
        // Lowest free slot must be the first one past the system area.
        assert_eq!(
            free[0], 0x27,
            "expected first free slot 39, got {}",
            free[0]
        );
    }
}
