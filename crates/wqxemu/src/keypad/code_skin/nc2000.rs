//! NC2000 code-drawn device skin.

use wqxemu_core::{KeyDef, MachineModel};

use super::{
    draw_aux_button, draw_hinge, draw_keys, draw_screen_numbers, Canvas, Rect, SZ_INFO, SZ_TEXT,
};

pub(super) fn draw(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
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
