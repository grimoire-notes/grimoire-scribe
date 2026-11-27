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
use crate::appicon::AppIcon;
use crate::status::{battery_from_sysfs, Battery, STATUS_SIZES};
use std::cell::{Cell, RefCell};
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

// ---- the standard sizes: what Grimoire's own screens lay their elements out at, and what the package kit
// (kithost.rs, `metric`) gives a package and draws an element at when it is given no size ----

/// A row of a list: Settings' rows, and the rows of the package manager's lists.
pub fn gui_row_h() -> i32 {
    ui_px(118.0)
}
/// The shortest Settings lets its rows be when a list is tight, and the smallest control Grimoire draws.
pub fn gui_row_min_h() -> i32 {
    ui_px(90.0)
}
/// An action button: the Package Store's actions (`gui_button_row`, which is `gui_button` in Classic and in Gothic the
/// carved frame with its label beside its icon).
pub fn gui_button_h() -> i32 {
    ui_px(96.0)
}
/// A text field: no screen of Grimoire draws one yet (the app host's `text_field` and the package kit's `field`
/// draw `gui_text_field`); 112, the height of the package manager's action rows, as the kit's design gives it.
pub fn gui_field_h() -> i32 {
    ui_px(112.0)
}
/// A filter chip: the home's and the Package Store's.
pub fn gui_chip_h() -> i32 {
    ui_px(88.0)
}
/// The width a chip takes for its label: the label in the chip's face and 78 px, at least 170 (the Package
/// Store's chips).
pub fn gui_chip_w(label: &str) -> i32 {
    (font_measure(FontFace::Bold, ui_text_size(), label) + ui_px(78.0)).max(ui_px(170.0))
}
/// A select: the home's Sort.
pub fn gui_select_h() -> i32 {
    ui_px(88.0)
}
/// The grays of the kit's elements: a field's hint, a disabled control, and a field's or a select's outline in
/// Classic. (Ink is 0 and paper 255.)
pub const GUI_GRAY_MUTED: u8 = 100;
pub const GUI_GRAY_DISABLED: u8 = 170;
pub const GUI_GRAY_RULE: u8 = 140;

/// The top status bar's scale at size small, medium and large. Medium is the bar as it has always
/// been; the Status bar settings page's size scales all of it, not just its clock. At 300 dpi the
/// bar is 72, 78 and 92 px tall (small was 0.82, 64 px, before #319).
const STATUS_SCALE: [f32; 3] = [0.92, 1.0, 1.18];

struct StatusUnder {
    w: i32,
    h: i32,
    px: Vec<u8>,
}

thread_local! {
    /// The Status bar settings the top status bar draws with (size, 12-hour time).
    static STATUS_STYLE: Cell<(i32, bool)> = const { Cell::new((1, false)) };
    /// Whether tapping the bar hid it (settings.ini status_on, unchanged by a restart).
    static STATUS_SHOWN: Cell<bool> = const { Cell::new(true) };
    /// The screen as it was under the bar when it was last drawn (Live view's tiles show this while
    /// the bar itself is its own layer: LIVE_LAYER_STATUS).
    static STATUS_UNDER: RefCell<StatusUnder> = const { RefCell::new(StatusUnder { w: 0, h: 0, px: Vec::new() }) };
}

/// Applies the Status bar settings to every top status bar (Settings > Sleep & Clock > Status bar).
/// The main page and the Status bar page call this when either choice changes.
pub fn gui_status_style(size: i32, twelve: bool) {
    STATUS_STYLE.with(|s| s.set((clampi(size, 0, STATUS_SIZES - 1), twelve)));
}

/// Shows or hides the notebook's top status bar. A tap on a notebook's bar sets it (status_toggle): a
/// shown bar hides, a hidden one comes back; the Status bar page's "Hide status bar inside Notebooks"
/// row changes it too, and settings.ini status_on keeps it across restarts. The other screens' bars are
/// always drawn: only a notebook can hide it.
pub fn gui_status_shown(on: bool) {
    STATUS_SHOWN.with(|s| s.set(on));
}

pub fn gui_status_visible() -> bool {
    STATUS_SHOWN.with(|s| s.get())
}

fn status_style() -> (i32, bool) {
    STATUS_STYLE.with(|s| s.get())
}

fn status_scale() -> f32 {
    STATUS_SCALE[status_style().0 as usize]
}

/// The status bar's height (`gui_status_bar`): the whole bar grows and shrinks with the Status bar
/// page's size.
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
    /// The room `gui_header_lead` keeps left of the title for the caller's mark (the home's bat),
    /// centred on the title's axis; empty for every other header.
    pub lead: Rect,
    /// The title, between Back (or the lead) and the actions.
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
    gui_header_lead(c, aw, title, back, (0, 0), actions)
}

/// `gui_header` with room for a mark of the caller's, `lead` (w, h), between Back and the title: the
/// home's top level puts Grimoire's bat there (library.rs draw_base). The room is `GuiHeader::lead`,
/// the title starts 34 px after it, and the caller draws the mark. With no lead (0, 0) the header is
/// `gui_header`'s, pixel for pixel.
pub fn gui_header_lead(
    c: &mut Canvas,
    aw: i32,
    title: &str,
    back: bool,
    lead: (i32, i32),
    actions: &[(IconKind, i32)],
) -> (GuiHeader, Vec<(Rect, i32)>) {
    gui_status_bar(c, aw, sys_wall_clock());
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
    let mut lead_r = RECT_EMPTY;
    if lead.0 > 0 && lead.1 > 0 {
        lead_r = Rect::new(tx, top + (th - lead.1) / 2, lead.0, lead.1);
        tx = lead_r.x + lead_r.w + ui_px(34.0);
    }
    let title_r = Rect::new(tx, top, (rx - tx - ui_px(20.0)).max(0), th);
    gui_page_title(c, title_r, title);
    (GuiHeader { back: back_r, lead: lead_r, title: title_r, foot: top + th }, rects)
}

/// The size of a screen's title (`gui_page_title`): the title size, larger in Gothic's blackletter.
pub fn gui_page_title_px() -> f32 {
    ui_title_size() * if gui_gothic() { 1.75 } else { 1.1 }
}

/// A screen's title in the modern header's type: the same size and position on the home, Apps,
/// Files and every Settings page.
pub fn gui_page_title(c: &mut Canvas, r: Rect, title: &str) {
    let px = gui_page_title_px();
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
        GUI_GRAY_DISABLED
    } else if filled {
        255
    } else {
        0
    };
    let edge = if st == ButtonStyle::Disabled { GUI_GRAY_DISABLED } else { 0 };
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

/// The kit's button with its icon at the left and its label beside it on the right, in both themes.
/// ui_button puts a Gothic label under the icon (carved buttons); a row of actions whose names must
/// be read beside their icons (the Store's Install, Uninstall, Repository URL) needs this instead.
pub fn gui_button_row(c: &mut Canvas, r: Rect, icon: IconKind, label: &str, st: ButtonStyle) {
    if r.w <= 0 || r.h <= 0 {
        return;
    }
    if !gui_gothic() {
        gui_button(c, r, icon, label, st);
        return;
    }
    // The carved frame alone, then the glyph and the label on one line.
    ui_button(c, r, IconKind::None, "", st);
    let fg: u8 = match st {
        ButtonStyle::Disabled => 170,
        ButtonStyle::Selected => 255,
        _ => 0,
    };
    let (pad, isz, gap, ts) = button_row_parts(r.h);
    ui_icon(c, icon, Rect::new(r.x + pad, r.y + (r.h - isz) / 2, isz, isz), fg);
    let lx = r.x + pad + isz + gap;
    let lw = (r.x + r.w - pad - lx).max(0);
    ui_text(c, FontFace::Caps, ts, ui_small_size().min(ts), Rect::new(lx, r.y, lw, r.h), label, 1, false, fg);
}

/// Gothic's `gui_button_row` `h` high: the room at its ends, its icon's side, the room between the icon and the
/// label, and the label's size.
fn button_row_parts(h: i32) -> (i32, i32, i32, f32) {
    let inner_h = h - 2 * ui_px(8.0);
    (ui_px(24.0), inner_h.min(ui_px(70.0)).max(ui_px(30.0)), ui_px(18.0), ui_small_size().min(inner_h as f32 * 0.34))
}

/// The width Gothic's `gui_button_row` `h` high needs for its icon and its whole label on one line (in Classic it is
/// `gui_button`).
pub fn gui_button_row_need_w(h: i32, label: &str) -> i32 {
    let (pad, isz, gap, ts) = button_row_parts(h);
    pad + isz + gap + font_measure(FontFace::Caps, ts, label) + pad
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
    ui_frame(c, r, rad, if gui_gothic() { 0 } else { GUI_GRAY_RULE }, false, ui_px(3.0));
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
        ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), tr, hint, 1, false, GUI_GRAY_MUTED);
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
    ui_frame(c, r, rad, GUI_GRAY_RULE, false, ui_px(3.0));
    let pad = ui_px(24.0);
    let cs = ui_px(44.0);
    let chev = Rect::new(r.x + r.w - pad - cs, r.y + (r.h - cs) / 2, cs, cs);
    ui_icon(c, IconKind::Down, chev, 0);
    let tx = r.x + pad;
    let tw = (chev.x - ui_px(12.0) - tx).max(0);
    // One line, as the reference home writes it ("Sort: Name (A-Z)"): a caption and a value in two
    // lines needed one more row of room than the chips beside them have.
    // A caption and value that do not fit even at the small size (a longer language) leave the
    // caption out: the value alone is what the select says, and nothing is cut short.
    let px = ui_text_size().min((r.h - ui_px(20.0)) as f32 * 0.52);
    let mut line = String::new();
    if caption.is_empty() {
        line.push_str(value);
    } else {
        csnprintf!(&mut line, 160, "%s: %s", caption, value);
        if font_fit_px(FontFace::Bold, px, ui_small_size().min(px), &line, tw) <= 0.0 {
            line.clear();
            line.push_str(value);
        }
    }
    ui_text(c, FontFace::Bold, px, ui_small_size(), Rect::new(tx, r.y, tw, r.h), &line, 1, false, 0);
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
pub fn gui_app_tile(c: &mut Canvas, r: Rect, icon: &AppIcon, label: &str, selected: bool) {
    let label_h = ui_px(52.0).min(r.h / 3).max(ui_px(30.0));
    let icon_r = Rect::new(r.x, r.y, r.w, (r.h - label_h - ui_px(6.0)).max(ui_px(40.0)));
    gui_surface(c, icon_r, selected, ui_px(3.0));
    let fg: u8 = if selected { 255 } else { 0 };
    let isz = (icon_r.w.min(icon_r.h) - 2 * ui_px(24.0)).max(ui_px(28.0));
    icon.draw(
        c,
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
    let pad = ui_px(16.0);
    let row_h = ui_px(84.0);
    let row_gap = ui_px(4.0);
    let title_h = if title.is_empty() { ui_px(8.0) } else { ui_px(62.0) };
    let icon_w = ui_px(46.0) + ui_px(14.0);
    let label_w = items
        .iter()
        .map(|it| font_measure(FontFace::Bold, ui_text_size(), it.label))
        .max()
        .unwrap_or(0);
    let w = (label_w + icon_w + 2 * pad)
        .max(ui_px(440.0))
        .min(screen.w - 2 * ui_px(18.0));
    let n = items.len() as i32;
    let h = 2 * pad + title_h + n * row_h + (n - 1).max(0) * row_gap;
    let card = popup_place(screen, anchor, w, h);
    gui_surface(c, card, false, ui_px(4.0));
    if !title.is_empty() {
        font_draw_left(
            c,
            FontFace::Bold,
            ui_text_size(),
            Rect::new(card.x + pad, card.y + ui_px(8.0), (card.w - 2 * pad).max(0), title_h),
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
    let pad = ui_px(16.0);
    let isz = (r.h - ui_px(20.0)).min(ui_px(46.0));
    let mut tx = r.x + pad;
    if icon != IconKind::None {
        ui_icon(c, icon, Rect::new(tx, r.y + (r.h - isz) / 2, isz, isz), fg);
        tx += isz + ui_px(14.0);
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
/// it is a tap on no control (the caller adds no hit). `now` is the platform's clock (light.rs
/// sys_wall_clock), as the minute checks that draw it again read it (library_tick,
/// library_app_minute): the simulator's script holds that clock still (docs/SIM.md, `clock`), and a
/// bar that read the machine's own would refresh in the middle of a scenario that counts refreshes.
pub fn gui_status_bar(c: &mut Canvas, aw: i32, now: i64) {
    gui_status_bar_in(c, aw, now, true, true);
}

/// The notebook's key status bar: the same time, connections and battery with no white background, so
/// what is drawn under it (the page) still shows through.
pub fn gui_status_bar_transparent(c: &mut Canvas, aw: i32, now: i64) {
    if gui_status_visible() {
        gui_status_bar_in(c, aw, now, true, false);
    }
}

/// The same bar with only the connection icons at the right: kept for the retired corner's callers.
pub fn gui_status_bar_connections(c: &mut Canvas, aw: i32, now: i64) {
    gui_status_bar_in(c, aw, now, false, true);
}

/// The screen under the bar, saved before the bar is drawn: Live view's page tiles take these pixels
/// where the bar covers them, so hiding the bar's layer (LIVE_LAYER_STATUS) shows the page, not the bar.
fn status_under_save(c: &Canvas, w: i32, h: i32) {
    if w <= 0 || h <= 0 || c.stride < w {
        return;
    }
    let stride = c.stride as usize;
    if c.px.len() < (h as usize).saturating_mul(stride) {
        return;
    }
    STATUS_UNDER.with(|u| {
        let mut u = u.borrow_mut();
        let n = (w as usize) * (h as usize);
        if u.px.len() != n {
            u.px.resize(n, 255);
        }
        for y in 0..h as usize {
            let from = y * stride;
            u.px[y * w as usize..(y + 1) * w as usize].copy_from_slice(&c.px[from..from + w as usize]);
        }
        u.w = w;
        u.h = h;
    });
}

/// Puts the saved screen under the bar into c at r (the part of the bar r covers). False when there
/// is no matching saved bar: the caller leaves the screen as it is.
pub fn gui_status_under_put(c: &mut Canvas, r: Rect) -> bool {
    if r.w <= 0 || r.h <= 0 || r.x < 0 || r.y < 0 || r.x + r.w > c.w || r.y + r.h > c.h || c.stride < c.w {
        return false;
    }
    let stride = c.stride as usize;
    if c.px.len() < (c.h as usize).saturating_mul(stride) {
        return false;
    }
    STATUS_UNDER.with(|u| {
        let u = u.borrow();
        if r.x + r.w > u.w || r.y + r.h > u.h || u.px.len() < (u.w as usize).saturating_mul(u.h as usize) {
            return false;
        }
        for y in 0..r.h as usize {
            let src = (r.y as usize + y) * u.w as usize + r.x as usize;
            let dst = (r.y as usize + y) * stride + r.x as usize;
            c.px[dst..dst + r.w as usize].copy_from_slice(&u.px[src..src + r.w as usize]);
        }
        true
    })
}

fn gui_status_bar_in(c: &mut Canvas, aw: i32, now: i64, clock: bool, fill: bool) {
    let scale = status_scale();
    let u = |v: f32| ui_px(v * scale);
    let h = gui_status_h();
    let m = gui_margin();
    status_under_save(c, aw, h);
    if fill {
        canvas_fill(c, Rect::new(0, 0, aw, h), 255);
    }
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
        assert!(rect_is_empty(head.lead));
        canvas_free(&mut c);
        gui_status_style(1, false);
    }

    // The small bar is 0.92 of the medium one (#319): 72 px at 300 dpi, where 0.82 made it 64. Medium and large
    // are as they were, 78 and 92.
    #[test]
    fn the_small_status_bar_is_72_px_at_300_dpi() {
        ui_init(300.0);
        let heights: Vec<i32> = (0..STATUS_SIZES)
            .map(|size| {
                gui_status_style(size, false);
                gui_status_h()
            })
            .collect();
        gui_status_style(1, false);
        assert_eq!(heights, [72, 78, 92]);
    }

    // A header with a lead keeps its room between the margin and the title, centred on the title's axis, and is
    // otherwise the header without one: a header with no lead draws the same pixels as gui_header.
    #[test]
    fn a_headers_lead_sits_before_the_title_and_no_lead_changes_nothing() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        gui_status_style(1, false);
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            let mut plain = Canvas::default();
            let mut none = Canvas::default();
            let mut led = Canvas::default();
            for c in [&mut plain, &mut none, &mut led] {
                assert_eq!(canvas_init(c, 1860, 400), 0);
            }
            let acts = [(IconKind::Gear, 1), (IconKind::Search, 2)];
            let (h0, a0) = gui_header(&mut plain, 1860, "Library", false, &acts);
            let (h1, a1) = gui_header_lead(&mut none, 1860, "Library", false, (0, 0), &acts);
            assert_eq!((h0.title, h0.foot, &a0), (h1.title, h1.foot, &a1), "{theme:?}");
            // (under the status bar, whose clock reads the machine's minute)
            let below = (gui_status_h() * plain.stride) as usize;
            assert!(plain.px[below..] == none.px[below..], "{theme:?}: no lead, the same pixels");
            let (h2, a2) = gui_header_lead(&mut led, 1860, "Library", false, (129, 42), &acts);
            assert_eq!(h2.lead, Rect::new(gui_margin(), gui_status_h() + (gui_head_h() - 42) / 2, 129, 42), "{theme:?}");
            assert_eq!(h2.title.x, h2.lead.x + 129 + ui_px(34.0), "{theme:?}");
            assert_eq!(h2.lead.y + h2.lead.h / 2, h2.title.y + h2.title.h / 2, "{theme:?}");
            assert_eq!(a2, a0, "{theme:?}: the actions stay where they were");
            for c in [&mut plain, &mut none, &mut led] {
                canvas_free(c);
            }
        }
        ui_set_theme(UiTheme::Gothic);
    }

    // The standard sizes the screens lay their elements out at (and the package kit gives a package): Settings' rows
    // and their tightest, the Package Store's action buttons, a field, the home's and the Store's chips, the home's Sort.
    #[test]
    fn the_standard_sizes_are_the_ones_the_screens_use() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        assert_eq!((gui_row_h(), gui_row_min_h(), gui_button_h(), gui_field_h(), gui_chip_h(), gui_select_h()), (118, 90, 96, 112, 88, 88));
        assert_eq!((ui_row_icon(), ui_line(), crate::ui::ui_row_pager_button()), (60, 3, (130, 90)));
        for (theme, radius, title) in [(UiTheme::Gothic, 16, 112.0), (UiTheme::Classic, 22, 70.4)] {
            ui_set_theme(theme);
            assert_eq!(ui_card_radius(), radius);
            assert!((gui_page_title_px() - title).abs() < 0.01, "{theme:?}");
            // (a chip is as wide as its label and 78 px, at least 170)
            assert_eq!(gui_chip_w(""), 170);
            assert_eq!(gui_chip_w("A label long enough"), font_measure(FontFace::Bold, ui_text_size(), "A label long enough") + 78);
        }
        // (ui_row_page lays its rows out by ui_row_page_fit: the same height, gap, rows a page and pages)
        for (h, n) in [(2000, 30), (2000, 5), (300, 9), (0, 3), (1000, 0)] {
            let p = crate::ui::ui_row_page(Rect::new(0, 0, 900, h), n, 0, gui_row_h(), gui_row_min_h());
            let (rh, gap, per, pages) = crate::ui::ui_row_page_fit(h, n, gui_row_h(), gui_row_min_h());
            assert_eq!((p.pages, p.rows.len()), (pages, per.min(n)), "{h} {n}");
            if p.rows.len() > 1 {
                assert_eq!((p.rows[0].h, p.rows[1].y - p.rows[0].y - p.rows[0].h), (rh, gap), "{h} {n}");
            }
        }
        ui_set_theme(UiTheme::Gothic);
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
