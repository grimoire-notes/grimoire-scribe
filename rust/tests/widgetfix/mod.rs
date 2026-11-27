// widgetfix: the WebAssembly modules the widget host's tests and scenarios run, written here byte by byte (no
// compiler, no file from anywhere else), and the installed packages the scenarios copy
// (tests/fixtures/widgethost/ and tests/fixtures/widgetruns/: `make widget-fixtures` writes them;
// test_widget_fixtures.rs checks the committed bytes are what this makes now).
//
// Every module here imports the ten functions of the `grimoire` ABI (docs/WIDGETS.md) and exports its memory,
// `widget_draw` and `widget_tap`. In most, each of the two is a straight line of the steps it is given (`Op`):
// constants pushed, imports called (`module`). That is enough to pass the host any numbers. The ones that
// loop, or have more than one page of one memory, are written from the bytes of their bodies (`Wasm`).
#![allow(dead_code)]

use grimoire::sha256::sha256_of;

/// One step of a function's body.
#[derive(Clone, Debug)]
pub enum Op {
    /// i32.const
    I(i32),
    /// a call of the import with this name
    Call(&'static str),
    /// local.get: a parameter (widget_draw's width, height and scale; widget_tap's hit id)
    Arg(u32),
    /// drop: a result that is not used
    Drop,
    /// i32.add
    Add,
    /// i32.store8: the value on top to the address under it
    Store8,
}

/// The ABI's imports in the order a module here imports them (a function's index), each with its type's index.
const IMPORTS: [(&str, u8); 10] = [
    ("rect", 0),
    ("text", 1),
    ("button", 1),
    ("hit", 0),
    ("status", 2),
    ("request_redraw", 3),
    ("setting", 4),
    ("state_get", 5),
    ("state_set", 2),
    ("now", 6),
];
/// The function types: how many i32 parameters, and the result (0x7f an i32, 0x7e an i64, 0 none).
const TYPES: [(usize, u8); 8] = [(5, 0), (8, 0), (2, 0), (1, 0), (4, 0x7f), (2, 0x7f), (0, 0x7e), (3, 0)];
const TYPE_DRAW: u8 = 7;
const TYPE_TAP: u8 = 3;

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

/// The steps as a function body's bytes, with no locals named before them and no end after them.
pub fn code(ops: &[Op]) -> Vec<u8> {
    let mut code = Vec::new();
    for op in ops {
        match op {
            Op::I(v) => {
                code.push(0x41);
                sleb(&mut code, *v);
            }
            Op::Call(import) => {
                let at = IMPORTS.iter().position(|(n, _)| n == import).unwrap_or_else(|| panic!("no import {import}"));
                code.push(0x10);
                uleb(&mut code, at as u32);
            }
            Op::Arg(n) => {
                code.push(0x20);
                uleb(&mut code, *n);
            }
            Op::Drop => code.push(0x1a),
            Op::Add => code.push(0x6a),
            Op::Store8 => code.extend_from_slice(&[0x3a, 0x00, 0x00]),
        }
    }
    code
}

fn body(ops: &[Op]) -> Vec<u8> {
    let mut code = vec![0u8]; // no locals
    code.extend(self::code(ops));
    code.push(0x0b);
    let mut out = Vec::new();
    uleb(&mut out, code.len() as u32);
    out.extend_from_slice(&code);
    out
}

/// A module whose `widget_draw` and `widget_tap` run these steps, with `data` in its memory (address, bytes).
pub fn module(draw: &[Op], tap: &[Op], data: &[(u32, &[u8])]) -> Vec<u8> {
    module_importing(draw, tap, data, &[])
}

/// `module`'s module that imports these functions from `grimoire` too, after the ABI's, each with one i32
/// parameter and no result.
pub fn module_importing(draw: &[Op], tap: &[Op], data: &[(u32, &[u8])], more: &[&str]) -> Vec<u8> {
    let mut out = b"\0asm\x01\0\0\0".to_vec();
    let mut s = Vec::new();
    uleb(&mut s, TYPES.len() as u32);
    for (params, result) in TYPES {
        s.push(0x60);
        uleb(&mut s, params as u32);
        s.extend(std::iter::repeat_n(0x7f, params));
        if result == 0 {
            s.push(0);
        } else {
            s.extend_from_slice(&[1, result]);
        }
    }
    section(&mut out, 1, &s);
    s.clear();
    uleb(&mut s, (IMPORTS.len() + more.len()) as u32);
    for (import, ty) in IMPORTS.into_iter().chain(more.iter().map(|m| (*m, TYPE_TAP))) {
        name(&mut s, "grimoire");
        name(&mut s, import);
        s.extend_from_slice(&[0x00, ty]);
    }
    section(&mut out, 2, &s);
    section(&mut out, 3, &[2, TYPE_DRAW, TYPE_TAP]);
    section(&mut out, 5, &[1, 0x00, 1]); // one memory, one page, no maximum
    let first = (IMPORTS.len() + more.len()) as u32; // widget_draw's index, after the imports
    s.clear();
    s.push(3);
    name(&mut s, "memory");
    s.extend_from_slice(&[0x02, 0]);
    name(&mut s, "widget_draw");
    s.push(0x00);
    uleb(&mut s, first);
    name(&mut s, "widget_tap");
    s.push(0x00);
    uleb(&mut s, first + 1);
    section(&mut out, 7, &s);
    s.clear();
    s.push(2);
    s.extend_from_slice(&body(draw));
    s.extend_from_slice(&body(tap));
    section(&mut out, 10, &s);
    if !data.is_empty() {
        s.clear();
        uleb(&mut s, data.len() as u32);
        for (at, bytes) in data {
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

// ---- modules with more than a straight line: loops, several memories, tables, a start function ----

/// The local every function of a `Wasm` module counts its loops in (each function there has four i32 locals
/// after its parameters; this index is one of them in `widget_draw` and in `widget_tap`).
pub const COUNTER: u32 = 3;

/// `body`, again and again with no way out: a loop that branches back to its start.
pub fn forever(body: &[u8]) -> Vec<u8> {
    let mut o = vec![0x03, 0x40];
    o.extend_from_slice(body);
    o.extend_from_slice(&[0x0c, 0x00, 0x0b]);
    o
}

/// `body`, `n` times, counted in the local COUNTER (which `body` may read: it runs from 0).
pub fn repeat(n: i32, body: &[u8]) -> Vec<u8> {
    let mut o = code(&[Op::I(0)]);
    o.push(0x21);
    uleb(&mut o, COUNTER);
    o.extend_from_slice(&[0x03, 0x40]);
    o.extend_from_slice(body);
    o.push(0x20);
    uleb(&mut o, COUNTER);
    o.extend(code(&[Op::I(1), Op::Add]));
    o.push(0x22);
    uleb(&mut o, COUNTER);
    o.extend(code(&[Op::I(n)]));
    // i32.lt_u, br_if 0, end
    o.extend_from_slice(&[0x49, 0x0d, 0x00, 0x0b]);
    o
}

/// local.get COUNTER, for a `repeat` body that draws by its turn.
pub fn counter() -> Vec<u8> {
    let mut o = vec![0x20];
    uleb(&mut o, COUNTER);
    o
}

/// A module given as the bytes of its functions' bodies, with any number of memories and tables (pages or
/// entries at the start, and the most each may grow to), a start function and functions beside the two exports.
/// It imports and exports what `module`'s do; the memory exported is the first.
pub struct Wasm {
    pub memories: Vec<(u32, Option<u32>)>,
    pub tables: Vec<(u32, Option<u32>)>,
    pub draw: Vec<u8>,
    pub tap: Vec<u8>,
    /// runs when the module is instantiated
    pub start: Option<Vec<u8>>,
    /// functions with no parameter and no result, from index FIRST_EXTRA
    pub extra: Vec<Vec<u8>>,
    pub data: Vec<(u32, Vec<u8>)>,
}

/// The index of a `Wasm` module's first function beside `widget_draw` and `widget_tap`.
pub const FIRST_EXTRA: u32 = IMPORTS.len() as u32 + 2;

impl Wasm {
    /// A module of one page of memory whose `widget_draw` is `draw` and whose `widget_tap` does nothing.
    pub fn new(draw: Vec<u8>) -> Wasm {
        Wasm { memories: vec![(1, None)], tables: Vec::new(), draw, tap: Vec::new(), start: None, extra: Vec::new(), data: Vec::new() }
    }

    pub fn bytes(&self) -> Vec<u8> {
        fn limits(s: &mut Vec<u8>, (min, max): (u32, Option<u32>)) {
            s.push(max.is_some() as u8);
            uleb(s, min);
            if let Some(max) = max {
                uleb(s, max);
            }
        }
        let mut out = b"\0asm\x01\0\0\0".to_vec();
        let mut s = Vec::new();
        // (the types of `module`, and one more: no parameter, no result)
        const TYPE_PLAIN: u8 = TYPES.len() as u8;
        uleb(&mut s, TYPES.len() as u32 + 1);
        for (params, result) in TYPES.into_iter().chain([(0, 0)]) {
            s.push(0x60);
            uleb(&mut s, params as u32);
            s.extend(std::iter::repeat_n(0x7f, params));
            if result == 0 {
                s.push(0);
            } else {
                s.extend_from_slice(&[1, result]);
            }
        }
        section(&mut out, 1, &s);
        s.clear();
        uleb(&mut s, IMPORTS.len() as u32);
        for (import, ty) in IMPORTS {
            name(&mut s, "grimoire");
            name(&mut s, import);
            s.extend_from_slice(&[0x00, ty]);
        }
        section(&mut out, 2, &s);
        let mut funcs: Vec<(u8, &[u8])> = vec![(TYPE_DRAW, &self.draw), (TYPE_TAP, &self.tap)];
        funcs.extend(self.extra.iter().map(|e| (TYPE_PLAIN, &e[..])));
        funcs.extend(self.start.iter().map(|e| (TYPE_PLAIN, &e[..])));
        s.clear();
        uleb(&mut s, funcs.len() as u32);
        s.extend(funcs.iter().map(|(ty, _)| *ty));
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
        s.push(3);
        name(&mut s, "memory");
        s.extend_from_slice(&[0x02, 0]);
        name(&mut s, "widget_draw");
        s.extend_from_slice(&[0x00, IMPORTS.len() as u8]);
        name(&mut s, "widget_tap");
        s.extend_from_slice(&[0x00, IMPORTS.len() as u8 + 1]);
        section(&mut out, 7, &s);
        if self.start.is_some() {
            s.clear();
            uleb(&mut s, IMPORTS.len() as u32 + funcs.len() as u32 - 1);
            section(&mut out, 8, &s);
        }
        s.clear();
        uleb(&mut s, funcs.len() as u32);
        for (_, code) in &funcs {
            let mut b = vec![1, 4, 0x7f]; // four i32 locals
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

pub fn call(import: &'static str, args: &[i32]) -> Vec<Op> {
    let mut v: Vec<Op> = args.iter().map(|&a| Op::I(a)).collect();
    v.push(Op::Call(import));
    v
}

pub fn rect(x: i32, y: i32, w: i32, h: i32, style: i32) -> Vec<Op> {
    call("rect", &[x, y, w, h, style])
}
#[allow(clippy::too_many_arguments)]
pub fn text(x: i32, y: i32, w: i32, h: i32, ptr: i32, len: i32, px: i32, center: i32) -> Vec<Op> {
    call("text", &[x, y, w, h, ptr, len, px, center])
}
#[allow(clippy::too_many_arguments)]
pub fn button(id: i32, x: i32, y: i32, w: i32, h: i32, ptr: i32, len: i32, style: i32) -> Vec<Op> {
    call("button", &[id, x, y, w, h, ptr, len, style])
}
pub fn hit(id: i32, x: i32, y: i32, w: i32, h: i32) -> Vec<Op> {
    call("hit", &[id, x, y, w, h])
}
pub fn status(ptr: i32, len: i32) -> Vec<Op> {
    call("status", &[ptr, len])
}

/// Where the modules here keep the saved state they read back, and the byte a tap writes.
pub const STATE_AT: i32 = 60000;
pub const TAP_AT: i32 = 61000;

/// `widget_draw`'s last steps in the modules that say which hit region was tapped: the saved state read into
/// memory and passed on as the status line.
pub fn status_is_state() -> Vec<Op> {
    vec![Op::I(STATE_AT), Op::I(STATE_AT), Op::I(64), Op::Call("state_get"), Op::Call("status")]
}

/// `widget_tap` for those modules: the hit id as one character ('0' + id, so '/' for -1, a tap on no region)
/// saved as the state.
pub fn tap_saves_id() -> Vec<Op> {
    vec![Op::I(TAP_AT), Op::Arg(0), Op::I(48), Op::Add, Op::Store8, Op::I(TAP_AT), Op::I(1), Op::Call("state_set")]
}

/// A package's manifest for `artifact`, its file `module.bin`. `size` is the lines under `spec.size`.
pub fn manifest(id: &str, name: &str, capabilities: &[&str], size: &str, artifact: &[u8]) -> String {
    format!(
        "name: {name}\nid: {id}\ntype: widget\nversion: 1.0.0\nauthor: Grimoire tests\ndescription: A module of the widget host's tests.\n\
         platforms: [all]\nartifact:\n  path: module.bin\n  sha256: {}\n  size: {}\ncapabilities: [{}]\n\
         spec:\n  runtime: wasm\n  size:\n{size}",
        sha256_of(artifact),
        artifact.len(),
        capabilities.join(", "),
    )
}

pub const FIXED_400_300: &str = "    mode: fixed\n    default: [400, 300]\n";
/// A resizable frame whose manifest asks for a smallest height under the smallest frame's (120).
pub const SMALLEST_120_80: &str = "    mode: resizable\n    default: [220, 160]\n    min: [120, 80]\n    max: [400, 300]\n";
/// A manifest that names no size: the frame is the default one, 220x160, and does not resize.
pub const UNSIZED: &str = "    mode: fixed\n";
pub const BOTH: [&str; 2] = ["editor:overlay", "storage:widget-local"];

/// A package of the scenarios' data root: its id, name, manifest and module, and the SHA-256 its index entry
/// records (the module's own, but for the package made to differ).
pub struct Package {
    pub id: String,
    pub name: &'static str,
    pub manifest: String,
    pub module: Vec<u8>,
    pub recorded: String,
}

fn package(id: &str, name: &'static str, capabilities: &[&str], size: &str, module: Vec<u8>) -> Package {
    Package { id: id.to_string(), name, manifest: manifest(id, name, capabilities, size, &module), recorded: sha256_of(&module), module }
}

/// An id of the greatest length an id may have, 128 bytes, with every kind of character an id has.
pub fn longest_id() -> String {
    let id = format!("test.host.long.{}", "0123456789-abcdefghij.".repeat(6));
    format!("{}z", &id[..127])
}

/// A module that draws far outside its body: a selected (black) button from far above and left of the body to
/// far below and right of it, under a hit region as large. A tap saves which region it hit, and the next draw
/// says so in the status line.
pub fn wide() -> Vec<u8> {
    let mut draw = hit(7, -100_000, -100_000, 300_000, 300_000);
    draw.extend(button(1, -5000, -5000, 20_000, 20_000, 0, 0, 1));
    draw.extend(status_is_state());
    module(&draw, &tap_saves_id(), &[])
}

/// Where `keeps` has its word.
pub const WORD_AT: i32 = 32;

/// A module that keeps a word: each draw says the saved state in the status line, saves "hello" and draws a
/// button with that label (region 1).
pub fn keeps() -> Vec<u8> {
    let mut draw = status_is_state();
    draw.extend(call("state_set", &[WORD_AT, 5]));
    draw.extend(button(1, 10, 10, 200, 80, WORD_AT, 5, 0));
    module(&draw, &tap_saves_id(), &[(WORD_AT as u32, b"hello")])
}

/// The steps of `ops`, `n` times over.
pub fn times(n: usize, ops: Vec<Op>) -> Vec<Op> {
    (0..n).flat_map(|_| ops.clone()).collect()
}

/// A module whose `widget_draw` runs `draw` and whose `widget_tap` does nothing.
pub fn draws(draw: &[Op]) -> Vec<u8> {
    module(draw, &[], &[])
}

/// A module that draws `n` small rectangles in a pass, side by side.
pub fn many(n: usize) -> Vec<u8> {
    draws(&(0..n as i32).flat_map(|i| rect(i % 32 * 10, i / 32 * 10, 8, 8, 0)).collect::<Vec<_>>())
}

/// The packages of tests/fixtures/widgethost, in the order of its index.
pub fn packages() -> Vec<Package> {
    vec![
        package("test.host.wide", "Wide", &BOTH, FIXED_400_300, wide()),
        // (the same module, in a package that declares no capability)
        package("test.host.plain", "Plain", &[], FIXED_400_300, wide()),
        // (one drawing command more than a pass may hold: 513)
        package("test.host.many", "Many", &BOTH, FIXED_400_300, many(513)),
        // (its file is not the one its install recorded: the index has another module's SHA-256)
        Package { recorded: sha256_of(&keeps()), ..package("test.host.changed", "Changed", &BOTH, FIXED_400_300, wide()) },
        // (installed with a smallest size of 120x80, under the smallest frame there is)
        package("test.host.small", "Small", &BOTH, SMALLEST_120_80, wide()),
        // (its id is as long as an id may be)
        package(&longest_id(), "Long", &BOTH, FIXED_400_300, keeps()),
    ]
}

/// A module whose drawing pass never ends: a loop with no way out.
pub fn loops() -> Vec<u8> {
    Wasm::new(forever(&[])).bytes()
}

/// Where `ticker` keeps its count.
pub const COUNT_AT: i32 = 100;

/// A module that counts its drawing passes: each pass adds one to a character that starts at '0' and says it in
/// the status line ("1" after the first pass), asks to be drawn again `after_ms` later, and draws a rectangle.
pub fn ticker(after_ms: i32) -> Vec<u8> {
    let mut draw = code(&[Op::I(COUNT_AT), Op::I(COUNT_AT)]);
    draw.extend_from_slice(&[0x2d, 0x00, 0x00]); // i32.load8_u
    draw.extend(code(&[Op::I(1), Op::Add, Op::Store8]));
    draw.extend(code(&status(COUNT_AT, 1)));
    draw.extend(code(&call("request_redraw", &[after_ms])));
    draw.extend(code(&rect(10, 10, 60, 30, 1)));
    let mut w = Wasm::new(draw);
    w.data = vec![(COUNT_AT as u32, b"0".to_vec())];
    w.bytes()
}

/// The packages of tests/fixtures/widgetruns, in the order of its index: modules that run for long, or again
/// and again.
pub fn packages_that_run() -> Vec<Package> {
    vec![
        // (its drawing pass never ends; its manifest names no size, so its frame is the default one, 220x160)
        package("test.runs.loop", "Loop", &BOTH, UNSIZED, loops()),
        // (it asks to be drawn again a second after each pass)
        package("test.runs.ticker", "Ticker", &BOTH, UNSIZED, ticker(1000)),
        // (the same module, in a package that declares no capability, as an earlier install could leave one)
        package("test.runs.bare", "Bare", &[], UNSIZED, ticker(1000)),
    ]
}

/// The data roots the scenarios copy: each a folder of tests/fixtures, with the packages installed in it.
pub fn roots() -> Vec<(&'static str, Vec<Package>)> {
    vec![("widgethost", packages()), ("widgetruns", packages_that_run())]
}

/// An installed data root's `packages` folder with these packages in it, each file's path and bytes.
pub fn scenario_files(packages: &[Package]) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut index = String::from("{\n  \"version\": 1,\n  \"packages\": [");
    for (i, p) in packages.iter().enumerate() {
        let dir = format!("packages/widgets/{}", p.id);
        index.push_str(if i > 0 { ",\n    " } else { "\n    " });
        index.push_str(&format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"version\":\"1.0.0\",\"author\":\"Grimoire tests\",\"description\":\"A module of the widget host's tests.\",\"icon\":\"\",\"path\":\"{dir}/module.bin\",\"sha\":\"{}\",\"installed_at\":1,\"source_url\":\"\"}}",
            p.id,
            p.name,
            p.recorded,
        ));
        files.push((format!("{dir}/package.yaml"), p.manifest.clone().into_bytes()));
        files.push((format!("{dir}/module.bin"), p.module.clone()));
    }
    index.push_str("\n  ]\n}\n");
    files.push(("packages/index.json".to_string(), index.into_bytes()));
    files
}
