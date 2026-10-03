//! CC800 code-drawn device skin.

use wqxemu_core::{KeyDef, MachineModel};

use super::{
    draw_aux_button, draw_hinge, draw_keys, draw_screen_numbers, Canvas, Rect, SZ_INFO, SZ_NUM,
    SZ_SMALL, SZ_TAG, SZ_TEXT, WHITE,
};

pub(super) fn draw(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
    // Outer shell: dark border
    canvas.gradient_rounded_rect(
        Rect {
            x: 40,
            y: 30,
            width: 1006,
            height: 1390,
        },
        34,
        0x3A3D42,
        0x2A2D32,
    );
    // Upper body: periwinkle / lavender blue
    canvas.gradient_rounded_rect(
        Rect {
            x: 56,
            y: 46,
            width: 974,
            height: 644,
        },
        26,
        0x8E9ACC,
        0x7A86BE,
    );
    // Lower body: silver gray
    canvas.gradient_rounded_rect(
        Rect {
            x: 56,
            y: 714,
            width: 974,
            height: 688,
        },
        28,
        0xD0D2D2,
        0xC2C4C4,
    );

    // LCD bezel and screen
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
        0xC8CACC,
    );
    draw_screen_numbers(canvas, screen, 436, 0x2A2E40);

    // F-key white pill buttons connected to the LCD by thin lines.
    for (y, label) in [(130, "报文"), (210, "变化"), (290, "事件"), (370, "闹钟")] {
        // Connector line from the LCD edge to the pill.
        canvas.line(780, y, 860, y, 2, 0xB0B4B8);
        draw_aux_button(
            canvas,
            Rect::centered(920, y, 88, 30),
            0xF8F8F6,
            0x1A3A8A,
            label,
            SZ_INFO,
        );
    }

    // Silver strip below LCD with ON/OFF, logo, and F-key white buttons
    canvas.rounded_rect(
        Rect {
            x: 70,
            y: 490,
            width: 946,
            height: 190,
        },
        16,
        0xC8CACC,
    );
    canvas.text_centered("ON/OFF", 150, 528, SZ_INFO, 0x3A3E42);
    draw_aux_button(
        canvas,
        Rect::centered(150, 572, 64, 36),
        0xF0F0EE,
        0x3A3E42,
        "",
        SZ_TAG,
    );
    canvas.text_centered("文曲星", 380, 575, 40, 0x3A3E42);
    canvas.text_centered("®", 448, 548, SZ_SMALL, 0x55595C);
    canvas.rounded_rect(Rect::centered(530, 572, 118, 40), 20, 0x1A1A1A);
    canvas.text_centered("真人发音", 530, 572, SZ_INFO, WHITE);
    for (x, label) in [
        (690, "F1插入"),
        (785, "F2删除"),
        (880, "F3查找"),
        (975, "F4修改"),
    ] {
        canvas.text_centered(label, x, 538, SZ_INFO, 0x3A3E42);
        draw_aux_button(
            canvas,
            Rect::centered(x, 578, 58, 32),
            0xF0F0EE,
            0x3A3E42,
            "",
            SZ_TAG,
        );
    }

    // Speaker grille panel on lower body.
    canvas.rounded_rect(
        Rect {
            x: 95,
            y: 760,
            width: 420,
            height: 130,
        },
        12,
        0xBEC0C2,
    );
    canvas.dot_grid((124, 790), 9, 3, 26, 3, 0x6E7276);

    // Brand text and web address panel on lower body.
    canvas.text_centered("文曲星", 640, 830, 40, 0x3A3E42);
    canvas.text_centered("2000A", 790, 830, SZ_NUM, 0x3A3E42);
    canvas.rounded_rect(
        Rect {
            x: 560,
            y: 762,
            width: 420,
            height: 36,
        },
        8,
        0xBEC0C2,
    );
    canvas.text_centered("www.ggv.com.cn", 770, 780, SZ_INFO, 0x5A5E62);

    draw_hinge(canvas, 0x2A2D32, 0xC8CACC, 0x5A5E62);

    draw_keys(canvas, MachineModel::Cc800, layout);

    canvas.circle(120, 940, 7, 0x3A3E42);
    canvas.text_centered("RESET", 186, 940, SZ_TEXT, 0x4A4E50);

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x2E3134);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x17191B);
}
