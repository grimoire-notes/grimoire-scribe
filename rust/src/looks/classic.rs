//! Classic: Terminess (or the family Settings > Font picks, or the owner's own font), rounded corners,
//! plain rules. It is the base look: the trait's own bodies (mod.rs) are Classic's, and the longer
//! ones are here, for the other themes to reuse.

use super::Look;
use crate::prelude::*;
use crate::ui::{carve_in, icon_label_px, label_inset, text_label_px};

pub struct Classic;

impl Look for Classic {
    fn id(&self) -> &'static str {
        "classic"
    }
    fn style_label(&self) -> &'static str {
        n_("Style (Classic)")
    }
}

/// Classic's button (Gothic's too, which carves it and lifts its label): an outline, filled black when
/// selected, a thicker one when primary; its icon over its label, or either alone.
pub fn button<L: Look + ?Sized>(l: &L, c: &mut Canvas, r: Rect, icon: IconKind, label: &str, st: ButtonStyle, text_px: f32) {
    let bg: u8 = if st == ButtonStyle::Selected { 0 } else { 255 };
    let fg = l.style_fg(st);
    let edge: u8 = if st == ButtonStyle::Disabled { crate::globalui::GUI_GRAY_DISABLED } else { 0 };
    let radius = l.corner_radius();
    l.frame(c, r, radius, bg, true, 0);
    l.frame(c, r, radius, edge, false, if st == ButtonStyle::Primary { ui_px(6.0) } else { ui_px(3.0) });
    let carve = l.carved(r.w, r.h);
    if carve {
        // The inner line of a carved panel.
        let inset = carve_in(st);
        canvas_chamfer_rect(
            c,
            rect_inflate(r, -inset),
            radius - inset / 3,
            if st == ButtonStyle::Selected { 255 } else { edge },
            false,
            ui_px(2.0),
        );
    }
    let has_label = !label.is_empty();
    let inner = rect_inflate(r, -ui_px(8.0));
    let li = label_inset(carve, st);
    let lx = r.x + li;
    let lw = r.w - 2 * li; // the label's columns
    if icon != IconKind::None && has_label {
        let ts = icon_label_px(inner.h);
        let lh = (ts * 1.15) as i32;
        let ir = Rect { x: inner.x, y: inner.y, w: inner.w, h: inner.h - lh };
        ui_icon(c, icon, ir, fg);
        let lr = Rect { x: lx, y: inner.y + inner.h - lh - l.label_lift(), w: lw, h: lh };
        font_draw_centered(c, FontFace::Caps, ts, lr, label, fg);
    } else if icon != IconKind::None {
        ui_icon(c, icon, rect_inflate(inner, -ui_px(6.0)), fg);
    } else if has_label {
        // A label too long for the button (a translation) takes the small size, then two lines.
        let ts = if text_px > 0.0 { text_px } else { text_label_px(inner.h) };
        ui_text(
            c,
            FontFace::Caps,
            ts,
            if text_px > 0.0 { ts } else { ts.min(ui_small_size()) },
            Rect { x: lx, y: inner.y, w: lw, h: inner.h },
            label,
            2,
            true,
            fg,
        );
    }
}

/// Classic's sheet: a rounded card with a thick border, its title left in Bold.
pub fn sheet(c: &mut Canvas, card: Rect, title: &str) -> Rect {
    canvas_round_rect(c, card, ui_px(24.0), 255, true, 0);
    canvas_round_rect(c, card, ui_px(24.0), 0, false, ui_px(6.0));
    let th = ui_px(120.0);
    if !title.is_empty() {
        font_draw_left(c, FontFace::Bold, ui_title_size(), Rect { x: card.x + ui_px(40.0), y: card.y, w: card.w - ui_px(80.0), h: th }, title, 0);
        return ui_sheet_inner(card, true);
    }
    ui_sheet_inner(card, false)
}
