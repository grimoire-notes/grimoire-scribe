// appfix: the WebAssembly modules the app host's tests and scenarios run, written here byte by byte (no
// compiler, no file from anywhere else), and the installed app packages the scenarios copy
// (tests/fixtures/apphost/: `make app-fixtures` writes it; test_app_fixtures.rs checks the committed bytes are
// what this makes now).
//
// Every module here imports every function of the `grimoire` ABI an app has (docs/APPS.md) and exports its
// memory, `app_draw`, `app_pointer`, `app_key` and `app_stop`. A function is given as the bytes of its body
// (`Wasm`), which the helpers below write: constants pushed and an import called (`call`), a loop with no way
// out (`forever`), a counted loop (`repeat`). That is enough to pass the host any numbers, in a straight line
// or again and again.
#![allow(dead_code)]

use grimoire::sha256::sha256_of;

/// The ABI's imports in the order a module here imports them (a function's index): the name, the types of the
/// parameters (`i` an i32, `f` an f32) and of the result (`v` none, `i` an i32, `l` an i64).
pub const IMPORTS: [(&str, &str, char); 48] = [
    ("fill_rect", "iiiii", 'v'),
    ("outline_rect", "iiiiii", 'v'),
    ("round_rect", "iiiiiiii", 'v'),
    ("line", "fffffi", 'v'),
    ("text", "iiiiiiiiii", 'v'),
    ("text_at", "iiiiiii", 'v'),
    ("text_width", "iiii", 'i'),
    ("text_height", "ii", 'i'),
    ("icon", "iiiiiii", 'v'),
    ("button", "iiiiiiiii", 'v'),
    ("chip", "iiiiiii", 'v'),
    ("row", "iiiiiiiiii", 'v'),
    ("surface", "iiiiii", 'v'),
    ("select", "iiiiiiii", 'v'),
    ("app_tile", "iiiiiiiii", 'v'),
    ("fab", "iiiiii", 'v'),
    ("card", "iiiii", 'v'),
    ("text_field", "iiiiiiiiii", 'v'),
    ("clip", "iiii", 'v'),
    ("clip_clear", "", 'v'),
    ("state_get", "ii", 'i'),
    ("state_set", "ii", 'v'),
    ("setting", "iiii", 'i'),
    ("status", "ii", 'v'),
    ("request_redraw", "i", 'v'),
    ("close", "", 'v'),
    ("show_status_bar", "i", 'v'),
    ("toast", "iii", 'v'),
    ("now", "", 'l'),
    ("now_ms", "", 'l'),
    ("time_text", "ii", 'i'),
    ("date_text", "ii", 'i'),
    ("battery_percent", "", 'i'),
    ("battery_charging", "", 'i'),
    ("wifi_connected", "", 'i'),
    ("wifi_ssid", "ii", 'i'),
    ("screen_width", "", 'i'),
    ("screen_height", "", 'i'),
    ("ui_scale_milli", "", 'i'),
    ("theme", "", 'i'),
    ("status_bar_height", "", 'i'),
    ("http_start", "iiiiiiii", 'i'),
    ("http_poll", "", 'i'),
    ("http_status", "", 'i'),
    ("http_body_len", "", 'i'),
    ("http_body", "ii", 'i'),
    ("http_error", "ii", 'i'),
    ("http_cancel", "", 'v'),
];

/// The functions a module here has after its imports, in order: the exports the host calls (`app_draw`,
/// `app_pointer`, `app_key`, `app_stop`), then `app_status_bar` when the module has one, then the extra
/// functions (no parameter, no result) and the start function.
const EXPORTS: [(&str, &str, char); 4] = [("app_draw", "iiii", 'v'), ("app_pointer", "iiii", 'v'), ("app_key", "iii", 'v'), ("app_stop", "", 'v')];

/// The index of a module's first function beside its exports, when it has no `app_status_bar` (one more with it).
pub const FIRST_EXTRA: u32 = (IMPORTS.len() + EXPORTS.len()) as u32;

/// The local every function of a module counts its loops in (each function has eight i32 locals after its
/// parameters, which are four at most: this index and the two after it are locals in every function).
pub const COUNTER: u32 = 5;

fn uleb(out: &mut Vec<u8>, mut v: u32) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn sleb(out: &mut Vec<u8>, mut v: i32) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if (v == 0 && b & 0x40 == 0) || (v == -1 && b & 0x40 != 0) {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn name(out: &mut Vec<u8>, s: &str) {
    uleb(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

fn section(out: &mut Vec<u8>, id: u8, body: &[u8]) {
    out.push(id);
    uleb(out, body.len() as u32);
    out.extend_from_slice(body);
}

fn type_byte(c: char) -> u8 {
    match c {
        'i' => 0x7f,
        'l' => 0x7e,
        'f' => 0x7d,
        other => panic!("no type {other}"),
    }
}

/// i32.const
pub fn i(v: i32) -> Vec<u8> {
    let mut o = vec![0x41];
    sleb(&mut o, v);
    o
}

/// f32.const
pub fn f(v: f32) -> Vec<u8> {
    let mut o = vec![0x43];
    o.extend_from_slice(&v.to_le_bytes());
    o
}

/// local.get: a parameter of the function (`app_draw`'s width, height, scale and elapsed time), or a local.
pub fn arg(n: u32) -> Vec<u8> {
    let mut o = vec![0x20];
    uleb(&mut o, n);
    o
}

/// The index of an import, as a function.
pub fn import(import: &str) -> u32 {
    IMPORTS.iter().position(|(n, _, _)| *n == import).unwrap_or_else(|| panic!("no import {import}")) as u32
}

/// A call of the import with what the code before it left on the stack; a result is left there.
pub fn invoke(import_name: &str) -> Vec<u8> {
    let mut o = vec![0x10];
    uleb(&mut o, import(import_name));
    o
}

/// A call of the import with these numbers (all i32), its result dropped when it has one.
pub fn call(import_name: &str, args: &[i32]) -> Vec<u8> {
    let (_, params, result) = IMPORTS[import(import_name) as usize];
    assert_eq!(params.len(), args.len(), "{import_name} takes {} numbers", params.len());
    assert!(params.bytes().all(|b| b == b'i'), "{import_name} takes numbers that are not i32");
    let mut o: Vec<u8> = args.iter().flat_map(|&a| i(a)).collect();
    o.extend(invoke(import_name));
    if result != 'v' {
        o.push(0x1a);
    }
    o
}

/// A call of `line` with these numbers.
pub fn line(x0: f32, y0: f32, x1: f32, y1: f32, width: f32, gray: i32) -> Vec<u8> {
    let mut o: Vec<u8> = [x0, y0, x1, y1, width].into_iter().flat_map(f).collect();
    o.extend(i(gray));
    o.extend(invoke("line"));
    o
}

/// `body`, again and again with no way out: a loop that branches back to its start.
pub fn forever(body: &[u8]) -> Vec<u8> {
    let mut o = vec![0x03, 0x40];
    o.extend_from_slice(body);
    o.extend_from_slice(&[0x0c, 0x00, 0x0b]);
    o
}

/// `body`, `n` times, counted in the local COUNTER (which `body` may read: it runs from 0).
pub fn repeat(n: i32, body: &[u8]) -> Vec<u8> {
    repeat_in(COUNTER, n, body)
}

/// `body`, `n` times, counted in the local `local` (COUNTER + 1 or COUNTER + 2, for a loop inside a `repeat`).
pub fn repeat_in(local: u32, n: i32, body: &[u8]) -> Vec<u8> {
    let mut o = i(0);
    o.push(0x21);
    uleb(&mut o, local);
    o.extend_from_slice(&[0x03, 0x40]);
    o.extend_from_slice(body);
    o.extend(arg(local));
    o.extend(i(1));
    o.push(0x6a);
    o.push(0x22);
    uleb(&mut o, local);
    o.extend(i(n));
    // i32.lt_u, br_if 0, end
    o.extend_from_slice(&[0x49, 0x0d, 0x00, 0x0b]);
    o
}

/// local.get COUNTER, for a `repeat` body that draws by its turn.
pub fn counter() -> Vec<u8> {
    arg(COUNTER)
}

/// The pieces of code one after another.
pub fn seq(parts: &[Vec<u8>]) -> Vec<u8> {
    parts.concat()
}

/// `part`, `n` times over, as a straight line.
pub fn times(n: usize, part: &[u8]) -> Vec<u8> {
    part.repeat(n)
}

/// A module given as the bytes of its functions' bodies, with any number of memories and tables (pages or
/// entries at the start, and the most each may grow to). The memory exported is the first.
#[derive(Clone, Debug)]
pub struct Wasm {
    pub memories: Vec<(u32, Option<u32>)>,
    pub tables: Vec<(u32, Option<u32>)>,
    pub draw: Vec<u8>,
    pub pointer: Vec<u8>,
    pub key: Vec<u8>,
    pub stop: Vec<u8>,
    /// `app_status_bar`, when the module has one: the number it returns
    pub status_bar: Option<i32>,
    /// runs when the module is instantiated
    pub start: Option<Vec<u8>>,
    /// functions with no parameter and no result, from index FIRST_EXTRA
    pub extra: Vec<Vec<u8>>,
    pub data: Vec<(u32, Vec<u8>)>,
}

impl Wasm {
    /// A module of one page of memory whose `app_draw` is `draw` and whose other exports do nothing.
    pub fn new(draw: Vec<u8>) -> Wasm {
        Wasm { memories: vec![(1, None)], tables: Vec::new(), draw, pointer: Vec::new(), key: Vec::new(), stop: Vec::new(), status_bar: None, start: None, extra: Vec::new(), data: Vec::new() }
    }

    /// The same with these bytes in its memory from address 0.
    pub fn with(draw: Vec<u8>, data: &[u8]) -> Wasm {
        Wasm { data: vec![(0, data.to_vec())], ..Wasm::new(draw) }
    }

    pub fn bytes(&self) -> Vec<u8> {
        fn limits(s: &mut Vec<u8>, (min, max): (u32, Option<u32>)) {
            s.push(max.is_some() as u8);
            uleb(s, min);
            if let Some(max) = max {
                uleb(s, max);
            }
        }
        // The function types: each signature once, in the order it is first met.
        let mut types: Vec<(&str, char)> = Vec::new();
        let mut type_of = |params: &'static str, result: char| -> u32 {
            match types.iter().position(|t| *t == (params, result)) {
                Some(at) => at as u32,
                None => {
                    types.push((params, result));
                    types.len() as u32 - 1
                }
            }
        };
        let imports: Vec<u32> = IMPORTS.iter().map(|(_, p, r)| type_of(p, *r)).collect();
        let mut funcs: Vec<(u32, &[u8])> = Vec::new();
        for ((_, p, r), code) in EXPORTS.iter().zip([&self.draw, &self.pointer, &self.key, &self.stop]) {
            funcs.push((type_of(p, *r), code));
        }
        let bar = self.status_bar.map(i);
        if let Some(code) = &bar {
            funcs.push((type_of("", 'i'), code));
        }
        let plain = type_of("", 'v');
        funcs.extend(self.extra.iter().map(|e| (plain, &e[..])));
        funcs.extend(self.start.iter().map(|e| (plain, &e[..])));

        let mut out = b"\0asm\x01\0\0\0".to_vec();
        let mut s = Vec::new();
        uleb(&mut s, types.len() as u32);
        for (params, result) in &types {
            s.push(0x60);
            uleb(&mut s, params.len() as u32);
            s.extend(params.chars().map(type_byte));
            if *result == 'v' {
                s.push(0);
            } else {
                s.extend_from_slice(&[1, type_byte(*result)]);
            }
        }
        section(&mut out, 1, &s);
        s.clear();
        uleb(&mut s, IMPORTS.len() as u32);
        for ((import, _, _), ty) in IMPORTS.iter().zip(&imports) {
            name(&mut s, "grimoire");
            name(&mut s, import);
            s.push(0x00);
            uleb(&mut s, *ty);
        }
        section(&mut out, 2, &s);
        s.clear();
        uleb(&mut s, funcs.len() as u32);
        for (ty, _) in &funcs {
            uleb(&mut s, *ty);
        }
        section(&mut out, 3, &s);
        if !self.tables.is_empty() {
            s.clear();
            uleb(&mut s, self.tables.len() as u32);
            for t in &self.tables {
                s.push(0x70); // funcref
                limits(&mut s, *t);
            }
            section(&mut out, 4, &s);
        }
        s.clear();
        uleb(&mut s, self.memories.len() as u32);
        for m in &self.memories {
            limits(&mut s, *m);
        }
        section(&mut out, 5, &s);
        s.clear();
        uleb(&mut s, EXPORTS.len() as u32 + 1 + self.status_bar.is_some() as u32);
        name(&mut s, "memory");
        s.extend_from_slice(&[0x02, 0]);
        for (at, (export, _, _)) in EXPORTS.iter().enumerate() {
            name(&mut s, export);
            s.push(0x00);
            uleb(&mut s, (IMPORTS.len() + at) as u32);
        }
        if self.status_bar.is_some() {
            name(&mut s, "app_status_bar");
            s.push(0x00);
            uleb(&mut s, FIRST_EXTRA);
        }
        section(&mut out, 7, &s);
        if self.start.is_some() {
            s.clear();
            uleb(&mut s, (IMPORTS.len() + funcs.len()) as u32 - 1);
            section(&mut out, 8, &s);
        }
        s.clear();
        uleb(&mut s, funcs.len() as u32);
        for (_, code) in &funcs {
            let mut b = vec![1, 8, 0x7f]; // eight i32 locals
            b.extend_from_slice(code);
            b.push(0x0b);
            uleb(&mut s, b.len() as u32);
            s.extend_from_slice(&b);
        }
        section(&mut out, 10, &s);
        if !self.data.is_empty() {
            s.clear();
            uleb(&mut s, self.data.len() as u32);
            for (at, bytes) in &self.data {
                s.extend_from_slice(&[0x00, 0x41]);
                sleb(&mut s, *at as i32);
                s.push(0x0b);
                uleb(&mut s, bytes.len() as u32);
                s.extend_from_slice(bytes);
            }
            section(&mut out, 11, &s);
        }
        out
    }
}

/// An app package's manifest for `artifact`, its file `app.wasm`.
pub fn manifest(id: &str, name: &str, capabilities: &[&str], status_bar: bool, artifact: &[u8]) -> String {
    format!(
        "name: {name}\nid: {id}\ntype: app\nversion: 1.0.0\nauthor: Grimoire tests\ndescription: A module of the app host's tests.\n\
         platforms: [all]\nartifact:\n  path: app.wasm\n  sha256: {}\n  size: {}\ncapabilities: [{}]\n\
         spec:\n  runtime: wasm\n  status_bar: {status_bar}\n",
        sha256_of(artifact),
        artifact.len(),
        capabilities.join(", "),
    )
}

pub const STORAGE: &str = "storage:app-local";
pub const NETWORK: &str = "network:http";
pub const BOTH: [&str; 2] = [STORAGE, NETWORK];

// ---- a drawing pass as a list of commands ----

/// One drawing command of the ABI with its numbers and texts as a person writes them (docs/APPS.md): what
/// `pass` turns into a module's `app_draw`, and what a test draws with the kit itself to compare.
#[derive(Clone, Debug)]
pub enum Cmd {
    Fill(i32, i32, i32, i32, i32),
    Outline(i32, i32, i32, i32, i32, i32),
    /// x, y, w, h, radius, gray, filled, thickness
    Round(i32, i32, i32, i32, i32, i32, i32, i32),
    Line(f32, f32, f32, f32, f32, i32),
    /// x, y, w, h, the text, px_milli, face, center, gray
    Text(i32, i32, i32, i32, &'static str, i32, i32, i32, i32),
    /// x, baseline, the text, px_milli, face, gray
    TextAt(i32, i32, &'static str, i32, i32, i32),
    Icon(&'static str, i32, i32, i32, i32, i32),
    /// x, y, w, h, label, icon, style
    Button(i32, i32, i32, i32, &'static str, &'static str, i32),
    Chip(i32, i32, i32, i32, &'static str, i32),
    /// x, y, w, h, icon, label, style, switch
    Row(i32, i32, i32, i32, &'static str, &'static str, i32, i32),
    Surface(i32, i32, i32, i32, i32, i32),
    /// x, y, w, h, caption, value
    Select(i32, i32, i32, i32, &'static str, &'static str),
    /// x, y, w, h, icon, label, selected
    AppTile(i32, i32, i32, i32, &'static str, &'static str, i32),
    Fab(i32, i32, i32, i32, &'static str),
    Card(i32, i32, i32, i32, i32),
    /// x, y, w, h, icon, text, hint
    TextField(i32, i32, i32, i32, &'static str, &'static str, &'static str),
    Clip(i32, i32, i32, i32),
    ClipClear,
}

/// Where `pass` puts the texts of its commands, one after another.
pub const TEXTS_AT: i32 = 1024;

/// A module whose `app_draw` makes these commands in order, their texts in its memory.
pub fn pass(cmds: &[Cmd]) -> Wasm {
    let mut data: Vec<u8> = Vec::new();
    // (a text's place and length; one already there is used again)
    let mut put = |text: &str| -> [i32; 2] {
        let at = data.windows(text.len().max(1)).position(|w| w == text.as_bytes()).filter(|_| !text.is_empty()).unwrap_or_else(|| {
            data.extend_from_slice(text.as_bytes());
            data.len() - text.len()
        });
        [TEXTS_AT + at as i32, text.len() as i32]
    };
    let mut code = Vec::new();
    for cmd in cmds {
        code.extend(match *cmd {
            Cmd::Fill(x, y, w, h, gray) => call("fill_rect", &[x, y, w, h, gray]),
            Cmd::Outline(x, y, w, h, t, gray) => call("outline_rect", &[x, y, w, h, t, gray]),
            Cmd::Round(x, y, w, h, radius, gray, filled, t) => call("round_rect", &[x, y, w, h, radius, gray, filled, t]),
            Cmd::Line(x0, y0, x1, y1, width, gray) => line(x0, y0, x1, y1, width, gray),
            Cmd::Text(x, y, w, h, text, px_milli, face, center, gray) => {
                let [p, n] = put(text);
                call("text", &[x, y, w, h, p, n, px_milli, face, center, gray])
            }
            Cmd::TextAt(x, baseline, text, px_milli, face, gray) => {
                let [p, n] = put(text);
                call("text_at", &[x, baseline, p, n, px_milli, face, gray])
            }
            Cmd::Icon(icon, x, y, w, h, gray) => {
                let [p, n] = put(icon);
                call("icon", &[p, n, x, y, w, h, gray])
            }
            Cmd::Button(x, y, w, h, label, icon, style) => {
                let ([lp, ln], [ip, il]) = (put(label), put(icon));
                call("button", &[x, y, w, h, lp, ln, ip, il, style])
            }
            Cmd::Chip(x, y, w, h, label, selected) => {
                let [p, n] = put(label);
                call("chip", &[x, y, w, h, p, n, selected])
            }
            Cmd::Row(x, y, w, h, icon, label, style, switch) => {
                let ([ip, il], [lp, ln]) = (put(icon), put(label));
                call("row", &[x, y, w, h, ip, il, lp, ln, style, switch])
            }
            Cmd::Surface(x, y, w, h, filled, thick) => call("surface", &[x, y, w, h, filled, thick]),
            Cmd::Select(x, y, w, h, caption, value) => {
                let ([cp, cl], [vp, vl]) = (put(caption), put(value));
                call("select", &[x, y, w, h, cp, cl, vp, vl])
            }
            Cmd::AppTile(x, y, w, h, icon, label, selected) => {
                let ([ip, il], [lp, ln]) = (put(icon), put(label));
                call("app_tile", &[x, y, w, h, ip, il, lp, ln, selected])
            }
            Cmd::Fab(x, y, w, h, icon) => {
                let [p, n] = put(icon);
                call("fab", &[x, y, w, h, p, n])
            }
            Cmd::Card(x, y, w, h, style) => call("card", &[x, y, w, h, style]),
            Cmd::TextField(x, y, w, h, icon, text, hint) => {
                let ([ip, il], [tp, tl], [hp, hl]) = (put(icon), put(text), put(hint));
                call("text_field", &[x, y, w, h, ip, il, tp, tl, hp, hl])
            }
            Cmd::Clip(x, y, w, h) => call("clip", &[x, y, w, h]),
            Cmd::ClipClear => call("clip_clear", &[]),
        });
    }
    Wasm { data: vec![(TEXTS_AT as u32, data)], ..Wasm::new(code) }
}

/// One of every drawing command, with the numbers and texts an app's screen has: a page of a settings
/// screen and a small board, on a body about as large as the screen.
pub fn honest() -> Vec<Cmd> {
    vec![
        Cmd::Fill(40, 40, 300, 120, 0),
        Cmd::Fill(360, 40, 20, 120, 128),
        Cmd::Outline(400, 40, 300, 120, 4, 0),
        Cmd::Outline(720, 40, 30, 120, 40, 90),
        Cmd::Round(760, 40, 300, 120, 24, 0, 1, 0),
        Cmd::Round(1100, 40, 300, 120, 24, 0, 0, 5),
        Cmd::Round(1440, 40, 120, 120, 60, 60, 1, 0),
        Cmd::Line(40.5, 200.25, 800.0, 260.0, 3.5, 0),
        Cmd::Line(900.0, 200.0, 900.0, 280.0, 1.0, 0),
        Cmd::Line(1000.0, 240.0, 1700.0, 240.0, 12.0, 100),
        Cmd::Text(40, 300, 900, 60, "A line of text in the regular face", 34_000, 0, 0, 0),
        Cmd::Text(40, 370, 1780, 90, "Centered and bold", 60_000, 1, 1, 0),
        Cmd::Text(1000, 300, 300, 60, "A line too long for the room it is given", 34_000, 0, 0, 80),
        Cmd::Text(1400, 300, 400, 60, "Title", 48_000, 2, 0, 0),
        Cmd::Text(40, 470, 600, 50, "caps and fixed 0123", 30_000, 3, 0, 0),
        Cmd::TextAt(700, 510, "fixed 0123456789", 36_000, 4, 0),
        Cmd::TextAt(1200, 510, "fixed bold", 36_000, 5, 0),
        Cmd::Icon("gear", 40, 540, 96, 96, 0),
        Cmd::Icon("md-clock", 160, 540, 96, 96, 0),
        Cmd::Icon("no such icon", 280, 540, 96, 96, 120),
        Cmd::Button(40, 660, 400, 120, "Play", "", 0),
        Cmd::Button(460, 660, 400, 120, "Chosen", "", 1),
        Cmd::Button(880, 660, 400, 120, "Not now", "", 2),
        Cmd::Button(1300, 660, 400, 120, "Go", "", 3),
        Cmd::Button(40, 800, 200, 200, "Files", "files", 0),
        Cmd::Button(260, 800, 200, 200, "", "search", 0),
        Cmd::Chip(500, 820, 260, 80, "All", 0),
        Cmd::Chip(780, 820, 260, 80, "Recent", 1),
        Cmd::Row(40, 1020, 1700, 120, "gear", "Sound", 0, 1),
        Cmd::Row(40, 1150, 1700, 120, "", "A row with no icon and no switch", 0, -1),
        Cmd::Row(40, 1280, 1700, 120, "language", "Chosen row", 1, 0),
        Cmd::Surface(40, 1420, 500, 160, 0, 3),
        Cmd::Surface(560, 1420, 500, 160, 1, 0),
        Cmd::Select(1100, 1440, 660, 100, "Sort", "Name"),
        Cmd::AppTile(40, 1600, 220, 260, "clock", "Clock", 0),
        Cmd::AppTile(280, 1600, 220, 260, "music", "Music", 1),
        Cmd::Card(540, 1600, 500, 200, 0),
        Cmd::Card(1060, 1600, 300, 200, 1),
        Cmd::Card(1380, 1600, 300, 200, 2),
        Cmd::TextField(40, 1900, 900, 100, "search", "query", "Search"),
        Cmd::TextField(960, 1900, 800, 100, "search", "", "Search"),
        Cmd::Clip(40, 2040, 300, 100),
        Cmd::Fill(0, 2000, 1860, 300, 128),
        Cmd::Text(60, 2050, 800, 80, "cut by the clip", 50_000, 1, 0, 0),
        Cmd::ClipClear,
        Cmd::Fill(1000, 2060, 50, 50, 0),
        Cmd::Fab(1600, 2060, 160, 160, "download"),
    ]
}

// ---- modules that keep something ----

/// Where `keeps` reads its saved state into, and has the words it saves.
pub const STATE_AT: i32 = 60000;
pub const WORD_AT: i32 = 32;

/// A module that keeps a word: each `app_draw` says the saved state in the status line, saves "hello" and fills
/// a black rectangle; a touch saves "touch", a key "key", and `app_stop` "stop".
pub fn keeps() -> Wasm {
    let said = seq(&[i(STATE_AT), i(STATE_AT), i(64), invoke("state_get"), invoke("status")]);
    let mut w = Wasm::new(seq(&[said, call("state_set", &[WORD_AT, 5]), call("fill_rect", &[40, 40, 200, 100, 0])]));
    w.pointer = call("state_set", &[WORD_AT + 8, 5]);
    w.key = call("state_set", &[WORD_AT + 16, 3]);
    w.stop = call("state_set", &[WORD_AT + 24, 4]);
    w.data = vec![(WORD_AT as u32, b"hello\0\0\0touch\0\0\0key\0\0\0\0\0stop".to_vec())];
    w
}

// ---- the modules of the scenarios ----

/// Where `counted` keeps its count (an i32), and the six digits it says it in.
pub const COUNT_AT: i32 = 128;
pub const DIGITS_AT: i32 = 136;

/// The code that counts a call: one more in the count at COUNT_AT, written as six decimal digits at DIGITS_AT.
pub fn counted() -> Vec<u8> {
    // i32.load and i32.store of a whole word, i32.store8 of a byte, at the address on the stack
    const LOAD: [u8; 3] = [0x28, 0x02, 0x00];
    const STORE: [u8; 3] = [0x36, 0x02, 0x00];
    const STORE8: [u8; 3] = [0x3a, 0x00, 0x00];
    let mut o = seq(&[i(COUNT_AT), i(COUNT_AT), LOAD.to_vec(), i(1), vec![0x6a], STORE.to_vec()]);
    for (place, tens) in [100_000, 10_000, 1000, 100, 10, 1].into_iter().enumerate() {
        // (the count over this power of ten, its last digit, as a character)
        o.extend(seq(&[i(DIGITS_AT + place as i32), i(COUNT_AT), LOAD.to_vec(), i(tens), vec![0x6e], i(10), vec![0x70], i(48), vec![0x6a], STORE8.to_vec()]));
    }
    o
}

/// A module that counts its drawing passes: each `app_draw` says the count in the status line (six digits,
/// "000001" after the first) and fills a black rectangle of 200x100 at 40,40.
pub fn counts() -> Vec<u8> {
    Wasm::new(seq(&[counted(), call("status", &[DIGITS_AT, 6]), call("fill_rect", &[40, 40, 200, 100, 0])])).bytes()
}

/// A module that counts its drawing passes as `counts` does and asks, in each, to be drawn again `after_ms`
/// later.
pub fn asks(after_ms: i32) -> Wasm {
    Wasm::new(seq(&[counted(), call("status", &[DIGITS_AT, 6]), call("fill_rect", &[40, 40, 200, 100, 0]), call("request_redraw", &[after_ms])]))
}

/// A module that counts its drawing passes and asks to be drawn again only where it is touched: `after_ms` after
/// each phase of a touch, and never in a drawing pass.
pub fn asks_when_touched(after_ms: i32) -> Wasm {
    let mut w = Wasm::new(seq(&[counted(), call("status", &[DIGITS_AT, 6]), call("fill_rect", &[40, 40, 200, 100, 0])]));
    w.pointer = call("request_redraw", &[after_ms]);
    w
}

/// A module whose `app_draw` never ends: a loop with no way out.
pub fn loops() -> Vec<u8> {
    Wasm::new(forever(&[])).bytes()
}

/// A module that draws far outside its body: black from far above and left of it to far below and right of it,
/// then the same through a `clip` as wide, and again with the clip taken away.
pub fn wide() -> Vec<u8> {
    let black = call("fill_rect", &[-5000, -5000, 20_000, 20_000, 0]);
    Wasm::with(seq(&[black.clone(), call("clip", &[-5000, -5000, 20_000, 20_000]), black.clone(), call("clip_clear", &[]), black, call("status", &[0, 4])]), b"wide").bytes()
}

/// A package of the scenarios' data root: its id, name, manifest and module.
pub struct Package {
    pub id: &'static str,
    pub name: &'static str,
    pub manifest: String,
    pub module: Vec<u8>,
}

fn package(id: &'static str, name: &'static str, capabilities: &[&str], status_bar: bool, module: Vec<u8>) -> Package {
    Package { id, name, manifest: manifest(id, name, capabilities, status_bar, &module), module }
}

/// The packages of tests/fixtures/apphost, in the order of its index.
pub fn packages() -> Vec<Package> {
    vec![
        // (its drawing pass never ends)
        package("test.app.loop", "Loop", &[], false, loops()),
        // (it counts its drawing passes)
        package("test.app.counts", "Counts", &[], false, counts()),
        // (it shows the status bar, and draws black from far outside its body)
        package("test.app.wide", "Wide", &[], true, wide()),
        // (each counts its drawing passes: Ticker asks for the next a second later, Asks as soon as an app may, and
        // Once only when it is touched, half a second after)
        package("test.app.ticker", "Ticker", &[], false, asks(1000).bytes()),
        package("test.app.asks", "Asks", &[], false, asks(0).bytes()),
        package("test.app.once", "Once", &[], false, asks_when_touched(500).bytes()),
    ]
}

/// An installed data root's `packages` folder with these app packages in it, each file's path and bytes.
pub fn scenario_files(packages: &[Package]) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut index = String::from("{\n  \"version\": 1,\n  \"packages\": [");
    for (i, p) in packages.iter().enumerate() {
        let dir = format!("packages/apps/{}", p.id);
        index.push_str(if i > 0 { ",\n    " } else { "\n    " });
        index.push_str(&format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"version\":\"1.0.0\",\"author\":\"Grimoire tests\",\"description\":\"A module of the app host's tests.\",\"icon\":\"\",\"path\":\"{dir}/app.wasm\",\"sha\":\"{}\",\"installed_at\":1,\"source_url\":\"\"}}",
            p.id,
            p.name,
            sha256_of(&p.module),
        ));
        files.push((format!("{dir}/package.yaml"), p.manifest.clone().into_bytes()));
        files.push((format!("{dir}/app.wasm"), p.module.clone()));
    }
    index.push_str("\n  ]\n}\n");
    files.push(("packages/apps-index.json".to_string(), index.into_bytes()));
    files
}
