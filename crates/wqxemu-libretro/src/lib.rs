// WQXEmu libretro core - RetroArch integration for Wenquxing emulators.
//
// This crate implements the libretro C API, allowing WQXEmu to be loaded
// as a core in RetroArch. It wraps the platform-independent wqxemu-core
// emulator and bridges it to the libretro callbacks.

#![allow(clippy::upper_case_acronyms)]
#![allow(static_mut_refs)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(private_interfaces)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::manual_range_contains)]

use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::panic;
use std::path::{Path, PathBuf};
use std::ptr;

use wqxemu_core::{key_id_for, layout_for, Emulator, MachineModel, LCD_HEIGHT, LCD_WIDTH};

// ============================================================
// libretro constants
// ============================================================

const RETRO_API_VERSION: u32 = 1;
const RETRO_REGION_NTSC: u32 = 0;

// Environment commands
const RETRO_ENVIRONMENT_SET_PIXEL_FORMAT: u32 = 1;
const RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS: u32 = 11;
const RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK: u32 = 12;
const RETRO_ENVIRONMENT_SET_VARIABLES: u32 = 16;
const RETRO_ENVIRONMENT_GET_VARIABLE: u32 = 17;
const RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE: u32 = 18;
const RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY: u32 = 9;

// Pixel format
const RETRO_PIXEL_FORMAT_XRGB8888: u32 = 0;
const RETRO_PIXEL_FORMAT_RGB565: u32 = 1;

// Memory types
const RETRO_MEMORY_SAVE_RAM: u32 = 0;
const RETRO_MEMORY_SYSTEM_RAM: u32 = 2;

const SERIALIZATION_BUFFER_SIZE: usize = 1024 * 1024;
const SERIALIZATION_HEADER_SIZE: usize = 12;
const SERIALIZATION_MAGIC: &[u8; 4] = b"WQXS";
const SERIALIZATION_FORMAT_VERSION: u8 = 1;

// Joypad constants
const RETRO_DEVICE_JOYPAD: u32 = 1;
const RETRO_DEVICE_ID_JOYPAD_B: u32 = 0;
const RETRO_DEVICE_ID_JOYPAD_Y: u32 = 1;
const RETRO_DEVICE_ID_JOYPAD_SELECT: u32 = 2;
const RETRO_DEVICE_ID_JOYPAD_START: u32 = 3;
const RETRO_DEVICE_ID_JOYPAD_UP: u32 = 4;
const RETRO_DEVICE_ID_JOYPAD_DOWN: u32 = 5;
const RETRO_DEVICE_ID_JOYPAD_LEFT: u32 = 6;
const RETRO_DEVICE_ID_JOYPAD_RIGHT: u32 = 7;
const RETRO_DEVICE_ID_JOYPAD_A: u32 = 8;
const RETRO_DEVICE_ID_JOYPAD_X: u32 = 9;
const RETRO_DEVICE_ID_JOYPAD_L: u32 = 10;
const RETRO_DEVICE_ID_JOYPAD_R: u32 = 11;

// Keyboard constants
const RETROK_RETURN: u32 = 13;
const RETROK_ESCAPE: u32 = 27;
const RETROK_SPACE: u32 = 32;
const RETROK_LEFT: u32 = 0x250;
const RETROK_UP: u32 = 0x251;
const RETROK_RIGHT: u32 = 0x252;
const RETROK_DOWN: u32 = 0x253;
const RETROK_A: u32 = 97;
const RETROK_Z: u32 = 122;
const RETROK_0: u32 = 48;
const RETROK_9: u32 = 57;
const RETROK_F1: u32 = 282;
const RETROK_F12: u32 = 293;
const RETROK_BACKSPACE: u32 = 8;
const RETROK_DELETE: u32 = 127;
const RETROK_PAGEUP: u32 = 0x254;
const RETROK_PAGEDOWN: u32 = 0x255;

// ============================================================
// libretro types
// ============================================================

type RetroEnvironmentT = Option<unsafe extern "C" fn(cmd: u32, data: *mut c_void) -> bool>;
type RetroVideoRefreshT =
    Option<unsafe extern "C" fn(data: *const c_void, width: u32, height: u32, pitch: usize)>;
type RetroAudioSampleT = Option<unsafe extern "C" fn(left: i16, right: i16)>;
type RetroAudioSampleBatchT =
    Option<unsafe extern "C" fn(data: *const i16, frames: usize) -> usize>;
type RetroInputPollT = Option<unsafe extern "C" fn()>;
type RetroInputStateT =
    Option<unsafe extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16>;
type RetroKeyboardEventT =
    Option<unsafe extern "C" fn(down: bool, keycode: u32, character: u32, key_modifiers: u16)>;

#[repr(C)]
struct RetroKeyboardCallback {
    callback: RetroKeyboardEventT,
}

#[repr(C)]
struct RetroSystemInfo {
    library_name: *const c_char,
    library_version: *const c_char,
    valid_extensions: *const c_char,
    need_fullpath: bool,
    block_extract: bool,
}

#[repr(C)]
struct RetroGameGeometry {
    base_width: u32,
    base_height: u32,
    max_width: u32,
    max_height: u32,
    aspect_ratio: f32,
}

#[repr(C)]
struct RetroSystemTiming {
    fps: f64,
    sample_rate: f64,
}

#[repr(C)]
struct RetroSystemAvInfo {
    geometry: RetroGameGeometry,
    timing: RetroSystemTiming,
}

#[repr(C)]
struct RetroGameInfo {
    path: *const c_char,
    data: *const c_void,
    size: usize,
    meta: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RetroVariable {
    key: *const c_char,
    value: *const c_char,
}

#[repr(C)]
struct RetroInputDescriptor {
    port: u32,
    device: u32,
    index: u32,
    id: u32,
    description: *const c_char,
}

// ============================================================
// Global state
// ============================================================

static mut EMULATOR: Option<Emulator> = None;
static mut ENV_CB: RetroEnvironmentT = None;
static mut VIDEO_CB: RetroVideoRefreshT = None;
static mut AUDIO_CB: RetroAudioSampleT = None;
static mut AUDIO_BATCH_CB: RetroAudioSampleBatchT = None;
static mut INPUT_POLL_CB: RetroInputPollT = None;
static mut INPUT_STATE_CB: RetroInputStateT = None;
static mut SYSTEM_DIR: Option<String> = None;

// ============================================================
// Helper functions
// ============================================================

unsafe fn get_emulator() -> &'static Emulator {
    EMULATOR.as_ref().expect("Emulator not initialized")
}

unsafe fn get_emulator_mut() -> &'static mut Emulator {
    EMULATOR.as_mut().expect("Emulator not initialized")
}

unsafe fn environment(cmd: u32, data: *mut c_void) -> bool {
    ENV_CB.map(|cb| cb(cmd, data)).unwrap_or(false)
}

fn extension_matches(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|expected| extension.eq_ignore_ascii_case(expected))
        })
}

fn find_companion(parent: &Path, stem: Option<&str>, extensions: &[&str]) -> Option<PathBuf> {
    let entries = std::fs::read_dir(parent).ok()?;
    let candidates: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && extension_matches(path, extensions))
        .collect();

    if let Some(stem) = stem {
        if let Some(path) = candidates.iter().find(|path| {
            path.file_stem()
                .and_then(|candidate| candidate.to_str())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(stem))
        }) {
            return Some(path.clone());
        }
    }

    (candidates.len() == 1).then(|| candidates[0].clone())
}

fn assemble_firmware_files(game_path: &Path) -> wqxemu_core::RomFiles {
    let parent = game_path.parent().unwrap_or_else(|| Path::new(""));
    let stem = game_path.file_stem().and_then(|stem| stem.to_str());
    let extension = game_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    let mut files = wqxemu_core::RomFiles::new(None, None, None, None);
    match extension.as_deref() {
        Some("nand") => files.nand = Some(game_path.to_path_buf()),
        Some("nand0") => files.nand0 = Some(game_path.to_path_buf()),
        Some("fls") | Some("nor") => files.nor = Some(game_path.to_path_buf()),
        _ => files.rom = Some(game_path.to_path_buf()),
    }

    if files.rom.is_none() {
        files.rom = find_companion(parent, stem, &["bin", "rom"]);
    }
    if files.nor.is_none() {
        files.nor = find_companion(parent, stem, &["fls", "nor"]);
    }
    if files.nand.is_none() {
        files.nand = find_companion(parent, stem, &["nand"]);
    }
    if files.nand0.is_none() {
        files.nand0 = find_companion(parent, stem, &["nand0"]);
    }

    files
}

fn machine_model_tag(model: MachineModel) -> u8 {
    match model {
        MachineModel::Nc1020 => 0,
        MachineModel::Pc1000 => 1,
        MachineModel::Cc800 => 2,
        MachineModel::Nc2000 => 3,
        MachineModel::Nc3000 => 4,
    }
}

fn write_serialized_state(model: MachineModel, payload: &[u8], output: &mut [u8]) -> bool {
    let Ok(payload_size) = u32::try_from(payload.len()) else {
        return false;
    };
    let Some(required_size) = SERIALIZATION_HEADER_SIZE.checked_add(payload.len()) else {
        return false;
    };
    if required_size > output.len() {
        return false;
    }

    output.fill(0);
    output[..4].copy_from_slice(SERIALIZATION_MAGIC);
    output[4] = SERIALIZATION_FORMAT_VERSION;
    output[5] = machine_model_tag(model);
    output[8..SERIALIZATION_HEADER_SIZE].copy_from_slice(&payload_size.to_le_bytes());
    output[SERIALIZATION_HEADER_SIZE..required_size].copy_from_slice(payload);
    true
}

fn serialized_state_payload(model: MachineModel, data: &[u8]) -> Option<&[u8]> {
    if data.len() < SERIALIZATION_HEADER_SIZE
        || &data[..4] != SERIALIZATION_MAGIC
        || data[4] != SERIALIZATION_FORMAT_VERSION
        || data[5] != machine_model_tag(model)
    {
        return None;
    }

    let payload_size = u32::from_le_bytes(data[8..SERIALIZATION_HEADER_SIZE].try_into().ok()?);
    let payload_size = usize::try_from(payload_size).ok()?;
    let end = SERIALIZATION_HEADER_SIZE.checked_add(payload_size)?;
    data.get(SERIALIZATION_HEADER_SIZE..end)
}

fn key_id_for_token(model: MachineModel, token: &str) -> Option<u8> {
    layout_for(model)
        .iter()
        .find(|key| {
            key.label
                .split('/')
                .chain(key.hint.split('/'))
                .any(|alias| alias == token)
        })
        .map(|key| key_id_for(model, key.row, key.col))
}

/// Map a RetroArch keyboard keycode to the active model's keypad matrix.
fn map_keyboard_key(model: MachineModel, keycode: u32) -> Option<u8> {
    let token = match keycode {
        RETROK_RETURN => "ENT".to_owned(),
        RETROK_ESCAPE => "ESC".to_owned(),
        RETROK_SPACE => "SPC".to_owned(),
        RETROK_BACKSPACE => "F2".to_owned(),
        RETROK_UP => "UP".to_owned(),
        RETROK_DOWN => "DN".to_owned(),
        RETROK_LEFT => "LT".to_owned(),
        RETROK_RIGHT => "RT".to_owned(),
        RETROK_PAGEUP => "PGUP".to_owned(),
        RETROK_PAGEDOWN => "PGDN".to_owned(),
        RETROK_DELETE if model == MachineModel::Nc1020 => "DEL".to_owned(),
        RETROK_DELETE => "F12".to_owned(),
        key if (RETROK_F1..=RETROK_F12).contains(&key) => {
            format!("F{}", key - RETROK_F1 + 1)
        }
        key if (RETROK_A..=RETROK_Z).contains(&key) => char::from_u32(key)
            .unwrap()
            .to_ascii_uppercase()
            .to_string(),
        key if (RETROK_0..=RETROK_9).contains(&key) => char::from_u32(key).unwrap().to_string(),
        _ => return None,
    };

    key_id_for_token(model, &token)
}

/// Map a RetroPad button to the active model's keypad matrix.
fn map_joypad_button(model: MachineModel, button: u32) -> Option<u8> {
    let token = match button {
        RETRO_DEVICE_ID_JOYPAD_UP => "UP",
        RETRO_DEVICE_ID_JOYPAD_DOWN => "DN",
        RETRO_DEVICE_ID_JOYPAD_LEFT => "LT",
        RETRO_DEVICE_ID_JOYPAD_RIGHT => "RT",
        RETRO_DEVICE_ID_JOYPAD_A => "ENT",
        RETRO_DEVICE_ID_JOYPAD_B => "ESC",
        RETRO_DEVICE_ID_JOYPAD_X => "F1",
        RETRO_DEVICE_ID_JOYPAD_Y => "F4",
        RETRO_DEVICE_ID_JOYPAD_L => "PGUP",
        RETRO_DEVICE_ID_JOYPAD_R => "PGDN",
        RETRO_DEVICE_ID_JOYPAD_START => "F10",
        RETRO_DEVICE_ID_JOYPAD_SELECT => "F11",
        _ => return None,
    };

    key_id_for_token(model, token)
}

unsafe extern "C" fn keyboard_event(
    down: bool,
    keycode: u32,
    _character: u32,
    _key_modifiers: u16,
) {
    if let Some(emulator) = EMULATOR.as_mut() {
        if let Some(key_id) = map_keyboard_key(emulator.model(), keycode) {
            emulator.set_key(key_id, down);
        }
    }
}

// ============================================================
// libretro API implementation
// ============================================================

/// Set environment callback
#[no_mangle]
pub extern "C" fn retro_set_environment(cb: RetroEnvironmentT) {
    unsafe {
        ENV_CB = cb;
    }
    // Set input descriptors
    set_input_descriptors();
    // Set core variables
    set_core_variables();
    let mut keyboard_callback = RetroKeyboardCallback {
        callback: Some(keyboard_event),
    };
    unsafe {
        environment(
            RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK,
            &mut keyboard_callback as *mut RetroKeyboardCallback as *mut c_void,
        );
    }
}

/// Set video refresh callback
#[no_mangle]
pub extern "C" fn retro_set_video_refresh(cb: RetroVideoRefreshT) {
    unsafe {
        VIDEO_CB = cb;
    }
}

/// Set audio sample callback
#[no_mangle]
pub extern "C" fn retro_set_audio_sample(cb: RetroAudioSampleT) {
    unsafe {
        AUDIO_CB = cb;
    }
}

/// Set audio sample batch callback
#[no_mangle]
pub extern "C" fn retro_set_audio_sample_batch(cb: RetroAudioSampleBatchT) {
    unsafe {
        AUDIO_BATCH_CB = cb;
    }
}

/// Set input poll callback
#[no_mangle]
pub extern "C" fn retro_set_input_poll(cb: RetroInputPollT) {
    unsafe {
        INPUT_POLL_CB = cb;
    }
}

/// Set input state callback
#[no_mangle]
pub extern "C" fn retro_set_input_state(cb: RetroInputStateT) {
    unsafe {
        INPUT_STATE_CB = cb;
    }
}

/// Return API version
#[no_mangle]
pub extern "C" fn retro_api_version() -> u32 {
    RETRO_API_VERSION
}

/// Initialize the core
#[no_mangle]
pub extern "C" fn retro_init() {
    unsafe {
        // Get system directory
        let mut sys_dir: *const c_char = ptr::null();
        if environment(
            RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY,
            &mut sys_dir as *mut *const c_char as *mut c_void,
        ) && !sys_dir.is_null()
        {
            SYSTEM_DIR = Some(CStr::from_ptr(sys_dir).to_string_lossy().into_owned());
        }

        // Set pixel format to XRGB8888
        let mut pixel_format = RETRO_PIXEL_FORMAT_XRGB8888;
        environment(
            RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,
            &mut pixel_format as *mut u32 as *mut c_void,
        );
    }
    log::info!("WQXEmu libretro core initialized");
}

/// Deinitialize the core
#[no_mangle]
pub extern "C" fn retro_deinit() {
    unsafe {
        EMULATOR = None;
    }
    log::info!("WQXEmu libretro core deinitialized");
}

/// Get system information
#[no_mangle]
pub extern "C" fn retro_get_system_info(info: *mut RetroSystemInfo) {
    unsafe {
        (*info) = RetroSystemInfo {
            library_name: c"WQXEmu".as_ptr(),
            library_version: c"0.1.0".as_ptr(),
            valid_extensions: c"bin|fls|rom|nor|nand|nand0".as_ptr(),
            need_fullpath: true,
            block_extract: true,
        };
    }
}

/// Get system AV information
#[no_mangle]
pub extern "C" fn retro_get_system_av_info(info: *mut RetroSystemAvInfo) {
    unsafe {
        let fps = EMULATOR
            .as_ref()
            .map(|emulator| emulator.frame_rate() as f64)
            .unwrap_or(30.0);
        (*info) = RetroSystemAvInfo {
            geometry: RetroGameGeometry {
                base_width: LCD_WIDTH as u32,
                base_height: LCD_HEIGHT as u32,
                max_width: LCD_WIDTH as u32,
                max_height: LCD_HEIGHT as u32,
                aspect_ratio: LCD_WIDTH as f32 / LCD_HEIGHT as f32,
            },
            timing: RetroSystemTiming {
                fps,
                sample_rate: 44100.0,
            },
        };
    }
}

/// Set controller port device
#[no_mangle]
pub extern "C" fn retro_set_controller_port_device(_port: u32, _device: u32) {
    // NC1020 only supports basic input
}

/// Load a game
#[no_mangle]
pub extern "C" fn retro_load_game(info: *const RetroGameInfo) -> bool {
    unsafe {
        let game_info = &*info;

        // Check if path is valid
        if game_info.path.is_null() {
            log::error!("Game path is null");
            return false;
        }

        let path = match CStr::from_ptr(game_info.path).to_str() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Invalid game path: {}", e);
                return false;
            }
        };

        let game_path = Path::new(path);
        let files = assemble_firmware_files(game_path);

        let model = wqxemu_core::detect_model(&files);
        log::info!("Detected model: {}", model.name());

        let mut emu = match Emulator::new(model, &files) {
            Ok(e) => e,
            Err(e) => {
                log::error!("Failed to create emulator: {}", e);
                return false;
            }
        };

        emu.reset();
        EMULATOR = Some(emu);

        log::info!("Game loaded: {}", path);
        true
    }
}

/// Unload a game
#[no_mangle]
pub extern "C" fn retro_unload_game() {
    unsafe {
        EMULATOR = None;
    }
}

/// Reset active cheat codes.
#[no_mangle]
pub extern "C" fn retro_cheat_reset() {
    // Cheats are not supported by this core.
}

/// Add or update a cheat code.
#[no_mangle]
pub extern "C" fn retro_cheat_set(_index: u32, _enabled: bool, _code: *const c_char) {
    // Cheats are not supported by this core.
}

/// Load content through a libretro subsystem.
#[no_mangle]
pub extern "C" fn retro_load_game_special(
    _game_type: u32,
    _info: *const RetroGameInfo,
    _num_info: usize,
) -> bool {
    false
}

/// Run one frame
#[no_mangle]
pub extern "C" fn retro_run() {
    unsafe {
        // Poll input
        if let Some(poll) = INPUT_POLL_CB {
            poll();
        }

        // Process joypad input
        if let (Some(get_state), Some(emu)) = (INPUT_STATE_CB, EMULATOR.as_mut()) {
            // Check all joypad buttons
            for button in 0..12 {
                let state = get_state(0, RETRO_DEVICE_JOYPAD, 0, button);
                if let Some(key_id) = map_joypad_button(emu.model(), button) {
                    emu.set_key(key_id, state != 0);
                }
            }
        }

        // Run one frame
        if let Some(ref mut emu) = EMULATOR {
            emu.run_frame();

            // Video output
            if let Some(video_cb) = VIDEO_CB {
                let pixels = emu.framebuffer();
                video_cb(
                    pixels.as_ptr() as *const c_void,
                    LCD_WIDTH as u32,
                    LCD_HEIGHT as u32,
                    LCD_WIDTH * 4, // pitch in bytes
                );
            }

            // Audio output
            if let Some(audio_batch_cb) = AUDIO_BATCH_CB {
                let mut audio_samples = Vec::new();
                emu.drain_audio(&mut audio_samples);
                if !audio_samples.is_empty() {
                    audio_batch_cb(audio_samples.as_ptr(), audio_samples.len() / 2);
                }
            }
        }
    }
}

/// Serialize save state
#[no_mangle]
pub extern "C" fn retro_serialize(data: *mut c_void, size: usize) -> bool {
    unsafe {
        if data.is_null() {
            return false;
        }
        let emu = match EMULATOR.as_ref() {
            Some(e) => e,
            None => return false,
        };

        let state = emu.save_state();
        let bytes = match state.serialize() {
            Ok(b) => b,
            Err(e) => {
                log::error!("Failed to serialize: {}", e);
                return false;
            }
        };

        let output = std::slice::from_raw_parts_mut(data as *mut u8, size);
        if !write_serialized_state(emu.model(), &bytes, output) {
            log::error!(
                "Save state too large: {} payload bytes, {} total bytes available",
                bytes.len(),
                size
            );
            return false;
        }
        true
    }
}

/// Deserialize save state
#[no_mangle]
pub extern "C" fn retro_unserialize(data: *const c_void, size: usize) -> bool {
    unsafe {
        if data.is_null() {
            return false;
        }
        let emu = match EMULATOR.as_mut() {
            Some(e) => e,
            None => return false,
        };

        let bytes = std::slice::from_raw_parts(data as *const u8, size);
        let Some(payload) = serialized_state_payload(emu.model(), bytes) else {
            log::error!("Invalid save state envelope");
            return false;
        };
        let state = match wqxemu_core::save::SaveState::deserialize(payload) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to deserialize: {}", e);
                return false;
            }
        };

        if let Err(e) = emu.load_state(&state) {
            log::error!("Failed to load state: {}", e);
            return false;
        }

        true
    }
}

/// Get save state size
#[no_mangle]
pub extern "C" fn retro_serialize_size() -> usize {
    SERIALIZATION_BUFFER_SIZE
}

/// Get memory data
#[no_mangle]
pub extern "C" fn retro_get_memory_data(id: u32) -> *mut c_void {
    let _ = id;
    ptr::null_mut()
}

/// Get memory size
#[no_mangle]
pub extern "C" fn retro_get_memory_size(id: u32) -> usize {
    let _ = id;
    0
}

/// Reset the core
#[no_mangle]
pub extern "C" fn retro_reset() {
    unsafe {
        if let Some(ref mut emu) = EMULATOR {
            emu.reset();
        }
    }
}

/// Get region
#[no_mangle]
pub extern "C" fn retro_get_region() -> u32 {
    RETRO_REGION_NTSC
}

// ============================================================
// Helper functions for setup
// ============================================================

/// Set input descriptors for RetroArch
fn set_input_descriptors() {
    // Input descriptors are optional but help RetroArch show proper labels
    // We'll skip the full implementation for now
}

/// Set core variables for RetroArch
fn set_core_variables() {
    // Core variables allow users to configure the emulator through RetroArch UI
    // We'll skip the full implementation for now
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_directory(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "wqxemu-libretro-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn assembles_uniquely_named_sibling_firmware() {
        let directory = test_directory("siblings");
        fs::create_dir_all(&directory).unwrap();
        let rom = directory.join("obj_lu.bin");
        let nor = directory.join("nc1020.fls");
        fs::write(&rom, []).unwrap();
        fs::write(&nor, []).unwrap();

        let files = assemble_firmware_files(&rom);

        assert_eq!(files.rom.as_deref(), Some(rom.as_path()));
        assert_eq!(files.nor.as_deref(), Some(nor.as_path()));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn prefers_same_stem_when_multiple_companions_exist() {
        let directory = test_directory("same-stem");
        fs::create_dir_all(&directory).unwrap();
        let rom = directory.join("pc1000.rom");
        let nor = directory.join("pc1000.fls");
        fs::write(&rom, []).unwrap();
        fs::write(&nor, []).unwrap();
        fs::write(directory.join("backup.fls"), []).unwrap();

        let files = assemble_firmware_files(&rom);

        assert_eq!(files.nor.as_deref(), Some(nor.as_path()));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unavailable_memory_regions_report_zero_size() {
        for id in [RETRO_MEMORY_SAVE_RAM, RETRO_MEMORY_SYSTEM_RAM, u32::MAX] {
            assert!(retro_get_memory_data(id).is_null());
            assert_eq!(retro_get_memory_size(id), 0);
        }
    }

    #[test]
    fn maps_navigation_to_each_models_matrix() {
        let expectations = [
            (MachineModel::Nc1020, 0x1a),
            (MachineModel::Pc1000, 0x32),
            (MachineModel::Cc800, 0x32),
            (MachineModel::Nc2000, 0x13),
            (MachineModel::Nc3000, 0x13),
        ];

        for (model, expected) in expectations {
            assert_eq!(map_keyboard_key(model, RETROK_UP), Some(expected));
            assert_eq!(
                map_joypad_button(model, RETRO_DEVICE_ID_JOYPAD_UP),
                Some(expected)
            );
        }
    }

    #[test]
    fn maps_keyboard_aliases_and_model_specific_power_keys() {
        assert_eq!(
            map_keyboard_key(MachineModel::Nc1020, RETROK_0 + 1),
            Some(0x34)
        );
        assert_eq!(
            map_keyboard_key(MachineModel::Pc1000, RETROK_0 + 1),
            Some(0x1c)
        );
        assert_eq!(
            map_keyboard_key(MachineModel::Nc2000, RETROK_DELETE),
            Some(0x00)
        );
        assert_eq!(
            map_keyboard_key(MachineModel::Nc1020, RETROK_DELETE),
            Some(0x0f)
        );
    }

    #[test]
    fn serialization_envelope_ignores_fixed_buffer_padding() {
        let payload = b"serialized state";
        let mut buffer = vec![0xaa; 128];

        assert!(write_serialized_state(
            MachineModel::Pc1000,
            payload,
            &mut buffer
        ));
        assert_eq!(
            serialized_state_payload(MachineModel::Pc1000, &buffer),
            Some(payload.as_slice())
        );
        assert!(buffer[SERIALIZATION_HEADER_SIZE + payload.len()..]
            .iter()
            .all(|byte| *byte == 0));
    }

    #[test]
    fn serialization_envelope_rejects_invalid_lengths() {
        let mut buffer = [0u8; 16];
        assert!(!write_serialized_state(
            MachineModel::Nc1020,
            &[0u8; 9],
            &mut buffer
        ));
        assert_eq!(
            serialized_state_payload(MachineModel::Nc1020, &buffer),
            None
        );

        buffer[..4].copy_from_slice(SERIALIZATION_MAGIC);
        buffer[4] = SERIALIZATION_FORMAT_VERSION;
        buffer[5] = machine_model_tag(MachineModel::Nc1020);
        buffer[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            serialized_state_payload(MachineModel::Nc1020, &buffer),
            None
        );
    }

    #[test]
    fn serialization_envelope_round_trips_an_emulator_state() {
        let emulator =
            Emulator::new(MachineModel::Nc1020, &wqxemu_core::RomFiles::default()).unwrap();
        let bytes = emulator.save_state().serialize().unwrap();
        let mut buffer = vec![0u8; SERIALIZATION_BUFFER_SIZE];

        assert!(write_serialized_state(
            MachineModel::Nc1020,
            &bytes,
            &mut buffer
        ));
        let restored = wqxemu_core::save::SaveState::deserialize(
            serialized_state_payload(MachineModel::Nc1020, &buffer).unwrap(),
        )
        .unwrap();

        assert_eq!(restored.version, emulator.save_state().version);
    }

    #[test]
    fn serialization_envelope_rejects_another_model() {
        let mut buffer = [0u8; 32];
        assert!(write_serialized_state(
            MachineModel::Nc2000,
            b"state",
            &mut buffer
        ));

        assert_eq!(
            serialized_state_payload(MachineModel::Nc3000, &buffer),
            None
        );
    }
}
