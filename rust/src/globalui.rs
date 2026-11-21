//! globalui.rs: the global UI elements (2.3, issue #209). One set of controls for every screen and
//! sheet, in the theme in use: Classic draws the modern rounded design (filled white surfaces, large
//! radii, pill chips and fields), Gothic draws the carved one (chamfered corners, inner rules and
//! lozenges). A screen never draws a Classic and a Gothic version of the same control: it calls the
//! element here, or the one in ui.rs built from the same pieces, and the theme is picked once.
//!
//! The elements are stateless, as ui.rs's are: a screen owns its layout, hits and pressed look, draws
//! a `gui_*` element where a control belongs, and adds the hit for its rect. That keeps a new menu
//! cheap to add, and keeps every menu looking like the Settings page it was modelled on.
//!
//! The kit covers the controls the redesign asks for: buttons and icon buttons, chips and segmented
//! filters, text fields (the on-screen keyboard is the existing themed `keyboard_draw`), selects,
//! sliders (ui_slider_draw through `gui_slider`), tooltips, slide-up sheets and their rows, floating
//! action buttons, app tiles and the status bar. Sizes follow `ui_px`, so a Kindle at another DPI
//! gets the same design at its scale.

use crate::prelude::*;
use crate::status::{battery_from_sysfs, Battery, STATUS_SIZES};
use std::cell::Cell;
use crate::ui::{
    ui_button, ui_card, ui_frame, ui_icon, ui_slider_draw, ui_small_size, ui_text,
    ui_text_size, ui_title_size, ButtonStyle, IconKind, Slider,
};

/// Whether the theme is the carved one.
pub fn gui_gothic() -> bool {
    ui_theme() == UiTheme::Gothic
}

/// The one corner radius of the kit's cards and fields.
pub fn gui_radius() -> i32 {
    if gui_gothic() {
        ui_px(16.0)
    } else {
        ui_px(24.0)
    }
}

/// The room between two elements of a screen, and around them.
pub fn gui_gap() -> i32 {
    ui_px(24.0)
}
pub fn gui_margin() -> i32 {
    ui_px(50.0)
}

/// The top status bar's scale at Clock size small, medium and large. Medium is the bar as it has
/// always been; the Clock settings size scales all of it, not just its clock.
const STATUS_SCALE: [f32; 3] = [0.82, 1.0, 1.18];

thread_local! {
    /// The Clock settings the top status bar draws with (size, 12-hour time).
    static STATUS_STYLE: Cell<(i32, bool)> = const { Cell::new((1, false)) };
}

/// Applies Clock settings to every top status bar (Settings > Sleep & Clock > Clock settings). The
/// main page and the Clock page call this when either choice changes.
pub fn gui_status_style(size: i32, twelve: bool) {
    STATUS_STYLE.with(|s| s.set((clampi(size, 0, STATUS_SIZES - 1), twelve)));
}

fn status_style() -> (i32, bool) {
    STATUS_STYLE.with(|s| s.get())
}

fn status_scale() -> f32 {
    STATUS_SCALE[status_style().0 as usize]
}

/// The status bar's height (`gui_status_bar`): the whole bar grows and shrinks with Clock settings'
/// size.
pub fn gui_status_h() -> i32 {
    ui_px(78.0 * status_scale())
}

/// The header under the status bar: its title and its round action buttons.
pub fn gui_head_h() -> i32 {
    ui_px(150.0)
}

/// The header's round action buttons, centred in it.
pub fn gui_top_button_h() -> i32 {
    ui_px(96.0)
}

/// Where a header's title, Back and actions are.
pub struct GuiHeader {
    /// The Back button at the left (empty when the screen has none).
    pub back: Rect,
    /// The title, between Back and the actions.
    pub title: Rect,
    /// The first row under the header: a screen's content starts here.
    pub foot: i32,
}

/// The one header under the status bar: the title at the left, an optional Back button, and evenly
/// separated icon actions at the right. The actions' hits are returned in the order given, so a
/// screen adds the ids it already owns. Every full screen uses this, so its title and buttons stand
/// where every other screen's do.
pub fn gui_header(
    c: &mut Canvas,
    aw: i32,
    title: &str,
    back: bool,
    actions: &[(IconKind, i32)],
) -> (GuiHeader, Vec<(Rect, i32)>) {
    gui_status_bar(c, aw, wall_time());
    let m = gui_margin();
    let top = gui_status_h();
    let th = gui_head_h();
    let bh = gui_top_button_h();
    // The title and every round button share one vertical axis: the buttons' centres are the title
    // band's centre, whichever theme and title size is in use.
    let by = top + (th - bh) / 2;
    let mut rx = aw - m;
    let mut rects = Vec::with_capacity(actions.len());
    for &(icon, id) in actions.iter().rev() {
        let r = Rect::new(rx - bh, by, bh, bh);
        gui_icon_button(c, r, icon, ButtonStyle::Plain);
        rects.push((r, id));
        rx = r.x - ui_px(12.0);
    }
    rects.reverse();
    let mut back_r = RECT_EMPTY;
    let mut tx = m;
    if back {
        back_r = Rect::new(m, by, bh, bh);
        gui_icon_button(c, back_r, IconKind::Back, ButtonStyle::Plain);
        tx = back_r.x + bh + ui_px(22.0);
    }
    let title_r = Rect::new(tx, top, (rx - tx - ui_px(20.0)).max(0), th);
    gui_page_title(c, title_r, title);
    (GuiHeader { back: back_r, title: title_r, foot: top + th }, rects)
}

/// A screen's title in the modern header's type: the same size and position on the home, Apps,
/// Files and every Settings page.
pub fn gui_page_title(c: &mut Canvas, r: Rect, title: &str) {
    let px = ui_title_size() * if gui_gothic() { 1.75 } else { 1.1 };
    if gui_gothic() {
        font_draw_left_name(c, FontFace::Title, px, r, title, 0);
    } else {
        ui_text(c, FontFace::Title, px, ui_small_size(), r, title, 1, false, 0);
    }
}

/// A surface the kit's rows and cards sit on. `filled` is the black of a chosen element; the outline
/// is the theme's (a cut corner in Gothic, a round one in Classic).
pub fn gui_surface(c: &mut Canvas, r: Rect, filled: bool, thick: i32) {
    if filled {
        ui_frame(c, r, gui_radius(), 0, true, 0);
        if gui_gothic() {
            ui_frame(c, rect_inflate(r, -ui_px(10.0)), gui_radius(), 255, false, ui_px(2.0));
        }
    } else {
        ui_frame(c, r, gui_radius(), 255, true, 0);
        ui_frame(c, r, gui_radius(), 0, false, thick);
    }
}

/// The kit's text button: an icon and a label, or either alone. Classic is a rounded pill with the
/// label in the modern face; Gothic is ui.rs's carved button, so every Gothic menu stays carved.
pub fn gui_button(c: &mut Canvas, r: Rect, icon: IconKind, label: &str, st: ButtonStyle) {
    if gui_gothic() {
        ui_button(c, r, icon, label, st);
        return;
    }
    if r.w <= 0 || r.h <= 0 {
        return;
    }
    let filled = matches!(st, ButtonStyle::Selected | ButtonStyle::Primary);
    let fg: u8 = if st == ButtonStyle::Disabled {
        170
    } else if filled {
        255
    } else {
        0
    };
    let edge = if st == ButtonStyle::Disabled { 170 } else { 0 };
    let rad = (r.h / 2).min(ui_px(30.0));
    ui_frame(c, r, rad, if filled { 0 } else { 255 }, true, 0);
    ui_frame(c, r, rad, edge, false, if st == ButtonStyle::Primary { ui_px(5.0) } else { ui_px(3.0) });
    let pad = ui_px(22.0);
    let has_label = !label.is_empty();
    let inner_h = r.h - 2 * ui_px(8.0);
    let ts = ui_text_size().min(inner_h as f32 * 0.46);
    if icon != IconKind::None && has_label {
        let isz = (inner_h.min(ui_px(70.0))).max(ui_px(30.0));
        ui_icon(c, icon, Rect::new(r.x + pad, r.y + (r.h - isz) / 2, isz, isz), fg);
        let lx = r.x + pad + isz + ui_px(18.0);
        let lw = (r.x + r.w - pad - lx).max(0);
        ui_text(c, FontFace::Bold, ts, ui_small_size(), Rect::new(lx, r.y, lw, r.h), label, 1, false, fg);
    } else if icon != IconKind::None {
        let isz = inner_h.min(ui_px(80.0));
        ui_icon(c, icon, Rect::new(r.x + (r.w - isz) / 2, r.y + (r.h - isz) / 2, isz, isz), fg);
    } else if has_label {
        ui_text(
            c,
            FontFace::Bold,
            ts,
            ui_small_size(),
            Rect::new(r.x + pad, r.y, (r.w - 2 * pad).max(0), r.h),
            label,
            1,
            true,
            fg,
        );
    }
}

/// A round icon button (Classic) or the carved square one (Gothic): the header's search, apps and
/// settings, a sheet's close, a card's `...`.
pub fn gui_icon_button(c: &mut Canvas, r: Rect, icon: IconKind, st: ButtonStyle) {
    if gui_gothic() {
        ui_button(c, r, icon, "", st);
        return;
    }
    let filled = st == ButtonStyle::Selected;
    let fg: u8 = if st == ButtonStyle::Disabled {
        170
    } else if filled {
        255
    } else {
        0
    };
    let edge = if st == ButtonStyle::Disabled { 170 } else { 0 };
    ui_frame(c, r, gui_radius(), if filled { 0 } else { 255 }, true, 0);
    ui_frame(c, r, gui_radius(), edge, false, ui_px(3.0));
    let isz = r.w.min(r.h) - 2 * ui_px(22.0);
    ui_icon(c, icon, Rect::new(r.x + (r.w - isz) / 2, r.y + (r.h - isz) / 2, isz, isz), fg);
}

/// A filter chip: a pill in Classic, a cut-cornered tab in Gothic. The chosen one is filled.
pub fn gui_chip(c: &mut Canvas, r: Rect, label: &str, selected: bool) {
    let rad = if gui_gothic() { ui_px(14.0) } else { r.h / 2 };
    ui_frame(c, r, rad, if selected { 0 } else { 255 }, true, 0);
    ui_frame(c, r, rad, 0, false, if selected { ui_px(3.0) } else { ui_px(3.0) });
    ui_text(
        c,
        FontFace::Bold,
        ui_text_size().min((r.h - ui_px(16.0)) as f32 * 0.46),
        ui_small_size(),
        rect_inflate(r, -ui_px(18.0)),
        label,
        1,
        true,
        if selected { 255 } else { 0 },
    );
}

/// A text field with a leading icon: the search box, a name field. Returns the clear button's rect
/// when there is text (RECT_EMPTY when there is not), for the caller's hit.
pub fn gui_text_field(c: &mut Canvas, r: Rect, icon: IconKind, text: &str, hint: &str) -> Rect {
    let rad = if gui_gothic() { ui_px(14.0) } else { r.h / 2 };
    ui_frame(c, r, rad, 255, true, 0);
    ui_frame(c, r, rad, if gui_gothic() { 0 } else { 140 }, false, ui_px(3.0));
    let pad = ui_px(24.0);
    let isz = (r.h - ui_px(34.0)).max(ui_px(24.0));
    ui_icon(c, icon, Rect::new(r.x + pad, r.y + (r.h - isz) / 2, isz, isz), 0);
    let has_text = !text.is_empty();
    let clear = if has_text {
        let cs = (r.h - ui_px(28.0)).min(ui_px(58.0));
        Rect::new(r.x + r.w - pad - cs, r.y + (r.h - cs) / 2, cs, cs)
    } else {
        RECT_EMPTY
    };
    let tx = r.x + pad + isz + ui_px(16.0);
    let tw = if has_text {
        (clear.x - ui_px(14.0) - tx).max(0)
    } else {
        (r.x + r.w - pad - tx).max(0)
    };
    let tr = Rect::new(tx, r.y, tw, r.h);
    if has_text {
        ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), tr, text, 1, false, 0);
    } else {
        ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), tr, hint, 1, false, 100);
    }
    if has_text {
        ui_icon(c, IconKind::Close, rect_inflate(clear, -ui_px(10.0)), 0);
    }
    clear
}

/// A select: a caption over the current value, with the theme's chevron. The tap opens the caller's
/// choice sheet, which is a `gui_slide_up` of `gui_sheet_row`s.
pub fn gui_select(c: &mut Canvas, r: Rect, caption: &str, value: &str) {
    let rad = if gui_gothic() { ui_px(14.0) } else { r.h / 2 };
    ui_frame(c, r, rad, 255, true, 0);
    ui_frame(c, r, rad, 140, false, ui_px(3.0));
    let pad = ui_px(24.0);
    let cs = ui_px(44.0);
    let chev = Rect::new(r.x + r.w - pad - cs, r.y + (r.h - cs) / 2, cs, cs);
    ui_icon(c, IconKind::Down, chev, 0);
    let tx = r.x + pad;
    let tw = (chev.x - ui_px(12.0) - tx).max(0);
    // One line, as the reference home writes it ("Sort: Name (A-Z)"): a caption and a value in two
    // lines needed one more row of room than the chips beside them have.
    let mut line = String::new();
    if caption.is_empty() {
        line.push_str(value);
    } else {
        csnprintf!(&mut line, 160, "%s: %s", caption, value);
    }
    ui_text(
        c,
        FontFace::Bold,
        ui_text_size().min((r.h - ui_px(20.0)) as f32 * 0.52),
        ui_small_size(),
        Rect::new(tx, r.y, tw, r.h),
        &line,
        1,
        false,
        0,
    );
}

/// The floating action button: a black disc in Classic, a carved black square in Gothic, with the
/// icon in white.
pub fn gui_fab(c: &mut Canvas, r: Rect, icon: IconKind) {
    if gui_gothic() {
        ui_frame(c, r, ui_px(18.0), 0, true, 0);
        ui_frame(c, rect_inflate(r, -ui_px(9.0)), ui_px(12.0), 255, false, ui_px(2.0));
    } else {
        let rad = r.w.min(r.h) as f32 / 2.0;
        canvas_disc_aa(c, r.x as f32 + r.w as f32 / 2.0, r.y as f32 + r.h as f32 / 2.0, rad, 0, 1.0);
    }
    let isz = ((r.w.min(r.h) as f32 * 0.44) as i32).max(ui_px(30.0));
    ui_icon(c, icon, Rect::new(r.x + (r.w - isz) / 2, r.y + (r.h - isz) / 2, isz, isz), 255);
}

/// A small app launcher: the icon in a rounded rectangle, its name below it, as a phone's home
/// screen lays apps out. `r` is the whole item; the name sits outside the icon's surface.
pub fn gui_app_tile(c: &mut Canvas, r: Rect, icon: IconKind, label: &str, selected: bool) {
    let label_h = ui_px(52.0).min(r.h / 3).max(ui_px(30.0));
    let icon_r = Rect::new(r.x, r.y, r.w, (r.h - label_h - ui_px(6.0)).max(ui_px(40.0)));
    gui_surface(c, icon_r, selected, ui_px(3.0));
    let fg: u8 = if selected { 255 } else { 0 };
    let isz = (icon_r.w.min(icon_r.h) - 2 * ui_px(24.0)).max(ui_px(28.0));
    ui_icon(
        c,
        icon,
        Rect::new(icon_r.x + (icon_r.w - isz) / 2, icon_r.y + (icon_r.h - isz) / 2, isz, isz),
        fg,
    );
    ui_text(
        c,
        FontFace::Bold,
        ui_small_size().min(label_h as f32 * 0.7),
        ui_small_size() * 0.75,
        Rect::new(r.x, icon_r.y + icon_r.h + ui_px(6.0), r.w, label_h),
        label,
        1,
        true,
        0,
    );
}

/// One row of `gui_popup`: an icon, a label, whether it is the current choice, and whether it is
/// unavailable. The popup draws the row itself, so every small menu in the app has one row design.
pub struct GuiPopupItem<'a> {
    pub icon: IconKind,
    pub label: &'a str,
    pub selected: bool,
    pub disabled: bool,
}

impl<'a> GuiPopupItem<'a> {
    pub fn new(icon: IconKind, label: &'a str) -> GuiPopupItem<'a> {
        GuiPopupItem { icon, label, selected: false, disabled: false }
    }
}

/// Where a popup w by h goes for anchor: above it when there is room, below it otherwise, aligned to
/// the anchor's trailing edge and kept inside the screen. Pure, so the placement is testable.
pub fn popup_place(screen: Rect, anchor: Rect, w: i32, h: i32) -> Rect {
    let m = ui_px(24.0);
    let gap = ui_px(18.0);
    let x = clampi(anchor.x + anchor.w - w, screen.x + m, screen.x + screen.w - w - m);
    let above = anchor.y - h - gap;
    let below = anchor.y + anchor.h + gap;
    let y = if above >= screen.y + m {
        above
    } else if below + h <= screen.y + screen.h - m {
        below
    } else {
        (screen.y + (screen.h - h) / 2).max(screen.y + m)
    };
    Rect::new(x, y.min(screen.y + screen.h - h - m), w, h)
}

/// The global popup menu (2.3, issue #209), for the + button, the sort select, a card's `...` and
/// other small interactions: a compact floating card at its anchor, with no rule between the title
/// and the rows. Returns the row rects for the caller's hits.
pub fn gui_popup(
    c: &mut Canvas,
    screen: Rect,
    anchor: Rect,
    title: &str,
    items: &[GuiPopupItem<'_>],
) -> Vec<Rect> {
    let pad = ui_px(26.0);
    let row_h = ui_px(108.0);
    let row_gap = ui_px(8.0);
    let title_h = if title.is_empty() { ui_px(24.0) } else { ui_px(88.0) };
    let icon_w = ui_px(52.0) + ui_px(22.0);
    let label_w = items
        .iter()
        .map(|it| font_measure(FontFace::Bold, ui_text_size(), it.label))
        .max()
        .unwrap_or(0);
    let w = (label_w + icon_w + 2 * pad)
        .max(ui_px(560.0))
        .min(screen.w - 2 * ui_px(24.0));
    let n = items.len() as i32;
    let h = 2 * pad + title_h + n * row_h + (n - 1).max(0) * row_gap;
    let card = popup_place(screen, anchor, w, h);
    gui_surface(c, card, false, ui_px(4.0));
    if !title.is_empty() {
        font_draw_left(
            c,
            FontFace::Bold,
            ui_text_size(),
            Rect::new(card.x + pad, card.y + ui_px(12.0), (card.w - 2 * pad).max(0), title_h),
            title,
            0,
        );
    }
    let mut rows = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let r = Rect::new(
            card.x + pad,
            card.y + pad + title_h + i as i32 * (row_h + row_gap),
            (card.w - 2 * pad).max(0),
            row_h,
        );
        rows.push(r);
        gui_popup_row(c, r, item.icon, item.label, item.selected, item.disabled);
    }
    rows
}

fn gui_popup_row(c: &mut Canvas, r: Rect, icon: IconKind, label: &str, selected: bool, disabled: bool) {
    let rad = if gui_gothic() { ui_px(12.0) } else { ui_px(18.0) };
    if selected {
        ui_frame(c, r, rad, 0, true, 0);
    }
    let fg: u8 = if disabled {
        170
    } else if selected {
        255
    } else {
        0
    };
    let pad = ui_px(22.0);
    let isz = (r.h - ui_px(28.0)).min(ui_px(56.0));
    let mut tx = r.x + pad;
    if icon != IconKind::None {
        ui_icon(c, icon, Rect::new(tx, r.y + (r.h - isz) / 2, isz, isz), fg);
        tx += isz + ui_px(22.0);
    }
    ui_text(
        c,
        FontFace::Bold,
        ui_text_size(),
        ui_small_size(),
        Rect::new(tx, r.y, (r.x + r.w - pad - tx).max(0), r.h),
        label,
        1,
        false,
        fg,
    );
}

/// A slide-up sheet at the foot of the screen, its grabber and title drawn: returns its content
/// rect (below the title, inside its margins). The caller fills it with rows and adds their hits.
pub fn gui_slide_up(c: &mut Canvas, aw: i32, ah: i32, h: i32, title: &str) -> Rect {
    let m = gui_margin().min(ui_px(30.0));
    let w = (aw - 2 * m).min(ui_px(1780.0));
    let h = h.min(ah - 2 * m);
    let card = Rect::new((aw - w) / 2, ah - h - m, w, h);
    ui_frame(c, card, ui_px(28.0), 255, true, 0);
    ui_frame(c, card, ui_px(28.0), 0, false, ui_px(4.0));
    if gui_gothic() {
        ui_frame(c, rect_inflate(card, -ui_px(12.0)), ui_px(22.0), 0, false, ui_px(2.0));
        canvas_lozenge(c, card.x as f32 + card.w as f32 / 2.0, card.y as f32 + ui_px(20.0) as f32, ui_px(8.0) as f32, 0);
    } else {
        let gw = ui_px(100.0);
        canvas_round_rect(c, Rect::new(card.x + (card.w - gw) / 2, card.y + ui_px(14.0), gw, ui_px(8.0)), ui_px(4.0), 170, true, 0);
    }
    let inner = ui_px(34.0);
    let title_h = ui_px(104.0);
    if !title.is_empty() {
        font_draw_left(
            c,
            FontFace::Title,
            ui_title_size() * if gui_gothic() { 1.0 } else { 0.72 },
            Rect::new(card.x + inner, card.y + ui_px(28.0), (card.w - 2 * inner).max(0), title_h),
            title,
            0,
        );
    }
    Rect::new(card.x + inner, card.y + ui_px(28.0) + title_h, card.w - 2 * inner, card.h - ui_px(28.0) - title_h - inner)
}

/// One row of a slide-up sheet: the kit's list row. `selected` fills it; `disabled` grays it and
/// the caller adds no hit.
pub fn gui_sheet_row(c: &mut Canvas, r: Rect, icon: IconKind, label: &str, selected: bool, disabled: bool) {
    let rad = if gui_gothic() { ui_px(14.0) } else { ui_px(20.0) };
    ui_frame(c, r, rad, if selected { 0 } else { 255 }, true, 0);
    ui_frame(c, r, rad, if disabled { 170 } else { 0 }, false, ui_px(3.0));
    let fg: u8 = if disabled {
        170
    } else if selected {
        255
    } else {
        0
    };
    let pad = ui_px(26.0);
    let isz = (r.h - ui_px(30.0)).min(ui_px(60.0));
    let mut tx = r.x + pad;
    if icon != IconKind::None {
        ui_icon(c, icon, Rect::new(tx, r.y + (r.h - isz) / 2, isz, isz), fg);
        tx += isz + ui_px(20.0);
    }
    ui_text(
        c,
        FontFace::Bold,
        ui_text_size(),
        ui_small_size(),
        Rect::new(tx, r.y, (r.x + r.w - pad - tx).max(0), r.h),
        label,
        1,
        false,
        fg,
    );
}

/// A tooltip under (or over) the control it explains, clamped to the screen. Returns its rect so the
/// caller knows what it covers (for a redraw or a hit).
pub fn gui_tooltip(c: &mut Canvas, screen: Rect, anchor: Rect, text: &str) -> Rect {
    let pad = ui_px(22.0);
    let ts = ui_small_size();
    let tw = font_measure(FontFace::Regular, ts, text) + 2 * pad;
    let w = tw.min(screen.w - 2 * ui_px(20.0));
    let h = (ts * 1.6) as i32 + ui_px(16.0);
    let r = tooltip_place(screen, anchor, w, h);
    ui_frame(c, r, if gui_gothic() { ui_px(12.0) } else { ui_px(16.0) }, 255, true, 0);
    ui_frame(c, r, if gui_gothic() { ui_px(12.0) } else { ui_px(16.0) }, 0, false, ui_px(3.0));
    ui_text(c, FontFace::Regular, ts, ts * 0.8, rect_inflate(r, -pad), text, 1, true, 0);
    // The little stem between the bubble and its control.
    let cx = anchor.x + anchor.w / 2;
    let cy = if r.y + r.h <= anchor.y { r.y + r.h } else { r.y };
    let to = if r.y + r.h <= anchor.y { anchor.y } else { anchor.y + anchor.h };
    canvas_line_aa(c, cx as f32, cy as f32, cx as f32, to as f32, ui_px(3.0) as f32, 0);
    canvas_lozenge(c, cx as f32, to as f32, ui_px(7.0) as f32, 0);
    r
}

/// Where gui_tooltip puts a w by h bubble for anchor: over the control when there is room, under it
/// otherwise, and inside screen's sides. Pure, so the layout is testable without a canvas.
pub fn tooltip_place(screen: Rect, anchor: Rect, w: i32, h: i32) -> Rect {
    let gap = ui_px(22.0);
    let mut x = anchor.x + (anchor.w - w) / 2;
    x = clampi(x, screen.x + ui_px(10.0), screen.x + screen.w - w - ui_px(10.0));
    let above = anchor.y - h - gap;
    let y = if above >= screen.y + ui_px(10.0) {
        above
    } else {
        anchor.y + anchor.h + gap
    };
    Rect::new(x, y.min(screen.y + screen.h - h - ui_px(10.0)), w, h)
}

/// The one slider for every sheet: ui.rs's slider is already themed, drawn here so a menu needs one
/// include and one call.
pub fn gui_slider(c: &mut Canvas, s: &Slider, value_text: Option<&str>) {
    ui_slider_draw(c, s, value_text);
}

/// The one on-screen keyboard for every text field: keyboard.rs's own, themed by ui.rs's frames, so a
/// screen that asks for a name or a search does not draw a Classic and a Gothic keyboard of its own.
pub fn gui_keyboard(c: &mut Canvas, kb: &mut crate::keyboard::Keyboard, screen: Rect) -> Rect {
    crate::keyboard::keyboard_draw(kb, c, screen)
}

/// A card in the kit's design (Settings' rows use ui_card, which is the same surface).
pub fn gui_card(c: &mut Canvas, r: Rect, st: ButtonStyle) {
    ui_card(c, r, st);
}

// A cached battery reading: the status bar is drawn on every Library redraw, and reading sysfs twice
// each time is not free on the Kindle. A change shows within half a minute (and on any redraw after
// that); a failed read is retried then too.
thread_local! {
    static BATTERY: Cell<(u32, i32, bool)> = const { Cell::new((0, -1, false)) };
}

fn battery_cached() -> Option<Battery> {
    const REFRESH_MS: u32 = 30_000;
    BATTERY.with(|cell| {
        let (at, percent, charging) = cell.get();
        let now = now_ms();
        if at != 0 && now.wrapping_sub(at) < REFRESH_MS {
            return (percent >= 0).then_some(Battery { percent, charging });
        }
        let b = battery_from_sysfs("/sys/class/power_supply");
        cell.set((now, b.map_or(-1, |b| b.percent), b.is_some_and(|b| b.charging)));
        b
    })
}

/// The status bar's battery: the pixel-art kind of shape (a body with its terminal and a fill by
/// percent), drawn here rather than taken from the icon font, which need not carry a battery.
pub fn gui_battery(c: &mut Canvas, r: Rect, percent: i32) {
    // The art keeps its proportions at every status bar size (34 px tall is the medium bar's).
    let k = r.h.max(1) / 34;
    let rad = (if gui_gothic() { 5 * k } else { 10 * k }).max(1);
    let tw = (8 * k).max(2);
    let gap = (5 * k).max(1);
    let body = Rect::new(r.x, r.y, r.w - tw - gap, r.h);
    ui_frame(c, body, rad, 255, true, 0);
    ui_frame(c, body, rad, 0, false, (3 * k).max(1));
    let term = Rect::new(body.x + body.w + (2 * k).max(1), r.y + r.h * 30 / 100, tw, r.h * 40 / 100);
    ui_frame(c, term, tw / 2, 0, true, 0);
    let inner = rect_inflate(body, -(7 * k).max(2));
    let fill = (inner.w as i64 * clampi(percent, 0, 100) as i64 / 100) as i32;
    if fill > 0 {
        let fr = Rect::new(inner.x, inner.y, fill.min(inner.w), inner.h);
        ui_frame(c, fr, rad / 2, 0, true, 0);
    }
}

/// The app's status bar: the time and date at the left, then (when the device has them) Wi-Fi,
/// Bluetooth and the battery percent at the right. Every screen draws it above its header; a tap in
/// it is a tap on no control (the caller adds no hit).
pub fn gui_status_bar(c: &mut Canvas, aw: i32, now: i64) {
    gui_status_bar_in(c, aw, now, true);
}

/// The same bar with only the connection icons at the right: the notebook page uses it when its own
/// clock corner is on, so the time and battery appear once, in that corner.
pub fn gui_status_bar_connections(c: &mut Canvas, aw: i32, now: i64) {
    gui_status_bar_in(c, aw, now, false);
}

fn gui_status_bar_in(c: &mut Canvas, aw: i32, now: i64, clock: bool) {
    let scale = status_scale();
    let u = |v: f32| ui_px(v * scale);
    let h = gui_status_h();
    let m = gui_margin();
    canvas_fill(c, Rect::new(0, 0, aw, h), 255);
    if clock {
        let mut time = String::new();
        format_clock_short(now, status_style().1, &mut time, 64);
        let time_px = u(38.0);
        let time_w = font_measure(FontFace::Bold, time_px as f32, &time) + u(2.0);
        font_draw_left(c, FontFace::Bold, time_px as f32, Rect::new(m, 0, time_w, h), &time, 0);
        let mut date = String::new();
        format_date_short(now, &mut date, 64);
        let date_x = m + time_w + u(18.0);
        font_draw_left(
            c,
            FontFace::Regular,
            u(32.0) as f32,
            Rect::new(date_x, 0, (aw / 2 - date_x).max(0), h),
            &date,
            90,
        );
    }
    let isz = u(48.0);
    let mut x = aw - m;
    if clock {
        if let Some(b) = battery_cached() {
            let bw = u(56.0);
            let bh = u(34.0);
            x -= bw + u(8.0);
            gui_battery(c, Rect::new(x, (h - bh) / 2, bw, bh), b.percent);
            x -= u(10.0);
            let mut pct = String::new();
            csnprintf!(&mut pct, 16, "%d%%", clampi(b.percent, 0, 100));
            let pct_px = u(30.0);
            let pw = font_measure(FontFace::Regular, pct_px as f32, &pct);
            x -= pw;
            font_draw_left(c, FontFace::Regular, pct_px as f32, Rect::new(x, 0, pw + 2, h), &pct, 0);
            x -= u(28.0);
        }
    }
    if bt_present() {
        x -= isz;
        let fg = if bt_state() == BtState::Stopped { 170 } else { 0 };
        ui_icon(c, IconKind::Bluetooth, Rect::new(x, (h - isz) / 2, isz, isz), fg);
        x -= u(22.0);
    }
    if wifi_available() {
        x -= isz;
        let fg = if wifi_status().is_some_and(|s| s.enabled) { 0 } else { 170 };
        ui_icon(c, IconKind::Wifi, Rect::new(x, (h - isz) / 2, isz, isz), fg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tooltip_stays_on_screen_and_prefers_above() {
        let screen = Rect::new(0, 0, 1000, 2000);
        let anchor = Rect::new(400, 500, 200, 100);
        let r = tooltip_place(screen, anchor, 300, 80);
        assert_eq!(r.x, 350);
        assert_eq!(r.y, 500 - 80 - ui_px(22.0));
        // Near the top it goes below the control, and near the right it stays inside.
        let top = tooltip_place(screen, Rect::new(900, 20, 100, 100), 300, 80);
        assert_eq!(top.x, 700 - ui_px(10.0));
        assert_eq!(top.y, 20 + 100 + ui_px(22.0));
    }

    #[test]
    fn a_popup_stays_on_screen_and_prefers_above_its_anchor() {
        let screen = Rect::new(0, 0, 1000, 2000);
        let anchor = Rect::new(700, 1600, 200, 100);
        let r = popup_place(screen, anchor, 500, 400);
        assert_eq!(r.x, 400, "aligned to the anchor's trailing edge");
        assert_eq!(r.y, 1600 - 400 - ui_px(18.0));
        // Near the top it goes below the anchor; near the right it stays inside.
        let low = popup_place(screen, Rect::new(900, 40, 80, 80), 500, 300);
        assert_eq!(low.x, 1000 - 500 - ui_px(24.0));
        assert_eq!(low.y, 40 + 80 + ui_px(18.0));
    }

    #[test]
    fn the_status_bar_scales_with_clock_size_and_headers_share_the_title_axis() {
        ui_init(300.0);
        gui_status_style(1, false);
        let medium = gui_status_h();
        assert_eq!(medium, ui_px(78.0));
        gui_status_style(0, false);
        let small = gui_status_h();
        gui_status_style(2, true);
        let large = gui_status_h();
        assert!(small < medium && medium < large, "{small} {medium} {large}");
        // The whole bar's contents scale too: the clock, date, icons and battery all use the same
        // factor, so a large bar is taller rather than just more padded.
        assert!(large - medium > ui_px(10.0));
        gui_status_style(1, false);
        assert_eq!(gui_status_h(), medium);

        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, 1860, 500), 0);
        let (head, actions) = gui_header(&mut c, 1860, "Library", true, &[(IconKind::Gear, 1)]);
        assert_eq!(head.back.y + head.back.h / 2, head.title.y + head.title.h / 2);
        assert_eq!(actions[0].0.y + actions[0].0.h / 2, head.title.y + head.title.h / 2);
        canvas_free(&mut c);
        gui_status_style(1, false);
    }

    #[test]
    fn the_kit_keeps_the_theme_s_radii() {
        ui_init(300.0);
        ui_set_theme(UiTheme::Classic);
        assert_eq!(gui_radius(), 24);
        assert!(!gui_gothic());
        ui_set_theme(UiTheme::Gothic);
        assert_eq!(gui_radius(), 16);
        assert!(gui_gothic());
        ui_set_theme(UiTheme::Gothic);
    }
}
