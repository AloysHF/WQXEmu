use image::{imageops::FilterType, ImageBuffer, Rgb};
use wqxemu_core::{KeyDef, MachineModel};

use super::{key_region, Rect, SOURCE_HEIGHT, SOURCE_WIDTH};

#[path = "font.rs"]
mod font;

const WINDOW_BACKGROUND: u32 = 0xF0F0F0;

const WHITE: u32 = 0xFFFFFF;
const BLACK: u32 = 0x000000;

const SZ_SMALL: u8 = 10;
const SZ_TAG: u8 = 12;
const SZ_TEXT: u8 = 14;
const SZ_KEY: u8 = 16;
const SZ_INFO: u8 = 20;
const SZ_NUM: u8 = 24;
const SZ_LOGO: u8 = 28;

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
    canvas.gradient_rounded_rect(
        Rect {
            x: 36,
            y: 28,
            width: 1014,
            height: 1392,
        },
        36,
        0x2566CE,
        0x1A4FA8,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 52,
            y: 44,
            width: 982,
            height: 648,
        },
        26,
        0xF7F7F5,
        0xE9EAE8,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 52,
            y: 716,
            width: 982,
            height: 688,
        },
        30,
        0xF5F5F3,
        0xE4E5E3,
    );

    canvas.gradient_rounded_rect(
        Rect {
            x: 70,
            y: 84,
            width: 830,
            height: 452,
        },
        18,
        0xB4B4B4,
        0xA2A2A2,
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

    canvas.gradient_rounded_rect(
        Rect {
            x: 70,
            y: 742,
            width: 946,
            height: 178,
        },
        16,
        0xC5C7C8,
        0xB4B6B7,
    );
    canvas.dot_grid((112, 764), 20, 10, 16, 4, 0x4E5254);
    canvas.text_centered("www.ggv.com.cn", 790, 898, SZ_TEXT, 0x85898D);

    canvas.text_centered("NC1020", 175, 60, SZ_LOGO, 0x1E5BC6);
    canvas.text_centered("文曲星", 225, 622, 48, 0x85898D);
    canvas.text_centered("®", 305, 588, SZ_SMALL, 0x85898D);
    canvas.rounded_rect(Rect::centered(490, 622, 132, 44), 22, 0xFFFFFF);
    canvas.text_centered("真人发音", 490, 622, SZ_KEY, 0x5A5E62);

    draw_keys(canvas, MachineModel::Nc1020, layout);

    draw_aux_button(
        canvas,
        Rect::centered(105, 950, 82, 52),
        0xFD8630,
        0x502800,
        "发音",
        SZ_TEXT,
    );
    draw_aux_button(
        canvas,
        Rect::centered(203, 950, 82, 52),
        0xFD8630,
        0x502800,
        "报时",
        SZ_TEXT,
    );
    canvas.text_centered("RESET", 295, 934, SZ_TEXT, 0x3A3E42);
    canvas.circle(295, 968, 8, 0x2456B0);

    canvas.ring(966, 560, 26, 4, 0xCDE6F4);
    canvas.text_centered("ON/OFF", 966, 616, SZ_TAG, 0x1E4E78);

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x1E5BC6);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x0E3E90);
    canvas.rounded_rect(Rect::centered(105, 1408, 92, 16), 6, 0x1E5BC6);
    canvas.rounded_rect(Rect::centered(981, 1408, 92, 16), 6, 0x1E5BC6);
}

fn draw_pc1000(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
    canvas.gradient_rounded_rect(
        Rect {
            x: 34,
            y: 26,
            width: 1018,
            height: 1396,
        },
        40,
        0x26292C,
        0x141618,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 50,
            y: 42,
            width: 986,
            height: 650,
        },
        30,
        0xCDCDCD,
        0xBFBFC0,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 50,
            y: 714,
            width: 986,
            height: 694,
        },
        32,
        0xCFCFD0,
        0xC0C1C1,
    );

    let bezel = Rect {
        x: screen.x - 16,
        y: screen.y - 16,
        width: screen.width + 32,
        height: screen.height + 62,
    };
    canvas.gradient_rounded_rect(bezel, 10, 0x3B3E41, 0x2E3134);
    canvas.rounded_rect(screen, 4, 0x94AA76);
    draw_screen_numbers(canvas, screen, screen.y + screen.height + 27, 0xB4B7B9);

    for (index, (y, label)) in [(150, "同反义"), (232, "变化"), (314, "辨析"), (396, "例句")]
        .into_iter()
        .enumerate()
    {
        canvas.text_centered(label, 852, y, SZ_TEXT, 0x3A3E42);
        draw_aux_button(
            canvas,
            Rect::centered(932, y, 78, 30),
            0x4A4E52,
            0x9FC4E8,
            &format!("F{}", index + 1),
            SZ_KEY,
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
    canvas.text_centered("文曲星", 240, 640, 34, 0xF2F2F0);
    canvas.text_centered("e1000", 430, 640, SZ_LOGO, 0x3E9BD8);
    canvas.text_centered("真人发音", 838, 494, SZ_KEY, 0x3A3E42);
    canvas.text_centered("Human Intonation", 838, 518, SZ_SMALL, 0x6E7276);
    canvas.text_centered("www.ggv.com.cn", 745, 550, SZ_TEXT, 0x85898D);

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
    canvas.circle_slats((185, 852), 98, 8, 10, 0x3A3D40);

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

    canvas.text_centered("ON/OFF", 344, 739, SZ_TAG, 0x2F5FA8);

    draw_keys(canvas, MachineModel::Pc1000, layout);

    draw_aux_button(
        canvas,
        Rect::centered(380, 933, 82, 50),
        0xF6AE28,
        0x5E4200,
        "发音",
        SZ_TEXT,
    );
    draw_aux_button(
        canvas,
        Rect::centered(478, 933, 82, 50),
        0xF6AE28,
        0x5E4200,
        "报时",
        SZ_TEXT,
    );

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x17191B);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x3A3D40);
}

fn draw_cc800(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
    canvas.gradient_rounded_rect(
        Rect {
            x: 40,
            y: 30,
            width: 1006,
            height: 1390,
        },
        34,
        0x44474A,
        0x35383B,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 56,
            y: 46,
            width: 974,
            height: 644,
        },
        26,
        0xD4D5D5,
        0xC6C7C7,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 56,
            y: 714,
            width: 974,
            height: 688,
        },
        28,
        0xCBCDCD,
        0xBEC0C0,
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

    for (y, label) in [(130, "报文"), (210, "变化"), (290, "事件"), (370, "闹钟")] {
        draw_aux_button(
            canvas,
            Rect::centered(908, y, 88, 30),
            0xF4F4F2,
            0x3A3E42,
            label,
            SZ_TAG,
        );
        canvas.triangle(852, y, 8, (1, 0), 0x3A3E42);
    }
    canvas.dot_grid((124, 534), 9, 2, 26, 3, 0x7E8286);
    canvas.dot_grid((124, 600), 9, 2, 26, 3, 0x7E8286);
    canvas.text_centered("文曲星", 790, 548, 40, 0x3A3E42);
    canvas.text_centered("®", 858, 518, SZ_SMALL, 0x55595C);
    canvas.text_centered("CC800", 900, 612, SZ_NUM, 0x3A3E42);
    canvas.text_centered("GOLDEN GLOBAL VIEW", 690, 572, SZ_SMALL, 0x2F5FA8);
    canvas.text_centered("www.ggv.com.cn", 660, 606, SZ_TAG, 0x7E8286);

    draw_hinge(canvas, 0xB9BBBC, 0xE9E9E7, 0x9A9DA1);

    draw_keys(canvas, MachineModel::Cc800, layout);

    canvas.circle(120, 902, 7, 0x3A3E42);
    canvas.text_centered("RESET", 186, 902, SZ_TEXT, 0x4A4E50);
    canvas.text_centered("ON/OFF", 978, 902, SZ_TEXT, 0x3A3E42);

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x2E3134);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x17191B);
}

fn draw_nc2000(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
    canvas.gradient_rounded_rect(
        Rect {
            x: 40,
            y: 28,
            width: 1006,
            height: 674,
        },
        30,
        0x9AA8DC,
        0x8898D0,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 54,
            y: 42,
            width: 978,
            height: 646,
        },
        24,
        0xC2C8EA,
        0xB4BBE4,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 40,
            y: 718,
            width: 1006,
            height: 702,
        },
        30,
        0xCFD2D8,
        0xC2C5CC,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 54,
            y: 732,
            width: 978,
            height: 674,
        },
        24,
        0xEAEAEA,
        0xDFDFE0,
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
    canvas.text_centered("www.ggv.com.cn", 866, 788, SZ_TEXT, 0x6E7276);
    canvas.text_centered("文曲星", 790, 862, 34, 0x5A5E6A);
    canvas.text_centered("NC2000A", 940, 862, SZ_INFO, 0x5A5E6A);
    canvas.dot_grid((640, 900), 12, 1, 22, 2, 0xB9BCC4);

    canvas.text_centered("文曲星", 285, 650, 34, 0x5A5E6A);
    canvas.rounded_rect(Rect::centered(505, 650, 122, 40), 20, 0xFFFFFF);
    canvas.text_centered("真人发音", 505, 650, SZ_TEXT, 0x5A5E6A);

    draw_keys(canvas, MachineModel::Nc2000, layout);

    draw_aux_button(
        canvas,
        Rect::centered(112, 946, 76, 48),
        0xF68729,
        0x502800,
        "发音",
        SZ_TEXT,
    );
    draw_aux_button(
        canvas,
        Rect::centered(208, 946, 76, 48),
        0xF68729,
        0x502800,
        "报时",
        SZ_TEXT,
    );
    canvas.text_centered("RESET", 305, 932, SZ_TEXT, 0x3A3E42);
    canvas.circle(305, 962, 8, 0x2F55B4);

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x9A9EA6);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x6E7276);
}

fn draw_nc3000(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
    canvas.gradient_rounded_rect(
        Rect {
            x: 38,
            y: 28,
            width: 1010,
            height: 1394,
        },
        34,
        0xA4A7AA,
        0x96999C,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 54,
            y: 44,
            width: 978,
            height: 646,
        },
        26,
        0xDBDCDD,
        0xCFCFD0,
    );
    canvas.gradient_rounded_rect(
        Rect {
            x: 54,
            y: 716,
            width: 978,
            height: 690,
        },
        28,
        0xCFD1D2,
        0xC2C4C5,
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

    for (index, (y, label)) in [
        (160, "同反义"),
        (232, "根查字"),
        (304, "解析"),
        (376, "例句"),
    ]
    .into_iter()
    .enumerate()
    {
        canvas.text_centered(label, 950, y, SZ_TEXT, 0x5A5E62);
        canvas.text_centered(&format!("F{}", index + 1), 1000, y, SZ_TEXT, 0x5A5E62);
    }
    canvas.rounded_rect(Rect::centered(178, 598, 150, 86), 10, 0xC8CACC);
    canvas.rounded_rect(Rect::centered(178, 598, 142, 78), 8, 0xFFFFFF);
    canvas.text_centered("剑桥", 178, 574, SZ_TEXT, 0xC02020);
    canvas.text_centered("AHD", 178, 596, SZ_TEXT, 0xC02020);
    canvas.rounded_rect(Rect::centered(150, 620, 34, 22), 2, 0x2F55B4);
    canvas.rounded_rect(Rect::centered(194, 620, 34, 22), 2, 0xC02020);

    canvas.text_centered("文曲星", 245, 84, 22, 0x9A9EA2);
    canvas.text_centered("NC3000", 385, 84, SZ_INFO, 0x9A9EA2);
    canvas.text_centered("Electronic dictionary", 543, 664, SZ_INFO, 0x8E9296);

    draw_hinge(canvas, 0xA06438, 0xC9CBCD, 0x9A9DA1);

    canvas.dot_grid((238, 796), 14, 8, 19, 4, 0x9A9EA2);
    canvas.dot_grid((78, 796), 6, 8, 19, 3, 0xC4C7CA);

    draw_keys(canvas, MachineModel::Nc3000, layout);

    canvas.text_centered("ON/OFF", 140, 780, SZ_TAG, 0x2F55B4);
    canvas.circle(132, 956, 28, 0x4E5256);
    canvas.circle(132, 956, 24, 0xC6C8CA);
    canvas.text_centered("网络", 132, 918, SZ_TAG, 0x2F55B4);
    for (x, label) in [(278, "发音"), (358, "报时"), (438, "网络")] {
        draw_aux_button(
            canvas,
            Rect::centered(x, 968, 64, 42),
            0xF6C040,
            0x6E4A00,
            label,
            SZ_TAG,
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
    canvas.text_centered("查看", 872, 782, SZ_TAG, 0x5A5E62);
    canvas.text_centered("A·P", 925, 782, SZ_SMALL, 0x5A5E62);
    canvas.text_centered("翻页", 1022, 890, SZ_TAG, 0x5A5E62);
    canvas.text_centered("发音", 858, 974, SZ_TAG, 0x5A5E62);
    canvas.text_centered("翻译", 926, 974, SZ_TAG, 0x5A5E62);

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
        if def.drow != 5 {
            let letter = def.label.split('/').next().unwrap_or(def.label);
            canvas.text_centered(
                letter,
                region.x + region.width * 27 / 100,
                region.y + region.height / 2,
                SZ_INFO,
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
                region.x + 26,
                region.y + region.height / 2,
                SZ_TAG,
                style.text,
            );
        }
        canvas.text_centered(
            number,
            segment.x + segment.width / 2 + 1,
            region.y + region.height / 2,
            SZ_NUM,
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
        let center_x = region.x + region.width / 2 - if mark.is_empty() { 0 } else { 6 };
        canvas.triangle(center_x, region.y + region.height / 2, 12, direction, color);
        if !mark.is_empty() {
            let size = if mark == "税" { SZ_TAG } else { SZ_SMALL };
            canvas.text_centered(
                mark,
                region.x + region.width / 2 + 16,
                region.y + region.height / 2 + 8,
                size,
                color,
            );
        }
    } else {
        let label = display_label(model, def);
        let size = if matches!(def.label, "ON" | "PWR") {
            SZ_TEXT
        } else {
            SZ_KEY
        };
        canvas.text_centered(
            label,
            region.x + region.width / 2,
            region.y + region.height / 2,
            size,
            style.text,
        );
        if let Some((sub, color)) = key_sublabel(model, def) {
            canvas.text_centered(
                sub,
                region.x + region.width - 20,
                region.y + region.height - 10,
                SZ_SMALL,
                color,
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
                region.x + region.width - 20,
                region.y + region.height - 10,
                SZ_SMALL,
                style.text,
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
            (SZ_SMALL, 35)
        } else {
            (SZ_TAG, 13)
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
                let right = region.x as i32 - 8;
                let label_width = text_width(caption, SZ_TEXT);
                let number = format!("F{}", def.dcol - 1);
                canvas.text_right(
                    caption,
                    right,
                    region.y + region.height / 2,
                    SZ_TEXT,
                    0x3A3E42,
                );
                canvas.text_right(
                    &number,
                    right - label_width - 8,
                    region.y + region.height / 2,
                    SZ_TEXT,
                    0x3A3E42,
                );
            }
            MachineModel::Nc2000 => {
                canvas.text_centered(
                    &format!("F{} {caption}", def.dcol - 1),
                    region.x + region.width / 2,
                    region.y - 13,
                    SZ_TAG,
                    0x2A3C78,
                );
            }
            MachineModel::Pc1000 => {
                canvas.text_centered(
                    &format!("F{}", def.dcol - 1),
                    region.x + region.width / 2,
                    region.y - 13,
                    SZ_SMALL,
                    0x9FC4E8,
                );
            }
            MachineModel::Cc800 => {
                canvas.text_centered(
                    caption,
                    region.x + region.width / 2,
                    region.y - 15,
                    SZ_TAG,
                    0x9B2A20,
                );
            }
            MachineModel::Nc3000 => {
                canvas.text_centered(
                    caption,
                    region.x + region.width / 2,
                    region.y - 15,
                    SZ_TAG,
                    0x3A3E42,
                );
            }
        }
    }
    if let Some((superscript, color)) = key_superscript(model, def) {
        let size = if superscript.chars().any(|ch| ch as u32 > 0x7F) {
            SZ_TAG
        } else {
            SZ_SMALL
        };
        canvas.text_centered(
            superscript,
            region.x + region.width / 2,
            region.y - 13,
            size,
            color,
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

fn draw_aux_button(canvas: &mut Canvas, region: Rect, face: u32, text: u32, label: &str, size: u8) {
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
        canvas.text_centered(&number.to_string(), x, y, SZ_NUM, color);
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
            shape: ButtonShape::Rect(6),
        },
        (MachineModel::Nc1020, 1, 2..=5) => ButtonStyle {
            face: 0xA5CEE3,
            border: 0x4E7896,
            text: 0x1E4E78,
            shape: ButtonShape::Rect(6),
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
            shape: ButtonShape::Rect(6),
        },
        (MachineModel::Nc1020, 5, 0) => ButtonStyle {
            face: 0xDCDBD9,
            border: 0x6E7072,
            text: 0xC0202C,
            shape: ButtonShape::Rect(6),
        },
        (MachineModel::Pc1000, 0, 6) => ButtonStyle {
            face: 0xF6AE28,
            border: 0x8E6208,
            text: 0x5E4200,
            shape: ButtonShape::Rect(13),
        },
        (MachineModel::Pc1000, 1, 2..=5) => ButtonStyle {
            face: 0x4A4F58,
            border: 0x14171B,
            text: 0xF2F4F6,
            shape: ButtonShape::Rect(13),
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
            shape: ButtonShape::Rect(13),
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
            face: 0xABC378,
            border: 0x5E7A34,
            text: 0x24350E,
            shape: ButtonShape::Rect(6),
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
            shape: ButtonShape::Rect(6),
        },
        (MachineModel::Nc2000, 5, 0) => ButtonStyle {
            face: 0xD8D9D7,
            border: 0x6E7276,
            text: 0xE06010,
            shape: ButtonShape::Rect(6),
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
        (_, 5, 6) => Some(("-", color)),
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
            (MachineModel::Nc2000, 3) => Some(("录音", color)),
            (_, 1) => Some(("SHIFT", color)),
            (_, 2) => Some(("CAPS", color)),
            _ => None,
        };
    }
    let latin = match (def.drow, def.dcol) {
        (2, 0) => "sin-1",
        (2, 1) => "cos-1",
        (2, 2) => "tan-1",
        (2, 3) => "hyp",
        (2, 8) => "#",
        (3, 0) => "10x",
        (3, 1) => "ex",
        (3, 3) => "x2",
        (3, 7) => "?",
        (3, 8) => "*",
        (4, 0) => ")",
        (4, 1) => "x!",
        _ => return None,
    };
    if model == MachineModel::Cc800 && (def.drow, def.dcol) == (2, 2) {
        return None;
    }
    Some((latin, color))
}

fn superscript_color(model: MachineModel) -> u32 {
    match model {
        MachineModel::Nc3000 => 0x2F55B4,
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
        (2, 9) => "MC",
        (3, 0) => "log",
        (3, 1) => "ln",
        (3, 2) => "xy",
        (3, 8) => "x",
        (5, 3) => "AC",
        (5, 6) => "=",
        _ => return None,
    };
    let color = if (def.drow, def.dcol) == (5, 3)
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
        MachineModel::Cc800 => 0xD8B45A,
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
        center: (usize, usize),
        radius: usize,
        slat_height: usize,
        gap: usize,
        color: u32,
    ) {
        let mut y = center.1.saturating_sub(radius) + gap;
        while y + slat_height < center.1 + radius {
            let dy = (center.1 as isize - (y + slat_height / 2) as isize).unsigned_abs();
            let half_width = if dy >= radius {
                0
            } else {
                ((radius * radius - dy * dy) as f64).sqrt() as usize
            };
            if half_width > gap {
                self.rounded_rect(
                    Rect {
                        x: center.0 - half_width + gap,
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

    fn text_right(&mut self, text: &str, right_x: i32, center_y: usize, size: u8, color: u32) {
        let width = text_width(text, size);
        let Some(top) = ink_top(text, size) else {
            return;
        };
        let bottom = ink_bottom(text, size);
        self.draw_text(
            text,
            right_x - width,
            center_y as i32 - (top + bottom) / 2,
            size,
            color,
        );
    }

    fn draw_text(&mut self, text: &str, x: i32, baseline_y: i32, size: u8, color: u32) {
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
