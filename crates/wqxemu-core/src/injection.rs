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

// ---------------------------------------------------------------------------
// PC1000 / CC800 block file system
// ---------------------------------------------------------------------------
//
// Both models keep their user files in the 512 KB NOR as whole 32 KB banks,
// starting at bank 2.  Every file begins with a 24-byte header:
//
//   +0   40 00              header magic ("normal file")
//   +2   name[16]           space padded, ASCII or WQX-internal Chinese
//   +18  00 00 00 00 00 xx  attributes (xx = 0 for applications/games)
//   +24  payload            the `.bin` image, which itself starts with
//                           `4C lo hi A5 AA 55` (JMP entry; magic)
//
// A file can span several consecutive banks; only its first bank carries the
// header.  The firmware builds its file lists by scanning the start of every
// bank in 2..=13 for the `40 00` magic, so a new file only needs a free run of
// fully-blank (0xFF) banks in that range.

/// Size of one NOR bank on the PC1000/CC800 (32 KB).
pub const BLOCK_BANK_SIZE: usize = 0x8000;
/// First bank index that can hold a user file.
pub const BLOCK_BANK_FIRST: usize = 2;
/// Last bank index that the firmware scans for file headers (inclusive).
pub const BLOCK_BANK_LAST: usize = 13;
/// Size of the per-file header that precedes the payload.
pub const BLOCK_HEADER_SIZE: usize = 24;

/// One file of the PC1000/CC800 block file system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockEntry {
    /// Stored (space-trimmed) file name, raw bytes decoded lossily.
    pub name: String,
    /// First bank holding the file.
    pub bank: usize,
    /// Number of banks the file occupies.
    pub banks: usize,
    /// Payload length in bytes (without the 24-byte header).
    pub length: usize,
    /// Attribute byte at header offset 23.
    pub attr: u8,
}

/// True when `bank` (a 32 KB slice) starts with a PC1000/CC800 file header.
pub fn block_is_header(bank: &[u8]) -> bool {
    bank.len() >= BLOCK_HEADER_SIZE && bank[0] == 0x40 && bank[1] == 0x00
}

fn block_is_blank(bank: &[u8]) -> bool {
    bank.iter().all(|&b| b == 0xFF)
}

/// Enumerate the files of the PC1000/CC800 block file system in a *logical*
/// (half-swap undone) NOR image.
pub fn block_entries(nor: &[u8]) -> Vec<BlockEntry> {
    let mut entries = Vec::new();
    let mut bank = BLOCK_BANK_FIRST;
    while bank <= BLOCK_BANK_LAST {
        let base = bank * BLOCK_BANK_SIZE;
        if base + BLOCK_BANK_SIZE > nor.len() {
            break;
        }
        if !block_is_header(&nor[base..base + BLOCK_BANK_SIZE]) {
            bank += 1;
            continue;
        }
        let raw_name = &nor[base + 2..base + 18];
        let name = block_decode_name(raw_name);
        // Count the contiguous banks that belong to this file: everything up
        // to the next header or the first completely blank bank.
        let mut banks = 1usize;
        while bank + banks <= BLOCK_BANK_LAST {
            let next = (bank + banks) * BLOCK_BANK_SIZE;
            if next + BLOCK_BANK_SIZE > nor.len() {
                break;
            }
            let slice = &nor[next..next + BLOCK_BANK_SIZE];
            if block_is_header(slice) || block_is_blank(slice) {
                break;
            }
            banks += 1;
        }
        let payload = &nor[base + BLOCK_HEADER_SIZE..base + banks * BLOCK_BANK_SIZE];
        let length = payload
            .iter()
            .rposition(|&b| b != 0xFF)
            .map(|p| p + 1)
            .unwrap_or(0);
        entries.push(BlockEntry {
            name,
            bank,
            banks,
            length,
            attr: nor[base + 23],
        });
        bank += banks;
    }
    entries
}

/// Decode the 16-byte name field.  ASCII bytes are kept as-is; the WQX
/// internal Chinese encoding (see `block_encode_name`) is rendered as `\xNN`
/// so that names stay comparable and unambiguous.
fn block_decode_name(raw: &[u8]) -> String {
    let mut out = String::new();
    for &b in raw {
        if b == 0x20 || b == 0x00 {
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
        } else if (0x21..0x7F).contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("\\x{b:02X}"));
        }
    }
    out.trim().to_string()
}

/// Encode a file name into the 16-byte, space-padded on-disk field.
fn block_encode_name(name: &str) -> [u8; 16] {
    let mut out = [b' '; 16];
    for (dst, src) in out.iter_mut().zip(name.bytes()) {
        *dst = if src.is_ascii() && src >= 0x20 {
            src
        } else {
            b'?'
        };
    }
    out
}

/// Result of an injection into the block file system.
#[derive(Debug, Clone)]
pub struct BlockInjection {
    /// Bank the file was written to.
    pub bank: usize,
    /// Number of banks it occupies.
    pub banks: usize,
    /// True when an existing file was overwritten in place.
    pub replaced: bool,
}

/// Add or update a file in the PC1000/CC800 block file system.
///
/// * `replace` – when set, the file with this (case-insensitive) name is
///   removed first, and its banks are reused even if the new file is larger.
/// * Otherwise the first free run of blank banks inside 2..=13 is used.
pub fn block_inject(
    nor: &mut [u8],
    name: &str,
    data: &[u8],
    replace: Option<&str>,
) -> anyhow::Result<BlockInjection> {
    let needed = (BLOCK_HEADER_SIZE + data.len()).div_ceil(BLOCK_BANK_SIZE);
    let total_banks = nor.len() / BLOCK_BANK_SIZE;
    if needed > BLOCK_BANK_LAST - BLOCK_BANK_FIRST + 1 {
        anyhow::bail!(
            "file is too large for the {} KB file area (needs {} banks)",
            (BLOCK_BANK_LAST - BLOCK_BANK_FIRST + 1) * BLOCK_BANK_SIZE / 1024,
            needed
        );
    }

    // Build an occupancy map of the file area.
    let mut occupied = vec![false; total_banks];
    for e in block_entries(nor) {
        let end = (e.bank + e.banks).min(total_banks);
        for slot in occupied.iter_mut().take(end).skip(e.bank) {
            *slot = true;
        }
    }

    let mut replaced = false;
    let mut freed_from = None;
    if let Some(target) = replace {
        let entries = block_entries(nor);
        let hit = entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(target.trim()))
            .or_else(|| {
                entries
                    .iter()
                    .find(|e| e.name.eq_ignore_ascii_case(name.trim()))
            });
        match hit {
            Some(e) => {
                let start = e.bank;
                let count = e.banks;
                let first = start * BLOCK_BANK_SIZE;
                nor[first..first + count * BLOCK_BANK_SIZE].fill(0xFF);
                for slot in occupied.iter_mut().skip(start).take(count) {
                    *slot = false;
                }
                freed_from = Some((start, count));
                replaced = true;
            }
            None => anyhow::bail!("no existing file named `{target}` to replace"),
        }
    }

    // Find a run of free banks (prefer reusing the freed run when it fits).
    let mut chosen = None;
    if let Some((start, count)) = freed_from {
        if count >= needed {
            chosen = Some(start);
        }
    }
    if chosen.is_none() {
        let mut run = 0usize;
        for (b, &taken) in occupied
            .iter()
            .enumerate()
            .take(BLOCK_BANK_LAST + 1)
            .skip(BLOCK_BANK_FIRST)
        {
            if !taken {
                run += 1;
                if run >= needed {
                    chosen = Some(b + 1 - needed);
                    break;
                }
            } else {
                run = 0;
            }
        }
    }
    let Some(bank) = chosen else {
        anyhow::bail!(
            "no free {} KB slot left in banks {BLOCK_BANK_FIRST}..={BLOCK_BANK_LAST}; \
             delete a file first or use --replace <name>",
            needed * BLOCK_BANK_SIZE / 1024
        );
    };

    let base = bank * BLOCK_BANK_SIZE;
    nor[base..base + needed * BLOCK_BANK_SIZE].fill(0xFF);
    nor[base] = 0x40;
    nor[base + 1] = 0x00;
    nor[base + 2..base + 18].copy_from_slice(&block_encode_name(name));
    // Attribute bytes 18..24 are zero for a plain application / game.
    nor[base + 18..base + 24].fill(0);
    nor[base + BLOCK_HEADER_SIZE..base + BLOCK_HEADER_SIZE + data.len()].copy_from_slice(data);

    Ok(BlockInjection {
        bank,
        banks: needed,
        replaced,
    })
}
