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

use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use std::panic;
use std::path::{Path, PathBuf};
use std::ptr;

#[cfg(test)]
use wqxemu_core::key_id_for_host_key;
use wqxemu_core::save::{read_persistent_state_file, write_persistent_state_file};
use wqxemu_core::{Emulator, FrontendInputState, HostKey, MachineModel, LCD_HEIGHT, LCD_WIDTH};

// ============================================================
// libretro constants
// ============================================================

const RETRO_API_VERSION: u32 = 1;
const RETRO_REGION_NTSC: u32 = 0;

// Environment commands
const RETRO_ENVIRONMENT_SET_PIXEL_FORMAT: u32 = 10;
const RETRO_ENVIRONMENT_SET_MESSAGE: u32 = 6;
const RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY: u32 = 9;
const RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS: u32 = 11;
const RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK: u32 = 12;
const RETRO_ENVIRONMENT_GET_VARIABLE: u32 = 15;
const RETRO_ENVIRONMENT_SET_VARIABLES: u32 = 16;
const RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME: u32 = 18;
const RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY: u32 = 31;

// Pixel format
const RETRO_PIXEL_FORMAT_XRGB8888: u32 = 1;

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
const RETROK_COMMA: u32 = 44;
const RETROK_PERIOD: u32 = 46;
const RETROK_SLASH: u32 = 47;
const RETROK_EQUALS: u32 = 61;
const RETROK_LEFTBRACKET: u32 = 91;
const RETROK_BACKSLASH: u32 = 92;
const RETROK_RIGHTBRACKET: u32 = 93;
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

#[repr(C)]
struct RetroMessage {
    msg: *const c_char,
    frames: u32,
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
static mut SAVE_DIR: Option<String> = None;
static mut PERSISTENT_STATE_PATH: Option<PathBuf> = None;
static mut FRONTEND_INPUT: Option<FrontendInputState> = None;

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

fn firmware_files_for_model(system_dir: &Path, model: MachineModel) -> wqxemu_core::RomFiles {
    let model_dir = system_dir.join("WQXEmu").join(model.name());
    match model {
        MachineModel::Nc1020 => wqxemu_core::RomFiles::new(
            Some(model_dir.join("obj_lu.bin")),
            Some(model_dir.join("nc1020.fls")),
            None,
            None,
        ),
        MachineModel::Pc1000 => wqxemu_core::RomFiles::new(
            Some(model_dir.join("pc1000.rom")),
            Some(model_dir.join("pc1000.nor")),
            None,
            None,
        ),
        MachineModel::Cc800 => wqxemu_core::RomFiles::new(
            Some(model_dir.join("obj.bin")),
            Some(model_dir.join("cc800.fls")),
            None,
            None,
        ),
        MachineModel::Nc2000 => wqxemu_core::RomFiles::new(
            None,
            Some(model_dir.join("nc2000.nor")),
            Some(model_dir.join("nc2000.nand")),
            Some(model_dir.join("nc2000.nand0")),
        ),
        MachineModel::Nc3000 => {
            let nand0 = model_dir.join("nc3000.nand0");
            wqxemu_core::RomFiles::new(
                None,
                Some(model_dir.join("nc3000.nor")),
                Some(model_dir.join("nc3000.nand")),
                nand0.is_file().then_some(nand0),
            )
        }
    }
}

fn missing_required_firmware(files: &wqxemu_core::RomFiles) -> Vec<&Path> {
    [&files.rom, &files.nor, &files.nand, &files.nand0]
        .into_iter()
        .flatten()
        .map(PathBuf::as_path)
        .filter(|path| !path.is_file())
        .collect()
}

fn model_from_option(value: &str) -> Option<MachineModel> {
    MachineModel::from_name(value)
}

unsafe fn selected_model() -> MachineModel {
    let mut variable = RetroVariable {
        key: c"wqxemu_model".as_ptr(),
        value: ptr::null(),
    };
    if environment(
        RETRO_ENVIRONMENT_GET_VARIABLE,
        &mut variable as *mut RetroVariable as *mut c_void,
    ) && !variable.value.is_null()
    {
        if let Some(model) = CStr::from_ptr(variable.value)
            .to_str()
            .ok()
            .and_then(model_from_option)
        {
            return model;
        }
    }
    MachineModel::Nc1020
}

unsafe fn show_message(message: &str, frames: u32) {
    let Ok(message) = CString::new(message) else {
        return;
    };
    let mut retro_message = RetroMessage {
        msg: message.as_ptr(),
        frames,
    };
    environment(
        RETRO_ENVIRONMENT_SET_MESSAGE,
        &mut retro_message as *mut RetroMessage as *mut c_void,
    );
}

unsafe fn report_error(message: &str) {
    log::error!("{message}");
    show_message(message, 300);
}

unsafe fn request_pixel_format() -> bool {
    let mut pixel_format = RETRO_PIXEL_FORMAT_XRGB8888;
    if environment(
        RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,
        &mut pixel_format as *mut u32 as *mut c_void,
    ) {
        true
    } else {
        report_error("The frontend does not support the required XRGB8888 pixel format");
        false
    }
}

fn persistent_state_path(
    save_dir: &Path,
    model: MachineModel,
    firmware_fingerprint: u64,
) -> PathBuf {
    save_dir
        .join(model.name())
        .join(format!("{firmware_fingerprint:016x}.wqxs"))
}

unsafe fn persist_active_emulator() {
    let (Some(emulator), Some(path)) = (EMULATOR.as_ref(), PERSISTENT_STATE_PATH.as_deref()) else {
        return;
    };
    let result = emulator
        .save_persistent_state()
        .and_then(|state| write_persistent_state_file(path, &state));
    match result {
        Ok(()) => log::info!("Persistent state saved to {}", path.display()),
        Err(error) => report_error(&format!(
            "Failed to save persistent state {}: {error}",
            path.display()
        )),
    }
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

fn retro_host_key(keycode: u32) -> Option<HostKey> {
    let host_key = match keycode {
        RETROK_RETURN => HostKey::Return,
        RETROK_ESCAPE => HostKey::Escape,
        RETROK_SPACE => HostKey::Space,
        RETROK_BACKSPACE => HostKey::Backspace,
        RETROK_DELETE => HostKey::Delete,
        RETROK_UP => HostKey::Up,
        RETROK_DOWN => HostKey::Down,
        RETROK_LEFT => HostKey::Left,
        RETROK_RIGHT => HostKey::Right,
        RETROK_PAGEUP => HostKey::PageUp,
        RETROK_PAGEDOWN => HostKey::PageDown,
        RETROK_PERIOD => HostKey::Period,
        RETROK_COMMA => HostKey::Comma,
        RETROK_SLASH => HostKey::Slash,
        RETROK_LEFTBRACKET => HostKey::LeftBracket,
        RETROK_RIGHTBRACKET => HostKey::RightBracket,
        RETROK_BACKSLASH => HostKey::Backslash,
        RETROK_EQUALS => HostKey::Equals,
        key if (RETROK_F1..=RETROK_F12).contains(&key) => {
            HostKey::Function((key - RETROK_F1 + 1) as u8)
        }
        key if (RETROK_A..=RETROK_Z).contains(&key) => {
            HostKey::Letter(char::from_u32(key)?.to_ascii_uppercase())
        }
        key if (RETROK_0..=RETROK_9).contains(&key) => HostKey::Digit((key - RETROK_0) as u8),
        _ => return None,
    };
    Some(host_key)
}

/// Map a RetroArch keyboard keycode to the active model's keypad matrix.
#[cfg(test)]
fn map_keyboard_key(model: MachineModel, keycode: u32) -> Option<u8> {
    key_id_for_host_key(model, retro_host_key(keycode)?)
}

/// Map a RetroPad button to the active model's keypad matrix.
fn retropad_host_key(button: u32) -> Option<HostKey> {
    let host_key = match button {
        RETRO_DEVICE_ID_JOYPAD_UP => HostKey::Up,
        RETRO_DEVICE_ID_JOYPAD_DOWN => HostKey::Down,
        RETRO_DEVICE_ID_JOYPAD_LEFT => HostKey::Left,
        RETRO_DEVICE_ID_JOYPAD_RIGHT => HostKey::Right,
        RETRO_DEVICE_ID_JOYPAD_A => HostKey::Return,
        RETRO_DEVICE_ID_JOYPAD_B => HostKey::Escape,
        RETRO_DEVICE_ID_JOYPAD_X => HostKey::Function(1),
        RETRO_DEVICE_ID_JOYPAD_Y => HostKey::Function(4),
        RETRO_DEVICE_ID_JOYPAD_L => HostKey::PageUp,
        RETRO_DEVICE_ID_JOYPAD_R => HostKey::PageDown,
        RETRO_DEVICE_ID_JOYPAD_START => HostKey::Function(10),
        RETRO_DEVICE_ID_JOYPAD_SELECT => HostKey::Function(11),
        _ => return None,
    };
    Some(host_key)
}

#[cfg(test)]
fn map_joypad_button(model: MachineModel, button: u32) -> Option<u8> {
    let host_key = retropad_host_key(button)?;
    key_id_for_host_key(model, host_key)
}

unsafe extern "C" fn keyboard_event(
    down: bool,
    keycode: u32,
    _character: u32,
    _key_modifiers: u16,
) {
    if let (Some(input), Some(host_key)) = (FRONTEND_INPUT.as_mut(), retro_host_key(keycode)) {
        input.set_keyboard_key(host_key, down);
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
    let mut supports_no_game = true;
    unsafe {
        environment(
            RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME,
            &mut supports_no_game as *mut bool as *mut c_void,
        );
    }
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
        SYSTEM_DIR = None;
        SAVE_DIR = None;
        PERSISTENT_STATE_PATH = None;
        FRONTEND_INPUT = None;
        let mut sys_dir: *const c_char = ptr::null();
        if environment(
            RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY,
            &mut sys_dir as *mut *const c_char as *mut c_void,
        ) && !sys_dir.is_null()
        {
            SYSTEM_DIR = Some(CStr::from_ptr(sys_dir).to_string_lossy().into_owned());
        }

        let mut save_dir: *const c_char = ptr::null();
        if environment(
            RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY,
            &mut save_dir as *mut *const c_char as *mut c_void,
        ) && !save_dir.is_null()
        {
            SAVE_DIR = Some(CStr::from_ptr(save_dir).to_string_lossy().into_owned());
        }
    }
    log::info!("WQXEmu libretro core initialized");
}

/// Deinitialize the core
#[no_mangle]
pub extern "C" fn retro_deinit() {
    unsafe {
        persist_active_emulator();
        EMULATOR = None;
        FRONTEND_INPUT = None;
        SYSTEM_DIR = None;
        SAVE_DIR = None;
        PERSISTENT_STATE_PATH = None;
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
            valid_extensions: c"".as_ptr(),
            need_fullpath: false,
            block_extract: false,
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
    // WQXEmu uses the standard RetroPad on port 0.
}

/// Start the selected machine without content.
#[no_mangle]
pub extern "C" fn retro_load_game(info: *const RetroGameInfo) -> bool {
    unsafe {
        if !info.is_null() {
            report_error("WQXEmu does not load firmware as content");
            return false;
        }
        if !request_pixel_format() {
            return false;
        }

        let Some(system_dir) = SYSTEM_DIR.as_deref() else {
            report_error("RetroArch system directory is not configured");
            return false;
        };
        let model = selected_model();
        let files = firmware_files_for_model(Path::new(system_dir), model);
        let missing = missing_required_firmware(&files);
        if !missing.is_empty() {
            let paths = missing
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            report_error(&format!(
                "Missing {} firmware in the RetroArch system directory: {paths}",
                model.name()
            ));
            return false;
        }

        let firmware_fingerprint = match files.fingerprint() {
            Ok(fingerprint) => fingerprint,
            Err(error) => {
                report_error(&format!(
                    "Failed to identify {} firmware: {error}",
                    model.name()
                ));
                return false;
            }
        };

        let mut emu = match Emulator::new(model, &files) {
            Ok(e) => e,
            Err(e) => {
                report_error(&format!("Failed to start {}: {e}", model.name()));
                return false;
            }
        };

        emu.reset();
        let persistent_path = SAVE_DIR
            .as_deref()
            .map(Path::new)
            .map(|save_dir| persistent_state_path(save_dir, model, firmware_fingerprint));
        if let Some(path) = persistent_path.as_deref().filter(|path| path.is_file()) {
            match read_persistent_state_file(path)
                .and_then(|state| emu.load_persistent_state(&state))
            {
                Ok(()) => log::info!("Persistent state loaded from {}", path.display()),
                Err(error) => report_error(&format!(
                    "Ignoring invalid persistent state {}: {error}",
                    path.display()
                )),
            }
        }
        let mut frontend_input = FrontendInputState::default();
        frontend_input.release_all(|key_id, down| emu.set_key(key_id, down));
        PERSISTENT_STATE_PATH = persistent_path;
        EMULATOR = Some(emu);
        FRONTEND_INPUT = Some(frontend_input);

        log::info!(
            "Started {} from the RetroArch system directory",
            model.name()
        );
        show_message(
            "Physical keyboard: enable Game Focus (default: Scroll Lock)",
            300,
        );
        true
    }
}

/// Unload a game
#[no_mangle]
pub extern "C" fn retro_unload_game() {
    unsafe {
        persist_active_emulator();
        EMULATOR = None;
        FRONTEND_INPUT = None;
        PERSISTENT_STATE_PATH = None;
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

        // Process RetroPad input.
        if let (Some(get_state), Some(input)) = (INPUT_STATE_CB, FRONTEND_INPUT.as_mut()) {
            for button in 0..12 {
                let state = get_state(0, RETRO_DEVICE_JOYPAD, 0, button);
                if let Some(host_key) = retropad_host_key(button) {
                    input.set_controller_key(host_key, state != 0);
                }
            }
        }

        if let (Some(input), Some(emu)) = (FRONTEND_INPUT.as_mut(), EMULATOR.as_mut()) {
            input.sync(emu.model(), |key_id, down| emu.set_key(key_id, down));
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
        if let Some(input) = FRONTEND_INPUT.as_mut() {
            input.release_all(|key_id, down| emu.set_key(key_id, down));
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
        if let Some(ref mut input) = FRONTEND_INPUT {
            input.reset_applied();
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
fn joypad_descriptor(id: u32, description: &'static CStr) -> RetroInputDescriptor {
    RetroInputDescriptor {
        port: 0,
        device: RETRO_DEVICE_JOYPAD,
        index: 0,
        id,
        description: description.as_ptr(),
    }
}

fn input_descriptors() -> [RetroInputDescriptor; 13] {
    [
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_B, c"Escape / Back"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_Y, c"F4"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_SELECT, c"F11 / Model Hotkey"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_START, c"F10 / Model Hotkey"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_UP, c"Up"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_DOWN, c"Down"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_LEFT, c"Left"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_RIGHT, c"Right"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_A, c"Enter / Confirm"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_X, c"F1"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_L, c"Page Up"),
        joypad_descriptor(RETRO_DEVICE_ID_JOYPAD_R, c"Page Down"),
        RetroInputDescriptor {
            port: 0,
            device: 0,
            index: 0,
            id: 0,
            description: ptr::null(),
        },
    ]
}

fn set_input_descriptors() {
    let descriptors = input_descriptors();
    unsafe {
        environment(
            RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS,
            descriptors.as_ptr() as *mut c_void,
        );
    }
}

/// Set core variables for RetroArch
fn set_core_variables() {
    let variables = [
        RetroVariable {
            key: c"wqxemu_model".as_ptr(),
            value: c"Machine Model (Restart); NC1020|PC1000|CC800|NC2000|NC3000".as_ptr(),
        },
        RetroVariable {
            key: ptr::null(),
            value: ptr::null(),
        },
    ];
    unsafe {
        environment(
            RETRO_ENVIRONMENT_SET_VARIABLES,
            variables.as_ptr() as *mut c_void,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolves_canonical_system_firmware_for_each_model() {
        let system_dir = Path::new("system");
        let cases = [
            (
                MachineModel::Nc1020,
                Some("obj_lu.bin"),
                "nc1020.fls",
                None,
                None,
            ),
            (
                MachineModel::Pc1000,
                Some("pc1000.rom"),
                "pc1000.nor",
                None,
                None,
            ),
            (
                MachineModel::Cc800,
                Some("obj.bin"),
                "cc800.fls",
                None,
                None,
            ),
            (
                MachineModel::Nc2000,
                None,
                "nc2000.nor",
                Some("nc2000.nand"),
                Some("nc2000.nand0"),
            ),
            (
                MachineModel::Nc3000,
                None,
                "nc3000.nor",
                Some("nc3000.nand"),
                None,
            ),
        ];

        for (model, rom, nor, nand, nand0) in cases {
            let files = firmware_files_for_model(system_dir, model);
            let model_dir = system_dir.join("WQXEmu").join(model.name());
            assert_eq!(files.rom, rom.map(|name| model_dir.join(name)));
            assert_eq!(files.nor, Some(model_dir.join(nor)));
            assert_eq!(files.nand, nand.map(|name| model_dir.join(name)));
            assert_eq!(files.nand0, nand0.map(|name| model_dir.join(name)));
        }
    }

    #[test]
    fn parses_all_machine_model_option_values() {
        for model in [
            MachineModel::Nc1020,
            MachineModel::Pc1000,
            MachineModel::Cc800,
            MachineModel::Nc2000,
            MachineModel::Nc3000,
        ] {
            assert_eq!(
                model_from_option(&model.name().to_ascii_uppercase()),
                Some(model)
            );
        }
        assert_eq!(model_from_option("Auto"), None);
    }

    #[test]
    fn persistent_state_paths_are_isolated_by_model_and_firmware() {
        let save_dir = Path::new("saves");
        assert_eq!(
            persistent_state_path(save_dir, MachineModel::Nc2000, 0x1234),
            save_dir.join("nc2000").join("0000000000001234.wqxs")
        );
        assert_ne!(
            persistent_state_path(save_dir, MachineModel::Nc2000, 0x1234),
            persistent_state_path(save_dir, MachineModel::Nc3000, 0x1234)
        );
        assert_ne!(
            persistent_state_path(save_dir, MachineModel::Nc2000, 0x1234),
            persistent_state_path(save_dir, MachineModel::Nc2000, 0x5678)
        );
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
        assert_eq!(
            map_keyboard_key(MachineModel::Nc2000, RETROK_LEFTBRACKET),
            Some(0x07)
        );
        assert_eq!(
            map_keyboard_key(MachineModel::Nc2000, RETROK_RIGHTBRACKET),
            Some(0x0f)
        );
        assert_eq!(
            map_keyboard_key(MachineModel::Nc2000, RETROK_BACKSLASH),
            Some(0x17)
        );
        assert_eq!(
            map_keyboard_key(MachineModel::Nc2000, RETROK_PERIOD),
            Some(0x2f)
        );
    }

    #[test]
    fn advertises_every_mapped_retropad_button() {
        let descriptors = input_descriptors();
        let expected = [
            (RETRO_DEVICE_ID_JOYPAD_B, "Escape / Back"),
            (RETRO_DEVICE_ID_JOYPAD_Y, "F4"),
            (RETRO_DEVICE_ID_JOYPAD_SELECT, "F11 / Model Hotkey"),
            (RETRO_DEVICE_ID_JOYPAD_START, "F10 / Model Hotkey"),
            (RETRO_DEVICE_ID_JOYPAD_UP, "Up"),
            (RETRO_DEVICE_ID_JOYPAD_DOWN, "Down"),
            (RETRO_DEVICE_ID_JOYPAD_LEFT, "Left"),
            (RETRO_DEVICE_ID_JOYPAD_RIGHT, "Right"),
            (RETRO_DEVICE_ID_JOYPAD_A, "Enter / Confirm"),
            (RETRO_DEVICE_ID_JOYPAD_X, "F1"),
            (RETRO_DEVICE_ID_JOYPAD_L, "Page Up"),
            (RETRO_DEVICE_ID_JOYPAD_R, "Page Down"),
        ];

        for (descriptor, (id, description)) in descriptors.iter().zip(expected) {
            assert_eq!(descriptor.port, 0);
            assert_eq!(descriptor.device, RETRO_DEVICE_JOYPAD);
            assert_eq!(descriptor.index, 0);
            assert_eq!(descriptor.id, id);
            assert_eq!(
                unsafe { CStr::from_ptr(descriptor.description) }
                    .to_str()
                    .unwrap(),
                description
            );
        }
        assert!(descriptors.last().unwrap().description.is_null());
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
