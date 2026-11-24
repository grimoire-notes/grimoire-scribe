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
    for (b, body) in [body_under_the_bar(), WHOLE, inner].into_iter().enumerate() {
        for (i, (name, cmds)) in passes_that_reach_out(body).into_iter().enumerate() {
            let mut rt = installed(&format!("test.body.{b}.{i}"), &BOTH, &pass(&cmds));
            for theme in [UiTheme::Gothic, UiTheme::Classic] {
                ui_set_theme(theme);
                let before = patterned(SCREEN.0, SCREEN.1);
                let mut c = before.clone();
                rt.draw(&mut c, body, 0);
                assert_eq!(rt.error(), None, "{name}");
                assert_eq!(changed_outside(&before, &c, body), 0, "{name} in {theme:?}, a body at {},{}: pixels outside the body changed", body.x, body.y);
                assert_eq!(c.clip, before.clip, "{name}: the canvas keeps its clip");
                assert_ne!(inked(&c, body), 0, "{name} in {theme:?}: the pass drew in the body");
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
