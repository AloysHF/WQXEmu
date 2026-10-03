//! PC1000 code-drawn device skin.

use wqxemu_core::{KeyDef, MachineModel};

use super::{
    draw_aux_button, draw_keys, draw_screen_numbers, Canvas, Rect, SZ_KEY, SZ_LOGO, SZ_SMALL,
    SZ_TAG, SZ_TEXT,
};

pub(super) fn draw(canvas: &mut Canvas, screen: Rect, layout: &[KeyDef]) {
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
