// Hardware machine abstraction for WQXEmu.
//
// All supported Wenquxing models share the same 6502 CPU, 160x80 LCD and
// SPR4096 NOR Flash controller, but differ in memory banking, IO register
// semantics and ROM file layout (NC1020/PC1000 use ROM + NOR, NC2000 adds
// NAND). The `Machine` trait abstracts those differences so the frontend
// and the generic `Emulator` shell stay model-agnostic.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::cpu::Cpu;
use crate::lcd::Lcd;
use crate::save::SaveState;

/// Supported hardware models.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MachineModel {
    /// NC1020 (SPDC1024 SoC, ROM + NOR Flash)
    Nc1020,
    /// PC1000 (different bank layout and IO semantics)
    Pc1000,
    /// CC800 (older SPDC1016 SoC, half-swapped ROM/NOR dumps)
    Cc800,
    /// NC2000 (ROM is replaced by NAND Flash)
    Nc2000,
    /// NC3000 (10.24 MHz variant, 1MB NOR + two-plane NAND)
    Nc3000,
}

impl MachineModel {
    /// Stable model name used by frontends and CLI.
    pub fn name(self) -> &'static str {
        match self {
            MachineModel::Nc1020 => "nc1020",
            MachineModel::Pc1000 => "pc1000",
            MachineModel::Cc800 => "cc800",
            MachineModel::Nc2000 => "nc2000",
            MachineModel::Nc3000 => "nc3000",
        }
    }

    /// Parse a model name (case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "nc1020" => Some(MachineModel::Nc1020),
            "pc1000" => Some(MachineModel::Pc1000),
            "cc800" => Some(MachineModel::Cc800),
            "nc2000" => Some(MachineModel::Nc2000),
            "nc3000" => Some(MachineModel::Nc3000),
            _ => None,
        }
    }
}

/// ROM / Flash files for a machine.
///
/// - `rom`: system ROM dump (NC1020 `obj_lu.bin` / `.rom`, PC1000 `.rom`)
/// - `nor`: NOR Flash dump (`.fls` / `.nor`)
/// - `nand`: NAND Flash dump (NC2000/NC3000 `.nand`)
/// - `nand0`: first NAND plane (NC2000/NC3000 `.nand0`, optional)
#[derive(Clone, Debug, Default)]
pub struct RomFiles {
    pub rom: Option<PathBuf>,
    pub nor: Option<PathBuf>,
    pub nand: Option<PathBuf>,
    pub nand0: Option<PathBuf>,
}

impl RomFiles {
    /// Build from loose file paths (any may be `None`).
    pub fn new(
        rom: Option<PathBuf>,
        nor: Option<PathBuf>,
        nand: Option<PathBuf>,
        nand0: Option<PathBuf>,
    ) -> Self {
        Self {
            rom,
            nor,
            nand,
            nand0,
        }
    }

    /// Discover firmware dumps in `dir` by file extension.
    ///
    /// Recognized extensions (case-insensitive): `.rom` / `.bin` map to the
    /// system ROM slot, `.fls` / `.nor` to NOR, `.nand` to NAND and `.nand0`
    /// to the first NAND plane. Any other entry is ignored. Fails when a
    /// slot matches more than one file or when nothing is discovered.
    pub fn from_dir(dir: &Path) -> Result<Self> {
        anyhow::ensure!(
            dir.is_dir(),
            "Firmware directory not found: {}",
            dir.display()
        );

        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<_>>()
            .with_context(|| format!("Failed to list directory: {}", dir.display()))?;
        entries.sort();

        let mut roms = Vec::new();
        let mut nors = Vec::new();
        let mut nands = Vec::new();
        let mut nands0 = Vec::new();
        for path in entries {
            if !path.is_file() {
                continue;
            }
            let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
                continue;
            };
            match extension.to_ascii_lowercase().as_str() {
                "rom" | "bin" => roms.push(path),
                "fls" | "nor" => nors.push(path),
                "nand" => nands.push(path),
                "nand0" => nands0.push(path),
                _ => {}
            }
        }

        let pick = |slot: &str, mut candidates: Vec<PathBuf>| -> Result<Option<PathBuf>> {
            match candidates.len() {
                0 => Ok(None),
                1 => Ok(candidates.pop()),
                _ => anyhow::bail!(
                    "Multiple {} dumps in {}: {}",
                    slot,
                    dir.display(),
                    candidates
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        };

        let files = Self {
            rom: pick("ROM", roms)?,
            nor: pick("NOR", nors)?,
            nand: pick("NAND", nands)?,
            nand0: pick("NAND0", nands0)?,
        };
        if files.rom.is_none()
            && files.nor.is_none()
            && files.nand.is_none()
            && files.nand0.is_none()
        {
            anyhow::bail!(
                "No firmware dumps found in {} (expected .rom/.bin, .fls/.nor, .nand or .nand0 files)",
                dir.display()
            );
        }
        Ok(files)
    }

    /// Verify that the file set matches `model`'s firmware layout: every
    /// required slot is present and every slot the model does not use is
    /// empty.
    pub fn validate_for_model(&self, model: MachineModel) -> Result<()> {
        let require = |path: &Option<PathBuf>, what: &str| -> Result<()> {
            if path.is_none() {
                anyhow::bail!("{} requires a {} dump", model.name(), what);
            }
            Ok(())
        };
        let reject = |path: &Option<PathBuf>, what: &str| -> Result<()> {
            if let Some(path) = path {
                anyhow::bail!(
                    "{} does not use a {} dump: {}",
                    model.name(),
                    what,
                    path.display()
                );
            }
            Ok(())
        };

        match model {
            MachineModel::Nc1020 | MachineModel::Pc1000 | MachineModel::Cc800 => {
                require(&self.rom, "system ROM")?;
                require(&self.nor, "NOR Flash")?;
                reject(&self.nand, "NAND Flash")?;
                reject(&self.nand0, "NAND0")?;
            }
            MachineModel::Nc2000 => {
                reject(&self.rom, "system ROM")?;
                require(&self.nor, "NOR Flash")?;
                require(&self.nand, "NAND Flash")?;
                require(&self.nand0, "NAND0")?;
            }
            MachineModel::Nc3000 => {
                reject(&self.rom, "system ROM")?;
                require(&self.nor, "NOR Flash")?;
                require(&self.nand, "NAND Flash")?;
            }
        }

        Ok(())
    }

    /// Calculate a stable identity for the complete source firmware set.
    pub fn fingerprint(&self) -> Result<u64> {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        let mut buffer = [0u8; 64 * 1024];
        for (slot, path) in [&self.rom, &self.nor, &self.nand, &self.nand0]
            .into_iter()
            .enumerate()
        {
            hash = fnv1a(hash, &[slot as u8, u8::from(path.is_some())]);
            let Some(path) = path else {
                continue;
            };
            let file = std::fs::File::open(path)?;
            hash = fnv1a(hash, &file.metadata()?.len().to_le_bytes());
            let mut reader = BufReader::new(file);
            loop {
                let count = reader.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hash = fnv1a(hash, &buffer[..count]);
            }
        }
        Ok(hash)
    }
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// A single hardware model implementation.
///
/// The machine owns every hardware component whose behavior differs
/// between models (memory map, IO registers, ROM/Flash storage, keyboard,
/// timers, audio) and exposes a uniform interface to the generic
/// `Emulator` shell, which owns the shared CPU and the frame loop.
pub trait Machine: Send {
    /// Which model this machine implements.
    fn model(&self) -> MachineModel;

    /// Reset the machine to its initial power-on state.
    fn reset(&mut self);

    /// Load system ROM / Flash files.
    fn load_rom(&mut self, files: &RomFiles) -> Result<()>;

    /// Set a keyboard key state (pressed or released).
    fn set_key(&mut self, key_id: u8, pressed: bool);

    /// Whether the device is currently in sleep mode.
    fn is_sleeping(&self) -> bool;

    /// Execute one CPU instruction. Returns the number of cycles consumed.
    fn step(&mut self, cpu: &mut Cpu) -> u64;

    /// CPU cycles executed for one host video frame.
    fn cycles_per_frame(&self) -> u64 {
        crate::lcd::CYCLES_PER_FRAME
    }

    /// Host video refresh rate for this machine.
    fn frame_rate(&self) -> u32 {
        crate::lcd::FRAME_RATE
    }

    /// Execute one video frame.
    fn run_frame(&mut self, cpu: &mut Cpu) {
        let target_cycles = self.cycles_per_frame();
        let mut cycles_this_frame = 0u64;
        while cycles_this_frame < target_cycles {
            cycles_this_frame += self.step(cpu);
        }
        self.end_of_frame(cpu);
    }

    /// Called once per frame after the CPU has run. Machines update their
    /// LCD framebuffer and handle wake-up here.
    fn end_of_frame(&mut self, cpu: &mut Cpu);

    /// Read a memory/IO address for debugging (no side effects).
    fn peek(&self, addr: u16) -> u8;

    /// Read a little-endian 16-bit word for debugging.
    fn peek_u16(&self, addr: u16) -> u16;

    /// Access the LCD controller.
    fn lcd(&self) -> &Lcd;

    /// Mutable access to the LCD controller.
    fn lcd_mut(&mut self) -> &mut Lcd;

    /// Drain pending audio samples.
    fn drain_audio(&mut self, out: &mut Vec<i16>);

    /// Load a NOR Flash dump from disk (models without NOR can leave the
    /// default implementation).
    fn load_nor(&mut self, _path: &Path) -> Result<()> {
        anyhow::bail!("NOR Flash loading is not supported for this model")
    }

    /// Persist NOR Flash contents to disk.
    fn save_nor(&self, _path: &Path) -> Result<()> {
        anyhow::bail!("NOR Flash saving is not supported for this model")
    }

    /// Enable/disable speed-up mode.
    fn set_speed_up(&mut self, _speed_up: bool) {}

    /// Capture a save state.
    fn save_state(&self, cpu: &Cpu) -> SaveState;

    /// Restore a save state.
    fn load_state(&mut self, cpu: &mut Cpu, state: &SaveState);

    /// Serialize the complete model-specific state for persistent sessions.
    fn save_persistent_state(&self) -> Result<Vec<u8>>;

    /// Restore model-specific state from a persistent session.
    fn load_persistent_state(&mut self, data: &[u8]) -> Result<()>;

    /// Identify immutable firmware that is intentionally omitted from sessions.
    fn persistent_state_identity(&self) -> u64 {
        0
    }
}

/// Shared hardware component types that machines embed.
///
/// Re-exported so machine implementations can construct them without
/// depending on the internal module paths.
pub use crate::audio::Audio as SharedAudio;
pub use crate::flash::Flash as SharedFlash;
pub use crate::input::Input as SharedInput;
pub use crate::timer::Timer as SharedTimer;

#[cfg(test)]
mod tests {
    use super::{MachineModel, RomFiles};
    use std::path::PathBuf;

    #[test]
    fn firmware_fingerprint_depends_on_slot_and_content() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.bin");
        let renamed = directory.path().join("renamed.bin");
        let different = directory.path().join("different.bin");
        std::fs::write(&first, b"firmware").unwrap();
        std::fs::write(&renamed, b"firmware").unwrap();
        std::fs::write(&different, b"different").unwrap();

        let rom = RomFiles::new(Some(first), None, None, None)
            .fingerprint()
            .unwrap();
        let same_rom = RomFiles::new(Some(renamed.clone()), None, None, None)
            .fingerprint()
            .unwrap();
        let nor = RomFiles::new(None, Some(renamed), None, None)
            .fingerprint()
            .unwrap();
        let other_rom = RomFiles::new(Some(different), None, None, None)
            .fingerprint()
            .unwrap();

        assert_eq!(rom, same_rom);
        assert_ne!(rom, nor);
        assert_ne!(rom, other_rom);
    }

    #[test]
    fn from_dir_maps_extensions_to_firmware_slots() {
        use std::ffi::OsStr;
        use std::path::Path;

        let nc1020 = tempfile::tempdir().unwrap();
        std::fs::write(nc1020.path().join("obj_lu.bin"), b"rom").unwrap();
        std::fs::write(nc1020.path().join("nc1020.fls"), b"nor").unwrap();
        std::fs::write(nc1020.path().join("notes.txt"), b"ignored").unwrap();
        std::fs::create_dir(nc1020.path().join("subdir.fls")).unwrap();

        let files = RomFiles::from_dir(nc1020.path()).unwrap();
        assert_eq!(
            files.rom.as_deref().map(Path::file_name),
            Some(Some(OsStr::new("obj_lu.bin")))
        );
        assert_eq!(
            files.nor.as_deref().map(Path::file_name),
            Some(Some(OsStr::new("nc1020.fls")))
        );
        assert!(files.nand.is_none());
        assert!(files.nand0.is_none());

        let pc1000 = tempfile::tempdir().unwrap();
        std::fs::write(pc1000.path().join("PC1000.ROM"), b"rom").unwrap();
        std::fs::write(pc1000.path().join("pc1000.NOR"), b"nor").unwrap();

        let files = RomFiles::from_dir(pc1000.path()).unwrap();
        assert_eq!(
            files.rom.as_deref().map(Path::file_name),
            Some(Some(OsStr::new("PC1000.ROM")))
        );
        assert_eq!(
            files.nor.as_deref().map(Path::file_name),
            Some(Some(OsStr::new("pc1000.NOR")))
        );

        let nc2000 = tempfile::tempdir().unwrap();
        for name in ["nc2000.nor", "nc2000.nand", "nc2000.nand0"] {
            std::fs::write(nc2000.path().join(name), b"flash").unwrap();
        }

        let files = RomFiles::from_dir(nc2000.path()).unwrap();
        assert!(files.rom.is_none());
        assert!(files.nor.is_some());
        assert!(files.nand.is_some());
        assert!(files.nand0.is_some());
    }

    #[test]
    fn from_dir_rejects_ambiguous_and_empty_directories() {
        let ambiguous = tempfile::tempdir().unwrap();
        std::fs::write(ambiguous.path().join("first.bin"), b"one").unwrap();
        std::fs::write(ambiguous.path().join("second.bin"), b"two").unwrap();

        let error = RomFiles::from_dir(ambiguous.path()).unwrap_err();
        assert!(error.to_string().contains("Multiple ROM dumps"));

        let empty = tempfile::tempdir().unwrap();
        let error = RomFiles::from_dir(empty.path()).unwrap_err();
        assert!(error.to_string().contains("No firmware dumps found"));

        let missing = empty.path().join("does-not-exist");
        assert!(RomFiles::from_dir(&missing).is_err());
    }

    #[test]
    fn firmware_sets_are_validated_for_the_selected_model() {
        let rom_and_nor = RomFiles::new(
            Some(PathBuf::from("system.bin")),
            Some(PathBuf::from("flash.nor")),
            None,
            None,
        );
        assert!(rom_and_nor.validate_for_model(MachineModel::Nc1020).is_ok());
        assert!(rom_and_nor.validate_for_model(MachineModel::Pc1000).is_ok());
        assert!(rom_and_nor.validate_for_model(MachineModel::Cc800).is_ok());

        let nc2000 = RomFiles::new(
            None,
            Some(PathBuf::from("flash.nor")),
            Some(PathBuf::from("flash.nand")),
            Some(PathBuf::from("flash.nand0")),
        );
        assert!(nc2000.validate_for_model(MachineModel::Nc2000).is_ok());
        assert!(nc2000.validate_for_model(MachineModel::Nc3000).is_ok());

        let missing_nand0 = RomFiles::new(
            None,
            Some(PathBuf::from("flash.nor")),
            Some(PathBuf::from("flash.nand")),
            None,
        );
        assert!(missing_nand0
            .validate_for_model(MachineModel::Nc2000)
            .is_err());
        assert!(missing_nand0
            .validate_for_model(MachineModel::Nc3000)
            .is_ok());

        let unexpected_rom = RomFiles::new(
            Some(PathBuf::from("system.bin")),
            Some(PathBuf::from("flash.nor")),
            Some(PathBuf::from("flash.nand")),
            None,
        );
        assert!(unexpected_rom
            .validate_for_model(MachineModel::Nc3000)
            .is_err());
    }
}
