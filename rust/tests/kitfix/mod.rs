// kitfix: the WebAssembly modules the package kit's tests and scenarios run, written here byte by byte (no compiler,
// no file from anywhere else), and the installed packages the scenarios copy (tests/fixtures/kit/: `make
// kit-fixtures` writes it; test_kit_fixtures.rs checks the committed bytes are what this makes now).
//
// Every module here imports every call of `grimoire_kit` level 1 (docs/KIT.md), the three calls of `grimoire`
// both hosts have with the same signature (`status`, `state_get`, `state_set`), and any other import a test
// names; it exports its memory and the exports of its host: `widget_draw` and `widget_tap` for a widget,
// `app_draw`, `app_pointer`, `app_key` and `app_stop` for an app. A function is given as the bytes of its body,
// which the helpers below write: constants pushed and an import called, a counted loop, a cursor that a call's
// returned extent moves on.
#![allow(dead_code)]

use grimoire::sha256::sha256_of;

/// The calls of `grimoire_kit` in the order a module here imports them (a function's index): the name, the types
/// of the parameters (`i` an i32, `f` an f32) and of the result (`v` none, `i` an i32, `l` an i64).
pub const KIT: [(&str, &str, char); 26] = [
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
];

/// The calls of `grimoire` a module here imports after the kit's: the ones both hosts have alike.
pub const HOST: [(&str, &str, char); 3] = [("status", "ii", 'v'), ("state_get", "ii", 'i'), ("state_set", "ii", 'v')];

/// Which host a module is for: its exports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Widget,
    App,
}

impl Host {
    fn exports(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Host::Widget => &[("widget_draw", "iii"), ("widget_tap", "i")],
            Host::App => &[("app_draw", "iiii"), ("app_pointer", "iiii"), ("app_key", "iii"), ("app_stop", "")],
        }
    }
}

/// The locals every function of a module has after its parameters (four at most): eight i32. These three are
/// locals in every function: the cursor a layout moves down (`Y`), a loop's count (`COUNTER`) and a spare (`T`).
pub const Y: u32 = 5;
pub const COUNTER: u32 = 6;
pub const T: u32 = 7;
/// And a cursor across (`X`).
pub const X: u32 = 8;

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

fn sleb(out: &mut Vec<u8>, mut v: i64) {
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
    sleb(&mut o, v as i64);
    o
}

/// f32.const
pub fn f(v: f32) -> Vec<u8> {
    let mut o = vec![0x43];
    o.extend_from_slice(&v.to_le_bytes());
    o
}

/// local.get: a parameter of the function, or a local.
pub fn arg(n: u32) -> Vec<u8> {
    let mut o = vec![0x20];
    uleb(&mut o, n);
    o
}

/// local.set
pub fn set(n: u32) -> Vec<u8> {
    let mut o = vec![0x21];
    uleb(&mut o, n);
    o
}

/// i32.add of the two numbers on the stack.
pub const ADD: u8 = 0x6a;
/// drop
pub const DROP: u8 = 0x1a;
/// i32.wrap_i64
pub const WRAP64: u8 = 0xa7;

/// The pieces of code one after another.
pub fn seq(parts: &[Vec<u8>]) -> Vec<u8> {
    parts.concat()
}

/// `part`, `n` times over, as a straight line.
pub fn times(n: usize, part: &[u8]) -> Vec<u8> {
    part.repeat(n)
}

/// `body`, `n` times, counted in the local COUNTER (which `body` may read: it runs from 0).
pub fn repeat(n: i32, body: &[u8]) -> Vec<u8> {
    let mut o = seq(&[i(0), set(COUNTER)]);
    o.extend_from_slice(&[0x03, 0x40]);
    o.extend_from_slice(body);
    o.extend(arg(COUNTER));
    o.extend(i(1));
    o.push(ADD);
    o.push(0x22);
    uleb(&mut o, COUNTER);
    o.extend(i(n));
    // i32.lt_u, br_if 0, end
    o.extend_from_slice(&[0x49, 0x0d, 0x00, 0x0b]);
    o
}

/// `body`, again and again with no way out.
pub fn forever(body: &[u8]) -> Vec<u8> {
    let mut o = vec![0x03, 0x40];
    o.extend_from_slice(body);
    o.extend_from_slice(&[0x0c, 0x00, 0x0b]);
    o
}

/// The index of a kit call, as a function.
fn kit_index(call: &str) -> u32 {
    KIT.iter().position(|(n, _, _)| *n == call).unwrap_or_else(|| panic!("no kit call {call}")) as u32
}

/// A call of the kit with what the code before it left on the stack; its result, if any, is left there.
pub fn invoke(call: &str) -> Vec<u8> {
    let mut o = vec![0x10];
    uleb(&mut o, kit_index(call));
    o
}

/// A call of the kit with these numbers (all i32), its result dropped when it has one.
pub fn call(name: &str, args: &[i32]) -> Vec<u8> {
    let (_, params, result) = KIT[kit_index(name) as usize];
    assert_eq!(params.len(), args.len(), "grimoire_kit.{name} takes {} numbers", params.len());
    assert!(params.bytes().all(|b| b == b'i'), "grimoire_kit.{name} takes numbers that are not i32");
    let mut o: Vec<u8> = args.iter().flat_map(|&a| i(a)).collect();
    o.extend(invoke(name));
    if result != 'v' {
        o.push(DROP);
    }
    o
}

/// A call of the kit's `line`.
pub fn line(x0: f32, y0: f32, x1: f32, y1: f32, width: f32, gray: i32) -> Vec<u8> {
    let mut o: Vec<u8> = [x0, y0, x1, y1, width].into_iter().flat_map(f).collect();
    o.extend(i(gray));
    o.extend(invoke("line"));
    o
}

/// A call of one of `grimoire`'s (HOST) with these numbers, its result dropped.
pub fn host(name: &str, args: &[i32]) -> Vec<u8> {
    let at = HOST.iter().position(|(n, _, _)| *n == name).unwrap_or_else(|| panic!("no host call {name}"));
    let mut o: Vec<u8> = args.iter().flat_map(|&a| i(a)).collect();
    o.push(0x10);
    uleb(&mut o, (KIT.len() + at) as u32);
    if HOST[at].2 != 'v' {
        o.push(DROP);
    }
    o
}

/// A call of the module's extra import number `n` (after the kit's and HOST) with these numbers, its result
/// dropped when it has one.
pub fn extra(module: &Kit, n: usize, args: &[i32]) -> Vec<u8> {
    let (_, _, params, result) = &module.imports[n];
    assert_eq!(params.len(), args.len());
    let mut o: Vec<u8> = args.iter().flat_map(|&a| i(a)).collect();
    o.push(0x10);
    uleb(&mut o, (KIT.len() + HOST.len() + n) as u32);
    if *result != 'v' {
        o.push(DROP);
    }
    o
}

/// A module for a host, given as the bytes of its functions' bodies (its exports, in the host's order), with one
/// memory of `pages` pages and `data` in it.
#[derive(Clone, Debug)]
pub struct Kit {
    pub host: Host,
    pub pages: u32,
    /// the exports' bodies in the host's order (`widget_draw`, `widget_tap`; or `app_draw`, `app_pointer`,
    /// `app_key`, `app_stop`); a missing one does nothing
    pub code: Vec<Vec<u8>>,
    /// imports after the kit's and HOST: module, name, parameters, result
    pub imports: Vec<(&'static str, &'static str, &'static str, char)>,
    pub data: Vec<(u32, Vec<u8>)>,
}

impl Kit {
    /// A module whose drawing pass (`widget_draw` or `app_draw`) is `draw`.
    pub fn new(host: Host, draw: Vec<u8>) -> Kit {
        Kit { host, pages: 1, code: vec![draw], imports: Vec::new(), data: Vec::new() }
    }

    /// The export after the drawing pass: `widget_tap`, or `app_pointer`.
    pub fn touch(mut self, code: Vec<u8>) -> Kit {
        while self.code.len() < 2 {
            self.code.push(Vec::new());
        }
        self.code[1] = code;
        self
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut types: Vec<(String, char)> = Vec::new();
        let mut type_of = |params: &str, result: char| -> u32 {
            match types.iter().position(|t| t.0 == params && t.1 == result) {
                Some(at) => at as u32,
                None => {
                    types.push((params.to_string(), result));
                    types.len() as u32 - 1
                }
            }
        };
        let mut imports: Vec<(&str, &str, u32)> = Vec::new();
        for (n, p, r) in KIT {
            imports.push(("grimoire_kit", n, type_of(p, r)));
        }
        for (n, p, r) in HOST {
            imports.push(("grimoire", n, type_of(p, r)));
        }
        for (m, n, p, r) in &self.imports {
            imports.push((m, n, type_of(p, *r)));
        }
        let exports = self.host.exports();
        let funcs: Vec<u32> = exports.iter().map(|(_, p)| type_of(p, 'v')).collect();

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
        uleb(&mut s, imports.len() as u32);
        for (m, n, ty) in &imports {
            name(&mut s, m);
            name(&mut s, n);
            s.push(0x00);
            uleb(&mut s, *ty);
        }
        section(&mut out, 2, &s);
        s.clear();
        uleb(&mut s, funcs.len() as u32);
        for ty in &funcs {
            uleb(&mut s, *ty);
        }
        section(&mut out, 3, &s);
        s.clear();
        s.extend_from_slice(&[1, 0]);
        uleb(&mut s, self.pages);
        section(&mut out, 5, &s);
        s.clear();
        uleb(&mut s, exports.len() as u32 + 1);
        name(&mut s, "memory");
        s.extend_from_slice(&[0x02, 0]);
        for (at, (export, _)) in exports.iter().enumerate() {
            name(&mut s, export);
            s.push(0x00);
            uleb(&mut s, (imports.len() + at) as u32);
        }
        section(&mut out, 7, &s);
        s.clear();
        uleb(&mut s, funcs.len() as u32);
        for at in 0..funcs.len() {
            let mut b = vec![1, 8, 0x7f]; // eight i32 locals
            b.extend_from_slice(self.code.get(at).map_or(&[][..], |c| &c[..]));
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
                sleb(&mut s, *at as i64);
                s.push(0x0b);
                uleb(&mut s, bytes.len() as u32);
                s.extend_from_slice(bytes);
            }
            section(&mut out, 11, &s);
        }
        out
    }
}

// ---- a drawing pass as a list of the kit's commands ----

/// One call of the kit with its numbers and texts as a person writes them (docs/KIT.md): what `pass` turns into
/// a module's drawing pass, and what a test draws with Grimoire's own drawers to compare.
#[derive(Clone, Debug)]
pub enum K {
    /// x, y, w, h, the text, role, flags
    Text(i32, i32, i32, i32, &'static str, i32, i32),
    /// x, y, w, h, the text, role, flags, first line
    Lines(i32, i32, i32, i32, &'static str, i32, i32, i32),
    /// id, x, y, w, h, label, icon, style
    Button(i32, i32, i32, i32, i32, &'static str, &'static str, i32),
    /// id, x, y, w, h, icon, style
    IconButton(i32, i32, i32, i32, i32, &'static str, i32),
    /// id, x, y, w, h, icon, label, style, switch
    Row(i32, i32, i32, i32, i32, &'static str, &'static str, i32, i32),
    /// id, x, y, w, h, label, selected
    Chip(i32, i32, i32, i32, i32, &'static str, i32),
    /// id, x, y, w, h, icon, text, hint, flags
    Field(i32, i32, i32, i32, i32, &'static str, &'static str, &'static str, i32),
    /// id, x, y, w, h, caption, value
    Select(i32, i32, i32, i32, i32, &'static str, &'static str),
    /// id, x, y, w, h, label, checked
    Checkbox(i32, i32, i32, i32, i32, &'static str, i32),
    /// x, y, w, h, style
    Card(i32, i32, i32, i32, i32),
    /// x, y, w
    Divider(i32, i32, i32),
    /// x, y, w, h, name, tone
    Icon(i32, i32, i32, i32, &'static str, i32),
    /// x, y, w, h, gray
    Fill(i32, i32, i32, i32, i32),
    /// x, y, w, h, gray, filled, thickness
    Frame(i32, i32, i32, i32, i32, i32, i32),
    Line(f32, f32, f32, f32, f32, i32),
    Clip(i32, i32, i32, i32),
    ClipClear,
    /// id, x, y, w, h
    Hit(i32, i32, i32, i32, i32),
    /// prev id, next id, x, y, w, page, pages
    Pager(i32, i32, i32, i32, i32, i32, i32),
}

/// Where `pass` puts the texts of its commands, one after another.
pub const TEXTS_AT: i32 = 1024;

/// The texts of a pass, laid one after another from TEXTS_AT (one already there is used again).
#[derive(Default)]
pub struct Texts(pub Vec<u8>);

impl Texts {
    pub fn put(&mut self, text: &str) -> [i32; 2] {
        let data = &mut self.0;
        let at = data.windows(text.len().max(1)).position(|w| w == text.as_bytes()).filter(|_| !text.is_empty()).unwrap_or_else(|| {
            data.extend_from_slice(text.as_bytes());
            data.len() - text.len()
        });
        [TEXTS_AT + at as i32, text.len() as i32]
    }
}

/// The code of one command, its texts put in `t`.
pub fn command(cmd: &K, t: &mut Texts) -> Vec<u8> {
    match *cmd {
        K::Text(x, y, w, h, s, role, flags) => {
            let [p, n] = t.put(s);
            call("text", &[x, y, w, h, p, n, role, flags])
        }
        K::Lines(x, y, w, h, s, role, flags, first) => {
            let [p, n] = t.put(s);
            call("text_lines", &[x, y, w, h, p, n, role, flags, first])
        }
        K::Button(id, x, y, w, h, label, icon, style) => {
            let ([lp, ll], [ip, il]) = (t.put(label), t.put(icon));
            call("button", &[id, x, y, w, h, lp, ll, ip, il, style])
        }
        K::IconButton(id, x, y, w, h, icon, style) => {
            let [ip, il] = t.put(icon);
            call("icon_button", &[id, x, y, w, h, ip, il, style])
        }
        K::Row(id, x, y, w, h, icon, label, style, switch) => {
            let ([ip, il], [lp, ll]) = (t.put(icon), t.put(label));
            call("row", &[id, x, y, w, h, ip, il, lp, ll, style, switch])
        }
        K::Chip(id, x, y, w, h, label, selected) => {
            let [lp, ll] = t.put(label);
            call("chip", &[id, x, y, w, h, lp, ll, selected])
        }
        K::Field(id, x, y, w, h, icon, text, hint, flags) => {
            let ([ip, il], [tp, tl], [hp, hl]) = (t.put(icon), t.put(text), t.put(hint));
            call("field", &[id, x, y, w, h, ip, il, tp, tl, hp, hl, flags])
        }
        K::Select(id, x, y, w, h, caption, value) => {
            let ([cp, cl], [vp, vl]) = (t.put(caption), t.put(value));
            call("select", &[id, x, y, w, h, cp, cl, vp, vl])
        }
        K::Checkbox(id, x, y, w, h, label, checked) => {
            let [lp, ll] = t.put(label);
            call("checkbox", &[id, x, y, w, h, lp, ll, checked])
        }
        K::Card(x, y, w, h, style) => call("card", &[x, y, w, h, style]),
        K::Divider(x, y, w) => call("divider", &[x, y, w]),
        K::Icon(x, y, w, h, icon, tone) => {
            let [p, n] = t.put(icon);
            call("icon", &[x, y, w, h, p, n, tone])
        }
        K::Fill(x, y, w, h, gray) => call("fill", &[x, y, w, h, gray]),
        K::Frame(x, y, w, h, gray, filled, thick) => call("frame", &[x, y, w, h, gray, filled, thick]),
        K::Line(x0, y0, x1, y1, width, gray) => line(x0, y0, x1, y1, width, gray),
        K::Clip(x, y, w, h) => call("clip", &[x, y, w, h]),
        K::ClipClear => call("clip_clear", &[]),
        K::Hit(id, x, y, w, h) => call("hit", &[id, x, y, w, h]),
        K::Pager(prev, next, x, y, w, page, pages) => call("pager", &[prev, next, x, y, w, page, pages]),
    }
}

/// A module whose drawing pass makes these commands in order, their texts in its memory.
pub fn pass(host: Host, cmds: &[K]) -> Kit {
    let mut t = Texts::default();
    let code: Vec<u8> = cmds.iter().flat_map(|c| command(c, &mut t)).collect();
    Kit { data: vec![(TEXTS_AT as u32, t.0)], ..Kit::new(host, code) }
}

/// The code that writes `value` (the number the code before it left on the stack) as `digits` decimal digits at
/// `at`, with leading zeros. It uses the local T.
pub fn digits_of(at: i32, digits: usize) -> Vec<u8> {
    // i32.store8 of a byte, at the address on the stack
    const STORE8: [u8; 3] = [0x3a, 0x00, 0x00];
    let mut o = set(T);
    let mut tens = 1;
    for place in (0..digits).rev() {
        // (the number over this power of ten, its last digit, as a character)
        o.extend(seq(&[i(at + place as i32), arg(T), i(tens), vec![0x6e], i(10), vec![0x70], i(48), vec![ADD], STORE8.to_vec()]));
        tens = tens.saturating_mul(10);
    }
    o
}

// ---- the packages of the scenarios ----

/// A widget's manifest for `artifact`, its file `module.bin`, in a frame of `w` x `h`.
pub fn widget_manifest(id: &str, name: &str, capabilities: &[&str], w: i32, h: i32, artifact: &[u8]) -> String {
    format!(
        "name: {name}\nid: {id}\ntype: widget\nversion: 1.0.0\nauthor: Grimoire tests\ndescription: A module of the package kit's tests.\n\
         platforms: [all]\nartifact:\n  path: module.bin\n  sha256: {}\n  size: {}\ncapabilities: [{}]\n\
         spec:\n  runtime: wasm\n  size:\n    mode: fixed\n    default: [{w}, {h}]\n",
        sha256_of(artifact),
        artifact.len(),
        capabilities.join(", "),
    )
}

/// An app's manifest for `artifact`, its file `app.wasm`.
pub fn app_manifest(id: &str, name: &str, capabilities: &[&str], status_bar: bool, artifact: &[u8]) -> String {
    format!(
        "name: {name}\nid: {id}\ntype: app\nversion: 1.0.0\nauthor: Grimoire tests\ndescription: A module of the package kit's tests.\n\
         platforms: [all]\nartifact:\n  path: app.wasm\n  sha256: {}\n  size: {}\ncapabilities: [{}]\n\
         spec:\n  runtime: wasm\n  status_bar: {status_bar}\n",
        sha256_of(artifact),
        artifact.len(),
        capabilities.join(", "),
    )
}

pub const OVERLAY: &str = "editor:overlay";
pub const WIDGET_BOTH: [&str; 2] = [OVERLAY, "storage:widget-local"];
pub const APP_STORAGE: &str = "storage:app-local";

/// Where the modules here keep the word their status line says (the saved state is read there), and the byte a
/// tap writes.
pub const STATE_AT: i32 = 60000;
pub const TAP_AT: i32 = 61000;

/// The code that says the saved state in the status line: it is read to STATE_AT, at most 64 bytes.
pub fn status_is_state() -> Vec<u8> {
    seq(&[i(STATE_AT), i(STATE_AT), i(64), vec![0x10], index_of_host("state_get"), vec![0x10], index_of_host("status")])
}

fn index_of_host(name: &str) -> Vec<u8> {
    let at = HOST.iter().position(|(n, _, _)| *n == name).unwrap();
    let mut o = Vec::new();
    uleb(&mut o, (KIT.len() + at) as u32);
    o
}

/// `widget_tap` that saves the hit id as one character ('0' + id, so '/' for -1, a tap on no region).
pub fn tap_saves_id() -> Vec<u8> {
    seq(&[i(TAP_AT), arg(0), i(48), vec![ADD], vec![0x3a, 0x00, 0x00], host("state_set", &[TAP_AT, 1])])
}

/// The code of an element of the kit drawn at the cursor (the local Y), which then moves down by the extent the
/// element returns and the theme's gap (`metric(GAP)`): the cursor layout docs/KIT.md shows. `before` are the
/// call's numbers before its y, `after` the ones after it.
pub fn at_cursor(name: &str, before: &[i32], after: &[i32]) -> Vec<u8> {
    let mut o: Vec<u8> = before.iter().flat_map(|&a| i(a)).collect();
    o.extend(arg(Y));
    o.extend(after.iter().flat_map(|&a| i(a)));
    o.extend(invoke(name));
    o.extend(arg(Y));
    o.push(ADD);
    o.extend(seq(&[i(21), invoke("metric"), vec![ADD], set(Y)]));
    o
}

/// The metric ids the modules here ask for (docs/KIT.md, Metrics).
pub const M_BUTTON_H: i32 = 12;
pub const M_MARGIN: i32 = 20;
pub const M_GAP: i32 = 21;

/// A button at `x` (code that leaves a number) and the cursor Y, at the theme's size, and a chip beside it: the
/// button returns its width, and the chip goes that far and the theme's gap to its right; then the cursor moves
/// down by the theme's button height and gap.
pub fn button_and_chip(x: &[u8], [bp, bn]: [i32; 2], [cp, cn]: [i32; 2]) -> Vec<u8> {
    seq(&[
        i(2),
        x.to_vec(),
        arg(Y),
        i(0),
        i(0),
        i(bp),
        i(bn),
        i(0),
        i(0),
        i(0),
        invoke("button"),
        x.to_vec(),
        vec![ADD],
        i(M_GAP),
        invoke("metric"),
        vec![ADD],
        set(X),
        i(3),
        arg(X),
        arg(Y),
        i(0),
        i(0),
        i(cp),
        i(cn),
        i(1),
        invoke("chip"),
        vec![DROP],
        arg(Y),
        i(M_BUTTON_H),
        invoke("metric"),
        vec![ADD],
        i(M_GAP),
        invoke("metric"),
        vec![ADD],
        set(Y),
    ])
}

/// The demo widget: a heading, a paragraph wrapped to the body (as tall as its lines), a row with a switch (hit
/// region 1), and a button at the theme's size (2) with a chip (3) beside it, each under the one before by the extent
/// it returned (a button's is its width: the chip is placed by it) and the theme's gap. Its status line is the saved state, which a tap sets to the hit region's id.
pub fn demo_widget() -> Kit {
    let mut t = Texts::default();
    let [hp, hn] = t.put("Package kit");
    let [pp, pn] = t.put("A widget drawn with Grimoire's own elements, at the theme's sizes, in the body it is given.");
    let [rp, rn] = t.put("A row");
    let [ip, il] = t.put("gear");
    let [bp, bn] = t.put("Button");
    let [cp, cn] = t.put("Chip");
    let code = seq(&[
        i(0),
        set(Y),
        at_cursor("text", &[0], &[0, 0, hp, hn, 4, 0]),
        at_cursor("text", &[0], &[0, 0, pp, pn, 0, 4]),
        at_cursor("row", &[1, 0], &[0, 0, ip, il, rp, rn, 0, 1]),
        button_and_chip(&i(0), [bp, bn], [cp, cn]),
        status_is_state(),
    ]);
    Kit { data: vec![(TEXTS_AT as u32, t.0)], ..Kit::new(Host::Widget, code).touch(tap_saves_id()) }
}

/// The demo app: from the theme's margin, in a column 560 px wide, a page title, a wrapped paragraph, a row with a
/// switch (hit region 1), a button (2), a chip (3), a field (4), a select (5) and a checkbox (6), each under the
/// one before as the demo widget's are; an icon button (7) at the top right; a pager (8 and 9) at the foot; and,
/// given no height, the row Settings > System draws fourth (10), at the place Settings draws it on a Scribe's
/// screen under the status bar (its top 716 on the screen, 656 px from the left, 1154 wide). Each touch adds one
/// to a count the status line says in six digits.
pub fn demo_app() -> Kit {
    let mut t = Texts::default();
    let [hp, hn] = t.put("Package kit");
    let [pp, pn] = t.put("An app drawn with Grimoire's own elements, at the theme's sizes: a row, a button, a chip, a field, a select and a checkbox.");
    let [rp, rn] = t.put("A row");
    let [ip, il] = t.put("gear");
    let [bp, bn] = t.put("Button");
    let [cp, cn] = t.put("Chip");
    let [sp, sn] = t.put("search");
    let [fp, fn_] = t.put("Search");
    let [kp, kn] = t.put("Sort");
    let [vp, vn] = t.put("Name");
    let [xp, xn] = t.put("A checkbox");
    let [dp, dn] = t.put("pdf");
    let [lp, ln] = t.put("Darker lines in imported PDFs: Off");
    const COUNT_AT: i32 = 128;
    const DIGITS_AT: i32 = 136;
    // i32.load and i32.store of a word
    let count = seq(&[i(COUNT_AT), i(COUNT_AT), vec![0x28, 0x02, 0x00], i(1), vec![ADD], vec![0x36, 0x02, 0x00]]);
    let say = seq(&[i(COUNT_AT), vec![0x28, 0x02, 0x00], digits_of(DIGITS_AT, 6), host("status", &[DIGITS_AT, 6])]);
    // (the x of every element: the theme's margin, kept in the local T after the cursor's start)
    let margin = seq(&[i(M_MARGIN), invoke("metric")]);
    let left = |name: &str, before: &[i32], after: &[i32]| -> Vec<u8> {
        let mut o: Vec<u8> = before.iter().flat_map(|&a| i(a)).collect();
        o.extend(margin.clone());
        o.extend(arg(Y));
        o.extend(after.iter().flat_map(|&a| i(a)));
        o.extend(invoke(name));
        o.extend(arg(Y));
        o.push(ADD);
        o.extend(seq(&[i(M_GAP), invoke("metric"), vec![ADD], set(Y)]));
        o
    };
    let code = seq(&[
        margin.clone(),
        set(Y),
        left("text", &[], &[560, 0, hp, hn, 7, 0]),
        left("text", &[], &[560, 0, pp, pn, 0, 4]),
        left("row", &[1], &[560, 0, ip, il, rp, rn, 0, 1]),
        button_and_chip(&margin, [bp, bn], [cp, cn]),
        left("field", &[4], &[560, 0, sp, sn, 0, 0, fp, fn_, 0]),
        left("select", &[5], &[560, 0, kp, kn, vp, vn]),
        left("checkbox", &[6], &[560, 0, xp, xn, 0]),
        call("icon_button", &[7, 1714, 50, 0, 0, sp, sn, 0]),
        call("pager", &[8, 9, 50, 2200, 1760, 1, 3]),
        call("row", &[10, 656, 638, 1154, 0, dp, dn, lp, ln, 0, -1]),
        count.clone(),
        say,
    ]);
    Kit { data: vec![(TEXTS_AT as u32, t.0)], ..Kit::new(Host::App, code).touch(count) }
}

/// A module that imports a call of `grimoire_kit` a later level would have.
pub fn later(host: Host) -> Kit {
    let mut k = Kit::new(host, Vec::new());
    k.imports.push(("grimoire_kit", "later_call", "", 'v'));
    k
}

/// A package of the scenarios' data root.
pub struct Package {
    pub id: &'static str,
    pub name: &'static str,
    pub app: bool,
    pub manifest: String,
    pub module: Vec<u8>,
}

fn widget(id: &'static str, name: &'static str, capabilities: &[&str], module: Vec<u8>) -> Package {
    Package { id, name, app: false, manifest: widget_manifest(id, name, capabilities, 900, 1100, &module), module }
}

fn app(id: &'static str, name: &'static str, module: Vec<u8>) -> Package {
    Package { id, name, app: true, manifest: app_manifest(id, name, &[], true, &module), module }
}

/// The packages of tests/fixtures/kit: the widgets in the order of their index, then the apps in theirs.
pub fn packages() -> Vec<Package> {
    vec![
        widget("test.kit.widget", "Kit widget", &WIDGET_BOTH, demo_widget().bytes()),
        // (the same module, in a package that declares no capability, as an earlier install could leave one)
        widget("test.kit.plain", "Kit plain", &[], demo_widget().bytes()),
        widget("test.kit.later", "Kit later", &WIDGET_BOTH, later(Host::Widget).bytes()),
        app("test.kit.app", "Kit app", demo_app().bytes()),
        app("test.kit.lateapp", "Kit later app", later(Host::App).bytes()),
    ]
}

/// The installed data root's `packages` folder with these packages in it, each file's path and bytes.
pub fn scenario_files(packages: &[Package]) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    for app in [false, true] {
        let mut index = String::from("{\n  \"version\": 1,\n  \"packages\": [");
        for (i, p) in packages.iter().filter(|p| p.app == app).enumerate() {
            let (dir, file) = if app { (format!("packages/apps/{}", p.id), "app.wasm") } else { (format!("packages/widgets/{}", p.id), "module.bin") };
            index.push_str(if i > 0 { ",\n    " } else { "\n    " });
            index.push_str(&format!(
                "{{\"id\":\"{}\",\"name\":\"{}\",\"version\":\"1.0.0\",\"author\":\"Grimoire tests\",\"description\":\"A module of the package kit's tests.\",\"icon\":\"\",\"path\":\"{dir}/{file}\",\"sha\":\"{}\",\"installed_at\":1,\"source_url\":\"\"}}",
                p.id,
                p.name,
                sha256_of(&p.module),
            ));
            files.push((format!("{dir}/package.yaml"), p.manifest.clone().into_bytes()));
            files.push((format!("{dir}/{file}"), p.module.clone()));
        }
        index.push_str("\n  ]\n}\n");
        files.push((if app { "packages/apps-index.json" } else { "packages/index.json" }.to_string(), index.into_bytes()));
    }
    files
}
