// GFFS (Generic File System) flash injection support.
//
// Wenquxing devices store user files in a private NOR/NAND file system that
// the firmware only exposes through a handful of "syscalls" dispatched via the
// 6502 BRK instruction (a 3-byte instruction: `BRK <id> <signature>`). The
// A host-side driver can use these syscalls by placing a tiny 6502
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
/// Protocol (the host-side reader that feeds the pump):
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
    /// Padding byte for the 16-byte name field: `b' '` for the NC1020 GFFS
    /// (space padded) and `0` for the NC2000/NC3000 (C string).
    pub name_pad: u8,
    /// CPU-cycle budget for the pump. The NC1020 finishes almost immediately,
    /// while the NC2000/NC3000 walk a long chain of nested syscalls and NAND
    /// page reads for a single file, so they need far more.
    pub pump_cycles: u64,
}

// nc1020 (non-Taiwan) pump routine, written for WQXEmu.
//
// Loaded at 0x3000 and entered with the file name already staged at 0x121C
// (plus the parameter bytes set by `InjectConfig::params`).  It opens the
// file, then loops: read one byte from the host bridge at 0x3FFF and hand it
// to the firmware's "write block" syscall, until the bridge reports end of
// file (a 0 byte), then closes the file and flushes.
//
// Disassembly (see `InjectionBridge::read` for the 0x3FFF protocol):
//
//   3000  00 01 93     BRK #$01,#$93        open file
//   3003  A9 00        LDA #$00             <- loop head
//   3005  8D 6E 04     STA $046E            clear the firmware's busy flags
//   3008  8D AE 04     STA $04AE
//   300B  AD FF 3F     LDA $3FFF            ready / end-of-file probe
//   300E  C9 00        CMP #$00
//   3010  F0 20        BEQ $3032            -> done
//   3012  AD FF 3F     LDA $3FFF            the payload byte
//   3015  8D 00 32     STA $3200            stage it in the scratch page
//   3018  A9 00        LDA #$00
//   301A  8D 0D 12     STA $120D            buffer pointer = $3200
//   301D  A9 32        LDA #$32
//   301F  8D 0E 12     STA $120E
//   3022  A9 01        LDA #$01             one block
//   3024  8D 0F 12     STA $120F
//   3027  A9 00        LDA #$00
//   3029  8D 10 12     STA $1210
//   302C  00 05 93     BRK #$05,#$93        write block
//   302F  4C 03 30     JMP $3003
//   3032  00 07 93     BRK #$07,#$93        close file
//   3035  00 01 C0     BRK #$01,#$C0        flush / return to the firmware
//
// The flag/pointer stores are re-issued on every iteration because the
// syscalls clobber those addresses.
static NC1020_PUT_ROUTINE: &[u8] = &[
    0x00, 0x01, 0x93, 0xA9, 0x00, 0x8D, 0x6E, 0x04, 0x8D, 0xAE, 0x04, 0xAD, 0xFF, 0x3F, 0xC9, 0x00,
    0xF0, 0x20, 0xAD, 0xFF, 0x3F, 0x8D, 0x00, 0x32, 0xA9, 0x00, 0x8D, 0x0D, 0x12, 0xA9, 0x32, 0x8D,
    0x0E, 0x12, 0xA9, 0x01, 0x8D, 0x0F, 0x12, 0xA9, 0x00, 0x8D, 0x10, 0x12, 0x00, 0x05, 0x93, 0x4C,
    0x03, 0x30, 0x00, 0x07, 0x93, 0x00, 0x01, 0xC0,
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
            name_pad: b' ',
            params: vec![
                (0x1219, 0x61),
                (0x121A, 0xF8),
                (0x121B, 0xFF),
                (0x122F, 0xFF),
                (0x1230, 0xFF),
                (0x1231, 0xFF),
            ],
            routine: NC1020_PUT_ROUTINE,
            pump_cycles: 4_000_000,
        }),
        MachineModel::Nc2000 => Some(InjectConfig {
            name_addr: 0x08BE,
            name_pad: 0,
            params: vec![],
            routine: NC2000_PUT_ROUTINE,
            pump_cycles: 400_000_000,
        }),
        MachineModel::Nc3000 => Some(InjectConfig {
            name_addr: 0x088D,
            name_pad: 0,
            params: vec![],
            routine: NC3000_PUT_ROUTINE,
            pump_cycles: 400_000_000,
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

// ---------------------------------------------------------------------------
// NC2000 / NC3000 (NAND) — status
// ---------------------------------------------------------------------------
//
// These models keep both the system and the user files in NAND (512+16 byte
// pages).  The NC2000 dump `nc2000.nand` is 65536 x 528 bytes; the NC3000 one
// is 131008 x 528 (pages 64.. are stored, the first 64 pages live in the
// separate `nand0` plane / are synthesised).
//
// The very first NAND page is a **file allocation table** made of 32-byte
// records with a `A8 A8` trailer, e.g.:
//
//   01 00 | BA 80 C0 79 00 00 79 00 00 00 00 00 | 01 00 | 00 00 |
//   B3 05 | FF .. | A8 A8
//   ^slot ^header fields                      ^count ^?    ^page ptrs
//
// Slots are numbered sequentially (`01 00`, `02 00`, ... `10 00` per page),
// and the tail of each record holds 16-bit NAND page numbers terminated by
// `FF FF`.  What is still unknown is how a record maps to a *name* (the names
// do not appear next to the FAT — they live in the directory pages that the
// firmware reads separately), and how chains longer than one record continue.
// Cracking those two points is what remains for NC2000/NC3000 support; the
// rest of the pipeline (per-model `save_nand`, `debug_nand_window`, a NAND
// counterpart of `block_inject`) can then mirror the NC1020 implementation.

#[cfg(test)]
mod tests {
    use super::*;

    fn blank_nor() -> Vec<u8> {
        vec![0xFF; 16 * BLOCK_BANK_SIZE]
    }

    #[test]
    fn block_inject_adds_and_lists_a_file() {
        let mut nor = blank_nor();
        let payload = vec![0xA9, 0x00, 0x85, 0x40];
        let info = block_inject(&mut nor, "GAME.BIN", &payload, None).unwrap();
        assert_eq!(info.bank, BLOCK_BANK_FIRST);
        assert_eq!(info.banks, 1);
        assert!(!info.replaced);

        let entries = block_entries(&nor);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "GAME.BIN");
        assert_eq!(entries[0].bank, BLOCK_BANK_FIRST);
        assert_eq!(entries[0].length, payload.len());
        assert_eq!(
            &nor[BLOCK_BANK_FIRST * BLOCK_BANK_SIZE + BLOCK_HEADER_SIZE + 0..][..4],
            &payload[..]
        );
    }

    #[test]
    fn block_inject_spans_banks_and_replace_reuses_them() {
        let mut nor = blank_nor();
        let big = vec![0x42; BLOCK_BANK_SIZE + 100];
        let info = block_inject(&mut nor, "BIG.BIN", &big, None).unwrap();
        assert_eq!(info.banks, 2);
        assert_eq!(block_entries(&nor)[0].banks, 2);

        // Replacing with a smaller file keeps the same starting bank.
        let small = vec![0x11; 16];
        let info = block_inject(&mut nor, "SMALL.BIN", &small, Some("BIG.BIN")).unwrap();
        assert!(info.replaced);
        assert_eq!(info.bank, BLOCK_BANK_FIRST);
        assert_eq!(info.banks, 1);
        let entries = block_entries(&nor);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "SMALL.BIN");
        // The second bank must have been released back to 0xFF.
        let second = (BLOCK_BANK_FIRST + 1) * BLOCK_BANK_SIZE;
        assert!(nor[second..second + BLOCK_BANK_SIZE]
            .iter()
            .all(|&b| b == 0xFF));
    }

    #[test]
    fn block_inject_reports_when_full() {
        let mut nor = blank_nor();
        for i in 0..(BLOCK_BANK_LAST - BLOCK_BANK_FIRST + 1) {
            block_inject(&mut nor, &format!("F{i}.BIN"), &[0x55; 8], None).unwrap();
        }
        let err = block_inject(&mut nor, "NEXT.BIN", &[0x55; 8], None).unwrap_err();
        assert!(err.to_string().contains("no free"));
    }

    #[test]
    fn nand_inject_adds_file_then_second_and_replaces() {
        const PAGES: usize = 2048;
        let mut nand = vec![0xFFu8; PAGES * NAND_FULL_PAGE];
        let params = NandFsParams {
            plane_offset: 0,
            fat_first_page: 0,
            root_dir_page: 31,
            fat_sig: [0xC0, 0x79],
            fat_const: [0x79, 0x00],
        };

        // First file -> slot 1, ~3 data pages (1500 bytes).
        let data1 = vec![0xAB; 1500];
        let info1 = nand_inject(&mut nand, &params, "A.BIN", &data1, None).unwrap();
        assert_eq!(info1.slot, 1);
        assert_eq!(info1.pages, 3);
        let fat = nand_fat_entries(&nand, &params);
        assert_eq!(fat.len(), 1);
        assert_eq!(fat[0].slot, 1);
        assert_eq!(fat[0].blocks, 3);
        assert_eq!(
            fat[0].pages,
            vec![
                info1.start_page as u16,
                info1.start_page as u16 + 1,
                info1.start_page as u16 + 2
            ]
        );
        let root = nand_root_entries(&nand, &params);
        let e = root.iter().find(|e| e.slot == 1).expect("slot 1 dir entry");
        assert_eq!(nand_name_to_string(&e.name), "A.BIN");
        let s = info1.start_page * NAND_FULL_PAGE;
        assert_eq!(&nand[s..s + 512], &data1[..512]);

        // Second file -> slot 2, contiguous after the first.
        let data2 = vec![0xCD; 800];
        let info2 = nand_inject(&mut nand, &params, "B.BIN", &data2, None).unwrap();
        assert_eq!(info2.slot, 2);
        assert_eq!(info2.start_page, info1.start_page + 3);
        let root = nand_root_entries(&nand, &params);
        assert!(root
            .iter()
            .any(|e| e.slot == 2 && nand_name_to_string(&e.name) == "B.BIN"));

        // Replace A.BIN -> reuse slot 1, free the old pages.
        let data3 = vec![0x11; 200];
        let info3 = nand_inject(&mut nand, &params, "A.BIN", &data3, Some("A.BIN")).unwrap();
        assert!(info3.replaced);
        assert_eq!(info3.slot, 1);
        let s = info1.start_page * NAND_FULL_PAGE;
        assert!(nand[s..s + NAND_FULL_PAGE].iter().all(|&b| b == 0xFF));
        // The FAT still has exactly two records (slot 1 + slot 2).
        assert_eq!(nand_fat_entries(&nand, &params).len(), 2);
    }

    #[test]
    fn nand_inject_preserves_existing_nc2000_records() {
        // Only run when the real dump is available (not in CI).
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let candidates = [
            format!("{manifest}/../../tmp/roms/nc2000/nc2000.nand"),
            "tmp/roms/nc2000/nc2000.nand".to_string(),
        ];
        let Some(raw) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            return;
        };
        let mut nand = raw.clone();
        let params = NandFsParams::for_raw(MachineModel::Nc2000).unwrap();
        let before = nand_fat_entries(&nand, &params);
        assert_eq!(before.len(), 28);

        let data = vec![0x42; 1024];
        let info = nand_inject(&mut nand, &params, "GAME.BIN", &data, None).unwrap();
        assert_eq!(info.slot, 29);
        let after = nand_fat_entries(&nand, &params);
        // 28 original + 1 new, and the original 28 are untouched.
        assert_eq!(after.len(), 29);
        for b in &before {
            let a = after.iter().find(|e| e.slot == b.slot).unwrap();
            assert_eq!(a.blocks, b.blocks);
            assert_eq!(a.pages, b.pages);
        }
        // The new record spans `blocks` contiguous pages from its start.
        let rec = after.iter().find(|e| e.slot == 29).unwrap();
        assert_eq!(rec.blocks as usize, info.pages);
        let root = nand_root_entries(&nand, &params);
        assert!(root
            .iter()
            .any(|e| e.slot == 29 && nand_name_to_string(&e.name) == "GAME.BIN"));
    }

    /// NC3000 uses the same record layout as the NC2000 but a different FAT
    /// signature (`C0 7A` / `7A 08`).  Guard against regressing to the NC2000
    /// signature, which silently parses to zero records.
    #[test]
    fn nand_inject_preserves_existing_nc3000_records() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let candidates = [
            format!("{manifest}/../../tmp/roms/nc3000/nc3000.nand"),
            "tmp/roms/nc3000/nc3000.nand".to_string(),
        ];
        let Some(raw) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            return;
        };
        let mut nand = raw.clone();
        let params = NandFsParams::for_raw(MachineModel::Nc3000).unwrap();
        // The NC3000 signature must differ from the NC2000 one.
        assert_eq!(params.fat_sig, [0xC0, 0x7A]);
        assert_eq!(params.fat_const, [0x7A, 0x08]);

        let before = nand_fat_entries(&nand, &params);
        assert_eq!(before.len(), 94, "NC3000 dump should expose 94 slots");

        let data = vec![0x42; 1024];
        let info = nand_inject(&mut nand, &params, "GAME.BIN", &data, None).unwrap();
        assert_eq!(info.slot, 95);
        let after = nand_fat_entries(&nand, &params);
        assert_eq!(after.len(), 95);
        for b in &before {
            let a = after.iter().find(|e| e.slot == b.slot).unwrap();
            assert_eq!(a.blocks, b.blocks);
            assert_eq!(a.pages, b.pages);
        }
        let rec = after.iter().find(|e| e.slot == 95).unwrap();
        assert_eq!(rec.blocks as usize, info.pages);
        let root = nand_root_entries(&nand, &params);
        assert!(root
            .iter()
            .any(|e| e.slot == 95 && nand_name_to_string(&e.name) == "GAME.BIN"));
    }
}

// ---------------------------------------------------------------------------
// NC2000 / NC3000 NAND file allocation table + root directory
// ---------------------------------------------------------------------------
//
// The first NAND pages hold a table of 32-byte records, one per file:
//
//   +0  u16 slot           1-based, sequential across records (1, 2, 3, ...)
//   +2  u16 flags          0x808A = file, 0xE0FF = directory, 0x80BA = root
//   +4  u16 signature      NC2000: C0 79, NC3000: C0 7A  (model-specific)
//   +8  u16 const          NC2000: 79 00, NC3000: 7A 08  (model-specific)
//   +14 u16 blocks         number of NAND pages the file occupies
//   +18 6 x u16            page span: [start, start+1, start+2, start+blocks]
//                          (only the first `min(blocks,3)` plus the end
//                          boundary are stored; the rest are 0xFFFF)
//   +30 A8 A8              record trailer
//
// Files are stored as a single contiguous span `[start, start+blocks)`; no
// spare-area linkage is required.  The root directory lives in page 31 as
// 16-byte records `[0:2]` slot, `[2:16]` GB2312/ASCII name (NUL terminated).
//
// Direct injection (see [`nand_inject`]) writes a FAT record + a root-dir
// entry.  Visibility notes, verified against the firmware itself (NAND read
// trace while driving the UI):
//
// * The stock menus (英汉/游戏/其他/资源管理…) are *hardcoded*: navigating them
//   reads only the font region (NC2000 pages 45698+, ~3000 16x16 glyphs) and
//   the per-block page-0 scan — never a directory page.  So an injected file
//   cannot show up in the built-in UI on a factory dump, no matter where it is
//   written.
// * User games (.bas / LAVA) are listed by *launcher applications* (文件管理 /
//   LAVA), which do browse this file system.  The root directory's plaintext
//   `[slot][GB2312]` format (also used by the secondary-volume directories
//   found at high pages, e.g. NC2000 pages 46624/46880) is what such apps
//   read, so a root-level entry is the right place for an injected file.
// * Those secondary directories reference slots beyond the single FAT (NC2000
//   slots 141..201 vs. the 28-slot FAT); no second allocation table exists
//   anywhere in the image, so they are a stale/download-tool volume that the
//   firmware does not mount.

/// One record of the NC2000/NC3000 NAND allocation table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NandFatEntry {
    /// 1-based slot number stored in the record.
    pub slot: u16,
    /// Flags word (record bytes 2..4).
    pub flags: u16,
    /// Number of NAND pages the file occupies (record bytes 14..16).
    pub blocks: u16,
    /// First page numbers listed in the record.
    pub pages: Vec<u16>,
}

/// NAND page size including the spare area (512 data + 16 spare).
pub const NAND_FULL_PAGE: usize = 528;

/// Parse the allocation table at the start of a NAND dump (logical, plane 0).
///
/// A valid record is identified by the model-specific signature at bytes `[4:6]`
/// (NC2000 `C0 79`, NC3000 `C0 7A`) and the `A8 A8` trailer.  The signature
/// check is essential: without it, blank / glyph pages that merely happen to
/// end in `A8 A8` are mistaken for records (the NC2000 dump otherwise reports
/// 56 phantom records instead of 28).
pub fn nand_fat_entries(nand: &[u8], params: &NandFsParams) -> Vec<NandFatEntry> {
    nand_fat_scan(
        nand,
        params.abs_page(params.fat_first_page),
        64,
        params.fat_sig,
    )
}

/// Low-level FAT scan starting at absolute page `fat_first_page`.
fn nand_fat_scan(
    nand: &[u8],
    fat_first_page: usize,
    max_pages: usize,
    sig: [u8; 2],
) -> Vec<NandFatEntry> {
    let mut out = Vec::new();
    let mut expect_slot: u16 = 1;
    for page in 0..max_pages {
        let base = (fat_first_page + page) * NAND_FULL_PAGE;
        if base + NAND_FULL_PAGE > nand.len() {
            break;
        }
        let mut page_has_entry = false;
        for rec in 0..(512 / 32) {
            let off = base + rec * 32;
            let e = &nand[off..off + 32];
            if e[4] != sig[0] || e[5] != sig[1] {
                continue;
            }
            if e[30] != 0xA8 || e[31] != 0xA8 {
                continue;
            }
            let slot = u16::from(e[0]) | (u16::from(e[1]) << 8);
            if slot != expect_slot {
                continue;
            }
            let flags = u16::from(e[2]) | (u16::from(e[3]) << 8);
            let blocks = u16::from(e[14]) | (u16::from(e[15]) << 8);
            let mut pages = Vec::new();
            for i in 0..6 {
                let p = u16::from(e[18 + i * 2]) | (u16::from(e[19 + i * 2]) << 8);
                if p == 0xFFFF {
                    break;
                }
                pages.push(p);
            }
            out.push(NandFatEntry {
                slot,
                flags,
                blocks,
                pages,
            });
            expect_slot += 1;
            page_has_entry = true;
        }
        if !page_has_entry {
            break;
        }
    }
    out
}

/// A directory entry of the NC2000/NC3000 NAND file system.
///
/// Directories are ordinary NAND pages made of 16-byte records living in the
/// pages owned by their slot:
///
///   +0  u16   slot of the child entry (0 = ".", 1 = "..")
///   +2  char  name, NUL terminated, GB2312 or ASCII
///
/// The root directory of the observed dumps sits in NAND page 31 and lists
/// `..`, `sysdir`, and one entry per sub directory ("应用程序", "文本文件", ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NandDirEntry {
    /// NAND page the entry was read from.
    pub page: usize,
    /// Child slot number.
    pub slot: u16,
    /// Raw name bytes (GB2312 / ASCII, NUL trimmed).
    pub name: Vec<u8>,
}

/// Render a directory name for display: ASCII stays as-is, anything else is
/// escaped as `\xNN` (there is no GB2312 table in the crate).
pub fn nand_name_to_string(raw: &[u8]) -> String {
    let mut s = String::new();
    for &b in raw {
        if (0x20..0x7F).contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{b:02X}"));
        }
    }
    s
}

/// Find directory pages in a NAND dump.
///
/// A page qualifies when most of its non-empty 16-byte records parse as a
/// NUL-terminated name.  Requiring a high parse ratio keeps glyph pages (which
/// have a similar byte shape) from being mistaken for directories.
pub fn nand_directories(nand: &[u8], min_entries: usize) -> Vec<NandDirEntry> {
    let mut out = Vec::new();
    for page in 0..nand.len() / NAND_FULL_PAGE {
        let base = page * NAND_FULL_PAGE;
        let data = &nand[base..base + 512];
        if data.iter().filter(|&&b| b == 0xFF).count() > 480 {
            continue;
        }
        let mut page_entries: Vec<NandDirEntry> = Vec::new();
        let mut non_empty = 0usize;
        let mut parsed = 0usize;
        for rec in 0..(512 / 16) {
            let off = rec * 16;
            let e = &data[off..off + 16];
            if e.iter().all(|&b| b == 0xFF) {
                continue;
            }
            non_empty += 1;
            let name = &e[2..16];
            let Some(nul) = name.iter().position(|&b| b == 0) else {
                continue;
            };
            if nul == 0 || nul > 13 {
                continue;
            }
            let raw = &name[..nul];
            let printable = raw
                .iter()
                .all(|&b| (0x20..0x7F).contains(&b) || (0xA1..=0xFE).contains(&b));
            if !printable {
                continue;
            }
            if name[nul..].iter().any(|&b| b != 0 && b != 0xFF) {
                continue;
            }
            parsed += 1;
            page_entries.push(NandDirEntry {
                page,
                slot: u16::from(e[0]) | (u16::from(e[1]) << 8),
                name: raw.to_vec(),
            });
        }
        if page_entries.len() >= min_entries && parsed * 2 >= non_empty.max(1) {
            out.append(&mut page_entries);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// NC2000 / NC3000 NAND direct file injection
// ---------------------------------------------------------------------------
//
// The firmware-driven pump (see `InjectConfig`) writes a file's *data* pages but
// never commits the FAT / directory metadata in WQXEmu, so it cannot register a
// file.  Instead we edit the on-disk NAND image directly — the same approach
// already used for the NC1020 / PC1000 / CC800.  The format, reverse-engineered
// from the device dumps, is:
//
//   * A contiguous run of data pages holds the file bytes (512 B each, the rest
//     of the last page is `0xFF`).  The firmware reads a file by spanning
//     `blocks` pages from the first page listed in its FAT record, so no
//     per-page spare linkage is required.
//   * The FAT is a sequence of 32-byte records, one per slot:
//       [0:2]  slot (LE, 1-based, sequential)
//       [2:4]  flags  (0x808A for a file, 0xE0FF for a directory)
//       [4:6]  signature `C0 79`
//       [8:10] `79 00`  (observed constant in every record)
//       [14:16] blocks (LE) — number of data pages
//       [18:30] up to 6 page numbers (LE), terminated by `0xFFFF`:
//                the first three are `start`, `start+1`, `start+2`; when
//                `blocks > 3` the fourth is `start + blocks` (the end marker).
//       [30:32] `A8 A8` trailer
//   * The root directory lives at a fixed page (31).  Its entries are 16-byte
//     records: `[0:2]` child slot (LE), `[2:16]` GB2312 / ASCII name,
//     NUL-terminated.  The root lists the top-level folders (sysdir, 應用程式,
//     文本文件) and a `..` parent (slot 0).  Adding a file entry here makes it
//     appear — and launch — from the device's root file list.

/// Geometry of the NC2000 / NC3000 NAND file system.
#[derive(Debug, Clone, Copy)]
pub struct NandFsParams {
    /// Pages that precede the main NAND plane in the *in-memory* image (the
    /// `nand0` plane).  The on-disk dump omits them, so this is `0` when
    /// editing a raw dump directly.
    pub plane_offset: usize,
    /// Logical page index where the allocation table begins.
    pub fat_first_page: usize,
    /// Logical page index of the root directory.
    pub root_dir_page: usize,
    /// Two-byte FAT record signature at offset `[4:6]`.  NC2000 uses
    /// `C0 79`; NC3000 uses `C0 7A`.
    pub fat_sig: [u8; 2],
    /// Two-byte constant written at offset `[8:10]` of every FAT record.
    /// NC2000 uses `79 00`; NC3000 uses `7A 08`.
    pub fat_const: [u8; 2],
}

impl NandFsParams {
    /// Geometry for a model that uses this NAND layout, or `None`.
    pub fn for_model(model: MachineModel) -> Option<NandFsParams> {
        match model {
            MachineModel::Nc2000 => Some(NandFsParams {
                plane_offset: 64,
                fat_first_page: 0,
                root_dir_page: 31,
                fat_sig: [0xC0, 0x79],
                fat_const: [0x79, 0x00],
            }),
            MachineModel::Nc3000 => Some(NandFsParams {
                plane_offset: 64,
                fat_first_page: 0,
                root_dir_page: 31,
                fat_sig: [0xC0, 0x7A],
                fat_const: [0x7A, 0x08],
            }),
            _ => None,
        }
    }
    /// Geometry for a raw (plane-stripped) NAND dump: no plane offset.
    pub fn for_raw(model: MachineModel) -> Option<NandFsParams> {
        NandFsParams::for_model(model).map(|p| NandFsParams {
            plane_offset: 0,
            ..p
        })
    }
    /// Absolute page index inside `nand` for a logical (main-plane) page.
    pub fn abs_page(&self, logical: usize) -> usize {
        self.plane_offset + logical
    }
}

/// Result of a direct NAND file injection.
#[derive(Debug, Clone)]
pub struct NandInjection {
    /// Allocated or reused slot number.
    pub slot: u16,
    /// Logical (main-plane) start page of the file's data.
    pub start_page: usize,
    /// Number of NAND pages the file occupies.
    pub pages: usize,
    /// Logical page of the root-directory entry that was added/updated.
    pub dir_page: usize,
    /// True when an existing file was overwritten in place.
    pub replaced: bool,
}

/// All logical pages referenced by the existing FAT (data + directory pages),
/// plus the FAT region and the root directory page.  Used to avoid clobbering
/// live data while allocating fresh pages.
fn nand_reserved_pages(nand: &[u8], params: &NandFsParams) -> std::collections::HashSet<usize> {
    let mut set = std::collections::HashSet::new();
    let fat = nand_fat_scan(
        nand,
        params.abs_page(params.fat_first_page),
        8,
        params.fat_sig,
    );
    for e in &fat {
        for &p in &e.pages {
            if p != 0xFFFF {
                set.insert(p as usize);
            }
        }
    }
    for p in params.fat_first_page..(params.fat_first_page + 8) {
        set.insert(p);
    }
    set.insert(params.root_dir_page);
    set
}

/// Find `count` consecutive, fully-blank (0xFF) NAND pages above the highest
/// used page.  Returns the logical (main-plane) start page, or `None`.
fn nand_find_free_pages(nand: &[u8], params: &NandFsParams, count: usize) -> Option<usize> {
    let reserved = nand_reserved_pages(nand, params);
    let total = nand.len() / NAND_FULL_PAGE;
    // Floor: start scanning just past the highest referenced page.
    let floor = reserved.iter().copied().max().unwrap_or(0);
    let mut run = 0usize;
    let mut start = None;
    for lp in (floor + 1)..total {
        let ap = params.abs_page(lp);
        if ap * NAND_FULL_PAGE + NAND_FULL_PAGE > nand.len() {
            break;
        }
        if reserved.contains(&lp) {
            run = 0;
            start = None;
            continue;
        }
        let seg = &nand[ap * NAND_FULL_PAGE..ap * NAND_FULL_PAGE + NAND_FULL_PAGE];
        if seg.iter().all(|&b| b == 0xFF) {
            if start.is_none() {
                start = Some(lp);
            }
            run += 1;
            if run >= count {
                return start;
            }
        } else {
            run = 0;
            start = None;
        }
    }
    None
}

/// Find the absolute byte offset of the first blank 16-byte record in the root
/// directory page, or `None` if it is full.
fn nand_find_free_dir_slot(nand: &[u8], params: &NandFsParams) -> Option<usize> {
    let ap = params.abs_page(params.root_dir_page);
    let base = ap * NAND_FULL_PAGE;
    if base + 512 > nand.len() {
        return None;
    }
    for rec in 0..(512 / 16) {
        let off = base + rec * 16;
        if nand[off..off + 16].iter().all(|&b| b == 0xFF) {
            return Some(off);
        }
    }
    None
}

/// Find the absolute byte offset of the root-directory entry whose name matches
/// `name` (case-insensitive), or `None`.
fn nand_find_dir_slot(nand: &[u8], params: &NandFsParams, name: &str) -> Option<usize> {
    let ap = params.abs_page(params.root_dir_page);
    let base = ap * NAND_FULL_PAGE;
    if base + 512 > nand.len() {
        return None;
    }
    for rec in 0..(512 / 16) {
        let off = base + rec * 16;
        let e = &nand[off..off + 16];
        if e.iter().all(|&b| b == 0xFF) {
            continue;
        }
        let name_field = &e[2..16];
        let Some(nul) = name_field.iter().position(|&b| b == 0) else {
            continue;
        };
        let raw = &name_field[..nul];
        let entry_name = nand_name_to_string(raw);
        if entry_name.eq_ignore_ascii_case(name.trim()) {
            return Some(off);
        }
    }
    None
}

/// Read the root directory entries (slot + name) of a NAND image.
pub fn nand_root_entries(nand: &[u8], params: &NandFsParams) -> Vec<NandDirEntry> {
    let ap = params.abs_page(params.root_dir_page);
    let base = ap * NAND_FULL_PAGE;
    let mut out = Vec::new();
    if base + 512 > nand.len() {
        return out;
    }
    for rec in 0..(512 / 16) {
        let off = base + rec * 16;
        let e = &nand[off..off + 16];
        if e.iter().all(|&b| b == 0xFF) {
            continue;
        }
        let name_field = &e[2..16];
        let Some(nul) = name_field.iter().position(|&b| b == 0) else {
            continue;
        };
        let raw = &name_field[..nul];
        out.push(NandDirEntry {
            page: params.root_dir_page,
            slot: u16::from(e[0]) | (u16::from(e[1]) << 8),
            name: raw.to_vec(),
        });
    }
    out
}

/// Build the page list stored in a FAT record for a file of `blocks` pages
/// starting at `start` (see the module docs for the exact layout).
fn fat_record_pages(start: u16, blocks: u16) -> Vec<u16> {
    let mut v = Vec::new();
    for i in 0..blocks.min(3) {
        v.push(start + i);
    }
    if blocks > 3 {
        v.push(start + blocks);
    }
    v
}

/// Write (or overwrite) the FAT record for `slot`.
fn write_fat_record(
    nand: &mut [u8],
    params: &NandFsParams,
    slot: u16,
    blocks: u16,
    start: u16,
) -> anyhow::Result<()> {
    let rec_index = (slot - 1) as usize;
    let page = params.abs_page(params.fat_first_page) + rec_index / 16;
    let rec = rec_index % 16;
    let off = page * NAND_FULL_PAGE + rec * 32;
    if off + 32 > nand.len() {
        anyhow::bail!("FAT record for slot {slot} is outside the image");
    }
    let e = &mut nand[off..off + 32];
    e.fill(0xFF);
    e[0] = (slot & 0xFF) as u8;
    e[1] = ((slot >> 8) & 0xFF) as u8;
    e[2] = 0x8A;
    e[3] = 0x80; // flags 0x808A (file)
    e[4] = params.fat_sig[0];
    e[5] = params.fat_sig[1]; // model-specific signature
    e[8] = params.fat_const[0];
    e[9] = params.fat_const[1];
    e[14] = (blocks & 0xFF) as u8;
    e[15] = ((blocks >> 8) & 0xFF) as u8;
    let pages = fat_record_pages(start, blocks);
    for i in 0..6 {
        if i < pages.len() {
            e[18 + i * 2] = (pages[i] & 0xFF) as u8;
            e[19 + i * 2] = ((pages[i] >> 8) & 0xFF) as u8;
        } else {
            e[18 + i * 2] = 0xFF;
            e[19 + i * 2] = 0xFF;
        }
    }
    e[30] = 0xA8;
    e[31] = 0xA8;
    Ok(())
}

/// Write (or overwrite) a root-directory entry at `off` (absolute byte offset
/// inside `nand`) for `slot` with `name` (GB2312 / ASCII, NUL-terminated).
fn write_dir_entry(nand: &mut [u8], off: usize, slot: u16, name: &str) -> anyhow::Result<()> {
    if off + 16 > nand.len() {
        anyhow::bail!("directory entry is outside the image");
    }
    let e = &mut nand[off..off + 16];
    e.fill(0xFF);
    e[0] = (slot & 0xFF) as u8;
    e[1] = ((slot >> 8) & 0xFF) as u8;
    // Name field is 14 bytes; copy the raw bytes (ASCII or GB2312) and NUL pad.
    let raw = name.trim().as_bytes();
    let n = raw.len().min(13);
    e[2..2 + n].copy_from_slice(&raw[..n]);
    e[2 + n] = 0;
    Ok(())
}

/// Directly add or update a file in the NC2000 / NC3000 NAND file system.
///
/// * `params` — geometry (use [`NandFsParams::for_raw`] for a raw dump or
///   [`NandFsParams::for_model`] for the in-memory image).
/// * `replace` — when set, an existing root entry whose name matches (or the
///   injected `name`) is overwritten in place and its old data pages freed.
///
/// Returns the slot, data-page span, and directory page that were written.
pub fn nand_inject(
    nand: &mut [u8],
    params: &NandFsParams,
    name: &str,
    data: &[u8],
    replace: Option<&str>,
) -> anyhow::Result<NandInjection> {
    let fat = nand_fat_scan(
        nand,
        params.abs_page(params.fat_first_page),
        8,
        params.fat_sig,
    );
    let max_slot = fat.iter().map(|e| e.slot).max().unwrap_or(0);
    let n_pages = data.len().div_ceil(512).max(1);

    let target_name = replace.unwrap_or(name);
    let existing_off = nand_find_dir_slot(nand, params, target_name)
        .or_else(|| nand_find_dir_slot(nand, params, name));

    let (slot, dir_off, replaced) = if let Some(off) = existing_off {
        // Reuse the existing slot by reading it back from the entry.
        let e = &nand[off..off + 2];
        let slot = u16::from(e[0]) | (u16::from(e[1]) << 8);
        (slot, off, true)
    } else {
        // The FAT scan window is 8 pages x 16 records = 128 slots (slots are
        // 1-based, so the highest allocatable slot is 128).
        if max_slot >= 128 {
            anyhow::bail!("FAT is full (slot {max_slot}); cannot allocate a new file");
        }
        let slot = max_slot + 1;
        let off = nand_find_free_dir_slot(nand, params)
            .ok_or_else(|| anyhow::anyhow!("root directory page is full"))?;
        (slot, off, false)
    };

    // Free the old data pages when replacing (the FAT record is rewritten next).
    if replaced {
        if let Some(old) = fat.iter().find(|e| e.slot == slot) {
            for &p in &old.pages {
                if p != 0xFFFF {
                    let ap = params.abs_page(p as usize);
                    let s = ap * NAND_FULL_PAGE;
                    if s + NAND_FULL_PAGE <= nand.len() {
                        nand[s..s + NAND_FULL_PAGE].fill(0xFF);
                    }
                }
            }
        }
    }

    // Allocate a contiguous run of fresh data pages.
    let start_logical = nand_find_free_pages(nand, params, n_pages)
        .ok_or_else(|| anyhow::anyhow!("no {n_pages} contiguous blank NAND pages available"))?;

    // Write the data pages (512 B each; the last page is padded with 0xFF).
    for i in 0..n_pages {
        let ap = params.abs_page(start_logical + i);
        let s = ap * NAND_FULL_PAGE;
        let end = ((i + 1) * 512).min(data.len());
        let chunk = &data[i * 512..end];
        nand[s..s + chunk.len()].copy_from_slice(chunk);
        for b in nand[s + chunk.len()..s + 512].iter_mut() {
            *b = 0xFF;
        }
        // Spare area: mark the page as a written data page (mirrors the dumps).
        nand[s + 512 + 10] = 0xAA;
        nand[s + 512 + 11] = 0x00;
    }

    write_fat_record(nand, params, slot, n_pages as u16, start_logical as u16)?;
    write_dir_entry(nand, dir_off, slot, name)?;

    Ok(NandInjection {
        slot,
        start_page: start_logical,
        pages: n_pages,
        dir_page: params.root_dir_page,
        replaced,
    })
}
