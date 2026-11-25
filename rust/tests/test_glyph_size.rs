// test_glyph_size.rs: a glyph's bitmap has a greatest size (font.rs, GLYPH_SIDE_MAX: 4,096 px each way and 4 MiB
// in all), whoever draws it. Icons in rectangles of any size and texts at any height are drawn through the font's
// own calls, as every screen draws them, in both themes. Each ends with nothing drawn outside its rectangle or
// the canvas's clip, and with the process no larger than the drawing of the greatest glyphs left it.
//
// One test, alone in its process: the measure is the process's own.
use grimoire::prelude::*;

/// The most the process has held so far, in bytes (getrusage: its greatest resident size, told in bytes on macOS
/// and in kilobytes elsewhere).
fn most_held() -> u64 {
    // SAFETY: getrusage fills the struct this owns before it is read.
    let ru = unsafe {
        let mut ru: libc::rusage = std::mem::zeroed();
        assert_eq!(libc::getrusage(libc::RUSAGE_SELF, &mut ru), 0);
        ru
    };
    let most = ru.ru_maxrss as u64;
    if cfg!(target_os = "macos") {
        most
    } else {
        most * 1024
    }
}

/// How many pixels of the canvas are not paper, and how many of those lie outside `keep`.
fn ink(c: &Canvas, keep: Rect) -> (usize, usize) {
    let (mut all, mut outside) = (0, 0);
    for y in 0..c.h {
        for x in 0..c.w {
            if c.px[(y * c.stride + x) as usize] != 255 {
                all += 1;
                outside += usize::from(!rect_contains(keep, x, y));
            }
        }
    }
    (all, outside)
}

const FACES: [FontFace; 6] = [FontFace::Regular, FontFace::Bold, FontFace::Title, FontFace::Caps, FontFace::Fixed, FontFace::FixedBold];
const COG: u32 = 0xF0493;
const CLIP: Rect = Rect { x: 0, y: 30, w: 1100, h: 800 };
const MIDDLE: (i32, i32) = (600, 450);

/// An icon in a rectangle of w by h about the canvas's middle: the ink it left, none of it outside the rectangle
/// or the clip.
fn icon(c: &mut Canvas, w: i32, h: i32) -> usize {
    c.px.fill(255);
    canvas_set_clip(c, CLIP);
    let r = Rect { x: MIDDLE.0 - w / 2, y: MIDDLE.1 - h / 2, w, h };
    font_draw_icon(c, FontFace::Regular, COG, r, 0);
    // (the font rounds a glyph out to whole pixels: a pixel over at most; a rectangle far larger than the canvas
    // holds all of the clip)
    let keep = if w > 1_000_000 || h > 1_000_000 { CLIP } else { rect_intersect(rect_inflate(r, 1), CLIP) };
    let (all, outside) = ink(c, keep);
    assert_eq!(outside, 0, "an icon in {w} by {h}: ink outside its rectangle or the clip");
    all
}

/// A text in each of the font's calls at a height: the ink it left, none of it outside the clip.
fn text(c: &mut Canvas, f: FontFace, px: f32) -> usize {
    c.px.fill(255);
    canvas_set_clip(c, CLIP);
    let r = Rect { x: 100, y: 100, w: 900, h: 600 };
    let wide = font_draw(c, f, px, 0, 780, "Tenletters", 0);
    assert_eq!(wide, font_measure(f, px, "Tenletters"), "{f:?} at {px} px: as wide drawn as measured");
    font_draw(c, f, px, 40, 700, "W", 0);
    font_draw_centered(c, f, px, r, "Wide", 0);
    font_draw_left(c, f, px, r, "Wide", 0);
    font_draw_wrapped(c, f, px, r, "Wide words", 3, 0, true, 0);
    let (all, outside) = ink(c, CLIP);
    assert_eq!(outside, 0, "{f:?} at {px} px: ink outside the clip");
    all
}

#[test]
fn drawing_at_any_size_leaves_the_process_no_larger_than_the_greatest_glyphs_do() {
    ui_init(300.0);
    assert_eq!(font_init(), 0);
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, 1200, 900), 0);
    let themes = [UiTheme::Classic, UiTheme::Gothic];
    let at_start = most_held();

    // What a screen draws, and on to the greatest glyphs: text to 3,000 px high and an icon in a square of
    // 2,000 px, in both themes, are drawn; and in text to 7,000 px high some letters still are of a size a glyph
    // has (a narrow one stays under it longest).
    for theme in themes {
        ui_set_theme(theme);
        for f in FACES {
            for px in [12.0, 44.0, 200.0, 768.0, 2000.0, 3000.0] {
                assert_ne!(text(&mut c, f, px), 0, "{theme:?}, {f:?} at {px} px is drawn");
            }
            for px in [3300.0, 3600.0, 4096.0, 5000.0, 6000.0, 7000.0] {
                text(&mut c, f, px);
            }
        }
        for side in [24, 96, 1024, 1280, 2000] {
            assert_ne!(icon(&mut c, side, side), 0, "an icon in a square of {side} px is drawn");
        }
    }
    let before = most_held();

    // Then sizes past the greatest glyph, from a little past it to the greatest numbers: squares, rectangles
    // with one long side, and text heights. Nothing of a glyph that is not made is drawn, and the process is no
    // larger than it was (with 32 MiB of room for what an allocator keeps).
    let room = 32u64 << 20;
    let held = |what: &str| {
        let grown = most_held().saturating_sub(before);
        assert!(grown <= room, "{what}: the process grew by {grown} bytes");
    };
    let squares = [2100, 3000, 4096, 4097, 6000, 12_000, 30_000, 50_000, 58_000, 66_000, 82_000, 83_000, 90_000, 100_000, 120_000, 133_000, 2_000_000_000, i32::MAX];
    let long = [(100_000, 90_000), (90_000, 100_000), (65_536, 65_537), (133_000, 5000), (5000, 133_000), (i32::MAX, 100_000)];
    let heights = [8192.0, 12_000.0, 20_000.0, 60_000.0, 100_000.0, 110_000.0, 140_000.0, 150_000.0, 1_000_000.0, 2_147_483.0];
    for theme in themes {
        ui_set_theme(theme);
        for side in squares {
            assert_eq!(icon(&mut c, side, side), 0, "an icon in a square of {side} px is not drawn");
            held(&format!("an icon in a square of {side} px"));
        }
        for (w, h) in long {
            assert_eq!(icon(&mut c, w, h), 0, "an icon in {w} by {h} is not drawn");
            held(&format!("an icon in {w} by {h}"));
        }
        // (one long side: the icon is as large as the shorter side lets it be, and is drawn)
        for (w, h) in [(133_000, 1024), (1024, 133_000), (2_000_000_000, 1280), (1280, i32::MAX)] {
            assert_ne!(icon(&mut c, w, h), 0, "an icon in {w} by {h} is drawn");
            held(&format!("an icon in {w} by {h}"));
        }
        for (w, h) in [(133_000, 1), (1, 133_000), (i32::MAX, 1), (1, i32::MAX)] {
            icon(&mut c, w, h);
            held(&format!("an icon in {w} by {h}"));
        }
        for f in FACES {
            for px in heights {
                // (from 8,192 px on no letter of these words is of a size a glyph has, in any face)
                assert_eq!(text(&mut c, f, px), 0, "{theme:?}, {f:?} at {px} px is not drawn");
                held(&format!("{theme:?}, {f:?} at {px} px"));
            }
        }
    }
    ui_set_theme(UiTheme::Gothic);
    eprintln!("glyph sizes: the process's greatest size was {at_start} bytes at the start, {before} after the greatest glyphs, and {} at the end", most_held());
}
