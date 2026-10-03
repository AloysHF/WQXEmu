//! NC3000 code-drawn device skin.

use wqxemu_core::{KeyDef, MachineModel};

use super::{
    draw_aux_button, draw_hinge, draw_keys, draw_screen_numbers, Canvas, Rect, SZ_INFO, SZ_SMALL,
    SZ_TAG, SZ_TEXT,
};

pub(super) fn draw(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
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
