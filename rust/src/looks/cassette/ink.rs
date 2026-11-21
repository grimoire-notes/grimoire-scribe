//! Cassette's brushes: corner brackets, crosshairs, dot-matrix signs, sprayed letters, the dimmed backdrop
//! and the frame of instruments. The Look in ../cassette.rs draws with them.

use crate::prelude::*;

/// The palette (the mockups' th-eink): paper, ink, and the four grays between.
pub const CS_INK: u8 = 0x11;
pub const CS_DIM: u8 = 0x55;
/// thumbnail edges, crosshairs, the bleed's grain
pub const CS_TL: u8 = 0x99;
/// a disabled control, a separator
pub const CS_LINE: u8 = 0xC9;
/// the selection's fill
pub const CS_SEL: u8 = 0xCF;

/// A deterministic hash of a pixel (the grain is the same on every redraw, so a refresh never flickers it).
fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^ (h >> 15)
}
/// 0..1 from the hash
fn rnd(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) & 0xffff) as f32 / 65535.0
}
/// Four L-shaped corner brackets around r: arms `arm` px long, `t` px thick, inside r.
pub fn cs_brackets(c: &mut Canvas, r: Rect, arm: i32, t: i32, gray: u8) {
    let arm = arm.min(r.w / 2).min(r.h / 2);
    let (x1, y1) = (r.x + r.w, r.y + r.h);
    for (x, y) in [(r.x, r.y), (x1 - arm, r.y), (r.x, y1 - t), (x1 - arm, y1 - t)] {
        canvas_fill(c, Rect::new(x, y, arm, t), gray);
    }
    for (x, y) in [(r.x, r.y), (x1 - t, r.y), (r.x, y1 - arm), (x1 - t, y1 - arm)] {
        canvas_fill(c, Rect::new(x, y, t, arm), gray);
    }
}

/// A crosshair mark (+) centred on (cx, cy), arms `arm` px from the centre.
pub fn cs_cross(c: &mut Canvas, cx: i32, cy: i32, arm: i32, t: i32, gray: u8) {
    canvas_fill(c, Rect::new(cx - arm, cy - t / 2, 2 * arm + 1, t.max(1)), gray);
    canvas_fill(c, Rect::new(cx - t / 2, cy - arm, t.max(1), 2 * arm + 1), gray);
}

/// How much the screen behind a sheet is dimmed (paper becomes this much of white).
const BACKDROP_DIM: f32 = 0.88;

/// What a sheet stands over: the screen outside `keep`, dimmed under a light gray (BACKDROP_DIM), as it is
/// otherwise, so the sheet's paper stands out from it.
pub fn cs_backdrop(c: &mut Canvas, keep: Rect) {
    let area = c.clip;
    if area.w <= 0 || area.h <= 0 {
        return;
    }
    let keep = rect_intersect(keep, area);
    for y in area.y..area.y + area.h {
        let row = (y * c.stride) as usize;
        let inside_rows = y >= keep.y && y < keep.y + keep.h;
        for x in area.x..area.x + area.w {
            if inside_rows && x >= keep.x && x < keep.x + keep.w {
                continue;
            }
            let v = &mut c.px[row + x as usize];
            *v = (*v as f32 * BACKDROP_DIM) as u8;
        }
    }
}

/// A small sign drawn dot by dot from rows of a pattern ('X' a dot, 'o' a half dot), centred in r, on a
/// grid of `cells` (at least the pattern's size). The signs a font's glyph turns to mush at a few dots
/// across (chevrons, arrows, a cross) are drawn so instead.
pub fn cs_dot_pattern(c: &mut Canvas, r: Rect, rows: &[&str], cells: i32, gray: u8) {
    let h = rows.len() as i32;
    let w = rows.iter().map(|s| s.len()).max().unwrap_or(0) as i32;
    let n = cells.max(w).max(h);
    let pitch = r.w.min(r.h) as f32 / n as f32;
    let ox = r.x as f32 + (r.w as f32 - w as f32 * pitch) / 2.0;
    let oy = r.y as f32 + (r.h as f32 - h as f32 * pitch) / 2.0;
    for (gy, row) in rows.iter().enumerate() {
        for (gx, ch) in row.bytes().enumerate() {
            let rad = match ch {
                b'X' => pitch * 0.4,
                b'o' => pitch * 0.2,
                _ => continue,
            };
            canvas_disc_aa(c, ox + (gx as f32 + 0.5) * pitch, oy + (gy as f32 + 0.5) * pitch, rad.max(1.0), gray, 1.0);
        }
    }
}

/// An icon drawn as a dot-matrix: `draw` puts the glyph on a blank canvas the size of r, and each
/// cell of a grid over it becomes a dot (well covered), a half dot (partly) or nothing. The dots keep
/// one pitch whatever the icon's size, as on a display's dot matrix.
pub fn cs_dot_icon(c: &mut Canvas, r: Rect, gray: u8, draw: impl FnOnce(&mut Canvas, Rect)) {
    if r.w < 4 || r.h < 4 {
        return;
    }
    let side = r.w.min(r.h);
    let n = clampi(side / ui_px(6.0).max(1), 11, 16);
    let pitch = side as f32 / n as f32;
    let mut m = Canvas::default();
    if canvas_init(&mut m, side, side) != 0 {
        return;
    }
    draw(&mut m, Rect::new(0, 0, side, side));
    let ox = r.x as f32 + (r.w - side) as f32 / 2.0;
    let oy = r.y as f32 + (r.h - side) as f32 / 2.0;
    // each cell's coverage first, then the dots: a covered cell with every neighbour covered is the
    // inside of a shape and takes a half dot, so a filled glyph reads as a dotted outline
    let n_u = n as usize;
    let mut cov = vec![0f32; n_u * n_u];
    for gy in 0..n {
        for gx in 0..n {
            let (x0, y0) = ((gx as f32 * pitch) as i32, (gy as f32 * pitch) as i32);
            let (x1, y1) = ((((gx + 1) as f32) * pitch) as i32, (((gy + 1) as f32) * pitch) as i32);
            let mut ink = 0u32;
            let mut all = 0u32;
            for y in y0..y1.min(side) {
                for x in x0..x1.min(side) {
                    ink += 255 - m.px[(y * m.stride + x) as usize] as u32;
                    all += 255;
                }
            }
            cov[gy as usize * n_u + gx as usize] = if all > 0 { ink as f32 / all as f32 } else { 0.0 };
        }
    }
    let on = |gx: i32, gy: i32| gx >= 0 && gy >= 0 && gx < n && gy < n && cov[gy as usize * n_u + gx as usize] > 0.42;
    for gy in 0..n {
        for gx in 0..n {
            let cv = cov[gy as usize * n_u + gx as usize];
            let inside = on(gx, gy) && on(gx - 1, gy) && on(gx + 1, gy) && on(gx, gy - 1) && on(gx, gy + 1);
            let rad = if cv > 0.42 && !inside {
                pitch * 0.4
            } else if cv > 0.12 {
                pitch * 0.2
            } else {
                continue;
            };
            canvas_disc_aa(c, ox + (gx as f32 + 0.5) * pitch, oy + (gy as f32 + 0.5) * pitch, rad.max(1.0), gray, 1.0);
        }
    }
}

/// The spray on a title: its first letters and (more) its last ones blur into grain, its middle stays
/// sharp. `x0..x1` is where the text was drawn inside r.
pub fn cs_spray(c: &mut Canvas, r: Rect, x0: i32, x1: i32, seed: u32) {
    cs_spray_at(c, r, x0, x1, seed, ui_px(7.0));
}

/// cs_spray with the grain's reach: `rmax` px of blur where the spray is heaviest (the logo takes twice
/// a title's, the broken photocopier look).
pub fn cs_spray_at(c: &mut Canvas, r: Rect, x0: i32, x1: i32, seed: u32, rmax: i32) {
    let r = rect_intersect(r, c.clip);
    if r.w <= 0 || r.h <= 0 || x1 <= x0 {
        return;
    }
    let (w, h) = (r.w as usize, r.h as usize);
    // the ink, and its summed-area table for box blurs of any radius
    let mut sat = vec![0u32; (w + 1) * (h + 1)];
    for y in 0..h {
        let mut row = 0u32;
        for x in 0..w {
            row += 255 - c.px[((r.y + y as i32) * c.stride + r.x + x as i32) as usize] as u32;
            sat[(y + 1) * (w + 1) + x + 1] = sat[y * (w + 1) + x + 1] + row;
        }
    }
    let rmax = rmax as f32;
    let span = (x1 - x0) as f32;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let f = ((r.x + x - x0) as f32 / span).clamp(0.0, 1.0);
            // N E sprayed a little, the middle sharp, the end most (the mockup's b2 b1 b0 ... b1 b2 b3 b3 b2)
            let k = (0.6 * (1.0 - f / 0.22).max(0.0)) + 0.8 * ((f - 0.6) / 0.3).clamp(0.0, 1.0);
            let rad = (rmax * k) as i32;
            if rad < 1 {
                continue;
            }
            let (ax, ay) = ((x - rad).max(0) as usize, (y - rad).max(0) as usize);
            let (bx, by) = ((x + rad + 1).min(w as i32) as usize, (y + rad + 1).min(h as i32) as usize);
            let s = sat[by * (w + 1) + bx] + sat[ay * (w + 1) + ax] - sat[ay * (w + 1) + bx] - sat[by * (w + 1) + ax];
            let a = s as f32 / (255.0 * ((bx - ax) * (by - ay)) as f32);
            // grain: a pixel is ink where the blurred ink beats a random threshold, weighted to keep
            // the letters readable
            let t = 0.12 + 0.76 * rnd(r.x + x, r.y + y, seed);
            let px = &mut c.px[((r.y + y) * c.stride + r.x + x) as usize];
            *px = if a * 1.35 > t { CS_INK } else { 255 };
        }
    }
}

/// A graph's node as a dot-matrix sign: a ring of dots, and inside it a grid of dots whose size says how
/// linked the node is (level 0 none, 1 and 2 finer, 4 full), as a display would plot it.
pub fn cs_node(c: &mut Canvas, x: f32, y: f32, r: f32, level: i32) {
    // a coarse matrix, so the node reads as dots and not as a gray disc
    let pitch = (ui_px(13.0) as f32).max(4.0).min(r / 2.5);
    // the ring: dots round the edge, as many as the circumference holds at the pitch
    let n = ((std::f32::consts::TAU * r / pitch).round() as i32).max(8);
    for k in 0..n {
        let a = k as f32 * std::f32::consts::TAU / n as f32;
        canvas_disc_aa(c, x + r * a.cos(), y + r * a.sin(), pitch * 0.26, CS_INK, 1.0);
    }
    let dot = match level {
        0 => return,
        1 => pitch * 0.12,
        2 => pitch * 0.2,
        _ => pitch * 0.3,
    };
    let inner = r - pitch * 1.2;
    let m = (inner / pitch).floor() as i32;
    for gy in -m..=m {
        for gx in -m..=m {
            let (dx, dy) = (gx as f32 * pitch, gy as f32 * pitch);
            if dx * dx + dy * dy <= inner * inner {
                canvas_disc_aa(c, x + dx, y + dy, dot.max(0.8), CS_INK, 1.0);
            }
        }
    }
}

/// A ring of dots (a node chosen, or the page that is open), as cs_node draws its edge.
pub fn cs_dot_ring(c: &mut Canvas, x: f32, y: f32, r: f32, dot: f32) {
    let pitch = (ui_px(16.0) as f32).max(4.0);
    let n = ((std::f32::consts::TAU * r / pitch).round() as i32).max(8);
    for k in 0..n {
        let a = k as f32 * std::f32::consts::TAU / n as f32;
        canvas_disc_aa(c, x + r * a.cos(), y + r * a.sin(), dot.min(pitch * 0.25), CS_INK, 1.0);
    }
}

/// A target's ticks on the four axes outside a ring of radius r: the chosen node, locked on.
pub fn cs_lock_ticks(c: &mut Canvas, x: f32, y: f32, r: f32) {
    let (a, b) = (r + ui_px(6.0) as f32, r + ui_px(26.0) as f32);
    let t = ui_px(3.0) as f32;
    canvas_line_aa(c, x - b, y, x - a, y, t, CS_INK);
    canvas_line_aa(c, x + a, y, x + b, y, t, CS_INK);
    canvas_line_aa(c, x, y - b, x, y - a, t, CS_INK);
    canvas_line_aa(c, x, y + a, x, y + b, t, CS_INK);
}

/// Text drawn letter by letter with `track` px between letters (font_measure has no tracking): the
/// spaced-out capitals of Cassette's titles and captions. Returns the width drawn.
pub fn cs_tracked(c: &mut Canvas, f: FontFace, px: f32, r: Rect, text: &str, track: i32, gray: u8) -> i32 {
    let mut x = r.x;
    let mut buf = [0u8; 4];
    for ch in text.chars() {
        let s: &str = ch.encode_utf8(&mut buf);
        let w = font_measure(f, px, s);
        if x + w > r.x + r.w {
            break;
        }
        font_draw_left(c, f, px, Rect::new(x, r.y, w + 2, r.h), s, gray);
        x += w + track;
    }
    (x - track - r.x).max(0)
}
/// The width cs_tracked takes for text.
pub fn cs_tracked_w(f: FontFace, px: f32, text: &str, track: i32) -> i32 {
    let mut buf = [0u8; 4];
    let n = text.chars().count() as i32;
    text.chars().map(|ch| font_measure(f, px, ch.encode_utf8(&mut buf))).sum::<i32>() + track * (n - 1).max(0)
}

/// A short code for a thing, from its name: "REG.2213" style serials on titles and panels. The same
/// name gives the same code on every redraw.
pub fn cs_serial(name: &str) -> u32 {
    name.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619)) % 9000 + 1000
}

/// A rail: a hairline from x0 to x1 at y, a dot where it starts, a short ink bar near its end and a tick
/// where it ends, as an instrument's leader line. `tag` (a serial, a reading) sits small above its end.
pub fn cs_rail(c: &mut Canvas, x0: i32, x1: i32, y: i32, tag: &str) {
    if x1 - x0 < ui_px(80.0) {
        return;
    }
    let t = ui_px(2.0).max(1);
    canvas_hline(c, x0, x1, y, t, CS_TL);
    canvas_disc_aa(c, x0 as f32, y as f32 + t as f32 / 2.0, ui_px(5.0) as f32, CS_INK, 1.0);
    let bar = ui_px(90.0).min((x1 - x0) / 4);
    canvas_fill(c, Rect::new(x1 - bar - ui_px(40.0), y - ui_px(2.0), bar, ui_px(5.0)), CS_INK);
    canvas_fill(c, Rect::new(x1 - ui_px(26.0), y - ui_px(2.0), ui_px(12.0), ui_px(5.0)), CS_INK);
    canvas_vline(c, x1 - t, y - ui_px(12.0), y + ui_px(12.0), t, CS_INK);
    if !tag.is_empty() {
        let px = ui_px(22.0) as f32;
        let w = cs_tracked_w(FontFace::Fixed, px, tag, ui_px(3.0));
        if w < (x1 - x0) / 2 {
            cs_tracked(c, FontFace::Fixed, px, Rect::new(x1 - w, y - ui_px(40.0), w + 2, ui_px(32.0)), tag, ui_px(3.0), CS_DIM);
        }
    }
}

/// The screen's frame of instruments, in the margin round it (`band` px): a scale down the left edge with
/// readings every 200 px, ticks along the top, two broken rails down the right, registration crosses in the
/// corners, and two lines of system data along the foot (`left`, `right`). Drawn small and gray: it is
/// the edge of the picture.
pub fn cs_hud(c: &mut Canvas, scr: Rect, band: i32, left: &str, right: &str) {
    let t = ui_px(2.0).max(1);
    let (x1, y1) = (scr.x + scr.w, scr.y + scr.h);
    // the scale on the left: a tick every 40 px, a longer one with its reading every 200
    let step = ui_px(40.0).max(4);
    let px = ui_px(20.0) as f32;
    let mut y = scr.y + band + step;
    while y < y1 - band * 2 {
        let major = (y - scr.y) % (step * 5) < step;
        canvas_hline(c, scr.x + ui_px(4.0), scr.x + ui_px(if major { 30.0 } else { 14.0 }), y, t, if major { CS_DIM } else { CS_TL });
        if major {
            let n = format!("{:03}", (y - scr.y) / ui_px(10.0).max(1));
            font_draw_left(c, FontFace::Fixed, px, Rect::new(scr.x + ui_px(4.0), y + ui_px(4.0), band, ui_px(24.0)), &n, CS_TL);
        }
        y += step;
    }
    // ticks along the top
    let mut x = scr.x + band * 2;
    let mut k = 0;
    while x < x1 - band * 2 {
        canvas_vline(c, x, scr.y + ui_px(2.0), scr.y + ui_px(if k % 5 == 0 { 18.0 } else { 9.0 }), t, CS_TL);
        x += step * 3 / 2;
        k += 1;
    }
    // two broken rails down the right, a dot on each now and then
    for (i, dx) in [ui_px(14.0), ui_px(26.0)].into_iter().enumerate() {
        let xr = x1 - dx;
        let seg = ui_px(if i == 0 { 260.0 } else { 140.0 });
        let mut yy = scr.y + band * 2 + i as i32 * ui_px(70.0);
        while yy < y1 - band * 3 {
            canvas_vline(c, xr, yy, (yy + seg).min(y1 - band * 3), t, CS_TL);
            if i == 0 {
                canvas_disc_aa(c, xr as f32 + 0.5, (yy + seg) as f32, ui_px(4.0) as f32, CS_DIM, 1.0);
            }
            yy += seg + ui_px(36.0);
        }
    }
    // registration crosses at the corners
    let a = ui_px(10.0);
    for (cx, cy) in [(scr.x + band / 2, scr.y + band / 2), (x1 - band / 2, scr.y + band / 2), (scr.x + band / 2, y1 - band / 2), (x1 - band / 2, y1 - band / 2)] {
        cs_cross(c, cx, cy, a, t, CS_DIM);
    }
    // the data along the foot
    let dpx = ui_px(21.0) as f32;
    let tr = ui_px(3.0);
    let fy = y1 - ui_px(30.0);
    let lw = cs_tracked(c, FontFace::Fixed, dpx, Rect::new(scr.x + band + ui_px(10.0), fy, scr.w / 2 - band, ui_px(26.0)), left, tr, CS_DIM);
    let rw = cs_tracked_w(FontFace::Fixed, dpx, right, tr).min(scr.w / 2 - band);
    cs_tracked(c, FontFace::Fixed, dpx, Rect::new(x1 - band - ui_px(10.0) - rw, fy, rw + 2, ui_px(26.0)), right, tr, CS_DIM);
    let _ = lw;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brackets_stay_inside_and_leave_the_middle() {
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, 100, 60), 0);
        cs_brackets(&mut c, Rect::new(10, 10, 80, 40), 12, 3, 0);
        let at = |c: &Canvas, x: i32, y: i32| c.px[(y * c.stride + x) as usize];
        assert_eq!(at(&c, 10, 10), 0);
        assert_eq!(at(&c, 89, 49), 0);
        assert_eq!(at(&c, 50, 10), 255, "no line between the corners");
        assert_eq!(at(&c, 9, 9), 255, "nothing outside");
    }

    #[test]
    fn a_dot_icon_is_dots_on_a_grid() {
        ui_init(300.0);
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, 80, 80), 0);
        cs_dot_icon(&mut c, Rect::new(0, 0, 80, 80), 0, |m, r| canvas_fill(m, r, 0));
        // a solid square: every cell a dot, the gaps between them paper
        let at = |x: i32, y: i32| c.px[(y * c.stride + x) as usize];
        assert!(at(3, 3) < 64);
        assert!(at(0, 0) > 192);
    }
}
