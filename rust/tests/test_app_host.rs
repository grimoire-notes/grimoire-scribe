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

/// The same for a test of many modules: the package is removed once its module is loaded, since no more than
/// 64 app packages are installed at a time. (The loaded module runs on; it has no folder to save a state in.)
fn loaded(id: &str, capabilities: &[&str], app: &Wasm) -> AppRuntime {
    let rt = installed(id, capabilities, app);
    app_package_remove(id, false).unwrap();
    rt
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

/// What a launch says when the module's file fails the check before a load.
const NOT_READ: &str = "The app's artifact could not be read.";

/// The error of a load of the app, None when it loads.
fn load(id: &str) -> Option<String> {
    AppRuntime::open(id, SCREEN.0, SCREEN.1, false).err()
}

/// A file of the data root with `from` replaced by `to`, as a hand with an editor would.
fn rewrite(path: &std::path::Path, from: &str, to: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(from), "{from} in {path:?}");
    std::fs::write(path, text.replace(from, to)).unwrap();
}

#[test]
fn a_module_is_checked_again_before_each_launch() {
    use std::os::unix::fs::symlink;
    let root = fresh_root("load");
    let module = pass(&honest()).bytes();
    let other = Wasm::new(call("fill_rect", &[0, 0, 10, 10, 0])).bytes();
    install("test.load", &BOTH, &module);
    let dir = root.join("packages/apps/test.load");
    let (file, yaml, index) = (dir.join("app.wasm"), dir.join("package.yaml"), root.join("packages/apps-index.json"));
    assert_eq!(load("test.load"), None);

    // The file's bytes are not the ones installed: one bit changed, another module, nothing, no file.
    let mut bit = module.clone();
    *bit.last_mut().unwrap() ^= 1;
    for bytes in [&bit[..], &other[..], &[][..]] {
        std::fs::write(&file, bytes).unwrap();
        assert_eq!(load("test.load").as_deref(), Some(NOT_READ));
    }
    std::fs::remove_file(&file).unwrap();
    assert_eq!(load("test.load").as_deref(), Some(NOT_READ));

    // A link in the file's place is not followed, whether it leads to the same bytes inside the folder or outside.
    let (inside, outside) = (dir.join("copy.bin"), root.join("packages/apps/elsewhere.bin"));
    std::fs::write(&inside, &module).unwrap();
    std::fs::write(&outside, &module).unwrap();
    for target in [std::path::Path::new("copy.bin"), &inside, &outside] {
        symlink(target, &file).unwrap();
        assert_eq!(load("test.load").as_deref(), Some(NOT_READ), "a link to {target:?}");
        std::fs::remove_file(&file).unwrap();
    }
    std::fs::write(&file, &module).unwrap();
    assert_eq!(load("test.load"), None);

    // The installed manifest names another file: outside the folder, by an absolute path, the manifest, the state,
    // a name the walk does not take, or a file of the folder that the index does not name.
    std::fs::create_dir_all(dir.join("state")).unwrap();
    std::fs::write(dir.join("state/runtime.bin"), &module).unwrap();
    std::fs::write(dir.join(".app.wasm"), &module).unwrap();
    let absolute = outside.to_str().unwrap().to_string();
    for path in ["../elsewhere.bin", "./app.wasm", "x/../app.wasm", absolute.as_str(), "/app.wasm", "package.yaml", "state/runtime.bin", ".app.wasm", "copy.bin", ""] {
        rewrite(&yaml, "path: app.wasm", &format!("path: {path}"));
        assert_eq!(load("test.load").as_deref(), Some(NOT_READ), "artifact.path: {path}");
        rewrite(&yaml, &format!("path: {path}"), "path: app.wasm");
    }
    // (the index and the manifest naming that file together, as an install writes them, load it)
    rewrite(&yaml, "path: app.wasm", "path: copy.bin");
    rewrite(&index, "test.load/app.wasm", "test.load/copy.bin");
    assert_eq!(load("test.load"), None);
    rewrite(&yaml, "path: copy.bin", "path: app.wasm");
    assert_eq!(load("test.load").as_deref(), Some(NOT_READ), "the index names another file than the manifest");
    rewrite(&index, "test.load/copy.bin", "test.load/app.wasm");
    assert_eq!(load("test.load"), None);
    // The index and the manifest agreeing on a file the rule for artifact.path does not take: the path is asked
    // again at the load, whatever the index says. The state folder, in any case of its letters (a storage that
    // ignores case finds it by each), with the module's own bytes there, and names the storage walk does not take.
    for path in ["state/runtime.bin", "STATE/runtime.bin", "State/settings.json", "sTaTe/app.wasm", ".app.wasm", "dist./app.wasm", "notes.sdr/app.wasm"] {
        let at = dir.join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(&at, &module).unwrap();
        rewrite(&yaml, "path: app.wasm", &format!("path: {path}"));
        rewrite(&index, "test.load/app.wasm", &format!("test.load/{path}"));
        assert_eq!(load("test.load").as_deref(), Some(NOT_READ), "artifact.path and the index: {path}");
        rewrite(&yaml, &format!("path: {path}"), "path: app.wasm");
        rewrite(&index, &format!("test.load/{path}"), "test.load/app.wasm");
        std::fs::remove_file(&at).unwrap();
    }
    for folder in ["state", "STATE", "State", "sTaTe", "dist.", "notes.sdr"] {
        let _ = std::fs::remove_dir_all(dir.join(folder));
    }
    // The manifest itself, in another case of its letters, with the index recording the manifest's own SHA-256:
    // no file of that name is read as a module.
    let recorded = grimoire::sha256::sha256_of(&module);
    for path in ["Package.yaml", "PACKAGE.YAML"] {
        rewrite(&yaml, "path: app.wasm", &format!("path: {path}"));
        rewrite(&index, "test.load/app.wasm", &format!("test.load/{path}"));
        let own = grimoire::sha256::sha256_of(&std::fs::read(&yaml).unwrap());
        rewrite(&index, &format!("\"sha\":\"{recorded}\""), &format!("\"sha\":\"{own}\""));
        assert_eq!(load("test.load").as_deref(), Some(NOT_READ), "artifact.path and the index: {path}");
        rewrite(&index, &format!("\"sha\":\"{own}\""), &format!("\"sha\":\"{recorded}\""));
        rewrite(&yaml, &format!("path: {path}"), "path: app.wasm");
        rewrite(&index, &format!("test.load/{path}"), "test.load/app.wasm");
    }
    assert_eq!(load("test.load"), None);

    // The index's record of the file: another digest, none, a short one, and the same one in capitals.
    for (sha, loads) in [(grimoire::sha256::sha256_of(&other), false), (String::new(), false), (recorded[..63].to_string(), false), (recorded.to_uppercase(), true)] {
        rewrite(&index, &format!("\"sha\":\"{recorded}\""), &format!("\"sha\":\"{sha}\""));
        assert_eq!(load("test.load").is_none(), loads, "sha {sha}");
        rewrite(&index, &format!("\"sha\":\"{sha}\""), &format!("\"sha\":\"{recorded}\""));
    }

    // A folder on the way to the file, and the app's own folder, replaced by a link to one with the same files.
    let m = manifest("test.deep", "Test", &BOTH, false, &module).replace("path: app.wasm", "path: dist/app.wasm");
    app_package_install(&m, &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    assert_eq!(load("test.deep"), None);
    let deep = root.join("packages/apps/test.deep");
    std::fs::rename(deep.join("dist"), deep.join("real")).unwrap();
    symlink("real", deep.join("dist")).unwrap();
    assert_eq!(load("test.deep").as_deref(), Some(NOT_READ), "a folder of the path is a link");
    std::fs::remove_file(deep.join("dist")).unwrap();
    std::fs::rename(deep.join("real"), deep.join("dist")).unwrap();
    assert_eq!(load("test.deep"), None);
    std::fs::rename(&deep, root.join("packages/apps/moved")).unwrap();
    symlink("moved", &deep).unwrap();
    assert!(load("test.deep").is_some(), "the app's folder is a link");
    std::fs::remove_file(&deep).unwrap();
    std::fs::rename(root.join("packages/apps/moved"), &deep).unwrap();
    assert_eq!(load("test.deep"), None);

    // An app that is not installed, and one whose manifest is another package's, are not there to launch.
    assert_eq!(load("test.none").as_deref(), Some("That app is no longer installed."));
    rewrite(&deep.join("package.yaml"), "id: test.deep", "id: test.other");
    assert_eq!(load("test.deep").as_deref(), Some("That app is no longer installed."));

    // The check is the launch's: an app that has loaded its module draws without reading the file again.
    let mut rt = AppRuntime::open("test.load", SCREEN.0, SCREEN.1, false).unwrap();
    std::fs::write(&file, &other).unwrap();
    let mut c = patterned(SCREEN.0, SCREEN.1);
    rt.draw(&mut c, WHOLE, 0);
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), None);
    assert!(inked(&c, WHOLE) > 100_000);
    assert_eq!(load("test.load").as_deref(), Some(NOT_READ), "the next launch checks it");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_app_without_the_storage_capability_has_no_saved_state() {
    let root = fresh_root("storage");
    let state_file = |id: &str| root.join(format!("packages/apps/{id}/state/runtime.bin"));
    let mut c = patterned(SCREEN.0, SCREEN.1);
    // Declared: the first draw finds nothing saved and saves a word, the next finds it, and so does a new
    // launch; a touch, a key and the stop each save theirs.
    let mut rt = installed("test.keeps", &[STORAGE], &keeps());
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.status(), "");
    assert_eq!(std::fs::read(state_file("test.keeps")).unwrap(), b"hello");
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.status(), "hello");
    rt.pointer(0, 5, 5, 0);
    assert_eq!(std::fs::read(state_file("test.keeps")).unwrap(), b"touch");
    rt.key(30, 0, true);
    assert_eq!(std::fs::read(state_file("test.keeps")).unwrap(), b"key");
    rt.stop();
    assert_eq!(std::fs::read(state_file("test.keeps")).unwrap(), b"stop");
    let mut rt = AppRuntime::open("test.keeps", SCREEN.0, SCREEN.1, false).unwrap();
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.status(), "stop");
    // Not declared (no capability, and the network alone): state_get gives nothing and state_set keeps nothing,
    // in a draw, a touch, a key and the stop, and neither the file nor its folder is made. The app draws all the
    // same: its rectangle is there.
    for (id, capabilities) in [("test.forgets", &[][..]), ("test.forgets.net", &[NETWORK][..])] {
        let mut rt = installed(id, capabilities, &keeps());
        for _ in 0..3 {
            let mut c = patterned(SCREEN.0, SCREEN.1);
            rt.draw(&mut c, WHOLE, 0);
            assert_eq!(rt.error(), None, "{id}");
            assert_eq!(rt.status(), "", "{id}");
            assert_eq!(inked(&c, Rect { x: 40, y: 40, w: 200, h: 100 }), 200 * 100, "{id}: an app with no capability draws");
            rt.pointer(0, 5, 5, 0);
            rt.key(30, 0, true);
        }
        rt.stop();
        assert_eq!(rt.error(), None, "{id}");
        assert!(!state_file(id).exists(), "{id}");
        assert!(!state_file(id).parent().unwrap().exists(), "{id}: no state folder is made either");
        // A state file from a version of the app that declared the capability is not read, and stays as it is.
        std::fs::create_dir_all(state_file(id).parent().unwrap()).unwrap();
        std::fs::write(state_file(id), b"kept").unwrap();
        let mut rt = AppRuntime::open(id, SCREEN.0, SCREEN.1, false).unwrap();
        rt.draw(&mut c, WHOLE, 0);
        rt.pointer(2, 5, 5, 0);
        rt.draw(&mut c, WHOLE, 0);
        rt.stop();
        assert_eq!(rt.status(), "", "{id}");
        assert_eq!(std::fs::read(state_file(id)).unwrap(), b"kept", "{id}");
    }
    // The settings of an app are the reader's, kept by the host with or without the capability.
    let with_setting = |id: &str, capabilities: &[&str]| {
        let module = Wasm::with(seq(&[i(100), i(0), i(4), i(100), i(64), invoke("setting"), invoke("status")]), b"dark").bytes();
        let m = format!("{}settings:\n  - key: dark\n    label: Dark\n    type: toggle\n    default: true\n", manifest(id, "Test", capabilities, false, &module));
        app_package_install(&m, &module, None, "", SCREEN.0, SCREEN.1).unwrap();
        AppRuntime::open(id, SCREEN.0, SCREEN.1, false).unwrap()
    };
    for (id, capabilities) in [("test.set.none", &[][..]), ("test.set.both", &BOTH[..])] {
        let mut rt = with_setting(id, capabilities);
        rt.draw(&mut c, WHOLE, 0);
        assert_eq!(rt.status(), "true", "{id}");
        assert_eq!(app_settings_save(id, &[("dark".to_string(), "false".to_string())]), 0);
        let mut rt = AppRuntime::open(id, SCREEN.0, SCREEN.1, false).unwrap();
        rt.draw(&mut c, WHOLE, 0);
        assert_eq!(rt.status(), "false", "{id}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// What an app says once a drawing pass of its module used up its instructions: the host's words, then the
/// interpreter's for its out-of-fuel trap.
const STOPPED: &str = "The app stopped: all fuel consumed by WebAssembly";

/// Whether the last call used its budget up. (A call the interpreter ends by itself leaves what the instructions
/// it was about to run cost less one, a few units of the thirty million.)
fn used_up(rt: &AppRuntime) -> bool {
    rt.fuel_used() <= APP_CALL_FUEL && rt.fuel_used() + 64 > APP_CALL_FUEL
}

#[test]
fn a_module_that_loops_for_ever_stops_when_its_instructions_are_used_up() {
    let root = fresh_root("loops");
    let before = patterned(SCREEN.0, SCREEN.1);
    // A drawing pass that never ends: the call is ended when its instructions are used up, the app says so,
    // nothing is drawn and the module is not called again.
    let mut rt = installed("test.loop.draw", &BOTH, &Wasm::new(forever(&[])));
    let mut c = before.clone();
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), Some(STOPPED));
    assert!(used_up(&rt), "{}", rt.fuel_used());
    assert_eq!(changed_outside(&before, &c, RECT_EMPTY), 0);
    rt.draw(&mut c, WHOLE, 0);
    rt.pointer(0, 4, 4, 0);
    rt.key(30, 0, true);
    rt.stop();
    assert_eq!(rt.error(), Some(STOPPED));
    assert_eq!(changed_outside(&before, &c, RECT_EMPTY), 0);
    // The same for a loop that calls an import on every turn, and for a touch and a key that never end.
    let mut rt = installed("test.loop.calls", &BOTH, &Wasm::new(forever(&call("now", &[]))));
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), Some(STOPPED));
    let mut touched = Wasm::new(call("fill_rect", &[0, 0, 8, 8, 0]));
    touched.pointer = forever(&[]);
    let mut rt = installed("test.loop.touch", &BOTH, &touched);
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), None);
    rt.pointer(0, 4, 4, 0);
    assert_eq!((rt.error(), used_up(&rt)), (Some("The app stopped while handling a touch."), true));
    let mut keyed = Wasm::new(call("fill_rect", &[0, 0, 8, 8, 0]));
    keyed.key = forever(&[]);
    let mut rt = installed("test.loop.key", &BOTH, &keyed);
    rt.draw(&mut c, WHOLE, 0);
    rt.key(30, 0, true);
    assert_eq!((rt.error(), used_up(&rt)), (Some("The app stopped while handling a key."), true));
    // A stop that never ends is ended too, and the app has stopped either way.
    let mut stops = Wasm::new(call("fill_rect", &[0, 0, 8, 8, 0]));
    stops.stop = forever(&[]);
    let mut rt = installed("test.loop.stop", &BOTH, &stops);
    rt.draw(&mut c, WHOLE, 0);
    rt.stop();
    assert_eq!((rt.error(), used_up(&rt)), (None, true));
    // A start function that never ends: the module does not start, and the launch says why.
    let mut starts = Wasm::new(Vec::new());
    starts.start = Some(forever(&[]));
    install("test.loop.start", &BOTH, &starts.bytes());
    assert_eq!(load("test.loop.start").as_deref(), Some("The app could not start: all fuel consumed by WebAssembly"));
    // A function that calls itself for ever ends too: the interpreter's own call stack has a depth.
    let mut deep = Wasm::new(vec![0x10, FIRST_EXTRA as u8]);
    deep.extra = vec![vec![0x10, FIRST_EXTRA as u8]];
    let mut rt = installed("test.loop.deep", &BOTH, &deep);
    rt.draw(&mut c, WHOLE, 0);
    assert!(rt.error().is_some_and(|e| e.starts_with("The app stopped: ")), "{:?}", rt.error());
    // A long call that does end runs to its end: 20,000 calls of an import in a row, and a loop of 200,000 turns.
    let mut rt = installed("test.loop.line", &BOTH, &Wasm::new(times(20_000, &call("now", &[]))));
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), None);
    let mut rt = installed("test.loop.turns", &BOTH, &Wasm::new(seq(&[repeat(200_000, &[]), call("fill_rect", &[0, 0, 8, 8, 0])])));
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), None);
    assert!(rt.fuel_used() > 1_000_000 && rt.fuel_used() < APP_CALL_FUEL);
    let _ = std::fs::remove_dir_all(&root);
}

/// Draws these commands on `body`, on a copy of `screen` (with `clip` as the canvas's clip, when one is given): as
/// one drawing pass where the call's budget pays for all of them, and else as the passes of their halves, and so
/// on down to a single command. `ran` gets each pass that ran: the screen before it and after it. A pass the
/// budget does not pay for stops with the budget used up and draws nothing. Returns how many of the commands
/// were drawn.
fn drawn_within_the_budget(ids: &mut usize, screen: &Canvas, cmds: &[Cmd], body: Rect, clip: Option<Rect>, ran: &mut dyn FnMut(&Canvas, &Canvas)) -> usize {
    *ids += 1;
    let mut rt = loaded(&format!("test.paid-for.{ids}"), &BOTH, &pass(cmds));
    let mut c = screen.clone();
    if let Some(clip) = clip {
        canvas_set_clip(&mut c, clip);
    }
    let clip = c.clip;
    rt.draw(&mut c, body, 0);
    assert_eq!(c.clip, clip, "the canvas keeps its clip");
    if rt.error().is_none() {
        ran(screen, &c);
        return cmds.len();
    }
    assert_eq!(rt.error(), Some(STOPPED), "{cmds:?}");
    assert!(used_up(&rt), "{cmds:?}: stopped with {} of the budget used", rt.fuel_used());
    assert!(c.px == screen.px, "{cmds:?}: a pass that is stopped draws nothing");
    if cmds.len() == 1 {
        return 0;
    }
    let (first, rest) = cmds.split_at(cmds.len() / 2);
    drawn_within_the_budget(ids, screen, first, body, Some(clip), ran) + drawn_within_the_budget(ids, screen, rest, body, Some(clip), ran)
}

/// Drawing passes that reach outside the body with every command there is: from far above and left of it to far
/// below and right of it, across each of its edges, and through a `clip` that names more than the body.
/// (Sizes that no sum overflows with: those are the next rule's.)
fn passes_that_reach_out(body: Rect) -> Vec<(&'static str, Vec<Cmd>)> {
    let every = |x: i32, y: i32, w: i32, h: i32| -> Vec<Cmd> {
        vec![
            Cmd::Fill(x, y, w, h, 0),
            Cmd::Outline(x, y, w, h, 30, 0),
            Cmd::Round(x, y, w, h, 60, 0, 1, 0),
            Cmd::Round(x, y, w, h, 60, 40, 0, 25),
            Cmd::Line(x as f32, y as f32, (x + w) as f32, (y + h) as f32, 40.0, 0),
            Cmd::Line((x + w) as f32, y as f32, x as f32, (y + h) as f32, 9.0, 0),
            Cmd::Text(x, y, w, h, "Words that reach past the edge of the body they are drawn in", 90_000, 1, 0, 0),
            Cmd::Text(x, y, w, h, "Centered words", 120_000, 0, 1, 0),
            Cmd::TextAt(x, y, "Words at a place of their own, set by their baseline", 80_000, 0, 0),
            Cmd::TextAt(x + w - 200, y + h, "The end of a line", 80_000, 5, 0),
            Cmd::Icon("gear", x, y, w, h, 0),
            Cmd::Button(x, y, w, h, "A label", "", 1),
            Cmd::Button(x, y, w, h, "A label", "files", 0),
            Cmd::Chip(x, y, w, h, "A chip", 1),
            Cmd::Row(x, y, w, h, "gear", "A row", 1, 1),
            Cmd::Surface(x, y, w, h, 1, 0),
            Cmd::Surface(x, y, w, h, 0, 20),
            Cmd::Select(x, y, w, h, "Sort", "Name"),
            Cmd::AppTile(x, y, w, h, "clock", "Clock", 1),
            Cmd::Fab(x, y, w, h, "download"),
            Cmd::Card(x, y, w, h, 1),
            Cmd::TextField(x, y, w, h, "search", "typed", "hint"),
        ]
    };
    let (bw, bh) = (body.w, body.h);
    let far: Vec<Cmd> = every(-1000, -1000, bw + 2000, bh + 2000);
    let mut edges = Vec::new();
    for (x, y) in [(-150, -150), (bw - 150, -150), (-150, bh - 150), (bw - 150, bh - 150), (-290, 400), (bw - 10, 400), (400, -290), (400, bh - 10)] {
        edges.extend(every(x, y, 300, 300));
    }
    // (wholly outside: above the body, where the status bar is when it is shown, and on each other side)
    let mut outside = Vec::new();
    for (x, y) in [(100, -400), (-400, 100), (bw + 100, 100), (100, bh + 100)] {
        outside.extend(every(x, y, 300, 300));
    }
    outside.push(Cmd::Fill(40, 40, 60, 60, 0));
    // A clip that names more than the body, and the clip taken away, leave the body as the part drawn on.
    let mut clipped = vec![Cmd::Clip(-5000, -5000, 20_000, 20_000)];
    clipped.extend(every(-1000, -1000, bw + 2000, bh + 2000));
    clipped.push(Cmd::Clip(-300, -300, 600, 600));
    clipped.extend(every(-200, -200, 500, 500));
    clipped.push(Cmd::ClipClear);
    clipped.extend(every(-1000, -1000, bw + 2000, bh + 2000));
    // (and a clip that has no part in the body draws nothing from then on, until it is taken away)
    clipped.push(Cmd::Clip(0, -500, bw, 400));
    clipped.push(Cmd::Fill(-5000, -5000, 20_000, 20_000, 128));
    vec![("far", far), ("edges", edges), ("outside", outside), ("clipped", clipped)]
}

#[test]
fn whatever_an_app_draws_lands_in_its_body() {
    let root = fresh_root("body");
    // The body under the status bar, where the bar is shown; the whole screen; and a body that is a part of a
    // larger canvas on every side.
    let inner = Rect { x: 300, y: 200, w: 900, h: 1400 };
    let screen = patterned(SCREEN.0, SCREEN.1);
    let mut ids = 0;
    for body in [body_under_the_bar(), WHOLE, inner] {
        for (name, cmds) in passes_that_reach_out(body) {
            for theme in [UiTheme::Gothic, UiTheme::Classic] {
                ui_set_theme(theme);
                // (each list is drawn as the passes the call's budget pays for: every one of its commands is drawn)
                let mut ink = 0;
                let drawn = drawn_within_the_budget(&mut ids, &screen, &cmds, body, None, &mut |before, after| {
                    assert_eq!(changed_outside(before, after, body), 0, "{name} in {theme:?}, a body at {},{}: pixels outside the body changed", body.x, body.y);
                    ink += inked(after, body);
                });
                assert_eq!(drawn, cmds.len(), "{name} in {theme:?}: the budget pays for each of these commands");
                assert_ne!(ink, 0, "{name} in {theme:?}: the passes drew in the body");
            }
        }
    }
    // The last command of "far" and of "clipped" covers the body: it is filled to its edges, and no farther.
    ui_set_theme(UiTheme::Gothic);
    let body = body_under_the_bar();
    let mut rt = installed("test.body.black", &BOTH, &pass(&[Cmd::Clip(-5000, -5000, 20_000, 20_000), Cmd::Fill(-5000, -5000, 20_000, 20_000, 0), Cmd::ClipClear, Cmd::Fill(-9, -9, 10, 10, 255)]));
    let before = patterned(SCREEN.0, SCREEN.1);
    let mut c = before.clone();
    rt.draw(&mut c, body, 0);
    assert_eq!(changed_outside(&before, &c, body), 0);
    assert_eq!(inked(&c, body), (body.w * body.h - 1) as usize, "the body is black to its edges, but for the one white pixel of the last command");
    assert_eq!(c.px[(body.y * c.stride + body.x) as usize], 255);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn drawing_keeps_to_the_clip_it_is_given_too() {
    let root = fresh_root("clip");
    let body = body_under_the_bar();
    let black = pass(&[Cmd::Fill(-5000, -5000, 20_000, 20_000, 0), Cmd::Clip(-5000, -5000, 20_000, 20_000), Cmd::Fill(-5000, -5000, 20_000, 20_000, 0), Cmd::ClipClear, Cmd::Fill(-5000, -5000, 20_000, 20_000, 0)]);
    let mut rt = installed("test.clip", &BOTH, &black);
    // The caller composes a part of the screen that holds a corner of the body: only that corner is drawn.
    let part = Rect { x: 100, y: 20, w: 500, h: 300 };
    let before = patterned(SCREEN.0, SCREEN.1);
    let mut c = before.clone();
    canvas_set_clip(&mut c, part);
    rt.draw(&mut c, body, 0);
    let corner = rect_intersect(part, body);
    assert_eq!(changed_outside(&before, &c, corner), 0);
    assert_eq!(inked(&c, corner), (corner.w * corner.h) as usize);
    assert!((corner.y..corner.y + corner.h).all(|y| (corner.x..corner.x + corner.w).all(|x| c.px[(y * c.stride + x) as usize] == 0)));
    assert_eq!(c.clip, part);
    // A part that holds none of the body (the status bar's strip): nothing is drawn.
    let mut c = before.clone();
    canvas_set_clip(&mut c, Rect { x: 0, y: 0, w: SCREEN.0, h: body.y });
    rt.draw(&mut c, body, 0);
    assert_eq!(changed_outside(&before, &c, RECT_EMPTY), 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// A module of eight pages of memory (so that the whole saved state, 256 KiB, lies in it) with letters from
/// address 0, whose `app_draw` is `draw`.
fn roomy(draw: Vec<u8>) -> Wasm {
    let mut w = Wasm::with(draw, &[b'n'; 2048]);
    w.memories = vec![(8, None)];
    w
}

const MAX: i32 = i32::MAX;
const MIN: i32 = i32::MIN;

/// One call of each import that takes a text, with a text of `len` bytes from address 0 (and none beside it,
/// where the call takes two): its name, and the call.
fn texts_of(len: i32) -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("a text", call("text", &[0, 0, 60_000, 60, 0, len, 20_000, 0, 0, 0])),
        ("a text at a place", call("text_at", &[10, 100, 0, len, 20_000, 0, 0])),
        ("a text that is measured", call("text_width", &[0, len, 20_000, 0])),
        ("a button's label", call("button", &[0, 0, 400, 120, 0, len, 0, 0, 0])),
        ("a chip's label", call("chip", &[0, 0, 400, 80, 0, len, 0])),
        ("a row's label", call("row", &[0, 0, 900, 120, 0, 0, 0, len, 0, -1])),
        ("a select's caption", call("select", &[0, 0, 700, 100, 0, len, 0, 0])),
        ("a select's value", call("select", &[0, 0, 700, 100, 0, 0, 0, len])),
        ("an app tile's label", call("app_tile", &[0, 0, 220, 260, 0, 0, 0, len, 0])),
        ("a text field's text", call("text_field", &[0, 0, 900, 100, 0, 0, 0, len, 0, 0])),
        ("a text field's hint", call("text_field", &[0, 0, 900, 100, 0, 0, 0, 0, 0, len])),
    ]
}

/// One call of each drawing command there is, small: its name, and the call.
fn last_commands() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("fill_rect", call("fill_rect", &[0, 0, 8, 8, 0])),
        ("outline_rect", call("outline_rect", &[0, 0, 40, 40, 2, 0])),
        ("round_rect", call("round_rect", &[0, 0, 40, 40, 8, 0, 1, 0])),
        ("line", line(0.0, 0.0, 40.0, 30.0, 2.0, 0)),
        ("text", call("text", &[0, 0, 400, 60, 0, 4, 34_000, 0, 0, 0])),
        ("text_at", call("text_at", &[0, 60, 0, 4, 34_000, 0, 0])),
        ("icon", call("icon", &[0, 4, 0, 0, 60, 60, 0])),
        ("button", call("button", &[0, 0, 300, 100, 0, 4, 0, 0, 0])),
        ("chip", call("chip", &[0, 0, 200, 80, 0, 4, 0])),
        ("row", call("row", &[0, 0, 600, 100, 0, 0, 0, 4, 0, 1])),
        ("surface", call("surface", &[0, 0, 200, 100, 0, 3])),
        ("select", call("select", &[0, 0, 500, 100, 0, 4, 0, 4])),
        ("app_tile", call("app_tile", &[0, 0, 200, 240, 0, 0, 0, 4, 0])),
        ("fab", call("fab", &[0, 0, 120, 120, 0, 0])),
        ("card", call("card", &[0, 0, 200, 100, 0])),
        ("text_field", call("text_field", &[0, 0, 600, 100, 0, 0, 0, 4, 0, 4])),
        ("clip", call("clip", &[0, 0, 100, 100])),
        ("clip_clear", call("clip_clear", &[])),
    ]
}

#[test]
fn a_call_at_a_limit_runs_and_one_past_it_stops_as_out_of_instructions() {
    let root = fresh_root("limits");
    let one = APP_TEXT_MAX as i32;
    let line = APP_STATUS_MAX as i32;
    let state = APP_STATE_MAX as i32;
    let moves = APP_CALL_STATE_MAX / APP_STATE_MAX;
    assert_eq!((APP_TEXT_MAX, APP_CALL_TEXT_MAX, APP_STATUS_MAX, APP_STATE_MAX, APP_CALL_STATE_MAX), (1024, 65_536, 256, 262_144, 4_194_304));
    // (name, a call at the limit, a call one past it)
    let mut cases: Vec<(String, Vec<u8>, Vec<u8>)> = Vec::new();
    for ((name, at), (_, past)) in texts_of(one).into_iter().zip(texts_of(one + 1)) {
        cases.push((name.to_string(), at, past));
    }
    for ((name, at), (_, past)) in texts_of(-1).into_iter().zip(texts_of(MAX)) {
        cases.push((format!("{name} of every length an i32 can say"), at, past));
    }
    // All the texts of a call: sixty-four of the longest, measured (the call's budget pays for those: it does not
    // pay for sixty-four of the longest labels), and then one byte more, of each kind in turn.
    let most = times(APP_CALL_TEXT_MAX / APP_TEXT_MAX, &call("text_width", &[0, one, 20_000, 0]));
    for (name, more) in texts_of(1) {
        cases.push((format!("all the texts of a call, the last {name}"), most.clone(), seq(&[most.clone(), more])));
    }
    // The drawing commands of a call: 4,096 of them, and one more; and each kind of command as the last.
    let commands = APP_CALL_COMMANDS_MAX;
    assert_eq!(commands, 4096);
    let small = call("fill_rect", &[0, 0, 8, 8, 0]);
    cases.push(("drawing commands".into(), times(commands, &small), times(commands + 1, &small)));
    for (name, last) in last_commands() {
        cases.push((format!("{name} as the last command"), seq(&[times(commands - 1, &small), last.clone()]), seq(&[times(commands, &small), last])));
    }
    cases.push(("the status line".into(), call("status", &[0, line]), call("status", &[0, line + 1])));
    cases.push(("a status line of every length".into(), call("status", &[0, -1]), call("status", &[0, MAX])));
    cases.push(("state written".into(), times(moves, &call("state_set", &[0, state])), times(moves + 1, &call("state_set", &[0, state]))));
    cases.push((
        "state read".into(),
        seq(&[call("state_set", &[0, state]), times(moves - 1, &call("state_get", &[0, state]))]),
        seq(&[call("state_set", &[0, state]), times(moves, &call("state_get", &[0, state]))]),
    ));
    cases.push((
        "state read and written by their lengths, whatever is asked".into(),
        times(moves, &call("state_set", &[0, MAX])),
        seq(&[times(moves, &call("state_set", &[0, MAX])), call("state_get", &[0, MAX])]),
    ));
    for (i, (name, at, past)) in cases.into_iter().enumerate() {
        let before = patterned(SCREEN.0, SCREEN.1);
        let mut c = before.clone();
        let mut rt = loaded(&format!("test.at.{i}"), &BOTH, &roomy(at));
        for _ in 0..2 {
            // (twice: the counts start again with each call)
            rt.draw(&mut c, WHOLE, 0);
            assert_eq!(rt.error(), None, "{name}: a call at the limit runs");
        }
        let mut c = before.clone();
        let mut rt = loaded(&format!("test.past.{i}"), &BOTH, &roomy(seq(&[call("status", &[0, 4]), past])));
        rt.draw(&mut c, WHOLE, 0);
        assert_eq!(rt.error(), Some(STOPPED), "{name}: a call one past the limit stops");
        // (it is the limit that stopped it: most of the call's budget is left)
        assert!(rt.fuel_used() < APP_CALL_FUEL / 2, "{name}: {} of the budget", rt.fuel_used());
        // Nothing of the stopped pass is drawn or kept, and the module is not called again.
        assert_eq!(changed_outside(&before, &c, RECT_EMPTY), 0, "{name}");
        assert_eq!((rt.text().as_str(), rt.status()), ("", ""), "{name}");
        rt.draw(&mut c, WHOLE, 0);
        rt.pointer(0, 4, 4, 0);
        assert_eq!(rt.error(), Some(STOPPED), "{name}");
        assert_eq!(changed_outside(&before, &c, RECT_EMPTY), 0, "{name}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_limits_hold_in_a_touch_and_a_touch_leaves_nothing_for_the_next_pass() {
    let root = fresh_root("touch-limits");
    let one = APP_TEXT_MAX as i32;
    // A touch that draws and reads as much text as a call may: what it recorded is dropped when the next call
    // starts, so the pass that follows holds its own commands only and may read as much again.
    let black = call("fill_rect", &[-50, -50, 4000, 4000, 0]);
    let all_texts = times(APP_CALL_TEXT_MAX / APP_TEXT_MAX, &call("text_width", &[0, one, 20_000, 0]));
    let mut w = roomy(seq(&[call("fill_rect", &[10, 10, 20, 20, 0]), all_texts.clone()]));
    w.pointer = seq(&[black.clone(), all_texts.clone()]);
    w.key = seq(&[black, all_texts.clone()]);
    let mut rt = installed("test.touch.draws", &BOTH, &w);
    let mut c = patterned(SCREEN.0, SCREEN.1);
    for _ in 0..2 {
        rt.draw(&mut c, WHOLE, 0);
        rt.pointer(0, 15, 15, 0);
        rt.key(30, 0, true);
        rt.draw(&mut c, WHOLE, 0);
        assert_eq!(rt.error(), None);
        assert_eq!(inked(&c, WHOLE), 20 * 20, "the black of the touch and of the key is not drawn: the pass's own square is");
    }
    // A touch, and a key, past a limit stop the app as any that fails does.
    let mut w = roomy(Vec::new());
    w.pointer = seq(&[all_texts.clone(), call("text_width", &[0, 1, 20_000, 0])]);
    let mut rt = installed("test.touch.past", &BOTH, &w);
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), None);
    rt.pointer(0, 1, 1, 0);
    assert_eq!(rt.error(), Some("The app stopped while handling a touch."));
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), Some("The app stopped while handling a touch."));
    let mut w = roomy(Vec::new());
    w.key = call("status", &[0, APP_STATUS_MAX as i32 + 1]);
    let mut rt = installed("test.key.past", &BOTH, &w);
    rt.draw(&mut c, WHOLE, 0);
    rt.key(30, 0, true);
    assert_eq!(rt.error(), Some("The app stopped while handling a key."));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_host_reads_no_more_of_a_modules_memory_than_a_limit_lets_it() {
    let root = fresh_root("reads");
    let mut c = patterned(SCREEN.0, SCREEN.1);
    // (eight pages of memory, so that a length past a limit still lies in the memory: what is read is the limit's)
    let mut read = |id: &str, capabilities: &[&str], draw: Vec<u8>| {
        let mut rt = loaded(&format!("test.reads.{id}"), capabilities, &roomy(draw));
        rt.draw(&mut c, WHOLE, 0);
        (rt.error().map(str::to_string), rt.bytes_read())
    };
    let (page, both) = (65536, &BOTH[..]);
    let stopped = Some(STOPPED.to_string());
    // state_set keeps the first 256 KiB of what it is given, and reads those alone, however long it is told.
    assert_eq!(read("state.kept", both, call("state_set", &[0, 100])), (None, 100));
    assert_eq!(read("state.all", both, call("state_set", &[0, 4 * page])), (None, APP_STATE_MAX));
    assert_eq!(read("state.more", both, call("state_set", &[0, 8 * page])), (None, APP_STATE_MAX));
    assert_eq!(read("state.most", both, call("state_set", &[0, MAX])), (None, APP_STATE_MAX));
    assert_eq!(read("state.none", &[NETWORK], call("state_set", &[0, 100])), (None, 0));
    // A setting's key is read when it is as long as a key can be, and not at all when it is longer.
    let key = WIDGET_SETTINGS_KEY_MAX as i32;
    assert_eq!(key, 64);
    assert_eq!(read("key.short", both, call("setting", &[0, 10, 4096, 64])), (None, 10));
    assert_eq!(read("key.longest", both, call("setting", &[0, key, 4096, 64])), (None, 64));
    assert_eq!(read("key.longer", both, call("setting", &[0, key + 1, 4096, 64])), (None, 0));
    assert_eq!(read("key.page", both, call("setting", &[0, page, 4096, 64])), (None, 0));
    assert_eq!(read("key.most", both, call("setting", &[0, MAX, 4096, 64])), (None, 0));
    // An icon's name is read when it is as long as a name can be, and not at all when it is longer.
    let name = APP_ICON_NAME_MAX as i32;
    assert_eq!(name, 64);
    for (import, args) in [("icon", [0, name, 40, 40, 96, 96, 0].to_vec()), ("fab", [40, 40, 160, 160, 0, name].to_vec())] {
        let at = args.iter().position(|a| *a == name).unwrap();
        let with = |len: i32| {
            let mut args = args.clone();
            args[at] = len;
            call(import, &args)
        };
        assert_eq!(read(&format!("name.{import}.short"), both, with(4)), (None, 4), "{import}");
        assert_eq!(read(&format!("name.{import}.longest"), both, with(name)), (None, 64), "{import}");
        assert_eq!(read(&format!("name.{import}.longer"), both, with(name + 1)), (None, 0), "{import}");
        assert_eq!(read(&format!("name.{import}.most"), both, with(MAX)), (None, 0), "{import}");
    }
    assert_eq!(read("name.button", both, call("button", &[0, 0, 400, 120, 0, 4, 0, page, 0])), (None, 4), "the label is read, the name is not");
    assert_eq!(read("name.row", both, call("row", &[0, 0, 900, 120, 0, MAX, 0, 4, 0, -1])), (None, 4));
    assert_eq!(read("name.tile", both, call("app_tile", &[0, 0, 220, 260, 0, name + 1, 0, 4, 0])), (None, 4));
    assert_eq!(read("name.field", both, call("text_field", &[0, 0, 900, 100, 0, name + 1, 0, 4, 0, 4])), (None, 8));
    // A toast's text: its first 256 bytes are read, whatever its length.
    assert_eq!(read("toast.short", both, call("toast", &[0, 100, 1500])), (None, 100));
    assert_eq!(read("toast.at", both, call("toast", &[0, 256, 1500])), (None, APP_TOAST_MAX));
    assert_eq!(read("toast.long", both, call("toast", &[0, 1000, 1500])), (None, APP_TOAST_MAX));
    assert_eq!(read("toast.most", both, call("toast", &[0, MAX, 1500])), (None, APP_TOAST_MAX));
    assert_eq!(read("toast.less", both, call("toast", &[0, MIN, 1500])), (None, 0));
    // The status line and each text: read at their limit, and not read past it (the call is stopped first).
    assert_eq!(read("status.at", both, call("status", &[0, APP_STATUS_MAX as i32])), (None, APP_STATUS_MAX));
    assert_eq!(read("status.past", both, call("status", &[0, APP_STATUS_MAX as i32 + 1])), (stopped.clone(), 0));
    for (i, ((name, at), (_, past))) in texts_of(APP_TEXT_MAX as i32).into_iter().zip(texts_of(8 * page)).enumerate() {
        assert_eq!(read(&format!("text.at.{i}"), both, at), (None, APP_TEXT_MAX), "{name}");
        assert_eq!(read(&format!("text.past.{i}"), both, past), (stopped.clone(), 0), "{name}: nothing is read of a text past the limit");
    }
    // state_get reads nothing of the module's memory (it writes), and what the calls of a pass read adds up.
    assert_eq!(read("sum", both, seq(&[call("state_set", &[0, 300]), call("state_get", &[4096, 300]), call("status", &[0, 50]), call("text_width", &[0, 7, 20_000, 0])])), (None, 357));
    // The connected network's name, the time and the date are the host's own short texts: they are written, never read.
    assert_eq!(read("written", both, seq(&[call("wifi_ssid", &[4096, MAX]), call("time_text", &[4096, MAX]), call("date_text", &[4096, MAX])])), (None, 0));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_icon_name_longer_than_any_icons_names_no_icon() {
    let root = fresh_root("names");
    // A name of 65 bytes is drawn as a name no icon has is: the grid. (A known name of four letters with spaces
    // after it is that icon while it is 64 bytes long, and the grid at 65.)
    let mut padded = b"gear".to_vec();
    padded.resize(80, b' ');
    let drawn = |id: &str, len: i32| {
        let mut rt = installed(id, &BOTH, &Wasm::with(seq(&[call("icon", &[0, len, 40, 40, 200, 200, 0]), call("fab", &[300, 40, 200, 200, 0, len])]), &padded));
        let mut c = patterned(SCREEN.0, SCREEN.1);
        rt.draw(&mut c, WHOLE, 0);
        assert_eq!(rt.error(), None, "{id}");
        c
    };
    let unknown = {
        let mut rt = installed("test.names.unknown", &BOTH, &Wasm::with(seq(&[call("icon", &[0, 7, 40, 40, 200, 200, 0]), call("fab", &[300, 40, 200, 200, 0, 7])]), b"no-such"));
        let mut c = patterned(SCREEN.0, SCREEN.1);
        rt.draw(&mut c, WHOLE, 0);
        c
    };
    let gear = drawn("test.names.gear", 4);
    assert!(gear.px != unknown.px);
    assert!(drawn("test.names.padded", 64).px == gear.px, "a name of 64 bytes is read");
    assert!(drawn("test.names.longer", 65).px == unknown.px, "a name of 65 bytes names no icon");
    assert!(drawn("test.names.most", MAX).px == unknown.px);
    // So it is for the icon of each of the kit's elements that has one: a name of 65 bytes is drawn as a name no
    // icon has, the grid, and not as the element with no icon, which a button, a row and a text field are with a
    // name of no bytes. (An app tile with no name shows the grid too.)
    let elements: [(&str, fn(i32) -> Vec<i32>, bool); 4] = [
        ("button", |len| vec![40, 40, 300, 300, 0, 0, 0, len, 0], true),
        ("row", |len| vec![40, 40, 1200, 160, 0, len, 0, 0, 0, -1], true),
        ("text_field", |len| vec![40, 40, 900, 120, 0, len, 0, 0, 0, 0], true),
        ("app_tile", |len| vec![40, 40, 220, 260, 0, len, 0, 0, 0], false),
    ];
    for (import, args, none_differs) in elements {
        let element = |what: &str, name: &[u8], len: i32| {
            let mut rt = loaded(&format!("test.names.{import}.{what}").replace('_', "-"), &BOTH, &Wasm::with(call(import, &args(len)), name));
            let mut c = patterned(SCREEN.0, SCREEN.1);
            rt.draw(&mut c, WHOLE, 0);
            assert_eq!(rt.error(), None, "{import} {what}");
            c
        };
        let (unknown, gear) = (element("unknown", b"no-such", 7), element("gear", &padded, 4));
        assert!(gear.px != unknown.px, "{import}");
        assert!(element("padded", &padded, 64).px == gear.px, "{import}: a name of 64 bytes is read");
        assert!(element("longer", &padded, 65).px == unknown.px, "{import}: a name of 65 bytes names no icon, and the grid is drawn");
        assert!(element("most", &padded, MAX).px == unknown.px, "{import}");
        if none_differs {
            assert!(element("none", b"", 0).px != unknown.px, "{import}: with no name it has no icon");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn request_redraw_has_a_shortest_and_a_longest_delay_and_one_request_stands() {
    let root = fresh_root("redraw");
    let mut c = patterned(SCREEN.0, SCREEN.1);
    assert_eq!((APP_REDRAW_MIN_MS, APP_REDRAW_MAX_MS), (100, 86_400_000));
    // (times are the app's clock, `now_ms`: milliseconds that wrap)
    let after = |t: u32, ms: i32| t.wrapping_add(ms as u32);
    for (k, (asked, given)) in [(MIN, 100), (-5, 100), (0, 100), (1, 100), (16, 100), (99, 100), (100, 100), (101, 101), (5000, 5000), (86_400_000, 86_400_000), (86_400_001, 86_400_000), (MAX, 86_400_000)].into_iter().enumerate() {
        let mut rt = installed(&format!("test.redraw.{k}"), &[], &asks(asked));
        let t0 = now_ms();
        assert!(!rt.redraw_due(after(t0, APP_REDRAW_MAX_MS)), "nothing is asked for before the first pass");
        rt.draw(&mut c, WHOLE, 0);
        let t1 = now_ms();
        assert!(!rt.redraw_due(after(t0, given - 1)), "request_redraw({asked}) is not due before {given} ms");
        assert!(rt.redraw_due(after(t1, given)), "request_redraw({asked}) is due after {given} ms");
        // The request is taken once, when it is due and not before; the pass after it asks again.
        assert!(!rt.redraw_taken(after(t0, given - 1)));
        assert!(rt.redraw_taken(after(t1, given)));
        assert!(!rt.redraw_taken(after(t1, given)) && !rt.redraw_due(after(t1, APP_REDRAW_MAX_MS)), "taken once");
        rt.draw(&mut c, WHOLE, 0);
        assert!(rt.redraw_due(after(now_ms(), given)), "the next pass asks again");
    }
    // One request stands, the soonest asked for, whichever call asked first.
    for (k, (first, then)) in [(5000, 300), (300, 5000)].into_iter().enumerate() {
        let mut rt = installed(&format!("test.redraw.two.{k}"), &[], &Wasm::new(seq(&[call("request_redraw", &[first]), call("request_redraw", &[then])])));
        let t0 = now_ms();
        rt.draw(&mut c, WHOLE, 0);
        let t1 = now_ms();
        assert!(!rt.redraw_due(after(t0, 299)) && rt.redraw_due(after(t1, 300)), "{first} then {then}: due after 300 ms");
        assert!(rt.redraw_taken(after(t1, 300)));
        assert!(!rt.redraw_due(after(t1, APP_REDRAW_MAX_MS)), "{first} then {then}: one request, taken");
    }
    // What a touch or a key asked for stands through the drawing pass that follows it, which asks for nothing,
    // and through any number of passes; it is due at its time, and taken once.
    let mut rt = installed("test.redraw.touched", &[], &asks_when_touched(700));
    rt.draw(&mut c, WHOLE, 0);
    assert!(!rt.redraw_due(after(now_ms(), APP_REDRAW_MAX_MS)), "a pass that asks for nothing leaves nothing due");
    let t0 = now_ms();
    rt.pointer(0, 5, 5, 0);
    let t1 = now_ms();
    for _ in 0..3 {
        rt.draw(&mut c, WHOLE, 0);
    }
    assert!(!rt.redraw_due(after(t0, 699)) && rt.redraw_due(after(t1, 700)));
    // (a second touch, later, does not put the first one's request off)
    rt.pointer(2, 5, 5, 0);
    assert!(rt.redraw_due(after(t1, 700)));
    assert!(rt.redraw_taken(after(t1, 700)));
    rt.draw(&mut c, WHOLE, 0);
    assert!(!rt.redraw_due(after(now_ms(), APP_REDRAW_MAX_MS)), "taken once: the pass it brought asks for nothing");
    // A module that asked and then stopped is not drawn again: nothing is due for it.
    let mut rt = installed("test.redraw.stops", &[], &Wasm::new(seq(&[call("request_redraw", &[100]), call("status", &[0, APP_STATUS_MAX as i32 + 1])])));
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), Some(STOPPED));
    assert!(!rt.redraw_due(after(now_ms(), APP_REDRAW_MAX_MS)) && !rt.redraw_taken(after(now_ms(), APP_REDRAW_MAX_MS)));
    let _ = std::fs::remove_dir_all(&root);
}

/// What a call's budget is charged for `with` beside `without` (two modules of the same instructions with other
/// numbers): the fuel a pass of the one used less the fuel a pass of the other used, on the whole screen.
fn charged_by(id: &str, capabilities: [&[&str]; 2], with: Vec<u8>, without: Vec<u8>) -> u64 {
    let mut c = patterned(SCREEN.0, SCREEN.1);
    let mut used = Vec::new();
    for (k, (code, caps)) in [(with, capabilities[0]), (without, capabilities[1])].into_iter().enumerate() {
        let mut rt = loaded(&format!("test.charge.{}.{k}", id.replace(['_', ' '], "-")), caps, &roomy(code));
        rt.draw(&mut c, WHOLE, 0);
        assert_eq!(rt.error(), None, "{id}");
        used.push(rt.fuel_used());
    }
    used[0].saturating_sub(used[1])
}

fn charged(id: &str, with: Vec<u8>, without: Vec<u8>) -> u64 {
    charged_by(id, [&BOTH, &BOTH], with, without)
}

/// A charge is the one expected. (The two passes compared are the same instructions with other numbers, and the
/// interpreter counts some tens of instructions more or less for the numbers a function holds.)
fn near(charged: u64, expected: u64, what: &str) {
    assert!(charged.abs_diff(expected) <= 64, "{what}: charged {charged}, expected {expected}");
}

#[test]
fn an_import_takes_its_charge_from_the_calls_budget() {
    let root = fresh_root("charges");
    // (the numbers of docs/APPS.md, "What a call's budget pays for")
    assert_eq!((APP_CALL_FUEL, APP_CHARGE_CALL, APP_CHARGE_SYSTEM, APP_CHARGE_REQUEST), (30_000_000, 500, 20_000, 7_500_000));
    assert_eq!((APP_CHARGE_FILL_PX, APP_CHARGE_EDGE_PX, APP_CHARGE_CORNER, APP_CHARGE_LINE_ROW), (32, 2, 32, 16));
    assert_eq!((APP_CHARGE_TEXT, APP_CHARGE_TALL_PX, APP_CHARGE_BOLD, APP_CHARGE_SQUARE, APP_CHARGE_SQUARE_BYTES, APP_CHARGE_LABEL_PX), (32, 64, 128, 16, 256, 48));
    assert_eq!((APP_CHARGE_ICON_PX, APP_CHARGE_SMALL_ICON_PX, APP_CHARGE_SWITCH, APP_CHARGE_MEASURE, APP_CHARGE_BYTE, APP_CHARGE_COPY_BYTES), (2, 64, 30_000, 32, 4, 4));
    let (body, fixed) = ((SCREEN.0 * SCREEN.1) as u64, 500);

    // Every call of an import: 500, and the instructions of the call itself (a few tens). The calls that ask the
    // system for something: 20,000 more.
    let nothing: Vec<(&str, Vec<i32>)> = vec![
        ("text_width", vec![0, 0, 34_000, 0]),
        ("text_height", vec![34_000, 0]),
        ("clip", vec![0, 0, 100, 100]),
        ("clip_clear", vec![]),
        ("state_get", vec![4096, 0]),
        ("state_set", vec![0, 0]),
        ("setting", vec![0, 0, 4096, 64]),
        ("status", vec![0, 0]),
        ("request_redraw", vec![1000]),
        ("show_status_bar", vec![0]),
        ("toast", vec![0, 0, 1500]),
        ("now", vec![]),
        ("now_ms", vec![]),
        ("wifi_connected", vec![]),
        ("wifi_ssid", vec![4096, 64]),
        ("screen_width", vec![]),
        ("screen_height", vec![]),
        ("ui_scale_milli", vec![]),
        ("theme", vec![]),
        ("status_bar_height", vec![]),
        ("http_status", vec![]),
        ("http_body_len", vec![]),
        ("http_body", vec![4096, 64]),
        ("http_error", vec![4096, 64]),
        ("http_cancel", vec![]),
    ];
    for (import, args) in &nothing {
        let one = call(import, args);
        let n = charged(import, times(2, &one), one.clone());
        assert!((fixed..fixed + 128).contains(&n), "{import}: {n}");
    }
    for import in ["time_text", "date_text"] {
        let one = call(import, &[4096, 64]);
        let n = charged(import, times(2, &one), one.clone());
        assert!((20_500..20_628).contains(&n), "{import}: {n}");
    }
    for import in ["battery_percent", "battery_charging", "http_poll"] {
        let one = call(import, &[]);
        let n = charged(import, times(2, &one), one.clone());
        assert!((20_500..20_628).contains(&n), "{import}: {n}");
    }

    // A rectangle and an outline: 1 for every 32 pixels of the part inside the body (here the whole screen).
    let fill = |x: i32, y: i32, w: i32, h: i32| call("fill_rect", &[x, y, w, h, 0]);
    near(charged("fill", fill(10, 10, 320, 200), fill(10, 10, 0, 0)), 320 * 200 / 32, "a rectangle");
    near(charged("fill.part", fill(-50, -30, 320, 200), fill(10, 10, 0, 0)), 270 * 170 / 32, "the part of it inside the body");
    near(charged("fill.body", fill(-5000, -5000, 20_000, 20_000), fill(0, 0, 0, 0)), body / 32, "the body");
    near(charged("fill.most", fill(MIN, MIN, MAX, MAX), fill(-5000, -5000, 20_000, 20_000)), 0, "the body, however large the rectangle");
    near(charged("fill.outside", fill(1860, 0, 500, 500), fill(0, 0, 0, 0)), 0, "a rectangle outside the body");
    near(charged("outline", call("outline_rect", &[10, 10, 320, 200, 4, 0]), call("outline_rect", &[10, 10, 0, 0, 4, 0])), 320 * 200 / 32, "an outline");
    // A round rectangle: and 2 for each pixel within its radius, or its outline's thickness, and 2 px of its
    // left and right sides.
    let round = |w: i32, h: i32, radius: i32, filled: i32, t: i32| call("round_rect", &[0, 0, w, h, radius, 0, filled, t]);
    let flat = 400 * 300 / 32;
    near(charged("round", round(400, 300, 50, 1, 0), round(0, 0, 50, 1, 0)), flat + 300 * (2 * 52) * 2, "a radius of 50");
    near(charged("round.none", round(400, 300, 0, 1, 0), round(0, 0, 0, 1, 0)), flat + 300 * (2 * 2) * 2, "no radius");
    near(charged("round.most", round(400, 300, 5000, 1, 0), round(0, 0, 0, 1, 0)), flat + 300 * (2 * 152) * 2, "a radius is half the smaller side at most");
    near(charged("round.all", round(400, 300, MAX, 1, 0), round(400, 240, MAX, 1, 0)), 400 * 60 / 32 + (300 * 304 - 240 * 244) * 2, "a taller rectangle has more of them");
    near(charged("round.thick", round(400, 300, 0, 0, 20), round(0, 0, 0, 0, 20)), flat + 300 * (2 * 22) * 2, "an outline 20 thick");
    near(charged("round.thickest", round(400, 300, 0, 0, MAX), round(0, 0, 0, 0, MAX)), flat + 300 * 400 * 2, "an outline thicker than the rectangle");
    // A line: for each row it crosses, 16 and 2 for each pixel of its width along the row.
    near(charged("line.level", line(0.0, 100.0, 400.0, 100.0, 4.0, 0), line(-500.0, -500.0, -400.0, -500.0, 4.0, 0)), 7 * (16 + 404 * 2), "a level line 4 wide: 7 rows of 404 pixels");
    near(charged("line.slant", line(0.0, 0.0, 300.0, 400.0, 4.0, 0), line(-500.0, -500.0, -400.0, -500.0, 4.0, 0)), 404 * (16 + 11 * 2), "a slanted line: 404 rows of 11 pixels");
    near(charged("line.wide", line(0.0, 0.0, 1860.0, 2480.0, 9000.0, 0), line(-500.0, -500.0, -400.0, -500.0, 4.0, 0)), 2480 * (16 + 1860 * 2), "a line as wide as a line may be: the body");
    near(charged("line.none", line(f32::NAN, 0.0, 300.0, 400.0, 4.0, 0), line(-500.0, -500.0, -400.0, -500.0, 4.0, 0)), 0, "no line");

    // A text: for each byte, 32 for each pixel of its height to 64 px, and above that half the square of the
    // height; and 16 for the square of its bytes, counted to 256, when it is wider than its room.
    let text = |w: i32, len: i32, px_milli: i32| call("text", &[0, 0, w, 100, 0, len, px_milli, 0, 0, 0]);
    let none = text(60_000, 0, 40_000);
    near(charged("text", text(60_000, 10, 40_000), none.clone()), 10 * 40 * 32, "ten letters 40 px high");
    near(charged("text.part", text(60_000, 10, 34_500), none.clone()), 10 * 35 * 32, "a part of a pixel counts as one");
    near(charged("text.least", text(60_000, 10, -7), none.clone()), 10 * 32, "a height under a pixel is paid as one");
    near(charged("text.64", text(60_000, 4, 64_000), none.clone()), 4 * 64 * 32, "64 px");
    near(charged("text.tall", text(60_000, 4, 128_000), none.clone()), 4 * 128 * 128 / 2, "128 px");
    near(charged("text.tallest", text(60_000, 4, 768_000), none.clone()), 4 * 768 * 768 / 2, "768 px");
    near(charged("text.taller", text(60_000, 4, MAX), none.clone()), 4 * 768 * 768 / 2, "a height over 768 px is drawn, and paid, as 768");
    near(charged("text.cut", text(20, 10, 40_000), none.clone()), 10 * 40 * 32 + 16 * 10 * 10, "ten letters wider than their room");
    near(charged("text.cut.long", text(20, 400, 1000), none.clone()), 400 * 32 + 16 * 256 * 256, "its bytes are counted to 256 in the square");
    near(charged("text.fits.long", text(60_000, 400, 1000), none.clone()), 400 * 32, "a long text that fits pays no square");
    // A text in the bold, the title or the caps face: and, for each byte, the cube of its height over 128. The
    // other faces are charged as the regular one.
    let faced = |len: i32, px_milli: i32, face: i32| call("text", &[0, 0, 60_000, 100, 0, len, px_milli, face, 0, 0]);
    for face in [1, 2, 3] {
        near(charged(&format!("text.face{face}.small"), faced(10, 40_000, face), faced(0, 40_000, face)), 10 * 40 * 32 + 10 * 40 * 40 * 40 / 128, "40 px");
        near(charged(&format!("text.face{face}.128"), faced(4, 128_000, face), faced(0, 128_000, face)), 4 * 128 * 128 / 2 + 4 * 128 * 128, "128 px");
        near(charged(&format!("text.face{face}.768"), faced(4, 768_000, face), faced(0, 768_000, face)), 4 * 768 * 768 / 2 + 4 * 768 * 768 * 6, "768 px");
        near(charged(&format!("text.face{face}.most"), faced(4, MAX, face), faced(0, MAX, face)), 4 * 768 * 768 / 2 + 4 * 768 * 768 * 6, "a height over 768 px is paid as 768");
    }
    for face in [0, 4, 5, 6, -1] {
        near(charged(&format!("text.plain{}.768", face + 1), faced(4, 768_000, face), faced(0, 768_000, face)), 4 * 768 * 768 / 2, "a face that is not printed over");
    }
    near(charged("text_at.bold", call("text_at", &[0, 100, 0, 4, 768_000, 3, 0]), call("text_at", &[0, 100, 0, 0, 768_000, 3, 0])), 4 * 768 * 768 / 2 + 4 * 768 * 768 * 6, "a text at a place in the caps face");
    let at = |len: i32, px_milli: i32| call("text_at", &[0, 100, 0, len, px_milli, 0, 0]);
    near(charged("text_at", at(10, 40_000), at(0, 40_000)), 10 * 40 * 32, "a text at a place");
    near(charged("text_at.long", at(40, 768_000), at(0, 768_000)), 40 * 768 * 768 / 2, "at 768 px");
    near(charged("text_at.many", at(400, 1000), at(0, 1000)), 400 * 32, "is never cut: no square, however long it is");
    // What text_width measures: 32 for each byte.
    near(charged("text_width", call("text_width", &[0, 100, 34_000, 0]), call("text_width", &[0, 0, 34_000, 0])), 100 * 32, "a hundred bytes measured");
    near(charged("text_width.tall", call("text_width", &[0, 100, 768_000, 0]), call("text_width", &[0, 0, 768_000, 0])), 100 * 32, "at any height");

    // An icon: 2 for each pixel of the square of its side (the smaller side of its rectangle, 1,024 px at most),
    // and 4 for each byte of its name.
    let icon = |len: i32, w: i32, h: i32| call("icon", &[0, len, 0, 0, w, h, 0]);
    near(charged("icon", icon(0, 96, 96), icon(0, 0, 0)), 96 * 96 * 2 - 2, "an icon of 96 px (one of no size pays for 1 px)");
    near(charged("icon.side", icon(0, 96, 4000), icon(0, 0, 0)), 96 * 96 * 2 - 2, "the smaller side");
    near(charged("icon.small", icon(0, 40, 40), icon(0, 0, 0)), 40 * 40 * 2 - 2, "a small one");
    near(charged("icon.most", icon(0, 4000, 4000), icon(0, 0, 0)), 1024 * 1024 * 2 - 2, "1,024 px at most");
    near(charged("icon.name", icon(40, 96, 96), icon(0, 96, 96)), 40 * 4, "its name");
    near(charged("icon.name.long", icon(65, 96, 96), icon(0, 96, 96)), 0, "a name longer than any is not read");

    // The kit's elements. A button: three frames (its fill, its outline, its inner line), each its pixels by
    // rows and those within 32 px of its sides one by one; its label as a text 48 px high with all its bytes in
    // the square; and its icon, when it has one, as large as its smaller side.
    let frame = |w: u64, h: u64, edge: u64| w * h / 32 + h * w.min(2 * edge) * 2;
    let button = |w: i32, h: i32, label: i32, icon: i32| call("button", &[0, 0, w, h, 0, label, 0, icon, 0]);
    near(charged("button", button(400, 120, 0, 0), button(0, 0, 0, 0)), 3 * frame(400, 120, 32), "a button");
    near(charged("button.label", button(400, 120, 10, 0), button(400, 120, 0, 0)), 10 * 48 * 32 + 16 * 10 * 10, "its label");
    near(charged("button.label.long", button(400, 120, 400, 0), button(400, 120, 0, 0)), 400 * 48 * 32 + 16 * 400 * 400, "all a label's bytes are in the square");
    near(charged("button.icon", button(400, 120, 0, 4), button(400, 120, 0, 0)), 120 * 120 * 2 + 4 * 4, "its icon, and the icon's name");
    // A chip, a select and a text field have round ends: two frames, each worked out pixel by pixel within half
    // the smaller side and 32 px of its sides. A label of one line counts its bytes to 256 in the square.
    let chip = |w: i32, h: i32, label: i32| call("chip", &[0, 0, w, h, 0, label, 0]);
    near(charged("chip", chip(260, 80, 0), chip(0, 0, 0)), 2 * frame(260, 80, 40 + 32), "a chip");
    near(charged("chip.label", chip(260, 80, 400), chip(260, 80, 0)), 400 * 48 * 32 + 16 * 256 * 256, "its label");
    let select = |w: i32, h: i32, caption: i32, value: i32| call("select", &[0, 0, w, h, 0, caption, 0, value]);
    near(charged("select", select(660, 100, 0, 0), select(0, 0, 0, 0)), 2 * frame(660, 100, 50 + 32), "a select");
    near(charged("select.texts", select(660, 100, 4, 6), select(660, 100, 0, 0)), 4 * 48 * 32 + 16 * 4 * 4 + 6 * 48 * 32 + 16 * 6 * 6, "its caption and its value");
    let field = |w: i32, h: i32, icon: i32, text: i32, hint: i32| call("text_field", &[0, 0, w, h, 0, icon, 0, text, 0, hint]);
    near(charged("field", field(900, 100, 0, 0, 0), field(0, 0, 0, 0, 0)), 2 * frame(900, 100, 50 + 32), "a text field");
    near(charged("field.icon", field(900, 100, 4, 0, 0), field(900, 100, 0, 0, 0)), 100 * 100 * 2 + 4 * 4, "its icon");
    // (a text field's icon is as tall as the field, whatever the field's width, so it is paid for by the field's
    // height: 1,024 px at most, as any icon, and 64 px at least, as the kit's small icons)
    near(charged("field.icon.narrow", field(1, 1314, 4, 0, 0), field(1, 1314, 0, 0, 0)), 1024 * 1024 * 2 + 4 * 4, "the icon of a field 1 px wide and 1,314 px high");
    near(charged("field.icon.tall", field(40, 500, 4, 0, 0), field(40, 500, 0, 0, 0)), 500 * 500 * 2 + 4 * 4, "the icon of a field 40 px wide and 500 px high");
    near(charged("field.icon.flat", field(900, 10, 4, 0, 0), field(900, 10, 0, 0, 0)), 64 * 64 * 2 + 4 * 4, "the icon of a field 10 px high");
    near(charged("field.icon.none", field(0, 0, 4, 0, 0), field(0, 0, 0, 0, 0)), 64 * 64 * 2 + 4 * 4, "the icon of a field of no size");
    near(charged("field.texts", field(900, 100, 0, 5, 6), field(900, 100, 0, 0, 0)), 5 * 48 * 32 + 16 * 5 * 5 + 6 * 48 * 32 + 16 * 6 * 6, "its text and its hint");
    // A select's chevron and the mark that clears a text field, each an icon of 64 px: against a chip of the same
    // size, which is the same two round frames with no icon.
    near(charged("select.chevron", select(660, 100, 0, 0), chip(660, 100, 0)), 64 * 64 * 2, "a select's chevron");
    near(charged("select.chevron.none", select(0, 0, 0, 0), chip(0, 0, 0)), 64 * 64 * 2, "a select of no size has its chevron");
    near(charged("field.mark", field(900, 100, 0, 0, 0), chip(900, 100, 0)), 64 * 64 * 2, "the mark that clears a text field");
    near(charged("field.mark.none", field(0, 0, 0, 0, 0), chip(0, 0, 0)), 64 * 64 * 2, "a text field of no size has its mark");
    // A row: its card (one frame), its icon and its switch (each of one size), and its label on up to two lines.
    let row = |w: i32, h: i32, icon: i32, label: i32, switch: i32| call("row", &[0, 0, w, h, 0, icon, 0, label, 0, switch]);
    near(charged("row", row(900, 120, 0, 0, -1), row(0, 0, 0, 0, -1)), frame(900, 120, 32), "a row");
    near(charged("row.switch", row(900, 120, 0, 0, 1), row(900, 120, 0, 0, -1)), 30_000, "its switch");
    near(charged("row.icon", row(900, 120, 4, 0, -1), row(900, 120, 0, 0, -1)), 64 * 64 * 2 + 4 * 4, "its icon");
    near(charged("row.label", row(900, 120, 0, 300, -1), row(900, 120, 0, 0, -1)), 300 * 48 * 32 + 16 * 300 * 300, "its label");
    // A surface: its fill, and its outline as thick as it is given. A card: one frame. An app tile: two frames,
    // its icon as large as its smaller side, its label on one line. A floating button: one frame with round
    // ends, and an icon half its size.
    let surface = |w: i32, h: i32, filled: i32, t: i32| call("surface", &[0, 0, w, h, filled, t]);
    near(charged("surface", surface(500, 160, 0, 3), surface(0, 0, 0, 3)), frame(500, 160, 32) + frame(500, 160, 35), "a surface with an outline 3 thick");
    near(charged("surface.thick", surface(500, 160, 0, 5000), surface(0, 0, 0, 5000)), frame(500, 160, 32) + frame(500, 160, 5032), "a thick outline");
    near(charged("surface.filled", surface(500, 160, 1, 5000), surface(0, 0, 1, 5000)), 2 * frame(500, 160, 32), "a filled one has no outline of the module's");
    near(charged("card", call("card", &[0, 0, 500, 200, 0]), call("card", &[0, 0, 0, 0, 0])), frame(500, 200, 32), "a card");
    let tile = |w: i32, h: i32, label: i32| call("app_tile", &[0, 0, w, h, 0, 0, 0, label, 0]);
    near(charged("tile", tile(220, 260, 0), tile(0, 0, 0)), 2 * frame(220, 260, 32) + 220 * 220 * 2 - 2, "an app tile and its icon");
    near(charged("tile.label", tile(220, 260, 5), tile(220, 260, 0)), 5 * 48 * 32 + 16 * 5 * 5, "its label");
    near(charged("fab", call("fab", &[0, 0, 160, 160, 0, 0]), call("fab", &[0, 0, 0, 0, 0, 0])), frame(160, 160, 80 + 32) + 80 * 80 * 2 - 2, "a floating button and its icon");

    // The status line, a toast and a setting's key: 4 for each byte. The saved state, and what a request reads
    // and gives: 1 for every 4 bytes.
    near(charged("status", call("status", &[0, 100]), call("status", &[0, 0])), 4 * 100, "the status line");
    near(charged("toast", call("toast", &[0, 100, 1500]), call("toast", &[0, 0, 1500])), 4 * 100, "a toast");
    near(charged("toast.long", call("toast", &[0, 2000, 1500]), call("toast", &[0, 0, 1500])), 4 * 256, "of which 256 bytes are read");
    near(charged("setting", call("setting", &[0, 40, 4096, 64]), call("setting", &[0, 0, 4096, 64])), 4 * 40, "a setting's key");
    near(charged("setting.long", call("setting", &[0, 65, 4096, 64]), call("setting", &[0, 0, 4096, 64])), 0, "no key is that long: nothing is read");
    near(charged("state_set", call("state_set", &[0, 4000]), call("state_set", &[0, 0])), 4000 / 4, "the state written");
    let read = |cap: i32| seq(&[call("state_set", &[0, 4000]), call("state_get", &[8192, cap])]);
    near(charged("state_get", read(8000), read(0)), 4000 / 4, "the state read");
    near(charged("state_get.some", read(1200), read(0)), 1200 / 4, "as much of it as is asked for");
    // A request that is read (the capability declared, none in flight, its lengths within their limits) pays for
    // the start of a curl and for its bytes, whether or not it then starts: this one's address is refused, and
    // nothing is started. Without the capability nothing is read and nothing is paid beside the call.
    let request = call("http_start", &[0, 3, 0, 4, 0, 0, 0, 2000]);
    let n = charged_by("request", [&BOTH, &[STORAGE]], request.clone(), request);
    near(n, 7_500_000 + (3 + 4 + 2000) / 4, "a request");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_settings_value_and_a_requests_answer_are_paid_for_by_their_bytes() {
    use std::io::{Read, Write};
    let root = fresh_root("answers");
    let mut c = patterned(SCREEN.0, SCREEN.1);
    // A setting's value: 4 for each byte written, as many as the module has room for. (One module: its key
    // handler asks with the room the key's code says, so two calls differ by the charge alone.)
    let value = "v".repeat(60);
    let mut w = roomy(Vec::new());
    w.key = seq(&[i(0), i(4), i(4096), arg(0), invoke("setting"), vec![0x1a]]);
    w.data = vec![(0, b"long".to_vec())];
    let module = w.bytes();
    let m = format!("{}settings:\n  - key: long\n    label: Long\n    type: choice\n    choices: [{value}, short]\n    default: {value}\n", manifest("test.answers.setting", "Test", &[], false, &module));
    app_package_install(&m, &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut rt = AppRuntime::open("test.answers.setting", SCREEN.0, SCREEN.1, false).unwrap();
    let mut used = |room: i32| {
        rt.key(room, 0, true);
        assert_eq!(rt.error(), None);
        rt.fuel_used()
    };
    // (a function's first call also pays for making it ready to run: the calls compared come after it)
    used(0);
    let none = used(0);
    assert_eq!(used(64) - none, 4 * 60, "the whole value");
    assert_eq!(used(4000) - none, 4 * 60);
    assert_eq!(used(25) - none, 4 * 25, "as much of it as there is room for");

    // A request's answer and its error text: 1 for every 4 bytes written. A server on this machine answers one
    // request with 4,000 bytes; the app starts the request in a touch, polls it in its drawing passes, and reads
    // the answer in a key's call, into the room the key's code says.
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut asked = [0u8; 2048];
        let _ = s.read(&mut asked);
        let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: 4000\r\nConnection: close\r\n\r\n{}", "x".repeat(4000)).as_bytes());
    });
    let request = |url: &str, read: &'static str| {
        let mut w = roomy(seq(&[call("http_poll", &[]), invoke("http_poll"), i(48), vec![0x6a], vec![0x21, COUNTER as u8], i(600), arg(COUNTER), vec![0x3a, 0x00, 0x00], call("status", &[600, 1])]));
        w.pointer = call("http_start", &[0, 3, 4, url.len() as i32, 0, 0, 0, 0]);
        w.key = seq(&[i(8192), arg(0), invoke(read), vec![0x1a]]);
        w.data = vec![(0, format!("GET {url}").into_bytes())];
        w
    };
    let ended = |rt: &mut AppRuntime, c: &mut Canvas| {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(45);
        loop {
            rt.draw(c, WHOLE, 0);
            assert_eq!(rt.error(), None);
            // (the status line is what http_poll answered, as a digit: 0 while the request runs, 1 once it has ended)
            if rt.status() == "1" {
                return;
            }
            assert!(std::time::Instant::now() < end, "the request did not end (this test starts curl: it must be on the PATH)");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    };
    let mut rt = installed("test.answers.body", &BOTH, &request(&format!("http://127.0.0.1:{port}/"), "http_body"));
    rt.pointer(0, 1, 1, 0);
    assert_eq!(rt.error(), None);
    // (the start of a request that is read: the fixed charge of the call, the start of its curl and its bytes)
    assert!(rt.fuel_used() >= APP_CHARGE_REQUEST + APP_CHARGE_CALL && rt.fuel_used() < APP_CHARGE_REQUEST + APP_CHARGE_CALL + 400, "{}", rt.fuel_used());
    ended(&mut rt, &mut c);
    server.join().unwrap();
    let mut used = |room: i32| {
        rt.key(room, 0, true);
        assert_eq!(rt.error(), None);
        rt.fuel_used()
    };
    used(0);
    let none = used(0);
    assert_eq!(used(4000) - none, 4000 / 4, "the whole answer");
    assert_eq!(used(60_000) - none, 4000 / 4);
    assert_eq!(used(1000) - none, 1000 / 4, "as much of it as there is room for");
    // The error text of a request that failed: nothing listens at the address, a port of this machine that was
    // just closed.
    let closed = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    let mut rt = installed("test.answers.error", &BOTH, &request(&format!("http://127.0.0.1:{closed}/"), "http_error"));
    rt.pointer(0, 1, 1, 0);
    ended(&mut rt, &mut c);
    let mut used = |room: i32| {
        rt.key(room, 0, true);
        assert_eq!(rt.error(), None);
        rt.fuel_used()
    };
    used(0);
    let none = used(0);
    let whole = used(4000) - none;
    assert!((20 / 4..=4096 / 4).contains(&whole), "curl's line for the failure, at 1 for every 4 bytes: {whole}");
    assert_eq!(used(60_000) - none, whole);
    assert_eq!(used(8) - none, 8 / 4, "as much of it as there is room for");
    let _ = std::fs::remove_dir_all(&root);
}

// A function's first call also pays the interpreter for making it ready to run: about 7 for each byte of its code
// (docs/APPS.md, Limits). A drawing pass of 100,000 bytes of code, nothing but `nop`, takes about 700,000 more from
// its first call's budget than from each later one, and the later ones take the same.
#[test]
fn a_functions_first_call_also_pays_for_making_it_ready() {
    let root = fresh_root("first-call");
    let code = 100_000;
    let mut rt = loaded("test.first.call", &BOTH, &Wasm::new(times(code, &[0x01])));
    let mut c = patterned(SCREEN.0, SCREEN.1);
    let mut used = || {
        rt.draw(&mut c, WHOLE, 0);
        assert_eq!(rt.error(), None);
        rt.fuel_used()
    };
    let (first, then, again) = (used(), used(), used());
    assert_eq!(then, again, "a later call takes what the one before it took");
    let ready = first - then;
    assert!(ready.abs_diff(7 * code as u64) <= 64, "the first call took {ready} more, for {code} bytes of code");
    let _ = std::fs::remove_dir_all(&root);
}

/// The canvas after a pass of `draw` on the whole screen, the app's error, and what the pass took from its budget.
fn pass_of(id: &str, draw: Vec<u8>, data: &[u8]) -> (Canvas, Option<String>, u64) {
    let mut w = roomy(draw);
    w.data = vec![(0, data.to_vec())];
    let mut rt = loaded(id, &BOTH, &w);
    let mut c = patterned(SCREEN.0, SCREEN.1);
    rt.draw(&mut c, WHOLE, 0);
    (c, rt.error().map(str::to_string), rt.fuel_used())
}

#[test]
fn the_hosts_work_for_a_call_is_paid_from_the_calls_budget() {
    let root = fresh_root("work");
    let before = patterned(SCREEN.0, SCREEN.1);
    let letters: Vec<u8> = (0..1024).map(|k| 33 + (k % 94) as u8).collect();
    let words: Vec<u8> = (0..1024).map(|k| if k % 2 == 0 { b'a' + (k / 2 % 26) as u8 } else { b' ' }).collect();
    let all = APP_CALL_COMMANDS_MAX as i32;
    // (a number that falls by `step` each turn of the loop it is in, from `from`)
    let falling = |from: i32, step: i32| seq(&[i(from), counter(), i(step), vec![0x6c, 0x6b]]);
    // (sixteen letters from a place that moves on each turn)
    let moving = seq(&[counter(), i(16), vec![0x6c], i(1008), vec![0x70]]);
    // Calls within every count and length a call is held to, each of them long work for the host on the whole
    // screen. The budget does not pay for any: each is stopped as a call out of instructions is, with the budget
    // used up, and nothing of it is drawn.
    let long: Vec<(&str, Vec<u8>, &[u8])> = vec![
        ("4,096 rectangles, each over the whole body", repeat(all, &call("fill_rect", &[-10, -10, 4000, 4000, 0])), &letters),
        ("4,096 round rectangles over the body, their radius half of it", repeat(all, &call("round_rect", &[0, 0, 1860, 2480, 1240, 0, 1, 0])), &letters),
        ("4,096 outlines over the body, 100,000 thick", repeat(all, &call("round_rect", &[-10, -10, 4000, 4000, 0, 0, 0, 100_000])), &letters),
        ("4,096 lines 4,096 wide", repeat(all, &line(0.0, 0.0, 1860.0, 2480.0, 4096.0, 0)), &letters),
        ("4,096 thin lines from corner to corner", repeat(all, &line(0.0, 0.0, 1860.0, 2480.0, 1.0, 0)), &letters),
        ("4,096 surfaces over the body with outlines 100,000 thick", repeat(all, &call("surface", &[-10, -10, 4000, 4000, 0, 100_000])), &letters),
        ("4,096 cards over the body", repeat(all, &call("card", &[0, 0, 1860, 2480, 1])), &letters),
        ("4,096 floating buttons over the body", repeat(all, &call("fab", &[-300, 0, 2480, 2480, 0, 4])), b"gear"),
        ("4,096 chips over the body", repeat(all, &call("chip", &[0, 0, 1860, 2480, 0, 16, 1])), &letters),
        ("4,096 buttons over the body, each with an icon", repeat(all, &call("button", &[0, 0, 1860, 2480, 0, 0, 0, 4, 0])), b"gear"),
        ("4,096 rows over the body", repeat(all, &call("row", &[0, 0, 1860, 2480, 0, 4, 8, 16, 0, 1])), b"gear    a label of a row"),
        ("4,096 selects over the body", repeat(all, &call("select", &[0, 0, 1860, 2480, 8, 8, 16, 8])), &letters),
        ("4,096 app tiles over the body", repeat(all, &call("app_tile", &[0, 0, 1860, 2480, 0, 4, 8, 8, 1])), b"gear    a label"),
        ("4,096 text fields over the body", repeat(all, &call("text_field", &[0, 0, 1860, 2480, 0, 4, 8, 8, 16, 8])), b"gear    typed   hint    "),
        ("3,400 text fields 1 px wide and 1,314 px high, each with its icon", repeat(3400, &call("text_field", &[-1, 0, 1, 1314, 0, 4, 0, 0, 0, 0])), b"gear"),
        ("15 text fields 1 px wide and 1,314 px high, each with its icon", repeat(15, &call("text_field", &[-1, 0, 1, 1314, 0, 4, 0, 0, 0, 0])), b"gear"),
        ("4,096 icons of 1,024 px", repeat(all, &call("icon", &[0, 4, 0, 0, 1280, 1280, 0])), b"gear"),
        ("4,096 icons of every size from 1,024 px down", repeat(all, &seq(&[i(0), i(4), i(0), i(0), seq(&[i(1280), counter(), i(1100), vec![0x70, 0x6b]]), seq(&[i(1280), counter(), i(1100), vec![0x70, 0x6b]]), i(0), invoke("icon")])), b"gear"),
        ("64 texts of 1,024 letters 768 px high", repeat(64, &call("text", &[0, 0, 60_000, 2000, 0, 1024, 768_000, 0, 0, 0])), &letters),
        ("4,096 texts of 4 letters 256 px high in the caps face", repeat(all, &call("text", &[0, 0, 60_000, 2000, 0, 4, 256_000, 3, 0, 0])), &letters),
        ("one text of 16 letters 768 px high in the bold face", call("text", &[0, 0, 60_000, 2000, 0, 16, 768_000, 1, 0, 0]), &letters),
        ("64 texts of 1,024 letters at a place, 768 px high", repeat(64, &call("text_at", &[0, 1500, 0, 1024, 768_000, 0, 0])), &letters),
        ("4,096 texts of 16 letters, each at its own height from 768 px down", repeat(all, &seq(&[i(0), i(0), i(60_000), i(2000), moving.clone(), i(16), falling(768_000, 180), i(0), i(0), i(0), invoke("text")])), &letters),
        ("4,096 texts of 16 letters, each at its own height from 64 px down", repeat(all, &seq(&[i(0), i(0), i(60_000), i(2000), moving.clone(), i(16), falling(64_000, 15), i(0), i(0), i(0), invoke("text")])), &letters),
        ("4,096 texts of 16 letters at a place, each at its own height", repeat(all, &seq(&[i(0), i(1500), moving, i(16), falling(768_000, 180), i(0), i(0), invoke("text_at")])), &letters),
        ("64 texts of 1,024 letters cut short to 300 px", repeat(64, &call("text", &[0, 0, 300, 100, 0, 1024, 34_000, 0, 0, 0])), &words),
        ("64 buttons 9,000 px wide with labels of 512 one-letter words", repeat(64, &call("button", &[0, 0, 9000, 300, 0, 1023, 0, 0, 0])), &words),
        ("64 rows with labels of 512 one-letter words", repeat(64, &call("row", &[0, 0, 1860, 300, 0, 0, 0, 1023, 0, -1])), &words),
        ("64 chips with labels of 1,024 letters", repeat(64, &call("chip", &[0, 0, 1860, 300, 0, 1024, 0])), &letters),
    ];
    for (k, (name, draw, data)) in long.into_iter().enumerate() {
        let (c, error, used) = pass_of(&format!("test.work.{k}"), draw, data);
        assert_eq!(error.as_deref(), Some(STOPPED), "{name}");
        assert_eq!(used, APP_CALL_FUEL, "{name}: the budget is used up");
        assert_eq!(changed_outside(&before, &c, RECT_EMPTY), 0, "{name}: nothing is drawn");
    }
    // What an honest call asks for is paid many times over: a clock's digits, in bold and as high as a text may
    // be, a page full of small text (70 lines of 90 letters), a board of a hundred keys over a background, and a
    // screen of thirty rows on a card. (the_limits_leave_room_for_an_honest_app has the fixture app's pass and the
    // pass of one of every command.)
    let line_of_text = [b'n'; 90];
    let honest_passes: Vec<(&str, Vec<u8>, &[u8], u64)> = vec![
        ("a clock's digits 400 px high, in bold", call("text", &[0, 0, 1860, 900, 0, 5, 400_000, 1, 1, 0]), b"12:34", 8),
        ("a clock's digits as high as a text may be", call("text", &[0, 0, 1860, 900, 0, 5, 768_000, 0, 1, 0]), b"12:34", 15),
        ("a page full of small text", repeat(70, &call("text", &[20, 0, 1820, 34, 0, 90, 34_000, 0, 0, 0])), &line_of_text, 4),
        ("a hundred keys", seq(&[call("fill_rect", &[0, 0, 1860, 2480, 255]), repeat(100, &call("button", &[0, 0, 160, 160, 0, 3, 0, 0, 0]))]), b"sin", 4),
        ("thirty rows on a card", seq(&[call("card", &[20, 20, 1820, 2440, 0]), repeat(30, &call("row", &[60, 60, 1740, 70, 4, 4, 0, 4, 0, 1]))]), b"rowsgear", 10),
        // (a text field's icon is as tall as the field: fourteen fields with an icon of 1,024 px are the most a call
        // pays for, and a search field of a screen's own size is paid for two hundred times over)
        ("14 text fields 1 px wide and 1,314 px high, each with its icon", repeat(14, &call("text_field", &[-1, 0, 1, 1314, 0, 4, 0, 0, 0, 0])), b"gear", 1),
        ("a search field with its icon", call("text_field", &[40, 40, 900, 100, 0, 6, 8, 5, 16, 6]), b"search  query   Search", 200),
    ];
    for (k, (name, draw, data, times_over)) in honest_passes.into_iter().enumerate() {
        let (_, error, used) = pass_of(&format!("test.paid.{k}"), draw, data);
        assert_eq!(error, None, "{name}");
        assert!(used * times_over < APP_CALL_FUEL, "{name}: {used} of the budget");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_limits_leave_room_for_an_honest_app() {
    let root = fresh_root("room");
    let mut c = patterned(SCREEN.0, SCREEN.1);
    // The fixture app of tests/fixtures/apppkg: a rectangle and a line of text.
    let (yaml, module) = fixture();
    let icon = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico")).unwrap();
    app_package_install(&yaml.replace("min_grimoire_version: 2.3.0\n", ""), &module, Some(&icon), "", SCREEN.0, SCREEN.1).unwrap();
    let mut rt = AppRuntime::open("com.example.fixture", SCREEN.0, SCREEN.1, false).unwrap();
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), None);
    // (docs/APPS.md, Limits: 3 commands, 22 bytes read, under 1% of a call's budget)
    assert_eq!((rt.commands(), rt.bytes_read()), (3, 22));
    assert!(rt.fuel_used() > 50_000 && rt.fuel_used() * 100 < APP_CALL_FUEL, "the fixture app's pass: {} of the budget", rt.fuel_used());
    // The pass of one of every command (rust/tests/appfix, `honest`): a page of a settings screen and a small board.
    let cmds = honest();
    let mut rt = installed("test.room.honest", &BOTH, &pass(&cmds));
    rt.draw(&mut c, body_under_the_bar(), 0);
    assert_eq!(rt.error(), None);
    // (47 commands and 348 bytes read, under a tenth of the budget)
    assert_eq!((rt.commands(), rt.bytes_read()), (47, 348));
    assert!(rt.fuel_used() > 1_000_000 && rt.fuel_used() * 10 < APP_CALL_FUEL, "the pass of one of every command: {} of the budget", rt.fuel_used());
    // A touch records nothing that is drawn, and the next pass is the pass again.
    rt.pointer(0, 100, 100, 0);
    rt.draw(&mut c, body_under_the_bar(), 0);
    assert_eq!((rt.error(), rt.commands()), (None, 47));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_loop_of_one_import_is_ended_by_the_calls_budget() {
    let root = fresh_root("import-loops");
    let mut c = patterned(SCREEN.0, SCREEN.1);
    let data = [b'k'; 512];
    // Each import of the ABI in a loop with no way out, with numbers it does something with: in an app with both
    // capabilities and in one with none (where the calls that need a capability return at once), the call is
    // stopped. For the calls that record no drawing command it is the budget that ends it, used up; a loop of a
    // drawing command ends at the commands of a call, or at the budget where that comes first.
    let drawing = ["fill_rect", "outline_rect", "round_rect", "line", "text", "text_at", "icon", "button", "chip", "row", "surface", "select", "app_tile", "fab", "card", "text_field", "clip", "clip_clear"];
    let one = |import: &'static str| -> Vec<u8> {
        if import == "line" {
            return line(10.0, 10.0, 200.0, 40.0, 2.0, 0);
        }
        let (_, params, _) = IMPORTS[appfix::import(import) as usize];
        let args: Vec<i32> = match import {
            "request_redraw" => vec![1000],
            "status" => vec![0, 256],
            "toast" => vec![0, 256, 1500],
            "state_get" | "time_text" | "date_text" | "wifi_ssid" | "http_body" | "http_error" => vec![4096, 64],
            "state_set" => vec![0, 0],
            "setting" => vec![0, 64, 4096, 64],
            "text" => vec![40, 40, 400, 60, 0, 4, 40_000, 0, 0, 0],
            "text_at" => vec![40, 200, 0, 4, 40_000, 0, 0],
            "text_width" => vec![0, 0, 40_000, 0],
            "text_height" => vec![40_000, 0],
            // (a request whose address is refused: nothing is started)
            "http_start" => vec![0, 3, 0, 4, 0, 0, 0, 0],
            _ => (0..params.len()).map(|at| if at % 2 == 0 { 40 } else { 4 }).collect(),
        };
        call(import, &args)
    };
    // (a loop of calls that each pay the fixed charge alone makes at most this many: the budget pays for no more)
    let most = (APP_CALL_FUEL / APP_CHARGE_CALL) as i32;
    assert_eq!(most, 60_000);
    for (import, _, _) in IMPORTS.iter() {
        let records = drawing.contains(import);
        for (kind, capabilities) in [("both", &BOTH[..]), ("none", &[][..])] {
            let mut w = Wasm::with(forever(&one(import)), &data);
            w.memories = vec![(2, None)];
            let mut rt = loaded(&format!("test.forever.{kind}.{}", import.replace('_', "-")), capabilities, &w);
            rt.draw(&mut c, WHOLE, 0);
            assert_eq!(rt.error(), Some(STOPPED), "{import}, {kind}");
            if !records {
                assert!(used_up(&rt), "{import}, {kind}: the budget is used up ({})", rt.fuel_used());
            }
        }
        if records {
            continue;
        }
        // The count: one call more than the budget pays fixed charges for is not made, and a tenth as many are.
        let mut w = Wasm::with(repeat(most + 1, &one(import)), &data);
        w.memories = vec![(2, None)];
        let mut rt = loaded(&format!("test.count.past.{}", import.replace('_', "-")), &[], &w);
        rt.draw(&mut c, WHOLE, 0);
        assert_eq!((rt.error(), used_up(&rt)), (Some(STOPPED), true), "{import}: {} calls ({})", most + 1, rt.fuel_used());
        let system = ["time_text", "date_text", "battery_percent", "battery_charging", "http_poll"].contains(import);
        let some = if system { most / 10 / 41 } else { most / 10 };
        let mut w = Wasm::with(repeat(some, &one(import)), &data);
        w.memories = vec![(2, None)];
        let mut rt = loaded(&format!("test.count.tenth.{}", import.replace('_', "-")), &[], &w);
        rt.draw(&mut c, WHOLE, 0);
        assert_eq!(rt.error(), None, "{import}: {some} calls");
        assert!(rt.fuel_used() >= some as u64 * APP_CHARGE_CALL && rt.fuel_used() < APP_CALL_FUEL / 2, "{import}: {}", rt.fuel_used());
    }
    // The same in a touch, a key and a stop, and in a start function.
    let now = call("now", &[]);
    let mut touched = Wasm::new(Vec::new());
    touched.pointer = forever(&now);
    touched.key = forever(&now);
    touched.stop = forever(&now);
    let mut rt = installed("test.forever.touch", &BOTH, &touched);
    rt.draw(&mut c, WHOLE, 0);
    rt.pointer(0, 1, 1, 0);
    assert_eq!((rt.error(), rt.fuel_used()), (Some("The app stopped while handling a touch."), APP_CALL_FUEL));
    let mut rt = installed("test.forever.key", &BOTH, &touched);
    rt.key(30, 0, true);
    assert_eq!((rt.error(), rt.fuel_used()), (Some("The app stopped while handling a key."), APP_CALL_FUEL));
    let mut rt = installed("test.forever.stop", &BOTH, &touched);
    rt.stop();
    assert_eq!((rt.error(), rt.fuel_used()), (None, APP_CALL_FUEL));
    let mut starts = Wasm::new(Vec::new());
    starts.start = Some(forever(&now));
    install("test.forever.start", &BOTH, &starts.bytes());
    assert_eq!(load("test.forever.start").as_deref(), Some("The app could not start: all fuel consumed by WebAssembly"));
    let _ = std::fs::remove_dir_all(&root);
}

/// A sequence of numbers that is the same on every run (xorshift), for passes of numbers nobody chose.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn of<T: Copy>(&mut self, pool: &[T]) -> T {
        pool[(self.next() % pool.len() as u64) as usize]
    }
    /// A place or a size: one of the numbers at which a sum overflows or a reach ends, or any number near a body.
    fn n(&mut self) -> i32 {
        const EDGES: [i32; 26] = [
            MIN, MIN + 1, -1_073_741_824, -131_073, -131_072, -70_000, -65_537, -65_536, -65_535, -5000, -1, 0, 1, 7, 100, 1860, 2480, 5000, 65_535, 65_536, 65_537, 131_072,
            131_073, 1_073_741_824, MAX - 1, MAX,
        ];
        match self.next() % 3 {
            0 => self.of(&EDGES),
            _ => (self.next() % 4000) as i32 - 1000,
        }
    }
    fn f(&mut self) -> f32 {
        const EDGES: [f32; 14] = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN, 1e30, -1e30, 1e9, -1e9, 70_000.0, -70_000.0, 0.0, -0.0, f32::MIN_POSITIVE];
        match self.next() % 3 {
            0 => self.of(&EDGES),
            _ => (self.next() % 400_000) as f32 / 100.0 - 1000.0,
        }
    }
    fn text(&mut self) -> &'static str {
        const LONG: &str = "many words in a row that go on for longer than any line of any body has room for, so that the line is cut short and, where the element takes two lines, broken into two at one of its spaces";
        self.of(&["", "x", "12:34", "A label", "two words", LONG, "a\0b", "tab\tand\nline", "\u{65e5}\u{672c}\u{8a9e}", "caf\u{e9} \u{1f600}", "   "])
    }
    fn icon(&mut self) -> &'static str {
        self.of(&["", "gear", "md-clock", "no such icon", "files", "search", "  download  "])
    }
    fn cmd(&mut self) -> Cmd {
        let (x, y, w, h) = (self.n(), self.n(), self.n(), self.n());
        let (a, b, c) = (self.n(), self.n(), self.n());
        match self.next() % 18 {
            0 => Cmd::Fill(x, y, w, h, a),
            1 => Cmd::Outline(x, y, w, h, a, b),
            2 => Cmd::Round(x, y, w, h, a, b, c & 1, self.n()),
            3 => Cmd::Line(self.f(), self.f(), self.f(), self.f(), self.f(), a),
            4 => Cmd::Text(x, y, w, h, self.text(), a, b, c & 1, self.n()),
            5 => Cmd::TextAt(x, y, self.text(), a, b, c),
            6 => Cmd::Icon(self.icon(), x, y, w, h, a),
            7 => Cmd::Button(x, y, w, h, self.text(), self.icon(), a),
            8 => Cmd::Chip(x, y, w, h, self.text(), a),
            9 => Cmd::Row(x, y, w, h, self.icon(), self.text(), a, b),
            10 => Cmd::Surface(x, y, w, h, a & 1, b),
            11 => Cmd::Select(x, y, w, h, self.text(), self.text()),
            12 => Cmd::AppTile(x, y, w, h, self.icon(), self.text(), a),
            13 => Cmd::Fab(x, y, w, h, self.icon()),
            14 => Cmd::Card(x, y, w, h, a),
            15 => Cmd::TextField(x, y, w, h, self.icon(), self.text(), self.text()),
            16 => Cmd::Clip(x, y, w, h),
            _ => Cmd::ClipClear,
        }
    }
}

#[test]
fn no_number_an_app_passes_overflows_a_sum_or_draws_outside_its_body() {
    let root = fresh_root("numbers");
    // Every command with the numbers at which a sum of two of them overflows, each once, by name.
    let mut edges = Vec::new();
    for (x, y, w, h) in [(MIN, MIN, MAX, MAX), (MAX, MAX, MAX, MAX), (MIN, MIN, MIN, MIN), (-1, -1, MAX, MAX), (MAX, 0, 10, 10), (0, MAX, 10, 10), (MIN, 0, MAX, 40), (0, 0, -50, -50), (0, 0, 0, 0), (-65_536, -65_536, 131_072, 131_072)] {
        for t in [MIN, -1, 0, 3, MAX] {
            edges.push(Cmd::Fill(x, y, w, h, t));
            edges.push(Cmd::Outline(x, y, w, h, t, 0));
            edges.push(Cmd::Round(x, y, w, h, t, 0, 1, t));
            edges.push(Cmd::Round(x, y, w, h, MAX, 0, 0, t));
            edges.push(Cmd::Surface(x, y, w, h, 0, t));
            edges.push(Cmd::Text(x, y, w, h, "12:34", t, t, 0, 0));
            edges.push(Cmd::TextAt(x, y, "12:34", t, 1, 0));
        }
        edges.push(Cmd::Icon("gear", x, y, w, h, 0));
        edges.push(Cmd::Button(x, y, w, h, "A label", "gear", 0));
        edges.push(Cmd::Chip(x, y, w, h, "A chip", 1));
        edges.push(Cmd::Row(x, y, w, h, "gear", "A row", 0, 1));
        edges.push(Cmd::Select(x, y, w, h, "Sort", "Name"));
        edges.push(Cmd::AppTile(x, y, w, h, "clock", "Clock", 0));
        edges.push(Cmd::Fab(x, y, w, h, "download"));
        edges.push(Cmd::Card(x, y, w, h, 2));
        edges.push(Cmd::TextField(x, y, w, h, "search", "typed", "hint"));
        edges.push(Cmd::Clip(x, y, w, h));
        edges.push(Cmd::Fill(-5000, -5000, 20_000, 20_000, 128));
        edges.push(Cmd::ClipClear);
    }
    for v in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN, 1e30, -1e30] {
        edges.push(Cmd::Line(v, 100.0, 900.0, 800.0, 6.0, 0));
        edges.push(Cmd::Line(100.0, v, 900.0, 800.0, 6.0, 0));
        edges.push(Cmd::Line(100.0, 100.0, v, 800.0, 6.0, 0));
        edges.push(Cmd::Line(100.0, 100.0, 900.0, v, 6.0, 0));
        edges.push(Cmd::Line(100.0, 100.0, 900.0, 800.0, v, 0));
        edges.push(Cmd::Line(v, v, v, v, v, 0));
        edges.push(Cmd::Line(v, v, -v, -v, 3.0, 0));
    }
    // Then passes of numbers nobody chose: sixty commands each, of any kind, on bodies of several shapes (the
    // screen under the bar, a part of it, one of a single pixel, one with no width, one that is partly off the
    // canvas on two sides), in both themes, with the whole canvas and with a part of it as the caller's clip.
    let mut dice = Dice(0x2545_F491_4F6C_DD1D);
    let mut passes: Vec<Vec<Cmd>> = vec![edges];
    passes.extend((0..59).map(|_| (0..60).map(|_| dice.cmd()).collect()));
    let bodies = [
        Rect { x: 300, y: 200, w: 700, h: 900 },
        body_under_the_bar(),
        Rect { x: 500, y: 500, w: 1, h: 1 },
        Rect { x: 500, y: 500, w: 0, h: 300 },
        Rect { x: -200, y: -100, w: 600, h: 500 },
        Rect { x: 1600, y: 2200, w: 600, h: 500 },
    ];
    // Each list is drawn as the passes the call's budget pays for (a command it does not pay for alone, a long
    // text as high as a text may be, is stopped and draws nothing).
    let screen = patterned(SCREEN.0, SCREEN.1);
    let (mut ids, mut drawn, mut all) = (0, 0, 0);
    for (i, cmds) in passes.iter().enumerate() {
        for (b, body) in [bodies[0], bodies[1 + i % 5]].into_iter().enumerate() {
            let theme = if (i + b) % 2 == 0 { UiTheme::Gothic } else { UiTheme::Classic };
            ui_set_theme(theme);
            let part = Rect { x: 100, y: 100, w: 1500, h: 2000 };
            let clip = if i % 3 == 2 { part } else { screen.clip };
            all += cmds.len();
            drawn += drawn_within_the_budget(&mut ids, &screen, cmds, body, Some(clip), &mut |before, after| {
                assert_eq!(changed_outside(before, after, rect_intersect(body, clip)), 0, "pass {i} in {theme:?}, a body at {},{} of {}x{}: pixels outside it changed", body.x, body.y, body.w, body.h);
            });
        }
    }
    eprintln!("numbers nobody chose: {drawn} of {all} commands drawn in {ids} passes");
    assert!(drawn * 10 > all * 9, "most of the commands are ones the budget pays for: {drawn} of {all}");
    ui_set_theme(UiTheme::Gothic);
    let _ = std::fs::remove_dir_all(&root);
}

/// The canvas after one pass, on the whole screen, of these commands.
fn drawn(id: &str, cmds: &[Cmd]) -> Canvas {
    let mut rt = loaded(id, &BOTH, &pass(cmds));
    let mut c = patterned(SCREEN.0, SCREEN.1);
    rt.draw(&mut c, WHOLE, 0);
    assert_eq!(rt.error(), None, "{id}");
    c
}

#[test]
fn a_text_is_drawn_and_measured_at_most_768_px_high() {
    let root = fresh_root("px");
    assert_eq!(APP_TEXT_PX_MAX, 768);
    for theme in [UiTheme::Gothic, UiTheme::Classic] {
        ui_set_theme(theme);
        let t = if theme == UiTheme::Gothic { "g" } else { "c" };
        for (kind, face) in [("text", 0), ("text", 1), ("at", 4)] {
            let letter = |px_milli: i32| -> Vec<Cmd> {
                if kind == "text" {
                    vec![Cmd::Text(0, 0, SCREEN.0, SCREEN.1, "W", px_milli, face, 0, 0)]
                } else {
                    vec![Cmd::TextAt(40, 1400, "W", px_milli, face, 0)]
                }
            };
            let id = |what: &str| format!("test.px.{t}.{kind}{face}.{what}");
            let (at, under) = (drawn(&id("at"), &letter(768_000)), drawn(&id("under"), &letter(767_000 - 20_000)));
            assert!(inked(&at, WHOLE) > 10_000, "{kind} in {theme:?}: a letter 768 px high is drawn");
            assert!(at.px != under.px, "{kind} in {theme:?}: a lower height is another size");
            // One thousandth past the height, and any height past it, is drawn as 768 px is, pixel for pixel.
            for (what, px_milli) in [("past", 768_001), ("double", 1_536_000), ("far", 100_000_000), ("most", MAX)] {
                assert!(drawn(&id(what), &letter(px_milli)).px == at.px, "{kind} in {theme:?}: a text of {px_milli} thousandths is drawn as one 768 px high");
            }
            // The least height is one thousandth of a pixel: nothing under it is another size.
            let least = drawn(&id("least"), &letter(1));
            for (what, px_milli) in [("zero", 0), ("minus", -40_000), ("min", MIN)] {
                assert!(drawn(&id(what), &letter(px_milli)).px == least.px, "{kind} in {theme:?}: {px_milli}");
            }
        }
    }
    ui_set_theme(UiTheme::Gothic);
    // What text_width and text_height answer for a height past the limit is what they answer at it. (The module
    // writes each answer as four bytes at 512 and says them in the status line.)
    let asked = |id: &str, px_milli: i32| -> Vec<u8> {
        let store = vec![0x36, 0x02, 0x00];
        let w = seq(&[i(512), i(0), i(5), i(px_milli), i(0), invoke("text_width"), store.clone()]);
        let h = seq(&[i(516), i(px_milli), i(0), invoke("text_height"), store]);
        let mut rt = loaded(id, &BOTH, &Wasm::with(seq(&[w, h, call("status", &[512, 8])]), b"12:34"));
        let mut c = patterned(SCREEN.0, SCREEN.1);
        rt.draw(&mut c, WHOLE, 0);
        assert_eq!(rt.error(), None);
        rt.status().as_bytes().to_vec()
    };
    let at = asked("test.px.asked.at", 768_000);
    assert_ne!(at, asked("test.px.asked.under", 700_000));
    for (what, px_milli) in [("past", 768_001), ("far", 100_000_000), ("most", MAX)] {
        assert_eq!(asked(&format!("test.px.asked.{what}"), px_milli), at, "{px_milli}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_icon_is_drawn_at_most_1024_px_each_way() {
    let root = fresh_root("icon-size");
    assert_eq!(APP_ICON_PX_MAX, 1024);
    // An icon is drawn a tenth of its rectangle's smaller side inside it, so the icon of a square of 1,280 px is
    // 1,024 px. In a larger rectangle with the same middle the icon is that same one, pixel for pixel.
    let (cx, cy) = (930, 1240);
    let icon = |side: i32| vec![Cmd::Icon("gear", cx - side / 2, cy - side / 2, side, side, 0)];
    let (at, under) = (drawn("test.icon.at", &icon(1280)), drawn("test.icon.under", &icon(1200)));
    assert!(inked(&at, WHOLE) > 100_000);
    assert!(at.px != under.px, "a smaller rectangle has a smaller icon");
    for (what, side) in [("past", 1282), ("large", 4000), ("reach", 100_000), ("most", MAX - 1)] {
        assert!(drawn(&format!("test.icon.{what}"), &icon(side)).px == at.px, "an icon in a square of {side} px is the largest icon, in its middle");
    }
    // The icons the kit's elements draw are no larger either. Each of these elements, larger than the screen,
    // is the kit's own drawing of it with no icon over 1,024 px, pixel for pixel, in both themes; with an icon
    // as large as the element makes it, the kit's drawing is another.
    for theme in [UiTheme::Gothic, UiTheme::Classic] {
        ui_set_theme(theme);
        let t = if theme == UiTheme::Gothic { "g" } else { "c" };
        for (k, element) in [
            Cmd::Button(-200, -200, 3000, 3600, "", "gear", 0),
            Cmd::Button(-200, -200, 3000, 3600, "A label", "gear", 0),
            Cmd::Fab(-800, -800, 3400, 4000, "gear"),
            Cmd::AppTile(-200, -200, 2400, 3000, "gear", "Gear", 0),
            Cmd::TextField(-100, -300, 2200, 3000, "gear", "", "hint"),
        ]
        .into_iter()
        .enumerate()
        {
            let cmds = [element];
            let c = drawn(&format!("test.icon.{t}.{k}"), &cmds);
            let mut held = patterned(SCREEN.0, SCREEN.1);
            font_with_icon_side_max(APP_ICON_PX_MAX, || kit(&mut held, WHOLE, &cmds));
            assert!(c.px == held.px, "{:?} in {theme:?}: its icon is 1,024 px at most", cmds[0]);
            let mut free = patterned(SCREEN.0, SCREEN.1);
            kit(&mut free, WHOLE, &cmds);
            assert!(c.px != free.px, "{:?} in {theme:?}: the element alone would make its icon larger", cmds[0]);
        }
    }
    ui_set_theme(UiTheme::Gothic);
    let _ = std::fs::remove_dir_all(&root);
}
