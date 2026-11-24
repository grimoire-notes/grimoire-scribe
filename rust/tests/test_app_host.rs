// test_app_host.rs: what the app host (apphost.rs) lets an installed app's module do, with modules written for
// the purpose (tests/appfix): each reaches and passes one rule of the host, and an honest one is drawn exactly
// as the kit draws it.
mod appfix;

use appfix::*;
use grimoire::globalui::{gui_app_tile, gui_chip, gui_fab, gui_select, gui_status_h, gui_surface, gui_text_field};
use grimoire::prelude::*;

/// The screen of the tests: a Scribe's.
const SCREEN: (i32, i32) = (1860, 2480);

/// A fresh data root for one test (the Makefile's test rules set GRIMOIRE_TEST_TMP), the UI at 300 dpi.
fn fresh_root(name: &str) -> std::path::PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let root = base.join(format!("app-host-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    ui_init(300.0);
    assert_eq!(font_init(), 0);
    ui_set_theme(UiTheme::Gothic);
    root
}

/// Installs a module as an app package, as an install from a repository does.
fn install(id: &str, capabilities: &[&str], module: &[u8]) {
    let m = manifest(id, "Test", capabilities, false, module);
    app_package_install(&m, module, None, "", SCREEN.0, SCREEN.1).unwrap();
}

/// Installs a module and loads it, as a launch does.
fn installed(id: &str, capabilities: &[&str], app: &Wasm) -> AppRuntime {
    install(id, capabilities, &app.bytes());
    AppRuntime::open(id, SCREEN.0, SCREEN.1, false).unwrap()
}

/// A canvas with no two neighbours alike, so that any pixel a draw changes shows.
fn patterned(w: i32, h: i32) -> Canvas {
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, w, h), 0);
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

/// How many pixels of `r` are not white: what a pass drew there (the host fills the body white first).
fn inked(c: &Canvas, r: Rect) -> usize {
    (r.y..r.y + r.h).map(|y| (r.x..r.x + r.w).filter(|&x| c.px[(y * c.stride + x) as usize] != 255).count()).sum()
}

/// The body of an app that shows the status bar, and of one that hides it.
fn body_under_the_bar() -> Rect {
    let top = gui_status_h();
    Rect { x: 0, y: top, w: SCREEN.0, h: SCREEN.1 - top }
}
const WHOLE: Rect = Rect { x: 0, y: 0, w: SCREEN.0, h: SCREEN.1 };

fn style(v: i32) -> ButtonStyle {
    match v {
        1 => ButtonStyle::Selected,
        2 => ButtonStyle::Disabled,
        3 => ButtonStyle::Primary,
        _ => ButtonStyle::Plain,
    }
}

fn face(v: i32) -> FontFace {
    [FontFace::Regular, FontFace::Bold, FontFace::Title, FontFace::Caps, FontFace::Fixed, FontFace::FixedBold][v.clamp(0, 5) as usize]
}

fn icon_of(name: &str) -> IconKind {
    if name.is_empty() {
        IconKind::None
    } else {
        widget_icon(name)
    }
}

/// The commands drawn with the kit itself, each at its place in `body`: what docs/APPS.md says each call
/// draws ("the same themed elements Settings, the Library and the editor use").
fn kit(c: &mut Canvas, body: Rect, cmds: &[Cmd]) {
    let at = |x: i32, y: i32, w: i32, h: i32| Rect { x: body.x + x, y: body.y + y, w, h };
    let whole = c.clip;
    canvas_fill(c, body, 255);
    for cmd in cmds {
        match *cmd {
            Cmd::Fill(x, y, w, h, gray) => canvas_fill(c, at(x, y, w, h), gray as u8),
            Cmd::Outline(x, y, w, h, t, gray) => canvas_rect_outline(c, at(x, y, w, h), t, gray as u8),
            Cmd::Round(x, y, w, h, radius, gray, filled, t) => canvas_round_rect(c, at(x, y, w, h), radius, gray as u8, filled != 0, t),
            Cmd::Line(x0, y0, x1, y1, width, gray) => canvas_line_aa(c, body.x as f32 + x0, body.y as f32 + y0, body.x as f32 + x1, body.y as f32 + y1, width, gray as u8),
            Cmd::Text(x, y, w, h, text, px_milli, f, center, gray) => {
                let px = px_milli as f32 / 1000.0;
                ui_text(c, face(f), px, px, at(x, y, w, h), text, 1, center != 0, gray as u8);
            }
            Cmd::TextAt(x, baseline, text, px_milli, f, gray) => {
                font_draw(c, face(f), px_milli as f32 / 1000.0, body.x + x, body.y + baseline, text, gray as u8);
            }
            Cmd::Icon(icon, x, y, w, h, gray) => ui_icon(c, widget_icon(icon), at(x, y, w, h), gray as u8),
            Cmd::Button(x, y, w, h, label, icon, st) => ui_button(c, at(x, y, w, h), icon_of(icon), label, style(st)),
            Cmd::Chip(x, y, w, h, label, selected) => gui_chip(c, at(x, y, w, h), label, selected != 0),
            Cmd::Row(x, y, w, h, icon, label, st, switch) => {
                let mut row = Row::new(icon_of(icon), label);
                row.style = style(st);
                if switch >= 0 {
                    row.switch = Some(switch != 0);
                }
                ui_row(c, at(x, y, w, h), &row);
            }
            Cmd::Surface(x, y, w, h, filled, thick) => gui_surface(c, at(x, y, w, h), filled != 0, thick),
            Cmd::Select(x, y, w, h, caption, value) => gui_select(c, at(x, y, w, h), caption, value),
            Cmd::AppTile(x, y, w, h, icon, label, selected) => gui_app_tile(c, at(x, y, w, h), &AppIcon::glyph(widget_icon(icon)), label, selected != 0),
            Cmd::Fab(x, y, w, h, icon) => gui_fab(c, at(x, y, w, h), widget_icon(icon)),
            Cmd::Card(x, y, w, h, st) => ui_card(c, at(x, y, w, h), style(st)),
            Cmd::TextField(x, y, w, h, icon, text, hint) => {
                gui_text_field(c, at(x, y, w, h), icon_of(icon), text, hint);
            }
            Cmd::Clip(x, y, w, h) => canvas_set_clip(c, at(x, y, w, h)),
            Cmd::ClipClear => c.clip = whole,
        }
    }
    c.clip = whole;
}

/// The texts of the commands that the host's `library.app.text` probe lists, in order.
fn texts(cmds: &[Cmd]) -> String {
    let mut out: Vec<&str> = Vec::new();
    for cmd in cmds {
        match *cmd {
            Cmd::Text(_, _, _, _, text, ..) | Cmd::TextAt(_, _, text, ..) => out.push(text),
            Cmd::Button(_, _, _, _, label, ..) | Cmd::Chip(_, _, _, _, label, _) | Cmd::Row(_, _, _, _, _, label, ..) | Cmd::AppTile(_, _, _, _, _, label, _) => out.push(label),
            Cmd::Select(_, _, _, _, _, value) => out.push(value),
            Cmd::TextField(_, _, _, _, _, text, hint) => out.extend([text, hint]),
            _ => {}
        }
    }
    out.join("|")
}

#[test]
fn an_honest_pass_is_drawn_as_the_kit_draws_it() {
    let root = fresh_root("honest");
    let cmds = honest();
    let mut rt = installed("test.honest", &BOTH, &pass(&cmds));
    for theme in [UiTheme::Gothic, UiTheme::Classic] {
        ui_set_theme(theme);
        for body in [WHOLE, body_under_the_bar()] {
            let before = patterned(SCREEN.0, SCREEN.1);
            let (mut drawn, mut want) = (before.clone(), before.clone());
            rt.draw(&mut drawn, body, 0);
            assert_eq!(rt.error(), None, "{theme:?}");
            kit(&mut want, body, &cmds);
            assert!(drawn.px == want.px, "{theme:?}, a body from y {}: the pass is the kit's own drawing, pixel for pixel", body.y);
            assert!(inked(&drawn, body) > 100_000, "{theme:?}: the pass drew");
            assert_eq!(changed_outside(&before, &drawn, body), 0, "{theme:?}: an honest pass keeps to its body");
            assert_eq!(drawn.clip, before.clip, "the canvas keeps its clip");
            assert_eq!(rt.text(), texts(&cmds));
        }
    }
    ui_set_theme(UiTheme::Gothic);
    let _ = std::fs::remove_dir_all(&root);
}

/// The fixture app of tests/fixtures/apppkg: its manifest and its module.
fn fixture() -> (String, Vec<u8>) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/apppkg/packages/apps/com.example.fixture");
    (std::fs::read_to_string(dir.join("package.yaml")).unwrap(), std::fs::read(dir.join("app.wasm")).unwrap())
}

#[test]
fn the_fixture_app_draws_its_rectangle_and_its_words() {
    let root = fresh_root("fixture");
    let (yaml, module) = fixture();
    // (the fixture's manifest asks for a Grimoire this tree's VERSION is not yet: its scenario copies it in place)
    let yaml = yaml.replace("min_grimoire_version: 2.3.0\n", "");
    let icon = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico")).unwrap();
    app_package_install(&yaml, &module, Some(&icon), "", SCREEN.0, SCREEN.1).unwrap();
    let mut rt = AppRuntime::open("com.example.fixture", SCREEN.0, SCREEN.1, false).unwrap();
    let before = patterned(SCREEN.0, SCREEN.1);
    let mut c = before.clone();
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), None);
    assert_eq!((rt.status(), rt.text().as_str()), ("fixture app", "fixture app"));
    // Its black rectangle, 400x180 at 80,120, and its words under it; white elsewhere.
    let block = Rect { x: 80, y: 120, w: 400, h: 180 };
    assert_eq!(inked(&c, block), 400 * 180);
    assert!(inked(&c, Rect { x: 80, y: 380, w: 1200, h: 128 }) > 1000);
    assert_eq!(inked(&c, Rect { x: 0, y: 600, w: SCREEN.0, h: SCREEN.1 - 600 }), 0);
    // A touch that lifts is its call of close(): the module runs on and keeps what it drew.
    rt.pointer(0, 900, 1300, 0);
    rt.pointer(2, 900, 1300, 0);
    assert_eq!(rt.error(), None);
    rt.draw(&mut c, WHOLE, 16);
    assert_eq!(rt.status(), "fixture app");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_module_that_calls_every_import_runs() {
    let root = fresh_root("every");
    // Each import of the ABI once, with numbers an app passes: the module links, and the pass ends well.
    let one = |import: &'static str| -> Vec<u8> {
        let (_, params, _) = IMPORTS[appfix::import(import) as usize];
        if import == "line" {
            return line(10.0, 10.0, 200.0, 40.0, 2.0, 0);
        }
        // (every pointer is address 0, every length 4, every other number 40: a text "word" and small sizes;
        // http_start's are a request no host starts without the capability)
        let args: Vec<i32> = match import {
            "request_redraw" => vec![1000],
            "status" => vec![0, 4],
            "toast" => vec![0, 4, 1500],
            "state_get" | "time_text" | "date_text" | "wifi_ssid" | "http_body" | "http_error" => vec![512, 64],
            "setting" => vec![0, 4, 512, 64],
            "text" => vec![40, 40, 400, 60, 0, 4, 40_000, 0, 0, 0],
            "text_at" => vec![40, 200, 0, 4, 40_000, 0, 0],
            "text_width" => vec![0, 4, 40_000, 0],
            "text_height" => vec![40_000, 0],
            "http_start" => vec![0, 4, 0, 4, 0, 0, 0, 0],
            _ => (0..params.len()).map(|at| if at % 2 == 0 { 40 } else { 4 }).collect(),
        };
        call(import, &args)
    };
    let all: Vec<u8> = IMPORTS.iter().filter(|(n, _, _)| *n != "close").flat_map(|(n, _, _)| one(n)).collect();
    for (id, capabilities) in [("test.every.both", &BOTH[..]), ("test.every.none", &[][..])] {
        let mut app = Wasm::with(all.clone(), b"word");
        app.pointer = one("close");
        app.status_bar = Some(1);
        let mut rt = installed(id, capabilities, &app);
        let mut c = patterned(SCREEN.0, SCREEN.1);
        for _ in 0..2 {
            rt.draw(&mut c, body_under_the_bar(), 16);
            assert_eq!(rt.error(), None, "{id}");
            assert_eq!(rt.status(), "word", "{id}");
        }
        rt.pointer(2, 5, 5, 0);
        rt.key(30, 0, true);
        rt.stop();
        assert_eq!(rt.error(), None, "{id}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// What a launch says of a module that did not start.
const NOT_STARTED: &str = "The app could not start: ";

/// A module's load, by its memories (pages at the start) and its tables (entries at the start): None when it
/// loads, else what the launch says.
fn load_of(id: &str, memories: &[u32], tables: &[u32]) -> Option<String> {
    let mut w = Wasm::new(Vec::new());
    w.memories = memories.iter().map(|&pages| (pages, None)).collect();
    w.tables = tables.iter().map(|&entries| (entries, None)).collect();
    install(id, &BOTH, &w.bytes());
    AppRuntime::open(id, SCREEN.0, SCREEN.1, false).err()
}

#[test]
fn a_module_has_one_memory_and_one_table_of_a_bounded_length() {
    let root = fresh_root("store");
    let not_started = |e: Option<String>| e.is_some_and(|e| e.starts_with(NOT_STARTED));
    let (pages, entries) = (48 * 1024 * 1024 / 65536, APP_TABLE_MAX as u32);
    assert_eq!((pages, entries), (768, 65_536));
    // One memory of 48 MiB and one table of 65,536 entries are the most a module has.
    assert_eq!(load_of("test.store.most", &[pages], &[entries]), None);
    assert_eq!(load_of("test.store.none", &[1], &[]), None);
    // A second memory, however small; many memories; one memory a page larger than the limit.
    assert!(not_started(load_of("test.store.two", &[1, 1], &[])));
    assert!(not_started(load_of("test.store.many", &[768; 16], &[])));
    assert!(not_started(load_of("test.store.large", &[pages + 1], &[])));
    // A second table; one table an entry longer than the limit; a table of twenty million entries.
    assert!(not_started(load_of("test.store.tables", &[1], &[1, 1])));
    assert!(not_started(load_of("test.store.long", &[1], &[entries + 1])));
    assert!(not_started(load_of("test.store.longer", &[1], &[20_000_000])));
    // In a call, growing past a limit fails as WebAssembly defines it: the result is -1, nothing is given,
    // and the module runs on. The status line is one character for each try: '1' where the result was -1.
    let tried = |grow: &[u8], at: i32| seq(&[i(at), grow.to_vec(), i(-1), vec![0x46], i(48), vec![0x6a, 0x3a, 0x00, 0x00]]);
    let table_grow = |by: i32| seq(&[vec![0xd0, 0x70], i(by), vec![0xfc, 0x0f, 0x00]]);
    let memory_grow = |by: i32| seq(&[i(by), vec![0x40, 0x00]]);
    let mut w = Wasm::new(seq(&[
        tried(&table_grow(entries as i32), 0),     // 1 + 65,536 entries: past the limit
        tried(&table_grow(30_000_000), 1),         // far past it
        tried(&table_grow(entries as i32 - 1), 2), // to the limit exactly: given
        tried(&table_grow(1), 3),                  // and one more: not given
        tried(&memory_grow(pages as i32), 4),      // 1 + 768 pages: past the limit
        tried(&memory_grow(pages as i32 - 1), 5),  // to the limit exactly: given
        tried(&memory_grow(1), 6),                 // and one more: not given
        call("status", &[0, 7]),
    ]));
    w.tables = vec![(1, None)];
    let mut rt = installed("test.store.grow", &BOTH, &w);
    let mut c = patterned(SCREEN.0, SCREEN.1);
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), None);
    assert_eq!(rt.status(), "1101101");
    let _ = std::fs::remove_dir_all(&root);
}
