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
}

#[derive(Clone, Copy)]
enum ButtonShape {
    Rect(usize),
    Capsule,
    Circle,
}

#[derive(Clone, Copy)]
struct ButtonStyle {
    face: u32,
    border: u32,
    text: u32,
    shape: ButtonShape,
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
            draw_key(canvas, region, def, model, palette);
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
            draw_aux_button(canvas, Rect::centered(105, 950, 82, 52), 0xEF8A24, 0x1B1B1B);
            canvas.text_centered("VOICE", 105, 950, 2, 0x1B1B1B);
            draw_aux_button(canvas, Rect::centered(203, 950, 82, 52), 0xEF8A24, 0x1B1B1B);
            canvas.text_centered("TIME", 203, 950, 3, 0x1B1B1B);
            canvas.circle(295, 970, 13, 0x1B5EA8);
            canvas.text_centered("RESET", 295, 938, 2, palette.key_text);
        }
        MachineModel::Nc2000 => {
            draw_aux_button(canvas, Rect::centered(112, 946, 76, 48), 0xF28A22, 0x161616);
            canvas.text_centered("VOICE", 112, 946, 2, 0x161616);
            draw_aux_button(canvas, Rect::centered(208, 946, 76, 48), 0xF28A22, 0x161616);
            canvas.text_centered("TIME", 208, 946, 3, 0x161616);
            canvas.circle(305, 962, 13, 0x365A97);
            canvas.text_centered("RESET", 305, 932, 2, palette.key_text);
        }
        MachineModel::Nc3000 => {
            draw_styled_button(
                canvas,
                Rect::centered(132, 964, 62, 58),
                ButtonStyle {
                    face: 0xB8BDC0,
                    border: 0x303335,
                    text: 0x153EBD,
                    shape: ButtonShape::Circle,
                },
            );
            canvas.text_centered("NET", 132, 1008, 4, 0x153EBD);
            for (x, label) in [(278, "VOICE"), (358, "TIME"), (438, "NET")] {
                draw_aux_button(canvas, Rect::centered(x, 968, 64, 42), 0xD5A900, 0xFFFFFF);
                canvas.text_centered(label, x, 968, 2, 0xFFFFFF);
            }
        }
        MachineModel::Pc1000 => {
            for (x, label) in [(380, "VOICE"), (478, "TIME")] {
                draw_aux_button(canvas, Rect::centered(x, 933, 82, 50), 0xF4A400, 0xFFFFFF);
                canvas.text_centered(label, x, 933, 2, 0xFFFFFF);
            }
        }
        MachineModel::Cc800 => {
            canvas.circle(120, 903, 15, 0x25292C);
            canvas.text_centered("RESET", 180, 903, 3, palette.trim);
        }
    }
}

fn draw_key(
    canvas: &mut Canvas,
    region: Rect,
    def: &KeyDef,
    model: MachineModel,
    palette: Palette,
) {
    let style = key_style(model, def, palette);
    let (inset, radius) = draw_styled_button(canvas, region, style);

    if let Some(number) = numeric_legend(def) {
        let segment_color = number_color(model);
        canvas.rounded_rect_right(inset, radius.saturating_sub(3), segment_color);
        let main_label = if def.drow == 5 {
            ""
        } else {
            def.label.split('/').next().unwrap_or(def.label)
        };
        if main_label != "0" && main_label != "." {
            canvas.text_centered(
                main_label,
                region.x + region.width * 3 / 10,
                region.y + region.height / 2,
                label_size(region, main_label).min(6),
                style.text,
            );
        }
        canvas.text_centered(
            number,
            region.x + region.width * 7 / 10,
            region.y + region.height / 2,
            6,
            number_text_color(model),
        );
    } else if let Some(direction) = arrow_direction(def.label) {
        canvas.triangle(
            region.x + region.width / 2,
            region.y + region.height / 2,
            13,
            direction,
            navigation_color(model),
        );
    } else {
        let label = compact_label(def.label);
        canvas.text_centered(
            label,
            region.x + region.width / 2,
            region.y + region.height / 2,
            label_size(region, label),
            style.text,
        );
    }
}

fn draw_aux_button(canvas: &mut Canvas, region: Rect, face: u32, text: u32) {
    draw_styled_button(
        canvas,
        region,
        ButtonStyle {
            face,
            border: 0x171A1C,
            text,
            shape: ButtonShape::Rect(8),
        },
    );
}

fn draw_styled_button(canvas: &mut Canvas, region: Rect, style: ButtonStyle) -> (Rect, usize) {
    let shadow = Rect {
        x: region.x + 3,
        y: region.y + 5,
        ..region
    };
    let radius = match style.shape {
        ButtonShape::Rect(radius) => radius,
        ButtonShape::Capsule | ButtonShape::Circle => region.height / 2,
    };
    canvas.rounded_rect(shadow, radius, 0x77797A);
    canvas.rounded_rect(region, radius, style.border);
    let inset = Rect {
        x: region.x + 4,
        y: region.y + 4,
        width: region.width.saturating_sub(8),
        height: region.height.saturating_sub(9),
    };
    canvas.rounded_rect(inset, radius.saturating_sub(3), style.face);
    canvas.line(
        inset.x + 5,
        inset.y + 3,
        inset.x + inset.width.saturating_sub(5),
        inset.y + 3,
        2,
        mix(style.face, 0xFFFFFF, 1, 3),
    );
    (inset, radius)
}

fn key_style(model: MachineModel, def: &KeyDef, palette: Palette) -> ButtonStyle {
    let base = ButtonStyle {
        face: palette.key,
        border: 0x171A1C,
        text: palette.key_text,
        shape: ButtonShape::Rect(
            if matches!(model, MachineModel::Nc1020 | MachineModel::Nc2000) {
                6
            } else {
                13
            },
        ),
    };
    match (model, def.drow, def.dcol) {
        (MachineModel::Pc1000, 0, 0..=5) => ButtonStyle {
            face: 0x403044,
            text: 0xFFFFFF,
            ..base
        },
        (MachineModel::Pc1000, 0, 6) => ButtonStyle {
            face: 0xF4A400,
            text: 0xFFFFFF,
            ..base
        },
        (MachineModel::Pc1000, 1, 8) => ButtonStyle {
            face: 0xF0D9D2,
            text: 0x173C9C,
            shape: ButtonShape::Circle,
            ..base
        },
        (MachineModel::Cc800, 0, 0..=5) => ButtonStyle {
            face: 0x00A989,
            text: 0xFFFFFF,
            shape: ButtonShape::Capsule,
            ..base
        },
        (MachineModel::Cc800, 0, 6) => ButtonStyle {
            face: 0x00A989,
            text: 0xFFFFFF,
            shape: ButtonShape::Circle,
            ..base
        },
        (MachineModel::Cc800, 1, 2..=5) => ButtonStyle {
            face: 0xFFC400,
            text: 0x9B1B26,
            shape: ButtonShape::Circle,
            ..base
        },
        (MachineModel::Cc800, 1, 8) => ButtonStyle {
            face: 0xC43B75,
            text: 0x171719,
            shape: ButtonShape::Circle,
            ..base
        },
        (MachineModel::Nc1020, 0, _) => ButtonStyle {
            face: 0xAAA0C2,
            text: 0x171719,
            ..base
        },
        (MachineModel::Nc1020, 1, 2..=5) => ButtonStyle {
            face: 0x80CDE9,
            text: 0x171719,
            ..base
        },
        (MachineModel::Nc1020, 1, 8) => ButtonStyle {
            face: 0x82D2ED,
            text: 0x171719,
            shape: ButtonShape::Circle,
            ..base
        },
        (MachineModel::Nc1020, 3, 9) => ButtonStyle {
            face: 0xF79332,
            text: 0x171719,
            ..base
        },
        (MachineModel::Nc2000, 0, _) => ButtonStyle {
            face: 0x9DC85C,
            text: 0x171719,
            ..base
        },
        (MachineModel::Nc2000, 3, 9) => ButtonStyle {
            face: 0x59C9D1,
            text: 0x171719,
            ..base
        },
        (MachineModel::Nc3000, 0, _) => ButtonStyle {
            face: 0x30343A,
            text: 0xFFFFFF,
            shape: ButtonShape::Capsule,
            ..base
        },
        (MachineModel::Nc3000, 1, 2..=5) => ButtonStyle {
            face: 0xA9ADB0,
            text: 0x171719,
            shape: ButtonShape::Circle,
            ..base
        },
        (MachineModel::Nc3000, 1, 8) => ButtonStyle {
            face: 0xB7B9B8,
            text: 0x173C9C,
            shape: ButtonShape::Circle,
            ..base
        },
        (MachineModel::Nc3000, 3, 9) => ButtonStyle {
            face: 0xD98BB9,
            text: 0x171B72,
            ..base
        },
        _ => base,
    }
}

fn numeric_legend(def: &KeyDef) -> Option<&'static str> {
    match (def.drow, def.dcol) {
        (2, 4..=6) => Some(["7", "8", "9"][(def.dcol - 4) as usize]),
        (3, 4..=6) => Some(["4", "5", "6"][(def.dcol - 4) as usize]),
        (4, 4..=6) => Some(["1", "2", "3"][(def.dcol - 4) as usize]),
        (5, 4) => Some("0"),
        (5, 5) => Some("."),
        _ => None,
    }
}

fn number_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Pc1000 => 0x55C9C8,
        MachineModel::Cc800 => 0xE8C45D,
        MachineModel::Nc1020 => 0x006CD8,
        MachineModel::Nc2000 => 0x5936D4,
        MachineModel::Nc3000 => 0x63BFC5,
    }
}

fn number_text_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Nc1020 | MachineModel::Nc2000 => 0xFFFFFF,
        _ => 0xF7F7F5,
    }
}

fn navigation_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Pc1000 | MachineModel::Cc800 | MachineModel::Nc3000 => 0xED8BC8,
        MachineModel::Nc1020 => 0xDD1E42,
        MachineModel::Nc2000 => 0xFF6D13,
    }
}

fn arrow_direction(label: &str) -> Option<(isize, isize)> {
    match label {
        "LT" => Some((-1, 0)),
        "RT" => Some((1, 0)),
        "UP" | "PGUP" => Some((0, -1)),
        "DN" | "PGDN" => Some((0, 1)),
        _ => None,
    }
}

fn draw_branding(canvas: &mut Canvas, model: MachineModel, palette: Palette) {
    let branding = format!("WENQUXING {}", model.name().to_ascii_uppercase());
    let (x, y) = match model {
        MachineModel::Cc800 => (820, 570),
        MachineModel::Nc3000 => (310, 78),
        MachineModel::Pc1000 => (300, 610),
        MachineModel::Nc1020 => (300, 610),
        MachineModel::Nc2000 => (770, 860),
    };
    canvas.text_centered(&branding, x, y, 4, palette.trim);
    canvas.text_centered("ELECTRONIC DICTIONARY", 810, 770, 3, palette.trim);
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
        },
        MachineModel::Cc800 => Palette {
            shell: 0xD3D5D4,
            shell_light: 0xF5F5F2,
            shell_dark: 0x999D9D,
            trim: 0x252B30,
            key: 0x303840,
            key_text: 0xFFFFFF,
        },
        MachineModel::Nc1020 => Palette {
            shell: 0xE6E7E7,
            shell_light: 0xFFFFFF,
            shell_dark: 0xAEB3B7,
            trim: 0x0066C5,
            key: 0xE4E4E2,
            key_text: 0x151719,
        },
        MachineModel::Nc2000 => Palette {
            shell: 0xDFE0E2,
            shell_light: 0xF9F9F7,
            shell_dark: 0xA9ACB9,
            trim: 0x737DB6,
            key: 0xE7E7E5,
            key_text: 0x151719,
        },
        MachineModel::Nc3000 => Palette {
            shell: 0xD0D2D3,
            shell_light: 0xF2F2F0,
            shell_dark: 0x8D9194,
            trim: 0x4E5357,
            key: 0x343B40,
            key_text: 0xF7F7F5,
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

    fn rounded_rect_right(&mut self, rect: Rect, radius: usize, color: u32) {
        let x1 = (rect.x + rect.width).min(SOURCE_WIDTH);
        let y1 = (rect.y + rect.height).min(SOURCE_HEIGHT);
        let radius = radius.min(rect.width / 2).min(rect.height / 2);
        let split_x = rect.x + rect.width / 2;
        for y in rect.y.min(SOURCE_HEIGHT)..y1 {
            for x in split_x.min(SOURCE_WIDTH)..x1 {
                let dx = x.saturating_sub(rect.x + rect.width - radius - 1);
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

    fn triangle(
        &mut self,
        center_x: usize,
        center_y: usize,
        radius: usize,
        direction: (isize, isize),
        color: u32,
    ) {
        let center_x = center_x as isize;
        let center_y = center_y as isize;
        let radius = radius as isize;
        let perpendicular = (-direction.1, direction.0);
        let tip = (
            center_x + direction.0 * radius,
            center_y + direction.1 * radius,
        );
        let base_center = (
            center_x - direction.0 * radius,
            center_y - direction.1 * radius,
        );
        let points = [
            tip,
            (
                base_center.0 + perpendicular.0 * radius,
                base_center.1 + perpendicular.1 * radius,
            ),
            (
                base_center.0 - perpendicular.0 * radius,
                base_center.1 - perpendicular.1 * radius,
            ),
        ];
        for y in (center_y - radius).max(0)..=(center_y + radius).min(SOURCE_HEIGHT as isize - 1) {
            for x in (center_x - radius).max(0)..=(center_x + radius).min(SOURCE_WIDTH as isize - 1)
            {
                let edge = |first: (isize, isize), second: (isize, isize)| {
                    (x - second.0) * (first.1 - second.1) - (first.0 - second.0) * (y - second.1)
                };
                let signs = [
                    edge(points[0], points[1]),
                    edge(points[1], points[2]),
                    edge(points[2], points[0]),
                ];
                if signs.iter().all(|sign| *sign >= 0) || signs.iter().all(|sign| *sign <= 0) {
                    self.pixels[y as usize * SOURCE_WIDTH + x as usize] = color;
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
