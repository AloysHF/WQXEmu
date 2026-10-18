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
    Emulator::new(model, &files)
        .with_context(|| format!("Failed to create {} emulator", model.name()))
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
    }
}
