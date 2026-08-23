// WQXEmu - Standalone desktop frontend for Wenquxing emulators.
//
// This is a simple desktop application that runs the emulator in a
// window with keyboard input support. The target model is selected with
// `--model` or auto-detected from the ROM files.

use anyhow::{Context, Result};
use clap::Parser;
use minifb::{Key, Window, WindowOptions};

use std::path::{Path, PathBuf};

#[cfg(test)]
use wqxemu_core::key_id_for_host_key;
use wqxemu_core::save::{read_persistent_state_file, write_persistent_state_file};
use wqxemu_core::{
    detect_model, layout_for, Emulator, FrontendInputState, HostKey, MachineModel, RomFiles,
    LCD_HEIGHT, LCD_WIDTH,
};

mod keypad;
use keypad::{DeviceSkin, SkinInput};

/// WQXEmu - Wenquxing Emulator
#[derive(Parser)]
#[command(name = "wqxemu", version, about)]
struct Args {
    /// Path to the system ROM dump (obj_lu.bin / *.rom)
    #[arg(long, value_name = "PATH", help = "Path to the system ROM dump")]
    rom: Option<PathBuf>,

    /// Path to the NOR Flash file (nc1020.fls / *.nor)
    #[arg(long, value_name = "PATH", help = "Path to the NOR Flash dump")]
    nor: Option<PathBuf>,

    /// Path to the NAND Flash file (NC2000/NC3000)
    #[arg(long, value_name = "PATH", help = "Path to the NAND Flash dump")]
    nand: Option<PathBuf>,

    /// Path to the first NAND plane (required for NC2000, optional for NC3000)
    #[arg(long, value_name = "PATH", help = "Path to the first NAND plane dump")]
    nand0: Option<PathBuf>,

    /// Load this persistent session if it exists and atomically update it on exit
    #[arg(
        long,
        value_name = "PATH",
        help = "Load/save a persistent session without modifying ROM or Flash dump files"
    )]
    state_file: Option<PathBuf>,

    /// Hardware model: nc1020, pc1000, cc800, nc2000 or nc3000 (default: auto-detect)
    #[arg(long, help = "Hardware model (nc1020, pc1000, cc800, nc2000, nc3000)")]
    model: Option<String>,

    /// Scale factor for the device skin (4 = 600 pixels high)
    #[arg(
        short = 's',
        long,
        default_value = "4",
        help = "Device skin scale factor (4 = 600 pixels high)"
    )]
    scale: u32,

    /// Take a screenshot after N frames and exit (saves as PNG)
    #[arg(short = 'S', long = "screenshot", value_name = "PATH")]
    screenshot: Option<String>,

    /// Number of frames to run before taking the screenshot (default: 30)
    #[arg(long = "screenshot-frames", default_value = "30")]
    screenshot_frames: u32,
}

fn minifb_host_key(key: Key) -> Option<HostKey> {
    let host_key = match key {
        Key::A => HostKey::Letter('A'),
        Key::B => HostKey::Letter('B'),
        Key::C => HostKey::Letter('C'),
        Key::D => HostKey::Letter('D'),
        Key::E => HostKey::Letter('E'),
        Key::F => HostKey::Letter('F'),
        Key::G => HostKey::Letter('G'),
        Key::H => HostKey::Letter('H'),
        Key::I => HostKey::Letter('I'),
        Key::J => HostKey::Letter('J'),
        Key::K => HostKey::Letter('K'),
        Key::L => HostKey::Letter('L'),
        Key::M => HostKey::Letter('M'),
        Key::N => HostKey::Letter('N'),
        Key::O => HostKey::Letter('O'),
        Key::P => HostKey::Letter('P'),
        Key::Q => HostKey::Letter('Q'),
        Key::R => HostKey::Letter('R'),
        Key::S => HostKey::Letter('S'),
        Key::T => HostKey::Letter('T'),
        Key::U => HostKey::Letter('U'),
        Key::V => HostKey::Letter('V'),
        Key::W => HostKey::Letter('W'),
        Key::X => HostKey::Letter('X'),
        Key::Y => HostKey::Letter('Y'),
        Key::Z => HostKey::Letter('Z'),
        Key::Key0 => HostKey::Digit(0),
        Key::Key1 => HostKey::Digit(1),
        Key::Key2 => HostKey::Digit(2),
        Key::Key3 => HostKey::Digit(3),
        Key::Key4 => HostKey::Digit(4),
        Key::Key5 => HostKey::Digit(5),
        Key::Key6 => HostKey::Digit(6),
        Key::Key7 => HostKey::Digit(7),
        Key::Key8 => HostKey::Digit(8),
        Key::Key9 => HostKey::Digit(9),
        Key::F1 => HostKey::Function(1),
        Key::F2 => HostKey::Function(2),
        Key::F3 => HostKey::Function(3),
        Key::F4 => HostKey::Function(4),
        Key::F5 => HostKey::Function(5),
        Key::F6 => HostKey::Function(6),
        Key::F7 => HostKey::Function(7),
        Key::F8 => HostKey::Function(8),
        Key::F9 => HostKey::Function(9),
        Key::F10 => HostKey::Function(10),
        Key::F11 => HostKey::Function(11),
        Key::F12 => HostKey::Function(12),
        Key::Enter => HostKey::Return,
        Key::Escape => HostKey::Escape,
        Key::Space => HostKey::Space,
        Key::Backspace => HostKey::Backspace,
        Key::Delete => HostKey::Delete,
        Key::Up => HostKey::Up,
        Key::Down => HostKey::Down,
        Key::Left => HostKey::Left,
        Key::Right => HostKey::Right,
        Key::PageUp => HostKey::PageUp,
        Key::PageDown => HostKey::PageDown,
        _ => return None,
    };
    Some(host_key)
}

#[cfg(test)]
fn map_key(model: MachineModel, key: Key) -> Option<u8> {
    key_id_for_host_key(model, minifb_host_key(key)?)
}

/// Map a resized window coordinate through the aspect-ratio letterbox.
fn window_to_skin_pos(
    mouse: (f32, f32),
    window_size: (usize, usize),
    skin_size: (usize, usize),
) -> Option<(usize, usize)> {
    let (window_width, window_height) = (window_size.0 as f32, window_size.1 as f32);
    let (skin_width, skin_height) = (skin_size.0 as f32, skin_size.1 as f32);
    if window_width <= 0.0 || window_height <= 0.0 {
        return None;
    }

    let scale = (window_width / skin_width).min(window_height / skin_height);
    let drawn_width = skin_width * scale;
    let drawn_height = skin_height * scale;
    let offset_x = (window_width - drawn_width) / 2.0;
    let offset_y = (window_height - drawn_height) / 2.0;
    if mouse.0 < offset_x
        || mouse.0 >= offset_x + drawn_width
        || mouse.1 < offset_y
        || mouse.1 >= offset_y + drawn_height
    {
        return None;
    }

    Some((
        ((mouse.0 - offset_x) / scale) as usize,
        ((mouse.1 - offset_y) / scale) as usize,
    ))
}

fn load_persistent_state_if_present(emu: &mut Emulator, path: Option<&Path>) -> Result<bool> {
    if let Some(path) = path.filter(|path| path.exists()) {
        let state = read_persistent_state_file(path)?;
        emu.load_persistent_state(&state)
            .with_context(|| format!("Failed to load persistent state: {}", path.display()))?;
        log::info!("Persistent state loaded from {}", path.display());
        return Ok(true);
    }
    Ok(false)
}

fn save_persistent_state_if_requested(emu: &Emulator, path: Option<&Path>) -> Result<()> {
    if let Some(path) = path {
        let state = emu.save_persistent_state()?;
        write_persistent_state_file(path, &state)?;
        log::info!("Persistent state saved to {}", path.display());
    }
    Ok(())
}

fn normalized_output_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return std::fs::canonicalize(path)
            .with_context(|| format!("Failed to resolve path: {}", path.display()));
    }
    let file_name = path
        .file_name()
        .context("State file path must include a file name")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = std::fs::canonicalize(parent)
        .with_context(|| format!("Failed to resolve directory: {}", parent.display()))?;
    Ok(parent.join(file_name))
}

fn validate_state_file_path(path: Option<&Path>, files: &RomFiles) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let state_path = normalized_output_path(path)?;
    for source in [&files.rom, &files.nor, &files.nand, &files.nand0]
        .into_iter()
        .flatten()
    {
        if normalized_output_path(source)? == state_path {
            anyhow::bail!(
                "State file must not overwrite a ROM or Flash dump: {}",
                path.display()
            );
        }
    }
    Ok(())
}

fn validate_firmware_files(model: MachineModel, files: &RomFiles) -> Result<()> {
    let require = |path: &Option<PathBuf>, option: &str| -> Result<()> {
        if path.is_none() {
            anyhow::bail!("{} requires {} <PATH>", model.name(), option);
        }
        Ok(())
    };
    let reject = |path: &Option<PathBuf>, option: &str| -> Result<()> {
        if path.is_some() {
            anyhow::bail!("{} does not use {}", model.name(), option);
        }
        Ok(())
    };

    match model {
        MachineModel::Nc1020 | MachineModel::Pc1000 | MachineModel::Cc800 => {
            require(&files.rom, "--rom")?;
            require(&files.nor, "--nor")?;
            reject(&files.nand, "--nand")?;
            reject(&files.nand0, "--nand0")?;
        }
        MachineModel::Nc2000 => {
            reject(&files.rom, "--rom")?;
            require(&files.nor, "--nor")?;
            require(&files.nand, "--nand")?;
            require(&files.nand0, "--nand0")?;
        }
        MachineModel::Nc3000 => {
            reject(&files.rom, "--rom")?;
            require(&files.nor, "--nor")?;
            require(&files.nand, "--nand")?;
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    // Assemble ROM files and pick the model
    let files = RomFiles::new(args.rom, args.nor, args.nand, args.nand0);
    let model = match &args.model {
        Some(name) => MachineModel::from_name(name)
            .ok_or_else(|| anyhow::anyhow!("unknown model: {}", name))?,
        None => detect_model(&files),
    };
    log::info!("Selected model: {}", model.name());
    validate_firmware_files(model, &files)?;
    validate_state_file_path(args.state_file.as_deref(), &files)?;

    // Create emulator
    let mut emu = Emulator::new(model, &files)?;
    emu.reset();
    load_persistent_state_if_present(&mut emu, args.state_file.as_deref())?;

    log::info!("Emulator initialized, PC=0x{:04X}", emu.pc());

    // If screenshot mode, run N frames and save screenshot to the
    // user-provided path, then exit.
    if let Some(ref screenshot_path) = args.screenshot {
        log::info!(
            "Running {} frames before taking screenshot...",
            args.screenshot_frames
        );
        for _ in 0..args.screenshot_frames {
            emu.run_frame();
        }
        let pixels = emu.framebuffer();
        save_screenshot(&pixels, screenshot_path)?;
        log::info!("Screenshot saved to {}", screenshot_path);
        save_persistent_state_if_requested(&emu, args.state_file.as_deref())?;
        return Ok(());
    }

    // Create a normal resizable window whose client area is the device skin.
    let skin = DeviceSkin::load(model, args.scale)?;
    let window_width = skin.width();
    let window_height = skin.height();

    let mut window = Window::new(
        &format!("WQXEmu - {}", model.name().to_uppercase()),
        window_width,
        window_height,
        WindowOptions {
            resize: true,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .expect("Failed to create window");

    window.set_target_fps(emu.frame_rate() as usize);

    // Main event loop
    let layout = layout_for(model);
    let mut input_state = FrontendInputState::default();
    let mut mouse_input: Option<SkinInput> = None;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Process input
        window
            .get_keys_pressed(minifb::KeyRepeat::No)
            .iter()
            .for_each(|key| {
                if let Some(host_key) = minifb_host_key(*key) {
                    input_state.set_keyboard_key(host_key, true);
                }
            });

        window.get_keys_released().iter().for_each(|key| {
            if let Some(host_key) = minifb_host_key(*key) {
                input_state.set_keyboard_key(host_key, false);
            }
        });

        // Mouse: click the virtual keypad.
        let mouse_down = window.get_mouse_down(minifb::MouseButton::Left);
        let mouse_pos = window
            .get_unscaled_mouse_pos(minifb::MouseMode::Discard)
            .and_then(|mouse| {
                window_to_skin_pos(mouse, window.get_size(), (window_width, window_height))
            });
        let next_input = mouse_down
            .then(|| mouse_pos.and_then(|(x, y)| skin.hit_test(x, y, layout)))
            .flatten();
        if mouse_input != next_input {
            if next_input == Some(SkinInput::Reset) {
                emu.reset();
                input_state.reset_applied();
            }
            mouse_input = next_input;
        }
        if let Some(SkinInput::Key(key_id)) = mouse_input {
            input_state.set_pointer_key(Some(key_id));
        } else {
            input_state.set_pointer_key(None);
        }
        input_state.sync(model, |key_id, down| emu.set_key(key_id, down));

        // Run one frame
        emu.run_frame();

        // Get framebuffer and render
        let pixels = emu.framebuffer();

        let buffer = skin.render(&pixels, layout, input_state.pressed());

        window
            .update_with_buffer(&buffer, window_width, window_height)
            .expect("Failed to update window");
    }

    save_persistent_state_if_requested(&emu, args.state_file.as_deref())?;

    Ok(())
}

/// Save framebuffer as PNG screenshot
fn save_screenshot(pixels: &[u32], path: &str) -> Result<()> {
    use image::{ImageBuffer, Rgba};

    let width = LCD_WIDTH as u32;
    let height = LCD_HEIGHT as u32;

    let img = ImageBuffer::from_fn(width, height, |x, y| {
        let idx = (y * width + x) as usize;
        let pixel = pixels[idx];
        let r = ((pixel >> 16) & 0xFF) as u8;
        let g = ((pixel >> 8) & 0xFF) as u8;
        let b = (pixel & 0xFF) as u8;
        let a = ((pixel >> 24) & 0xFF) as u8;
        Rgba([r, g, b, a])
    });

    img.save(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        map_key, validate_firmware_files, validate_state_file_path, window_to_skin_pos, Args,
    };
    use clap::Parser;
    use minifb::Key;
    use std::path::{Path, PathBuf};
    use wqxemu_core::save::{read_persistent_state_file, write_persistent_state_file};
    use wqxemu_core::{key_ids, MachineModel, RomFiles};

    #[test]
    fn firmware_paths_use_explicit_storage_options() {
        let args = Args::try_parse_from([
            "wqxemu",
            "--rom",
            "system.bin",
            "--nor",
            "flash.nor",
            "--nand",
            "flash.nand",
            "--nand0",
            "flash.nand0",
        ])
        .unwrap();

        assert_eq!(args.rom, Some(PathBuf::from("system.bin")));
        assert_eq!(args.nor, Some(PathBuf::from("flash.nor")));
        assert_eq!(args.nand, Some(PathBuf::from("flash.nand")));
        assert_eq!(args.nand0, Some(PathBuf::from("flash.nand0")));
    }

    #[test]
    fn legacy_firmware_arguments_are_rejected() {
        assert!(Args::try_parse_from(["wqxemu", "system.bin"]).is_err());
        assert!(Args::try_parse_from(["wqxemu", "-n", "flash.nor"]).is_err());
        assert!(Args::try_parse_from(["wqxemu", "--nor-path", "flash.nor"]).is_err());
        assert!(Args::try_parse_from(["wqxemu", "--nand-path", "flash.nand"]).is_err());
        assert!(Args::try_parse_from(["wqxemu", "--nand0-path", "flash.nand0"]).is_err());
    }

    #[test]
    fn firmware_files_are_validated_for_the_selected_model() {
        let rom_and_nor = RomFiles::new(
            Some(PathBuf::from("system.bin")),
            Some(PathBuf::from("flash.nor")),
            None,
            None,
        );
        assert!(validate_firmware_files(MachineModel::Nc1020, &rom_and_nor).is_ok());
        assert!(validate_firmware_files(MachineModel::Pc1000, &rom_and_nor).is_ok());
        assert!(validate_firmware_files(MachineModel::Cc800, &rom_and_nor).is_ok());

        let nc2000 = RomFiles::new(
            None,
            Some(PathBuf::from("flash.nor")),
            Some(PathBuf::from("flash.nand")),
            Some(PathBuf::from("flash.nand0")),
        );
        assert!(validate_firmware_files(MachineModel::Nc2000, &nc2000).is_ok());
        assert!(validate_firmware_files(MachineModel::Nc3000, &nc2000).is_ok());

        let missing_nand0 = RomFiles::new(
            None,
            Some(PathBuf::from("flash.nor")),
            Some(PathBuf::from("flash.nand")),
            None,
        );
        assert!(validate_firmware_files(MachineModel::Nc2000, &missing_nand0).is_err());
        assert!(validate_firmware_files(MachineModel::Nc3000, &missing_nand0).is_ok());

        let unexpected_rom = RomFiles::new(
            Some(PathBuf::from("system.bin")),
            Some(PathBuf::from("flash.nor")),
            Some(PathBuf::from("flash.nand")),
            None,
        );
        assert!(validate_firmware_files(MachineModel::Nc3000, &unexpected_rom).is_err());
    }

    #[test]
    fn persistent_state_file_is_opt_in_and_accepts_any_path() {
        let default_args = Args::try_parse_from(["wqxemu"]).unwrap();
        assert!(default_args.state_file.is_none());

        let enabled_args = Args::try_parse_from(["wqxemu", "--state-file", "device.wqxs"]).unwrap();
        assert_eq!(enabled_args.state_file, Some(PathBuf::from("device.wqxs")));
    }

    #[test]
    fn persistent_state_file_must_not_alias_a_source_dump() {
        let files = RomFiles::new(None, Some(PathBuf::from("device.nor")), None, None);
        assert!(validate_state_file_path(Some(Path::new("device.wqxs")), &files).is_ok());
        assert!(validate_state_file_path(Some(Path::new("device.nor")), &files).is_err());
    }

    #[test]
    fn persistent_state_file_is_compressed_and_atomically_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("device.wqxs");

        write_persistent_state_file(&path, b"first state").unwrap();
        write_persistent_state_file(&path, b"replacement state").unwrap();

        assert_eq!(
            read_persistent_state_file(&path).unwrap(),
            b"replacement state"
        );
    }

    #[test]
    fn resized_window_coordinates_follow_the_letterboxed_skin() {
        assert_eq!(
            window_to_skin_pos((400.0, 300.0), (800, 600), (400, 600)),
            Some((200, 300))
        );
        assert_eq!(
            window_to_skin_pos((100.0, 300.0), (800, 600), (400, 600)),
            None
        );
    }

    #[test]
    fn nc1020_pc_keyboard_uses_reference_matrix_ids() {
        assert_eq!(map_key(MachineModel::Nc1020, Key::F5), Some(key_ids::F5));
        assert_eq!(map_key(MachineModel::Nc1020, Key::F9), Some(key_ids::F9));
        assert_eq!(map_key(MachineModel::Nc1020, Key::Q), Some(0x20));
        assert_eq!(map_key(MachineModel::Nc1020, Key::A), Some(0x28));
        assert_eq!(map_key(MachineModel::Nc1020, Key::Space), Some(0x3E));
        assert_eq!(map_key(MachineModel::Nc1020, Key::Key1), Some(0x34));
    }
}
