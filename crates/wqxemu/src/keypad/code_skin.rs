use image::{imageops::FilterType, ImageBuffer, Rgb};
use wqxemu_core::{KeyDef, MachineModel};

use super::{key_region, Rect, SOURCE_HEIGHT, SOURCE_WIDTH};

const WINDOW_BACKGROUND: u32 = 0xF0F0F0;

const WHITE: u32 = 0xFFFFFF;
const BLACK: u32 = 0x000000;

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
    let mut canvas = Canvas::new();
    draw_device(&mut canvas, model, screen, layout);
    canvas.resize(width, height)
}

fn draw_device(canvas: &mut Canvas, model: MachineModel, screen: Rect, layout: &[KeyDef]) {
    match model {
        MachineModel::Nc1020 => draw_nc1020(canvas, screen, layout),
        MachineModel::Nc2000 => draw_nc2000(canvas, screen, layout),
        MachineModel::Nc3000 => draw_nc3000(canvas, screen, layout),
        MachineModel::Pc1000 => draw_pc1000(canvas, screen, layout),
        MachineModel::Cc800 => draw_cc800(canvas, screen, layout),
    }
}

fn draw_nc1020(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
    canvas.rounded_rect(
        Rect {
            x: 36,
            y: 28,
            width: 1014,
            height: 1392,
        },
        36,
        0x1E5BC6,
    );
    canvas.rounded_rect(
        Rect {
            x: 52,
            y: 44,
            width: 982,
            height: 648,
        },
        26,
        0xF2F2F0,
    );
    canvas.rounded_rect(
        Rect {
            x: 52,
            y: 716,
            width: 982,
            height: 688,
        },
        30,
        0xF2F2F0,
    );

    canvas.rounded_rect(
        Rect {
            x: 70,
            y: 84,
            width: 830,
            height: 452,
        },
        18,
        0xADADAD,
    );
    let bezel = Rect {
        x: screen.x - 14,
        y: screen.y - 14,
        width: screen.width + 28,
        height: screen.height + 30,
    };
    canvas.rounded_rect(bezel, 8, 0x2E3236);
    canvas.rounded_rect(screen, 4, 0x96AC79);
    draw_screen_numbers(canvas, screen, 496, 0x3A3E42);
    canvas.line(70, 560, 1010, 560, 2, 0xD5D5D3);

    draw_hinge(canvas, 0x1A50AC, 0xC9CBCD, 0x9A9DA1);

    canvas.rounded_rect(
        Rect {
            x: 70,
            y: 742,
            width: 946,
            height: 178,
        },
        16,
        0xBFC1C2,
    );
    canvas.dot_grid((112, 764), 20, 10, 16, 4, 0x4E5254);
    canvas.text_centered("WWW.GGV.COM.CN", 790, 898, 3, 0x85898D);

    canvas.text_centered("NC1020", 175, 60, 5, 0x1E5BC6);
    canvas.text_centered("WENQUXING", 240, 622, 6, 0x85898D);
    canvas.rounded_rect(Rect::centered(475, 622, 130, 44), 22, 0xFFFFFF);
    canvas.text_centered("VOICE", 475, 622, 3, 0x6E7276);

    draw_keys(canvas, MachineModel::Nc1020, layout);

    draw_aux_button(
        canvas,
        Rect::centered(105, 950, 82, 52),
        0xFD8630,
        0x502800,
        "VOICE",
        2,
    );
    draw_aux_button(
        canvas,
        Rect::centered(203, 950, 82, 52),
        0xFD8630,
        0x502800,
        "TIME",
        3,
    );
    canvas.text_centered("RESET", 295, 934, 3, 0x3A3E42);
    canvas.circle(295, 968, 8, 0x2456B0);

    canvas.ring(966, 560, 26, 4, 0xCDE6F4);
    canvas.text_centered("ON/OFF", 966, 616, 2, 0x1E4E78);

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x1E5BC6);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x0E3E90);
    canvas.rounded_rect(Rect::centered(105, 1408, 92, 16), 6, 0x1E5BC6);
    canvas.rounded_rect(Rect::centered(981, 1408, 92, 16), 6, 0x1E5BC6);
}

fn draw_pc1000(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
    canvas.rounded_rect(
        Rect {
            x: 34,
            y: 26,
            width: 1018,
            height: 1396,
        },
        40,
        0x1B1D1F,
    );
    canvas.rounded_rect(
        Rect {
            x: 50,
            y: 42,
            width: 986,
            height: 650,
        },
        30,
        0xC7C7C7,
    );
    canvas.rounded_rect(
        Rect {
            x: 50,
            y: 714,
            width: 986,
            height: 694,
        },
        32,
        0xC9CACA,
    );

    let bezel = Rect {
        x: screen.x - 16,
        y: screen.y - 16,
        width: screen.width + 32,
        height: screen.height + 62,
    };
    canvas.rounded_rect(bezel, 10, 0x35383B);
    canvas.rounded_rect(screen, 4, 0x94AA76);
    draw_screen_numbers(canvas, screen, screen.y + screen.height + 27, 0xB4B7B9);

    for (index, y) in [150, 232, 314, 396].into_iter().enumerate() {
        draw_aux_button(
            canvas,
            Rect::centered(932, y, 78, 30),
            0x4A4E52,
            0x9FC4E8,
            &format!("F{}", index + 1),
            3,
        );
    }
    canvas.rounded_rect(
        Rect {
            x: 50,
            y: 588,
            width: 986,
            height: 104,
        },
        22,
        0x2A2D30,
    );
    canvas.text_centered("WENQUXING", 230, 640, 5, 0xF2F2F0);
    canvas.text_centered("PC1000", 510, 640, 5, 0x3E9BD8);
    canvas.text_centered("HUMAN INTONATION", 890, 502, 2, 0x6E7276);
    canvas.text_centered("WWW.GGV.COM.CN", 745, 534, 3, 0x85898D);

    canvas.rounded_rect(Rect::centered(165, 712, 214, 64), 24, 0x313437);
    canvas.rounded_rect(
        Rect {
            x: 272,
            y: 698,
            width: 300,
            height: 28,
        },
        12,
        0xC9CBCD,
    );

    canvas.circle(185, 852, 116, 0x2E3134);
    canvas.circle(185, 852, 106, 0xC4C6C7);
    canvas.circle_slats(185, 852, 98, 8, 10, 0x3A3D40);

    canvas.rounded_rect(
        Rect {
            x: 560,
            y: 690,
            width: 458,
            height: 90,
        },
        34,
        0xB4B6B7,
    );
    canvas.rounded_rect(
        Rect {
            x: 578,
            y: 700,
            width: 422,
            height: 70,
        },
        30,
        0x404346,
    );
    canvas.triangle(650, 735, 13, (1, 0), 0xE8E8E8);
    canvas.rounded_rect(Rect::centered(790, 735, 22, 22), 3, 0xE8E8E8);
    canvas.circle(950, 735, 9, 0xC02020);

    canvas.text_centered("ON/OFF", 344, 739, 2, 0x2F5FA8);

    draw_keys(canvas, MachineModel::Pc1000, layout);

    draw_aux_button(
        canvas,
        Rect::centered(380, 933, 82, 50),
        0xF6AE28,
        0x5E4200,
        "VOICE",
        2,
    );
    draw_aux_button(
        canvas,
        Rect::centered(478, 933, 82, 50),
        0xF6AE28,
        0x5E4200,
        "TIME",
        3,
    );

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x17191B);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x3A3D40);
}

fn draw_cc800(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
    canvas.rounded_rect(
        Rect {
            x: 40,
            y: 30,
            width: 1006,
            height: 1390,
        },
        34,
        0x3E4144,
    );
    canvas.rounded_rect(
        Rect {
            x: 56,
            y: 46,
            width: 974,
            height: 644,
        },
        26,
        0xCFD0D0,
    );
    canvas.rounded_rect(
        Rect {
            x: 56,
            y: 714,
            width: 974,
            height: 688,
        },
        28,
        0xC6C8C8,
    );

    let bezel = Rect {
        x: screen.x - 12,
        y: screen.y - 12,
        width: screen.width + 24,
        height: screen.height + 26,
    };
    canvas.rounded_rect(bezel, 8, 0x2E3134);
    canvas.rounded_rect(screen, 4, 0x94AA76);
    canvas.rounded_rect(
        Rect {
            x: 95,
            y: 420,
            width: 580,
            height: 32,
        },
        8,
        0xB4B6B5,
    );
    draw_screen_numbers(canvas, screen, 436, 0x3A3E42);

    for (index, y) in [130, 210, 290, 370].into_iter().enumerate() {
        draw_aux_button(
            canvas,
            Rect::centered(908, y, 88, 30),
            0xF4F4F2,
            0x3A3E42,
            &format!("F{}", index + 1),
            3,
        );
    }
    canvas.dot_grid((124, 534), 9, 2, 26, 3, 0x7E8286);
    canvas.dot_grid((124, 600), 9, 2, 26, 3, 0x7E8286);
    canvas.text_centered("WENQUXING", 810, 548, 5, 0x55595C);
    canvas.text_centered("CC800", 900, 612, 6, 0x3A3E42);
    canvas.text_centered("WWW.GGV.COM.CN", 660, 606, 2, 0x7E8286);

    draw_hinge(canvas, 0xB9BBBC, 0xE9E9E7, 0x9A9DA1);

    draw_keys(canvas, MachineModel::Cc800, layout);

    canvas.circle(120, 902, 7, 0x3A3E42);
    canvas.text_centered("RESET", 186, 902, 3, 0x4A4E50);
    canvas.text_centered("ON/OFF", 978, 902, 2, 0x3A3E42);

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x2E3134);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x17191B);
}

fn draw_nc2000(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
    canvas.rounded_rect(
        Rect {
            x: 40,
            y: 28,
            width: 1006,
            height: 674,
        },
        30,
        0x93A2D6,
    );
    canvas.rounded_rect(
        Rect {
            x: 54,
            y: 42,
            width: 978,
            height: 646,
        },
        24,
        0xBCC2E7,
    );
    canvas.rounded_rect(
        Rect {
            x: 40,
            y: 718,
            width: 1006,
            height: 702,
        },
        30,
        0xC9CCD4,
    );
    canvas.rounded_rect(
        Rect {
            x: 54,
            y: 732,
            width: 978,
            height: 674,
        },
        24,
        0xE6E6E7,
    );

    canvas.rounded_rect(
        Rect {
            x: 58,
            y: 56,
            width: 970,
            height: 466,
        },
        14,
        0xEDEEF4,
    );
    let bezel = Rect {
        x: screen.x - 12,
        y: screen.y - 12,
        width: screen.width + 24,
        height: screen.height + 28,
    };
    canvas.rounded_rect(bezel, 8, 0x39498C);
    canvas.rounded_rect(screen, 4, 0xA6BA79);
    draw_screen_numbers(canvas, screen, 502, 0x2A3C78);

    draw_hinge(canvas, 0x4A6AC8, 0xC9CBCD, 0x9A9DA1);

    canvas.dot_grid((112, 764), 20, 10, 16, 4, 0x4E5866);
    canvas.text_centered("WWW.GGV.COM.CN", 866, 788, 3, 0x6E7276);
    canvas.text_centered("NC2000", 856, 862, 5, 0x5A5E6A);
    canvas.dot_grid((640, 900), 12, 1, 22, 2, 0xB9BCC4);

    canvas.text_centered("WENQUXING", 285, 650, 5, 0x5A5E6A);
    canvas.rounded_rect(Rect::centered(505, 650, 120, 40), 20, 0xFFFFFF);
    canvas.text_centered("VOICE", 505, 650, 2, 0x5A5E6A);

    draw_keys(canvas, MachineModel::Nc2000, layout);

    draw_aux_button(
        canvas,
        Rect::centered(112, 946, 76, 48),
        0xF68729,
        0x502800,
        "VOICE",
        2,
    );
    draw_aux_button(
        canvas,
        Rect::centered(208, 946, 76, 48),
        0xF68729,
        0x502800,
        "TIME",
        3,
    );
    canvas.text_centered("RESET", 305, 932, 3, 0x3A3E42);
    canvas.circle(305, 962, 8, 0x2F55B4);

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x9A9EA6);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x6E7276);
}

fn draw_nc3000(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
    canvas.rounded_rect(
        Rect {
            x: 38,
            y: 28,
            width: 1010,
            height: 1394,
        },
        34,
        0x9EA1A4,
    );
    canvas.rounded_rect(
        Rect {
            x: 54,
            y: 44,
            width: 978,
            height: 646,
        },
        26,
        0xD7D8D9,
    );
    canvas.rounded_rect(
        Rect {
            x: 54,
            y: 716,
            width: 978,
            height: 690,
        },
        28,
        0xC9CBCC,
    );

    let bezel = Rect {
        x: screen.x - 14,
        y: screen.y - 14,
        width: screen.width + 28,
        height: screen.height + 32,
    };
    canvas.rounded_rect(bezel, 8, 0x3E4246);
    canvas.rounded_rect(screen, 4, 0x8DA36F);
    draw_screen_numbers(canvas, screen, 512, 0x3A3E42);
    canvas.dot_grid((248, 540), 18, 2, 40, 3, 0xB4B7BA);

    for (index, y) in [160, 232, 304, 376].into_iter().enumerate() {
        canvas.text_centered(&format!("F{}", index + 1), 985, y, 3, 0x5A5E62);
    }
    canvas.rounded_rect(Rect::centered(178, 598, 150, 86), 10, 0xC8CACC);
    canvas.rounded_rect(Rect::centered(178, 598, 142, 78), 8, 0xFFFFFF);
    canvas.text_centered("AHD", 178, 578, 3, 0xC02020);
    canvas.rounded_rect(Rect::centered(150, 618, 34, 22), 2, 0x2F55B4);
    canvas.rounded_rect(Rect::centered(194, 618, 34, 22), 2, 0xC02020);

    canvas.text_centered("WENQUXING NC3000", 300, 84, 4, 0x9A9EA2);
    canvas.text_centered("ELECTRONIC DICTIONARY", 543, 664, 4, 0x8E9296);

    draw_hinge(canvas, 0xA06438, 0xC9CBCD, 0x9A9DA1);

    canvas.dot_grid((238, 796), 14, 8, 19, 4, 0x9A9EA2);
    canvas.dot_grid((78, 796), 6, 8, 19, 3, 0xC4C7CA);

    draw_keys(canvas, MachineModel::Nc3000, layout);

    canvas.text_centered("ON/OFF", 140, 780, 2, 0x2F55B4);
    canvas.circle(132, 956, 28, 0x4E5256);
    canvas.circle(132, 956, 24, 0xC6C8CA);
    canvas.text_centered("NET", 132, 918, 2, 0x2F55B4);
    for (x, label) in [(278, "VOICE"), (358, "TIME"), (438, "NET")] {
        draw_aux_button(
            canvas,
            Rect::centered(x, 968, 64, 42),
            0xF6C040,
            0x6E4A00,
            label,
            2,
        );
    }

    canvas.circle(905, 890, 98, 0x8E9296);
    canvas.circle(905, 890, 92, 0xD5D7D8);
    canvas.circle(905, 890, 72, 0xBEC1C3);
    canvas.circle(905, 890, 40, 0x587287);
    canvas.circle(897, 882, 30, 0x66808F);
    canvas.triangle(905, 833, 9, (0, -1), 0x4E5256);
    canvas.triangle(905, 947, 9, (0, 1), 0x4E5256);
    canvas.triangle(848, 890, 9, (-1, 0), 0x4E5256);
    canvas.triangle(962, 890, 9, (1, 0), 0x4E5256);
    canvas.text_centered("A.P", 905, 778, 2, 0x5A5E62);

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x8E9296);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x5A5E62);
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
        let segment_color = segment_color(model);
        let seg_width = inset.width * 45 / 100;
        let segment = Rect {
            x: inset.x + inset.width - seg_width,
            y: inset.y + 2,
            width: seg_width.saturating_sub(2),
            height: inset.height - 4,
        };
        canvas.gradient_rounded_rect_right(
            segment,
            radius.saturating_sub(3).max(2),
            mix(segment_color, WHITE, 5, 1),
            mix(segment_color, BLACK, 9, 1),
        );
        let main_label = if def.drow == 5 {
            ""
        } else {
            def.label.split('/').next().unwrap_or(def.label)
        };
        if !main_label.is_empty() {
            canvas.text_centered(
                main_label,
                region.x + region.width * 27 / 100,
                region.y + region.height / 2,
                5,
                style.text,
            );
        }
        canvas.text_centered(
            number,
            segment.x + segment.width / 2 + 1,
            region.y + region.height / 2,
            5,
            segment_text_color(model),
        );
    } else if let Some(direction) = arrow_direction(def.label) {
        let color = arrow_color(model, def.label);
        let mark = arrow_mark(model, def.label);
        let center_x = region.x + region.width / 2 - if mark.is_some() { 5 } else { 0 };
        canvas.triangle(center_x, region.y + region.height / 2, 12, direction, color);
        if let Some(mark) = mark {
            canvas.text_centered(
                mark,
                region.x + region.width / 2 + 17,
                region.y + region.height / 2 + 8,
                2,
                color,
            );
        }
    } else {
        let label = compact_label(def.label);
        let size = if matches!(def.label, "ON" | "PWR") {
            3
        } else {
            label_size(region, label)
        };
        canvas.text_centered(
            label,
            region.x + region.width / 2,
            region.y + region.height / 2,
            size,
            style.text,
        );
        if let Some(sub) = key_sublabel(def) {
            canvas.text_centered(
                sub,
                region.x + region.width - 19,
                region.y + region.height - 10,
                2,
                sublabel_color(model),
            );
        }
    }

    if let Some(superscript) = key_superscript(model, def) {
        canvas.text_centered(
            superscript,
            region.x + region.width / 2,
            region.y - 11,
            2,
            superscript_color(model),
        );
    }
}

fn draw_styled_button(canvas: &mut Canvas, region: Rect, style: ButtonStyle) -> (Rect, usize) {
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
    canvas.rounded_rect(
        Rect {
            x: inset.x + 4,
            y: inset.y + 2,
            width: inset.width - 8,
            height: 2,
        },
        1,
        mix(style.face, WHITE, 7, 1),
    );
    (inset, inner_radius)
}

fn draw_aux_button(
    canvas: &mut Canvas,
    region: Rect,
    face: u32,
    text: u32,
    label: &str,
    size: usize,
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

fn draw_hinge(canvas: &mut Canvas, drum: u32, bar: u32, notch: u32) {
    canvas.rounded_rect(Rect::centered(165, 712, 214, 66), 26, drum);
    canvas.rounded_rect(Rect::centered(921, 712, 214, 66), 26, drum);
    canvas.rounded_rect(Rect::centered(543, 712, 470, 30), 12, bar);
    canvas.rounded_rect(Rect::centered(543, 712, 130, 14), 6, notch);
}

fn draw_screen_numbers(canvas: &mut Canvas, screen: Rect, y: usize, color: u32) {
    for number in 1..=9 {
        let x = screen.x + screen.width * number / 10;
        canvas.text_centered(&number.to_string(), x, y, 5, color);
    }
}

fn key_style(model: MachineModel, def: &KeyDef) -> ButtonStyle {
    let base = match model {
        MachineModel::Nc1020 => ButtonStyle {
            face: 0xDCDBD9,
            border: 0x6E7072,
            text: 0x1B1D1F,
            shape: ButtonShape::Rect(6),
        },
        MachineModel::Nc2000 => ButtonStyle {
            face: 0xD8D9D7,
            border: 0x6E7276,
            text: 0x1B1D1F,
            shape: ButtonShape::Rect(6),
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
            face: 0xADA3C1,
            border: 0x55506A,
            text: 0x2E2A3A,
            ..base
        },
        (MachineModel::Nc1020, 1, 2..=5) => ButtonStyle {
            face: 0xA5CEE3,
            border: 0x4E7896,
            text: 0x1E4E78,
            ..base
        },
        (MachineModel::Nc1020, 1, 8) => ButtonStyle {
            face: 0xA5CDE2,
            border: 0x4E7896,
            text: 0x1E4E78,
            shape: ButtonShape::Circle,
        },
        (MachineModel::Nc1020, 3, 9) => ButtonStyle {
            face: 0xFD8630,
            border: 0xA85410,
            text: 0x502800,
            ..base
        },
        (MachineModel::Nc1020, 5, 0) => ButtonStyle {
            text: 0xC0202C,
            ..base
        },
        (MachineModel::Pc1000, 0, 6) => ButtonStyle {
            face: 0xF6AE28,
            border: 0x8E6208,
            text: 0x5E4200,
            ..base
        },
        (MachineModel::Pc1000, 1, 2..=5) => ButtonStyle {
            text: 0x9FC4E8,
            ..base
        },
        (MachineModel::Pc1000, 1, 8) => ButtonStyle {
            face: 0xF6E0D6,
            border: 0x9A9DA0,
            text: 0x2F5FA8,
            shape: ButtonShape::Capsule,
        },
        (MachineModel::Pc1000, 3, 9) => ButtonStyle {
            face: 0xFDB930,
            border: 0x8E6208,
            text: 0x5E4000,
            ..base
        },
        (MachineModel::Pc1000, 5, 0) => ButtonStyle {
            text: 0xE884B4,
            ..base
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
            ..base
        },
        (MachineModel::Nc2000, 0, _) => ButtonStyle {
            face: 0xABC378,
            border: 0x5E7A34,
            text: 0x24350E,
            ..base
        },
        (MachineModel::Nc2000, 1, 2..=5) | (MachineModel::Nc2000, 1, 8) => ButtonStyle {
            face: 0xE2E4EC,
            border: 0x6E7694,
            text: 0x2A3C78,
            shape: ButtonShape::Rect(8),
        },
        (MachineModel::Nc2000, 3, 9) => ButtonStyle {
            face: 0x5FC0C8,
            border: 0x2E7A82,
            text: 0x0F3540,
            ..base
        },
        (MachineModel::Nc2000, 5, 0) => ButtonStyle {
            text: 0xE06010,
            ..base
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

fn segment_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Nc1020 => 0x0F56C4,
        MachineModel::Pc1000 => 0x6FD1CF,
        MachineModel::Cc800 => 0xE3BE62,
        MachineModel::Nc2000 => 0x4048C8,
        MachineModel::Nc3000 => 0x7AC8CE,
    }
}

fn segment_text_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Pc1000 => 0x0F3548,
        _ => 0xFFFFFF,
    }
}

fn arrow_color(model: MachineModel, label: &str) -> u32 {
    match model {
        MachineModel::Nc1020 => 0xC4202C,
        MachineModel::Nc2000 if matches!(label, "DN" | "PGDN") => 0x4048C8,
        MachineModel::Nc2000 => 0xF08228,
        _ => 0xE884B4,
    }
}

fn arrow_mark(model: MachineModel, label: &str) -> Option<&'static str> {
    match label {
        "UP" => Some("-"),
        "PGDN" => Some("M-"),
        "RT" => Some("M+"),
        "DN" if model != MachineModel::Nc1020 => Some("+"),
        _ => None,
    }
}

fn key_superscript(model: MachineModel, def: &KeyDef) -> Option<&'static str> {
    if model == MachineModel::Cc800 && (def.drow, def.dcol) == (2, 2) {
        return None;
    }
    match (def.drow, def.dcol) {
        (2, 0) => Some("SIN-1"),
        (2, 1) => Some("COS-1"),
        (2, 2) => Some("TAN-1"),
        (2, 3) => Some("HYP"),
        (2, 8) => Some("#"),
        (3, 0) => Some("10X"),
        (3, 1) => Some("EX"),
        (3, 3) => Some("X2"),
        (3, 7) => Some("?"),
        (3, 8) => Some("*"),
        (4, 0) => Some(")"),
        (4, 1) => Some("X!"),
        (5, 1) => Some("SHIFT"),
        (5, 2) => Some("CAPS"),
        _ => None,
    }
}

fn superscript_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Nc3000 => 0x2F55B4,
        _ => 0xC0202C,
    }
}

fn key_sublabel(def: &KeyDef) -> Option<&'static str> {
    match (def.drow, def.dcol) {
        (2, 0) => Some("sin"),
        (2, 1) => Some("cos"),
        (2, 2) => Some("tan"),
        (2, 3) => Some("1/x"),
        (2, 7) => Some("%"),
        (2, 9) => Some("MC"),
        (3, 0) => Some("log"),
        (3, 1) => Some("ln"),
        (3, 2) => Some("XY"),
        (3, 8) => Some("X"),
        _ => None,
    }
}

fn sublabel_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Nc1020 | MachineModel::Nc2000 => 0x2456B0,
        MachineModel::Pc1000 => 0x8ED4D2,
        MachineModel::Cc800 => 0xD8B45A,
        MachineModel::Nc3000 => 0x7AC8CE,
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
    horizontal.min(vertical).clamp(2, 5)
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
        self.paint_rounded_rect(rect, radius, |_| color);
    }

    fn gradient_rounded_rect(&mut self, rect: Rect, radius: usize, top: u32, bottom: u32) {
        let span = rect.height.max(1).saturating_sub(1);
        self.paint_rounded_rect(rect, radius, |row| {
            let weight = row.min(span);
            mix(top, bottom, (span - weight) as u32, weight as u32)
        });
    }

    fn gradient_rounded_rect_right(&mut self, rect: Rect, radius: usize, top: u32, bottom: u32) {
        let span = rect.height.max(1).saturating_sub(1);
        self.paint_rounded_rect_right(rect, radius, |row| {
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

    fn paint_rounded_rect_right(
        &mut self,
        rect: Rect,
        radius: usize,
        color: impl Fn(usize) -> u32,
    ) {
        let x1 = (rect.x + rect.width).min(SOURCE_WIDTH);
        let y1 = (rect.y + rect.height).min(SOURCE_HEIGHT);
        let radius = radius.min(rect.width / 2).min(rect.height / 2);
        let split_x = rect.x + rect.width / 2;
        for y in rect.y.min(SOURCE_HEIGHT)..y1 {
            let color = color(y.saturating_sub(rect.y));
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

    fn ring(
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

    fn circle_slats(
        &mut self,
        center_x: usize,
        center_y: usize,
        radius: usize,
        slat_height: usize,
        gap: usize,
        color: u32,
    ) {
        let mut y = center_y.saturating_sub(radius) + gap;
        while y + slat_height < center_y + radius {
            let dy = (center_y as isize - (y + slat_height / 2) as isize).unsigned_abs();
            let half_width = if dy >= radius {
                0
            } else {
                ((radius * radius - dy * dy) as f64).sqrt() as usize
            };
            if half_width > gap {
                self.rounded_rect(
                    Rect {
                        x: center_x - half_width + gap,
                        y,
                        width: (half_width - gap) * 2,
                        height: slat_height,
                    },
                    slat_height / 2,
                    color,
                );
            }
            y += slat_height + gap;
        }
    }

    fn dot_grid(
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
        '+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        '*' => [0x0A, 0x04, 0x1F, 0x04, 0x0A, 0x00, 0x00],
        '#' => [0x0A, 0x0A, 0x1F, 0x0A, 0x1F, 0x0A, 0x0A],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '=' => [0x00, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x00],
        '?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        _ => [0; 7],
    }
}
