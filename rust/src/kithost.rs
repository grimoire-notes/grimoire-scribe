//! The package kit (2.3, issue #267; docs/KIT.md is its reference): the `grimoire_kit` import module, which the
//! widget host (widgetwasm.rs) and the app host (apphost.rs) both link beside their own `grimoire` module. Its
//! calls are written once, here, and mean the same in a widget's frame and in an app: the theme's standard sizes
//! (`metric`), the kit's level and Grimoire's version, text by role, and the theme's elements, which take the
//! theme's standard size when they are given none, so that a package's screen is drawn as Grimoire's own are.
//! Each host's `grimoire` module stays as it was; a module may import from both, and its commands are drawn in
//! the order it called them.
//!
//! Every call keeps the rules of the host it runs in (wasmhost.rs; docs/WIDGETS.md and docs/APPS.md, Limits):
//! the fixed charge first, whatever the call is given; a length compared with its limit before the module's
//! memory is read; the host's work paid from the call's budget, by the app host's charges (apphost.rs) in both
//! hosts; drawing held to the body; a drawing command and a hit region counted against the host's own limits;
//! and a widget's drawing calls, as its own, only for a package that declares `editor:overlay`. A host gives
//! the kit its body, its limits and its records through `KitHost`. The work of each call is a function here that
//! no host's type is part of, so one copy of it serves both hosts: `kit_link` installs, for each host, a call
//! that hands the function the call's budget, the module's memory and the host (`Call`).

use crate::apphost::{
    APP_CHARGE_BOLD, APP_CHARGE_BYTE, APP_CHARGE_CALL, APP_CHARGE_CORNER, APP_CHARGE_EDGE_PX, APP_CHARGE_FILL_PX,
    APP_CHARGE_ICON_PX, APP_CHARGE_LABEL_PX, APP_CHARGE_LINE_ROW, APP_CHARGE_MEASURE, APP_CHARGE_SMALL_ICON_PX,
    APP_CHARGE_COPY_BYTES, APP_CHARGE_SQUARE, APP_CHARGE_SQUARE_BYTES, APP_CHARGE_SWITCH, APP_CHARGE_TALL_PX, APP_CHARGE_TEXT,
    APP_ICON_NAME_MAX, APP_ICON_PX_MAX, APP_LINE_WIDTH_MAX, APP_TEXT_PX_MAX,
};
use crate::globalui::{
    gui_button, gui_button_h, gui_button_row, gui_button_row_need_w, gui_chip, gui_chip_h, gui_chip_w, gui_field_h, gui_gap, gui_gothic, gui_head_h,
    gui_icon_button, gui_margin, gui_page_title_px, gui_row_h, gui_row_min_h, gui_select, gui_select_h, gui_text_field,
    gui_top_button_h, GUI_GRAY_DISABLED, GUI_GRAY_MUTED, GUI_GRAY_RULE,
};
use crate::prelude::*;
use crate::wasmhost::{in_body, line_in_reach, on_screen, TextCharge};
use wasmi::{Caller, FuncType, Linker, Val, ValType};

/// The import module's name.
pub const KIT: &str = "grimoire_kit";
/// The kit's level: the set of calls, metrics, roles and flags this Grimoire has (docs/KIT.md, Levels).
pub const KIT_LEVEL: i32 = 1;
/// The calls of level 1, every one a module may import from `grimoire_kit`: the name, the types of the parameters
/// (`i` an i32, `f` an f32) and of the result (`v` none, `i` an i32, `l` an i64). `dispatch` takes them by their
/// place here.
pub const KIT_CALLS: [(&str, &str, char); 28] = [
    ("level", "", 'i'),
    ("version", "ii", 'i'),
    ("metric", "i", 'i'),
    ("clock_ms", "", 'l'),
    ("text", "iiiiiiii", 'i'),
    ("text_lines", "iiiiiiiii", 'i'),
    ("measure_width", "iii", 'i'),
    ("measure_height", "iiii", 'i'),
    ("button", "iiiiiiiiii", 'i'),
    ("icon_button", "iiiiiiii", 'i'),
    ("row", "iiiiiiiiiii", 'i'),
    ("chip", "iiiiiiii", 'i'),
    ("field", "iiiiiiiiiiii", 'i'),
    ("select", "iiiiiiiii", 'i'),
    ("checkbox", "iiiiiiii", 'i'),
    ("card", "iiiii", 'v'),
    ("divider", "iii", 'v'),
    ("icon", "iiiiiii", 'v'),
    ("fill", "iiiii", 'v'),
    ("frame", "iiiiiii", 'v'),
    ("line", "fffffi", 'v'),
    ("clip", "iiii", 'v'),
    ("clip_clear", "", 'v'),
    ("hit", "iiiii", 'v'),
    ("list_page", "iiiii", 'l'),
    ("pager", "iiiiiii", 'i'),
    ("event_points", "ii", 'i'),
    ("key_char", "ii", 'i'),
];

// ---- the limits of the kit's own (the rest are each host's: docs/KIT.md, Limits) ----

/// The bytes `version` writes at most.
pub const KIT_VERSION_MAX: usize = 32;
/// The rows `list_page` and `pager` count at most; more is taken as this many.
pub const KIT_LIST_ROWS_MAX: i32 = 10_000;
/// The heights `list_page` lays out with, in pixels, at most; a larger one is taken as this.
pub const KIT_LIST_PX_MAX: i32 = 32_767;
/// A frame's outline is at most this many pixels thick; a thicker one is drawn at this thickness.
pub const KIT_THICKNESS_MAX: i32 = 4096;

// ---- the charges of the kit's own (the elements are charged as the app host's are: apphost.rs) ----

/// Every call of the kit, whatever it is given and whether or not the package may draw.
pub const KIT_CHARGE_CALL: u64 = APP_CHARGE_CALL;
/// A text broken into lines that is drawn (`text` with WRAP, `text_lines`): this for each byte, beside its letters.
/// Each word is measured once, and a word wider than the line is broken between its letters by doubling and halving
/// (`kit_lines`).
pub const KIT_CHARGE_WRAP: u64 = 384;
/// A text broken into lines that is not drawn (`measure_height`): this for each byte, the work of breaking it with
/// no letters' charge beside it (measured: its worst case takes about the budget's own time for what it pays).
pub const KIT_CHARGE_MEASURE_LINES: u64 = 1024;

// ---- metric ids (docs/KIT.md, Metrics) ----
pub const M_TEXT_BODY: i32 = 1;
pub const M_TEXT_SMALL: i32 = 2;
pub const M_TEXT_TITLE: i32 = 3;
pub const M_TEXT_DISPLAY: i32 = 4;
pub const M_LINE_BODY: i32 = 5;
pub const M_LINE_SMALL: i32 = 6;
pub const M_LINE_TITLE: i32 = 7;
pub const M_LINE_DISPLAY: i32 = 8;
pub const M_LINE_MONO: i32 = 9;
pub const M_ROW_H: i32 = 10;
pub const M_ROW_MIN_H: i32 = 11;
pub const M_BUTTON_H: i32 = 12;
pub const M_CHIP_H: i32 = 13;
pub const M_FIELD_H: i32 = 14;
pub const M_ICON: i32 = 15;
pub const M_ICON_BUTTON: i32 = 16;
pub const M_TOUCH_MIN: i32 = 17;
pub const M_HEADER_H: i32 = 18;
pub const M_SELECT_H: i32 = 19;
pub const M_MARGIN: i32 = 20;
pub const M_GAP: i32 = 21;
pub const M_ROW_GAP: i32 = 22;
pub const M_PAD: i32 = 23;
pub const M_RADIUS: i32 = 24;
pub const M_LINE: i32 = 25;
pub const M_SCALE_MILLI: i32 = 30;
pub const M_DPI: i32 = 31;
pub const M_THEME: i32 = 32;
pub const M_FONT: i32 = 33;
pub const M_STATUS_BAR_H: i32 = 34;
pub const M_KEYBOARD_TOP: i32 = 35;
pub const M_BODY_W: i32 = 36;
pub const M_BODY_H: i32 = 37;
pub const M_GRAY_INK: i32 = 40;
pub const M_GRAY_PAPER: i32 = 41;
pub const M_GRAY_MUTED: i32 = 42;
pub const M_GRAY_DISABLED: i32 = 43;
pub const M_GRAY_RULE: i32 = 44;

/// The theme's standard size or value `id`, in screen pixels unless docs/KIT.md says otherwise, for a call drawing
/// on a body of `body` under a status bar `status_bar_h` high; -1 for an id this level does not have. Each size a
/// screen of Grimoire's draws at is the function that screen reads (globalui.rs, ui.rs), so the two cannot differ;
/// FIELD_H, which no screen draws yet, is the kit's own.
pub fn kit_metric(id: i32, body: (i32, i32), status_bar_h: i32) -> i32 {
    let line = |role: i32| {
        let (f, px) = role_face(role);
        line_h(f, px)
    };
    match id {
        M_TEXT_BODY => ui_text_size().round() as i32,
        M_TEXT_SMALL => ui_small_size().round() as i32,
        M_TEXT_TITLE => ui_title_size().round() as i32,
        M_TEXT_DISPLAY => gui_page_title_px().round() as i32,
        M_LINE_BODY => line(ROLE_BODY),
        M_LINE_SMALL => line(ROLE_SMALL),
        M_LINE_TITLE => line(ROLE_TITLE),
        M_LINE_DISPLAY => line(ROLE_DISPLAY),
        M_LINE_MONO => line(ROLE_MONO),
        M_ROW_H => gui_row_h(),
        M_ROW_MIN_H | M_TOUCH_MIN => gui_row_min_h(),
        M_BUTTON_H => gui_button_h(),
        M_CHIP_H => gui_chip_h(),
        M_FIELD_H => gui_field_h(),
        M_ICON => ui_row_icon(),
        M_ICON_BUTTON => gui_top_button_h(),
        M_HEADER_H => gui_head_h(),
        M_SELECT_H => gui_select_h(),
        M_MARGIN => gui_margin(),
        M_GAP => gui_gap(),
        M_ROW_GAP => ui_row_space(),
        M_PAD => ui_row_pad(),
        M_RADIUS => ui_card_radius(),
        M_LINE => ui_line(),
        M_SCALE_MILLI => ui_px(1000.0),
        M_DPI => ui_px(300.0),
        M_THEME => (ui_theme() == UiTheme::Classic) as i32,
        // (Classic's family of Settings > Font, 4 for the reader's own font, and 5 for Gothic's faces)
        M_FONT => {
            if gui_gothic() {
                5
            } else if font_own() {
                4
            } else {
                font_family() as i32
            }
        }
        M_STATUS_BAR_H => status_bar_h,
        // (no keyboard of the kit's is open in this level)
        M_KEYBOARD_TOP => -1,
        M_BODY_W => body.0,
        M_BODY_H => body.1,
        M_GRAY_INK => 0,
        M_GRAY_PAPER => 255,
        M_GRAY_MUTED => GUI_GRAY_MUTED as i32,
        M_GRAY_DISABLED => GUI_GRAY_DISABLED as i32,
        M_GRAY_RULE => GUI_GRAY_RULE as i32,
        _ => -1,
    }
}

// ---- text by role ----

pub const ROLE_BODY: i32 = 0;
pub const ROLE_SMALL: i32 = 1;
pub const ROLE_LABEL: i32 = 2;
pub const ROLE_CAPTION: i32 = 3;
pub const ROLE_TITLE: i32 = 4;
pub const ROLE_MONO: i32 = 5;
pub const ROLE_MONO_SMALL: i32 = 6;
pub const ROLE_DISPLAY: i32 = 7;

/// The face and the size of a role in the theme in use; a role this level does not have is BODY.
pub fn role_face(role: i32) -> (FontFace, f32) {
    match role {
        ROLE_SMALL => (FontFace::Regular, ui_small_size()),
        ROLE_LABEL => (FontFace::Bold, ui_text_size()),
        ROLE_CAPTION => (FontFace::Bold, ui_small_size()),
        ROLE_TITLE => (FontFace::Title, ui_title_size()),
        ROLE_MONO => (FontFace::Fixed, ui_text_size()),
        ROLE_MONO_SMALL => (FontFace::Fixed, ui_small_size()),
        ROLE_DISPLAY => (FontFace::Title, gui_page_title_px()),
        _ => (FontFace::Regular, ui_text_size()),
    }
}

fn line_h(f: FontFace, px: f32) -> i32 {
    (font_ascent(f, px) + font_descent(f, px)).max(1)
}

/// A text's flags: its alignment (0 left, 1 centred, 2 right), WRAP, FIT and its tone.
pub const FLAG_ALIGN: i32 = 3;
pub const FLAG_WRAP: i32 = 4;
pub const FLAG_FIT: i32 = 8;

/// The gray of a tone (a text's flags >> 4, an icon's tone): ink, muted, disabled or paper; ink for another.
fn tone_gray(tone: i32) -> u8 {
    match tone {
        1 => GUI_GRAY_MUTED,
        2 => GUI_GRAY_DISABLED,
        3 => 255,
        _ => 0,
    }
}

/// How many bytes of `s` from its start fit `w` (at a character's end; 0 when not even the first character
/// does; all of `s` when it fits). The count of characters is found by doubling from one, then halving: the
/// prefixes measured add up to a few times the part that fits, however long `s` is.
fn fitting(f: FontFace, px: f32, s: &str, w: i32) -> usize {
    let fits = |end: usize| font_measure(f, px, &s[..end]) <= w;
    let mut ends = s.char_indices().map(|(i, c)| i + c.len_utf8());
    // (at[k]: where the first k + 1 characters end, read as far as needed)
    let mut at: Vec<usize> = Vec::new();
    let (mut lo, mut step) = (0usize, 1usize);
    let mut hi = loop {
        let want = lo + step;
        while at.len() < want {
            match ends.next() {
                Some(end) => at.push(end),
                None => break,
            }
        }
        if at.len() < want {
            let all = at.len();
            if all == lo || fits(at[all - 1]) {
                return s.len();
            }
            break all;
        }
        if !fits(at[want - 1]) {
            break want;
        }
        lo = want;
        step *= 2;
    };
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if fits(at[mid - 1]) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        0
    } else {
        at[lo - 1]
    }
}

/// The lines of a text in a face for a width: at its own line breaks, then as many whole words as fit on each
/// line, and a word wider than the width broken between its characters (at least one to a line). The license
/// viewer breaks a license so (widgetpkgui.rs, `license_lines`); `text` with WRAP, `text_lines` and
/// `measure_height` break a package's text so, its work growing with the text's length times the log of a line's.
/// (A carriage return before a line break is white space between words, as a tab is.)
pub fn kit_lines(f: FontFace, px: f32, text: &str, w: i32) -> Vec<String> {
    let space = font_measure(f, px, " ");
    let mut out = Vec::new();
    for raw in text.split('\n') {
        let mut line = String::new();
        let mut lw = 0i32;
        for word in raw.split_whitespace() {
            let ww = font_measure(f, px, word);
            if !line.is_empty() && lw.saturating_add(space).saturating_add(ww) > w {
                out.push(std::mem::take(&mut line));
                lw = 0;
            }
            if !line.is_empty() {
                line.push(' ');
                lw += space;
            }
            if ww <= w {
                line.push_str(word);
                lw += ww;
                continue;
            }
            // (a word wider than the line starts one of its own: as many of its characters to a line as fit, one at
            // least, and the rest of the word, which fits, on the line words go on from)
            let mut rest = word;
            loop {
                let n = fitting(f, px, rest, w).max(rest.chars().next().map_or(0, char::len_utf8));
                if n >= rest.len() {
                    break;
                }
                out.push(rest[..n].to_string());
                rest = &rest[n..];
            }
            line.push_str(rest);
            lw = font_measure(f, px, rest);
        }
        out.push(line);
    }
    out
}

/// One line of text in `r`, vertically centred, at its alignment; cut short with "..." where it is too wide (as
/// Grimoire's labels are). Right-aligned text that is too wide is drawn as left-aligned text is: it fills the
/// width.
fn one_line(c: &mut Canvas, f: FontFace, px: f32, r: Rect, text: &str, align: i32, gray: u8) {
    match align {
        1 => font_draw_centered(c, f, px, r, text, gray),
        2 if font_measure(f, px, text) <= r.w => {
            let w = font_measure(f, px, text);
            let base = r.y + (r.h - line_h(f, px)) / 2 + font_ascent(f, px);
            font_draw(c, f, px, r.x + r.w - w, base, text, gray);
        }
        _ => font_draw_left_name(c, f, px, r, text, gray),
    }
}

/// The size a FIT text is drawn at in a width: its role's size, else a smaller one down to the small size that
/// holds it on one line; None when it takes two lines at the smallest.
fn fit_px(f: FontFace, px: f32, text: &str, w: i32) -> Option<f32> {
    let s = font_fit_px(f, px, px.min(ui_small_size()), text, w);
    (s > 0.0).then_some(s)
}

/// A text of `text`: one line, or Grimoire's label rule (FIT: a smaller size down to the small one, then two
/// lines), at the role's face and its flags' alignment and tone.
fn draw_text(c: &mut Canvas, r: Rect, text: &str, role: i32, flags: i32) {
    let (f, px) = role_face(role);
    let (align, gray) = (flags & FLAG_ALIGN, tone_gray((flags >> 4) & 3));
    if flags & FLAG_FIT == 0 {
        return one_line(c, f, px, r, text, align, gray);
    }
    match fit_px(f, px, text, r.w) {
        Some(s) => one_line(c, f, s, r, text, align, gray),
        None => {
            let min = px.min(ui_small_size());
            font_draw_wrapped(c, f, min, r, text, 2.min((r.h / line_h(f, min)).max(1)), 0, align == 1, gray);
        }
    }
}

/// Lines of a role from the top of `r` (a page of `text_lines`), or centred in it with the last one shown cut
/// short when more follow (`text` with WRAP): as many as `r` is high for, one at least.
fn draw_lines(c: &mut Canvas, r: Rect, lines: &[String], role: i32, flags: i32, wrapped: bool) {
    let (f, px) = role_face(role);
    let (align, gray) = (flags & FLAG_ALIGN, tone_gray((flags >> 4) & 3));
    let lh = line_h(f, px);
    let shown = ((r.h / lh).max(1) as usize).min(lines.len());
    let top = if wrapped { r.y + (r.h - shown as i32 * lh) / 2 } else { r.y };
    for (i, line) in lines.iter().take(shown).enumerate() {
        let slot = Rect::new(r.x, top + i as i32 * lh, r.w, lh);
        if wrapped && i + 1 == shown && shown < lines.len() {
            // (the rest of the text, on the last line there is room for, cut short)
            return one_line(c, f, px, slot, &lines[i..].join(" "), align, gray);
        }
        one_line(c, f, px, slot, line, align, gray);
    }
}

// ---- the commands ----

/// One drawing command of the kit, with its rectangle relative to the body's top left, as the call recorded it
/// (its sizes taken, its texts read): what `kit_render` draws. A host keeps it among its own commands, in the
/// order the module called them.
#[derive(Clone, Debug)]
pub enum KitCmd {
    /// a text on one line, or fitted (FIT): its text, role and flags
    Text(Rect, String, i32, i32),
    /// lines: a page of `text_lines` (false), or a text with WRAP (true)
    Lines(Rect, Vec<String>, i32, i32, bool),
    /// label, icon, style
    Button(Rect, String, String, i32),
    /// icon, style
    IconButton(Rect, String, i32),
    /// icon, label, style, switch
    Row(Rect, String, String, i32, i32),
    Chip(Rect, String, bool),
    /// icon, text, hint, the text drawn as stars
    Field(Rect, String, String, String, bool),
    /// caption, value
    Select(Rect, String, String),
    Checkbox(Rect, String, bool),
    Card(Rect, i32),
    Divider(Rect),
    Icon(Rect, String, u8),
    Fill(Rect, u8),
    /// gray, filled, thickness
    Frame(Rect, u8, bool, i32),
    Line(f32, f32, f32, f32, f32, u8),
    Clip(Rect),
    ClipClear,
    /// the pager's rectangle, its page and its pages
    Pager(Rect, i32, i32),
}

fn style_of(v: i32) -> ButtonStyle {
    match v {
        1 => ButtonStyle::Selected,
        2 => ButtonStyle::Disabled,
        3 => ButtonStyle::Primary,
        _ => ButtonStyle::Plain,
    }
}

fn icon_of(name: &str) -> IconKind {
    if name.is_empty() {
        IconKind::None
    } else {
        widget_icon(name)
    }
}

/// The arrows of a pager in `r`: Settings' (`ui_row_page`'s), at its left and right ends.
fn pager_arrows(r: Rect) -> (Rect, Rect) {
    let (bw, bh) = ui_row_pager_button();
    let next_x = (r.x as i64 + r.w as i64 - bw as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    (Rect::new(r.x, r.y, bw, bh), Rect::new(next_x, r.y, bw, bh))
}

/// Draws one command of the kit on `body` of the canvas, with `clip` the part of the canvas the host draws on
/// (the body, or less), in the theme in use. Its texts go on `texts`, as the host's own commands' do, for the
/// probes. A package's texts are not reported to Grimoire's translation fit check, and an icon is drawn at most
/// APP_ICON_PX_MAX each way, as an app's are.
pub fn kit_render(c: &mut Canvas, body: Rect, clip: Rect, cmd: KitCmd, texts: &mut Vec<String>) {
    font_without_overflow_hook(|| font_with_icon_side_max(APP_ICON_PX_MAX, || render(c, body, clip, cmd, texts)));
}

fn render(c: &mut Canvas, body: Rect, clip: Rect, cmd: KitCmd, texts: &mut Vec<String>) {
    let at = |r: Rect| on_screen(body, r.x, r.y, r.w, r.h);
    match cmd {
        KitCmd::Text(r, text, role, flags) => {
            draw_text(c, at(r), &text, role, flags);
            texts.push(text);
        }
        KitCmd::Lines(r, lines, role, flags, wrapped) => {
            draw_lines(c, at(r), &lines, role, flags, wrapped);
            texts.push(lines.join(" "));
        }
        KitCmd::Button(r, label, icon, style) => {
            // (with an icon and a label, the Package Store's action button, the label beside the icon in both themes;
            // with one of them, Grimoire's button)
            let kind = icon_of(&icon);
            if kind != IconKind::None && !label.is_empty() {
                gui_button_row(c, at(r), kind, &label, style_of(style));
            } else {
                gui_button(c, at(r), kind, &label, style_of(style));
            }
            texts.push(label);
        }
        KitCmd::IconButton(r, icon, style) => gui_icon_button(c, at(r), icon_of(&icon), style_of(style)),
        KitCmd::Row(r, icon, label, style, switch) => {
            let mut row = Row::new(icon_of(&icon), &label).named();
            row.style = style_of(style);
            if switch >= 0 {
                row.switch = Some(switch != 0);
            }
            ui_row(c, at(r), &row);
            texts.push(label);
        }
        KitCmd::Chip(r, label, selected) => {
            gui_chip(c, at(r), &label, selected);
            texts.push(label);
        }
        KitCmd::Field(r, icon, text, hint, stars) => {
            // (a password's text is drawn as a star for each of its characters, as Grimoire's keyboard draws one,
            // and the probes read the stars)
            let shown = if stars { "*".repeat(text.chars().count()) } else { text };
            gui_text_field(c, at(r), icon_of(&icon), &shown, &hint);
            texts.push(shown);
            texts.push(hint);
        }
        KitCmd::Select(r, caption, value) => {
            gui_select(c, at(r), &caption, &value);
            texts.push(value);
        }
        KitCmd::Checkbox(r, label, checked) => {
            ui_checkbox(c, at(r), &label, checked);
            texts.push(label);
        }
        KitCmd::Card(r, style) => ui_card(c, at(r), style_of(style)),
        KitCmd::Divider(r) => {
            let r = at(r);
            ui_rule(c, r.x, r.x + r.w, r.y, ui_line(), 0);
        }
        KitCmd::Icon(r, name, gray) => ui_icon(c, widget_icon(&name), at(r), gray),
        KitCmd::Fill(r, gray) => canvas_fill(c, at(r), gray),
        KitCmd::Frame(r, gray, filled, t) => ui_frame(c, at(r), ui_card_radius(), gray, filled, t),
        KitCmd::Line(x0, y0, x1, y1, width, gray) => {
            if let Some((x0, y0, x1, y1)) = line_in_reach(body, x0, y0, x1, y1) {
                canvas_line_aa(c, body.x as f32 + x0, body.y as f32 + y0, body.x as f32 + x1, body.y as f32 + y1, width, gray);
            }
        }
        KitCmd::Clip(r) => canvas_set_clip(c, rect_intersect(at(r), clip)),
        KitCmd::ClipClear => canvas_set_clip(c, clip),
        KitCmd::Pager(r, page, pages) => {
            let (prev, next) = pager_arrows(at(r));
            ui_row_pager(c, &RowPage { rows: Vec::new(), first: 0, page, pages, prev, next });
        }
    }
}

// ---- the charges (the app host's numbers: apphost.rs and docs/APPS.md, "What a call's budget pays for") ----

/// The app host's numbers for the charge of a text (`wasmhost::TextCharge`).
const TEXT_CHARGE: TextCharge = TextCharge { byte_px: APP_CHARGE_TEXT, tall_px: APP_CHARGE_TALL_PX, square: APP_CHARGE_SQUARE, px_max: APP_TEXT_PX_MAX as u64 };

/// A command's rectangle as it is drawn (`on_screen`), relative to the top left of a body of `body`.
pub fn drawn(body: (i32, i32), r: Rect) -> Rect {
    on_screen(Rect::new(0, 0, body.0, body.1), r.x, r.y, r.w, r.h)
}

/// How far from its sides an element with round ends is worked out pixel by pixel: half its smaller side (the
/// radius of its ends in the theme that rounds them) and the kit's corner.
pub fn round_end(body: (i32, i32), r: Rect) -> i64 {
    let r = drawn(body, r);
    (r.w.min(r.h).max(0) / 2) as i64 + APP_CHARGE_CORNER
}

/// The charge for one frame of a shape (a fill, or an outline): one for every APP_CHARGE_FILL_PX pixels of its
/// rectangle inside the body, which are filled by rows, and APP_CHARGE_EDGE_PX for each of those within `edge`
/// pixels of its left and right sides, which are worked out one by one (a corner, a round end, a thick outline).
pub fn shape_charge(body: (i32, i32), r: Rect, edge: i64) -> u64 {
    let Some(r) = in_body(Rect::new(0, 0, body.0, body.1), r) else { return 0 };
    let (w, h) = (r.w as u64, r.h as u64);
    w * h / APP_CHARGE_FILL_PX + h * w.min(2 * edge.max(0) as u64) * APP_CHARGE_EDGE_PX
}

/// The charge for a line `width` pixels wide: for each row of the body it crosses, APP_CHARGE_LINE_ROW and
/// APP_CHARGE_EDGE_PX for each pixel of the row the canvas works out (the line's width along the row, and no
/// more than the row has in the line's box).
pub fn line_charge(body: (i32, i32), x0: f32, y0: f32, x1: f32, y1: f32, width: f32) -> u64 {
    let body = Rect::new(0, 0, body.0, body.1);
    let Some((x0, y0, x1, y1)) = line_in_reach(body, x0, y0, x1, y1) else { return 0 };
    let b = rect_intersect(rect_from_points(x0, y0, x1, y1, width * 0.5 + 1.0), body);
    if rect_is_empty(b) {
        return 0;
    }
    let (dx, dy) = ((x1 - x0) as f64, (y1 - y0) as f64);
    let along = if dy != 0.0 { ((width as f64 + 1.0) * (dx * dx + dy * dy).sqrt() / dy.abs()).ceil() + 4.0 } else { b.w as f64 };
    b.h as u64 * (APP_CHARGE_LINE_ROW + (b.w as f64).min(along) as u64 * APP_CHARGE_EDGE_PX)
}

/// The charge for a text of `bytes` bytes drawn `px` pixels high (a part of a pixel counts as one) in `face`: by
/// its bytes and its height, and by the cube of the height more in a face that may be printed over
/// (APP_CHARGE_BOLD).
pub fn drawn_text_charge(bytes: usize, px: f32, face: FontFace) -> u64 {
    let h = (px.ceil() as i64).clamp(1, APP_TEXT_PX_MAX as i64);
    let n = TEXT_CHARGE.of(bytes, h, 0);
    match face {
        FontFace::Bold | FontFace::Title | FontFace::Caps => n + bytes as u64 * (h * h * h) as u64 / APP_CHARGE_BOLD,
        _ => n,
    }
}

/// The charge for cutting a text of `bytes` bytes short to fit its room: the square of its bytes, counted to
/// APP_CHARGE_SQUARE_BYTES (a text is cut to its first 251 bytes before the steps that measure it: font.rs, `fit`).
pub fn cut_text_charge(bytes: usize) -> u64 {
    let m = (bytes as u64).min(APP_CHARGE_SQUARE_BYTES);
    APP_CHARGE_SQUARE * m * m
}

/// The charge for fitting a text of `bytes` bytes on up to two lines (Grimoire's label rule, FIT): the square of
/// all its bytes, by steps that measure all of it.
pub fn fit_text_charge(bytes: usize) -> u64 {
    APP_CHARGE_SQUARE * bytes as u64 * bytes as u64
}

/// The charge for a label, a caption, a value, a text or a hint of one of the elements: a text APP_CHARGE_LABEL_PX
/// high that is fitted to its room, on two lines where the element takes two (all its bytes are then in the
/// square) and on one elsewhere.
pub fn label_charge(bytes: usize, two_lines: bool) -> u64 {
    TEXT_CHARGE.of(bytes, APP_CHARGE_LABEL_PX, if two_lines { u64::MAX } else { APP_CHARGE_SQUARE_BYTES })
}

/// The charge for an icon `side` pixels each way (1 to APP_ICON_PX_MAX): by the square of its side.
pub fn icon_charge(side: i64) -> u64 {
    let s = side.clamp(1, APP_ICON_PX_MAX as i64) as u64;
    s * s * APP_CHARGE_ICON_PX
}

/// The charge for an icon as large as the smaller side of `r` as drawn, or the `part`th of it.
fn icon_in(body: (i32, i32), r: Rect, part: i32) -> u64 {
    let r = drawn(body, r);
    icon_charge((r.w.min(r.h) / part) as i64)
}

// ---- a call ----

/// What a host gives the kit's calls: its body, whether the package may draw, its limits and its records.
pub trait KitHost {
    /// The size of the body the call draws on: a drawing pass's, and in another call the last pass's.
    fn kit_body(&self) -> (i32, i32);
    /// Whether what the package draws is shown: a widget's `editor:overlay`; an app's always.
    fn kit_may_draw(&self) -> bool;
    /// Room for one more drawing command in the call, or the host's way of ending a call past a limit.
    fn kit_room(&self) -> Result<(), wasmi::Error>;
    /// Room for one more hit region, the same way.
    fn kit_hit_room(&self) -> Result<(), wasmi::Error>;
    /// Counts a text of `n` bytes about to be read, against the host's limits of one text and of the call's
    /// texts together.
    fn kit_text(&mut self, n: usize) -> Result<(), wasmi::Error>;
    /// Records a drawing command, a hit region, or both.
    fn kit_push(&mut self, cmd: Option<KitCmd>, hit: Option<(i32, Rect)>);
    /// Counts `n` bytes read from the module's memory.
    fn kit_read(&mut self, n: usize);
    /// The height of the status bar over the package: the notebook's for a widget, 0 for an app that hides it.
    fn kit_status_bar_h(&self) -> i32;
    /// The points of the DRAG_MOVE the module is being told of (`kit_event`), oldest first; none in another call.
    fn kit_points(&self) -> &[(i32, i32)];
}

/// A call of the kit as its work sees it: the call's budget, the module's memory and the host.
pub trait Call {
    /// Takes `n` from the call's budget (`wasmhost::charge`).
    fn charge(&mut self, n: u64) -> Result<(), wasmi::Error>;
    /// `len` bytes of the module's memory from `ptr`, counted by the host (`wasmhost::read_mem`).
    fn read(&mut self, ptr: i32, len: i32) -> Vec<u8>;
    /// Writes into the module's memory as much as `cap` and the memory take (`wasmhost::write_mem`).
    fn write(&mut self, ptr: i32, cap: i32, bytes: &[u8]) -> i32;
    fn host(&mut self) -> &mut dyn KitHost;
}

struct With<'a, 'b, T>(&'a mut Caller<'b, T>);

impl<T: KitHost> Call for With<'_, '_, T> {
    fn charge(&mut self, n: u64) -> Result<(), wasmi::Error> {
        crate::wasmhost::charge(self.0, n)
    }
    fn read(&mut self, ptr: i32, len: i32) -> Vec<u8> {
        let bytes = crate::wasmhost::read_mem(self.0, ptr, len);
        self.0.data_mut().kit_read(bytes.len());
        bytes
    }
    fn write(&mut self, ptr: i32, cap: i32, bytes: &[u8]) -> i32 {
        crate::wasmhost::write_mem(self.0, ptr, cap, bytes)
    }
    fn host(&mut self) -> &mut dyn KitHost {
        self.0.data_mut()
    }
}

/// Installs every call of `grimoire_kit` in a host's linker: each with its signature, and all of them one entry
/// that hands the call, by its place in KIT_CALLS, to `dispatch` (one entry for each host, whatever the number of
/// calls).
pub fn kit_link<T: KitHost + 'static>(linker: &mut Linker<T>) -> Result<(), String> {
    for (at, &(name, params, result)) in KIT_CALLS.iter().enumerate() {
        linker
            .func_new(KIT, name, func_type(params, result), move |mut caller: Caller<'_, T>, args: &[Val], out: &mut [Val]| {
                dispatch(&mut With(&mut caller), at, args, out)
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn func_type(params: &str, result: char) -> FuncType {
    let ty = |c: char| match c {
        'f' => ValType::F32,
        'l' => ValType::I64,
        _ => ValType::I32,
    };
    let results: Vec<ValType> = if result == 'v' { Vec::new() } else { vec![ty(result)] };
    FuncType::new(params.chars().map(ty).collect::<Vec<_>>(), results)
}

/// The first `N` numbers of a call.
fn first<const N: usize>(a: &[i32; 12]) -> [i32; N] {
    std::array::from_fn(|k| a[k])
}

/// Calls the work of the call at `at` in KIT_CALLS with the call's numbers (the interpreter has checked them against
/// its signature), and gives back its result.
fn dispatch(cx: &mut dyn Call, at: usize, args: &[Val], out: &mut [Val]) -> Result<(), wasmi::Error> {
    let mut a = [0i32; 12];
    let mut f = [0f32; 5];
    for (k, v) in args.iter().enumerate().take(12) {
        match v {
            Val::I32(v) => a[k] = *v,
            Val::F32(v) if k < 5 => f[k] = v.to_float(),
            _ => {}
        }
    }
    let i = |v: i32| Some(Val::I32(v));
    let got = match at {
        0 => i(level(cx, [])?),
        1 => i(version(cx, first(&a))?),
        2 => i(metric(cx, first(&a))?),
        3 => Some(Val::I64(clock_ms(cx, [])?)),
        4 => i(text(cx, first(&a))?),
        5 => i(text_lines(cx, first(&a))?),
        6 => i(measure_width(cx, first(&a))?),
        7 => i(measure_height(cx, first(&a))?),
        8 => i(button(cx, first(&a))?),
        9 => i(icon_button(cx, first(&a))?),
        10 => i(row(cx, first(&a))?),
        11 => i(chip(cx, first(&a))?),
        12 => i(field(cx, first(&a))?),
        13 => i(select(cx, first(&a))?),
        14 => i(checkbox(cx, first(&a))?),
        15 => card(cx, first(&a)).map(|_| None)?,
        16 => divider(cx, first(&a)).map(|_| None)?,
        17 => icon(cx, first(&a)).map(|_| None)?,
        18 => fill(cx, first(&a)).map(|_| None)?,
        19 => frame(cx, first(&a)).map(|_| None)?,
        20 => line(cx, f, a[5]).map(|_| None)?,
        21 => clip(cx, first(&a)).map(|_| None)?,
        22 => clip_clear(cx, []).map(|_| None)?,
        23 => hit(cx, first(&a)).map(|_| None)?,
        24 => Some(Val::I64(list_page(cx, first(&a))?)),
        25 => i(pager(cx, first(&a))?),
        26 => i(event_points(cx, first(&a))?),
        27 => i(key_char(cx, first(&a))?),
        _ => None,
    };
    if let (Some(v), Some(slot)) = (got, out.first_mut()) {
        *slot = v;
    }
    Ok(())
}

/// The first call of `grimoire_kit` a module imports that this Grimoire does not have (a call of a later level), as
/// the refusal names it: `grimoire_kit.<name>` for a name of a call's shape (`kit_call_shaped`), `grimoire_kit` alone
/// for any other. None when it has every one the module imports.
pub fn kit_missing_import(module: &wasmi::Module) -> Option<String> {
    let name = module.imports().find(|i| i.module() == KIT && !KIT_CALLS.iter().any(|c| c.0 == i.name()))?.name();
    Some(if kit_call_shaped(name) { format!("{KIT}.{name}") } else { KIT.to_string() })
}

/// The bytes of a kit call's name at most.
pub const KIT_CALL_NAME_MAX: usize = 64;

/// Whether `name` has the shape of a kit call's name, at this level or a later one: 1 to KIT_CALL_NAME_MAX bytes of
/// `a` to `z`, `0` to `9` and `_`.
pub fn kit_call_shaped(name: &str) -> bool {
    (1..=KIT_CALL_NAME_MAX).contains(&name.len()) && name.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

// ---- input: the events of `kit_event`, a contact recognised, and the pass input owes (docs/KIT.md, Events) ----
//
// A module may export `kit_event(kind, id, a, b, c) -> i32`. Both hosts call it for what happens in the body: a
// contact recognised as a tap, a press or a drag (`KitPointer`, one recogniser for both), a key (an app's), a change
// of the package's settings. Each is a call of the module with the budget and the stop rules of the host's other
// calls. What it returns says whether its picture changed: a drawing pass is then owed, not drawn at once; the
// host's tick draws it, once for every event since the last pass, at a pace while a contact is down (`KitInput`),
// and refreshes only the part of the screen the pass changed.

/// The export a module is told of input by.
pub const KIT_EVENT: &str = "kit_event";
/// The kinds of event this level sends (`kind`). A module ignores a kind it does not know, and returns 0.
pub const EV_TAP: i32 = 1;
pub const EV_PRESS: i32 = 2;
pub const EV_DRAG_START: i32 = 3;
pub const EV_DRAG_MOVE: i32 = 4;
pub const EV_DRAG_END: i32 = 5;
pub const EV_KEY: i32 = 6;
pub const EV_SETTINGS: i32 = 9;
/// The points a DRAG_MOVE holds at most: the newest this many samples gathered since the one before.
pub const KIT_POINTS_MAX: usize = 256;
/// The bytes of one point `event_points` writes: its x and its y, each an i32.
pub const KIT_POINT_BYTES: usize = 8;
/// While a contact is down, a pass input owes is drawn at most once in this many milliseconds: the pace a
/// selection's drag shows itself at (editor/select.rs, DRAG_SHOW_MS).
pub const KIT_PASS_MS: u32 = 120;

/// One event for `kit_event`: its kind and its four numbers (docs/KIT.md, Events), and for a DRAG_MOVE the points it
/// gathered, oldest first, which `event_points` copies during the call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KitEvent {
    pub kind: i32,
    pub id: i32,
    pub a: i32,
    pub b: i32,
    pub c: i32,
    pub points: Vec<(i32, i32)>,
}

impl KitEvent {
    pub fn new(kind: i32, id: i32, a: i32, b: i32, c: i32) -> KitEvent {
        KitEvent { kind, id, a, b, c, points: Vec::new() }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Contact {
    /// no contact
    #[default]
    Up,
    /// down, within the slop, not yet held: a tap if it lifts so
    Down,
    /// held HOLD_MS within the slop: a press; its lift is no tap
    Pressed,
    /// moved past the slop
    Dragging,
    /// cancelled (a palm, or the platform took it for a gesture): nothing more until its lift
    Cancelled,
}

/// One contact on a body, recognised as a tap, a press or a drag, for widgets and apps alike. The host gives it the
/// contact's samples, body-local, and its tick; each answers the events to send. A contact that comes down is
/// nothing yet; one that lifts within the slop of where it came down, before HOLD_MS, is a TAP; one held HOLD_MS
/// within the slop is a PRESS, and its lift is nothing; one that moves past the slop (a press too) is a drag:
/// DRAG_START, then its samples gathered (the newest KIT_POINTS_MAX) for a DRAG_MOVE the host asks for at the pace of
/// its passes (`take_moves`), then at the lift a last DRAG_MOVE with the samples left and DRAG_END. A contact the
/// platform cancels (a gesture took it, the pen came), or one the panel calls palm-sized, is no tap and no press, and
/// a drag it was ends with DRAG_END marked cancelled.
#[derive(Clone, Debug, Default)]
pub struct KitPointer {
    contact: Contact,
    /// the hit region under the contact's start, -1 for none
    id: i32,
    /// 0 touch, 1 pen
    source: i32,
    /// where it came down, and its newest sample
    x0: i32,
    y0: i32,
    x: i32,
    y: i32,
    /// when it came down (the app's clock, `now_ms`)
    t0: u32,
    /// how far it may move from where it came down and still be a tap or a press, in pixels
    slop: i32,
    /// the samples gathered since the last DRAG_MOVE, at most KIT_POINTS_MAX
    points: std::collections::VecDeque<(i32, i32)>,
}

impl KitPointer {
    /// A contact came down at (x, y) on hit region `id` (-1 none), from `source` (0 touch, 1 pen), at `now`. A
    /// contact may move `slop` pixels each way and stay a tap or a press. A palm-sized one is cancelled at once. A
    /// contact that was still down (its lift was never seen) ends first.
    pub fn down(&mut self, x: i32, y: i32, source: i32, id: i32, now: u32, slop: i32, palm: bool) -> Vec<KitEvent> {
        let out = self.cancel();
        let points = std::mem::take(&mut self.points);
        *self = KitPointer { contact: if palm { Contact::Cancelled } else { Contact::Down }, id, source, x0: x, y0: y, x, y, t0: now, slop: slop.max(0), points };
        out
    }

    /// A sample of the contact at (x, y): DRAG_START when it first goes past the slop; while it drags, the sample
    /// is gathered. A palm-sized sample cancels the contact.
    pub fn moved(&mut self, x: i32, y: i32, palm: bool) -> Vec<KitEvent> {
        if matches!(self.contact, Contact::Up | Contact::Cancelled) {
            return Vec::new();
        }
        if palm {
            return self.cancel();
        }
        self.x = x;
        self.y = y;
        let mut out = Vec::new();
        if matches!(self.contact, Contact::Down | Contact::Pressed) && self.past_slop(x, y) {
            self.contact = Contact::Dragging;
            out.push(KitEvent::new(EV_DRAG_START, self.id, self.x0, self.y0, self.source));
        }
        if self.contact == Contact::Dragging {
            self.gather(x, y);
        }
        out
    }

    /// The contact lifted at (x, y), or was cancelled: a TAP, or a drag's last DRAG_MOVE and its DRAG_END, or
    /// nothing (a press's lift, a cancelled tap). A lift past the slop of a contact whose moves were not seen (they
    /// left the body) is a drag's: DRAG_START, then its end.
    pub fn up(&mut self, x: i32, y: i32, cancelled: bool) -> Vec<KitEvent> {
        let mut out = Vec::new();
        if !cancelled && matches!(self.contact, Contact::Down | Contact::Pressed) && self.past_slop(x, y) {
            out = self.moved(x, y, false);
        }
        out.extend(match self.contact {
            Contact::Down if !cancelled => vec![KitEvent::new(EV_TAP, self.id, self.x0, self.y0, self.source)],
            Contact::Dragging if !cancelled => {
                if self.points.back() != Some(&(x, y)) {
                    self.gather(x, y);
                }
                self.x = x;
                self.y = y;
                let mut out: Vec<KitEvent> = self.take_moves().into_iter().collect();
                out.push(KitEvent::new(EV_DRAG_END, self.id, x, y, 0));
                out
            }
            _ => self.cancel(),
        });
        self.contact = Contact::Up;
        self.points.clear();
        out
    }

    /// The host's tick at `now`: a contact held HOLD_MS within the slop is a PRESS, once. (The hold is a duration on
    /// the u32 clock, `now.wrapping_sub(t0)`: the same at any uptime, across the clock's wrap too.)
    pub fn tick(&mut self, now: u32) -> Option<KitEvent> {
        if self.contact != Contact::Down || now.wrapping_sub(self.t0) < HOLD_MS {
            return None;
        }
        self.contact = Contact::Pressed;
        Some(KitEvent::new(EV_PRESS, self.id, self.x0, self.y0, self.source))
    }

    /// The samples a drag gathered since the last DRAG_MOVE, as one DRAG_MOVE: at the newest, with how many there are,
    /// oldest first. None while there are none.
    pub fn take_moves(&mut self) -> Option<KitEvent> {
        if self.contact != Contact::Dragging || self.points.is_empty() {
            return None;
        }
        let points: Vec<(i32, i32)> = self.points.drain(..).collect();
        let &(x, y) = points.last()?;
        Some(KitEvent { points: points.clone(), ..KitEvent::new(EV_DRAG_MOVE, self.id, x, y, points.len() as i32) })
    }

    /// The contact is let go as cancelled: a drag it was ends so (DRAG_END marked cancelled, at its newest sample).
    fn cancel(&mut self) -> Vec<KitEvent> {
        let out = if self.contact == Contact::Dragging { vec![KitEvent::new(EV_DRAG_END, self.id, self.x, self.y, 1)] } else { Vec::new() };
        if self.contact != Contact::Up {
            self.contact = Contact::Cancelled;
        }
        self.points.clear();
        out
    }

    fn past_slop(&self, x: i32, y: i32) -> bool {
        (x as i64 - self.x0 as i64).abs() > self.slop as i64 || (y as i64 - self.y0 as i64).abs() > self.slop as i64
    }

    fn gather(&mut self, x: i32, y: i32) {
        if self.points.len() >= KIT_POINTS_MAX {
            self.points.pop_front();
        }
        self.points.push_back((x, y));
    }

    /// A contact is down (whatever it is recognised as).
    pub fn is_down(&self) -> bool {
        self.contact != Contact::Up
    }

    /// A contact is down and may still be a tap: within the slop, not yet held.
    pub fn undecided(&self) -> bool {
        self.contact == Contact::Down
    }

    /// The contact never went past the slop and was not cancelled: a tap, or a press (what a widget's own
    /// `widget_tap` is called for at the lift).
    pub fn still(&self) -> bool {
        matches!(self.contact, Contact::Down | Contact::Pressed)
    }

    /// The contact drags, and has samples a DRAG_MOVE would hand over.
    pub fn has_moves(&self) -> bool {
        self.contact == Contact::Dragging && !self.points.is_empty()
    }

    /// Where the contact came down.
    pub fn start(&self) -> (i32, i32) {
        (self.x0, self.y0)
    }
}

/// What input owes a body's drawing: the recogniser of its contact, whether a pass is owed, the pace while a contact
/// is down, and the part of the screen its passes refreshed fast. Every input event still calls the module at once;
/// the pass it owes waits for the host's tick, which draws it once for all the events since the last pass
/// (`pass_now`). A fast refresh shows grays dithered to black and white, and the lift's refresh drives only the pixels
/// that change: so all that the fast refreshes covered is owed one full-depth refresh once the contact has lifted
/// (`take_worn`).
#[derive(Clone, Debug, Default)]
pub struct KitInput {
    pub pointer: KitPointer,
    due: bool,
    /// when the pace last gave a turn (the app's clock); None before the first, which is due at once
    at: Option<u32>,
    /// the part of the screen this body's passes refreshed fast, since it was last taken
    worn: Rect,
}

impl KitInput {
    /// A call of the module for input changed its picture (a `kit_event` that returned nonzero, or a host's own
    /// export, which says nothing): a pass is owed.
    pub fn owe(&mut self) {
        self.due = true;
    }

    pub fn owed(&self) -> bool {
        self.due
    }

    /// A drawing pass was drawn, for whatever reason: it answers what input owed.
    pub fn answered(&mut self) {
        self.due = false;
    }

    /// The pace's turn at `now`: while a contact that can no longer be a tap is down (a press, a drag), once every
    /// KIT_PASS_MS, and never while the panel's last fast refresh is still showing (`fast_busy`). On its turn the
    /// host hands a drag's gathered samples to the module and draws the pass owed. Taking it starts the next wait.
    /// (The wait is a duration on the u32 clock, `now.wrapping_sub(at)`: the same at any uptime, across the clock's
    /// wrap too.)
    pub fn turn(&mut self, now: u32, fast_busy: bool) -> bool {
        if !self.pointer.is_down() || self.pointer.undecided() || fast_busy || self.at.is_some_and(|at| now.wrapping_sub(at) < KIT_PASS_MS) {
            return false;
        }
        self.at = Some(now);
        true
    }

    /// A pass input owed went out as a refresh of `r` in `mode`: a fast one is kept in the part the fast refreshes
    /// covered.
    pub fn refreshed(&mut self, r: Rect, mode: RefreshMode) {
        if mode == RefreshMode::Fast {
            self.worn = rect_union(self.worn, r);
        }
    }

    /// Once no contact is down: the part of the screen the passes refreshed fast, owed one full-depth refresh whatever
    /// the lift's pass changed, taken once. None while a contact is down, or when nothing was refreshed fast.
    pub fn take_worn(&mut self) -> Option<Rect> {
        if self.pointer.is_down() || rect_is_empty(self.worn) {
            return None;
        }
        Some(std::mem::replace(&mut self.worn, RECT_EMPTY))
    }

    /// Whether the pass input owes is drawn now (`turn`: this tick took the pace's turn), and the refresh it takes:
    /// none while the panel's last fast refresh is still showing; while a contact is down, on the pace's turn alone,
    /// refreshed fast (none while it may still be a tap: one pass for a tap, at its lift); with no contact down, at
    /// once, refreshed as the screen's UI is.
    pub fn pass_now(&self, turn: bool, fast_busy: bool) -> Option<RefreshMode> {
        if !self.due || fast_busy {
            return None;
        }
        if !self.pointer.is_down() {
            return Some(RefreshMode::Ui);
        }
        turn.then_some(RefreshMode::Fast)
    }
}

/// A copy of `at` of the screen (cut to it), to see what a pass changes there (canvas.rs `canvas_changed_rect` of the
/// copy, the screen and the part returned). None when there is no memory for it (or nothing of `at` is on the screen).
pub fn kit_keep(screen: &Canvas, at: Rect) -> Option<(Canvas, Rect)> {
    let at = rect_intersect(at, Rect { x: 0, y: 0, w: screen.w, h: screen.h });
    if rect_is_empty(at) {
        return None;
    }
    let mut kept = Canvas::default();
    if canvas_init(&mut kept, at.w, at.h) != 0 {
        return None;
    }
    canvas_blit(&mut kept, screen, at, 0, 0);
    Some((kept, at))
}

// ---- the work of each call ----
//
// Each pays the fixed charge first, whatever it is given. A drawing call then does nothing more (and returns 0)
// for a package that may not draw; it needs room for one more command (and for a hit region, when it is given an
// id of 0 or more); its texts' lengths are compared with the host's limits before they are read; it pays for its
// work; and it records its command.

type Res<R> = Result<R, wasmi::Error>;

/// The fixed charge, and whether the package may draw: what a drawing call starts with.
fn drawing(cx: &mut dyn Call) -> Res<bool> {
    cx.charge(KIT_CHARGE_CALL)?;
    if !cx.host().kit_may_draw() {
        return Ok(false);
    }
    cx.host().kit_room()?;
    Ok(true)
}

/// Room for the hit region of an element given an id of 0 or more.
fn hit_room(cx: &mut dyn Call, id: i32) -> Res<()> {
    if id >= 0 {
        cx.host().kit_hit_room()?;
    }
    Ok(())
}

/// A text of the call, read once its length is within the host's limits and the call has paid `cost` of it.
fn text_of(cx: &mut dyn Call, ptr: i32, len: i32, cost: impl FnOnce(usize) -> u64) -> Res<String> {
    let n = len.max(0) as usize;
    cx.host().kit_text(n)?;
    cx.charge(cost(n))?;
    Ok(String::from_utf8_lossy(&cx.read(ptr, len)).into_owned())
}

/// An icon's name, read when it is no longer than a name can be (APP_ICON_NAME_MAX), for 4 a byte. A longer one
/// is not read, and is drawn as a name of no icon is (the grid).
fn icon_name(cx: &mut dyn Call, ptr: i32, len: i32) -> Res<String> {
    if len.max(0) as usize > APP_ICON_NAME_MAX {
        return Ok("?".to_string());
    }
    cx.charge(len.max(0) as u64 * APP_CHARGE_BYTE)?;
    Ok(String::from_utf8_lossy(&cx.read(ptr, len)).into_owned())
}

/// A width given as 0 or less: the rest of the body from `x`.
fn rest_w(cx: &mut dyn Call, x: i32, w: i32) -> i32 {
    if w > 0 {
        return w;
    }
    (cx.host().kit_body().0 as i64 - x as i64).clamp(0, i32::MAX as i64) as i32
}

/// A size given as 0 or less: `standard`.
fn or(v: i32, standard: i32) -> i32 {
    if v > 0 {
        v
    } else {
        standard
    }
}

/// Records an element's command, and its hit region when it has an id of 0 or more.
fn push(cx: &mut dyn Call, cmd: KitCmd, id: i32, r: Rect) {
    cx.host().kit_push(Some(cmd), (id >= 0).then_some((id, r)));
}

fn level(cx: &mut dyn Call, []: [i32; 0]) -> Res<i32> {
    cx.charge(KIT_CHARGE_CALL)?;
    Ok(KIT_LEVEL)
}

fn version(cx: &mut dyn Call, [ptr, cap]: [i32; 2]) -> Res<i32> {
    cx.charge(KIT_CHARGE_CALL)?;
    let v = &GRIMOIRE_VERSION.as_bytes()[..GRIMOIRE_VERSION.len().min(KIT_VERSION_MAX)];
    cx.charge(v.len().min(cap.max(0) as usize) as u64 * APP_CHARGE_BYTE)?;
    Ok(cx.write(ptr, cap, v))
}

fn metric(cx: &mut dyn Call, [id]: [i32; 1]) -> Res<i32> {
    cx.charge(KIT_CHARGE_CALL)?;
    let host = cx.host();
    Ok(kit_metric(id, host.kit_body(), host.kit_status_bar_h()))
}

fn clock_ms(cx: &mut dyn Call, []: [i32; 0]) -> Res<i64> {
    cx.charge(KIT_CHARGE_CALL)?;
    Ok(now_ms() as i64)
}

fn measure_width(cx: &mut dyn Call, [ptr, len, role]: [i32; 3]) -> Res<i32> {
    cx.charge(KIT_CHARGE_CALL)?;
    let text = text_of(cx, ptr, len, |n| n as u64 * APP_CHARGE_MEASURE)?;
    let (f, px) = role_face(role);
    Ok(font_measure(f, px, &text))
}

fn measure_height(cx: &mut dyn Call, [ptr, len, role, w]: [i32; 4]) -> Res<i32> {
    cx.charge(KIT_CHARGE_CALL)?;
    let w = rest_w(cx, 0, w);
    let text = text_of(cx, ptr, len, |n| n as u64 * KIT_CHARGE_MEASURE_LINES)?;
    let (f, px) = role_face(role);
    Ok(kit_lines(f, px, &text, w).len() as i32 * line_h(f, px))
}

fn text(cx: &mut dyn Call, [x, y, w, h, ptr, len, role, flags]: [i32; 8]) -> Res<i32> {
    if !drawing(cx)? {
        return Ok(0);
    }
    let (f, px) = role_face(role);
    let w = rest_w(cx, x, w);
    // (its letters; and the steps that break it into lines, fit it on two, or cut it short, each measuring it)
    let wrap = flags & FLAG_WRAP != 0;
    let fit = !wrap && flags & FLAG_FIT != 0;
    let text = text_of(cx, ptr, len, |n| {
        drawn_text_charge(n, px, f) + if wrap { n as u64 * KIT_CHARGE_WRAP } else if fit { fit_text_charge(n) } else { 0 }
    })?;
    if wrap {
        let lines = kit_lines(f, px, &text, w);
        let h = or(h, lines.len() as i32 * line_h(f, px));
        cx.host().kit_push(Some(KitCmd::Lines(Rect::new(x, y, w, h), lines, role, flags, true)), None);
        return Ok(h);
    }
    if !fit && font_measure(f, px, &text) > w {
        cx.charge(cut_text_charge(text.len()))?;
    }
    let standard = if fit && fit_px(f, px, &text, w).is_none() { 2 * line_h(f, px.min(ui_small_size())) } else { line_h(f, px) };
    let h = or(h, standard);
    cx.host().kit_push(Some(KitCmd::Text(Rect::new(x, y, w, h), text, role, flags)), None);
    Ok(h)
}

fn text_lines(cx: &mut dyn Call, [x, y, w, h, ptr, len, role, flags, first]: [i32; 9]) -> Res<i32> {
    if !drawing(cx)? {
        return Ok(0);
    }
    let (f, px) = role_face(role);
    let w = rest_w(cx, x, w);
    let h = if h > 0 { h } else { (cx.host().kit_body().1 as i64 - y as i64).clamp(0, i32::MAX as i64) as i32 };
    let text = text_of(cx, ptr, len, |n| drawn_text_charge(n, px, f) + n as u64 * KIT_CHARGE_WRAP)?;
    let lines = kit_lines(f, px, &text, w);
    let n = lines.len();
    let from = (first.max(0) as usize).min(n);
    let shown: Vec<String> = lines.into_iter().skip(from).take((h / line_h(f, px)).max(0) as usize).collect();
    cx.host().kit_push(Some(KitCmd::Lines(Rect::new(x, y, w, h), shown, role, flags & !FLAG_WRAP, false)), None);
    Ok(n as i32)
}

/// The width a button `h` high needs for its label and icon as the kit draws it, at least `h` (a square).
pub fn button_need_w(h: i32, icon: bool, label: &str) -> i32 {
    let need = if gui_gothic() && icon && !label.is_empty() {
        gui_button_row_need_w(h, label)
    } else if gui_gothic() {
        ui_button_need_w(h, if icon { IconKind::Grid } else { IconKind::None }, label, ButtonStyle::Plain)
    } else if label.is_empty() {
        0
    } else {
        // (gui_button's pill: its label in the bold face, its icon beside it, padded at both ends)
        let inner_h = h - 2 * ui_px(8.0);
        let ts = ui_text_size().min(inner_h as f32 * 0.46);
        let isz = if icon { inner_h.min(ui_px(70.0)).max(ui_px(30.0)) + ui_px(18.0) } else { 0 };
        font_measure(FontFace::Bold, ts, label) + isz + 2 * ui_px(22.0)
    };
    need.max(h)
}

fn button(cx: &mut dyn Call, [id, x, y, w, h, lp, ll, ip, il, style]: [i32; 10]) -> Res<i32> {
    if !drawing(cx)? {
        return Ok(0);
    }
    hit_room(cx, id)?;
    let label = text_of(cx, lp, ll, |n| label_charge(n, true))?;
    let icon = icon_name(cx, ip, il)?;
    let h = or(h, gui_button_h());
    let w = if w > 0 { w } else { button_need_w(h, !icon.is_empty(), &label) };
    let r = Rect::new(x, y, w, h);
    // (its fill, its outline and its inner line; and its icon as large as its smaller side)
    let body = cx.host().kit_body();
    cx.charge(3 * shape_charge(body, r, APP_CHARGE_CORNER) + if icon.is_empty() { 0 } else { icon_in(body, r, 1) })?;
    push(cx, KitCmd::Button(r, label, icon, style), id, r);
    Ok(w)
}

fn icon_button(cx: &mut dyn Call, [id, x, y, w, h, ip, il, style]: [i32; 8]) -> Res<i32> {
    if !drawing(cx)? {
        return Ok(0);
    }
    hit_room(cx, id)?;
    let icon = icon_name(cx, ip, il)?;
    let h = or(h, gui_top_button_h());
    let w = or(w, h);
    let r = Rect::new(x, y, w, h);
    let body = cx.host().kit_body();
    cx.charge(3 * shape_charge(body, r, APP_CHARGE_CORNER) + icon_in(body, r, 1))?;
    push(cx, KitCmd::IconButton(r, icon, style), id, r);
    Ok(w)
}

fn row(cx: &mut dyn Call, [id, x, y, w, h, ip, il, lp, ll, style, switch]: [i32; 11]) -> Res<i32> {
    if !drawing(cx)? {
        return Ok(0);
    }
    hit_room(cx, id)?;
    let icon = icon_name(cx, ip, il)?;
    let label = text_of(cx, lp, ll, |n| label_charge(n, true))?;
    let (w, h) = (rest_w(cx, x, w), or(h, gui_row_h()));
    let r = Rect::new(x, y, w, h);
    // (its card; its icon and its switch are of one size whatever the row's)
    let small = if icon.is_empty() { 0 } else { icon_charge(APP_CHARGE_SMALL_ICON_PX) } + if switch >= 0 { APP_CHARGE_SWITCH } else { 0 };
    let body = cx.host().kit_body();
    cx.charge(shape_charge(body, r, APP_CHARGE_CORNER) + small)?;
    push(cx, KitCmd::Row(r, icon, label, style, switch), id, r);
    Ok(h)
}

fn chip(cx: &mut dyn Call, [id, x, y, w, h, lp, ll, selected]: [i32; 8]) -> Res<i32> {
    if !drawing(cx)? {
        return Ok(0);
    }
    hit_room(cx, id)?;
    let label = text_of(cx, lp, ll, |n| label_charge(n, false))?;
    let (w, h) = (or(w, gui_chip_w(&label)), or(h, gui_chip_h()));
    let r = Rect::new(x, y, w, h);
    let body = cx.host().kit_body();
    cx.charge(2 * shape_charge(body, r, round_end(body, r)))?;
    push(cx, KitCmd::Chip(r, label, selected != 0), id, r);
    Ok(w)
}

fn field(cx: &mut dyn Call, [id, x, y, w, h, ip, il, tp, tl, hp, hl, flags]: [i32; 12]) -> Res<i32> {
    if !drawing(cx)? {
        return Ok(0);
    }
    hit_room(cx, id)?;
    let icon = icon_name(cx, ip, il)?;
    let text = text_of(cx, tp, tl, |n| label_charge(n, false))?;
    let hint = text_of(cx, hp, hl, |n| label_charge(n, false))?;
    let (w, h) = (rest_w(cx, x, w), or(h, gui_field_h()));
    let r = Rect::new(x, y, w, h);
    // (its icon is as tall as the field, and never smaller than the kit's small icons; the mark that clears a
    // field with text in it is of one size)
    let body = cx.host().kit_body();
    let tall = if icon.is_empty() { 0 } else { icon_charge((drawn(body, r).h as i64).max(APP_CHARGE_SMALL_ICON_PX)) };
    cx.charge(2 * shape_charge(body, r, round_end(body, r)) + tall + icon_charge(APP_CHARGE_SMALL_ICON_PX))?;
    push(cx, KitCmd::Field(r, icon, text, hint, flags & 1 != 0), id, r);
    Ok(h)
}

fn select(cx: &mut dyn Call, [id, x, y, w, h, cp, cl, vp, vl]: [i32; 9]) -> Res<i32> {
    if !drawing(cx)? {
        return Ok(0);
    }
    hit_room(cx, id)?;
    let caption = text_of(cx, cp, cl, |n| label_charge(n, false))?;
    let value = text_of(cx, vp, vl, |n| label_charge(n, false))?;
    let (w, h) = (rest_w(cx, x, w), or(h, gui_select_h()));
    let r = Rect::new(x, y, w, h);
    // (and its chevron, of one size)
    let body = cx.host().kit_body();
    cx.charge(2 * shape_charge(body, r, round_end(body, r)) + icon_charge(APP_CHARGE_SMALL_ICON_PX))?;
    push(cx, KitCmd::Select(r, caption, value), id, r);
    Ok(h)
}

fn checkbox(cx: &mut dyn Call, [id, x, y, w, h, lp, ll, checked]: [i32; 8]) -> Res<i32> {
    if !drawing(cx)? {
        return Ok(0);
    }
    hit_room(cx, id)?;
    let label = text_of(cx, lp, ll, |n| label_charge(n, true))?;
    let (w, h) = (rest_w(cx, x, w), or(h, gui_row_min_h()));
    let r = Rect::new(x, y, w, h);
    // (its rectangle is filled white first; its box is of one size, filled and outlined, with its mark)
    let body = cx.host().kit_body();
    let side = ui_px(64.0);
    let boxed = Rect::new(0, 0, side, side);
    cx.charge(shape_charge(body, r, 0) + 2 * shape_charge((side, side), boxed, side as i64) + icon_charge(side as i64))?;
    push(cx, KitCmd::Checkbox(r, label, checked != 0), id, r);
    Ok(h)
}

fn card(cx: &mut dyn Call, [x, y, w, h, style]: [i32; 5]) -> Res<()> {
    if !drawing(cx)? {
        return Ok(());
    }
    let r = Rect::new(x, y, rest_w(cx, x, w), or(h, gui_row_h()));
    let body = cx.host().kit_body();
    cx.charge(shape_charge(body, r, APP_CHARGE_CORNER))?;
    cx.host().kit_push(Some(KitCmd::Card(r, style)), None);
    Ok(())
}

fn divider(cx: &mut dyn Call, [x, y, w]: [i32; 3]) -> Res<()> {
    if !drawing(cx)? {
        return Ok(());
    }
    // (the theme's rule: in Gothic a thick line over a thin one, the room of both)
    let r = Rect::new(x, y, rest_w(cx, x, w), ui_line() + ui_px(7.0));
    let body = cx.host().kit_body();
    cx.charge(shape_charge(body, r, 0))?;
    cx.host().kit_push(Some(KitCmd::Divider(r)), None);
    Ok(())
}

fn icon(cx: &mut dyn Call, [x, y, w, h, np, nl, tone]: [i32; 7]) -> Res<()> {
    if !drawing(cx)? {
        return Ok(());
    }
    let name = icon_name(cx, np, nl)?;
    let r = Rect::new(x, y, or(w, ui_row_icon()), or(h, ui_row_icon()));
    let body = cx.host().kit_body();
    cx.charge(icon_in(body, r, 1))?;
    cx.host().kit_push(Some(KitCmd::Icon(r, name, tone_gray(tone))), None);
    Ok(())
}

fn fill(cx: &mut dyn Call, [x, y, w, h, gray]: [i32; 5]) -> Res<()> {
    if !drawing(cx)? {
        return Ok(());
    }
    let r = Rect::new(x, y, rest_w(cx, x, w), h.max(0));
    let body = cx.host().kit_body();
    cx.charge(shape_charge(body, r, 0))?;
    cx.host().kit_push(Some(KitCmd::Fill(r, gray.clamp(0, 255) as u8)), None);
    Ok(())
}

fn frame(cx: &mut dyn Call, [x, y, w, h, gray, filled, t]: [i32; 7]) -> Res<()> {
    if !drawing(cx)? {
        return Ok(());
    }
    let r = Rect::new(x, y, rest_w(cx, x, w), h.max(0));
    let t = if t > 0 { t.min(KIT_THICKNESS_MAX) } else { ui_line() };
    // (the pixels within its corners' radius, or its outline's thickness, of its sides are worked out one by one)
    let body = cx.host().kit_body();
    let d = drawn(body, r);
    let edge = ui_card_radius().clamp(0, (d.w.min(d.h) / 2).max(0)).max(if filled != 0 { 0 } else { t }) as i64 + 2;
    cx.charge(shape_charge(body, r, edge))?;
    cx.host().kit_push(Some(KitCmd::Frame(r, gray.clamp(0, 255) as u8, filled != 0, t)), None);
    Ok(())
}

fn line(cx: &mut dyn Call, [x0, y0, x1, y1, width]: [f32; 5], gray: i32) -> Res<()> {
    if !drawing(cx)? {
        return Ok(());
    }
    // (a width that is not a number is the least)
    #[allow(clippy::manual_clamp)]
    let width = width.max(0.1).min(APP_LINE_WIDTH_MAX);
    let body = cx.host().kit_body();
    cx.charge(line_charge(body, x0, y0, x1, y1, width))?;
    cx.host().kit_push(Some(KitCmd::Line(x0, y0, x1, y1, width, gray.clamp(0, 255) as u8)), None);
    Ok(())
}

fn clip(cx: &mut dyn Call, [x, y, w, h]: [i32; 4]) -> Res<()> {
    if drawing(cx)? {
        cx.host().kit_push(Some(KitCmd::Clip(Rect::new(x, y, w, h))), None);
    }
    Ok(())
}

fn clip_clear(cx: &mut dyn Call, []: [i32; 0]) -> Res<()> {
    if drawing(cx)? {
        cx.host().kit_push(Some(KitCmd::ClipClear), None);
    }
    Ok(())
}

fn hit(cx: &mut dyn Call, [id, x, y, w, h]: [i32; 5]) -> Res<()> {
    cx.charge(KIT_CHARGE_CALL)?;
    if id >= 0 && cx.host().kit_may_draw() {
        cx.host().kit_hit_room()?;
        cx.host().kit_push(None, Some((id, Rect::new(x, y, w, h))));
    }
    Ok(())
}

/// `ui_row_page`'s answer for a list of `n` rows in an area `area_h` high: the rows a page holds, the rows'
/// height, the gap between them and the pages, each in 16 bits, the first lowest.
pub fn kit_list_page(n: i32, area_h: i32, pref_h: i32, min_h: i32) -> i64 {
    let n = n.clamp(0, KIT_LIST_ROWS_MAX) as usize;
    let (area_h, pref_h, min_h) = (area_h.clamp(0, KIT_LIST_PX_MAX), pref_h.clamp(1, KIT_LIST_PX_MAX), min_h.clamp(0, KIT_LIST_PX_MAX));
    let (rh, gap, per, pages) = ui_row_page_fit(area_h, n, pref_h, min_h);
    let field = |v: i64| v.clamp(0, 0xffff);
    field(per as i64) | field(rh as i64) << 16 | field(gap as i64) << 32 | field(pages as i64) << 48
}

fn list_page(cx: &mut dyn Call, [n, _page, area_h, pref_h, min_h]: [i32; 5]) -> Res<i64> {
    cx.charge(KIT_CHARGE_CALL)?;
    Ok(kit_list_page(n, area_h, pref_h, min_h))
}

fn pager(cx: &mut dyn Call, [prev_id, next_id, x, y, w, page, pages]: [i32; 7]) -> Res<i32> {
    if !drawing(cx)? {
        return Ok(0);
    }
    let pages = pages.clamp(0, KIT_LIST_ROWS_MAX);
    if pages <= 1 {
        return Ok(0);
    }
    let page = page.clamp(0, pages - 1);
    let r = Rect::new(x, y, rest_w(cx, x, w), ui_row_pager_button().1);
    let (prev, next) = pager_arrows(r);
    // (two buttons, each three frames and its icon, and the count between them)
    let body = cx.host().kit_body();
    let arrow = |b: Rect| 3 * shape_charge(body, b, APP_CHARGE_CORNER) + icon_in(body, b, 1);
    let count = format!("{}/{}", page + 1, pages);
    cx.charge(arrow(prev) + arrow(next) + drawn_text_charge(count.len(), ui_small_size(), FontFace::Fixed))?;
    let prev_hit = (page > 0 && prev_id >= 0).then_some((prev_id, prev));
    let next_hit = (page + 1 < pages && next_id >= 0).then_some((next_id, next));
    if prev_hit.is_some() {
        cx.host().kit_hit_room()?;
    }
    cx.host().kit_push(Some(KitCmd::Pager(r, page, pages)), prev_hit);
    if next_hit.is_some() {
        cx.host().kit_hit_room()?;
        cx.host().kit_push(None, next_hit);
    }
    Ok(r.h)
}

/// The points of the DRAG_MOVE being handled, as (x, y) pairs of i32 (little-endian, as the module's memory holds
/// numbers), oldest first: as many whole points as `cap` bytes hold, the event has (at most KIT_POINTS_MAX) and the
/// memory takes; 0 in a call that is not a DRAG_MOVE's. Returns how many it wrote whole.
fn event_points(cx: &mut dyn Call, [ptr, cap]: [i32; 2]) -> Res<i32> {
    cx.charge(KIT_CHARGE_CALL)?;
    let n = cx.host().kit_points().len().min(cap.max(0) as usize / KIT_POINT_BYTES);
    if n == 0 {
        return Ok(0);
    }
    cx.charge((n * KIT_POINT_BYTES) as u64 / APP_CHARGE_COPY_BYTES)?;
    let mut bytes = Vec::with_capacity(n * KIT_POINT_BYTES);
    for &(x, y) in &cx.host().kit_points()[..n] {
        bytes.extend_from_slice(&x.to_le_bytes());
        bytes.extend_from_slice(&y.to_le_bytes());
    }
    Ok(cx.write(ptr, bytes.len() as i32, &bytes) / KIT_POINT_BYTES as i32)
}

/// The character a key types with these modifiers, as a hardware keyboard types into Grimoire's own keyboards
/// (`keyboard_hw_char`: the US layout with Shift), or 0 for a key that types none.
fn key_char(cx: &mut dyn Call, [code, mods]: [i32; 2]) -> Res<i32> {
    cx.charge(KIT_CHARGE_CALL)?;
    Ok(crate::keyboard::keyboard_hw_char(code, mods as u32).map_or(0, |c| c as i32))
}
