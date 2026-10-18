// Generic emulator shell around a hardware `Machine`.
//
// The shell owns the shared 6502 CPU and the frame loop, while the
// model-specific hardware lives in the `Machine` implementation. This is
// the type frontends interact with; it stays model-agnostic.

use std::path::Path;

use anyhow::{Context, Result};

use crate::audio::SAMPLE_RATE;
use crate::cpu::{Cpu, RESET_VECTOR};
use crate::injection::{inject_config, GffsEntry};
use crate::lcd::{LCD_HEIGHT, LCD_WIDTH};
use crate::machine::{Machine, MachineModel, RomFiles};
use crate::machines;
use crate::save::{PersistentState, SaveState, PERSISTENT_STATE_VERSION};

/// Result of a single file-injection attempt.
#[derive(Debug, Clone)]
pub struct InjectReport {
    /// File name staged into the GFFS directory.
    pub name: String,
    /// Number of payload bytes delivered to the firmware.
    pub bytes_sent: usize,
    /// Original file size in bytes.
    pub file_size: usize,
    /// Whether the file is visible in the on-disk GFFS directory after a
    /// clean re-mount, i.e. the firmware actually accepted and persisted it.
    pub recognized: bool,
    /// Full list of user-file directory entries after the operation.
    pub entries: Vec<GffsEntry>,
}

/// Main emulator struct.
pub struct Emulator {
    /// CPU state
    pub cpu: Cpu,
    /// Model-specific hardware
    machine: Box<dyn Machine>,
    /// Frame counter
    frame_count: u64,
    /// Speed-up mode
    speed_up: bool,
}

impl Emulator {
    /// Create an emulator for the given model, loading its ROM files.
    pub fn new(model: MachineModel, files: &RomFiles) -> Result<Self> {
        let mut machine = machines::create_machine(model, files)?;
        machine.reset();

        let mut emu = Self {
            cpu: Cpu::new(),
            machine,
            frame_count: 0,
            speed_up: false,
        };

        // Read the reset vector from the machine's memory
        let reset_vec = emu.machine.peek_u16(RESET_VECTOR);
        emu.cpu.pc = reset_vec;
        log::info!(
            "Emulator initialized: model={}, PC=0x{:04X}",
            model.name(),
            reset_vec
        );

        Ok(emu)
    }

    /// Convenience constructor for the NC1020 model (ROM + optional NOR).
    pub fn from_rom(rom_path: &str, nor_path: Option<&str>) -> Result<Self> {
        let files = RomFiles::new(
            Some(Path::new(rom_path).to_path_buf()),
            nor_path.map(|p| Path::new(p).to_path_buf()),
            None,
            None,
        );
        Self::new(MachineModel::Nc1020, &files)
    }

    /// Reset the emulator to its initial power-on state.
    pub fn reset(&mut self) {
        self.machine.reset();
        self.cpu.reset(0);
        let reset_vec = self.machine.peek_u16(RESET_VECTOR);
        self.cpu.pc = reset_vec;
        self.frame_count = 0;
        log::info!("Emulator reset, PC=0x{:04X}", reset_vec);
    }

    /// Load a NOR Flash file (supported by machines that use NOR).
    pub fn load_nor(&mut self, path: &str) -> Result<()> {
        self.machine.load_nor(Path::new(path))
    }

    /// Save the NOR Flash contents to a file.
    pub fn save_nor(&self, path: &str) -> Result<()> {
        self.machine.save_nor(Path::new(path))
    }

    /// Save the NAND Flash contents to a file (models that keep the GFFS in NAND).
    pub fn save_nand(&self, path: &str) -> Result<()> {
        self.machine.save_nand(Path::new(path))
    }

    /// Set a key state.
    pub fn set_key(&mut self, key_id: u8, pressed: bool) {
        self.machine.set_key(key_id, pressed);
    }

    /// Set speed-up mode.
    pub fn set_speed_up(&mut self, speed_up: bool) {
        self.speed_up = speed_up;
        self.machine.set_speed_up(speed_up);
    }

    /// Run one machine video frame.
    pub fn run_frame(&mut self) {
        self.machine.run_frame(&mut self.cpu);
        self.frame_count += 1;
    }

    /// Run the CPU directly for `cycles` (bypassing the per-frame LCD/EOFrame
    /// work). Used by the flash-injection flow to boot the firmware and pump
    /// the file quickly without frame pacing.
    pub fn run_cycles(&mut self, cycles: u64) {
        let mut remaining = cycles;
        while remaining > 0 {
            let used = self.machine.step(&mut self.cpu);
            remaining = remaining.saturating_sub(used);
        }
    }

    /// Boot the firmware far enough that its BRK syscall dispatcher and the
    /// GFFS file system are initialised. wangyu's `put` relies on the firmware
    /// already being up (its emulator runs continuously); for an offline
    /// batch injection we have to run the boot ourselves before diverting the
    /// CPU to the pump routine at 0x3000.
    ///
    /// The firmware parks itself in sleep mode once it reaches the main menu,
    /// so we stop as soon as `is_sleeping()` is true, capped at a frame budget
    /// so we never hang if a model never sleeps.
    pub fn boot_for_injection(&mut self) {
        let mut frames = 0;
        const MAX_FRAMES: u32 = 400;
        while frames < MAX_FRAMES && !self.machine.is_sleeping() {
            self.run_frame();
            frames += 1;
        }
        log::info!(
            "Injection boot complete after {} frames (sleeping={})",
            frames,
            self.machine.is_sleeping()
        );
    }

    /// Inject `data` into the machine's NOR/NAND flash as a file named `name`
    /// by driving the firmware's own GFFS write syscalls through the 0x3FFF
    /// pump protocol.
    ///
    /// This is an offline re-implementation of wangyu's `put` command: it
    /// boots the firmware, stages the file name / parameters and the pump
    /// routine in RAM, diverts the CPU to the pump at 0x3000, and feeds the
    /// file bytes through the magic 0x3FFF IO port until the firmware commits
    /// the file. Afterward it performs a clean reboot and re-mounts the file
    /// system to confirm the firmware actually recognises the new file.
    ///
    /// The caller is responsible for persisting the modified flash with
    /// [`Emulator::save_nor`] (or the NAND equivalent); this method only
    /// mutates the in-memory flash image so that `tmp/roms` stays untouched.
    pub fn inject_file(&mut self, name: &str, data: &[u8]) -> Result<InjectReport> {
        let Some(cfg) = inject_config(self.model()) else {
            anyhow::bail!(
                "Model `{}` does not yet support firmware-driven flash injection",
                self.model().name()
            );
        };
        if data.len() > 0xFFFF {
            anyhow::bail!(
                "File `{}` is too large ({} bytes); GFFS entries are limited to 65535 bytes",
                name,
                data.len()
            );
        }

        // 1. Make sure the firmware (BRK dispatcher + GFFS) is up.
        self.boot_for_injection();

        // 2. Stage the 16-byte, space-padded file name.
        let mut padded = [b' '; 16];
        let name_bytes = name.as_bytes();
        let n = name_bytes.len().min(16);
        padded[..n].copy_from_slice(&name_bytes[..n]);
        for (i, &b) in padded.iter().enumerate() {
            self.machine.poke(cfg.name_addr + i as u16, b);
        }

        // 3. Stage the model-specific parameter bytes.
        for &(addr, val) in &cfg.params {
            self.machine.poke(addr, val);
        }

        // 4. Warm-reset flag: the pump's final `BRK 0x01 0xC0` reboots the
        //    device; with this flag set the reboot is a quick warm boot that
        //    re-mounts (rather than reformats) the file system.
        self.machine.poke(0x02, 1);
        self.machine.poke(0x03, 1);

        // 5. Copy the pump routine to 0x3000. `WQX_SIG` lets us override the
        //    BRK signature byte (0x93) to probe which signature a firmware
        //    build expects.
        let mut routine: Vec<u8> = cfg.routine.to_vec();
        if let Ok(sig) = std::env::var("WQX_SIG") {
            if let Ok(value) = u8::from_str_radix(sig.trim_start_matches("0x"), 16) {
                for b in routine.iter_mut() {
                    if *b == 0x93 {
                        *b = value;
                    }
                }
            }
        }
        // WQX_WID overrides the "write block" BRK id (0x05 in wangyu's
        // nc1020 routine) so we can probe a firmware's id semantics.
        if let Ok(wid) = std::env::var("WQX_WID") {
            if let Ok(value) = u8::from_str_radix(wid.trim_start_matches("0x"), 16) {
                for i in 0..routine.len().saturating_sub(2) {
                    if routine[i] == 0x00 && routine[i + 2] == 0x93 && routine[i + 1] == 0x05 {
                        routine[i + 1] = value;
                    }
                }
            }
        }
        // WQX_CID overrides the "commit" BRK id (0x07).
        if let Ok(cid) = std::env::var("WQX_CID") {
            if let Ok(value) = u8::from_str_radix(cid.trim_start_matches("0x"), 16) {
                for i in 0..routine.len().saturating_sub(2) {
                    if routine[i] == 0x00 && routine[i + 2] == 0x93 && routine[i + 1] == 0x07 {
                        routine[i + 1] = value;
                    }
                }
            }
        }
        for (i, &b) in routine.iter().enumerate() {
            self.machine.poke(0x3000 + i as u16, b);
        }

        // 6. Arm the byte bridge and divert the CPU to the pump.
        self.machine.inject_activate(data.to_vec());
        self.cpu.pc = 0x3000;

        if std::env::var("WQX_TRACE").is_ok() {
            let mut trace: Vec<u16> = Vec::new();
            for _ in 0..4000u32 {
                trace.push(self.cpu.pc);
                self.machine.step(&mut self.cpu);
                if !self.machine.inject_active() {
                    break;
                }
            }
            let hits = |t: u16| trace.contains(&t);
            let max = trace.iter().copied().max().unwrap_or(0);
            log::info!(
                "TRACE n={} E999={} E8E1={} E768={} max=0x{:04X}",
                trace.len(),
                hits(0xE999),
                hits(0xE8E1),
                hits(0xE768),
                max
            );
            log::info!("TRACE seq: {:04X?}", &trace[..trace.len().min(160)]);
        }

        // 7. Pump until the bridge reports completion. The bridge deactivates
        //    itself once the last byte has been delivered and the firmware
        //    reads the terminating 0.
        const STEP_CYCLES: u64 = 2_000;
        const MAX_STEPS: u64 = 2_000_000 / STEP_CYCLES; // ~2,000,000 cycles cap
        let mut guard = 0u64;
        while self.machine.inject_active() && guard < MAX_STEPS {
            self.run_cycles(STEP_CYCLES);
            guard += 1;
        }
        if self.machine.inject_active() {
            self.machine.inject_activate(Vec::new()); // force deactivate
            anyhow::bail!(
                "Injection of `{}` timed out (firmware did not finish pumping)",
                name
            );
        }

        // 8. Let the commit syscall (BRK 0x07 ...) finish writing to flash.
        //    The following `BRK 0x01 0xC0` begins a warm reset; bound it so we
        //    don't emulate a whole reboot.
        self.run_cycles(300_000);

        let bytes_sent = self.machine.inject_bytes_sent();

        // 9. Validate end-to-end: clean reboot and re-mount the file system,
        //    then check whether the firmware now lists our file. `gffs_entries`
        //    reads the on-disk directory directly, which is the ground truth
        //    for what was persisted.
        self.reset();
        self.boot_for_injection();
        let entries = self.machine.gffs_entries();
        let recognized = entries
            .iter()
            .any(|e| e.name.eq_ignore_ascii_case(name.trim()));
        let (wc, po, eo) = self.machine.flash_stats();
        log::info!(
            "inject_file: name={} bytes_sent={} recognized={} entries={} flash(w={} prog={} erase={})",
            name,
            bytes_sent,
            recognized,
            entries.len(),
            wc,
            po,
            eo
        );

        Ok(InjectReport {
            name: name.to_string(),
            bytes_sent,
            file_size: data.len(),
            recognized,
            entries,
        })
    }

    /// List the user-file directory entries currently persisted in flash.
    pub fn gffs_list(&self) -> Vec<GffsEntry> {
        self.machine.gffs_entries()
    }

    /// Read a window of the on-disk NOR image (logical, already un-swapped).
    /// Debug helper for tracing flash mutations.
    pub fn debug_nor_bytes(&self, off: usize, len: usize) -> Vec<u8> {
        self.machine.debug_nor_window(off, len)
    }

    /// Debug helper: fill the whole NOR image with `val` (e.g. 0xFF for a
    /// blank/unformatted flash).
    pub fn debug_fill_nor(&mut self, val: u8) {
        self.machine.debug_fill_nor(val);
    }

    /// Diagnostics: `(write_calls, program_ops, erase_ops)` for the NOR flash.
    pub fn flash_stats(&self) -> (u32, u32, u32) {
        self.machine.flash_stats()
    }

    /// Debug helper: step the CPU instruction-by-instruction for `cycles`,
    /// recording every BRK syscall dispatch the firmware itself performs
    /// (PC hits the obj_lu dispatcher 0xE8E1). Returns distinct
    /// `(id, sig, handler_ptr)` tuples in first-seen order.
    pub fn trace_syscalls(&mut self, cycles: u64) -> Vec<(u8, u8, u16)> {
        let mut seen: Vec<(u8, u8, u16)> = Vec::new();
        let mut last: Option<(u8, u8)> = None;
        let mut remaining = cycles;
        while remaining > 0 {
            let pc = self.cpu.pc;
            if pc == 0xE8E1 {
                // Dispatcher entry: 0xB0 = id, 0x046D = signature.
                last = Some((self.machine.peek(0xB0), self.machine.peek(0x046D)));
            } else if pc == 0xE996 {
                // `jmp (0xB0)`: 0xB0/B1 now hold the final handler pointer.
                if let Some((id, sig)) = last {
                    let ptr = self.machine.peek_u16(0xB0);
                    if !seen.contains(&(id, sig, ptr)) {
                        seen.push((id, sig, ptr));
                    }
                }
            }
            remaining = remaining.saturating_sub(self.machine.step(&mut self.cpu));
        }
        seen
    }

    /// Trace syscalls across a scripted boot + key sequence, returning the
    /// distinct `(id, sig, handler)` tuples issued while the keys run.
    pub fn trace_key_session(
        &mut self,
        boot_frames: u32,
        keys: &[u8],
        post_frames: u32,
    ) -> Vec<(u8, u8, u16)> {
        let mut seen: Vec<(u8, u8, u16)> = Vec::new();
        let mut last: Option<(u8, u8)> = None;
        for _ in 0..boot_frames {
            self.step_frame_traced(&mut seen, &mut last);
        }
        for &key in keys {
            self.machine.set_key(key, true);
            for _ in 0..8 {
                self.step_frame_traced(&mut seen, &mut last);
            }
            self.machine.set_key(key, false);
            for _ in 0..12 {
                self.step_frame_traced(&mut seen, &mut last);
            }
        }
        for _ in 0..post_frames {
            self.step_frame_traced(&mut seen, &mut last);
        }
        seen
    }

    fn step_frame_traced(&mut self, seen: &mut Vec<(u8, u8, u16)>, last: &mut Option<(u8, u8)>) {
        let target = self.machine.cycles_per_frame();
        let mut cycles = 0u64;
        while cycles < target {
            let pc = self.cpu.pc;
            if pc == 0xE8E1 {
                *last = Some((self.machine.peek(0xB0), self.machine.peek(0x046D)));
            } else if pc == 0xE996 {
                if let Some((id, sig)) = *last {
                    let ptr = self.machine.peek_u16(0xB0);
                    if !seen.contains(&(id, sig, ptr)) {
                        seen.push((id, sig, ptr));
                    }
                }
            }
            cycles += self.machine.step(&mut self.cpu);
        }
        self.machine.end_of_frame(&mut self.cpu);
    }

    /// Debug helper: press `key_id` for `hold_frames`, then release with a gap.
    pub fn debug_press_key(&mut self, key_id: u8, hold_frames: u32) {
        self.machine.set_key(key_id, true);
        for _ in 0..hold_frames {
            self.run_frame();
        }
        self.machine.set_key(key_id, false);
        for _ in 0..12 {
            self.run_frame();
        }
    }

    /// Read a memory/IO address for debugging.
    pub fn peek(&self, addr: u16) -> u8 {
        self.machine.peek(addr)
    }

    /// Read a little-endian 16-bit word for debugging.
    pub fn peek_u16(&self, addr: u16) -> u16 {
        self.machine.peek_u16(addr)
    }

    /// Get the LCD framebuffer as XRGB8888 pixels.
    pub fn framebuffer(&self) -> Vec<u32> {
        self.machine.lcd().framebuffer_xrgb8888()
    }

    /// Get the LCD framebuffer as raw bytes.
    pub fn framebuffer_raw(&self) -> &[u8] {
        self.machine.lcd().framebuffer_raw()
    }

    /// Get the frame counter.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get the LCD width.
    pub fn lcd_width(&self) -> usize {
        LCD_WIDTH
    }

    /// Get the LCD height.
    pub fn lcd_height(&self) -> usize {
        LCD_HEIGHT
    }

    /// Get the sample rate for audio.
    pub fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    /// Get the machine's host video refresh rate.
    pub fn frame_rate(&self) -> u32 {
        self.machine.frame_rate()
    }

    /// Drain audio samples.
    pub fn drain_audio(&mut self, output: &mut Vec<i16>) {
        self.machine.drain_audio(output);
    }

    /// Check if the emulator is in sleep mode.
    pub fn is_sleeping(&self) -> bool {
        self.machine.is_sleeping()
    }

    /// Get current CPU PC.
    pub fn pc(&self) -> u16 {
        self.cpu.pc
    }

    /// Get the active machine model.
    pub fn model(&self) -> MachineModel {
        self.machine.model()
    }

    /// Create a save state.
    pub fn save_state(&self) -> SaveState {
        self.machine.save_state(&self.cpu)
    }

    /// Load a save state.
    pub fn load_state(&mut self, state: &SaveState) -> Result<()> {
        self.machine.load_state(&mut self.cpu, state);
        Ok(())
    }

    /// Serialize a complete cross-process session.
    pub fn save_persistent_state(&self) -> Result<Vec<u8>> {
        let state = PersistentState {
            version: PERSISTENT_STATE_VERSION,
            model: self.model(),
            machine_identity: self.machine.persistent_state_identity(),
            cpu: self.cpu.clone(),
            frame_count: self.frame_count,
            speed_up: self.speed_up,
            machine: self.machine.save_persistent_state()?,
        };
        bincode::serialize(&state).context("Failed to serialize persistent state")
    }

    /// Restore a complete cross-process session.
    pub fn load_persistent_state(&mut self, data: &[u8]) -> Result<()> {
        let state: PersistentState =
            bincode::deserialize(data).context("Failed to deserialize persistent state")?;
        if state.version != PERSISTENT_STATE_VERSION {
            anyhow::bail!(
                "Persistent state version mismatch: expected {}, got {}",
                PERSISTENT_STATE_VERSION,
                state.version
            );
        }
        if state.model != self.model() {
            anyhow::bail!(
                "Persistent state model mismatch: expected {}, got {}",
                self.model().name(),
                state.model.name()
            );
        }
        let machine_identity = self.machine.persistent_state_identity();
        if state.machine_identity != machine_identity {
            anyhow::bail!("Persistent state firmware mismatch");
        }

        self.machine.load_persistent_state(&state.machine)?;
        self.cpu = state.cpu;
        self.frame_count = state.frame_count;
        self.speed_up = state.speed_up;
        self.machine.set_speed_up(state.speed_up);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistent_state_round_trips_without_rom_or_flash_paths() {
        let mut emulator = Emulator::new(MachineModel::Nc1020, &RomFiles::default()).unwrap();
        emulator.cpu.pc = 0x1234;
        emulator.frame_count = 42;

        let state = emulator.save_persistent_state().unwrap();
        emulator.cpu.pc = 0x5678;
        emulator.frame_count = 99;
        emulator.load_persistent_state(&state).unwrap();

        assert_eq!(emulator.pc(), 0x1234);
        assert_eq!(emulator.frame_count(), 42);
    }

    #[test]
    fn persistent_state_rejects_a_different_machine_model() {
        let source = Emulator::new(MachineModel::Nc1020, &RomFiles::default()).unwrap();
        let state = source.save_persistent_state().unwrap();

        let mut target = Emulator::new(MachineModel::Pc1000, &RomFiles::default()).unwrap();
        let error = target.load_persistent_state(&state).unwrap_err();
        assert!(error.to_string().contains("model mismatch"));
    }
}
