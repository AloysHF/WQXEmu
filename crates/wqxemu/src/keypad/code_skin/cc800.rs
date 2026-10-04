//! CC800 code-drawn device skin.

use wqxemu_core::{KeyDef, MachineModel};

use super::{
    draw_aux_button, draw_hinge, draw_keys, draw_screen_numbers, Canvas, Rect, SZ_INFO, SZ_NUM,
    SZ_SMALL, SZ_TAG, SZ_TEXT,
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
    // Upper body: silver / light gray
    canvas.gradient_rounded_rect(
        Rect {
            x: 56,
            y: 46,
            width: 974,
            height: 644,
        },
        26,
        0xD8DADB,
        0xCACCCE,
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

    // Number strip (1-9) in a curved silver panel below LCD
    canvas.rounded_rect(
        Rect {
            x: 95,
            y: 416,
            width: 580,
            height: 38,
        },
        12,
        0xC0C2C4,
    );
    draw_screen_numbers(canvas, screen, 435, 0x2A2E40);

    // F-key white pill buttons with "▶Fn" prefix labels
    for (index, (y, label)) in [(130, "报文"), (210, "变化"), (290, "事件"), (370, "闹钟")]
        .into_iter()
        .enumerate()
    {
        let fn_label = format!("▶F{}", index + 1);
        canvas.text_centered(&fn_label, 790, y, SZ_INFO, 0x3A3E42);
        draw_aux_button(
            canvas,
            Rect::centered(910, y, 88, 32),
            0xF8F8F6,
            0x1A3A8A,
            label,
            SZ_INFO,
        );
    }

    // Brand area: GOLDEN GLOBAL VIEW + 文曲星 + www.ggv.com.cn + CC800
    // Layout matches reference: GOLDEN left, 文曲星 right, www below, CC800 below 文曲星
    canvas.text_centered("GOLDEN GLOBAL VIEW", 530, 615, SZ_TAG, 0x2A2E40);
    canvas.text_centered("www.ggv.com.cn", 530, 650, SZ_TEXT, 0x5A5E62);
    canvas.text_centered("文曲星", 800, 580, 52, 0x3A3E42);
    canvas.text_centered("®", 890, 545, SZ_SMALL, 0x55595C);
    canvas.text_centered("CC800", 840, 650, SZ_NUM, 0x3A3E42);

    // Speaker grille dots on left side of upper body (above hinge)
    canvas.dot_grid((115, 600), 9, 3, 28, 3, 0x4E5254);

    draw_hinge(canvas, 0x2A2D32, 0xC8CACC, 0x5A5E62);

    draw_keys(canvas, MachineModel::Cc800, layout);

    canvas.circle(120, 910, 7, 0x3A3E42);
    canvas.text_centered("RESET", 186, 910, SZ_TEXT, 0x4A4E50);
    // ON/OFF label beside the pink power button, aligned with network/F-keys row
    canvas.text_centered("ON/OFF", 950, 910, SZ_INFO, 0x3A3E42);

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x2E3134);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x17191B);
}
