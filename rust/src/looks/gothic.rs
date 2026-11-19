//! Gothic: book and blackletter pixel type, cut-stone corners, double rules, lozenges and pointed
//! arches. Classic's drawing with Gothic's shapes and faces.

use super::Look;
use crate::font::{fm, FaceMap, TTF_JACQUARD, TTF_JERSEY, TTF_TERM, TTF_TERM_BOLD};
use crate::prelude::*;

pub struct Gothic;

impl Look for Gothic {
    fn id(&self) -> &'static str {
        "gothic"
    }
    fn style_label(&self) -> &'static str {
        n_("Style (Gothic)")
    }
    fn name(&self) -> &'static str {
        // TRANSLATORS: a style in Settings > Display > Style: book and blackletter type, pointed arches.
        n_("Gothic")
    }
    fn faces(&self) -> Option<[FaceMap; FontFace::COUNT]> {
        Some([
            fm(TTF_JERSEY, 1.15, false),   // FONT_REGULAR
            fm(TTF_JERSEY, 1.15, true),    // FONT_BOLD
            fm(TTF_JACQUARD, 1.1, false),  // FONT_TITLE
            fm(TTF_JERSEY, 1.15, true),    // FONT_CAPS
            fm(TTF_TERM, 1.0, false),      // FONT_FIXED
            fm(TTF_TERM_BOLD, 1.0, false), // FONT_FIXED_BOLD
        ])
    }
    fn share_pages_gothic(&self) -> bool {
        true
    }

    // Gothic names use the pixel face's regular weight: its overprinted bold turns a name in capitals
    // into a solid black bar.
    fn name_face(&self) -> FontFace {
        FontFace::Regular
    }
    fn heading_face(&self) -> FontFace {
        FontFace::Title
    }
    // (Gothic keeps its carved corners where Classic rounds them wider, issue #209)
    fn corner_radius(&self) -> i32 {
        ui_px(16.0)
    }
    fn mark_scale(&self) -> f32 {
        1.8
    }
    // The frieze hangs below the header rule; leave air under its drops.
    fn header_drop(&self) -> i32 {
        ui_px(24.0)
    }
    // A button big enough for one has the inner line of a carved panel, 2 px wide, inside its edge.
    fn carved(&self, w: i32, h: i32) -> bool {
        h >= ui_px(90.0) && w >= ui_px(90.0)
    }
    fn label_lift(&self) -> i32 {
        ui_px(6.0)
    }
    // (the lozenge is wider than the gap ui_slider_layout leaves it)
    fn slider_knob_radius(&self) -> i32 {
        ui_px(32.0)
    }
    fn sheet_metrics(&self) -> (i32, i32, i32) {
        (ui_px(130.0), ui_px(40.0), ui_px(44.0))
    }
    fn toast_inset(&self) -> i32 {
        ui_px(50.0)
    }
    // The chip's ends are points as deep as half the chip (frame's cut corners): the text keeps all but
    // 7 px of that from each tip.
    fn chip_pad(&self, h: i32) -> i32 {
        h / 2 - ui_px(7.0)
    }

    fn frame(&self, c: &mut Canvas, r: Rect, radius: i32, gray: u8, filled: bool, t: i32) {
        canvas_chamfer_rect(c, r, radius + radius / 4, gray, filled, t);
    }
    fn frieze(&self, c: &mut Canvas, x0: i32, x1: i32, y: i32, h: i32, gray: u8) {
        if x1 - x0 < h {
            return;
        }
        // A cornice of pointed arches hanging from the rule at y, a lozenge drop at every springer.
        let mut span = h as f32 / 0.866;
        let n = ((x1 - x0) as f32 / span) as i32;
        if n < 1 {
            return;
        }
        span = (x1 - x0) as f32 / n as f32;
        let ys = fmla(span, 0.866, y as f32);
        let w = ui_px(3.0) as f32;
        for i in 0..n {
            arch(c, fmla(span, i as f32, x0 as f32), fmla(span, (i + 1) as f32, x0 as f32), ys, w, gray);
        }
        for i in 0..=n {
            let x = fmla(span, i as f32, x0 as f32);
            canvas_line_aa(c, x, y as f32, x, ys, w, gray);
            canvas_lozenge(c, x, ys + ui_px(7.0) as f32, ui_px(7.0) as f32, gray);
        }
    }
    // Thick over thin, the way a printer's rule frames a title page.
    fn rule(&self, c: &mut Canvas, x0: i32, x1: i32, y: i32, t: i32, gray: u8) {
        canvas_hline(c, x0, x1, y, t, gray);
        canvas_hline(c, x0, x1, y + t + ui_px(5.0), ui_px(2.0), gray);
    }
    fn bullet(&self, c: &mut Canvas, cx: f32, cy: f32, k: f32) {
        canvas_lozenge(c, cx, cy, ui_px(10.0 * k) as f32, 0);
    }
    // A groove of two hairlines, filled solid up to a lozenge knob.
    fn slider_track(&self, c: &mut Canvas, tr: Rect, fx: i32, cy: f32) {
        canvas_hline(c, tr.x, tr.x + tr.w, tr.y, ui_px(2.0), 0);
        canvas_hline(c, tr.x, tr.x + tr.w, tr.y + tr.h - ui_px(2.0), ui_px(2.0), 0);
        canvas_fill(c, Rect { x: tr.x, y: tr.y, w: fx - tr.x, h: tr.h }, 0);
        canvas_lozenge(c, fx as f32, cy, self.slider_knob_radius() as f32, 0);
        canvas_lozenge(c, fx as f32, cy, ui_px(20.0) as f32, 255);
        canvas_lozenge(c, fx as f32, cy, ui_px(10.0) as f32, 0);
    }
    // A card with cut corners and an inner line, its title centred over an ornament.
    fn sheet(&self, c: &mut Canvas, card: Rect, title: &str) -> Rect {
        canvas_chamfer_rect(c, card, ui_px(34.0), 255, true, 0);
        canvas_chamfer_rect(c, card, ui_px(34.0), 0, false, ui_px(6.0));
        let inner = rect_inflate(card, -ui_px(16.0));
        canvas_chamfer_rect(c, inner, ui_px(26.0), 0, false, ui_px(2.0));
        let th = ui_px(130.0);
        if !title.is_empty() {
            font_draw_centered(
                c,
                FontFace::Title,
                ui_title_size(),
                Rect { x: card.x + ui_px(80.0), y: card.y + ui_px(10.0), w: card.w - ui_px(160.0), h: th - ui_px(20.0) },
                title,
                0,
            );
            ornament(c, card.x + ui_px(60.0), card.x + card.w - ui_px(60.0), card.y + th, 0);
            return ui_sheet_inner(card, true);
        }
        ui_sheet_inner(card, false)
    }
    // A black banner with pointed ends and a white inner line.
    fn toast_plate(&self, c: &mut Canvas, r: Rect, ends: i32) {
        canvas_chamfer_rect(c, r, ends, 0, true, 0);
        canvas_chamfer_rect(c, rect_inflate(r, -ui_px(9.0)), ends - ui_px(9.0), 255, false, ui_px(2.0));
    }
    // A lozenge, filled when ticked.
    fn checkbox_box(&self, c: &mut Canvas, b: Rect, checked: bool) {
        let bs = b.w;
        let cx = b.x as f32 + bs as f32 / 2.0;
        let cy = b.y as f32 + bs as f32 / 2.0;
        let rr = bs as f32 / 2.0 + ui_px(4.0) as f32;
        canvas_lozenge(c, cx, cy, rr, 0);
        canvas_lozenge(c, cx, cy, rr - ui_px(5.0) as f32, 255);
        if checked {
            canvas_lozenge(c, cx, cy, rr - ui_px(14.0) as f32, 0);
        }
    }
}

/// A pointed (equilateral) arch: springers at (x0, ys) and (x1, ys), stroked with width w.
// (3.14159265f is the C's float literal, not f32::consts::PI)
#[allow(clippy::approx_constant, clippy::excessive_precision)]
fn arch(c: &mut Canvas, x0: f32, x1: f32, ys: f32, w: f32, gray: u8) {
    let span = x1 - x0;
    const SEG: i32 = 8;
    for side in 0..2 {
        // Left half: arc centred on the right springer; right half mirrored.
        let ccx = if side == 0 { x1 } else { x0 };
        let a0 = if side == 0 { 3.14159265f32 } else { 0.0 };
        let a1 = if side == 0 { 2.0943951f32 } else { 1.0471976 };
        // (ccx + span * cosf(a0) and ys - span * sinf(a0) are contracted multiply-adds in the C build: fmla)
        let mut px = fmla(span, a0.cos(), ccx);
        let mut py = fmla(-span, a0.sin(), ys);
        for i in 1..=SEG {
            let th = a0 + (a1 - a0) * i as f32 / SEG as f32;
            let qx = fmla(span, th.cos(), ccx);
            let qy = fmla(-span, th.sin(), ys);
            canvas_line_aa(c, px, py, qx, qy, w, gray);
            px = qx;
            py = qy;
        }
    }
}

/// An ornamental divider: thin rule broken by a lozenge with two small ones beside it.
fn ornament(c: &mut Canvas, x0: i32, x1: i32, y: i32, gray: u8) {
    let mid = (x0 + x1) / 2;
    let gap = ui_px(60.0);
    canvas_hline(c, x0, mid - gap, y - ui_px(1.0), ui_px(2.0), gray);
    canvas_hline(c, mid + gap, x1, y - ui_px(1.0), ui_px(2.0), gray);
    canvas_lozenge(c, mid as f32, y as f32, ui_px(16.0) as f32, gray);
    canvas_lozenge(c, (mid - ui_px(36.0)) as f32, y as f32, ui_px(8.0) as f32, gray);
    canvas_lozenge(c, (mid + ui_px(36.0)) as f32, y as f32, ui_px(8.0) as f32, gray);
}
