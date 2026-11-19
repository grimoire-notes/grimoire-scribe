//! Cassette futurism on e-ink (Rust only): monospace type, corner brackets instead of outlines, gray
//! (not inverted) selection, dot-matrix icons, sprayed titles, a frame of instruments round the
//! Library and its other screens and a dimmed backdrop behind sheets. Its brushes are in cassette/ink.rs.

mod ink;

use super::{Look, NewFoot, NewSheetSpacing, ScreenInfo, ShelfInfo};
use crate::font::{fm, fm_bold, FaceMap, TTF_ARCHIVO, TTF_JETBRAINS, TTF_JETBRAINS_BOLD, TTF_SPACEMONO, TTF_SPACEMONO_BOLD, TTF_TERM};
use crate::prelude::*;
use crate::ui::{icon_label_px, label_inset, text_label_px};
use ink::*;

pub struct Cassette;

impl Look for Cassette {
    fn id(&self) -> &'static str {
        "cassette"
    }
    fn style_label(&self) -> &'static str {
        n_("Style (Cassette)")
    }
    fn name(&self) -> &'static str {
        // TRANSLATORS: a style in Settings > Display > Style: monospace type, corner brackets, dot-matrix
        // icons (cassette futurism).
        n_("Cassette")
    }
    // The mockups' type. Mono text, Space Mono for caps (button words, headings), Archivo Black for
    // titles; the FIXED faces (clock, counts) Space Mono too. (The text a little smaller than the bold: a
    // mono face is wide, and the narrowest columns, the Links bar's "pág. 2000", hold it so.)
    fn faces(&self) -> Option<[FaceMap; FontFace::COUNT]> {
        let mut m = [
            fm_bold(TTF_JETBRAINS, 1.03),      // FONT_REGULAR (its fallback Terminess regular, below)
            fm_bold(TTF_JETBRAINS_BOLD, 1.08), // FONT_BOLD
            fm_bold(TTF_ARCHIVO, 1.05),        // FONT_TITLE
            fm_bold(TTF_SPACEMONO_BOLD, 1.08), // FONT_CAPS
            fm(TTF_SPACEMONO, 1.1, false),     // FONT_FIXED
            fm_bold(TTF_SPACEMONO_BOLD, 1.1),  // FONT_FIXED_BOLD
        ];
        m[FontFace::Regular as usize].fallback = TTF_TERM;
        Some(m)
    }

    fn caps_labels(&self) -> bool {
        true
    }
    fn row_icon_px(&self) -> i32 {
        ui_px(44.0)
    }
    fn row_face(&self, font: FontFace, st: ButtonStyle) -> FontFace {
        if st == ButtonStyle::Selected { FontFace::Bold } else { font }
    }
    // (a small sign in the row's icon room)
    fn menu_icon_side(&self, rh: i32) -> i32 {
        (rh - ui_px(30.0)).min(ui_px(48.0))
    }
    fn slider_knob_radius(&self) -> i32 {
        ui_px(32.0)
    }
    fn sheet_metrics(&self) -> (i32, i32, i32) {
        (ui_px(140.0), ui_px(40.0), ui_px(44.0))
    }
    // (more of the dimmed screen round a sheet, so it reads as floating over it)
    fn sheet_margin(&self) -> i32 {
        ui_px(110.0)
    }
    fn toast_inset(&self) -> i32 {
        ui_px(50.0)
    }
    fn new_sheet_spacing(&self) -> NewSheetSpacing {
        NewSheetSpacing { head: ui_px(110.0), gap: ui_px(50.0), foot: ui_px(520.0), spread: true }
    }
    fn backdrop_behind_sheets(&self) -> bool {
        true
    }

    // Square, as a printed panel. An ink outline becomes a faint hairline held by ink corners
    // (registration marks), so every card and box in the app reads as the same printed form.
    fn frame(&self, c: &mut Canvas, r: Rect, _radius: i32, gray: u8, filled: bool, t: i32) {
        if filled {
            canvas_fill(c, r, gray);
        } else if gray < 64 && r.w.min(r.h) < ui_px(180.0) && r.w > ui_px(40.0) && r.h > ui_px(40.0) {
            // a small box (a key, a chip): the hairline alone, the corners would crowd it
            canvas_rect_outline(c, r, ui_px(2.0).max(1), CS_LINE);
        } else if gray < 64 && r.w > ui_px(40.0) && r.h > ui_px(40.0) {
            canvas_rect_outline(c, r, ui_px(2.0).max(1), CS_LINE);
            cs_brackets(c, r, ui_px(14.0).min(r.h / 4).min(r.w / 4), ui_px(2.0).max(t.min(ui_px(3.0))), CS_INK);
        } else {
            canvas_rect_outline(c, r, t.max(1), gray);
        }
    }
    // a hairline between two crosshairs
    fn rule(&self, c: &mut Canvas, x0: i32, x1: i32, y: i32, _t: i32, gray: u8) {
        let arm = ui_px(14.0);
        canvas_hline(c, x0 + arm * 2, x1 - arm * 2, y, ui_px(2.0), if gray == 0 { CS_TL } else { gray });
        cs_cross(c, x0 + arm, y, arm, ui_px(2.0), if gray == 0 { CS_INK } else { gray });
        cs_cross(c, x1 - arm, y, arm, ui_px(2.0), if gray == 0 { CS_INK } else { gray });
    }
    fn icon(&self, c: &mut Canvas, k: IconKind, cp: u32, r: Rect, gray: u8) {
        let inset = r.w.min(r.h) / 10;
        if let Some(rows) = sign_rows(k) {
            cs_dot_pattern(c, rect_inflate(r, -inset), rows, 9, gray);
            return;
        }
        cs_dot_icon(c, rect_inflate(r, -inset / 2), gray, |m, mr| font_draw_icon(m, FontFace::Regular, cp, rect_inflate(mr, -inset / 2), 0));
    }
    fn sign(&self, c: &mut Canvas, r: Rect, draw: &dyn Fn(&mut Canvas, Rect, u8)) {
        cs_dot_icon(c, r, CS_INK, |m, mr| draw(m, mr, 0));
    }
    // (the selection is gray, so its text stays ink)
    fn style_fg(&self, st: ButtonStyle) -> u8 {
        match st {
            ButtonStyle::Disabled => if ui_bare() { CS_LINE } else { CS_TL },
            _ => CS_INK,
        }
    }
    // brackets on clean paper
    fn panel_frame(&self, c: &mut Canvas, p: Rect, _radius: i32) {
        canvas_fill(c, p, 255);
        cs_brackets(c, p, ui_px(30.0), ui_px(5.0), CS_INK);
    }
    fn panel_divider(&self, c: &mut Canvas, x: i32, w: i32, y: i32, _gray: u8) {
        cs_cross(c, x + w / 2, y, ui_px(14.0), ui_px(2.0), CS_TL);
    }
    fn panel_head_rule(&self, c: &mut Canvas, p: Rect, y: i32) {
        cs_cross(c, p.x + p.w / 2, y - ui_px(2.0), ui_px(9.0), ui_px(2.0), CS_TL);
    }
    // a counter's digits, no chip: "1 / 4" as a tape counter shows it
    fn counter_chip(&self, c: &mut Canvas, r: Rect, px: f32, text: &str) {
        font_draw_centered(c, FontFace::Fixed, px, r, text, CS_INK);
    }
    fn button(&self, c: &mut Canvas, r: Rect, icon: IconKind, label: &str, st: ButtonStyle, text_px: f32) {
        if label.is_empty() && matches!(st, ButtonStyle::Plain | ButtonStyle::Disabled) {
            // a sign alone (close, back, an arrow) needs no frame: the sign is the control
            canvas_fill(c, r, 255);
        } else {
            panel(c, r, st);
        }
        let fg = self.style_fg(st);
        let caps = label.to_uppercase();
        let inner = rect_inflate(r, -ui_px(8.0));
        let li = label_inset(false, st);
        // small signs and small words with room round them: the clean console look
        if icon != IconKind::None && !caps.is_empty() {
            let ts = icon_label_px(inner.h) * 0.82;
            let wide_ts = ts.max((ui_small_size() * 0.82).min(inner.h as f32 * 0.32));
            let isz = (inner.h * 4 / 10).min(ui_px(48.0));
            let gap = ui_px(22.0);
            // a wide button whose word fits beside its icon: the pair side by side, centred (the mockups'
            // RENAME); else the icon over the word, as in the other themes
            if r.w >= 2 * r.h && font_measure(FontFace::Caps, wide_ts, &caps) <= r.w - 2 * li - isz - gap {
                let ts = wide_ts;
                let tw = font_measure(FontFace::Caps, ts, &caps).min(r.w - 2 * li - isz - gap);
                let x = r.x + (r.w - isz - gap - tw) / 2;
                ui_icon(c, icon, Rect::new(x, r.y + (r.h - isz) / 2, isz, isz), fg);
                ui_text(c, FontFace::Caps, ts, ts.min(ui_small_size()), Rect::new(x + isz + gap, r.y, tw + 2, r.h), &caps, 1, false, fg);
            } else {
                let lh = (ts * 1.25) as i32;
                let ir = Rect { h: inner.h - lh, ..inner };
                let side = (ir.w.min(ir.h) * 6 / 10).min(ui_px(52.0));
                ui_icon(c, icon, Rect::new(ir.x + (ir.w - side) / 2, ir.y + (ir.h - side) / 2 + ui_px(4.0), side, side), fg);
                font_draw_centered(c, FontFace::Caps, ts, Rect::new(r.x + li, inner.y + inner.h - lh, r.w - 2 * li, lh), &caps, fg);
            }
        } else if icon != IconKind::None {
            // (a toolbar's cells keep their signs a little larger: they are read at a glance while writing)
            let k = if ui_bare() { 0.62 } else { 0.5 };
            let side = ((inner.w.min(inner.h) as f32 * k) as i32).clamp(ui_px(24.0).min(inner.w.min(inner.h)), ui_px(64.0));
            ui_icon(c, icon, Rect::new(inner.x + (inner.w - side) / 2, inner.y + (inner.h - side) / 2, side, side), fg);
        } else if !caps.is_empty() {
            let ts = if text_px > 0.0 { text_px } else { text_label_px(inner.h) * 0.78 };
            ui_text(c, FontFace::Caps, ts, if text_px > 0.0 { ts } else { ts.min(ui_small_size()) },
                Rect { x: r.x + li, y: inner.y, w: r.w - 2 * li, h: inner.h }, &caps, 2, true, fg);
        }
    }
    // the word alone (the mockups' row of sheet actions; gray when selected or primary)
    fn button_bare(&self, c: &mut Canvas, r: Rect, label: &str, st: ButtonStyle) {
        if st == ButtonStyle::Primary || st == ButtonStyle::Selected {
            ui_button(c, r, IconKind::None, label, st);
            return;
        }
        canvas_fill(c, r, 255);
        let inner = rect_inflate(r, -ui_px(8.0));
        let ts = text_label_px(inner.h).min(ui_px(36.0) as f32);
        let li = label_inset(false, st);
        ui_text(c, FontFace::Caps, ts, ts.min(ui_small_size()), Rect { x: r.x + li, y: inner.y, w: r.w - 2 * li, h: inner.h },
            &label.to_uppercase(), 2, true, self.style_fg(st));
    }
    fn hollow_colors(&self, st: ButtonStyle) -> (u8, u8) {
        let bg = if matches!(st, ButtonStyle::Selected | ButtonStyle::Primary) {
            if ui_bare() { CS_LINE } else { CS_SEL }
        } else {
            255
        };
        (self.style_fg(st), bg)
    }
    // a list row is a line of type over a hairline, the chosen one a gray band
    fn card(&self, c: &mut Canvas, r: Rect, st: ButtonStyle) {
        match st {
            ButtonStyle::Selected => canvas_fill(c, r, CS_SEL),
            _ => {
                canvas_fill(c, r, 255);
                canvas_hline(c, r.x, r.x + r.w, r.y + r.h - ui_px(2.0), ui_px(2.0).max(1), CS_LINE);
                canvas_fill(c, Rect::new(r.x, r.y + r.h - ui_px(2.0), ui_px(12.0), ui_px(2.0).max(1)), if st == ButtonStyle::Disabled { CS_LINE } else { CS_INK });
            }
        }
    }
    // a slide switch on a tape deck: a slot, the cap at the side it is on, the on side shaded
    fn switch(&self, c: &mut Canvas, track: Rect, sw_h: i32, on: bool) {
        let t = ui_px(3.0);
        canvas_fill(c, track, if on { CS_SEL } else { 255 });
        canvas_rect_outline(c, track, t, CS_INK);
        let kw = sw_h - 4 * t;
        let kx = if on { track.x + track.w - 2 * t - kw } else { track.x + 2 * t };
        let k = Rect::new(kx, track.y + 2 * t, kw, kw);
        canvas_fill(c, k, if on { CS_INK } else { 255 });
        canvas_rect_outline(c, k, t, CS_INK);
        for i in 1..=3 {
            canvas_vline(c, k.x + k.w * i / 4, k.y + k.h / 4, k.y + k.h * 3 / 4, ui_px(2.0), if on { 255 } else { CS_INK });
        }
    }
    // a fader: a hairline slot with ticks, ink up to a square cap
    fn slider_track(&self, c: &mut Canvas, tr: Rect, fx: i32, _cy: f32) {
        let mid = tr.y + tr.h / 2;
        canvas_hline(c, tr.x, tr.x + tr.w, mid - ui_px(1.0), ui_px(2.0), CS_TL);
        for i in 0..=10 {
            let x = tr.x + tr.w * i / 10;
            canvas_vline(c, x - ui_px(1.0), mid - ui_px(if i % 5 == 0 { 16.0 } else { 9.0 }), mid + ui_px(if i % 5 == 0 { 16.0 } else { 9.0 }), ui_px(2.0), CS_TL);
        }
        canvas_fill(c, Rect::new(tr.x, mid - ui_px(4.0), fx - tr.x, ui_px(8.0)), CS_INK);
        let k = self.slider_knob_radius();
        let kr = Rect::new(fx - k / 2, mid - k, k, 2 * k);
        canvas_fill(c, kr, 255);
        canvas_rect_outline(c, kr, ui_px(4.0), CS_INK);
        canvas_hline(c, kr.x + ui_px(6.0), kr.x + kr.w - ui_px(6.0), mid - ui_px(1.0), ui_px(3.0), CS_INK);
    }
    // the screen behind dimmed, the sheet clean paper over it (where it was just drawn)
    fn sheet(&self, c: &mut Canvas, card: Rect, title: &str) -> Rect {
        if ui_backdrop() {
            cs_backdrop(c, card);
        }
        canvas_fill(c, card, 255);
        cs_brackets(c, card, ui_px(42.0), ui_px(5.0), CS_INK);
        if title.is_empty() {
            return ui_sheet_inner(card, false);
        }
        sheet_title(c, Rect::new(card.x + ui_px(56.0), card.y + ui_px(24.0), card.w - ui_px(112.0), ui_px(112.0)), title);
        ui_sheet_inner(card, true)
    }
    // the same small spaced capitals as a sheet's title, sprayed at the end, left-aligned, centred vertically
    fn page_title(&self, c: &mut Canvas, r: Rect, title: &str) {
        tracked_title(c, r, title, "MOD");
    }
    // a screen's header (Files, a folder) as a page's heading
    fn header_title(&self, c: &mut Canvas, r: Rect, title: &str) -> bool {
        tracked_title(c, r, title, "DIR");
        true
    }
    // an ink plate with paper brackets inside its corners
    fn toast_plate(&self, c: &mut Canvas, r: Rect, _ends: i32) {
        canvas_fill(c, r, CS_INK);
        cs_brackets(c, rect_inflate(r, -ui_px(12.0)), ui_px(22.0), ui_px(3.0), 255);
    }
    fn checkbox_box(&self, c: &mut Canvas, b: Rect, checked: bool) {
        canvas_rect_outline(c, b, ui_px(4.0), CS_INK);
        if checked {
            canvas_fill(c, rect_inflate(b, -ui_px(13.0)), CS_INK);
        }
    }
    // a row of keys on one hairline, the chosen one a gray cap, hairline ticks between
    fn segmented(&self, c: &mut Canvas, r: Rect, n: i32, on: i32, seg: &dyn Fn(i32) -> Rect) -> bool {
        canvas_fill(c, r, 255);
        if (0..n).contains(&on) {
            canvas_fill(c, seg(on), CS_SEL);
        }
        ui_card(c, r, ButtonStyle::Plain);
        let w = r.w / n;
        for i in 1..n {
            canvas_vline(c, r.x + i * w, r.y + r.h / 4, r.y + r.h * 3 / 4, ui_px(2.0).max(1), CS_LINE);
        }
        true
    }
    // (gray selection, its text ink)
    fn menu_row_card(&self, c: &mut Canvas, r: Rect, st: ButtonStyle) -> bool {
        ui_card(c, r, st);
        true
    }

    // The frame is thin and gray, thick and ink when selected; the selected tile's cell is shaded, its
    // label's room with it.
    fn tile_frame(&self, c: &mut Canvas, tile: Rect, gap: i32, cell_h: i32, sel: bool) {
        if sel {
            let cell = Rect::new(tile.x - gap / 2, tile.y - gap / 2, tile.w + gap, cell_h);
            canvas_fill(c, Rect::new(cell.x, cell.y, cell.w, tile.y - cell.y), CS_SEL);
            canvas_fill(c, Rect::new(cell.x, tile.y + tile.h, cell.w, cell.y + cell.h - tile.y - tile.h), CS_SEL);
            canvas_fill(c, Rect::new(cell.x, tile.y, tile.x - cell.x, tile.h), CS_SEL);
            canvas_fill(c, Rect::new(tile.x + tile.w, tile.y, cell.x + cell.w - tile.x - tile.w, tile.h), CS_SEL);
        }
        canvas_rect_outline(c, tile, if sel { ui_px(5.0) } else { ui_px(2.0) }, if sel { CS_INK } else { CS_TL });
    }
    // the tile's number before its label, as a catalogue does: "01 Narrow, no margin", left-aligned,
    // the number dim
    fn tile_label(&self, c: &mut Canvas, tr: Rect, gap: i32, n: i32, label: &str, selected: bool) -> bool {
        let lf = if selected { FontFace::Bold } else { FontFace::Regular };
        let ls = ui_small_size() * 0.9;
        let mut x = tr.x + ui_px(4.0);
        if n > 0 {
            let num = format!("{n:02}");
            let nw = font_measure(FontFace::Fixed, ls, &num);
            font_draw_left(c, FontFace::Fixed, ls, Rect::new(x, tr.y + tr.h + ui_px(12.0), nw + 2, ui_px(40.0)), &num, CS_DIM);
            x += nw + ui_px(14.0);
        }
        let w = tr.x + tr.w + gap / 2 - x;
        if font_measure(FontFace::Regular, ls, label) <= w {
            font_draw_left(c, lf, ls, Rect::new(x, tr.y + tr.h + ui_px(12.0), w, ui_px(40.0)), label, CS_INK);
        } else {
            // two lines; a label that two lines cannot hold ("Tablatura de guitarra") takes a smaller
            // size, down to three quarters, so it is never cut
            let mut px = ls;
            let too_wide = |px: f32| label.split(' ').any(|word| font_measure(lf, px, word) > w);
            while px > ls * 0.75 && (font_wrap_lines(lf, px, label, w, None) > 2 || too_wide(px)) {
                px -= 0.5;
            }
            font_draw_wrapped(c, lf, px, Rect::new(x, tr.y + tr.h + ui_px(12.0), w, ui_px(80.0)), label, 2, ui_px(40.0), false, CS_INK);
        }
        true
    }
    // crosshairs only at the grid's four outer corners: registration marks, not a lattice
    fn grid_marks(&self, c: &mut Canvas, x0: i32, y0: i32, g: &PaperGrid, gap: i32) {
        for r in [0, g.rows] {
            for col in [0, g.cols] {
                cs_cross(c, x0 - gap / 2 + col * (g.tw + gap), y0 - gap / 2 + r * g.cell_h, ui_px(16.0), ui_px(2.0), CS_TL);
            }
        }
    }

    // plain: paper with a hairline along its top, the field a line to type on
    fn keyboard_bed(&self, c: &mut Canvas, area: Rect, f: Rect) {
        canvas_fill(c, area, 255);
        canvas_hline(c, area.x, area.x + area.w, area.y, ui_px(2.0).max(1), CS_LINE);
        canvas_fill(c, f, 255);
        canvas_hline(c, f.x, f.x + f.w, f.y + f.h - ui_px(2.0), ui_px(2.0).max(1), CS_INK);
    }
    // plain keys: the letters on paper, a key held (or Shift, or Done) a gray cap
    fn key_cap(&self, c: &mut Canvas, r: Rect, st: ButtonStyle) -> u8 {
        canvas_fill(c, r, if matches!(st, ButtonStyle::Selected | ButtonStyle::Primary) { CS_SEL } else { 255 });
        CS_INK
    }
    fn status_box(&self, c: &mut Canvas, r: Rect) {
        canvas_fill(c, r, 255);
        cs_brackets(c, r, ui_px(18.0).min(r.h / 3), ui_px(4.0), CS_INK);
    }
    // each lit unit a dot, the battery a dot-matrix sign
    fn battery_units(&self, c: &mut Canvas, r: Rect, u: i32, gray: u8) {
        if gray != 0 {
            canvas_fill(c, r, gray);
            return;
        }
        for y in (r.y..r.y + r.h).step_by(u.max(1) as usize) {
            for x in (r.x..r.x + r.w).step_by(u.max(1) as usize) {
                canvas_disc_aa(c, x as f32 + u as f32 / 2.0, y as f32 + u as f32 / 2.0, u as f32 * 0.42, 0, 1.0);
            }
        }
    }
    // a ring of dots: the selection and the open page as a plotter marks them
    fn map_ring(&self, c: &mut Canvas, x: f32, y: f32, r: f32, t: f32) -> bool {
        if r <= ui_px(10.0) as f32 {
            return false;
        }
        cs_dot_ring(c, x, y, r - t / 2.0, (t * 0.55).clamp(1.5, ui_px(5.0) as f32));
        true
    }
    // a link as a trace: a gray hairline, ink when it is the chosen node's (wider for more links)
    fn map_link(&self, c: &mut Canvas, a: (f32, f32), b: (f32, f32), width: f32, selected: bool) -> bool {
        canvas_line_aa(c, a.0, a.1, b.0, b.1, (width * 0.6).max(1.5), if selected { CS_INK } else { CS_DIM });
        true
    }
    // a target's ticks outside the chosen node's ring: locked on
    fn map_lock(&self, c: &mut Canvas, x: f32, y: f32, r: f32) {
        if r > ui_px(10.0) as f32 {
            cs_lock_ticks(c, x, y, r + 12.0);
        }
    }
    fn map_node(&self, c: &mut Canvas, x: f32, y: f32, r: f32, level: i32) -> bool {
        if r < ui_px(14.0) as f32 {
            return false;
        }
        cs_node(c, x, y, r, level);
        true
    }

    // the app's name as a maker's mark: small spaced capitals, the mark and the name sprayed into grain at
    // both ends, as a worn photocopy
    fn library_mark(&self, c: &mut Canvas, r: Rect, title: &str, x0: i32, x: i32, header_h: i32) -> Option<i32> {
        let name_w = cs_tracked(c, FontFace::Title, ui_px(46.0) as f32, r, &title.to_uppercase(), ui_px(14.0), CS_INK);
        let ys = ui_px(36.0);
        cs_spray_at(c, Rect::new(x0 - ui_px(12.0), ys, x + name_w + ui_px(24.0) - (x0 - ui_px(12.0)), header_h - ys - ui_px(24.0)), x0, x + name_w, 7, ui_px(6.0));
        Some(name_w)
    }
    // a readout under the rule: what this shelf holds, and a rail to the edge
    fn library_readout(&self, c: &mut Canvas, s: &ShelfInfo) {
        let where_ = if s.folder.is_empty() { "ROOT".to_string() } else { s.folder.to_uppercase() };
        let line = format!("ARCHIVE // {where_} // {:02} NB // {:02} DIR", s.notebooks, s.folders);
        let ry = s.header_h + ui_px(14.0);
        let w = cs_tracked(c, FontFace::Fixed, ui_px(21.0) as f32, Rect::new(s.margin, ry, s.w / 2, ui_px(26.0)), &line, ui_px(3.0), CS_DIM);
        cs_rail(c, s.margin + w + ui_px(30.0), s.w - s.margin, ry + ui_px(13.0), "");
    }
    // PAGE 1 / 1: the word as a caption (the translation's, before its number), the count in figures
    fn library_pager(&self, c: &mut Canvas, pr: Rect, nr: Rect, page: i32, pages: i32) -> bool {
        let fmt = tr("Page %d of %d");
        let word = fmt.split("%d").next().unwrap_or("").trim();
        let count = format!("{} / {}", page + 1, pages);
        let px = ui_text_size();
        let cw = font_measure(FontFace::Fixed, px, &count);
        let ww = font_measure(FontFace::Caps, ui_small_size() * 0.8, &word.to_uppercase()) + word.chars().count() as i32 * ui_px(6.0);
        let mid = Rect::new(pr.x + pr.w, pr.y, nr.x - pr.x - pr.w, pr.h);
        let x = mid.x + (mid.w - ww - ui_px(24.0) - cw) / 2;
        caption(c, Rect::new(x, mid.y, ww, mid.h), word);
        font_draw_left(c, FontFace::Fixed, px, Rect::new(x + ww + ui_px(24.0), mid.y, cw + 2, mid.h), &count, CS_INK);
        true
    }
    // The frame of instruments in the margin round a screen (the Library and its Settings, Files, Pen feel,
    // the editor with a sheet up), with what the system knows (version, panel, what the screen lists, date,
    // time, battery), then the edge of the picture out of focus. Only on a whole redraw: the band is
    // softened once over what was just drawn.
    fn screen_overlay(&self, c: &mut Canvas, info: &ScreenInfo) {
        let t = sys_wall_clock();
        let tm = local_tm(t);
        let hm = format!("{:02}:{:02}", tm.tm_hour, tm.tm_min);
        let pct = status_battery_percent();
        let left = format!("SYS.GRIMOIRE {} // {}x{} @{}DPI // IDX {:02}", GRIMOIRE_VERSION.to_uppercase(), info.w, info.h, info.dpi.round() as i32, info.items);
        let right = format!(
            "{:02}.{:02}.{} // {} // PWR {}",
            tm.tm_mday,
            tm.tm_mon + 1,
            tm.tm_year + 1900,
            hm,
            if pct >= 0 { format!("{pct:02}%") } else { "--".to_string() }
        );
        let band = ui_px(46.0);
        cs_hud(c, Rect::new(0, 0, info.w, info.h), band, &left, &right);
        // (the focus falls off a little inside the margin too, so the edge of the content softens with it)
        cs_defocus(c, ui_px(64.0));
    }
    // PAPER, spaced out, and the paper chosen at the right: "▸ Narrow, no margin"
    fn new_notebook_heading(&self, c: &mut Canvas, hr: Rect, chosen: &str) -> bool {
        let mut x = hr.x;
        for ch in tr("Paper").to_uppercase().chars() {
            let s = ch.to_string();
            let w = font_measure(FontFace::Caps, ui_small_size(), &s);
            font_draw_left(c, FontFace::Caps, ui_small_size(), Rect::new(x, hr.y, w + 2, hr.h), &s, CS_INK);
            x += w + ui_px(10.0);
        }
        let line = format!("\u{25B8} {chosen}");
        let px = ui_small_size() * 0.9;
        let w = font_measure(FontFace::Regular, px, &line).min(hr.w / 2);
        font_draw_left_name(c, FontFace::Regular, px, Rect::new(hr.x + hr.w - w, hr.y, w, hr.h), &line, CS_INK);
        true
    }
    // NAME and IN captions with their values (the name in large type with a block caret), the Rename
    // button beside them, and the four actions in one row, Create gray.
    fn new_notebook_foot(&self, c: &mut Canvas, in_: Rect, y: i32, name: &str, folder: &str, tpl: bool) -> Option<NewFoot> {
        let cap_w = ui_px(110.0);
        // the name block halfway between the papers and the actions
        let y = (y + in_.y + in_.h - ui_px(140.0)) / 2 - ui_px(80.0);
        let rn = Rect::new(in_.x + in_.w - ui_px(360.0), y + ui_px(10.0), ui_px(360.0), ui_px(116.0));
        let vx = in_.x + cap_w;
        let vw = rn.x - ui_px(40.0) - vx;
        let row1 = Rect::new(in_.x, y + ui_px(4.0), in_.w, ui_px(80.0));
        caption(c, Rect::new(row1.x, row1.y, cap_w, row1.h), caption_of(tr("Name: %s")));
        let npx = ui_px(58.0) as f32;
        let nw = font_measure(FontFace::FixedBold, npx, name).min(vw - ui_px(40.0));
        font_draw_left_name(c, FontFace::FixedBold, npx, Rect::new(vx, row1.y, nw + 2, row1.h), name, CS_INK);
        let ch = font_ascent(FontFace::FixedBold, npx);
        canvas_fill(c, Rect::new(vx + nw + ui_px(18.0), row1.y + (row1.h - ch) / 2, ui_px(20.0), ch), CS_INK);
        let row2 = Rect::new(in_.x, row1.y + row1.h, in_.w, ui_px(60.0));
        caption(c, Rect::new(row2.x, row2.y, cap_w, row2.h), caption_of(tr("In: %s")));
        let place = if folder.is_empty() { tr("Library (top level)") } else { folder };
        font_draw_left_name(c, FontFace::Regular, ui_small_size(), Rect::new(vx, row2.y, vw, row2.h), place, CS_INK);
        ui_button(c, rn, IconKind::Pencil, tr("Rename"), ButtonStyle::Plain);
        // the four actions: Cancel, More layouts..., From a file..., Create
        let bh = ui_px(140.0);
        let gap = ui_px(24.0);
        let bw = (in_.w - 3 * gap) / 4;
        let by = in_.y + in_.h - bh;
        let at = |i: i32| Rect::new(in_.x + i * (bw + gap), by, bw, bh);
        self.button_bare(c, at(0), tr("Cancel"), ButtonStyle::Plain);
        self.button_bare(c, at(1), tr("More layouts..."), if tpl { ButtonStyle::Selected } else { ButtonStyle::Plain });
        self.button_bare(c, at(2), tr("From a file..."), ButtonStyle::Plain);
        let create = format!("{} \u{2192}", tr("Create"));
        ui_button(c, at(3), IconKind::None, &create, ButtonStyle::Primary);
        Some(NewFoot { rename: rn, cancel: at(0), more: at(1), file: at(2), create: at(3) })
    }
}

/// A control's surface: plain ones are corner brackets on paper, selected and primary ones a gray panel,
/// disabled ones faint brackets. In a toolbar (ui_with_bare_buttons) the cells are bare: no brackets
/// each (the toolbar has its own), the selected one a gray cell.
fn panel(c: &mut Canvas, r: Rect, st: ButtonStyle) {
    if ui_bare() {
        canvas_fill(c, r, if matches!(st, ButtonStyle::Selected | ButtonStyle::Primary) { CS_LINE } else { 255 });
        return;
    }
    // short, thin corners: the control is outlined by where its corners are, no more
    let arm = ui_px(16.0).min(r.h / 4).max(ui_px(8.0));
    let t = ui_px(2.0).max(1);
    match st {
        ButtonStyle::Primary | ButtonStyle::Selected => canvas_fill(c, r, CS_SEL),
        ButtonStyle::Disabled => {
            canvas_fill(c, r, 255);
            cs_brackets(c, r, arm, t, CS_LINE);
        }
        ButtonStyle::Plain => {
            canvas_fill(c, r, 255);
            cs_brackets(c, r, arm, t, CS_INK);
        }
    }
}

/// The hand-set signs: the icons a glyph turns to mush in a few dots, drawn dot by dot on a 9 by 9 grid
/// as the mockups have them.
fn sign_rows(k: IconKind) -> Option<&'static [&'static str]> {
    const COLLAPSE: [&str; 7] = ["X...X..", ".X...X.", "..X...X", ".X...X.", "X...X..", "", ""];
    const EXPAND: [&str; 7] = ["..X...X", ".X...X.", "X...X..", ".X...X.", "..X...X", "", ""];
    const GRIP: [&str; 5] = ["X.X.X", ".....", "X.X.X", ".....", "X.X.X"];
    const PREV: [&str; 9] = ["....X", "...Xo", "..Xo.", ".Xo..", "Xo...", ".Xo..", "..Xo.", "...Xo", "....X"];
    const NEXT: [&str; 9] = ["X....", "oX...", ".oX..", "..oX.", "...oX", "..oX.", ".oX..", "oX...", "X...."];
    const UP: [&str; 5] = ["....X....", "...XoX...", "..Xo.oX..", ".Xo...oX.", "Xo.....oX"];
    const DOWN: [&str; 5] = ["Xo.....oX", ".Xo...oX.", "..Xo.oX..", "...XoX...", "....X...."];
    const PLUS: [&str; 9] = ["....X....", "....X....", "....X....", "....X....", "XXXXoXXXX", "....X....", "....X....", "....X....", "....X...."];
    const MINUS: [&str; 1] = ["XXXXXXXXX"];
    const CLOSE: [&str; 9] = ["X.......X", ".X.....X.", "..X...X..", "...X.X...", "....o....", "...X.X...", "..X...X..", ".X.....X.", "X.......X"];
    const CHECK: [&str; 7] = [".......X", "......X.", ".....X..", "X...X...", ".X.X....", "..X.....", ""];
    const BACK: [&str; 7] = ["...X.....", "..X......", ".X.......", "XXXXXXXXX", ".X.......", "..X......", "...X....."];
    const MORE: [&str; 1] = ["X...X...X"];
    Some(match k {
        IconKind::Collapse => &COLLAPSE,
        IconKind::Expand => &EXPAND,
        IconKind::Grip => &GRIP,
        IconKind::Prev => &PREV,
        IconKind::Next => &NEXT,
        IconKind::Up => &UP,
        IconKind::Down => &DOWN,
        IconKind::Plus => &PLUS,
        IconKind::Minus => &MINUS,
        IconKind::Close => &CLOSE,
        IconKind::Check => &CHECK,
        IconKind::Back => &BACK,
        IconKind::More => &MORE,
        _ => return None,
    })
}

/// A sheet's title in r: a small crosshair, then the words in capitals, small and spaced wide (a
/// premium label, not a headline), with a faint spray on the last letters. A title too long for that (a
/// translation) closes up its spacing, then takes a smaller size, then two lines, as ui_text does.
fn sheet_title(c: &mut Canvas, r: Rect, title: &str) {
    let isz = ui_px(40.0);
    crosshair(c, Rect::new(r.x, r.y + (r.h - isz) / 2, isz, isz), CS_INK);
    let tx = r.x + isz + ui_px(36.0);
    let tw = r.x + r.w - tx;
    let caps = title.to_uppercase();
    let tr = Rect::new(tx, r.y, tw, r.h);
    for (px, track) in [(ui_px(60.0), ui_px(18.0)), (ui_px(60.0), ui_px(9.0)), (ui_px(52.0), ui_px(6.0)), (ui_px(46.0), ui_px(4.0))] {
        let px = px as f32;
        if cs_tracked_w(FontFace::Title, px, &caps, track) <= tw {
            let w = cs_tracked(c, FontFace::Title, px, tr, &caps, track, CS_INK);
            cs_spray(c, Rect::new(tx - ui_px(10.0), r.y, w + ui_px(20.0), r.h), tx + w / 2, tx + w, caps.len() as u32);
            // a leader rail from the title to the sheet's edge, its register number over the end
            cs_rail(c, tx + w + ui_px(40.0), r.x + r.w, r.y + r.h / 2, &format!("REG.{}", cs_serial(title)));
            return;
        }
    }
    ui_text(c, FontFace::Title, ui_px(46.0) as f32, ui_small_size(), tr, &caps, 2, false, CS_INK);
}

/// A heading in r: small spaced capitals, sprayed into grain at their end as a sheet's title is, and a
/// leader rail with its tag (`MOD.2213`) to r's end. Too long for that (a translation, a folder's name): the
/// spacing closes up, then a smaller size, then two lines, as ui_text does.
fn tracked_title(c: &mut Canvas, r: Rect, title: &str, tag: &str) {
    let caps = title.to_uppercase();
    for (px, track) in [(ui_px(50.0), ui_px(14.0)), (ui_px(50.0), ui_px(7.0)), (ui_px(44.0), ui_px(4.0))] {
        let px = px as f32;
        if cs_tracked_w(FontFace::Title, px, &caps, track) <= r.w {
            let w = cs_tracked(c, FontFace::Title, px, r, &caps, track, CS_INK);
            cs_spray(c, Rect::new(r.x - ui_px(10.0), r.y, w + ui_px(20.0), r.h), r.x + w / 2, r.x + w, caps.len() as u32);
            if r.x + w + ui_px(80.0) < r.x + r.w {
                cs_rail(c, r.x + w + ui_px(40.0), r.x + r.w, r.y + r.h / 2, &format!("{tag}.{}", cs_serial(title)));
            }
            return;
        }
    }
    ui_text(c, FontFace::Title, ui_px(44.0) as f32, ui_small_size(), r, &caps, 2, false, CS_INK);
}

/// The crosshair a title starts with: a dot-matrix plus, open at its heart.
fn crosshair(c: &mut Canvas, r: Rect, gray: u8) {
    cs_dot_icon(c, r, gray, |m, mr| {
        let u = (mr.w / 9).max(1);
        let mid = mr.x + mr.w / 2;
        canvas_fill(m, Rect::new(mid - u / 2, mr.y, u, mr.h), 0);
        canvas_fill(m, Rect::new(mr.x, mr.y + mr.h / 2 - u / 2, mr.w, u), 0);
        canvas_fill(m, Rect::new(mid - u * 3 / 2, mr.y + mr.h / 2 - u * 3 / 2, u * 3, u * 3), 255);
        canvas_fill(m, Rect::new(mid - u / 2, mr.y + mr.h / 2 - u / 2, u, u), 0);
    });
}

/// A label's word before its value, from a translation of "Word: %s" ("Name: %s" gives "Name"), printed
/// as a small caption over or beside the value.
fn caption_of(fmt: &str) -> &str {
    fmt.split("%s").next().unwrap_or(fmt).trim_end_matches([':', ' ', '\u{a0}', '：'])
}

/// Such a caption: small capitals, dim, spaced out.
fn caption(c: &mut Canvas, r: Rect, word: &str) {
    let caps = word.to_uppercase();
    let px = ui_small_size() * 0.8;
    // letter-spaced by hand: each character in its own cell
    let mut x = r.x;
    let track = ui_px(6.0);
    for ch in caps.chars() {
        let s = ch.to_string();
        let w = font_measure(FontFace::Caps, px, &s);
        if x + w > r.x + r.w {
            break;
        }
        font_draw_left(c, FontFace::Caps, px, Rect::new(x, r.y, w + 2, r.h), &s, CS_DIM);
        x += w + track;
    }
}
