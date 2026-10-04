//! Code-drawn device skins for all supported models.
//!
//! Each model's shell drawing lives in its own module so changes to one
//! device skin cannot affect another. Shared rendering infrastructure
//! (Canvas, key drawing, text) stays here.

use image::{imageops::FilterType, ImageBuffer, Rgb};
use wqxemu_core::{KeyDef, MachineModel};

use super::{key_region, Rect, SOURCE_HEIGHT, SOURCE_WIDTH};

mod font;

mod cc800;
mod nc1020;
mod nc2000;
mod nc3000;
mod pc1000;

pub(super) const WINDOW_BACKGROUND: u32 = 0xF0F0F0;
pub(super) const WHITE: u32 = 0xFFFFFF;
pub(super) const BLACK: u32 = 0x000000;

pub(super) const SZ_SMALL: u8 = 10;
pub(super) const SZ_TAG: u8 = 12;
pub(super) const SZ_TEXT: u8 = 14;
pub(super) const SZ_KEY: u8 = 16;
pub(super) const SZ_INFO: u8 = 20;
pub(super) const SZ_NUM: u8 = 24;
pub(super) const SZ_LOGO: u8 = 28;

#[derive(Clone, Copy)]
pub(super) enum ButtonShape {
    Rect(usize),
    Capsule,
    Circle,
}

#[derive(Clone, Copy)]
pub(super) struct ButtonStyle {
    pub(super) face: u32,
    pub(super) border: u32,
    pub(super) text: u32,
    pub(super) shape: ButtonShape,
}

pub(super) fn render(
    model: MachineModel,
    width: usize,
    height: usize,
    screen: Rect,
    layout: &[KeyDef],
) -> Vec<u32> {
    let mut canvas = Canvas::new();
    draw_device(&mut canvas, model, screen, layout);
    canvas.resize(width, height)
}

fn draw_device(canvas: &mut Canvas, model: MachineModel, screen: Rect, layout: &[KeyDef]) {
    match model {
        MachineModel::Nc1020 => nc1020::draw(canvas, screen, layout),
        MachineModel::Nc2000 => nc2000::draw(canvas, screen, layout),
        MachineModel::Nc3000 => nc3000::draw(canvas, screen, layout),
        MachineModel::Pc1000 => pc1000::draw(canvas, screen, layout),
        MachineModel::Cc800 => cc800::draw(canvas, screen, layout),
    }
}

fn draw_keys(canvas: &mut Canvas, model: MachineModel, layout: &[KeyDef]) {
    for def in layout {
        if let Some(region) = key_region(model, def) {
            draw_key(canvas, model, region, def);
        }
    }
}

fn draw_key(canvas: &mut Canvas, model: MachineModel, region: Rect, def: &KeyDef) {
    let style = key_style(model, def);
    let (inset, radius) = draw_styled_button(canvas, region, style);

    if let Some(number) = numeric_legend(def) {
        let seg_color = segment_color(model);
        // Colored segment covers the right ~40% of the key face. The left
        // edge is flat (a clean split against the dark half) and the right
        // edge follows the key's rounded corner.
        let seg_width = inset.width * 40 / 100;
        let segment = Rect {
            x: inset.x + inset.width - seg_width,
            y: inset.y + 1,
            width: seg_width - 1,
            height: inset.height - 2,
        };
        canvas.gradient_rounded_rect(
            segment,
            radius.saturating_sub(2).max(2),
            mix(seg_color, WHITE, 5, 1),
            mix(seg_color, BLACK, 8, 1),
        );
        // Square off the segment's left edge so the split stays vertical.
        canvas.gradient_rounded_rect(
            Rect {
                x: segment.x,
                y: segment.y,
                width: 3,
                height: segment.height,
            },
            0,
            mix(seg_color, WHITE, 5, 1),
            mix(seg_color, BLACK, 8, 1),
        );
        if def.drow != 5 {
            let letter = def.label.split('/').next().unwrap_or(def.label);
            canvas.text_centered(
                letter,
                region.x + region.width * 25 / 100,
                region.y + region.height / 2,
                SZ_LOGO,
                style.text,
            );
        }
        if def.drow == 5 && def.dcol == 4 {
            let symbol = if model == MachineModel::Nc3000 {
                "零点"
            } else {
                "符号"
            };
            canvas.text_centered(
                symbol,
                region.x + 24,
                region.y + region.height / 2,
                SZ_TAG,
                style.text,
            );
        }
        if def.drow == 5 && def.dcol == 5 {
            // Dot key shows "·" on the left and "•" on the segment.
            canvas.text_centered(
                "·",
                region.x + region.width * 25 / 100,
                region.y + region.height / 2,
                SZ_NUM,
                style.text,
            );
        }
        canvas.text_centered(
            number,
            segment.x + segment.width / 2 + 1,
            region.y + region.height / 2,
            SZ_LOGO,
            segment_text_color(model),
        );
    } else if let Some(direction) = arrow_direction(def.label) {
        let color = arrow_color(model, def.label);
        let mark = match def.label {
            "PGUP" => "税",
            "UP" => "-",
            "PGDN" => "M-",
            "RT" => "M+",
            "DN" if model != MachineModel::Nc1020 => "+",
            _ => "",
        };
        // Mark text uses a separate color: dark on light bodies, light on dark keys.
        let mark_color = match (def.label, model) {
            ("PGUP", MachineModel::Cc800 | MachineModel::Pc1000 | MachineModel::Nc3000) => 0xF0F0F0,
            ("PGUP", _) => 0x2A2E32,
            _ => sublabel_color(model),
        };
        // PGUP/PGDN are page-style keys and use double arrowheads;
        // the cursor keys (UP/DN/LT/RT) use a single arrowhead.
        let double = matches!(def.label, "PGUP" | "PGDN");
        let center_x = region.x + region.width / 2 - if mark.is_empty() { 0 } else { 6 };
        let center_y = region.y + region.height / 2;
        if double {
            let (dx, dy) = direction;
            canvas.triangle(
                (center_x as i32 - dx as i32 * 10).max(0) as usize,
                (center_y as i32 - dy as i32 * 10).max(0) as usize,
                10,
                direction,
                color,
            );
            canvas.triangle(
                (center_x as i32 + dx as i32 * 10).max(0) as usize,
                (center_y as i32 + dy as i32 * 10).max(0) as usize,
                10,
                direction,
                color,
            );
        } else {
            canvas.triangle(center_x, center_y, 12, direction, color);
        }
        if !mark.is_empty() {
            let size = if mark == "税" { SZ_TAG } else { SZ_SMALL };
            canvas.text_centered(
                mark,
                region.x + region.width / 2 + 16,
                region.y + region.height / 2 + 8,
                size,
                mark_color,
            );
        }
    } else {
        let label = display_label(model, def);
        let has_sub = key_sublabel(model, def).is_some();
        let is_cjk = label.chars().any(|ch| ch as u32 > 0x7F);
        let size = if matches!(def.label, "ON" | "PWR") {
            SZ_TEXT
        } else if has_sub && !is_cjk {
            SZ_LOGO
        } else if def.drow == 0 {
            // Hotkey row (英汉/名片/...): larger face text to match the reference.
            SZ_INFO
        } else {
            SZ_KEY
        };
        if has_sub && !is_cjk {
            // Latin two-function key: primary letter on the left,
            // vertically centered; secondary label at the lower-right.
            canvas.text_centered(
                label,
                region.x + region.width * 30 / 100,
                region.y + region.height / 2,
                size,
                style.text,
            );
            if let Some((sub, color)) = key_sublabel(model, def) {
                canvas.text_centered(
                    sub,
                    region.x + region.width * 70 / 100,
                    region.y + region.height * 72 / 100,
                    SZ_INFO,
                    color,
                );
            }
        } else if has_sub {
            // CJK two-function key: centered main label with a
            // lower-right subscript, matching the device silk-screen.
            canvas.text_centered(
                label,
                region.x + region.width / 2,
                region.y + region.height * 40 / 100,
                size,
                style.text,
            );
            if let Some((sub, color)) = key_sublabel(model, def) {
                canvas.text_centered(
                    sub,
                    region.x + region.width * 70 / 100,
                    region.y + region.height * 72 / 100,
                    SZ_KEY,
                    color,
                );
            }
        } else {
            canvas.text_centered(
                label,
                region.x + region.width / 2,
                region.y + region.height / 2,
                size,
                style.text,
            );
        }
        if def.label == "ENT" {
            let tail = if model == MachineModel::Cc800 {
                "MB"
            } else {
                "MR"
            };
            canvas.text_centered(
                tail,
                region.x + region.width * 70 / 100,
                region.y + region.height * 70 / 100,
                SZ_INFO,
                sublabel_color(model),
            );
        }
    }

    draw_key_captions(canvas, model, region, def);
}

/// Small colored captions printed on the shell beside a key: the category
/// labels above the hotkeys, the F-key captions, and the scientific
/// superscripts above the keyboard rows.
fn draw_key_captions(canvas: &mut Canvas, model: MachineModel, region: Rect, def: &KeyDef) {
    if let Some((category, color)) = category_label(model, def) {
        let (size, offset) = if model == MachineModel::Nc3000 {
            (SZ_INFO, 12)
        } else {
            (SZ_INFO, 16)
        };
        canvas.text_centered(
            category,
            region.x + region.width / 2,
            region.y - offset,
            size,
            color,
        );
    }
    if let Some(caption) = f_key_caption(model, def) {
        match model {
            MachineModel::Nc1020 => {
                // White pill button to the LEFT of the blue F key.
                let pill_captions = ["同反义", "变化", "解析", "例句"];
                let pill = pill_captions[(def.dcol - 2) as usize];
                draw_aux_button(
                    canvas,
                    Rect::centered(856, region.y + region.height / 2, 78, 28),
                    0xFAFAF8,
                    0x3A3E42,
                    pill,
                    SZ_TAG,
                );
                // Label text ("F1 插入") above the blue button, shifted right.
                let label = format!("F{} {}", def.dcol - 1, caption);
                canvas.text_centered(
                    &label,
                    region.x + region.width / 2 + 8,
                    region.y - 16,
                    SZ_INFO,
                    0x3A3E42,
                );
            }
            MachineModel::Nc2000 => {
                canvas.text_centered(
                    &format!("F{} {caption}", def.dcol - 1),
                    region.x + region.width / 2,
                    region.y - 16,
                    SZ_INFO,
                    0x2A3C78,
                );
            }
            MachineModel::Pc1000 => {
                canvas.text_centered(
                    &format!("F{}", def.dcol - 1),
                    region.x + region.width / 2,
                    region.y - 16,
                    SZ_INFO,
                    0x1A3298,
                );
            }
            MachineModel::Cc800 => {
                canvas.text_centered(
                    caption,
                    region.x + region.width / 2,
                    region.y - 18,
                    SZ_INFO,
                    0x9B2A20,
                );
            }
            MachineModel::Nc3000 => {
                canvas.text_centered(
                    caption,
                    region.x + region.width / 2,
                    region.y - 18,
                    SZ_INFO,
                    0x3A3E42,
                );
            }
        }
    }
    if let Some((superscript, color)) = key_superscript(model, def) {
        canvas.text_centered(
            superscript,
            region.x + region.width / 2,
            region.y - 8,
            SZ_INFO,
            color,
        );
    }
}

pub(super) fn draw_styled_button(
    canvas: &mut Canvas,
    region: Rect,
    style: ButtonStyle,
) -> (Rect, usize) {
    let radius = match style.shape {
        ButtonShape::Rect(radius) => radius,
        ButtonShape::Capsule | ButtonShape::Circle => region.height / 2,
    };
    let shadow = Rect {
        x: region.x + 3,
        y: region.y + 4,
        ..region
    };
    canvas.rounded_rect(shadow, radius, mix(style.border, 0xAAAAAA, 1, 1));
    canvas.rounded_rect(region, radius, style.border);
    let inset = Rect {
        x: region.x + 3,
        y: region.y + 3,
        width: region.width - 6,
        height: region.height - 7,
    };
    let inner_radius = radius.saturating_sub(2).max(2);
    canvas.gradient_rounded_rect(
        inset,
        inner_radius,
        mix(style.face, WHITE, 5, 1),
        mix(style.face, BLACK, 9, 1),
    );
    // Specular highlight: a thin line across the button face. For round
    // buttons it sits at the vertical center (matching the device silk);
    // for rectangular keys it stays near the top edge.
    let spec_y = match style.shape {
        ButtonShape::Circle => inset.y + inset.height / 2,
        _ => inset.y + 2,
    };
    canvas.rounded_rect(
        Rect {
            x: inset.x + 4,
            y: spec_y,
            width: inset.width - 8,
            height: 2,
        },
        1,
        mix(style.face, WHITE, 7, 1),
    );
    (inset, inner_radius)
}

pub(super) fn draw_aux_button(
    canvas: &mut Canvas,
    region: Rect,
    face: u32,
    text: u32,
    label: &str,
    size: u8,
) {
    draw_styled_button(
        canvas,
        region,
        ButtonStyle {
            face,
            border: mix(face, BLACK, 2, 1),
            text,
            shape: ButtonShape::Rect(8),
        },
    );
    canvas.text_centered(
        label,
        region.x + region.width / 2,
        region.y + region.height / 2,
        size,
        text,
    );
}

pub(super) fn draw_hinge(canvas: &mut Canvas, drum: u32, bar: u32, notch: u32) {
    // Silver cylinder body with colored end caps.
    canvas.rounded_rect(Rect::centered(165, 712, 214, 66), 26, bar);
    canvas.rounded_rect(Rect::centered(75, 712, 60, 66), 26, drum);
    canvas.rounded_rect(Rect::centered(255, 712, 60, 66), 26, drum);
    canvas.rounded_rect(Rect::centered(921, 712, 214, 66), 26, bar);
    canvas.rounded_rect(Rect::centered(831, 712, 60, 66), 26, drum);
    canvas.rounded_rect(Rect::centered(1011, 712, 60, 66), 26, drum);
    canvas.rounded_rect(Rect::centered(543, 712, 470, 30), 12, bar);
    canvas.rounded_rect(Rect::centered(543, 712, 130, 14), 6, notch);
}

pub(super) fn draw_screen_numbers(canvas: &mut Canvas, screen: Rect, y: usize, color: u32) {
    for number in 1..=9 {
        let x = screen.x + screen.width * number / 10;
        canvas.text_centered(&number.to_string(), x, y, SZ_NUM, color);
    }
}

fn key_style(model: MachineModel, def: &KeyDef) -> ButtonStyle {
    let base = match model {
        MachineModel::Nc1020 => ButtonStyle {
            face: 0xDCD8D0,
            border: 0x8E8878,
            text: 0x2A2418,
            shape: ButtonShape::Rect(8),
        },
        MachineModel::Nc2000 => ButtonStyle {
            face: 0xF8F8F8,
            border: 0x2A2E32,
            text: 0x1B1D1F,
            shape: ButtonShape::Rect(3),
        },
        MachineModel::Pc1000 => ButtonStyle {
            face: 0x4A4F58,
            border: 0x14171B,
            text: 0xF2F4F6,
            shape: ButtonShape::Rect(13),
        },
        MachineModel::Cc800 => ButtonStyle {
            face: 0x262A30,
            border: 0x0C0E10,
            text: 0xF5F6F7,
            shape: ButtonShape::Rect(13),
        },
        MachineModel::Nc3000 => ButtonStyle {
            face: 0x4A4E54,
            border: 0x14161A,
            text: 0xF2F4F6,
            shape: ButtonShape::Rect(13),
        },
    };
    match (model, def.drow, def.dcol) {
        (MachineModel::Nc1020, 0, _) => ButtonStyle {
            face: 0xB0A8C2,
            border: 0x5A5470,
            text: 0x2E2A3A,
            shape: ButtonShape::Rect(8),
        },
        (MachineModel::Nc1020, 1, 2..=5) => ButtonStyle {
            face: 0xA8D0E4,
            border: 0x4E7896,
            text: 0x1E4E78,
            shape: ButtonShape::Rect(8),
        },
        (MachineModel::Nc1020, 1, 8) => ButtonStyle {
            face: 0xA8D0E4,
            border: 0x4E7896,
            text: 0x1E4E78,
            shape: ButtonShape::Circle,
        },
        (MachineModel::Nc1020, 3, 9) => ButtonStyle {
            face: 0xF89040,
            border: 0xA85410,
            text: 0x502800,
            shape: ButtonShape::Rect(8),
        },
        (MachineModel::Nc1020, 5, 0) => ButtonStyle {
            face: 0xE8E0D0,
            border: 0x8E8878,
            text: 0xC0202C,
            shape: ButtonShape::Rect(8),
        },
        (MachineModel::Pc1000, 0, 6) => ButtonStyle {
            face: 0xF6AE28,
            border: 0x8E6208,
            text: 0xFFFFFF,
            shape: ButtonShape::Capsule,
        },
        (MachineModel::Pc1000, 1, 2..=5) => ButtonStyle {
            face: 0x4A4F58,
            border: 0x14171B,
            text: 0xF2F4F6,
            shape: ButtonShape::Capsule,
        },
        (MachineModel::Pc1000, 1, 8) => ButtonStyle {
            face: 0xF6E0D6,
            border: 0x1A1C1E,
            text: 0x2F5FA8,
            shape: ButtonShape::Capsule,
        },
        (MachineModel::Pc1000, 3, 9) => ButtonStyle {
            face: 0xFDB930,
            border: 0x8E6208,
            text: 0xFFFFFF,
            shape: ButtonShape::Capsule,
        },
        (MachineModel::Pc1000, 5, 0) => ButtonStyle {
            face: 0x4A4F58,
            border: 0x14171B,
            text: 0xE884B4,
            shape: ButtonShape::Rect(13),
        },
        (MachineModel::Cc800, 0, 0..=5) => ButtonStyle {
            face: 0x0BA78B,
            border: 0x065844,
            text: 0xF2FFFB,
            shape: ButtonShape::Capsule,
        },
        (MachineModel::Cc800, 0, 6) => ButtonStyle {
            face: 0x0BA78B,
            border: 0x065844,
            text: 0xF2FFFB,
            shape: ButtonShape::Circle,
        },
        (MachineModel::Cc800, 1, 2..=5) => ButtonStyle {
            face: 0xF2C231,
            border: 0x8E6E08,
            text: 0x8E2418,
            shape: ButtonShape::Circle,
        },
        (MachineModel::Cc800, 1, 8) => ButtonStyle {
            face: 0xBE4470,
            border: 0x6E1E38,
            text: 0xF6DCE6,
            shape: ButtonShape::Circle,
        },
        (MachineModel::Cc800, 3, 9) => ButtonStyle {
            face: 0x0BA78B,
            border: 0x065844,
            text: 0xF2FFFB,
            shape: ButtonShape::Rect(13),
        },
        (MachineModel::Nc2000, 0, _) => ButtonStyle {
            face: 0x3A9A78,
            border: 0x2A2E32,
            text: 0xF0FFF8,
            shape: ButtonShape::Rect(3),
        },
        (MachineModel::Nc2000, 1, 2..=5) | (MachineModel::Nc2000, 1, 8) => ButtonStyle {
            face: 0xF0F0F0,
            border: 0x2A2E32,
            text: 0x2A3C78,
            shape: ButtonShape::Rect(3),
        },
        (MachineModel::Nc2000, 3, 9) => ButtonStyle {
            face: 0x5FC0C8,
            border: 0x2A2E32,
            text: 0x0F3540,
            shape: ButtonShape::Rect(3),
        },
        (MachineModel::Nc2000, 5, 0) => ButtonStyle {
            face: 0xF0F0F0,
            border: 0x2A2E32,
            text: 0xE06010,
            shape: ButtonShape::Rect(3),
        },
        (MachineModel::Nc3000, 1, 2..=5) => ButtonStyle {
            face: 0xBEC1C3,
            border: 0x4E5256,
            text: 0x3A3E42,
            shape: ButtonShape::Circle,
        },
        (MachineModel::Nc3000, 1, 8) => ButtonStyle {
            face: 0xC2C4C6,
            border: 0x4E5256,
            text: 0x8E9296,
            shape: ButtonShape::Circle,
        },
        (MachineModel::Nc3000, 3, 9) => ButtonStyle {
            face: 0xE0A2C2,
            border: 0x9E4E6E,
            text: 0x7E2450,
            shape: ButtonShape::Rect(13),
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
        (5, 5) => Some("•"),
        _ => None,
    }
}

fn segment_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Nc1020 => 0x1A5ECC,
        MachineModel::Pc1000 => 0x6FD1CF,
        MachineModel::Cc800 => 0xE8C040,
        MachineModel::Nc2000 => 0x4048C8,
        MachineModel::Nc3000 => 0x7AC8CE,
    }
}

fn segment_text_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Pc1000 | MachineModel::Nc3000 | MachineModel::Cc800 => 0x1A1A1A,
        _ => 0xFFFFFF,
    }
}

fn arrow_color(model: MachineModel, _label: &str) -> u32 {
    match model {
        MachineModel::Nc1020 => 0xC4202C,
        MachineModel::Nc2000 => 0xF08228,
        _ => 0xE884B4,
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

/// The label printed on a key face, using the real device silk-screen text.
fn display_label(model: MachineModel, def: &KeyDef) -> &'static str {
    if def.drow == 0 {
        return match (model, def.dcol) {
            (MachineModel::Nc1020, 0) => "英汉",
            (MachineModel::Nc1020, 1) => "名片",
            (MachineModel::Nc1020, 2) => "计算",
            (MachineModel::Nc1020, 3) => "行程",
            (MachineModel::Nc1020, 4) => "测验",
            (MachineModel::Nc1020, 5) => "时间",
            (MachineModel::Nc1020, 6) => "网络",
            (MachineModel::Nc2000, 0) => "英汉",
            (MachineModel::Nc2000, 1) => "名片",
            (MachineModel::Nc2000, 2) => "计算",
            (MachineModel::Nc2000, 3) => "行程",
            (MachineModel::Nc2000, 4) => "测验",
            (MachineModel::Nc2000, 5) => "时间",
            (MachineModel::Nc2000, 6) => "网络",
            (MachineModel::Pc1000 | MachineModel::Cc800, 0) => "英汉",
            (MachineModel::Pc1000 | MachineModel::Cc800, 1) => "名片",
            (MachineModel::Pc1000 | MachineModel::Cc800, 2) => "计算",
            (MachineModel::Pc1000 | MachineModel::Cc800, 3) => "提醒",
            (MachineModel::Pc1000 | MachineModel::Cc800, 4) => "资料",
            (MachineModel::Pc1000 | MachineModel::Cc800, 5) => "时间",
            (MachineModel::Pc1000 | MachineModel::Cc800, 6) => "网络",
            (MachineModel::Nc3000, 0) => "PDA",
            (MachineModel::Nc3000, 1) => "计算",
            (MachineModel::Nc3000, 2) => "时间",
            (MachineModel::Nc3000, 3) => "英汉",
            (MachineModel::Nc3000, 4) => "AHD",
            (MachineModel::Nc3000, 5) => "剑桥",
            _ => def.label,
        };
    }
    match (model, def.drow, def.dcol) {
        (MachineModel::Cc800, 1, 8) => "",
        (MachineModel::Nc2000, 1, 8) => "",
        (MachineModel::Pc1000, 1, 8) => "",
        (MachineModel::Nc2000, 1, 2..=5) => "",
        (MachineModel::Nc3000, 1, 8) => "",
        (MachineModel::Nc3000, 1, 2..=5) => match def.dcol {
            2 => "F1",
            3 => "F2",
            4 => "F3",
            _ => "F4",
        },
        (MachineModel::Pc1000, 1, 2) => "插入",
        (MachineModel::Pc1000, 1, 3) => "删除",
        (MachineModel::Pc1000, 1, 4) => "查找",
        (MachineModel::Pc1000, 1, 5) => "修改",
        (_, 1, 2..=5) => match def.dcol {
            2 => "F1",
            3 => "F2",
            4 => "F3",
            _ => "F4",
        },
        (_, 3, 9) => "输入",
        (_, 5, 0) => match model {
            MachineModel::Nc2000 => "变焦",
            MachineModel::Nc3000 => "帮助",
            _ => "求助",
        },
        (_, 5, 1) => match model {
            MachineModel::Nc3000 => "中英度",
            _ => "中英数",
        },
        (_, 5, 2) => "输入法",
        (_, 5, 3) => match model {
            MachineModel::Nc3000 => "删除",
            _ => "跳出",
        },
        (_, 5, 6) => "空格",
        _ => def.label,
    }
}

/// The colored caption printed on the shell above a hotkey.
fn category_label(model: MachineModel, def: &KeyDef) -> Option<(&'static str, u32)> {
    let entries: &[&str] = match model {
        MachineModel::Nc1020 => &["汉英", "通讯", "换算", "资料", "游戏", "其他"],
        MachineModel::Nc2000 => &["汉英", "通讯", "换算", "记事", "游戏", "其他"],
        MachineModel::Pc1000 | MachineModel::Cc800 => {
            &["汉英", "记事", "换算", "测验", "游戏", "其他"]
        }
        MachineModel::Nc3000 => &["游戏", "换算", "系统", "汉英", "词库", "学习"],
    };
    let color = category_color(model);
    match (model, def.drow, def.dcol) {
        (MachineModel::Nc3000, 0, column @ 0..=5) => Some((entries[column as usize], color)),
        (MachineModel::Nc3000, _, _) => None,
        (MachineModel::Cc800, 0, 6) => Some(("网络", color)),
        (_, 0, column @ 0..=5) => Some((entries[column as usize], color)),
        _ => None,
    }
}

fn category_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Nc1020 => 0xC0202C,
        MachineModel::Cc800 => 0x3A3E42,
        _ => 0x2F55B4,
    }
}

/// The function-key caption shown beside or above the F1-F4 buttons.
fn f_key_caption(model: MachineModel, def: &KeyDef) -> Option<&'static str> {
    if def.drow != 1 || !matches!(def.dcol, 2..=5) {
        return None;
    }
    Some(match (model, def.dcol) {
        (_, 2) => "插入",
        (_, 3) => "删除",
        (MachineModel::Nc1020, 4) => "查设",
        (_, 4) => "查找",
        _ => "修改",
    })
}

fn key_superscript(model: MachineModel, def: &KeyDef) -> Option<(&'static str, u32)> {
    let color = superscript_color(model);
    if def.drow == 5 {
        return match (model, def.dcol) {
            (MachineModel::Nc2000, 2) => Some(("反查CAPS", color)),
            (MachineModel::Nc2000, 4) => Some(("录音", color)),
            (MachineModel::Cc800, 4) => Some(("继 续", color)),
            (MachineModel::Nc3000, 4) => Some(("双解", color)),
            (MachineModel::Nc3000, 5) => Some(("英解 −", color)),
            (MachineModel::Nc3000, 6) => Some(("汉解 √", color)),
            (_, 1) => Some(("SHIFT", color)),
            (_, 2) => Some(("CAPS", color)),
            (_, 5) => Some(("−", color)),
            (_, 6) => Some(("√", color)),
            _ => None,
        };
    }
    let latin = match (def.drow, def.dcol) {
        (2, 0) => "sin⁻¹",
        (2, 1) => "cos⁻¹",
        (2, 2) => "tan⁻¹",
        (2, 3) => "hyp",
        (2, 8) => "#",
        (2, 9) => "合",
        (3, 0) => "10ˣ",
        (3, 1) => "eˣ",
        (3, 2) => "ʸ√x",
        (3, 3) => "x²",
        (3, 7) => "?",
        (3, 8) => "*",
        (4, 0) => ")",
        (4, 1) => "x!",
        (4, 2) => "° //”",
        _ => return None,
    };
    Some((latin, color))
}

fn superscript_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Nc3000 | MachineModel::Nc2000 => 0x2A3C78,
        MachineModel::Pc1000 => 0x1A3298,
        _ => 0xC0202C,
    }
}

fn key_sublabel(model: MachineModel, def: &KeyDef) -> Option<(&'static str, u32)> {
    let label = match (def.drow, def.dcol) {
        (2, 0) => "sin",
        (2, 1) => "cos",
        (2, 2) => "tan",
        (2, 3) => "1/x",
        (2, 7) => "%",
        (2, 8) => "÷",
        (2, 9) => "MC",
        (3, 0) => "log",
        (3, 1) => "ln",
        (3, 2) => "xʸ",
        (3, 3) => "√",
        (3, 7) => "±",
        (3, 8) => "x",
        (4, 0) => "(",
        (4, 1) => "π",
        (4, 2) => "EXP",
        (4, 3) => "c",
        (5, 3) => "AC",
        (5, 6) => "=",
        _ => return None,
    };
    // V key's "c" subscript is red on the reference device.
    let color = if matches!((def.drow, def.dcol), (5, 3) | (4, 3))
        && matches!(model, MachineModel::Nc1020 | MachineModel::Nc2000)
    {
        0xC0202C
    } else {
        sublabel_color(model)
    };
    Some((label, color))
}

fn sublabel_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Nc1020 | MachineModel::Nc2000 => 0x2456B0,
        MachineModel::Pc1000 => 0x8ED4D2,
        MachineModel::Cc800 => 0x6FA8E0,
        MachineModel::Nc3000 => 0x7AC8CE,
    }
}

fn text_width(text: &str, size: u8) -> i32 {
    let mut width = 0;
    let count = text.chars().count();
    for (index, ch) in text.chars().enumerate() {
        match font::glyph(ch, size) {
            Some(glyph) => {
                if index + 1 == count {
                    width += glyph.x_offset + glyph.width as i32;
                } else {
                    width += glyph.advance;
                }
            }
            None => width += size as i32 / 2,
        }
    }
    width
}

pub(super) fn mix(first: u32, second: u32, first_weight: u32, second_weight: u32) -> u32 {
    let total = first_weight + second_weight;
    let channel = |shift: u32| {
        (((first >> shift) & 0xFFu32) * first_weight
            + ((second >> shift) & 0xFFu32) * second_weight)
            / total
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

pub(super) struct Canvas {
    pixels: Vec<u32>,
}

impl Canvas {
    pub(super) fn new() -> Self {
        Self {
            pixels: vec![WINDOW_BACKGROUND; SOURCE_WIDTH * SOURCE_HEIGHT],
        }
    }

    pub(super) fn rounded_rect(&mut self, rect: Rect, radius: usize, color: u32) {
        self.paint_rounded_rect(rect, radius, |_| color);
    }

    pub(super) fn gradient_rounded_rect(
        &mut self,
        rect: Rect,
        radius: usize,
        top: u32,
        bottom: u32,
    ) {
        let span = rect.height.max(1).saturating_sub(1);
        self.paint_rounded_rect(rect, radius, |row| {
            let weight = row.min(span);
            mix(top, bottom, (span - weight) as u32, weight as u32)
        });
    }

    fn paint_rounded_rect(&mut self, rect: Rect, radius: usize, color: impl Fn(usize) -> u32) {
        let x1 = (rect.x + rect.width).min(SOURCE_WIDTH);
        let y1 = (rect.y + rect.height).min(SOURCE_HEIGHT);
        let radius = radius.min(rect.width / 2).min(rect.height / 2);
        for y in rect.y.min(SOURCE_HEIGHT)..y1 {
            let color = color(y.saturating_sub(rect.y));
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

    pub(super) fn circle(&mut self, center_x: usize, center_y: usize, radius: usize, color: u32) {
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

    pub(super) fn ring(
        &mut self,
        center_x: usize,
        center_y: usize,
        radius: usize,
        thickness: usize,
        color: u32,
    ) {
        let inner = radius.saturating_sub(thickness);
        let rect = Rect::centered(center_x, center_y, radius * 2 + 1, radius * 2 + 1);
        for y in rect.y..(rect.y + rect.height).min(SOURCE_HEIGHT) {
            for x in rect.x..(rect.x + rect.width).min(SOURCE_WIDTH) {
                let dx = x as isize - center_x as isize;
                let dy = y as isize - center_y as isize;
                let distance = (dx * dx + dy * dy) as usize;
                if distance <= radius * radius && distance > inner * inner {
                    self.pixels[y * SOURCE_WIDTH + x] = color;
                }
            }
        }
    }

    /// Speaker grille: dark recessed interior with raised metallic bars.
    /// Each bar gets a bright top highlight and a shaded bottom edge so the
    /// grille reads as stamped metal like the reference device.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn circle_grille(
        &mut self,
        center: (usize, usize),
        radius: usize,
        bar_height: usize,
        gap: usize,
        bar: u32,
        highlight: u32,
        shade: u32,
    ) {
        self.circle(center.0, center.1, radius, shade);
        let mut y = center.1.saturating_sub(radius) + gap;
        while y + bar_height < center.1 + radius {
            let dy = (center.1 as isize - (y + bar_height / 2) as isize).unsigned_abs();
            let half_width = if dy >= radius {
                0
            } else {
                ((radius * radius - dy * dy) as f64).sqrt() as usize
            };
            if half_width > gap {
                let x = center.0 - half_width + gap;
                let width = (half_width - gap) * 2;
                // Bar face with a bright top edge and a darker underside.
                // The ends stay nearly square: the circle clips them.
                self.rounded_rect(
                    Rect {
                        x,
                        y,
                        width,
                        height: bar_height,
                    },
                    3,
                    bar,
                );
                self.rounded_rect(
                    Rect {
                        x: x + 1,
                        y,
                        width: width.saturating_sub(2),
                        height: 2,
                    },
                    1,
                    highlight,
                );
                self.rounded_rect(
                    Rect {
                        x: x + 1,
                        y: y + bar_height.saturating_sub(2),
                        width: width.saturating_sub(2),
                        height: 2,
                    },
                    1,
                    mix(bar, shade, 1, 2),
                );
            }
            y += bar_height + gap;
        }
    }

    pub(super) fn dot_grid(
        &mut self,
        origin: (usize, usize),
        columns: usize,
        rows: usize,
        spacing: usize,
        radius: usize,
        color: u32,
    ) {
        for row in 0..rows {
            for column in 0..columns {
                self.circle(
                    origin.0 + column * spacing,
                    origin.1 + row * spacing,
                    radius,
                    color,
                );
            }
        }
    }

    pub(super) fn triangle(
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

    pub(super) fn line(
        &mut self,
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
        thickness: usize,
        color: u32,
    ) {
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

    pub(super) fn text_centered(
        &mut self,
        text: &str,
        center_x: usize,
        center_y: usize,
        size: u8,
        color: u32,
    ) {
        let width = text_width(text, size);
        let Some(top) = ink_top(text, size) else {
            return;
        };
        let bottom = ink_bottom(text, size);
        self.draw_text(
            text,
            center_x as i32 - width / 2,
            center_y as i32 - (top + bottom) / 2,
            size,
            color,
        );
    }

    pub(super) fn draw_text(&mut self, text: &str, x: i32, baseline_y: i32, size: u8, color: u32) {
        let mut pen = x;
        for ch in text.chars() {
            match font::glyph(ch, size) {
                Some(glyph) => {
                    self.draw_glyph(
                        glyph,
                        pen + glyph.x_offset,
                        baseline_y + glyph.y_offset,
                        color,
                    );
                    pen += glyph.advance;
                }
                None => pen += size as i32 / 2,
            }
        }
    }

    fn draw_glyph(&mut self, glyph: &font::Glyph, x: i32, y: i32, color: u32) {
        for row in 0..glyph.height {
            let pixel_y = y + row as i32;
            if pixel_y < 0 || pixel_y >= SOURCE_HEIGHT as i32 {
                continue;
            }
            for column in 0..glyph.width {
                let alpha = glyph.alpha[row * glyph.width + column] as u32;
                if alpha == 0 {
                    continue;
                }
                let pixel_x = x + column as i32;
                if pixel_x < 0 || pixel_x >= SOURCE_WIDTH as i32 {
                    continue;
                }
                let index = pixel_y as usize * SOURCE_WIDTH + pixel_x as usize;
                let background = self.pixels[index];
                self.pixels[index] = mix(color, background, alpha, 255 - alpha);
            }
        }
    }

    /// Draw a glyph rotated 90 degrees so ring side labels match the device
    /// silk-screen (character tops facing outward). `clockwise` picks the
    /// rotation direction: true = tops face right, false = tops face left.
    /// `(x, y)` is the top-left of the rotated glyph's bounding box.
    fn draw_glyph_rotated(
        &mut self,
        glyph: &font::Glyph,
        x: i32,
        y: i32,
        color: u32,
        clockwise: bool,
    ) {
        let width = glyph.width as i32;
        let height = glyph.height as i32;
        for row in 0..glyph.height {
            for column in 0..glyph.width {
                let alpha = glyph.alpha[row * glyph.width + column] as u32;
                if alpha == 0 {
                    continue;
                }
                // Original (column, row) maps into a height x width box.
                let (rotated_x, rotated_y) = if clockwise {
                    (height - 1 - row as i32, column as i32)
                } else {
                    (row as i32, width - 1 - column as i32)
                };
                let pixel_x = x + rotated_x;
                let pixel_y = y + rotated_y;
                if pixel_x < 0
                    || pixel_x >= SOURCE_WIDTH as i32
                    || pixel_y < 0
                    || pixel_y >= SOURCE_HEIGHT as i32
                {
                    continue;
                }
                let index = pixel_y as usize * SOURCE_WIDTH + pixel_x as usize;
                let background = self.pixels[index];
                self.pixels[index] = mix(color, background, alpha, 255 - alpha);
            }
        }
    }

    /// Draw text with each character rotated 90 degrees and stacked
    /// vertically, centered on (center_x, center_y). Used for the D-pad
    /// ring side labels.
    pub(super) fn text_vertical(
        &mut self,
        text: &str,
        center_x: usize,
        center_y: usize,
        size: u8,
        color: u32,
        clockwise: bool,
    ) {
        let glyphs: Vec<&font::Glyph> = text
            .chars()
            .filter_map(|ch| font::glyph(ch, size))
            .collect();
        if glyphs.is_empty() {
            return;
        }
        // Each rotated character's vertical extent is its upright advance.
        let total_height: i32 = glyphs.iter().map(|glyph| glyph.advance).sum();
        let mut pen_y = center_y as i32 - total_height / 2;
        for glyph in glyphs {
            let rotated_width = glyph.height as i32;
            let rotated_height = glyph.width as i32;
            let x = center_x as i32 - rotated_width / 2;
            let y = pen_y + (glyph.advance - rotated_height) / 2;
            self.draw_glyph_rotated(glyph, x, y, color, clockwise);
            pen_y += glyph.advance;
        }
    }

    /// Draw a glyph rotated around its ink center by `angle` radians
    /// (clockwise positive in screen coordinates).
    fn draw_glyph_at_angle(
        &mut self,
        glyph: &font::Glyph,
        center_x: f32,
        center_y: f32,
        color: u32,
        angle: f32,
    ) {
        let cos = angle.cos();
        let sin = angle.sin();
        let half_width = glyph.width as f32 / 2.0;
        let half_height = glyph.height as f32 / 2.0;
        for row in 0..glyph.height {
            for column in 0..glyph.width {
                let alpha = glyph.alpha[row * glyph.width + column] as u32;
                if alpha == 0 {
                    continue;
                }
                let dx = column as f32 - half_width + 0.5;
                let dy = row as f32 - half_height + 0.5;
                // Clockwise rotation in screen coordinates (y grows downward).
                let rotated_x = dx * cos - dy * sin;
                let rotated_y = dx * sin + dy * cos;
                let pixel_x = (center_x + rotated_x) as i32;
                let pixel_y = (center_y + rotated_y) as i32;
                if pixel_x < 0
                    || pixel_x >= SOURCE_WIDTH as i32
                    || pixel_y < 0
                    || pixel_y >= SOURCE_HEIGHT as i32
                {
                    continue;
                }
                let index = pixel_y as usize * SOURCE_WIDTH + pixel_x as usize;
                let background = self.pixels[index];
                self.pixels[index] = mix(color, background, alpha, 255 - alpha);
            }
        }
    }

    /// Draw text along a circular arc so the silk-screen curves like the
    /// reference device. Each character's ink center sits on the circle of
    /// `radius` around (center_x, center_y) and is rotated to follow the
    /// tangent. `top` selects the upper or lower arc; character tops face
    /// away from the circle on the top arc and toward it on the bottom arc,
    /// matching how the printed labels read.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn text_arc(
        &mut self,
        text: &str,
        center_x: usize,
        center_y: usize,
        radius: f32,
        size: u8,
        color: u32,
        top: bool,
    ) {
        let glyphs: Vec<&font::Glyph> = text
            .chars()
            .filter_map(|ch| font::glyph(ch, size))
            .collect();
        if glyphs.is_empty() {
            return;
        }
        let total: f32 = glyphs.iter().map(|glyph| glyph.advance as f32).sum();
        let mut pen = -total / 2.0;
        for glyph in glyphs {
            let offset = pen + glyph.advance as f32 / 2.0;
            // Angular position from the vertical, positive toward the right.
            let theta = offset / radius;
            let (sin, cos) = (theta.sin(), theta.cos());
            let (position_x, position_y, rotation) = if top {
                (
                    center_x as f32 + radius * sin,
                    center_y as f32 - radius * cos,
                    theta,
                )
            } else {
                (
                    center_x as f32 + radius * sin,
                    center_y as f32 + radius * cos,
                    -theta,
                )
            };
            self.draw_glyph_at_angle(glyph, position_x, position_y, color, rotation);
            pen += glyph.advance as f32;
        }
    }

    pub(super) fn resize(self, width: usize, height: usize) -> Vec<u32> {
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

fn ink_top(text: &str, size: u8) -> Option<i32> {
    text.chars()
        .filter_map(|ch| font::glyph(ch, size))
        .map(|glyph| glyph.y_offset)
        .min()
}

fn ink_bottom(text: &str, size: u8) -> i32 {
    text.chars()
        .filter_map(|ch| font::glyph(ch, size))
        .map(|glyph| glyph.y_offset + glyph.height as i32)
        .max()
        .unwrap_or(0)
}
