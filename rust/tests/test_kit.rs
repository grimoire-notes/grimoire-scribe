// test_kit.rs: the package kit (kithost.rs, docs/KIT.md), with modules written for the purpose (tests/kitfix) in
// both hosts: each element given no size is drawn as Grimoire's own screen draws it, pixel for pixel, in Gothic and
// Classic and each family of Settings > Font; and each call keeps the rules of its host (the fixed charge first,
// lengths compared before the module's memory is read, the host's work paid from the call's budget, drawing held
// to the body, the hosts' limits, a widget's capability to draw).
mod kitfix;

use grimoire::globalui::{
    gui_button, gui_button_h, gui_button_row, gui_button_row_need_w, gui_chip, gui_chip_h, gui_chip_w, gui_field_h, gui_gap, gui_head_h, gui_icon_button, gui_margin,
    gui_page_title_px, gui_row_h, gui_row_min_h, gui_select, gui_select_h, gui_status_h, gui_text_field, gui_top_button_h,
};
use kitfix::KIT as KIT_IMPORTS;
use grimoire::kithost::*;
use grimoire::prelude::*;
use kitfix::*;

/// The screen of the tests: a Scribe's.
const SCREEN: (i32, i32) = (1860, 2480);
/// The bodies the passes draw on: an app's under the status bar, and a large widget frame's somewhere on the screen.
fn app_body() -> Rect {
    let top = gui_status_h();
    Rect { x: 0, y: top, w: SCREEN.0, h: SCREEN.1 - top }
}
const WIDGET_BODY: Rect = Rect { x: 214, y: 192, w: 1272, h: 2200 };
const MAX: i32 = i32::MAX;
const MIN: i32 = i32::MIN;

/// A fresh data root for one test (the Makefile's test rules set GRIMOIRE_TEST_TMP), the UI at 300 dpi.
fn fresh_root(name: &str) -> std::path::PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let root = base.join(format!("kit-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    ui_init(300.0);
    assert_eq!(font_init(), 0);
    ui_set_theme(UiTheme::Gothic);
    font_set_family(0);
    root
}

/// A loaded module of either host.
enum Rt {
    W(WidgetRuntime),
    A(AppRuntime),
}

impl Rt {
    fn body(&self) -> Rect {
        match self {
            Rt::W(_) => WIDGET_BODY,
            Rt::A(_) => app_body(),
        }
    }
    fn draw(&mut self, c: &mut Canvas) {
        let body = self.body();
        match self {
            Rt::W(r) => r.draw(c, body),
            Rt::A(r) => r.draw(c, body, 0),
        }
    }
    fn error(&self) -> Option<&str> {
        match self {
            Rt::W(r) => r.error(),
            Rt::A(r) => r.error(),
        }
    }
    fn status(&self) -> String {
        match self {
            Rt::W(r) => r.status().to_string(),
            Rt::A(r) => r.status().to_string(),
        }
    }
    fn text(&self) -> String {
        match self {
            Rt::W(r) => r.text(),
            Rt::A(r) => r.text(),
        }
    }
    fn fuel(&self) -> u64 {
        match self {
            Rt::W(r) => r.fuel_used(),
            Rt::A(r) => r.fuel_used(),
        }
    }
    fn read(&self) -> usize {
        match self {
            Rt::W(r) => r.bytes_read(),
            Rt::A(r) => r.bytes_read(),
        }
    }
    fn hits(&self) -> usize {
        match self {
            Rt::W(r) => r.hit_count(),
            Rt::A(r) => r.hits().len(),
        }
    }
    fn hit(&self, id: i32) -> Option<Rect> {
        match self {
            Rt::W(r) => r.hit(id),
            Rt::A(r) => r.hit(id),
        }
    }
    fn stopped(&self) -> &'static str {
        match self {
            Rt::W(_) => "The widget stopped: all fuel consumed by WebAssembly",
            Rt::A(_) => "The app stopped: all fuel consumed by WebAssembly",
        }
    }
}

/// Installs a module as a package of its host, as an install from a repository does, loads it and removes the
/// package (no more than 64 are installed at a time; the loaded module runs on). A widget without `overlay` is a
/// package an earlier install left without `editor:overlay`.
fn load(id: &str, kit: &Kit, overlay: bool) -> Result<Rt, String> {
    let module = kit.bytes();
    match kit.host {
        Host::Widget => {
            let m = widget_manifest(id, "Test", &WIDGET_BOTH, 400, 300, &module);
            widget_package_install(&m, &module, "", SCREEN.0, SCREEN.1).unwrap();
            if !overlay {
                let yaml = format!("{}/package.yaml", widget_package_dir(id));
                let text = std::fs::read_to_string(&yaml).unwrap().replace("capabilities: [editor:overlay, storage:widget-local]", "capabilities: [storage:widget-local]");
                std::fs::write(&yaml, text).unwrap();
            }
            let rt = WidgetRuntime::new(id, (WIDGET_BODY.w, WIDGET_BODY.h)).map(Rt::W);
            widget_package_remove(id, false).unwrap();
            rt
        }
        Host::App => {
            let m = app_manifest(id, "Test", &[], true, &module);
            app_package_install(&m, &module, None, "", SCREEN.0, SCREEN.1).unwrap();
            let rt = AppRuntime::open(id, SCREEN.0, SCREEN.1, false).map(Rt::A);
            app_package_remove(id, false).unwrap();
            rt
        }
    }
}

fn loaded(id: &str, kit: &Kit) -> Rt {
    load(id, kit, true).unwrap()
}

/// A canvas with no two neighbours alike, so that any pixel a draw changes shows.
fn patterned() -> Canvas {
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, SCREEN.0, SCREEN.1), 0);
    for (i, p) in c.px.iter_mut().enumerate() {
        *p = (i * 7 % 251) as u8;
    }
    c
}

/// How many pixels outside `keep` differ between the two canvases.
fn changed_outside(before: &Canvas, after: &Canvas, keep: Rect) -> usize {
    let mut n = 0;
    for y in 0..before.h {
        for x in 0..before.w {
            let at = (y * before.stride + x) as usize;
            if !rect_contains(keep, x, y) && before.px[at] != after.px[at] {
                n += 1;
            }
        }
    }
    n
}

/// How many pixels of `r` are not white.
fn inked(c: &Canvas, r: Rect) -> usize {
    (r.y..r.y + r.h).map(|y| (r.x..r.x + r.w).filter(|&x| c.px[(y * c.stride + x) as usize] != 255).count()).sum()
}

/// The canvas after one pass of the module.
fn drawn(rt: &mut Rt) -> Canvas {
    let mut c = patterned();
    rt.draw(&mut c);
    c
}

const HOSTS: [Host; 2] = [Host::Widget, Host::App];

// (the goldens' keys: "g" Gothic, "c" Classic)
fn look(theme: UiTheme) -> &'static str {
    &theme.id()[..1]
}

fn tag(host: Host) -> &'static str {
    match host {
        Host::Widget => "w",
        Host::App => "a",
    }
}

// ---- the elements at the standard sizes, pixel for pixel ----

fn icon_of(name: &str) -> IconKind {
    if name.is_empty() {
        IconKind::None
    } else {
        widget_icon(name)
    }
}

fn style(v: i32) -> ButtonStyle {
    match v {
        1 => ButtonStyle::Selected,
        2 => ButtonStyle::Disabled,
        3 => ButtonStyle::Primary,
        _ => ButtonStyle::Plain,
    }
}

/// The face and size of a role, as docs/KIT.md gives them: written out here, not asked of the kit.
fn role(r: i32) -> (FontFace, f32) {
    match r {
        1 => (FontFace::Regular, ui_small_size()),
        2 => (FontFace::Bold, ui_text_size()),
        3 => (FontFace::Bold, ui_small_size()),
        4 => (FontFace::Title, ui_title_size()),
        5 => (FontFace::Fixed, ui_text_size()),
        6 => (FontFace::Fixed, ui_small_size()),
        7 => (FontFace::Title, ui_title_size() * if ui_theme() == UiTheme::Gothic { 1.75 } else { 1.1 }),
        _ => (FontFace::Regular, ui_text_size()),
    }
}

fn line_of(r: i32) -> i32 {
    let (f, px) = role(r);
    font_ascent(f, px) + font_descent(f, px)
}

/// The elements and texts of a page, each given no size where it has a standard one.
fn page() -> Vec<K> {
    vec![
        K::Row(1, 20, 20, 0, 0, "gear", "A row of the kit", 0, 1),
        K::Row(-1, 20, 150, 600, 0, "", "A chosen row with no icon", 1, -1),
        K::Row(-1, 640, 150, 500, 0, "language", "A disabled row", 2, 0),
        K::Button(2, 20, 290, 0, 0, "Button", "", 0),
        K::Button(-1, 400, 290, 0, 0, "Primary", "", 3),
        K::Button(-1, 800, 290, 0, 0, "", "files", 1),
        K::IconButton(7, 1000, 290, 0, 0, "search", 0),
        K::Chip(3, 20, 420, 0, 0, "Chip", 1),
        K::Chip(-1, 300, 420, 0, 0, "Recent", 0),
        K::Field(4, 20, 530, 0, 0, "search", "typed", "Search", 0),
        K::Field(-1, 20, 660, 600, 0, "", "secret", "Password", 1),
        K::Field(-1, 640, 660, 500, 0, "search", "", "Search", 0),
        K::Select(5, 20, 790, 600, 0, "Sort", "Name"),
        K::Checkbox(6, 20, 900, 0, 0, "A checkbox", 1),
        K::Checkbox(-1, 620, 900, 500, 0, "Not ticked", 0),
        K::Card(20, 1010, 0, 0, 0),
        K::Divider(20, 1150, 0),
        K::Icon(20, 1180, 0, 0, "gear", 0),
        K::Icon(100, 1180, 0, 0, "md-clock", 2),
        K::Fill(200, 1180, 200, 60, 128),
        K::Frame(420, 1180, 200, 80, 0, 0, 0),
        K::Frame(640, 1180, 200, 80, 0, 1, 0),
        K::Line(860.0, 1180.0, 1100.0, 1260.0, 3.0, 0),
        K::Text(20, 1280, 0, 0, "Body text", 0, 0),
        K::Text(20, 1340, 500, 0, "Small text", 1, 0),
        K::Text(540, 1340, 500, 0, "A label", 2, 1),
        K::Text(20, 1400, 500, 0, "A caption", 3, 2),
        K::Text(540, 1400, 500, 0, "Muted", 0, 16),
        K::Text(20, 1460, 900, 0, "A title", 4, 0),
        K::Text(20, 1540, 600, 0, "Mono 0123", 5, 0),
        K::Text(640, 1540, 400, 0, "mono small", 6, 32),
        K::Text(20, 1600, 1000, 0, "Display", 7, 0),
        K::Text(20, 1730, 900, 0, "First line\nSecond line\nThird", 0, 4),
        K::Text(20, 1900, 300, 0, "A label too long for its room is fitted on two lines", 2, 8),
        K::Lines(940, 1730, 300, 2 * 52, "one\ntwo\nthree\nfour\nfive", 0, 0, 2),
        K::Pager(8, 9, 20, 2050, 1100, 1, 3),
        K::Hit(11, 1150, 290, 60, 60),
        K::Button(-1, 640, 1900, 0, 0, "Install", "download", 3),
        K::Button(-1, 640, 1610, 0, 0, "License", "paper", 0),
    ]
}

/// The same page drawn by Grimoire's own drawers, at the sizes Grimoire's screens give them (globalui.rs and
/// ui.rs, which the screens read): what docs/KIT.md says each call draws.
fn grimoires_own(c: &mut Canvas, body: Rect) {
    let at = |x: i32, y: i32, w: i32, h: i32| Rect { x: body.x + x, y: body.y + y, w, h };
    let rest = |x: i32| body.w - x;
    let whole = c.clip;
    canvas_set_clip(c, rect_intersect(whole, body));
    canvas_fill(c, body, 255);
    let row = |c: &mut Canvas, r: Rect, icon: &str, label: &str, st: i32, switch: i32| {
        let mut row = Row::new(icon_of(icon), label);
        row.style = style(st);
        if switch >= 0 {
            row.switch = Some(switch != 0);
        }
        ui_row(c, r, &row);
    };
    row(c, at(20, 20, rest(20), gui_row_h()), "gear", "A row of the kit", 0, 1);
    row(c, at(20, 150, 600, gui_row_h()), "", "A chosen row with no icon", 1, -1);
    row(c, at(640, 150, 500, gui_row_h()), "language", "A disabled row", 2, 0);
    let bh = gui_button_h();
    gui_button(c, at(20, 290, button_need_w(bh, false, "Button"), bh), IconKind::None, "Button", ButtonStyle::Plain);
    gui_button(c, at(400, 290, button_need_w(bh, false, "Primary"), bh), IconKind::None, "Primary", ButtonStyle::Primary);
    gui_button(c, at(800, 290, bh, bh), widget_icon("files"), "", ButtonStyle::Selected);
    gui_icon_button(c, at(1000, 290, gui_top_button_h(), gui_top_button_h()), widget_icon("search"), ButtonStyle::Plain);
    gui_chip(c, at(20, 420, gui_chip_w("Chip"), gui_chip_h()), "Chip", true);
    gui_chip(c, at(300, 420, gui_chip_w("Recent"), gui_chip_h()), "Recent", false);
    gui_text_field(c, at(20, 530, rest(20), gui_field_h()), widget_icon("search"), "typed", "Search");
    gui_text_field(c, at(20, 660, 600, gui_field_h()), IconKind::None, "******", "Password");
    gui_text_field(c, at(640, 660, 500, gui_field_h()), widget_icon("search"), "", "Search");
    gui_select(c, at(20, 790, 600, gui_select_h()), "Sort", "Name");
    ui_checkbox(c, at(20, 900, rest(20), gui_row_min_h()), "A checkbox", true);
    ui_checkbox(c, at(620, 900, 500, gui_row_min_h()), "Not ticked", false);
    ui_card(c, at(20, 1010, rest(20), gui_row_h()), ButtonStyle::Plain);
    ui_rule(c, body.x + 20, body.x + body.w, body.y + 1150, ui_line(), 0);
    ui_icon(c, widget_icon("gear"), at(20, 1180, ui_row_icon(), ui_row_icon()), 0);
    ui_icon(c, widget_icon("md-clock"), at(100, 1180, ui_row_icon(), ui_row_icon()), 170);
    canvas_fill(c, at(200, 1180, 200, 60), 128);
    ui_frame(c, at(420, 1180, 200, 80), ui_card_radius(), 0, false, ui_line());
    ui_frame(c, at(640, 1180, 200, 80), ui_card_radius(), 0, true, ui_line());
    canvas_line_aa(c, body.x as f32 + 860.0, body.y as f32 + 1180.0, body.x as f32 + 1100.0, body.y as f32 + 1260.0, 3.0, 0);
    // (a text on one line is drawn as Grimoire's labels are: left, centred or at the right, vertically centred in
    // a slot one line high, and never reported to the translations' fit check)
    let text = |c: &mut Canvas, r: Rect, s: &str, ro: i32, align: i32, gray: u8| {
        let (f, px) = role(ro);
        match align {
            1 => font_draw_centered(c, f, px, r, s, gray),
            2 => {
                let w = font_measure(f, px, s);
                font_draw(c, f, px, r.x + r.w - w, r.y + (r.h - line_of(ro)) / 2 + font_ascent(f, px), s, gray);
            }
            _ => font_draw_left_name(c, f, px, r, s, gray),
        }
    };
    text(c, at(20, 1280, rest(20), line_of(0)), "Body text", 0, 0, 0);
    text(c, at(20, 1340, 500, line_of(1)), "Small text", 1, 0, 0);
    text(c, at(540, 1340, 500, line_of(2)), "A label", 2, 1, 0);
    text(c, at(20, 1400, 500, line_of(3)), "A caption", 3, 2, 0);
    text(c, at(540, 1400, 500, line_of(0)), "Muted", 0, 0, 100);
    text(c, at(20, 1460, 900, line_of(4)), "A title", 4, 0, 0);
    text(c, at(20, 1540, 600, line_of(5)), "Mono 0123", 5, 0, 0);
    text(c, at(640, 1540, 400, line_of(6)), "mono small", 6, 0, 170);
    text(c, at(20, 1600, 1000, line_of(7)), "Display", 7, 0, 0);
    // (a text with WRAP: its own lines, one under the other, the block as tall as its lines)
    for (k, s) in ["First line", "Second line", "Third"].iter().enumerate() {
        text(c, at(20, 1730 + k as i32 * line_of(0), 900, line_of(0)), s, 0, 0, 0);
    }
    // (FIT: Grimoire's label rule, the role's size, then a smaller one down to the small size, then two lines)
    let (f, px) = role(2);
    let fit = "A label too long for its room is fitted on two lines";
    let fit_h = if font_fit_px(f, px, ui_small_size(), fit, 300) > 0.0 { line_of(2) } else { 2 * (font_ascent(f, ui_small_size()) + font_descent(f, ui_small_size())) };
    ui_text(c, f, px, ui_small_size(), at(20, 1900, 300, fit_h), fit, 2, false, 0);
    // (a page of text_lines: from its third line, as many as its room holds, from the top)
    for (k, s) in ["three", "four"].iter().enumerate() {
        text(c, at(940, 1730 + k as i32 * line_of(0), 300, line_of(0)), s, 0, 0, 0);
    }
    let (bw, ph) = ui_row_pager_button();
    ui_row_pager(c, &RowPage { rows: Vec::new(), first: 0, page: 1, pages: 3, prev: at(20, 2050, bw, ph), next: at(1120 - bw, 2050, bw, ph) });
    // (a button with an icon and a label: the Package Store's action button, at the width it needs)
    gui_button_row(c, at(640, 1900, button_need_w(bh, true, "Install"), bh), widget_icon("download"), "Install", ButtonStyle::Primary);
    gui_button_row(c, at(640, 1610, button_need_w(bh, true, "License"), bh), widget_icon("paper"), "License", ButtonStyle::Plain);
    c.clip = whole;
}

#[test]
fn each_element_given_no_size_is_grimoires_own_pixel_for_pixel() {
    let root = fresh_root("own");
    let cmds = page();
    // (the page of text_lines asks for room for two lines of the body role: 52 px in every theme and family here)
    let looks: Vec<(UiTheme, usize)> = vec![(UiTheme::Gothic, 0), (UiTheme::Classic, 0), (UiTheme::Classic, 1), (UiTheme::Classic, 2), (UiTheme::Classic, 3)];
    for host in HOSTS {
        for &(theme, family) in &looks {
            ui_set_theme(theme);
            font_set_family(family);
            let lh = line_of(0);
            let mut cmds = cmds.clone();
            // (the page of text_lines has room for two lines of the body role in this look)
            if let Some(K::Lines(_, _, _, h, ..)) = cmds.iter_mut().find(|c| matches!(c, K::Lines(..))) {
                *h = 2 * lh;
            }
            let mut rt2 = loaded(&format!("test.kit.own.{}.{}.{family}", tag(host), look(theme)), &pass(host, &cmds));
            let before = patterned();
            let (mut got, mut want) = (before.clone(), before.clone());
            rt2.draw(&mut got);
            assert_eq!(rt2.error(), None, "{host:?} {theme:?} {family}");
            grimoires_own(&mut want, rt2.body());
            let differing = got.px.iter().zip(&want.px).filter(|(a, b)| a != b).count();
            assert_eq!(differing, 0, "{host:?}, {theme:?}, family {family}: the kit's page is Grimoire's own drawing, pixel for pixel");
            assert!(inked(&got, rt2.body()) > 100_000, "{host:?} {theme:?}: the page drew");
            assert_eq!(changed_outside(&before, &got, rt2.body()), 0, "{host:?} {theme:?}: the page keeps to its body");
            // (the probes read each element's text, in the order drawn; a password's as stars)
            assert_eq!(
                rt2.text(),
                "A row of the kit|A chosen row with no icon|A disabled row|Button|Primary||Chip|Recent|typed|Search|******|Password||Search|Name|A checkbox|Not ticked|Body text|Small text|A label|A caption|Muted|A title|Mono 0123|mono small|Display|First line Second line Third|A label too long for its room is fitted on two lines|three four|Install|License"
            );
            // (the elements given an id are its hit regions, at their places in the body)
            assert_eq!(rt2.hits(), 10, "{host:?}");
            assert_eq!(rt2.hit(1), Some(Rect::new(20, 20, rt2.body().w - 20, gui_row_h())));
            assert_eq!(rt2.hit(3), Some(Rect::new(20, 420, gui_chip_w("Chip"), gui_chip_h())));
            assert_eq!(rt2.hit(7), Some(Rect::new(1000, 290, gui_top_button_h(), gui_top_button_h())));
            assert_eq!(rt2.hit(8), Some(Rect::new(20, 2050, 130, 90)));
            assert_eq!(rt2.hit(9), Some(Rect::new(1120 - 130, 2050, 130, 90)));
            assert_eq!(rt2.hit(11), Some(Rect::new(1150, 290, 60, 60)));
        }
    }
    ui_set_theme(UiTheme::Gothic);
    font_set_family(0);
    let _ = std::fs::remove_dir_all(&root);
}

// ---- what the calls return ----

/// Code that writes the i32 the code before it left on the stack, plus 1,000,000,000, as ten digits at `at`.
fn says_at(at: i32) -> Vec<u8> {
    seq(&[i(1_000_000_000), vec![ADD], digits_of(at, 10)])
}

/// What each piece of `code` (each leaving an i32) left, run in a drawing pass of this host: a module for each
/// twenty of them (a status line holds 256 bytes).
fn results(host: Host, id: &str, code: Vec<Vec<u8>>, t: &Texts) -> Vec<i64> {
    let mut out = Vec::new();
    for (k, chunk) in code.chunks(20).enumerate() {
        let mut rt = loaded(&format!("{id}.{k}"), &saying(host, chunk.to_vec(), Texts(t.0.clone())));
        out.extend(said(&mut rt));
    }
    out
}

/// A module whose drawing pass runs each piece of `code` (each leaving an i32) and says what each left, as ten
/// digits less 1,000,000,000, in its status line, one after another with a space between.
fn saying(host: Host, code: Vec<Vec<u8>>, t: Texts) -> Kit {
    let n = code.len() as i32;
    let mut body = Vec::new();
    for (k, piece) in code.iter().enumerate() {
        body.extend(piece.clone());
        body.extend(says_at(30_000 + k as i32 * 11));
    }
    let spaces: Vec<(u32, Vec<u8>)> = (0..n).map(|k| ((30_000 + k * 11 + 10) as u32, b" ".to_vec())).collect();
    body.extend(host_status(30_000, n * 11 - 1));
    let mut kit = Kit::new(host, body);
    kit.pages = 2;
    kit.data = vec![(TEXTS_AT as u32, t.0)];
    kit.data.extend(spaces);
    kit
}

fn host_status(ptr: i32, len: i32) -> Vec<u8> {
    kitfix::host("status", &[ptr, len])
}

/// The numbers a `saying` module said.
fn said(rt: &mut Rt) -> Vec<i64> {
    let _ = drawn(rt);
    assert_eq!(rt.error(), None);
    rt.status().split(' ').map(|d| d.parse::<i64>().unwrap() - 1_000_000_000).collect()
}

/// A call of the kit that leaves its i32 result.
fn kept(name: &str, args: &[i32]) -> Vec<u8> {
    let mut o: Vec<u8> = args.iter().flat_map(|&a| i(a)).collect();
    o.extend(invoke(name));
    o
}

#[test]
fn metric_answers_the_sizes_grimoires_screens_use_and_minus_one_for_an_id_it_does_not_have() {
    let root = fresh_root("metric");
    let ids: Vec<i32> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 30, 31, 32, 33, 34, 35, 36, 37, 40, 41, 42, 43, 44, 0, -1, 26, 29, 38, 45, 99, MAX, MIN];
    for host in HOSTS {
        for (theme, family) in [(UiTheme::Gothic, 0), (UiTheme::Classic, 0), (UiTheme::Classic, 3)] {
            ui_set_theme(theme);
            font_set_family(family);
            let got = results(host, &format!("test.kit.metric.{}.{}.{family}", tag(host), look(theme)), ids.iter().map(|&id| kept("metric", &[id])).collect(), &Texts::default());
            let body = match host {
                Host::Widget => WIDGET_BODY,
                Host::App => app_body(),
            };
            let gothic = theme == UiTheme::Gothic;
            let want: Vec<i64> = vec![
                44,
                34,
                64,
                if gothic { 112 } else { 70 },
                line_of(0) as i64,
                line_of(1) as i64,
                line_of(4) as i64,
                line_of(7) as i64,
                line_of(5) as i64,
                118,
                90,
                96,
                88,
                112,
                60,
                96,
                90,
                150,
                88,
                50,
                24,
                16,
                30,
                if gothic { 16 } else { 22 },
                3,
                1000,
                300,
                if gothic { 0 } else { 1 },
                if gothic { 5 } else { family as i64 },
                // (a widget's: the notebook's status bar; an app's that shows it: its own)
                gui_status_h() as i64,
                -1,
                body.w as i64,
                body.h as i64,
                0,
                255,
                100,
                170,
                140,
            ];
            assert_eq!(&got[..want.len()], &want[..], "{host:?} {theme:?} {family}");
            assert!(got[want.len()..].iter().all(|&v| v == -1), "{host:?}: ids this level does not have: {:?}", &got[want.len()..]);
        }
    }
    // The values are the functions the screens read.
    ui_set_theme(UiTheme::Gothic);
    assert_eq!(
        (gui_row_h(), gui_row_min_h(), gui_button_h(), gui_chip_h(), gui_field_h(), gui_select_h(), gui_top_button_h(), gui_head_h(), gui_margin(), gui_gap()),
        (118, 90, 96, 88, 112, 88, 96, 150, 50, 24)
    );
    assert_eq!((ui_row_space(), ui_row_pad(), ui_row_icon(), ui_card_radius(), ui_line(), gui_page_title_px()), (16, 30, 60, 16, 3, 112.0));
    ui_set_theme(UiTheme::Classic);
    assert_eq!((ui_card_radius(), gui_page_title_px().round()), (22, 70.0));
    // An app that hides the status bar is told 0 for its height.
    let mut hidden = saying(Host::App, vec![kept("metric", &[34])], Texts::default());
    hidden.imports.push(("grimoire", "show_status_bar", "i", 'v'));
    hidden.code[0] = seq(&[i(0), vec![0x10], vec![(KIT_IMPORTS.len() + HOST.len()) as u8], hidden.code[0].clone()]);
    let mut rt = loaded("test.kit.metric.hidden", &hidden);
    assert_eq!(said(&mut rt), vec![0]);
    ui_set_theme(UiTheme::Gothic);
    font_set_family(0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_calls_return_the_level_the_version_and_the_extent_each_element_took() {
    let root = fresh_root("returns");
    for host in HOSTS {
        let mut t = Texts::default();
        let [lp, ln] = t.put("Button");
        let [cp, cn] = t.put("Chip");
        let [pp, pn] = t.put("First line\nSecond line\nThird");
        let [gp, gn] = t.put("gear");
        let code = vec![
            kept("level", &[]),
            kept("version", &[20_000, 64]),
            kept("version", &[20_000, 3]),
            kept("version", &[20_000, 0]),
            kept("row", &[-1, 0, 0, 0, 0, 0, 0, lp, ln, 0, -1]),
            kept("row", &[-1, 0, 0, 0, 200, 0, 0, lp, ln, 0, -1]),
            kept("button", &[-1, 0, 0, 0, 0, lp, ln, 0, 0, 0]),
            kept("button", &[-1, 0, 0, 0, 60, 0, 0, gp, gn, 0]),
            kept("button", &[-1, 0, 0, 333, 0, lp, ln, 0, 0, 0]),
            kept("button", &[-1, 0, 0, 0, 0, lp, ln, gp, gn, 0]),
            kept("icon_button", &[-1, 0, 0, 0, 0, gp, gn, 0]),
            kept("icon_button", &[-1, 0, 0, 0, 70, gp, gn, 0]),
            kept("chip", &[-1, 0, 0, 0, 0, cp, cn, 0]),
            kept("field", &[-1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            kept("select", &[-1, 0, 0, 0, 0, 0, 0, 0, 0]),
            kept("checkbox", &[-1, 0, 0, 0, 0, 0, 0, 0]),
            kept("text", &[0, 0, 0, 0, pp, pn, 0, 4]),
            kept("text", &[0, 0, 0, 0, lp, ln, 4, 0]),
            kept("text", &[0, 0, 0, 77, lp, ln, 0, 0]),
            kept("text_lines", &[0, 0, 400, 100, pp, pn, 0, 0, 1]),
            kept("measure_width", &[lp, ln, 2]),
            kept("measure_height", &[pp, pn, 0, 0]),
            kept("measure_height", &[pp, pn, 1, 600]),
            kept("pager", &[-1, -1, 0, 0, 0, 0, 3]),
            kept("pager", &[-1, -1, 0, 0, 0, 0, 1]),
        ];
        let got = results(host, &format!("test.kit.returns.{}", tag(host)), code, &t);
        let v = GRIMOIRE_VERSION.len().min(32) as i64;
        let need = |h: i32, icon: bool, label: &str| button_need_w(h, icon, label) as i64;
        assert_eq!(
            got,
            vec![
                1,
                v,
                3,
                0,
                118,
                200,
                need(96, false, "Button"),
                60,
                333,
                // (in Gothic, with an icon and a label: what the Store's action button needs for them on one line)
                gui_button_row_need_w(96, "Button") as i64,
                96,
                70,
                gui_chip_w("Chip") as i64,
                112,
                88,
                90,
                3 * line_of(0) as i64,
                line_of(4) as i64,
                77,
                3,
                font_measure(FontFace::Bold, ui_text_size(), "Button") as i64,
                3 * line_of(0) as i64,
                3 * line_of(1) as i64,
                90,
                0
            ],
            "{host:?}"
        );
        // (version writes Grimoire's version, at most the room given)
        assert!(GRIMOIRE_VERSION.starts_with("2.") || GRIMOIRE_VERSION.starts_with("3."));
    }
    // The clock is the app's (now_ms), and list_page is ui_row_page's answer, packed.
    let packed = kit_list_page(30, 2000, 118, 90);
    let (per, rh, gap, pages) = (packed & 0xffff, packed >> 16 & 0xffff, packed >> 32 & 0xffff, packed >> 48 & 0xffff);
    let p = ui_row_page(Rect::new(0, 0, 1000, 2000), 30, 0, 118, 90);
    assert_eq!((rh, gap), (p.rows[0].h as i64, (p.rows[1].y - p.rows[0].y - p.rows[0].h) as i64));
    assert_eq!((per, pages), (p.rows.len() as i64, p.pages as i64));
    assert_eq!(pages, (30 + per - 1) / per);
    // (every number a module passes: the rows and heights held to their limits, no sum past its bits)
    for (n, h, pref, min) in [(MAX, MAX, MAX, MAX), (MIN, MIN, MIN, MIN), (10_000, 32_767, 32_767, 0), (1, 0, 1, 0), (0, 2000, 118, 90)] {
        let v = kit_list_page(n, h, pref, min);
        assert!(v >= 0 && v >> 48 <= 10_000, "{n} {h} {pref} {min}: {v:x}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

// ---- the rules of a call ----

/// What a call's budget is charged for `with` beside `without` in this host: the fuel a pass of the one used less
/// the fuel a pass of the other used, on the host's body, each the module's second pass.
fn charged(host: Host, id: &str, with: Vec<u8>, without: Vec<u8>, data: &[u8]) -> u64 {
    let mut used = Vec::new();
    for (k, code) in [with, without].into_iter().enumerate() {
        let mut kit = Kit::new(host, code);
        kit.pages = 2;
        kit.data = vec![(0, data.to_vec())];
        let mut rt = loaded(&format!("test.kit.charge.{}.{id}.{k}", tag(host)), &kit);
        // (the second pass: a function's first call also pays for making it ready, by the bytes of its code)
        let _ = drawn(&mut rt);
        let _ = drawn(&mut rt);
        assert_eq!(rt.error(), None, "{host:?} {id}");
        used.push(rt.fuel());
    }
    used[0].saturating_sub(used[1])
}

/// A charge is the one expected (the two passes compared are the same instructions with other numbers, and the
/// interpreter counts some tens of instructions more or less for the numbers a function holds).
fn near(charged: u64, expected: u64, what: &str) {
    assert!(charged.abs_diff(expected) <= 64, "{what}: charged {charged}, expected {expected}");
}

#[test]
fn each_call_takes_its_charge_from_the_calls_budget_the_same_in_both_hosts() {
    let root = fresh_root("charges");
    assert_eq!((KIT_CHARGE_CALL, KIT_CHARGE_WRAP, KIT_CHARGE_MEASURE_LINES), (500, 384, 1024));
    let letters: Vec<u8> = (0..2048).map(|k| b'a' + (k % 26) as u8).collect();
    let frame = |w: u64, h: u64, edge: u64| w * h / 32 + h * w.min(2 * edge) * 2;
    for host in HOSTS {
        let h = |name: &str| format!("{}.{}", name.replace('_', "-"), tag(host));
        // Every call: the fixed charge, whatever it is given; the calls with no work of the host's beside it.
        // (and those that are given nothing to work on: no room to write in, no byte to measure)
        for (name, args) in [("level", vec![]), ("metric", vec![12]), ("metric", vec![MAX]), ("clock_ms", vec![]), ("list_page", vec![MAX, 0, MAX, MAX, MAX]), ("clip", vec![0, 0, 10, 10]), ("clip_clear", vec![]), ("hit", vec![1, 0, 0, 10, 10]), ("pager", vec![1, 2, 0, 0, 0, 0, 1]), ("version", vec![8000, 0]), ("measure_width", vec![0, 0, 0]), ("measure_height", vec![0, 0, 0, 300])] {
            let one = call(name, &args);
            let n = charged(host, &h(name), times(2, &one), one.clone(), &letters);
            assert!((500..500 + 96).contains(&n), "{host:?} {name}: {n}");
        }
        // version: and 4 for each byte written (ten calls, so that a few bytes show beside the interpreter's count).
        let v = GRIMOIRE_VERSION.len().min(32) as u64;
        near(charged(host, &h("version"), times(10, &call("version", &[8000, 64])), times(10, &call("version", &[8000, 0])), &letters), 10 * 4 * v, "version");
        near(charged(host, &h("version.3"), times(10, &call("version", &[8000, 3])), times(10, &call("version", &[8000, 0])), &letters), 10 * 12, "version, three bytes");
        // measure_width: 32 for each byte; measure_height: 1,024 for each byte (the text broken into lines, not drawn).
        near(charged(host, &h("width"), call("measure_width", &[0, 100, 0]), call("measure_width", &[0, 0, 0]), &letters), 3200, "measure_width");
        near(charged(host, &h("height"), call("measure_height", &[0, 100, 0, 300]), call("measure_height", &[0, 0, 0, 300]), &letters), 102_400, "measure_height");
        // text: its letters as the app host's `text` (32 a byte a pixel of height to 64 px, the bold, title and caps
        // faces the cube of the height more); cut short: the square of its bytes to 256; WRAP: 384 a byte more;
        // FIT: the square of all its bytes more.
        let text = |w: i32, len: i32, role: i32, flags: i32| call("text", &[0, 0, w, 100, 0, len, role, flags]);
        near(charged(host, &h("text"), text(60_000, 10, 0, 0), text(60_000, 0, 0, 0), &letters), 10 * 44 * 32, "a body text");
        near(charged(host, &h("text.label"), text(60_000, 10, 2, 0), text(60_000, 0, 2, 0), &letters), 10 * 44 * 32 + 10 * 44 * 44 * 44 / 128, "a label, in the bold face");
        let display = if ui_theme() == UiTheme::Gothic { 112 } else { 71 };
        near(charged(host, &h("text.display"), text(60_000, 4, 7, 0), text(60_000, 0, 7, 0), &letters), 4 * display * display / 2 + 4 * display * display * display / 128, "a page title");
        near(charged(host, &h("text.cut"), text(20, 10, 0, 0), text(20, 0, 0, 0), &letters), 10 * 44 * 32 + 16 * 100, "a text cut short");
        near(charged(host, &h("text.cut.long"), text(20, 1000, 1, 0), text(20, 0, 1, 0), &letters), 1000 * 34 * 32 + 16 * 256 * 256, "its bytes to 256 in the square");
        near(charged(host, &h("text.wrap"), text(300, 100, 0, 4), text(300, 0, 0, 4), &letters), 100 * 44 * 32 + 100 * 384, "a text wrapped");
        near(charged(host, &h("text.fit"), text(300, 100, 0, 8), text(300, 0, 0, 8), &letters), 100 * 44 * 32 + 16 * 100 * 100, "a text fitted");
        near(charged(host, &h("text.fit.long"), text(300, 1000, 1, 8), text(300, 0, 1, 8), &letters), 1000 * 34 * 32 + 16 * 1000 * 1000, "all its bytes in the square");
        near(charged(host, &h("lines"), call("text_lines", &[0, 0, 300, 100, 0, 100, 0, 0, 0]), call("text_lines", &[0, 0, 300, 100, 0, 0, 0, 0, 0]), &letters), 100 * 44 * 32 + 100 * 384, "a page of lines");
        // The elements, as the app host's: frames by their pixels and corners, labels at 48 px, icons by their side.
        let row = |w: i32, hh: i32, icon: i32, label: i32, switch: i32| call("row", &[-1, 0, 0, w, hh, 0, icon, 0, label, 0, switch]);
        near(charged(host, &h("row"), row(900, 120, 0, 0, -1), row(1, 1, 0, 0, -1), &letters), frame(900, 120, 32) - frame(1, 1, 32), "a row");
        near(charged(host, &h("row.std"), row(900, 0, 0, 0, -1), row(900, 1, 0, 0, -1), &letters), frame(900, 118, 32) - frame(900, 1, 32), "a row of the standard height");
        near(charged(host, &h("row.switch"), row(900, 120, 0, 0, 1), row(900, 120, 0, 0, -1), &letters), 30_000, "its switch");
        near(charged(host, &h("row.icon"), row(900, 120, 4, 0, -1), row(900, 120, 0, 0, -1), &letters), 64 * 64 * 2 + 16, "its icon and the icon's name");
        near(charged(host, &h("row.label"), row(900, 120, 0, 300, -1), row(900, 120, 0, 0, -1), &letters), 300 * 48 * 32 + 16 * 300 * 300, "its label on up to two lines");
        near(charged(host, &h("row.label.long"), row(900, 120, 0, 1000, -1), row(900, 120, 0, 0, -1), &letters), 1000 * 48 * 32 + 16 * 1000 * 1000, "all its label's bytes in the square");
        let button = |w: i32, hh: i32, label: i32, icon: i32| call("button", &[-1, 0, 0, w, hh, 0, label, 0, icon, 0]);
        near(charged(host, &h("button"), button(400, 120, 0, 0), button(1, 1, 0, 0), &letters), 3 * (frame(400, 120, 32) - frame(1, 1, 32)), "a button");
        near(charged(host, &h("button.label"), button(400, 120, 10, 0), button(400, 120, 0, 0), &letters), 10 * 48 * 32 + 16 * 100, "its label");
        near(charged(host, &h("button.label.long"), button(400, 120, 1000, 0), button(400, 120, 0, 0), &letters), 1000 * 48 * 32 + 16 * 1000 * 1000, "its label on up to two lines");
        near(charged(host, &h("button.icon"), button(400, 120, 0, 4), button(400, 120, 0, 0), &letters), 120 * 120 * 2 + 16, "its icon");
        near(charged(host, &h("icon_button"), call("icon_button", &[-1, 0, 0, 96, 96, 0, 0, 0]), call("icon_button", &[-1, 0, 0, 1, 1, 0, 0, 0]), &letters), 3 * (frame(96, 96, 32) - frame(1, 1, 32)) + 96 * 96 * 2 - 2, "an icon button");
        let chip = |w: i32, label: i32| call("chip", &[-1, 0, 0, w, 80, 0, label, 0]);
        near(charged(host, &h("chip"), chip(260, 0), chip(1, 0), &letters), 2 * frame(260, 80, 72) - 2 * frame(1, 80, 32), "a chip");
        near(charged(host, &h("chip.label"), chip(260, 400), chip(260, 0), &letters), 400 * 48 * 32 + 16 * 256 * 256, "its label");
        let field = |w: i32, icon: i32, text: i32, hint: i32| call("field", &[-1, 0, 0, w, 100, 0, icon, 0, text, 0, hint, 0]);
        near(charged(host, &h("field"), field(900, 0, 0, 0), field(1, 0, 0, 0), &letters), 2 * frame(900, 100, 82) - 2 * frame(1, 100, 32), "a field");
        near(charged(host, &h("field.icon"), field(900, 4, 0, 0), field(900, 0, 0, 0), &letters), 100 * 100 * 2 + 16, "its icon, as tall as the field");
        near(charged(host, &h("field.texts"), field(900, 0, 5, 6), field(900, 0, 0, 0), &letters), 5 * 48 * 32 + 16 * 25 + 6 * 48 * 32 + 16 * 36, "its text and its hint");
        near(charged(host, &h("field.whole"), field(1, 0, 0, 0), Vec::new(), &letters), 500 + 2 * frame(1, 100, 32) + 64 * 64 * 2, "a field: the call, its frames and the mark that clears it");
        let select = |w: i32, caption: i32, value: i32| call("select", &[-1, 0, 0, w, 100, 0, caption, 0, value]);
        near(charged(host, &h("select"), select(660, 0, 0), select(1, 0, 0), &letters), 2 * frame(660, 100, 82) - 2 * frame(1, 100, 32), "a select");
        near(charged(host, &h("select.texts"), select(660, 4, 6), select(660, 0, 0), &letters), 4 * 48 * 32 + 16 * 16 + 6 * 48 * 32 + 16 * 36, "its caption and its value");
        near(charged(host, &h("select.whole"), select(1, 0, 0), Vec::new(), &letters), 500 + 2 * frame(1, 100, 32) + 64 * 64 * 2, "a select: the call, its frames and its chevron");
        let check = |w: i32, label: i32| call("checkbox", &[-1, 0, 0, w, 90, 0, label, 0]);
        near(charged(host, &h("checkbox"), check(900, 0), check(1, 0), &letters), frame(900, 90, 0) - frame(1, 90, 0), "a checkbox");
        near(charged(host, &h("checkbox.label"), check(900, 20), check(900, 0), &letters), 20 * 48 * 32 + 16 * 400, "its label");
        near(charged(host, &h("checkbox.label.long"), check(900, 1000), check(900, 0), &letters), 1000 * 48 * 32 + 16 * 1000 * 1000, "its label on up to two lines");
        near(charged(host, &h("checkbox.whole"), check(1, 0), Vec::new(), &letters), 500 + frame(1, 90, 0) + 2 * frame(64, 64, 64) + 64 * 64 * 2, "a checkbox: the call, its rectangle, its box and its mark");
        near(charged(host, &h("card"), call("card", &[0, 0, 500, 200, 0]), call("card", &[0, 0, 1, 1, 0]), &letters), frame(500, 200, 32) - frame(1, 1, 32), "a card");
        near(charged(host, &h("divider"), call("divider", &[0, 0, 900]), call("divider", &[0, 0, 1]), &letters), frame(900, 10, 0) - frame(1, 10, 0), "a divider: the room of the theme's rule");
        near(charged(host, &h("icon"), call("icon", &[0, 0, 96, 96, 0, 4, 0]), call("icon", &[0, 0, 1, 1, 0, 0, 0]), &letters), 96 * 96 * 2 - 2 + 16, "an icon and its name");
        near(charged(host, &h("icon.most"), call("icon", &[0, 0, 4000, 4000, 0, 0, 0]), call("icon", &[0, 0, 1, 1, 0, 0, 0]), &letters), 1024 * 1024 * 2 - 2, "an icon is 1,024 px at most");
        near(charged(host, &h("icon.name"), call("icon", &[0, 0, 96, 96, 0, 64, 0]), call("icon", &[0, 0, 96, 96, 0, 0, 0]), &letters), 64 * 4, "an icon's name of 64 bytes");
        near(charged(host, &h("icon.name.long"), call("icon", &[0, 0, 96, 96, 0, 65, 0]), call("icon", &[0, 0, 96, 96, 0, 0, 0]), &letters), 0, "a name longer than any is not read");
        near(charged(host, &h("fill"), call("fill", &[10, 10, 320, 200, 0]), call("fill", &[10, 10, 1, 0, 0]), &letters), 320 * 200 / 32, "a fill");
        near(charged(host, &h("fill.part"), call("fill", &[-50, -30, 320, 200, 0]), call("fill", &[10, 10, 1, 0, 0]), &letters), 270 * 170 / 32, "the part of it inside the body");
        let radius = ui_card_radius() as u64 + 2;
        near(charged(host, &h("frame"), call("frame", &[0, 0, 400, 300, 0, 1, 0]), call("frame", &[0, 0, 1, 0, 0, 1, 0]), &letters), frame(400, 300, radius), "a filled frame");
        near(charged(host, &h("frame.thick"), call("frame", &[0, 0, 400, 300, 0, 0, 40]), call("frame", &[0, 0, 1, 0, 0, 0, 40]), &letters), frame(400, 300, 42), "an outline 40 thick");
        near(charged(host, &h("line"), line(0.0, 100.0, 400.0, 100.0, 4.0, 0), line(-500.0, -500.0, -400.0, -500.0, 4.0, 0), &letters), 7 * (16 + 404 * 2), "a level line 4 wide");
        // (the pager: two buttons of three frames and an icon each, and its count)
        let arrow = 3 * frame(130, 90, 32) + 90 * 90 * 2;
        let count = drawn_text_charge(3, ui_small_size(), FontFace::Fixed);
        near(charged(host, &h("pager"), call("pager", &[1, 2, 0, 0, 1000, 1, 3]), call("pager", &[1, 2, 0, 0, 1000, 0, 1]), &letters), 2 * arrow + count, "a pager");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_call_at_a_limit_runs_and_one_past_it_stops_as_out_of_instructions() {
    let root = fresh_root("limits");
    let letters: Vec<u8> = (0..70_000).map(|k| b'a' + (k % 26) as u8).collect();
    let module = |host: Host, code: Vec<u8>| {
        let mut k = Kit::new(host, code);
        k.pages = 2;
        k.data = vec![(0, letters.clone())];
        k
    };
    for host in HOSTS {
        let commands = match host {
            Host::Widget => WIDGET_PASS_COMMANDS_MAX,
            Host::App => APP_CALL_COMMANDS_MAX,
        } as i32;
        let hits = match host {
            Host::Widget => WIDGET_PASS_HITS_MAX,
            Host::App => APP_CALL_HITS_MAX,
        } as i32;
        let (one, all) = match host {
            Host::Widget => (WIDGET_TEXT_MAX as i32, WIDGET_PASS_TEXT_MAX as i32),
            Host::App => (APP_TEXT_MAX as i32, APP_CALL_TEXT_MAX as i32),
        };
        let cases: Vec<(&str, Vec<u8>, Vec<u8>)> = vec![
            ("the drawing commands", repeat(commands, &call("fill", &[0, 0, 1, 1, 0])), repeat(commands + 1, &call("fill", &[0, 0, 1, 1, 0]))),
            ("the hit regions", repeat(hits, &call("hit", &[1, 0, 0, 1, 1])), repeat(hits + 1, &call("hit", &[1, 0, 0, 1, 1]))),
            ("the hit regions of the elements", repeat(hits, &call("card", &[0, 0, 1, 1, 0]).into_iter().chain(call("hit", &[1, 0, 0, 1, 1])).collect::<Vec<_>>()), repeat(hits + 1, &call("chip", &[1, 0, 0, 1, 1, 0, 0, 0]))),
            // (a pager has two regions: one region and as many pagers again go one past at the last one's second; the
            // pagers are above the body, where their frames take nothing from the budget by their pixels)
            ("the hit regions of pagers", repeat(hits / 2, &call("pager", &[1, 2, 0, -1000, 300, 1, 3])), seq(&[call("hit", &[1, 0, 0, 1, 1]), repeat(hits / 2, &call("pager", &[1, 2, 0, -1000, 300, 1, 3]))])),
            // (and a pager on its last page, whose one region is its previous page's arrow)
            ("the hit region of a pager's previous arrow", seq(&[repeat(hits - 1, &call("hit", &[1, 0, 0, 1, 1])), call("pager", &[1, 2, 0, -1000, 300, 2, 3])]), seq(&[repeat(hits, &call("hit", &[1, 0, 0, 1, 1])), call("pager", &[1, 2, 0, -1000, 300, 2, 3])])),
            ("the bytes of one text", call("text", &[0, 0, 100, 40, 0, one, 0, 0]), call("text", &[0, 0, 100, 40, 0, one + 1, 0, 0])),
            ("the bytes of one measured text", call("measure_width", &[0, one, 0]), call("measure_width", &[0, one + 1, 0])),
            ("the bytes of a label", call("row", &[-1, 0, 0, 10, 10, 0, 0, 0, one, 0, -1]), call("row", &[-1, 0, 0, 10, 10, 0, 0, 0, one + 1, 0, -1])),
            ("the bytes of a hint", call("field", &[-1, 0, 0, 10, 10, 0, 0, 0, 0, 0, one, 0]), call("field", &[-1, 0, 0, 10, 10, 0, 0, 0, 0, 0, one + 1, 0])),
            ("the bytes of the texts of a call", repeat(all / 1000, &call("measure_width", &[0, 1000, 0])), repeat(all / 1000 + 1, &call("measure_width", &[0, 1000, 0]))),
        ];
        // (each element given an id needs room for its region: one more than the regions there is room for stops
        // the call, whatever kind it is)
        let mut cases = cases;
        for (kind, element) in [
            ("button", call("button", &[1, 0, 0, 10, 10, 0, 0, 0, 0, 0])),
            ("icon_button", call("icon_button", &[1, 0, 0, 10, 10, 0, 0, 0])),
            ("row", call("row", &[1, 0, 0, 10, 10, 0, 0, 0, 0, 0, -1])),
            ("chip", call("chip", &[1, 0, 0, 10, 10, 0, 0, 0])),
            ("field", call("field", &[1, 0, 0, 10, 10, 0, 0, 0, 0, 0, 0, 0])),
            ("select", call("select", &[1, 0, 0, 10, 10, 0, 0, 0, 0])),
            ("checkbox", call("checkbox", &[1, 0, 0, 10, 10, 0, 0, 0])),
            ("pager", call("pager", &[-1, 2, 0, -1000, 300, 0, 3])),
        ] {
            let what: &'static str = Box::leak(format!("the hit region of a {kind}").into_boxed_str());
            cases.push((what, seq(&[repeat(hits - 1, &call("hit", &[1, 0, 0, 1, 1])), element.clone()]), seq(&[repeat(hits, &call("hit", &[1, 0, 0, 1, 1])), element])));
        }
        for (what, at, past) in cases {
            let mut rt = loaded(&format!("test.kit.limit.at.{}.{}", tag(host), what.len()), &module(host, at));
            let before = patterned();
            let mut c = before.clone();
            rt.draw(&mut c);
            assert_eq!(rt.error(), None, "{host:?} at {what}");
            let mut rt = loaded(&format!("test.kit.limit.past.{}.{}", tag(host), what.len()), &module(host, past));
            let mut c = before.clone();
            rt.draw(&mut c);
            assert_eq!(rt.error(), Some(rt.stopped()), "{host:?} past {what}: stopped as a call out of instructions is");
            assert_eq!(changed_outside(&before, &c, RECT_EMPTY), 0, "{host:?} past {what}: nothing of the pass is drawn");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_length_is_compared_with_its_limit_before_the_modules_memory_is_read() {
    let root = fresh_root("reads");
    // The bytes the host read of the module's memory in the pass, for calls given lengths at and past their limits:
    // a text past its limit stops the call before a byte is read; an icon's name past 64 bytes is not read (it is
    // drawn as a name of no icon is); a version is written, never read.
    let data: Vec<u8> = (0..4096).map(|k| b'a' + (k % 26) as u8).collect();
    for host in HOSTS {
        let one = match host {
            Host::Widget => WIDGET_TEXT_MAX,
            Host::App => APP_TEXT_MAX,
        } as i32;
        let cases: Vec<(&str, Vec<u8>, usize, bool)> = vec![
            ("a text at the limit", call("text", &[0, 0, 100, 40, 0, one, 0, 0]), one as usize, true),
            ("a text past it", call("text", &[0, 0, 100, 40, 0, one + 1, 0, 0]), 0, false),
            ("a text of i32::MAX bytes", call("text", &[0, 0, 100, 40, 0, MAX, 0, 0]), 0, false),
            ("a negative length", call("text", &[0, 0, 100, 40, 0, -5, 0, 0]), 0, true),
            ("a text past the memory's end", call("text", &[0, 0, 100, 40, 2 * 65536 - 10, 100, 0, 0]), 10, true),
            ("an icon's name of 64 bytes", call("icon", &[0, 0, 60, 60, 0, 64, 0]), 64, true),
            ("an icon's name of 65 bytes", call("icon", &[0, 0, 60, 60, 0, 65, 0]), 0, true),
            ("a row's icon past 64 and its label", call("row", &[1, 0, 0, 0, 0, 0, 65, 0, 10, 0, -1]), 10, true),
            ("a version", call("version", &[100, 64]), 0, true),
            ("a measured height past the limit", call("measure_height", &[0, one + 1, 0, 0]), 0, false),
            ("a page of lines past the limit", call("text_lines", &[0, 0, 100, 100, 0, one + 1, 0, 0, 0]), 0, false),
        ];
        for (k, (what, code, read, runs)) in cases.into_iter().enumerate() {
            let mut kit = Kit::new(host, code);
            kit.pages = 2;
            kit.data = vec![(0, data.clone())];
            let mut rt = loaded(&format!("test.kit.reads.{}.{k}", tag(host)), &kit);
            let _ = drawn(&mut rt);
            assert_eq!(rt.error().is_none(), runs, "{host:?}: {what}: {:?}", rt.error());
            assert_eq!(rt.read(), read, "{host:?}: {what}");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn version_writes_no_more_than_the_room_given_and_the_memory_holds() {
    let root = fresh_root("version");
    let v = GRIMOIRE_VERSION.as_bytes();
    for host in HOSTS {
        // (a page of memory filled with '#'; the version written at each place with each room, then the page said)
        for (k, (ptr, cap)) in [(100, 64), (100, v.len() as i32), (100, v.len() as i32 - 1), (100, 1), (100, 0), (100, -1), (65536 - 3, 64), (65536, 64), (-1, 64), (MAX, 64), (MIN, 64)].into_iter().enumerate() {
            let code = seq(&[kept("version", &[ptr, cap]), vec![DROP], host_status(90, 200)]);
            let mut kit = Kit::new(host, code);
            kit.data = vec![(0, vec![b'#'; 65536])];
            let mut rt = loaded(&format!("test.kit.version.{}.{k}", tag(host)), &kit);
            let _ = drawn(&mut rt);
            assert_eq!(rt.error(), None);
            let status = rt.status();
            let n = if ptr < 0 || cap <= 0 || ptr >= 65536 { 0 } else { v.len().min(cap as usize).min((65536 - ptr) as usize) };
            if (90..290).contains(&ptr) {
                let at = (ptr - 90) as usize;
                assert_eq!(&status.as_bytes()[at..at + n], &v[..n], "{host:?} {ptr} {cap}");
                assert!(status.as_bytes()[..at].iter().chain(&status.as_bytes()[at + n..]).all(|&b| b == b'#'), "{host:?} {ptr} {cap}: {status}");
            } else {
                assert!(status.bytes().all(|b| b == b'#'), "{host:?} {ptr} {cap}");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Passes that reach far outside the body, each one element at one place, with numbers no sum may overflow with
/// (each a pass of its own: two elements with round ends over the whole body are more than a budget pays for).
fn reaching() -> Vec<Vec<K>> {
    let mut out = Vec::new();
    for (x, y, w, h) in [(-5000, -5000, 20_000, 20_000), (MIN, MIN, MAX, MAX), (MAX, MAX, MAX, MAX), (-1, -1, MAX, MAX), (MIN, 0, MAX, 40), (0, 0, MIN, MIN), (MAX - 5, 10, 0, 0), (MIN, MIN, 0, 0)] {
        out.extend([
            K::Fill(x, y, w, h, 0),
            K::Frame(x, y, w, h, 0, 0, MAX),
            K::Row(1, x, y, w, h, "gear", "a row", 1, 1),
            K::Button(2, x, y, w, h, "button", "files", 1),
            K::IconButton(3, x, y, w, h, "search", 1),
            K::Chip(4, x, y, w, h, "chip", 1),
            K::Field(5, x, y, w, h, "search", "text", "hint", 0),
            K::Select(6, x, y, w, h, "a", "b"),
            K::Checkbox(7, x, y, w, h, "check", 1),
            K::Card(x, y, w, h, 1),
            K::Divider(x, y, w),
            K::Icon(x, y, w, h, "gear", 0),
            K::Text(x, y, w, h, "text", 7, 4),
            K::Text(x, y, w, h, "a text that is fitted", 2, 8),
            K::Lines(x, y, w, h, "one two three four", 0, 2, MAX),
            K::Pager(8, 9, x, y, w, MAX, MAX),
            K::Hit(10, x, y, w, h),
        ].map(|k| vec![k]));
        out.push(vec![K::Clip(x, y, w, h), K::Fill(-10, -10, 4000, 4000, 0), K::ClipClear, K::Fill(x, y, w, h, 0)]);
    }
    out.push(vec![K::Line(-1e9, -1e9, 1e9, 1e9, 1e30, 0)]);
    out.push(vec![K::Line(f32::NAN, 0.0, 100.0, 100.0, 3.0, 0)]);
    out
}

#[test]
fn whatever_the_kit_draws_lands_in_the_body_and_its_hit_regions_are_cut_to_it() {
    let root = fresh_root("body");
    let before = patterned();
    for host in HOSTS {
        let mut drew = 0;
        for (k, cmds) in reaching().into_iter().enumerate() {
            let mut rt = loaded(&format!("test.kit.body.{}.{k}", tag(host)), &pass(host, &cmds));
            let mut c = before.clone();
            rt.draw(&mut c);
            assert_eq!(rt.error(), None, "{host:?} {cmds:?}");
            assert_eq!(changed_outside(&before, &c, rt.body()), 0, "{host:?} {cmds:?}: nothing outside the body");
            drew += inked(&c, rt.body());
            let body = rt.body();
            for id in 1..=10 {
                if let Some(r) = rt.hit(id) {
                    assert!(r.x >= 0 && r.y >= 0 && r.w > 0 && r.h > 0 && r.x + r.w <= body.w && r.y + r.h <= body.h, "{host:?}: hit {id} {r:?} is in the body");
                }
            }
        }
        assert!(drew > 1_000_000, "{host:?}: the passes drew");
        // A clip holds what is drawn after it to its rectangle, cut to the body: a fill of the whole screen after a
        // clip whose part inside the body is 10 px each way.
        let mut rt = loaded(&format!("test.kit.clip.{}", tag(host)), &pass(host, &[K::Clip(-20, -20, 30, 30), K::Fill(-5000, -5000, 20_000, 20_000, 0), K::ClipClear]));
        let mut c = before.clone();
        rt.draw(&mut c);
        assert_eq!(inked(&c, rt.body()), 100, "{host:?}: the clip's part inside the body");
        assert_eq!(changed_outside(&before, &c, rt.body()), 0);
        // And the canvas keeps the clip it had.
        assert_eq!(c.clip, before.clip);
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_widget_that_may_not_draw_draws_nothing_and_has_no_hit_region() {
    let root = fresh_root("overlay");
    let mut rt = load("test.kit.bare", &pass(Host::Widget, &page()), false).unwrap();
    let before = patterned();
    let mut c = before.clone();
    rt.draw(&mut c);
    assert_eq!(rt.error(), None);
    assert_eq!(inked(&c, WIDGET_BODY), 0, "nothing is drawn");
    assert_eq!((rt.hits(), rt.text().as_str()), (0, ""));
    // (and its drawing calls return 0, while the calls that draw nothing answer as for any widget)
    let mut t = Texts::default();
    let [lp, ln] = t.put("label");
    let code = vec![kept("row", &[1, 0, 0, 0, 0, 0, 0, lp, ln, 0, -1]), kept("button", &[1, 0, 0, 0, 0, lp, ln, 0, 0, 0]), kept("text", &[0, 0, 0, 0, lp, ln, 0, 0]), kept("level", &[]), kept("metric", &[10])];
    let kit = saying(Host::Widget, code, t);
    let mut rt = load("test.kit.bare.returns", &kit, false).unwrap();
    // (its status line is shown: `status` needs no capability)
    let _ = drawn(&mut rt);
    assert_eq!(rt.error(), None);
    let got: Vec<i64> = rt.status().split(' ').map(|d| d.parse::<i64>().unwrap() - 1_000_000_000).collect();
    assert_eq!(got, vec![0, 0, 0, 1, 118]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_widget_that_may_not_draw_pays_the_fixed_charge_for_each_drawing_call() {
    let root = fresh_root("barecharge");
    let letters: Vec<u8> = (0..2048).map(|k| b'a' + (k % 26) as u8).collect();
    // (the fuel of a pass of two calls against a pass of one, in a widget without `editor:overlay`)
    let bare = |id: &str, code: Vec<u8>| {
        let mut kit = Kit::new(Host::Widget, code);
        kit.pages = 2;
        kit.data = vec![(0, letters.clone())];
        let mut rt = load(&format!("test.kit.barecharge.{id}"), &kit, false).unwrap();
        let _ = drawn(&mut rt);
        let _ = drawn(&mut rt);
        assert_eq!(rt.error(), None, "{id}");
        rt.fuel()
    };
    for (name, args) in [
        ("fill", vec![0, 0, 900, 900, 0]),
        ("row", vec![1, 0, 0, 900, 0, 0, 4, 0, 100, 0, 1]),
        ("text", vec![0, 0, 300, 100, 0, 100, 0, 8]),
        ("clip", vec![0, 0, 10, 10]),
        ("hit", vec![1, 0, 0, 10, 10]),
        ("pager", vec![1, 2, 0, 0, 1000, 1, 3]),
    ] {
        let one = call(name, &args);
        let n = bare(&format!("{name}.2"), times(2, &one)).saturating_sub(bare(&format!("{name}.1"), one));
        assert!((500..500 + 96).contains(&n), "{name}: {n}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_callers_clip_smaller_than_the_body_is_kept() {
    let root = fresh_root("callerclip");
    // The editor draws a frame clipped to the part of the screen it redraws: nothing the kit draws, clip_clear and
    // a clip of everything among it, reaches outside that part, and the canvas has its clip again after the pass.
    for (host, own) in [(Host::App, ("grimoire", "fill_rect", "iiiii", 'v')), (Host::Widget, ("grimoire", "rect", "iiiii", 'v'))] {
        let mut k = Kit::new(host, Vec::new());
        k.imports.push(own);
        k.code[0] = seq(&[
            call("clip_clear", &[]),
            call("fill", &[-100, -100, 100_000, 100_000, 0]),
            call("clip", &[-100, -100, 100_000, 100_000]),
            call("fill", &[-100, -100, 100_000, 100_000, 30]),
            extra(&k, 0, &[-100, -100, 100_000, 100_000, 1]),
            call("row", &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1]),
        ]);
        let mut rt = loaded(&format!("test.kit.callerclip.{}", tag(host)), &k);
        let body = rt.body();
        let part = Rect::new(body.x + 100, body.y + 100, 300, 200);
        let before = patterned();
        let mut c = before.clone();
        c.clip = part;
        rt.draw(&mut c);
        assert_eq!(rt.error(), None);
        assert_eq!(changed_outside(&before, &c, part), 0, "{host:?}: nothing outside the caller's clip");
        assert!(inked(&c, part) > 0, "{host:?}: the part is drawn on");
        assert_eq!(c.clip, part, "{host:?}: the caller's clip after the pass");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_apps_hit_regions_are_those_of_its_last_drawing_pass() {
    let root = fresh_root("apphits");
    // (a touch that makes regions of its own: they are not the next pass's)
    let kit = Kit::new(Host::App, call("hit", &[1, 0, 0, 100, 100])).touch(times(3, &call("hit", &[2, 200, 200, 100, 100])));
    let Rt::A(mut rt) = loaded("test.kit.apphits", &kit) else { unreachable!() };
    let mut c = patterned();
    rt.draw(&mut c, app_body(), 0);
    rt.pointer(0, 10, 10, 0);
    rt.draw(&mut c, app_body(), 0);
    assert_eq!(rt.error(), None);
    assert_eq!(rt.hits(), &[(1, Rect::new(0, 0, 100, 100))]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn wrapped_text_centres_its_block_and_cuts_its_last_line_short_and_a_right_text_too_wide_is_drawn_as_left() {
    let root = fresh_root("textrules");
    for host in HOSTS {
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            let lh = line_of(0);
            let (f, px) = role(0);
            let one = |id: &str, cmds: &[K]| {
                let mut rt = loaded(&format!("test.kit.textrules.{}.{}.{id}", tag(host), look(theme)), &pass(host, cmds));
                let c = drawn(&mut rt);
                assert_eq!(rt.error(), None);
                (c, rt.body())
            };
            let own = |body: Rect, draw: &dyn Fn(&mut Canvas, Rect)| {
                let mut c = patterned();
                let whole = c.clip;
                canvas_set_clip(&mut c, rect_intersect(whole, body));
                canvas_fill(&mut c, body, 255);
                draw(&mut c, body);
                c.clip = whole;
                c
            };
            // Three lines in a room for six: the block in its middle.
            let (got, body) = one("centred", &[K::Text(20, 100, 600, 6 * lh, "First line\nSecond line\nThird", 0, 4)]);
            let want = own(body, &|c, b| {
                for (k, s) in ["First line", "Second line", "Third"].iter().enumerate() {
                    font_draw_left_name(c, f, px, Rect::new(b.x + 20, b.y + 100 + (6 * lh - 3 * lh) / 2 + k as i32 * lh, 600, lh), s, 0);
                }
            });
            assert!(got.px == want.px, "{host:?} {theme:?}: the block of lines is centred in its room");
            // Four lines in a room for two: the first, then the rest on the second, cut short.
            let (got, body) = one("cut", &[K::Text(20, 100, 260, 2 * lh, "alpha\nbravo charlie delta echo\nfoxtrot\ngolf", 0, 4)]);
            let want = own(body, &|c, b| {
                font_draw_left_name(c, f, px, Rect::new(b.x + 20, b.y + 100, 260, lh), "alpha", 0);
                let lines = kit_lines(f, px, "alpha\nbravo charlie delta echo\nfoxtrot\ngolf", 260);
                font_draw_left_name(c, f, px, Rect::new(b.x + 20, b.y + 100 + lh, 260, lh), &lines[1..].join(" "), 0);
            });
            assert!(got.px == want.px, "{host:?} {theme:?}: the last line shown holds the rest, cut short");
            // A right-aligned text too wide for its room: as a left-aligned one.
            let (right, _) = one("right", &[K::Text(20, 100, 120, 0, "A text far wider than its room", 0, 2)]);
            let (left, _) = one("left", &[K::Text(20, 100, 120, 0, "A text far wider than its room", 0, 0)]);
            assert!(right.px == left.px, "{host:?} {theme:?}");
        }
    }
    ui_set_theme(UiTheme::Gothic);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn text_lines_given_no_height_takes_the_rest_of_the_body_and_a_pager_and_hit_make_only_the_regions_they_say() {
    let root = fresh_root("lineshits");
    for host in HOSTS {
        let lh = line_of(0);
        let body = match host {
            Host::Widget => WIDGET_BODY,
            Host::App => app_body(),
        };
        // (three lines from 4 lines above the body's foot: all three drawn, as with a height of three lines)
        let y = body.h - 4 * lh;
        let mut a = loaded(&format!("test.kit.lineshits.a.{}", tag(host)), &pass(host, &[K::Lines(20, y, 600, 0, "one\ntwo\nthree", 0, 0, 0)]));
        let mut b = loaded(&format!("test.kit.lineshits.b.{}", tag(host)), &pass(host, &[K::Lines(20, y, 600, 3 * lh, "one\ntwo\nthree", 0, 0, 0)]));
        let (ca, cb) = (drawn(&mut a), drawn(&mut b));
        assert!(inked(&ca, body) > 0 && ca.px == cb.px, "{host:?}");
        // A pager on its first page has no region for the previous page's arrow; `hit` with id -1 makes none.
        let mut rt = loaded(&format!("test.kit.lineshits.p.{}", tag(host)), &pass(host, &[K::Pager(8, 9, 20, 100, 1100, 0, 3), K::Hit(-1, 0, 0, 100, 100)]));
        let _ = drawn(&mut rt);
        assert_eq!((rt.hits(), rt.hit(8), rt.hit(9).is_some()), (1, None, true), "{host:?}");
    }
    // (a line break with a carriage return before it is a line break)
    assert_eq!(kit_lines(FontFace::Regular, 44.0, "one\r\ntwo\r\n", 600), vec!["one", "two", ""]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_button_with_an_icon_or_a_label_alone_is_grimoires_own_in_gothic() {
    let root = fresh_root("onebutton");
    ui_set_theme(UiTheme::Gothic);
    // An icon alone: the carved icon button of the Package Store's header and pager (gui_icon_button). A label alone:
    // the carved button of Grimoire's dialogs (ui_button). Neither is the Store's action button, whose label is beside
    // its icon.
    for host in HOSTS {
        let mut rt = loaded(&format!("test.kit.onebutton.{}", tag(host)), &pass(host, &[K::Button(-1, 20, 100, 110, 76, "", "back", 0), K::Button(-1, 20, 300, 400, 96, "Cancel", "", 0)]));
        let got = drawn(&mut rt);
        let body = rt.body();
        let mut want = patterned();
        let whole = want.clip;
        canvas_set_clip(&mut want, rect_intersect(whole, body));
        canvas_fill(&mut want, body, 255);
        gui_icon_button(&mut want, Rect::new(body.x + 20, body.y + 100, 110, 76), widget_icon("back"), ButtonStyle::Plain);
        ui_button(&mut want, Rect::new(body.x + 20, body.y + 300, 400, 96), IconKind::None, "Cancel", ButtonStyle::Plain);
        want.clip = whole;
        assert!(got.px == want.px, "{host:?}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_unknown_role_style_or_tone_is_the_first_one() {
    let root = fresh_root("unknown");
    for host in HOSTS {
        let one = |id: &str, cmds: &[K]| {
            let mut rt = loaded(&format!("test.kit.unknown.{}.{id}", tag(host)), &pass(host, cmds));
            drawn(&mut rt)
        };
        for (what, a, b) in [
            ("a role", vec![K::Text(10, 10, 600, 0, "Words", 99, 0)], vec![K::Text(10, 10, 600, 0, "Words", 0, 0)]),
            ("a negative role", vec![K::Text(10, 10, 600, 0, "Words", -3, 0)], vec![K::Text(10, 10, 600, 0, "Words", 0, 0)]),
            ("an alignment", vec![K::Text(10, 10, 600, 0, "Words", 0, 3)], vec![K::Text(10, 10, 600, 0, "Words", 0, 0)]),
            ("a style", vec![K::Row(1, 10, 10, 600, 0, "gear", "Row", 7, -1)], vec![K::Row(1, 10, 10, 600, 0, "gear", "Row", 0, -1)]),
            ("a switch", vec![K::Row(1, 10, 10, 600, 0, "gear", "Row", 0, -9)], vec![K::Row(1, 10, 10, 600, 0, "gear", "Row", 0, -1)]),
            ("a tone", vec![K::Icon(10, 10, 0, 0, "gear", 9)], vec![K::Icon(10, 10, 0, 0, "gear", 0)]),
            ("an icon", vec![K::Icon(10, 10, 0, 0, "no such icon", 0)], vec![K::Icon(10, 10, 0, 0, "grid", 0)]),
        ] {
            assert!(one(&format!("{}a", what.len()), &a).px == one(&format!("{}b", what.len()), &b).px, "{host:?}: {what}");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_pager_counts_at_most_10000_pages() {
    let root = fresh_root("pages");
    for host in HOSTS {
        let one = |id: &str, page: i32, pages: i32| {
            let mut rt = loaded(&format!("test.kit.pages.{}.{id}", tag(host)), &pass(host, &[K::Pager(8, 9, 20, 100, 1100, page, pages)]));
            drawn(&mut rt)
        };
        let most = one("most", 9999, 10_000);
        assert!(one("more", 15_000, 20_000).px == most.px, "{host:?}: 20,000 pages are counted as 10,000, the page as the last");
        assert!(one("max", MAX, MAX).px == most.px, "{host:?}");
        assert!(one("fewer", 0, 3).px != most.px, "{host:?}: (a count that shows)");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_module_that_imports_a_call_of_a_later_level_is_not_started_and_says_which() {
    let root = fresh_root("later");
    for host in HOSTS {
        let e = load(&format!("test.kit.later.{}", tag(host)), &later(host), true).err();
        let who = match host {
            Host::Widget => "This widget",
            Host::App => "This app",
        };
        assert_eq!(e, Some(format!("{who} needs a newer Grimoire (grimoire_kit.later_call).")), "{host:?}");
        // (a module that imports every call of level 1 starts)
        assert!(load(&format!("test.kit.all.{}", tag(host)), &Kit::new(host, Vec::new()), true).is_ok());
    }
    // (the calls the host links, with their signatures, are the ones the fixtures' writer imports)
    assert_eq!(KIT_CALLS, KIT_IMPORTS);
    let _ = std::fs::remove_dir_all(&root);
}

/// A module that imports, besides every call of level 1, `name` from `module`.
fn importing(host: Host, module: &'static str, name: &str) -> Kit {
    let mut k = Kit::new(host, Vec::new());
    k.imports.push((module, Box::leak(name.to_string().into_boxed_str()), "", 'v'));
    k
}

#[test]
fn the_refusal_names_a_call_only_when_the_name_has_a_calls_shape() {
    let root = fresh_root("shape");
    let long = "x".repeat(99_000);
    let names: [(&str, bool); 9] = [
        ("later_call", true),
        ("level_2_call", true),
        (&"a".repeat(64), true),
        (&"a".repeat(65), false),
        ("Row", false),
        ("row.x", false),
        ("row).\nUpdate (\u{202e}", false),
        ("", false),
        (&long, false),
    ];
    for host in HOSTS {
        let who = match host {
            Host::Widget => "This widget",
            Host::App => "This app",
        };
        for (k, (name, shaped)) in names.iter().enumerate() {
            let e = load(&format!("test.kit.shape.{}.{k}", tag(host)), &importing(host, "grimoire_kit", name), true).err();
            let call = if *shaped { format!("grimoire_kit.{name}") } else { "grimoire_kit".to_string() };
            assert_eq!(e, Some(format!("{who} needs a newer Grimoire ({call}).")), "{host:?}: a name of {} bytes", name.len());
        }
    }
    assert_eq!(KIT_CALL_NAME_MAX, 64);
    assert!(KIT_CALLS.iter().all(|c| kit_call_shaped(c.0)), "every call of level 1 has a call's shape");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_text_a_widgets_frame_shows_and_an_apps_toast_are_at_most_256_bytes() {
    let root = fresh_root("shown");
    assert_eq!(MESSAGE_SHOWN_MAX, 256);
    let long = "x".repeat(99_000);
    for module in ["grimoire_kit", "grimoire"] {
        // A widget's frame: what it says in place of its body, through the frame as the editor draws it.
        let id = format!("test.kit.shown.w.{}", module.len());
        let bytes = importing(Host::Widget, module, &long).bytes();
        widget_package_install(&widget_manifest(&id, "Test", &WIDGET_BOTH, 400, 300, &bytes), &bytes, "", SCREEN.0, SCREEN.1).unwrap();
        widget_catalog_invalidate();
        let mut frame = WidgetFrame::new(widget_def(&id).unwrap());
        frame.ensure_runtime();
        let (reason, whose) = frame.says().unwrap();
        assert!(reason.len() <= 256 && frame.error().unwrap().len() <= 256, "{module}: {} bytes", reason.len());
        assert_eq!(whose, "Test v1.0.0 · Grimoire tests");
        if module == "grimoire_kit" {
            assert_eq!(reason, "This widget needs a newer Grimoire (grimoire_kit).");
        } else {
            assert!(reason.starts_with("The widget could not start: ") && reason.len() > 200, "{module}: {reason}");
        }
        widget_package_remove(&id, false).unwrap();
        // An app: the toast that says why it did not open.
        let id = format!("test.kit.shown.a.{}", module.len());
        let bytes = importing(Host::App, module, &long).bytes();
        app_package_install(&app_manifest(&id, "Test", &[], true, &bytes), &bytes, None, "", SCREEN.0, SCREEN.1).unwrap();
        let mut a = App { w: SCREEN.0, h: SCREEN.1, ..App::default() };
        assert!(!apphost_launch(&mut a, &id), "{module}");
        let toast = a.toast.as_str().to_string();
        assert!(toast.len() <= 256, "{module}: {} bytes", toast.len());
        if module == "grimoire_kit" {
            assert_eq!(toast, "This app needs a newer Grimoire (grimoire_kit).");
        } else {
            assert!(toast.starts_with("The app could not start: ") && toast.len() > 200, "{module}: {toast}");
        }
        app_package_remove(&id, false).unwrap();
    }
    // Any reason a frame keeps is cut between two characters, and so is whose widget it is.
    let id = "test.kit.shown.name";
    let bytes = Kit::new(Host::Widget, Vec::new()).bytes();
    widget_package_install(&widget_manifest(id, &"N".repeat(1000), &WIDGET_BOTH, 400, 300, &bytes), &bytes, "", SCREEN.0, SCREEN.1).unwrap();
    widget_catalog_invalidate();
    let mut frame = WidgetFrame::new(widget_def(id).unwrap());
    frame.runtime_error = Some("\u{20ac}".repeat(200));
    let (reason, whose) = frame.says().unwrap();
    assert_eq!(reason, "\u{20ac}".repeat(85), "the most whole characters in 256 bytes");
    assert_eq!(whose, "N".repeat(256));
    widget_package_remove(id, false).unwrap();
    widget_catalog_invalidate();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_kits_commands_and_the_hosts_own_are_drawn_in_the_order_called() {
    let root = fresh_root("order");
    // The host's own call filling the whole body gray (an app's fill_rect, a widget's selected rect), then a white
    // fill of the kit's: the kit's shows; the other way round, it does not.
    let kit_row = K::Fill(100, 100, 800, 118, 255);
    for (host, own, own_args) in [(Host::App, ("grimoire", "fill_rect", "iiiii", 'v'), [-10, -10, 4000, 4000, 128]), (Host::Widget, ("grimoire", "rect", "iiiii", 'v'), [-10, -10, 4000, 4000, 1])] {
        let mut t = Texts::default();
        let row = command(&kit_row, &mut t);
        let mut first = Kit::new(host, Vec::new());
        first.imports.push(own);
        let black = extra(&first, 0, &own_args);
        let make = |code: Vec<u8>| {
            let mut k = first.clone();
            k.code[0] = code;
            k.data = vec![(TEXTS_AT as u32, t.0.clone())];
            k
        };
        let mut over = loaded(&format!("test.kit.order.over.{}", tag(host)), &make(seq(&[black.clone(), row.clone()])));
        let mut under = loaded(&format!("test.kit.order.under.{}", tag(host)), &make(seq(&[row, black])));
        let r = Rect { x: over.body().x + 100, y: over.body().y + 100, w: 800, h: 118 };
        let (a, b) = (drawn(&mut over), drawn(&mut under));
        let white = |c: &Canvas| (r.y..r.y + r.h).map(|y| (r.x..r.x + r.w).filter(|&x| c.px[(y * c.stride + x) as usize] == 255).count()).sum::<usize>();
        assert_eq!(white(&a), 800 * 118, "{host:?}: the kit's fill drawn after the host's own shows");
        assert_eq!(white(&b), 0, "{host:?}: drawn before it, it is covered");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_packages_text_is_not_reported_to_the_translations_fit_check() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static HEARD: AtomicUsize = AtomicUsize::new(0);
    let root = fresh_root("fit");
    font_set_overflow_hook(Some(|_, _, _| {
        HEARD.fetch_add(1, Ordering::SeqCst);
    }));
    // Texts and labels far too long for their room, in every element that draws one: none is heard of.
    let long = "A text far too long for the room it is given, which Grimoire cuts short";
    let cmds = vec![
        K::Text(0, 0, 100, 0, long, 0, 0),
        K::Text(0, 60, 100, 0, long, 0, 1),
        K::Text(0, 120, 100, 0, long, 0, 2),
        K::Text(0, 180, 100, 60, long, 0, 8),
        K::Text(0, 240, 100, 60, long, 0, 4),
        K::Row(-1, 0, 300, 200, 0, "gear", long, 0, 1),
        K::Button(-1, 0, 440, 200, 0, long, "", 0),
        K::Chip(-1, 0, 560, 200, 0, long, 0),
        K::Field(-1, 0, 660, 200, 0, "search", long, long, 0),
        K::Select(-1, 0, 780, 200, 0, long, long),
        K::Checkbox(-1, 0, 880, 200, 0, long, 0),
    ];
    for theme in [UiTheme::Gothic, UiTheme::Classic] {
        ui_set_theme(theme);
        for host in HOSTS {
            let mut rt = loaded(&format!("test.kit.fit.{}.{}", tag(host), look(theme)), &pass(host, &cmds));
            HEARD.store(0, Ordering::SeqCst);
            let _ = drawn(&mut rt);
            assert_eq!(rt.error(), None);
            assert_eq!(HEARD.load(Ordering::SeqCst), 0, "{host:?} {theme:?}");
        }
    }
    // (the hook hears of Grimoire's own text cut short, after a package's has been drawn)
    let mut c = patterned();
    HEARD.store(0, Ordering::SeqCst);
    font_draw_left(&mut c, FontFace::Regular, 44.0, Rect::new(0, 0, 100, 60), long, 0);
    assert_eq!(HEARD.load(Ordering::SeqCst), 1);
    font_set_overflow_hook(None);
    ui_set_theme(UiTheme::Gothic);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_icon_is_drawn_at_most_1024_px_each_way_in_both_hosts() {
    let root = fresh_root("icons");
    for host in HOSTS {
        // An icon in a rectangle of 100,000 px is drawn at 1,024 px, in the middle (of what of it is in the body).
        let side = 100_000;
        let body = match host {
            Host::Widget => WIDGET_BODY,
            Host::App => app_body(),
        };
        let (cx, cy) = (body.w / 2, body.h / 2);
        let mut rt = loaded(&format!("test.kit.icon.{}", tag(host)), &pass(host, &[K::Icon(cx - side / 2, cy - side / 2, side, side, "gear", 0)]));
        let c = drawn(&mut rt);
        assert_eq!(rt.error(), None);
        assert!(inked(&c, body) > 100_000, "{host:?}: the icon is drawn at 1,024 px");
        // (ui_icon draws in its rectangle less a tenth of its side all round: an icon of 1,024 px is a square of 1,280)
        let mut rt = loaded(&format!("test.kit.icon.held.{}", tag(host)), &pass(host, &[K::Icon(cx - 640, cy - 640, 1280, 1280, "gear", 0)]));
        assert!(drawn(&mut rt).px == c.px, "{host:?}: as an icon of 1,024 px there");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn kit_lines_breaks_at_the_texts_own_lines_and_between_words_and_a_long_word_between_its_letters() {
    let root = fresh_root("lines");
    let (f, px) = (FontFace::Regular, 44.0);
    let w = 400;
    for theme in [UiTheme::Gothic, UiTheme::Classic] {
        ui_set_theme(theme);
        let lines = kit_lines(f, px, "one two three four five six seven eight nine ten\nshort\n\nlast", w);
        assert!(lines.iter().all(|l| font_measure(f, px, l) <= w), "{lines:?}");
        assert!(lines.len() > 4 && lines.contains(&"short".to_string()) && lines.contains(&String::new()) && lines.last().unwrap() == "last", "{lines:?}");
        assert_eq!(lines.iter().take_while(|l| *l != "short").cloned().collect::<Vec<_>>().join(" "), "one two three four five six seven eight nine ten");
        // (a word wider than the line takes lines of its own, as many letters to each as fit, none lost)
        let word = "abcdefghijklmnopqrstuvwxyz".repeat(4);
        let lines = kit_lines(f, px, &format!("a {word} b"), w);
        assert!(lines.iter().all(|l| font_measure(f, px, l) <= w), "{lines:?}");
        assert_eq!(lines.concat().replace(' ', ""), format!("a{word}b"));
        // (a letter wider than the line is a line of its own)
        assert_eq!(kit_lines(f, px, "WWW", 1), vec!["W", "W", "W"]);
        // (a text with no words is one line, empty; letters of more than one byte are never split)
        assert_eq!(kit_lines(f, px, "", w), vec![String::new()]);
        let kana = "ひらがなカタカナ".repeat(10);
        assert_eq!(kit_lines(f, px, &kana, 200).concat(), kana);
    }
    ui_set_theme(UiTheme::Gothic);
    let _ = std::fs::remove_dir_all(&root);
}
