// Hardware machine abstraction for WQXEmu.
//
// All supported Wenquxing models share the same 6502 CPU, 160x80 LCD and
// SPR4096 NOR Flash controller, but differ in memory banking, IO register
// semantics and ROM file layout (NC1020/PC1000 use ROM + NOR, NC2000 adds
// NAND). The `Machine` trait abstracts those differences so the frontend
// and the generic `Emulator` shell stay model-agnostic.

use anyhow::Result;
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

    /// Parse a model name. Ids are matched byte-for-byte; the same names are
    /// used for firmware folders, core options and command-line arguments.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "nc1020" => Some(MachineModel::Nc1020),
            "pc1000" => Some(MachineModel::Pc1000),
            "cc800" => Some(MachineModel::Cc800),
            "nc2000" => Some(MachineModel::Nc2000),
            "nc3000" => Some(MachineModel::Nc3000),
            _ => None,
        }
    }

    /// Folder name below the system directory that holds this model's dumps.
    pub fn firmware_dir_name(self) -> &'static str {
        self.name()
    }

    /// Name of the system ROM dump for this model. The NC1020 and CC800 dumps
    /// keep their historical dump names.
    fn rom_file_name(self) -> Option<&'static str> {
        match self {
            MachineModel::Nc1020 => Some("obj_lu.bin"),
            MachineModel::Pc1000 => Some("pc1000.rom"),
            MachineModel::Cc800 => Some("obj.bin"),
            MachineModel::Nc2000 | MachineModel::Nc3000 => None,
        }
    }

    /// Name of the NOR Flash dump for this model. NC1020 and CC800 dumps use
    /// the historical `.fls` extension.
    fn nor_file_name(self) -> &'static str {
        match self {
            MachineModel::Nc1020 => "nc1020.fls",
            MachineModel::Pc1000 => "pc1000.nor",
            MachineModel::Cc800 => "cc800.fls",
            MachineModel::Nc2000 => "nc2000.nor",
            MachineModel::Nc3000 => "nc3000.nor",
        }
    }
}

/// ROM / Flash files for a machine.
///
/// Every frontend reads the same fixed layout below its system directory:
///
/// ```text
/// <system>/
/// └── wqxemu/
///     ├── nc1020/{obj_lu.bin, nc1020.fls}
///     ├── cc800/{obj.bin, cc800.fls}
///     ├── pc1000/{pc1000.rom, pc1000.nor}
///     ├── nc2000/{nc2000.nor, nc2000.nand, nc2000.nand0}
///     └── nc3000/{nc3000.nor, nc3000.nand[, nc3000.nand0]}
/// ```
///
/// The NC1020 and CC800 system ROMs keep their historical dump names.
#[derive(Clone, Debug, Default)]
pub struct RomFiles {
    pub rom: Option<PathBuf>,
    pub nor: Option<PathBuf>,
    pub nand: Option<PathBuf>,
    pub nand0: Option<PathBuf>,
}

/// Root folder below the system directory that holds every model.
pub const FIRMWARE_ROOT: &str = "wqxemu";

/// Return the entry of `dir` whose name equals `name` exactly.
///
/// Directory entries are compared instead of opening `dir/name` directly
/// because Windows would match names that differ only in case.
pub fn exact_child(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|entry| entry.file_name().to_str() == Some(name))
        .map(|entry| entry.path())
}

/// Firmware directory of `model`, if it exists.
pub fn firmware_directory(system_dir: &Path, model: MachineModel) -> Option<PathBuf> {
    exact_child(system_dir, FIRMWARE_ROOT)
        .filter(|path| path.is_dir())
        .and_then(|root| exact_child(&root, model.firmware_dir_name()))
        .filter(|path| path.is_dir())
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

    /// Resolve the fixed firmware set for `model`.
    ///
    /// Only the files the model uses are read, by their exact names. Every
    /// missing required file is listed in the returned error.
    pub fn for_model(system_dir: &Path, model: MachineModel) -> Result<Self> {
        let dir = firmware_directory(system_dir, model).ok_or_else(|| {
            anyhow::anyhow!(
                "Firmware directory not found: {}/wqxemu/{}",
                system_dir.display(),
                model.firmware_dir_name()
            )
        })?;

        let name = model.name();
        let mut missing = Vec::new();
        let mut slot = |file_name: &str, required: bool| -> Option<PathBuf> {
            match exact_child(&dir, file_name).filter(|path| path.is_file()) {
                Some(path) => Some(path),
                None => {
                    if required {
                        missing.push(file_name.to_owned());
                    }
                    None
                }
            }
        };

        let mut files = Self::default();
        match model {
            MachineModel::Nc1020 | MachineModel::Pc1000 | MachineModel::Cc800 => {
                if let Some(rom_name) = model.rom_file_name() {
                    files.rom = slot(rom_name, true);
                }
                files.nor = slot(model.nor_file_name(), true);
            }
            MachineModel::Nc2000 => {
                files.nor = slot(model.nor_file_name(), true);
                files.nand = slot(&format!("{name}.nand"), true);
                files.nand0 = slot(&format!("{name}.nand0"), true);
            }
            MachineModel::Nc3000 => {
                files.nor = slot(model.nor_file_name(), true);
                files.nand = slot(&format!("{name}.nand"), true);
                files.nand0 = slot(&format!("{name}.nand0"), false);
            }
        }

        if !missing.is_empty() {
            anyhow::bail!(
                "{} firmware is incomplete in {}: missing {}",
                name,
                dir.display(),
                missing.join(", ")
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
    fn for_model_reads_the_fixed_names() {
        let root = tempfile::tempdir().unwrap();
        let nc1020 = root.path().join("wqxemu").join("nc1020");
        std::fs::create_dir_all(&nc1020).unwrap();
        std::fs::write(nc1020.join("obj_lu.bin"), b"rom").unwrap();
        std::fs::write(nc1020.join("nc1020.fls"), b"nor").unwrap();
        std::fs::write(nc1020.join("notes.txt"), b"ignored").unwrap();

        let files = RomFiles::for_model(root.path(), MachineModel::Nc1020).unwrap();
        assert_eq!(files.rom, Some(nc1020.join("obj_lu.bin")));
        assert_eq!(files.nor, Some(nc1020.join("nc1020.fls")));
        assert!(files.nand.is_none());
        assert!(files.nand0.is_none());

        let nc2000 = root.path().join("wqxemu").join("nc2000");
        std::fs::create_dir_all(&nc2000).unwrap();
        for name in ["nc2000.nor", "nc2000.nand", "nc2000.nand0"] {
            std::fs::write(nc2000.join(name), b"flash").unwrap();
        }

        let files = RomFiles::for_model(root.path(), MachineModel::Nc2000).unwrap();
        assert!(files.rom.is_none());
        assert!(files.nor.is_some());
        assert!(files.nand.is_some());
        assert!(files.nand0.is_some());

        let nc3000 = root.path().join("wqxemu").join("nc3000");
        std::fs::create_dir_all(&nc3000).unwrap();
        std::fs::write(nc3000.join("nc3000.nor"), b"nor").unwrap();
        std::fs::write(nc3000.join("nc3000.nand"), b"nand").unwrap();
        let files = RomFiles::for_model(root.path(), MachineModel::Nc3000).unwrap();
        assert!(files.nand0.is_none(), "nc3000.nand0 is optional");
    }

    #[test]
    fn for_model_rejects_missing_and_misnamed_files() {
        let root = tempfile::tempdir().unwrap();
        let nc1020 = root.path().join("wqxemu").join("nc1020");
        std::fs::create_dir_all(&nc1020).unwrap();
        std::fs::write(nc1020.join("OBJ_LU.BIN"), b"rom").unwrap();
        std::fs::write(nc1020.join("nc1020.fls"), b"nor").unwrap();

        let error = RomFiles::for_model(root.path(), MachineModel::Nc1020).unwrap_err();
        assert!(error.to_string().contains("obj_lu.bin"), "{error}");

        let empty = tempfile::tempdir().unwrap();
        let error = RomFiles::for_model(empty.path(), MachineModel::Nc1020).unwrap_err();
        assert!(
            error.to_string().contains("Firmware directory not found"),
            "{error}"
        );
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
