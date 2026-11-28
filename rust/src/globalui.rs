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
    ui_button, ui_card, ui_frame, ui_icon, ui_ornament, ui_rule, ui_slider_draw, ui_small_size, ui_text,
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

/// The gray of the status bar's band (#337), behind the whole bar on every screen: one of the panel's sixteen
/// levels (11 x 17), so a grays refresh shows it flat. It is one level darker than the desk a notebook's page
/// lies on (205, which the panel shows as 204), so sideways the band stays apart from the desk beside the page.
pub const GUI_GRAY_STATUS_BAND: u8 = 187;

/// The gray of a status bar icon whose radio is off (Wi-Fi or Bluetooth): a panel level (6 x 17) five levels
/// darker than the band and six lighter than the black of one that is on, so it reads on the band and tells
/// off from on.
pub const GUI_GRAY_STATUS_OFF: u8 = 102;

struct StatusUnder {
    w: i32,
    h: i32,
    px: Vec<u8>,
}

thread_local! {
    /// The Status bar settings the top status bar draws with (size, 12-hour time).
    static STATUS_STYLE: Cell<(i32, bool)> = const { Cell::new((1, false)) };
    /// The screen as it was under the bar when it was last drawn (Live view's tiles show this while
    /// the bar itself is its own layer: LIVE_LAYER_STATUS).
    static STATUS_UNDER: RefCell<StatusUnder> = const { RefCell::new(StatusUnder { w: 0, h: 0, px: Vec::new() }) };
    /// The bar as it was last drawn on its band (#337): a notebook's flush puts it back where anything drew into
    /// its rows since (gui_status_drawn_put).
    static STATUS_DRAWN: RefCell<StatusUnder> = const { RefCell::new(StatusUnder { w: 0, h: 0, px: Vec::new() }) };
}

/// Applies the Status bar settings to every top status bar (Settings > Sleep & Clock > Status bar).
/// The main page and the Status bar page call this when either choice changes.
pub fn gui_status_style(size: i32, twelve: bool) {
    STATUS_STYLE.with(|s| s.set((clampi(size, 0, STATUS_SIZES - 1), twelve)));
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
    let title_r = Rect::new(tx, top, (rx - tx - ui_px(20.0)).max(0), th);
    gui_page_title(c, title_r, title);
    (GuiHeader { back: back_r, title: title_r, foot: top + th }, rects)
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

/// The radius `gui_fab` gives ui_frame: Gothic's carved square (its corners cut by 18 + 18 / 4 px), Classic's
/// rounded rectangle at the theme's own radius (`gui_radius`), as its cards and buttons have (#332: it was a disc).
pub fn gui_fab_radius() -> i32 {
    if gui_gothic() {
        ui_px(18.0)
    } else {
        gui_radius()
    }
}

/// The corners `gui_fab` drew: cut by so many px (Gothic's carved square) or round at so many px (Classic's rounded
/// rectangle). The home keeps it for its library.home.fab key (docs/SIM.md).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GuiFabShape {
    #[default]
    None,
    Cut(i32),
    Round(i32),
}

/// The floating action button: a black rounded rectangle in Classic, a carved black square in Gothic,
/// with the icon in white. The home's + and an app's `fab` draw it. Returns the corners it drew, the shape the
/// button is filled by (ui_frame's two shapes at `gui_fab_radius`).
pub fn gui_fab(c: &mut Canvas, r: Rect, icon: IconKind) -> GuiFabShape {
    let radius = gui_fab_radius();
    let shape = if gui_gothic() { GuiFabShape::Cut(radius + radius / 4) } else { GuiFabShape::Round(radius) };
    match shape {
        GuiFabShape::Cut(k) => canvas_chamfer_rect(c, r, k, 0, true, 0),
        GuiFabShape::Round(k) => canvas_round_rect(c, r, k, 0, true, 0),
        GuiFabShape::None => {}
    }
    if gui_gothic() {
        ui_frame(c, rect_inflate(r, -ui_px(9.0)), ui_px(12.0), 255, false, ui_px(2.0));
    }
    let isz = ((r.w.min(r.h) as f32 * 0.44) as i32).max(ui_px(30.0));
    ui_icon(c, icon, Rect::new(r.x + (r.w - isz) / 2, r.y + (r.h - isz) / 2, isz, isz), 255);
    shape
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

// ---- the home's app squares (#332): the home's row and the Apps page show their launchers as even squares, every
// one the same size with its icon in the middle and its name under it. The screen lays them out (library.rs
// app_row_slots) and adds the hits. ----

/// The side of an app's square.
pub fn gui_app_square_side() -> i32 {
    ui_px(176.0)
}

/// The side of the icon's box in a square: a whole multiple of 64 px with at least 24 px of the square round it,
/// so an app package's bundled 64 px icon is drawn at a whole number of screen pixels for each of its own (twice
/// its size, 128 px, at 300 dpi).
pub fn gui_app_icon_side() -> i32 {
    let room = gui_app_square_side() - 2 * ui_px(24.0);
    if room >= 64 {
        room / 64 * 64
    } else {
        room.max(1)
    }
}

/// The paper between a square and its name.
pub fn gui_app_label_gap() -> i32 {
    ui_px(8.0)
}

/// One line of an app's name: the box a name of one line is centred in, and the distance between the two lines of
/// a name that takes two.
pub fn gui_app_label_line_h() -> i32 {
    ui_px(36.0)
}

/// The room under a square for its name: two lines (`gui_app_label_fit`).
pub fn gui_app_label_h() -> i32 {
    2 * gui_app_label_line_h()
}

/// A launcher's whole room: its square, the paper under it and its name's two lines. The row and the Apps page are
/// laid out by it; a launcher's hit is what `gui_app_square` drew, never taller.
pub fn gui_app_h() -> i32 {
    gui_app_square_side() + gui_app_label_gap() + gui_app_label_h()
}

/// How an app's name is set in the width `w` under its square (#332), so that a name is whole: one line at the
/// small size; when that is too wide, one line at a smaller size down to four fifths of it, where the face has one
/// (Classic's faces do; Gothic's pixel face has no size between the small one and two thirds of it); when that is
/// too wide too, two lines at the small size, broken at a space or between two CJK characters. Returns the size and
/// the lines. A name that two lines do not hold either (a word wider than `w`, or a third line) is broken or cut
/// short by font_draw_wrapped, which tells the fit check (GRIMOIRE_CHECK_FIT, make l10n-check) as it does of any
/// text cut short.
pub fn gui_app_label_fit(label: &str, w: i32) -> (f32, i32) {
    let px = ui_small_size();
    if font_measure(FontFace::Bold, px, label) <= w {
        return (px, 1);
    }
    let fs = font_fit_px(FontFace::Bold, px, px * 0.8, label, w);
    if fs > 0.0 {
        return (fs, 1);
    }
    (px, 2)
}

/// An app's name under its square `sq`, centred in `label_w` (the row's step, so a long name may reach into the
/// paper beside its square, never into the next name; less in the first and last columns, so it stays inside the
/// margins: library.rs launcher_label_w), set as `gui_app_label_fit` says. Returns the box its lines
/// take, as wide as the widest of them (never wider than `label_w`), and how many lines it took.
fn gui_app_label(c: &mut Canvas, sq: Rect, label_w: i32, label: &str) -> (Rect, i32) {
    let lw = label_w.max(sq.w);
    let (px, lines) = gui_app_label_fit(label, lw);
    let lh = gui_app_label_line_h();
    let r = Rect::new(sq.x + (sq.w - lw) / 2, sq.y + sq.h + gui_app_label_gap(), lw, lines * lh);
    let tw = if lines == 1 {
        font_draw_centered(c, FontFace::Bold, px, r, label, 0);
        font_measure(FontFace::Bold, px, label)
    } else {
        font_draw_wrapped(c, FontFace::Bold, px, r, label, 2, lh, true, 0);
        let mut widest = 0;
        font_wrap_lines(FontFace::Bold, px, label, lw, Some(&mut widest));
        widest
    }
    .min(lw);
    // (each line is centred in r as the font centres it, so the widest starts here)
    (Rect::new(r.x + (lw - tw) / 2, r.y, tw, r.h), lines)
}

/// An app launcher as an even square: the theme's surface `sq`, the icon in the middle of it
/// (`gui_app_icon_side`), and the name under it (`gui_app_label`). Returns the launcher as drawn, its square and the
/// box its name's lines take together, which is its hit (so a tap anywhere on the name lands on it, and a press's
/// inverted look covers the whole name and nothing past it), and the lines its name took.
pub fn gui_app_square(c: &mut Canvas, sq: Rect, label_w: i32, icon: &AppIcon, label: &str, selected: bool) -> (Rect, i32) {
    gui_surface(c, sq, selected, ui_px(3.0));
    let fg: u8 = if selected { 255 } else { 0 };
    let isz = gui_app_icon_side().min(sq.w.min(sq.h));
    icon.draw(c, Rect::new(sq.x + (sq.w - isz) / 2, sq.y + (sq.h - isz) / 2, isz, isz), fg);
    let (name, lines) = gui_app_label(c, sq, label_w, label);
    (rect_union(sq, name), lines)
}

/// The home row's Show All as its last square: three rows of three of the theme's bullet where an app's icon is (a
/// lozenge in Gothic, as What's new's lists have it, a dot in Classic), and its name under it as an app's. Returns
/// what `gui_app_square` returns.
pub fn gui_app_show_all(c: &mut Canvas, sq: Rect, label_w: i32, label: &str) -> (Rect, i32) {
    let drawn = gui_app_square(c, sq, label_w, &AppIcon::glyph(IconKind::None), label, false);
    let pitch = ui_px(38.0) as f32;
    let (cx, cy) = (sq.x as f32 + sq.w as f32 / 2.0, sq.y as f32 + sq.h as f32 / 2.0);
    for row in -1..=1 {
        for col in -1..=1 {
            let (x, y) = (cx + col as f32 * pitch, cy + row as f32 * pitch);
            if gui_gothic() {
                canvas_lozenge(c, x, y, ui_px(14.0) as f32, 0);
            } else {
                canvas_disc_aa(c, x, y, ui_px(11.0) as f32, 0, 1.0);
            }
        }
    }
    drawn
}

// ---- a page's rules (#332): the marks that part the home, themed once, as a printed page is ruled. A screen draws
// a mark in the paper between two of its parts, never inside a control, so a press's fast refresh (black and white
// only) never carries one. Every mark is at least 2 px, black or a gray that is one of the panel's sixteen levels. ----

/// The gray of the light rule: one of the panel's sixteen levels (8 x 17), so it shows on the panel as it does in a
/// shot.
pub const GUI_GRAY_HAIR: u8 = 136;

/// The gray of Classic's section break: a panel level (6 x 17), darker than the light rule and lighter than the
/// black title and foot rules, so the page's main split ranks between them.
pub const GUI_GRAY_SPLIT: u8 = 102;

/// The height of `gui_title_rule` and of `gui_foot_rule`: Gothic's thick (4 px), 5 px of paper and thin (2 px);
/// Classic's one 3 px line.
pub fn gui_title_rule_h() -> i32 {
    if gui_gothic() {
        ui_px(4.0) + ui_px(5.0) + ui_px(2.0)
    } else {
        ui_px(3.0)
    }
}

/// The rule under a page's title, its top at `y`: in Gothic thick over thin (ui_rule), as a printer rules a title
/// page; in Classic one 3 px black line.
pub fn gui_title_rule(c: &mut Canvas, x0: i32, x1: i32, y: i32) {
    if gui_gothic() {
        ui_rule(c, x0, x1, y, ui_px(4.0), 0);
    } else {
        canvas_hline(c, x0, x1, y, ui_px(3.0), 0);
    }
}

/// The rule over a page's foot, its top at `y`: the title rule turned over (thin over thick) in Gothic, so the two
/// close the page between them; one 3 px black line in Classic.
pub fn gui_foot_rule(c: &mut Canvas, x0: i32, x1: i32, y: i32) {
    if gui_gothic() {
        canvas_hline(c, x0, x1, y, ui_px(2.0), 0);
        canvas_hline(c, x0, x1, y + ui_px(2.0) + ui_px(5.0), ui_px(4.0), 0);
    } else {
        canvas_hline(c, x0, x1, y, ui_px(3.0), 0);
    }
}

/// How far `gui_section_break` reaches above and below its centre line: Gothic's middle lozenge (16 px), Classic's
/// 2 px line (1 px).
pub fn gui_section_break_half() -> i32 {
    if gui_gothic() {
        ui_px(16.0)
    } else {
        ui_px(1.0)
    }
}

/// The section break centred on `cy`, a page's main division: in Gothic the divider under a Gothic sheet's title
/// (a 2 px black rule broken by a 16 px lozenge between two of 8, ui_ornament); in Classic one plain 2 px line of
/// gray 102 (`GUI_GRAY_SPLIT`), with no dots, which a screen that pages could take for its page dots.
pub fn gui_section_break(c: &mut Canvas, x0: i32, x1: i32, cy: i32) {
    if gui_gothic() {
        ui_ornament(c, x0, x1, cy, 0);
    } else {
        canvas_hline(c, x0, x1, cy - ui_px(1.0), ui_px(2.0), GUI_GRAY_SPLIT);
    }
}

/// The height of `gui_hair_rule`.
pub fn gui_hair_rule_h() -> i32 {
    ui_px(2.0)
}

/// The light rule, its top at `y`: 2 px of gray 136 in both themes, the rule under a table's head row.
pub fn gui_hair_rule(c: &mut Canvas, x0: i32, x1: i32, y: i32) {
    canvas_hline(c, x0, x1, y, gui_hair_rule_h(), GUI_GRAY_HAIR);
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
/// percent), drawn here rather than taken from the icon font, which need not carry a battery. The body
/// is filled with `paper`, the gray of what it stands on (the bar's band).
pub fn gui_battery(c: &mut Canvas, r: Rect, percent: i32, paper: u8) {
    // The art keeps its proportions at every status bar size (34 px tall is the medium bar's).
    let k = r.h.max(1) / 34;
    let rad = (if gui_gothic() { 5 * k } else { 10 * k }).max(1);
    let tw = (8 * k).max(2);
    let gap = (5 * k).max(1);
    let body = Rect::new(r.x, r.y, r.w - tw - gap, r.h);
    ui_frame(c, body, rad, paper, true, 0);
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
/// Bluetooth and the battery percent at the right, on a band of gray the bar's full width and height
/// (#337: `GUI_GRAY_STATUS_BAND`, every item drawn in black, or in `GUI_GRAY_STATUS_OFF` for a radio
/// that is off). Every screen draws it above its header; a tap in it is a tap on no control (the
/// caller adds no hit). `now` is the platform's clock (light.rs sys_wall_clock), as the minute checks
/// that draw it again read it (library_tick, library_app_minute): the simulator's script holds that
/// clock still (docs/SIM.md, `clock`), and a bar that read the machine's own would refresh in the
/// middle of a scenario that counts refreshes.
pub fn gui_status_bar(c: &mut Canvas, aw: i32, now: i64) {
    gui_status_bar_in(c, aw, now, true, true);
}

/// The notebook's status bar (#337): the same band, drawn over the page and anything else there. The
/// page goes on beneath it (ink written there stays in the page), and a tap on the bar hides it for as
/// long as the notebook is open (status.rs), which shows that part of the page.
pub fn gui_status_bar_over_page(c: &mut Canvas, aw: i32, now: i64) {
    gui_status_bar_in(c, aw, now, true, true);
}

/// The same bar with only the connection icons at the right: kept for the retired corner's callers.
pub fn gui_status_bar_connections(c: &mut Canvas, aw: i32, now: i64) {
    gui_status_bar_in(c, aw, now, false, true);
}

/// The screen under the bar, saved before the bar is drawn: Live view's page tiles take these pixels
/// where the bar covers them, so hiding the bar's layer (LIVE_LAYER_STATUS) shows the page, not the bar.
fn status_under_save(c: &Canvas, w: i32, h: i32) {
    strip_save(&STATUS_UNDER, c, w, h);
}

/// Puts the saved screen under the bar into c at r (the part of the bar r covers). False when there
/// is no matching saved bar: the caller leaves the screen as it is.
pub fn gui_status_under_put(c: &mut Canvas, r: Rect) -> bool {
    strip_put(&STATUS_UNDER, c, r)
}

/// Puts the bar as it was last drawn on its band into c at r, the part of the strip r covers (#337: a notebook's
/// flush, where something drew into the bar's rows since: app.rs band_keep). False when no bar of that size was
/// drawn: the caller draws it again.
pub fn gui_status_drawn_put(c: &mut Canvas, r: Rect) -> bool {
    strip_put(&STATUS_DRAWN, c, r)
}

/// Keeps the top w x h of c in `cell`.
fn strip_save(cell: &'static std::thread::LocalKey<RefCell<StatusUnder>>, c: &Canvas, w: i32, h: i32) {
    if w <= 0 || h <= 0 || c.stride < w {
        return;
    }
    let stride = c.stride as usize;
    if c.px.len() < (h as usize).saturating_mul(stride) {
        return;
    }
    cell.with(|u| {
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

/// Puts what `cell` keeps into c at r. False when r is not inside what it keeps.
fn strip_put(cell: &'static std::thread::LocalKey<RefCell<StatusUnder>>, c: &mut Canvas, r: Rect) -> bool {
    if r.w <= 0 || r.h <= 0 || r.x < 0 || r.y < 0 || r.x + r.w > c.w || r.y + r.h > c.h || c.stride < c.w {
        return false;
    }
    let stride = c.stride as usize;
    if c.px.len() < (c.h as usize).saturating_mul(stride) {
        return false;
    }
    cell.with(|u| {
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

/// What the bar's right end shows: the battery when it can be read, and each radio the device has, on or off.
#[derive(Clone, Copy, Debug, Default)]
struct StatusItems {
    battery: Option<Battery>,
    /// Bluetooth's helper is there, and whether it runs
    bluetooth: Option<bool>,
    /// Wi-Fi's helper is there, and whether the radio is on
    wifi: Option<bool>,
}

fn status_items() -> StatusItems {
    StatusItems {
        battery: battery_cached(),
        bluetooth: bt_present().then(|| bt_state() != BtState::Stopped),
        wifi: wifi_available().then(|| wifi_status().is_some_and(|s| s.enabled)),
    }
}

fn gui_status_bar_in(c: &mut Canvas, aw: i32, now: i64, clock: bool, fill: bool) {
    gui_status_bar_items(c, aw, now, clock, fill, status_items());
}

fn gui_status_bar_items(c: &mut Canvas, aw: i32, now: i64, clock: bool, fill: bool, items: StatusItems) {
    let scale = status_scale();
    let u = |v: f32| ui_px(v * scale);
    let h = gui_status_h();
    let m = gui_margin();
    let band = GUI_GRAY_STATUS_BAND;
    status_under_save(c, aw, h);
    if fill {
        canvas_fill(c, Rect::new(0, 0, aw, h), band);
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
        // (black on the band, as the time is: the face tells the date from the time)
        font_draw_left(
            c,
            FontFace::Regular,
            u(32.0) as f32,
            Rect::new(date_x, 0, (aw / 2 - date_x).max(0), h),
            &date,
            0,
        );
    }
    let isz = u(48.0);
    let mut x = aw - m;
    if clock {
        if let Some(b) = items.battery {
            let bw = u(56.0);
            let bh = u(34.0);
            x -= bw + u(8.0);
            gui_battery(c, Rect::new(x, (h - bh) / 2, bw, bh), b.percent, if fill { band } else { 255 });
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
    if let Some(on) = items.bluetooth {
        x -= isz;
        ui_icon(c, IconKind::Bluetooth, Rect::new(x, (h - isz) / 2, isz, isz), if on { 0 } else { GUI_GRAY_STATUS_OFF });
        x -= u(22.0);
    }
    if let Some(on) = items.wifi {
        x -= isz;
        ui_icon(c, IconKind::Wifi, Rect::new(x, (h - isz) / 2, isz, isz), if on { 0 } else { GUI_GRAY_STATUS_OFF });
    }
    if fill && clock {
        strip_save(&STATUS_DRAWN, c, aw, h);
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

    // The one header's Back, title and actions stand where every screen's header has them, by numbers of their own
    // (the home's included since its bat went, #332): at 300 dpi under the medium bar, the title at the 50 px margin,
    // or 22 px after Back, to 20 px before the first action, over the 150 px band; the actions 96 px round, 12 px
    // apart, at the right margin, in the order given.
    #[test]
    fn a_headers_back_title_and_actions_stand_where_every_screen_has_them() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        gui_status_style(1, false);
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            let mut plain = Canvas::default();
            assert_eq!(canvas_init(&mut plain, 1860, 400), 0);
            let acts = [(IconKind::Search, 1), (IconKind::Gear, 2)];
            let (h0, a0) = gui_header(&mut plain, 1860, "Library", false, &acts);
            assert!(rect_is_empty(h0.back), "{theme:?}");
            assert_eq!((h0.title, h0.foot), (Rect::new(50, 78, 1524, 150), 228), "{theme:?}");
            assert_eq!(a0, [(Rect::new(1606, 105, 96, 96), 1), (Rect::new(1714, 105, 96, 96), 2)], "{theme:?}");
            let (hb, ab) = gui_header(&mut plain, 1860, "School", true, &acts);
            assert_eq!((hb.back, hb.title), (Rect::new(50, 105, 96, 96), Rect::new(168, 78, 1406, 150)), "{theme:?}");
            assert_eq!(ab, a0, "{theme:?}: the actions stay where they were");
            canvas_free(&mut plain);
        }
        ui_set_theme(UiTheme::Gothic);
    }

    // Classic's + button is a rounded rectangle at the theme's radius (#332; it was a disc), the same size: a point
    // 10 px in from its corner is inside it, which a disc's would not be, and the corner itself is outside. Gothic's
    // carved square keeps its cut corners and its white inner line.
    #[test]
    fn classics_plus_button_is_a_rounded_rectangle_and_gothics_is_carved() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let r = Rect::new(20, 20, 144, 144);
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            let mut c = Canvas::default();
            assert_eq!(canvas_init(&mut c, 184, 184), 0);
            canvas_fill(&mut c, Rect::new(0, 0, 184, 184), 255);
            let shape = gui_fab(&mut c, r, IconKind::Plus);
            assert_eq!(shape, if theme == UiTheme::Classic { GuiFabShape::Round(gui_radius()) } else { GuiFabShape::Cut(ui_px(22.0)) }, "{theme:?}: what it says it drew");
            let px = |x: i32, y: i32| c.px[(y * c.stride + x) as usize];
            for (cx, cy) in [(r.x, r.y), (r.x + r.w - 1, r.y), (r.x, r.y + r.h - 1), (r.x + r.w - 1, r.y + r.h - 1)] {
                assert_eq!(px(cx, cy), 255, "{theme:?}: the corner ({cx}, {cy}) is outside");
            }
            if theme == UiTheme::Classic {
                assert_eq!(gui_fab_radius(), gui_radius());
                for (x, y) in [(r.x + 10, r.y + 10), (r.x + r.w - 11, r.y + 10), (r.x + 10, r.y + r.h - 11), (r.x + r.w - 11, r.y + r.h - 11)] {
                    assert_eq!(px(x, y), 0, "{theme:?}: ({x}, {y}) is inside the rounded rectangle");
                }
            } else {
                assert_eq!(gui_fab_radius(), ui_px(18.0));
                // (the cut corner, 22 px, leaves (r.x + 10, r.y + 10) outside; the white line 9 px in)
                assert_eq!(px(r.x + 10, r.y + 10), 255, "{theme:?}");
                assert_eq!(px(r.x + 40, r.y + 10), 255, "{theme:?}: the inner line");
                assert_eq!(px(r.x + 40, r.y + 4), 0, "{theme:?}");
            }
            canvas_free(&mut c);
        }
        ui_set_theme(UiTheme::Gothic);
    }

    // The page's rules rank as the owner chose them (#332): in Classic the title and foot rules are black, the section
    // break a plain line of gray 102 with no dots, and the light rule gray 136 in both themes; every gray is one of
    // the panel's sixteen levels.
    #[test]
    fn classics_section_break_is_gray_between_the_black_rules_and_the_light_rule() {
        ui_init(300.0);
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            let mut c = Canvas::default();
            assert_eq!(canvas_init(&mut c, 400, 200), 0);
            canvas_fill(&mut c, Rect::new(0, 0, 400, 200), 255);
            gui_title_rule(&mut c, 10, 390, 10);
            gui_section_break(&mut c, 10, 390, 80);
            gui_hair_rule(&mut c, 10, 390, 120);
            gui_foot_rule(&mut c, 10, 390, 160);
            let px = |x: i32, y: i32| c.px[(y * c.stride + x) as usize];
            assert_eq!((px(20, 10), px(20, 160 + gui_title_rule_h() - 1)), (0, 0), "{theme:?}: the title and foot rules");
            assert_eq!((px(20, 120), px(20, 121), px(20, 122)), (GUI_GRAY_HAIR, GUI_GRAY_HAIR, 255), "{theme:?}: the light rule");
            if theme == UiTheme::Classic {
                // one plain line, the same gray along its whole length, its two rows centred on 80
                for x in [10, 100, 199, 200, 201, 300, 389] {
                    assert_eq!((px(x, 78), px(x, 79), px(x, 80), px(x, 81)), (255, GUI_GRAY_SPLIT, GUI_GRAY_SPLIT, 255), "Classic: the split at x {x}");
                }
                assert_eq!(px(390, 79), 255);
            } else {
                assert_eq!(px(20, 80), 0, "Gothic: the ornament's rule is black");
            }
            canvas_free(&mut c);
        }
        assert!(GUI_GRAY_SPLIT < GUI_GRAY_HAIR && GUI_GRAY_SPLIT % 17 == 0 && GUI_GRAY_HAIR % 17 == 0);
        ui_set_theme(UiTheme::Gothic);
    }

    // An app's name under its square is whole (#332): one line at the small size when it fits, else a smaller size
    // down to four fifths where the face has one (Classic), else two lines at the small size (Gothic's pixel face),
    // and only a name two lines cannot hold either is cut short, which the fit check hears of.
    #[test]
    fn an_apps_long_name_takes_a_smaller_size_or_two_lines() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        thread_local! {
            static CUTS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
        }
        fn hear(text: &str, _need: i32, _have: i32) {
            CUTS.with(|c| c.borrow_mut().push(text.to_string()));
        }
        font_set_overflow_hook(Some(hear));
        let small = ui_small_size();
        let w = 210;
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            assert_eq!(gui_app_label_fit("Files", w), (small, 1), "{theme:?}");
            let long = "Pokaż wszystko";
            let (px, lines) = gui_app_label_fit(long, w);
            if theme == UiTheme::Gothic {
                assert_eq!((px, lines), (small, 2), "{theme:?}: two lines at the small size");
            } else {
                assert_eq!(lines, 1, "{theme:?}");
                assert!(px < small && px >= small * 0.8 && font_measure(FontFace::Bold, px, long) <= w, "{theme:?}: {px}");
            }
            let mut c = Canvas::default();
            assert_eq!(canvas_init(&mut c, 400, 400), 0);
            canvas_fill(&mut c, Rect::new(0, 0, 400, 400), 255);
            let sq = Rect::new(112, 20, gui_app_square_side(), gui_app_square_side());
            for name in ["Files", long, "Sudoku Puzzles", "パッケージストア"] {
                gui_app_square(&mut c, sq, w, &AppIcon::glyph(IconKind::Grid), name, false);
            }
            assert_eq!(CUTS.with(|c| c.borrow().clone()), Vec::<String>::new(), "{theme:?}: nothing cut short");
            // (the names stay inside their two lines' room)
            let below = sq.y + sq.h + gui_app_label_gap() + gui_app_label_h() + ui_px(4.0);
            assert!((below..400).all(|y| c.px[(y * c.stride) as usize..(y * c.stride + 400) as usize].iter().all(|&p| p == 255)), "{theme:?}");
            gui_app_square(&mut c, sq, w, &AppIcon::glyph(IconKind::Grid), "Supercalifragilisticexpialidocious", false);
            assert!(!CUTS.with(|c| c.borrow().is_empty()), "{theme:?}: a word wider than the square's step is heard of");
            CUTS.with(|c| c.borrow_mut().clear());
            canvas_free(&mut c);
        }
        font_set_overflow_hook(None);
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

    // The status bar stands on a band of gray 187 (#337, the owner's pick: "204 or even a step darker") its full
    // width and height, at every size, in both themes, upright and sideways, and on nothing below it: a panel level,
    // so a grays refresh shows it flat. The time and the date are black on it, an off radio's icon gray 102 (five
    // levels darker than the band: it reads, and tells off from the black of on), the battery's body the band's gray.
    #[test]
    fn the_status_bar_stands_on_a_band_of_one_panel_gray_its_full_width_and_height() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        assert_eq!(GUI_GRAY_STATUS_BAND % 17, 0);
        assert_eq!(GUI_GRAY_STATUS_OFF % 17, 0);
        assert!(GUI_GRAY_STATUS_BAND - GUI_GRAY_STATUS_OFF >= 4 * 17 && GUI_GRAY_STATUS_OFF >= 4 * 17);
        let off = StatusItems {
            battery: Some(Battery { percent: 87, charging: false }),
            bluetooth: Some(false),
            wifi: Some(true),
        };
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            for size in 0..STATUS_SIZES {
                gui_status_style(size, false);
                for aw in [1860, 2480] {
                    let mut c = Canvas::default();
                    assert_eq!(canvas_init(&mut c, aw, 300), 0);
                    canvas_fill(&mut c, Rect::new(0, 0, aw, 300), 255);
                    gui_status_bar_items(&mut c, aw, 1_795_772_460, true, true, off);
                    let h = gui_status_h();
                    let at = |x: i32, y: i32| c.px[(y * c.stride + x) as usize];
                    let at_ = format!("{theme:?} size {size} {aw}");
                    // the band: every column at its top and bottom rows, both ends of every row
                    for x in 0..aw {
                        assert_eq!((at(x, 0), at(x, h - 1)), (GUI_GRAY_STATUS_BAND, GUI_GRAY_STATUS_BAND), "{at_}: column {x}");
                        assert_eq!(at(x, h), 255, "{at_}: nothing under the band, column {x}");
                    }
                    for y in 0..h {
                        assert_eq!((at(0, y), at(aw - 1, y)), (GUI_GRAY_STATUS_BAND, GUI_GRAY_STATUS_BAND), "{at_}: row {y}");
                    }
                    // nothing on the band is lighter than it; the time and the date are black
                    let row: Vec<u8> = (0..h).flat_map(|y| (0..aw).map(move |x| (x, y))).map(|(x, y)| at(x, y)).collect();
                    assert!(row.iter().all(|&v| v <= GUI_GRAY_STATUS_BAND), "{at_}");
                    let left: Vec<u8> = (0..h).flat_map(|y| (0..aw / 2).map(move |x| (x, y))).map(|(x, y)| at(x, y)).collect();
                    assert!(left.iter().filter(|&&v| v < 17).count() > 200, "{at_}: the time and the date in black");
                    // Bluetooth off: its glyph's box holds gray 102 at its core and nothing darker; Wi-Fi on holds black
                    let isz = ui_px(48.0 * status_scale());
                    let u = |v: f32| ui_px(v * status_scale());
                    let pct_w = font_measure(FontFace::Regular, u(30.0) as f32, "87%");
                    let bt_right = aw - gui_margin() - u(56.0) - u(8.0) - u(10.0) - pct_w - u(28.0);
                    let glyph = |x0: i32| -> Vec<u8> {
                        (0..h).flat_map(|y| (x0..x0 + isz).map(move |x| (x, y))).map(|(x, y)| at(x, y)).collect()
                    };
                    let bt = glyph(bt_right - isz);
                    let (lo, hi) = (bt.iter().copied().min().unwrap_or(255), bt.iter().copied().max().unwrap_or(0));
                    // (its core at the off gray's panel level: the glyph's edge blends round it by a value or so)
                    assert!(lo >> 4 == GUI_GRAY_STATUS_OFF >> 4 && hi <= GUI_GRAY_STATUS_BAND, "{at_}: Bluetooth off, {lo} to {hi}");
                    let wifi = glyph(bt_right - isz - u(22.0) - isz);
                    assert!(wifi.iter().any(|&v| v < 17), "{at_}: Wi-Fi on");
                    canvas_free(&mut c);
                }
            }
        }
        gui_status_style(1, false);
        ui_set_theme(UiTheme::Gothic);
    }
}
