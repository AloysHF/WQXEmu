// GFFS (Generic File System) flash injection support.
//
// Wenquxing devices store user files in a private NOR/NAND file system that
// the firmware only exposes through a handful of "syscalls" dispatched via the
// 6502 BRK instruction (a 3-byte instruction: `BRK <id> <signature>`). The
// reference tool `wangyu-NC2000` drives these syscalls by placing a tiny 6502
// "pump" routine at 0x3000 that reads file bytes from the magic IO port
// 0x3FFF (one `1` ready byte followed by one data byte, terminating with a
// `0`) and feeds them to the firmware's write-block syscall.
//
// We reproduce that exact protocol in WQXEmu so that files can be injected
// offline (no interactive shell) into a copy of the firmware image. The on-disk
// data layout is entirely owned by the firmware, so this is the only reliable
// way to add files without breaking the file system.

use std::collections::VecDeque;

use crate::machine::MachineModel;

/// Reads performed by the 0x3000 pump routine against the magic port 0x3FFF.
///
/// Protocol (matches `dummy_io_for_read` in wangyu's `cmd.cpp`):
/// the host returns `1` on even reads and the next queued byte on odd reads;
/// when the queue is empty the next read returns `0` to signal end-of-file.
#[derive(Default)]
pub struct InjectionBridge {
    /// Whether the bridge is currently active (only then does 0x3FFF read
    /// as the magic port instead of normal RAM).
    pub active: bool,
    queue: VecDeque<u8>,
    cnt: i32,
    /// Total bytes delivered to the firmware so far.
    pub bytes_sent: usize,
}

impl InjectionBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Arm the bridge with the bytes of the file to be injected.
    pub fn start(&mut self, data: Vec<u8>) {
        self.queue = data.into();
        self.cnt = 0;
        self.bytes_sent = 0;
        self.active = true;
    }

    /// Deactivate the bridge (returns normal memory behaviour for 0x3FFF).
    pub fn stop(&mut self) {
        self.active = false;
        self.queue.clear();
        self.cnt = -1;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Called by the machine's memory read when the CPU accesses 0x3FFF while
    /// the bridge is active. Returns `None` when the address is not the magic
    /// port or the bridge is idle so the caller can fall through to RAM.
    pub fn read(&mut self, addr: u16) -> Option<u8> {
        if !self.active || addr != 0x3FFF {
            return None;
        }
        if self.queue.is_empty() {
            self.active = false;
            return Some(0);
        }
        let value = if self.cnt % 2 == 0 {
            1
        } else {
            let byte = self.queue.pop_front().unwrap();
            self.bytes_sent += 1;
            byte
        };
        self.cnt += 1;
        Some(value)
    }
}

/// A single GFFS directory entry as recovered from the on-disk directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GffsEntry {
    pub name: String,
    pub length: u32,
    pub start: u16,
}

/// Parse the 32-byte directory entries starting at `base` inside a *logical*
/// (already un-swapped) NOR image. Entries with a non-printable or empty name
/// are skipped.
pub fn gffs_parse_dir(nor: &[u8], base: usize) -> Vec<GffsEntry> {
    let mut entries = Vec::new();
    if base + 32 > nor.len() {
        return entries;
    }
    let mut off = base;
    while off + 32 <= nor.len() {
        let e = &nor[off..off + 32];
        // Two trailing bytes must be 0xFF 0xFF for a valid entry.
        if e[30] != 0xFF || e[31] != 0xFF {
            break;
        }
        let raw_name = &e[2..18];
        if !raw_name
            .iter()
            .all(|&b| b.is_ascii() && (b == b' ' || b.is_ascii_graphic()))
        {
            off += 32;
            continue;
        }
        let name = String::from_utf8_lossy(raw_name).trim_end().to_string();
        if name.is_empty() {
            off += 32;
            continue;
        }
        let length = u32::from(e[24]) | (u32::from(e[25]) << 8);
        let start = u16::from(e[26]) | (u16::from(e[27]) << 8);
        entries.push(GffsEntry {
            name,
            length,
            start,
        });
        off += 32;
    }
    entries
}

/// Per-model parameters for the firmware-driven file injection pump.
pub struct InjectConfig {
    /// RAM address where the (space-padded, 16-byte) file name is placed.
    pub name_addr: u16,
    /// RAM locations the firmware expects to be pre-loaded with parameters
    /// (file attributes / write mode). Empty for models whose pump routine
    /// initialises these itself.
    pub params: Vec<(u16, u8)>,
    /// The exact 6502 pump routine copied to 0x3000.
    pub routine: &'static [u8],
}

// nc1020 (non-Taiwan) pump routine, copied verbatim from wangyu's `cmd.cpp`.
static NC1020_PUT_ROUTINE: &[u8] = &[
    0xea, 0xea, 0xea, 0x00, 0x01, 0x93, 0xA9, 0x00, 0x8D, 0x6E, 0x04, 0x8D, 0xAE, 0x04, 0xAD, 0xFF,
    0x3F, 0xC9, 0x00, 0xF0, 0x20, 0xAD, 0xFF, 0x3F, 0x8D, 0x00, 0x32, 0xA9, 0x00, 0x8D, 0x0D, 0x12,
    0xA9, 0x32, 0x8D, 0x0E, 0x12, 0xA9, 0x01, 0x8D, 0x0F, 0x12, 0xA9, 0x00, 0x8D, 0x10, 0x12, 0x00,
    0x05, 0x93, 0xB8, 0x50, 0xD1, 0x00, 0x07, 0x93, 0x00, 0x01, 0xC0,
];

// nc2000 (non-Taiwan, is_nc2000_rom) pump routine.
static NC2000_PUT_ROUTINE: &[u8] = &[
    0x00, 0x1D, 0x05, 0xA9, 0x70, 0x8D, 0xFA, 0x08, 0xA9, 0xEF, 0x8D, 0xFB, 0x08, 0x8D, 0xFC, 0x08,
    0x00, 0x15, 0x05, 0xA9, 0x00, 0x8D, 0xF6, 0x03, 0xAD, 0xFF, 0x3F, 0xC9, 0x00, 0xF0, 0x21, 0xAD,
    0xFF, 0x3F, 0x8D, 0x00, 0x32, 0xA9, 0x00, 0x85, 0xDD, 0xA9, 0x32, 0x85, 0xDE, 0xA9, 0x01, 0x8D,
    0xF7, 0x08, 0xA9, 0x00, 0x8D, 0xF8, 0x08, 0x8D, 0xF9, 0x08, 0x00, 0x18, 0x05, 0xB8, 0x50, 0xD3,
    0x00, 0x17, 0x05, 0x00, 0x01, 0xC0,
];

// nc3000 pump routine.
static NC3000_PUT_ROUTINE: &[u8] = &[
    0xA9, 0x70, 0x8D, 0xC9, 0x08, 0xA9, 0xEF, 0x8D, 0xCA, 0x08, 0x8D, 0xCB, 0x08, 0x00, 0x15, 0x05,
    0xA9, 0x00, 0x8D, 0xF6, 0x03, 0xAD, 0xFF, 0x3F, 0xC9, 0x00, 0xF0, 0x21, 0xAD, 0xFF, 0x3F, 0x8D,
    0x00, 0x32, 0xA9, 0x00, 0x85, 0xE0, 0xA9, 0x32, 0x85, 0xE1, 0xA9, 0x01, 0x8D, 0xC6, 0x08, 0xA9,
    0x00, 0x8D, 0xC7, 0x08, 0x8D, 0xC8, 0x08, 0x00, 0x18, 0x05, 0x4C, 0x10, 0x30, 0x00, 0x17, 0x05,
    0x00, 0x28, 0x05, 0x4C, 0x40, 0x30,
];

/// Returns the injection configuration for `model`, or `None` when the model
/// is not yet supported by the firmware-driven pump.
pub fn inject_config(model: MachineModel) -> Option<InjectConfig> {
    match model {
        MachineModel::Nc1020 => Some(InjectConfig {
            name_addr: 0x121C,
            params: vec![
                (0x1219, 0x61),
                (0x121A, 0xF8),
                (0x121B, 0xFF),
                (0x122F, 0xFF),
                (0x1230, 0xFF),
                (0x1231, 0xFF),
            ],
            routine: NC1020_PUT_ROUTINE,
        }),
        MachineModel::Nc2000 => Some(InjectConfig {
            name_addr: 0x08BE,
            params: vec![],
            routine: NC2000_PUT_ROUTINE,
        }),
        MachineModel::Nc3000 => Some(InjectConfig {
            name_addr: 0x088D,
            params: vec![],
            routine: NC3000_PUT_ROUTINE,
        }),
        MachineModel::Pc1000 | MachineModel::Cc800 => None,
    }
}
