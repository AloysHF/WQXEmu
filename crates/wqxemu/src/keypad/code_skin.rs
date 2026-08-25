use image::{imageops::FilterType, ImageBuffer, Rgb};
use wqxemu_core::{KeyDef, MachineModel};

use super::{key_region, Rect, SOURCE_HEIGHT, SOURCE_WIDTH};

const WINDOW_BACKGROUND: u32 = 0xF0F0F0;

#[derive(Clone, Copy)]
struct Palette {
    shell: u32,
    shell_light: u32,
    shell_dark: u32,
    trim: u32,
    key: u32,
    key_text: u32,
    accent: u32,
    accent_text: u32,
}

pub(super) fn render(
    model: MachineModel,
    width: usize,
    height: usize,
    screen: Rect,
    layout: &[KeyDef],
) -> Vec<u32> {
    let palette = palette_for(model);
    let mut canvas = Canvas::new();
    draw_device(&mut canvas, model, screen, layout, palette);
    canvas.resize(width, height)
}

fn draw_device(
    canvas: &mut Canvas,
    model: MachineModel,
    screen: Rect,
    layout: &[KeyDef],
    palette: Palette,
) {
    canvas.rounded_rect(Rect::centered(543, 383, 1030, 700), 58, 0x55585C);
    canvas.rounded_rect(Rect::centered(543, 380, 1008, 674), 48, palette.shell_dark);
    canvas.rounded_rect(Rect::centered(543, 375, 984, 650), 42, palette.shell);
    canvas.rounded_rect(Rect::centered(543, 1054, 1030, 710), 54, 0x77797B);
    canvas.rounded_rect(Rect::centered(543, 1048, 1010, 690), 46, palette.shell_dark);
    canvas.rounded_rect(Rect::centered(543, 1040, 990, 670), 38, palette.shell);

    draw_lid(canvas, model, screen, palette);
    draw_hinge(canvas, palette);
    draw_speaker(canvas, model, palette);
    draw_special_controls(canvas, model, palette);

    for def in layout {
        if let Some(region) = key_region(model, def) {
            draw_key(canvas, region, def, palette);
        }
    }

    draw_branding(canvas, model, palette);
    draw_latch(canvas, palette);
}

fn draw_lid(canvas: &mut Canvas, model: MachineModel, screen: Rect, palette: Palette) {
    let bezel = Rect {
        x: screen.x.saturating_sub(24),
        y: screen.y.saturating_sub(22),
        width: screen.width + 48,
        height: screen.height + 62,
    };
    canvas.rounded_rect(bezel, 15, palette.trim);
    canvas.rounded_rect(
        Rect {
            x: screen.x.saturating_sub(8),
            y: screen.y.saturating_sub(8),
            width: screen.width + 16,
            height: screen.height + 16,
        },
        8,
        0x31383A,
    );
    canvas.rounded_rect(screen, 5, 0x95AA78);

    let number_y = screen.y + screen.height + 27;
    for number in 1..=9 {
        let x = screen.x + screen.width * number / 10;
        canvas.text_centered(&number.to_string(), x, number_y, 5, 0x25292B);
    }

    if model == MachineModel::Nc3000 {
        for row in 0..2 {
            for col in 0..12 {
                canvas.circle(278 + col * 72, 520 + row * 35, 5, 0x777B7E);
            }
        }
    }

    canvas.line(
        90,
        bezel.y + bezel.height + 17,
        996,
        bezel.y + bezel.height + 17,
        3,
        palette.shell_light,
    );
}

fn draw_hinge(canvas: &mut Canvas, palette: Palette) {
    canvas.rounded_rect(Rect::centered(165, 712, 210, 64), 23, 0x565A5D);
    canvas.rounded_rect(Rect::centered(543, 712, 430, 48), 17, palette.shell_light);
    canvas.rounded_rect(Rect::centered(921, 712, 210, 64), 23, 0x565A5D);
    canvas.rounded_rect(Rect::centered(543, 712, 142, 26), 7, palette.shell_dark);
}

fn draw_speaker(canvas: &mut Canvas, model: MachineModel, palette: Palette) {
    let (start_x, start_y, columns, rows, spacing) = match model {
        MachineModel::Pc1000 => (98, 746, 7, 8, 25),
        MachineModel::Cc800 => (124, 532, 10, 3, 28),
        MachineModel::Nc1020 | MachineModel::Nc2000 => (108, 758, 20, 10, 16),
        MachineModel::Nc3000 => (76, 790, 21, 10, 19),
    };
    for row in 0..rows {
        for col in 0..columns {
            canvas.circle(
                start_x + col * spacing,
                start_y + row * spacing,
                5,
                palette.trim,
            );
        }
    }
}

fn draw_special_controls(canvas: &mut Canvas, model: MachineModel, palette: Palette) {
    match model {
        MachineModel::Nc1020 => {
            draw_button(canvas, Rect::centered(105, 950, 82, 52), 0xEF8A24, 0x1B1B1B);
            canvas.text_centered("VOICE", 105, 950, 5, 0x1B1B1B);
            draw_button(canvas, Rect::centered(203, 950, 82, 52), 0xEF8A24, 0x1B1B1B);
            canvas.text_centered("TIME", 203, 950, 5, 0x1B1B1B);
            canvas.circle(295, 970, 13, 0x1B5EA8);
            canvas.text_centered("RESET", 295, 938, 4, palette.key_text);
        }
        MachineModel::Nc2000 => {
            canvas.circle(305, 962, 13, 0x365A97);
            canvas.text_centered("RESET", 305, 932, 4, palette.key_text);
        }
        MachineModel::Nc3000 => {
            draw_button(canvas, Rect::centered(132, 964, 62, 58), 0xB8BDC0, 0x303335);
            canvas.text_centered("NET", 132, 1008, 4, 0x153EBD);
        }
        MachineModel::Pc1000 | MachineModel::Cc800 => {}
    }
}

fn draw_key(canvas: &mut Canvas, region: Rect, def: &KeyDef, palette: Palette) {
    let is_hot_key = def.drow == 0;
    let is_function = def.drow == 1;
    let (face, text) = if is_hot_key {
        (palette.accent, palette.accent_text)
    } else if is_function {
        (mix(palette.accent, 0xFFFFFF, 1, 4), palette.key_text)
    } else {
        (palette.key, palette.key_text)
    };
    draw_button(canvas, region, face, text);
    let label = compact_label(def.label);
    let pixel_size = label_size(region, label);
    canvas.text_centered(
        label,
        region.x + region.width / 2,
        region.y + region.height / 2,
        pixel_size,
        text,
    );

    if !def.hint.is_empty() && def.hint != def.label && def.drow >= 2 {
        canvas.text_centered(
            def.hint,
            region.x + region.width / 2,
            region.y.saturating_sub(13),
            3,
            palette.accent,
        );
    }
}

fn draw_button(canvas: &mut Canvas, region: Rect, face: u32, _text: u32) {
    let shadow = Rect {
        x: region.x + 3,
        y: region.y + 5,
        ..region
    };
    let radius = (region.height / 5).max(5);
    canvas.rounded_rect(shadow, radius, 0x77797A);
    canvas.rounded_rect(region, radius, 0x171A1C);
    let inset = Rect {
        x: region.x + 4,
        y: region.y + 4,
        width: region.width.saturating_sub(8),
        height: region.height.saturating_sub(9),
    };
    canvas.rounded_rect(inset, radius.saturating_sub(3), face);
    canvas.line(
        inset.x + 5,
        inset.y + 3,
        inset.x + inset.width.saturating_sub(5),
        inset.y + 3,
        2,
        mix(face, 0xFFFFFF, 1, 3),
    );
}

fn draw_branding(canvas: &mut Canvas, model: MachineModel, palette: Palette) {
    let model_name = model.name().to_ascii_uppercase();
    canvas.text_centered("WENQUXING", 543, 620, 8, palette.trim);
    canvas.text_centered(&model_name, 543, 654, 8, palette.accent);
    canvas.text_centered("ELECTRONIC DICTIONARY", 790, 770, 5, palette.trim);
}

fn draw_latch(canvas: &mut Canvas, palette: Palette) {
    canvas.rounded_rect(Rect::centered(543, 1370, 104, 28), 5, palette.shell_dark);
    canvas.rounded_rect(Rect::centered(543, 1370, 70, 16), 3, 0x333638);
}

fn compact_label(label: &str) -> &str {
    match label {
        "PGUP" => "PU",
        "PGDN" => "PD",
        "SHIFT" => "SHFT",
        _ => label,
    }
}

fn label_size(region: Rect, label: &str) -> usize {
    let length = label.chars().count().max(1);
    let horizontal = region.width.saturating_sub(14) / (length * 6 - 1);
    let vertical = region.height.saturating_sub(14) / 7;
    horizontal.min(vertical).clamp(2, 7)
}

fn palette_for(model: MachineModel) -> Palette {
    match model {
        MachineModel::Pc1000 => Palette {
            shell: 0xD7D8D8,
            shell_light: 0xF4F4F2,
            shell_dark: 0x979B9E,
            trim: 0x20262B,
            key: 0x293038,
            key_text: 0xF5F7FA,
            accent: 0x163D98,
            accent_text: 0xFFFFFF,
        },
        MachineModel::Cc800 => Palette {
            shell: 0xD3D5D4,
            shell_light: 0xF5F5F2,
            shell_dark: 0x999D9D,
            trim: 0x252B30,
            key: 0x303840,
            key_text: 0xFFFFFF,
            accent: 0x00A989,
            accent_text: 0xFFFFFF,
        },
        MachineModel::Nc1020 => Palette {
            shell: 0xE6E7E7,
            shell_light: 0xFFFFFF,
            shell_dark: 0xAEB3B7,
            trim: 0x0066C5,
            key: 0xE4E4E2,
            key_text: 0x151719,
            accent: 0x056FD3,
            accent_text: 0xFFFFFF,
        },
        MachineModel::Nc2000 => Palette {
            shell: 0xDFE0E2,
            shell_light: 0xF9F9F7,
            shell_dark: 0xA9ACB9,
            trim: 0x737DB6,
            key: 0xE7E7E5,
            key_text: 0x151719,
            accent: 0x6450D4,
            accent_text: 0xFFFFFF,
        },
        MachineModel::Nc3000 => Palette {
            shell: 0xD0D2D3,
            shell_light: 0xF2F2F0,
            shell_dark: 0x8D9194,
            trim: 0x4E5357,
            key: 0x343B40,
            key_text: 0xF7F7F5,
            accent: 0x315E73,
            accent_text: 0xFFFFFF,
        },
    }
}

fn mix(first: u32, second: u32, first_weight: u32, second_weight: u32) -> u32 {
    let total = first_weight + second_weight;
    let channel = |shift: u32| {
        (((first >> shift) & 0xFFu32) * first_weight
            + ((second >> shift) & 0xFFu32) * second_weight)
            / total
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

struct Canvas {
    pixels: Vec<u32>,
}

impl Canvas {
    fn new() -> Self {
        Self {
            pixels: vec![WINDOW_BACKGROUND; SOURCE_WIDTH * SOURCE_HEIGHT],
        }
    }

    fn rounded_rect(&mut self, rect: Rect, radius: usize, color: u32) {
        let x1 = (rect.x + rect.width).min(SOURCE_WIDTH);
        let y1 = (rect.y + rect.height).min(SOURCE_HEIGHT);
        let radius = radius.min(rect.width / 2).min(rect.height / 2);
        for y in rect.y.min(SOURCE_HEIGHT)..y1 {
            for x in rect.x.min(SOURCE_WIDTH)..x1 {
                let dx = if x < rect.x + radius {
                    rect.x + radius - x
                } else {
                    x.saturating_sub(rect.x + rect.width - radius - 1)
                };
                let dy = if y < rect.y + radius {
                    rect.y + radius - y
                } else {
                    y.saturating_sub(rect.y + rect.height - radius - 1)
                };
                if dx == 0 || dy == 0 || dx * dx + dy * dy <= radius * radius {
                    self.pixels[y * SOURCE_WIDTH + x] = color;
                }
            }
        }
    }

    fn circle(&mut self, center_x: usize, center_y: usize, radius: usize, color: u32) {
        let rect = Rect::centered(center_x, center_y, radius * 2 + 1, radius * 2 + 1);
        for y in rect.y..(rect.y + rect.height).min(SOURCE_HEIGHT) {
            for x in rect.x..(rect.x + rect.width).min(SOURCE_WIDTH) {
                let dx = x as isize - center_x as isize;
                let dy = y as isize - center_y as isize;
                if dx * dx + dy * dy <= (radius * radius) as isize {
                    self.pixels[y * SOURCE_WIDTH + x] = color;
                }
            }
        }
    }

    fn line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, thickness: usize, color: u32) {
        let steps = x0.abs_diff(x1).max(y0.abs_diff(y1)).max(1);
        for step in 0..=steps {
            let x = x0 as isize + (x1 as isize - x0 as isize) * step as isize / steps as isize;
            let y = y0 as isize + (y1 as isize - y0 as isize) * step as isize / steps as isize;
            self.rounded_rect(
                Rect::centered(x.max(0) as usize, y.max(0) as usize, thickness, thickness),
                thickness / 2,
                color,
            );
        }
    }

    fn text_centered(
        &mut self,
        text: &str,
        center_x: usize,
        center_y: usize,
        pixel_size: usize,
        color: u32,
    ) {
        if pixel_size == 0 || text.is_empty() {
            return;
        }
        let text_width = text.chars().count() * 6 * pixel_size - pixel_size;
        let start_x = center_x.saturating_sub(text_width / 2);
        let start_y = center_y.saturating_sub(7 * pixel_size / 2);
        for (index, character) in text.chars().enumerate() {
            let glyph = glyph(character.to_ascii_uppercase());
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        self.rounded_rect(
                            Rect {
                                x: start_x + (index * 6 + col) * pixel_size,
                                y: start_y + row * pixel_size,
                                width: pixel_size,
                                height: pixel_size,
                            },
                            0,
                            color,
                        );
                    }
                }
            }
        }
    }

    fn resize(self, width: usize, height: usize) -> Vec<u32> {
        let mut raw = Vec::with_capacity(self.pixels.len() * 3);
        for color in self.pixels {
            raw.push(((color >> 16) & 0xFF) as u8);
            raw.push(((color >> 8) & 0xFF) as u8);
            raw.push((color & 0xFF) as u8);
        }
        let source: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_raw(SOURCE_WIDTH as u32, SOURCE_HEIGHT as u32, raw)
                .expect("code skin buffer dimensions must match");
        let resized =
            image::imageops::resize(&source, width as u32, height as u32, FilterType::Lanczos3);
        resized
            .pixels()
            .map(|pixel| {
                let [r, g, b] = pixel.0;
                ((r as u32) << 16) | ((g as u32) << 8) | b as u32
            })
            .collect()
    }
}

fn glyph(character: char) -> [u8; 7] {
    match character {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x12, 0x12, 0x0C],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x14, 0x04, 0x04, 0x04, 0x1F],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
        '6' => [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x0E],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        _ => [0; 7],
    }
}
