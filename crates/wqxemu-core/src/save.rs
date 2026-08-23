// Save state implementation for NC1020 emulator.
//
// Save states contain the complete emulator state and can be serialized
// to/from binary format for instant save/load.

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::cpu::Cpu;
use crate::machine::MachineModel;

/// Save state version for compatibility checking
const SAVE_STATE_VERSION: u32 = 1;

/// Persistent session version for compatibility checking.
pub(crate) const PERSISTENT_STATE_VERSION: u32 = 1;

/// File payload used by the standalone frontend for cross-process sessions.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PersistentState {
    pub version: u32,
    pub model: MachineModel,
    pub machine_identity: u64,
    pub cpu: Cpu,
    pub frame_count: u64,
    pub speed_up: bool,
    pub machine: Vec<u8>,
}

/// Stable fingerprint used to reject sessions from another firmware image.
pub(crate) fn persistent_identity(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Read a compressed persistent session from disk.
pub fn read_persistent_state_file(path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open state file: {}", path.display()))?;
    let mut decoder = GzDecoder::new(BufReader::new(file));
    let mut state = Vec::new();
    decoder
        .read_to_end(&mut state)
        .with_context(|| format!("Failed to decompress state file: {}", path.display()))?;
    Ok(state)
}

/// Atomically write a compressed persistent session to disk.
pub fn write_persistent_state_file(path: &Path, state: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create state directory: {}", parent.display()))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).with_context(|| {
        format!(
            "Failed to create temporary state file for {}",
            path.display()
        )
    })?;
    {
        let writer = BufWriter::new(temporary.as_file_mut());
        let mut encoder = GzEncoder::new(writer, Compression::fast());
        encoder
            .write_all(state)
            .context("Failed to compress persistent state")?;
        let mut writer = encoder
            .finish()
            .context("Failed to finish persistent state compression")?;
        writer.flush().context("Failed to flush persistent state")?;
    }
    temporary
        .as_file()
        .sync_all()
        .context("Failed to sync persistent state")?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("Failed to replace state file: {}", path.display()))?;
    Ok(())
}

/// Complete emulator save state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveState {
    /// Version number for compatibility
    pub version: u32,
    /// CPU state
    pub cpu: crate::cpu::Cpu,
    /// RAM contents (32KB)
    pub ram: Vec<u8>,
    /// IO register contents
    pub io: Vec<u8>,
    /// Timer state
    pub timer: crate::timer::Timer,
    /// Input state
    pub input: crate::input::Input,
    /// Audio state
    pub audio: crate::audio::Audio,
    /// Flash state
    pub flash: crate::flash::Flash,
    /// LCD address
    pub lcd_addr: u32,
    /// NOR Flash contents (if modified)
    pub nor_dirty: bool,
}

impl SaveState {
    /// Create a new save state from current emulator state
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cpu: &crate::cpu::Cpu,
        ram: &[u8],
        io: &[u8],
        timer: &crate::timer::Timer,
        input: &crate::input::Input,
        audio: &crate::audio::Audio,
        flash: &crate::flash::Flash,
        lcd_addr: u32,
    ) -> Self {
        Self {
            version: SAVE_STATE_VERSION,
            cpu: cpu.clone(),
            ram: ram.to_vec(),
            io: io.to_vec(),
            timer: timer.clone(),
            input: input.clone(),
            audio: audio.clone(),
            flash: flash.clone(),
            lcd_addr,
            nor_dirty: false,
        }
    }

    /// Serialize save state to bytes
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("Failed to serialize save state")
    }

    /// Deserialize save state from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let state: Self = bincode::deserialize(data).context("Failed to deserialize save state")?;

        if state.version != SAVE_STATE_VERSION {
            anyhow::bail!(
                "Save state version mismatch: expected {}, got {}",
                SAVE_STATE_VERSION,
                state.version
            );
        }

        Ok(state)
    }

    /// Calculate a simple checksum of the save state
    pub fn checksum(&self) -> u32 {
        let data = self.serialize().unwrap_or_default();
        let mut checksum: u32 = 0;
        for (i, &byte) in data.iter().enumerate() {
            checksum = checksum.wrapping_add(byte as u32 * (i as u32 + 1));
        }
        checksum
    }
}
