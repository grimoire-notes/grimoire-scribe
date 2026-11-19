//! Looks: how the app is drawn. Each theme is one `Look` in its own file here, listed once in LOOKS;
//! the screens and ui.rs never name a theme, they ask `look()`.
//!
//! The trait's own bodies are Classic's (classic.rs holds the longer ones), so a theme writes only what
//! it draws differently. There are two kinds of method:
//! * a measure or a choice (a size, a face, a margin): the shared drawing asks it, so a theme that differs
//!   in a number changes the number, not the drawing (Gothic, mostly);
//! * a piece of drawing (a frame, a button, a sheet, a screen's corner): a theme may draw it entirely its
//!   own way. A screen's hook that returns `false` or `None` leaves the screen drawing its own default.
//!
//! A new theme: a file here with a `Look`, its line in LOOKS, its `mod` line, and its Style label in the
//! translations (po/). Its fonts, if it brings any, are listed in font.rs (TTF_*) and assets.rs.

use crate::font::FaceMap;
use crate::prelude::*;

pub mod classic;
mod gothic;

/// Every theme, in the order Settings' Style row and the theme shortcut step through them. Gothic first
/// (the default and Settings.theme 0), Classic second (Settings.theme 1): settings.ini names them by id.
pub static LOOKS: &[&dyn Look] = &[
    &gothic::Gothic,
    &classic::Classic,
];

/// A theme: an index of LOOKS (Settings.theme's number), settings.ini's `theme=` by its id.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct UiTheme(u8);

#[allow(non_upper_case_globals)] // (named as the enum's variants were: the tests and screens use them)
impl UiTheme {
    pub const Gothic: UiTheme = UiTheme(0);
    pub const Classic: UiTheme = UiTheme(1);

    /// Every theme, in LOOKS' order.
    pub fn all() -> impl Iterator<Item = UiTheme> {
        (0..LOOKS.len() as u8).map(UiTheme)
    }
    pub fn look(self) -> &'static dyn Look {
        LOOKS[self.0 as usize]
    }
    /// settings.ini's theme= value
    pub fn id(self) -> &'static str {
        self.look().id()
    }
    /// Settings.theme's number
    pub fn index(self) -> i32 {
        self.0 as i32
    }
    /// Settings.theme's number back (any other: Gothic, the default)
    pub fn from_i32(v: i32) -> UiTheme {
        if v >= 0 && (v as usize) < LOOKS.len() { UiTheme(v as u8) } else { UiTheme::Gothic }
    }
    pub fn from_id(id: &[u8]) -> Option<UiTheme> {
        UiTheme::all().find(|t| t.id().as_bytes() == id)
    }
    /// the theme after this one (Settings' Style row, the theme shortcut)
    pub fn next(self) -> UiTheme {
        UiTheme((self.0 + 1) % LOOKS.len() as u8)
    }
}

/// The theme in use.
pub fn look() -> &'static dyn Look {
    ui_theme().look()
}

/// What the Library's shelf holds, for a theme's readout under its header (Look::library_readout).
pub struct ShelfInfo<'a> {
    /// the screen's side margin, its width, and the header's height (where its rule is)
    pub margin: i32,
    pub w: i32,
    pub header_h: i32,
    pub notebooks: usize,
    pub folders: usize,
    /// the folder open, empty for the Library's top
    pub folder: &'a str,
}

/// The Library as a theme frames it on a whole redraw (Look::library_overlay).
pub struct ScreenInfo {
    pub w: i32,
    pub h: i32,
    pub dpi: f32,
    /// the entries of the shelf shown
    pub items: usize,
}

/// New notebook's spacing: its heading over the papers, the gap between tiles, the room under the
/// tiles for the name and the buttons, and whether the rows spread over the room the tiles leave.
#[derive(Clone, Copy, Debug)]
pub struct NewSheetSpacing {
    pub head: i32,
    pub gap: i32,
    pub foot: i32,
    pub spread: bool,
}

/// Where a theme's own foot of New notebook put its controls (Look::new_notebook_foot): the screen
/// adds their hits.
pub struct NewFoot {
    pub rename: Rect,
    pub cancel: Rect,
    pub more: Rect,
    pub file: Rect,
    pub create: Rect,
}

pub trait Look: Sync {
    // ---- what the theme is ----

    /// settings.ini's `theme=` value
    fn id(&self) -> &'static str;
    /// Settings' Style row with this theme on, untranslated (marked with n_ where it is written)
    fn style_label(&self) -> &'static str;
    /// The theme's name in Settings' Style list, untranslated (marked with n_)
    fn name(&self) -> &'static str;
    /// The theme's own faces, whatever font Settings has (None: Classic's family, or the owner's own
    /// font, and Settings' Font row with them).
    fn faces(&self) -> Option<[FaceMap; FontFace::COUNT]> {
        None
    }
    /// Whether the share pages take Gothic's style (else Classic's).
    fn share_pages_gothic(&self) -> bool {
        false
    }

    // ---- measures and choices the shared drawing asks ----

    /// The face of a notebook's or folder's name on its card (and a selected template's).
    fn name_face(&self) -> FontFace {
        FontFace::Bold
    }
    /// The face of a heading drawn left in a panel (the shapes panel's title).
    fn heading_face(&self) -> FontFace {
        FontFace::Bold
    }
    /// The Library's name in its header, times the title size.
    fn mark_scale(&self) -> f32 {
        1.25
    }
    /// The air the Library's grid keeps under its header besides the margin (Gothic's frieze hangs there).
    fn header_drop(&self) -> i32 {
        0
    }
    /// Whether a button w by h px has the inner line of a carved panel.
    fn carved(&self, _w: i32, _h: i32) -> bool {
        false
    }
    /// How far a button's label under its icon is raised.
    fn label_lift(&self) -> i32 {
        0
    }
    /// Whether a control's label is printed in capitals.
    fn caps_labels(&self) -> bool {
        false
    }
    /// The corners of a button and of a Settings card (Classic's modern rounded design, 2.3, issue #209).
    fn corner_radius(&self) -> i32 {
        ui_px(22.0)
    }
    /// A list row's icon.
    fn row_icon_px(&self) -> i32 {
        ui_px(60.0)
    }
    /// The face a list row's label is drawn in, in style st.
    fn row_face(&self, font: FontFace, _st: ButtonStyle) -> FontFace {
        font
    }
    /// A menu row's icon, for a row rh px tall.
    fn menu_icon_side(&self, rh: i32) -> i32 {
        rh - ui_px(30.0)
    }
    fn slider_knob_radius(&self) -> i32 {
        ui_px(26.0)
    }
    /// A sheet's title band, its inset under a title, and its inset without one.
    fn sheet_metrics(&self) -> (i32, i32, i32) {
        (ui_px(120.0), ui_px(36.0), ui_px(36.0))
    }
    /// What a sheet leaves of the screen at its sides.
    fn sheet_margin(&self) -> i32 {
        ui_px(70.0)
    }
    /// How far a toast's text keeps from its edge.
    fn toast_inset(&self) -> i32 {
        ui_px(30.0)
    }
    /// The room a chip h px tall keeps on each side of its text.
    fn chip_pad(&self, _h: i32) -> i32 {
        ui_px(24.0)
    }
    fn new_sheet_spacing(&self) -> NewSheetSpacing {
        NewSheetSpacing { head: ui_px(80.0), gap: ui_px(30.0), foot: ui_px(420.0), spread: false }
    }
    /// Whether the editor's sheets stand over the page blurred behind them (the screen is then composed
    /// whole while one is up, with ui_with_backdrop).
    fn blurs_behind_sheets(&self) -> bool {
        false
    }

    // ---- the widgets (ui.rs's ui_* call these) ----

    /// A box in the theme's shape, the arguments of canvas_round_rect.
    fn frame(&self, c: &mut Canvas, r: Rect, radius: i32, gray: u8, filled: bool, t: i32) {
        canvas_round_rect(c, r, radius, gray, filled, t);
    }
    /// An ornament h px deep hanging from a header's rule at y.
    fn frieze(&self, _c: &mut Canvas, _x0: i32, _x1: i32, _y: i32, _h: i32, _gray: u8) {}
    /// A horizontal rule t px thick.
    fn rule(&self, c: &mut Canvas, x0: i32, x1: i32, y: i32, t: i32, gray: u8) {
        canvas_hline(c, x0, x1, y, t, gray);
    }
    /// The icon k (its glyph cp) fitted to r.
    fn icon(&self, c: &mut Canvas, _k: IconKind, cp: u32, r: Rect, gray: u8) {
        let inset = r.w.min(r.h) / 10;
        font_draw_icon(c, FontFace::Regular, cp, rect_inflate(r, -inset), gray);
    }
    /// A sign drawn by `draw` in r (in gray), where it stands as an icon would.
    fn sign(&self, c: &mut Canvas, r: Rect, draw: &dyn Fn(&mut Canvas, Rect, u8)) {
        draw(c, r, 0);
    }
    /// A small round mark (About's lists), k times its size.
    fn bullet(&self, c: &mut Canvas, cx: f32, cy: f32, k: f32) {
        canvas_disc_aa(c, cx, cy, ui_px(8.0 * k) as f32, 0, 1.0);
    }
    /// The gray of a control's text in style st.
    fn style_fg(&self, st: ButtonStyle) -> u8 {
        match st {
            ButtonStyle::Selected => 255,
            ButtonStyle::Disabled => crate::globalui::GUI_GRAY_DISABLED,
            _ => 0,
        }
    }
    /// A toolbar's or a floating panel's frame.
    fn panel_frame(&self, c: &mut Canvas, p: Rect, radius: i32) {
        self.frame(c, p, radius, 255, true, 0);
        // (the floating clock's outline: the toolbar and the clock match, issue #194)
        self.frame(c, p, radius, 0, false, ui_px(2.0));
    }
    /// A divider across a toolbar at y.
    fn panel_divider(&self, c: &mut Canvas, x: i32, w: i32, y: i32, gray: u8) {
        canvas_hline(c, x, x + w, y, ui_px(3.0), gray);
    }
    /// The line under the editor's panel's header (its grip), at y.
    fn panel_head_rule(&self, c: &mut Canvas, p: Rect, y: i32) {
        canvas_hline(c, p.x + ui_px(6.0), p.x + p.w - ui_px(6.0), y - ui_px(4.0), ui_px(3.0), 0);
    }
    /// A count the panel shows (its pages: "1 / 4"), a chip that opens them.
    fn counter_chip(&self, c: &mut Canvas, r: Rect, px: f32, text: &str) {
        self.frame(c, r, ui_px(10.0), 0, false, ui_px(2.0));
        font_draw_centered(c, FontFace::Bold, px, r, text, 0);
    }
    /// A button: its icon over its label, either alone; a label alone at text_px when that is not 0.
    fn button(&self, c: &mut Canvas, r: Rect, icon: IconKind, label: &str, st: ButtonStyle, text_px: f32) {
        classic::button(self, c, r, icon, label, st, text_px);
    }
    /// A button that is its word alone where the theme has such (a row of sheet actions).
    fn button_bare(&self, c: &mut Canvas, r: Rect, label: &str, st: ButtonStyle) {
        self.button(c, r, IconKind::None, label, st, 0.0);
    }
    /// The edge and the inside of a hollow icon on a button in style st.
    fn hollow_colors(&self, st: ButtonStyle) -> (u8, u8) {
        let bg: u8 = if st == ButtonStyle::Selected { 0 } else { 255 };
        (self.style_fg(st), bg)
    }
    /// A Settings card: the shape a row, a choice or a control sits in.
    fn card(&self, c: &mut Canvas, r: Rect, st: ButtonStyle) {
        match st {
            ButtonStyle::Selected => self.frame(c, r, ui_card_radius(), 0, true, 0),
            ButtonStyle::Disabled => self.frame(c, r, ui_card_radius(), crate::globalui::GUI_GRAY_DISABLED, false, ui_line()),
            _ => self.frame(c, r, ui_card_radius(), 0, false, ui_line()),
        }
    }
    /// Settings' on/off switch in its track (sw_h tall).
    fn switch(&self, c: &mut Canvas, track: Rect, sw_h: i32, on: bool) {
        self.frame(c, track, sw_h / 2, 0, on, ui_px(3.0));
        let kw = sw_h * 46 / 62;
        let inset = sw_h * 8 / 62;
        let kx = if on { track.x + track.w - kw - inset } else { track.x + inset };
        self.frame(c, Rect::new(kx, track.y + (track.h - kw) / 2, kw, kw), kw / 2, if on { 255 } else { 0 }, true, 0);
    }
    /// A slider's track and knob: the knob at fx, cy the track's middle.
    fn slider_track(&self, c: &mut Canvas, tr: Rect, fx: i32, cy: f32) {
        canvas_round_rect(c, tr, tr.h / 2, 190, true, 0);
        canvas_round_rect(c, Rect { x: tr.x, y: tr.y, w: fx - tr.x + tr.h / 2, h: tr.h }, tr.h / 2, 0, true, 0);
        canvas_disc_aa(c, fx as f32, cy, self.slider_knob_radius() as f32, 0, 1.0);
        canvas_disc_aa(c, fx as f32, cy, ui_px(12.0) as f32, 255, 1.0);
    }
    /// A modal sheet's card and its title; returns the content rect (ui_sheet_inner's).
    fn sheet(&self, c: &mut Canvas, card: Rect, title: &str) -> Rect {
        classic::sheet(c, card, title)
    }
    /// A page's heading (Settings and its submenus), in r.
    fn page_title(&self, c: &mut Canvas, r: Rect, title: &str) {
        font_draw_left(c, FontFace::Title, ui_title_size(), r, title, 0);
    }
    /// A toast's box (its ends `ends` deep), under its white text.
    fn toast_plate(&self, c: &mut Canvas, r: Rect, ends: i32) {
        canvas_round_rect(c, r, ends, 0, true, 0);
    }
    /// A checkbox's box b.
    fn checkbox_box(&self, c: &mut Canvas, b: Rect, checked: bool) {
        canvas_round_rect(c, b, ui_px(10.0), if checked { 0 } else { 255 }, true, 0);
        canvas_round_rect(c, b, ui_px(10.0), 0, false, ui_px(4.0));
        if checked {
            ui_icon(c, IconKind::Check, b, 255);
        }
    }
    /// A Settings strip of n segments with segment `on` picked (its segments' rects: seg). true: drawn.
    fn segmented(&self, _c: &mut Canvas, _r: Rect, _n: i32, _on: i32, _seg: &dyn Fn(i32) -> Rect) -> bool {
        false
    }
    /// A menu row's card in style st. true: drawn (else the menu draws its own).
    fn menu_row_card(&self, _c: &mut Canvas, _r: Rect, _st: ButtonStyle) -> bool {
        false
    }

    // ---- paper tiles (New notebook, the editor's paper sheet, Templates) ----

    /// A paper tile's frame (cell_h: its grid row's height).
    fn tile_frame(&self, c: &mut Canvas, tile: Rect, _gap: i32, _cell_h: i32, sel: bool) {
        self.frame(c, rect_inflate(tile, if sel { ui_px(10.0) } else { 0 }), ui_px(12.0), 0, false, if sel { ui_px(9.0) } else { ui_px(3.0) });
    }
    /// A tile's label with its number n (1 up; 0 none). true: drawn (else the tile's usual label).
    fn tile_label(&self, _c: &mut Canvas, _tile: Rect, _gap: i32, _n: i32, _label: &str, _sel: bool) -> bool {
        false
    }
    /// Marks round a grid of tiles (x0, y0: the first tile's corner).
    fn grid_marks(&self, _c: &mut Canvas, _x0: i32, _y0: i32, _g: &PaperGrid, _gap: i32) {}

    // ---- the keyboard, the status corner, the links map ----

    /// The on-screen keyboard's area and its text field.
    fn keyboard_bed(&self, c: &mut Canvas, area: Rect, field: Rect) {
        self.frame(c, area, ui_px(24.0), 255, true, 0);
        self.frame(c, area, ui_px(24.0), 0, false, ui_px(6.0));
        self.frame(c, field, ui_px(12.0), 255, true, 0);
        self.frame(c, field, ui_px(12.0), 0, false, ui_px(3.0));
    }
    /// A key's cap in style st; returns its letters' gray.
    fn key_cap(&self, c: &mut Canvas, r: Rect, st: ButtonStyle) -> u8 {
        self.frame(c, r, ui_px(12.0), if st == ButtonStyle::Selected { 0 } else { 255 }, true, 0);
        self.frame(c, r, ui_px(12.0), 0, false, if st == ButtonStyle::Primary { ui_px(6.0) } else { ui_px(3.0) });
        if st == ButtonStyle::Selected { 255 } else { 0 }
    }
    /// The clock and battery corner's box.
    fn status_box(&self, c: &mut Canvas, r: Rect) {
        self.frame(c, r, ui_px(8.0), 255, true, 0);
        self.frame(c, r, ui_px(8.0), 0, false, ui_px(2.0));
    }
    /// Lit units of the battery's pixel art, r in screen pixels (u: a unit).
    fn battery_units(&self, c: &mut Canvas, r: Rect, _u: i32, gray: u8) {
        canvas_fill(c, r, gray);
    }
    /// The ring round the chosen node or the open page (t: its stroke). true: drawn.
    fn map_ring(&self, _c: &mut Canvas, _x: f32, _y: f32, _r: f32, _t: f32) -> bool {
        false
    }
    /// A link between two nodes. true: drawn.
    fn map_link(&self, _c: &mut Canvas, _a: (f32, f32), _b: (f32, f32), _width: f32, _selected: bool) -> bool {
        false
    }
    /// A mark round the chosen node (r: the node's radius; its ring is 12 px out).
    fn map_lock(&self, _c: &mut Canvas, _x: f32, _y: f32, _r: f32) {}
    /// A node (level: how linked it is, 0 to 4). true: drawn.
    fn map_node(&self, _c: &mut Canvas, _x: f32, _y: f32, _r: f32, _level: i32) -> bool {
        false
    }

    // ---- the Library's screens ----

    /// The app's name at the Library's top, after its mark (x0: the mark's left, x: the name's).
    /// Some(the name's width): drawn.
    fn library_mark(&self, _c: &mut Canvas, _r: Rect, _title: &str, _x0: i32, _x: i32, _header_h: i32) -> Option<i32> {
        None
    }
    /// Anything under the Library's header rule.
    fn library_readout(&self, _c: &mut Canvas, _shelf: &ShelfInfo) {}
    /// The Library's page count between its arrows (pr, nr). true: drawn.
    fn library_pager(&self, _c: &mut Canvas, _pr: Rect, _nr: Rect, _page: i32, _pages: i32) -> bool {
        false
    }
    /// The Library framed on a whole redraw, over everything.
    fn library_overlay(&self, _c: &mut Canvas, _info: &ScreenInfo) {}
    /// New notebook's heading over the papers, with the paper chosen. true: drawn.
    fn new_notebook_heading(&self, _c: &mut Canvas, _r: Rect, _chosen: &str) -> bool {
        false
    }
    /// New notebook's foot from y: the name, its folder, Rename and the four actions. Some: drawn.
    fn new_notebook_foot(&self, _c: &mut Canvas, _in_: Rect, _y: i32, _name: &str, _folder: &str, _tpl: bool) -> Option<NewFoot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_numbers_name_their_themes() {
        // (Settings.theme 0 and 1 were Gothic and Classic before themes were a list: settings.ini names
        // them by id now, but the numbers stay what they were)
        assert_eq!(UiTheme::Gothic.id(), "gothic");
        assert_eq!(UiTheme::Classic.id(), "classic");
        let ids: Vec<_> = UiTheme::all().map(|t| t.id()).collect();
        let mut seen = ids.clone();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), ids.len(), "two themes with one id: {ids:?}");
        for t in UiTheme::all() {
            assert_eq!(UiTheme::from_id(t.id().as_bytes()), Some(t));
            assert_eq!(UiTheme::from_i32(t.index()), t);
        }
        assert_eq!(UiTheme::from_i32(-1), UiTheme::Gothic);
        assert_eq!(UiTheme::from_i32(LOOKS.len() as i32), UiTheme::Gothic);
        assert_eq!(UiTheme::from_id(b"comic"), None);
        // the shortcut goes round them all
        let mut t = UiTheme::Gothic;
        for _ in 0..LOOKS.len() {
            t = t.next();
        }
        assert_eq!(t, UiTheme::Gothic);
    }

    #[test]
    fn each_theme_is_the_file_named_by_its_id() {
        // (the scenario runner finds the themes to check by these files: tests/scenario_runner.py, l10n)
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/looks");
        for t in UiTheme::all() {
            assert!(dir.join(format!("{}.rs", t.id())).is_file(), "src/looks/{}.rs", t.id());
        }
    }
}
