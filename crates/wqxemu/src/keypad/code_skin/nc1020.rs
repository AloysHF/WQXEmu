//! NC1020 code-drawn device skin.

use wqxemu_core::{KeyDef, MachineModel};

use super::{
    draw_aux_button, draw_hinge, draw_keys, draw_screen_numbers, Canvas, Rect, SZ_KEY, SZ_SMALL,
    SZ_TAG, SZ_TEXT, WHITE,
};

pub(super) fn draw(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
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

    draw_hinge(canvas, 0x2A6ACC, 0xE8EAEA, 0xD0D2D4);

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

    // Model name in dark brand tone at the upper-left of the shell.
    canvas.text_centered("NC1020", 175, 64, 36, 0x141E38);

    // Large calligraphic-style brand text (matches the reference's brush logo size).
    canvas.text_centered("文曲星", 275, 612, 72, 0x2A2E32);
    canvas.text_centered("®", 395, 562, SZ_SMALL, 0x2A2E32);
    canvas.rounded_rect(Rect::centered(545, 618, 152, 48), 24, 0x1A1A1A);
    canvas.text_centered("真人发音", 545, 618, SZ_KEY, WHITE);
    // Decorative dots around the pronunciation badge
    for (dx, dy) in [
        (-82, -8),
        (-82, 8),
        (82, -8),
        (82, 8),
        (-70, -22),
        (70, -22),
        (-70, 22),
        (70, 22),
    ] {
        let x = (545_i32 + dx).max(0) as usize;
        let y = (618_i32 + dy).max(0) as usize;
        canvas.circle(x, y, 2, 0x1A1A1A);
    }

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

    // Large round ON/OFF button with glossy look
    canvas.circle(966, 558, 32, 0x7EACC8);
    canvas.circle(966, 558, 28, 0xB8DAF0);
    canvas.circle(958, 548, 18, 0xD8EEFC);
    canvas.ring(966, 558, 32, 3, 0x5A8AAE);
    canvas.text_centered("ON/OFF", 966, 618, SZ_TAG, 0x1E4E78);

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x1E5BC6);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x0E3E90);
    canvas.rounded_rect(Rect::centered(105, 1408, 92, 16), 6, 0x1E5BC6);
    canvas.rounded_rect(Rect::centered(981, 1408, 92, 16), 6, 0x1E5BC6);
}
