//! PC1000 code-drawn device skin.

use wqxemu_core::{KeyDef, MachineModel};

use super::{
    draw_aux_button, draw_keys, draw_screen_numbers, Canvas, Rect, SZ_INFO, SZ_KEY, SZ_LOGO,
    SZ_SMALL, SZ_TAG, SZ_TEXT,
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
        canvas.text_centered(label, 848, y, SZ_TEXT, 0x3A3E42);
        draw_aux_button(
            canvas,
            Rect::centered(936, y, 56, 30),
            0x2A2E32,
            0xFFFFFF,
            &format!("F{}", index + 1),
            SZ_TAG,
        );
    }
    // Branding prints directly on the silver lid: embossed brand name and
    // the teal PC1000 mark, matching the reference photo.
    canvas.text_centered("文曲星", 200, 588, 34, 0x8A8E92);
    canvas.text_centered("PC1000", 400, 588, SZ_LOGO, 0x0D7A8C);
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

    // Speaker: silver ring around a recessed grille of raised metal bars.
    // Bars are thick and nearly square-ended with narrow gaps, matching
    // the stamped-metal grille on the reference device.
    canvas.circle(185, 852, 118, 0x9A9EA2);
    canvas.circle(185, 852, 112, 0x2E3134);
    canvas.circle_grille((185, 852), 106, 13, 3, 0xC8CBCD, 0xFFFFFF, 0x1A1C1E);

    // Media control bar: silver face with a thin dark edge and black
    // transport icons matching the reference (play, skip-back, rec/stop).
    canvas.rounded_rect(
        Rect {
            x: 560,
            y: 690,
            width: 458,
            height: 90,
        },
        34,
        0x8A8E92,
    );
    canvas.rounded_rect(
        Rect {
            x: 564,
            y: 694,
            width: 450,
            height: 82,
        },
        30,
        0xC8CBCD,
    );
    let ink = 0x1A1C1E;
    // Play: solid right-pointing triangle.
    canvas.triangle(650, 735, 15, (1, 0), ink);
    // Skip-back: a thin stop bar then two left-pointing triangles.
    canvas.rounded_rect(Rect::centered(748, 735, 5, 28), 0, ink);
    canvas.triangle(774, 735, 11, (-1, 0), ink);
    canvas.triangle(800, 735, 11, (-1, 0), ink);
    // Record / stop: red disc, slash, black square.
    canvas.circle(918, 735, 11, 0xC02020);
    canvas.line(942, 750, 958, 720, 3, ink);
    canvas.rounded_rect(Rect::centered(984, 735, 18, 18), 2, ink);

    canvas.text_centered("ON/", 344, 728, SZ_INFO, 0x1A3298);
    canvas.text_centered("OFF", 344, 754, SZ_INFO, 0x1A3298);

    draw_keys(canvas, MachineModel::Pc1000, layout);

    draw_aux_button(
        canvas,
        Rect::centered(380, 933, 82, 50),
        0xF6AE28,
        0xFFFFFF,
        "发音",
        SZ_TEXT,
    );
    draw_aux_button(
        canvas,
        Rect::centered(478, 933, 82, 50),
        0xF6AE28,
        0xFFFFFF,
        "报时",
        SZ_TEXT,
    );

    canvas.rounded_rect(Rect::centered(543, 1370, 112, 26), 8, 0x17191B);
    canvas.rounded_rect(Rect::centered(543, 1370, 72, 14), 5, 0x3A3D40);
}
