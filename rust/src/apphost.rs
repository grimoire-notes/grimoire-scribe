//! Apps (2.3): the full-screen host for installed app packages.
//!
//! An app is a WebAssembly module in an app package (`type: app`, apppkg.rs). This module owns the
//! full-screen surface, the app lifecycle and the `grimoire` ABI. It is deliberately a sibling of
//! widgetwasm.rs, not a mode of it: an app controls the whole screen, receives every pointer phase,
//! can keep or hide the shared status bar, and returns Home with a two-finger bottom-to-middle
//! swipe or its own call to `close()`.
//!
//! The host is an overlay on the Library screen, so Grimoire's screen switching, settings and
//! library state stay exactly as they are. While an app is open it owns drawing and input.
//!
//! ABI exports (all optional except `app_draw`):
//!
//! ```text
//! app_draw(width: i32, height: i32, scale_milli: i32, elapsed_ms: i32)
//! app_pointer(phase: i32, x: i32, y: i32, source: i32)
//! app_key(code: i32, mods: i32, down: i32)
//! app_stop()
//! app_status_bar() -> i32
//! ```
//!
//! `phase` is 0 down, 1 move, 2 up; `source` is 0 touch, 1 pen. Coordinates are body-local.
//! A module may also export the package kit's `kit_event(kind, id, a, b, c) -> i32` (kithost.rs): the body's
//! contact recognised as a tap, a press or a drag, a key, a change of its settings. Input calls the module at once,
//! `app_pointer` for every phase and `kit_event` for each event; the drawing pass either owes is drawn by the tick,
//! once for all the input since the last pass, paced while a contact is down, and only the part of the screen it
//! changed is refreshed (`apphost_tick`). Keys reach an open app before Grimoire's shortcuts (app.rs).
//! The body is all a module draws on: every command is drawn with the canvas clipped to it, whatever
//! coordinates and sizes it carries, and the `clip` call makes that part smaller, never larger
//! (`AppRuntime::draw`). The shared status bar, when it is shown, is above the body.
//! `app_stop` is called exactly once when the app is leaving, whether by gesture, `close()`, an
//! update, or removal. It is the safe place to save state. `elapsed_ms` is the time since the last
//! drawing pass by the app's clock (`now_ms`).
//!
//! Imports (`grimoire`): drawing and Grimoire's global UI elements, state and settings, time and
//! date, battery and Wi-Fi state, screen geometry, and `close()`. All strings are UTF-8 in the
//! module's memory. See docs/APPS.md for the full list.
//!
//! A module has the capabilities its manifest declares and no other (docs/APPS.md, Capabilities):
//! `storage:app-local` is the saved state (`state_get`, `state_set`, the state file), and `network:http`
//! the `http_*` calls. Without the capability the call is still there to link against: `state_get`
//! gives nothing, `state_set` keeps nothing, and `http_start` answers -2. Drawing, touches, keys and
//! every other call need none: an app is what it draws.
//!
//! Each call of the module has a fixed instruction budget, from which the host's work for the imports it calls
//! is paid as well, and is held to the limits below (docs/APPS.md, Limits).
//!
//! A module may also import the package kit, `grimoire_kit` (kithost.rs, docs/KIT.md), which the widget host
//! links too: its drawing calls are recorded with this host's own commands, in the order the module called them,
//! and held to this host's limits. An element of the kit given an id of 0 or more is a hit region; a drawing
//! pass's hit regions are the runner's names for the app's controls (`apphost_control_point`).

use crate::globalui::{gui_app_tile, gui_chip, gui_fab, gui_select, gui_status_bar, gui_status_h, gui_surface, gui_text_field};
use crate::kithost::{
    drawn_text_charge, icon_charge, kit_keep, kit_link, kit_missing_import, kit_render, label_charge, KitCmd, KitEvent, KitHost,
    KitInput, EV_KEY, EV_SETTINGS, KIT_EVENT,
};
use crate::prelude::*;
use crate::wasmhost::{charge, in_body, line_in_reach, on_screen, Redraw};
use std::time::Instant;
use std::os::unix::process::CommandExt;
use std::process::Command;
use wasmi::{Caller, Config, Engine, Linker, Module, Store, StoreLimits};

const ABI: &str = "grimoire";
const STATE_FILE: &str = "state/runtime.bin";

// ---- the limits: what one app and one call of its module may ask for (docs/APPS.md, Limits) ----
//
// The host calls a module's `app_draw` (a drawing pass), `app_pointer`, `app_key`, `app_status_bar` and
// `app_stop`, and runs its start. These are the numbers by which the host counts what a call asks of it, and
// what a module may hold. The counts start at nothing with each call, a count or a length is compared with
// its limit before the module's memory is read, and a call that goes past one is stopped as a call that used
// up its instructions is (`past`).

// (The module's file is at most WIDGET_PKG_ARTIFACT_MAX, widgetpkg.rs.)
/// A call's instruction budget: the module's own instructions and the host's work for its imports (the charges
/// below) are paid from it. Every call of the module by the host has this much. A call that needs more is
/// stopped. A function's first call also pays the interpreter for making it ready to run, about 7 for each
/// byte of its code (docs/APPS.md, Limits).
pub const APP_CALL_FUEL: u64 = 30_000_000;
/// A module has one linear memory, and it may not grow past this.
const MEMORY_MAX: usize = 48 * 1024 * 1024;
/// A module has at most one table, of at most this many entries (the function references its indirect calls
/// go through), and a store holds one instance of one module (`wasmhost::store_limits`). A module that
/// declares a second memory or table, or one larger than its limit, is not started; `memory.grow` and
/// `table.grow` past a limit fail as WebAssembly defines it (they return -1), and the module runs on.
pub const APP_TABLE_MAX: usize = 65_536;
/// The saved state: `state_set` keeps the first this many bytes.
pub const APP_STATE_MAX: usize = 256 * 1024;
/// The bytes `state_get` and `state_set` may move in one call, all their uses together: the whole state read
/// and written several times over.
pub const APP_CALL_STATE_MAX: usize = 16 * APP_STATE_MAX;
/// The drawing commands of one call, of every kind together (the eighteen drawing calls, `clip` and
/// `clip_clear` among them, and the kit's).
pub const APP_CALL_COMMANDS_MAX: usize = 4096;
/// The hit regions of one call: the kit's elements given an id, and its `hit`.
pub const APP_CALL_HITS_MAX: usize = 512;
/// The bytes of one text: a `text`'s, a `text_at`'s, what `text_width` measures, each label, caption, value
/// and hint of the drawing calls' elements, and each text the package kit reads.
pub const APP_TEXT_MAX: usize = 1024;
/// The bytes of all those texts of one call together.
pub const APP_CALL_TEXT_MAX: usize = 64 * 1024;
/// The bytes of an icon's name. No icon has a longer one: such a name is not read, and names no icon.
pub const APP_ICON_NAME_MAX: usize = 64;
/// The bytes of the status line.
pub const APP_STATUS_MAX: usize = 256;
/// The bytes of a toast's text that are read: its first this many (a toast shows no more).
pub const APP_TOAST_MAX: usize = 256;
/// The bytes of the connected network's name that `wifi_ssid` writes: the longest a network's name is.
pub const APP_SSID_MAX: usize = 32;
/// The delay `request_redraw` may name, in milliseconds; one outside is taken as the nearer end. The shortest
/// is how often an app can have itself drawn again, and the screen refreshed, by asking.
pub const APP_REDRAW_MIN_MS: i32 = 100;
pub const APP_REDRAW_MAX_MS: i32 = 86_400_000;
/// A text is drawn and measured at most this many pixels high; a larger height is taken as this one. Five
/// digits this high are as wide as the screen.
pub const APP_TEXT_PX_MAX: i32 = 768;
/// An icon is drawn at most this many pixels each way; in a larger rectangle it is this size, in the middle.
/// (Under the greatest size the font makes a glyph at, for any caller: font.rs, GLYPH_SIDE_MAX.)
pub const APP_ICON_PX_MAX: i32 = 1024;
/// A line is drawn at most this many pixels wide, wider than any body; a wider one is drawn at this width.
pub const APP_LINE_WIDTH_MAX: f32 = 4096.0;
/// A thickness (an outline's, a round rectangle's, a surface's) is held within this of nothing: a line
/// thicker than half of the largest rectangle `on_screen` gives fills it as this one does, and one less than
/// nothing draws as nothing does.
const THICK_REACH: i32 = 1 << 17;

// ---- the charges: what the host's work for an import takes from the call's budget (docs/APPS.md, Limits) ----
//
// The budget counts the module's instructions. The host's work for an import is no instruction of the module's,
// so each import takes a charge for it from the same budget (`wasmhost::charge`, which the widget host takes
// its own with): a fixed one for every call, and one by size where the work grows with what the import is
// given. The numbers are in instructions: about what the work costs beside a module's own, as measured in a
// release build on a desktop machine, where the longest drawing a budget pays for takes about four times as long
// as the same budget spent on instructions alone (labels broken into a line for every letter, in the Gothic
// theme), and a label's time follows the font in use, for the same charge (not measured on a Kindle). When the
// budget cannot pay, the call ends exactly as a call that used up its instructions ends. The charges are the
// same in both themes: where the themes draw an element differently, the charge is for the one that takes more
// work.

/// Every call of an import, whatever it is given and whether or not the app has the capability for it.
pub const APP_CHARGE_CALL: u64 = 500;
/// A shape (a rectangle, an outline, a round rectangle, each fill and outline of the kit's elements): one for
/// this many pixels of the part of it inside the body, which are filled by rows.
pub const APP_CHARGE_FILL_PX: u64 = 32;
/// And this for each pixel of it that is worked out by itself: those within a round rectangle's radius or an
/// outline's thickness of its left and right sides, a line's, and the corners and round ends of the kit's
/// elements.
pub const APP_CHARGE_EDGE_PX: u64 = 2;
/// How far from its left and right sides one of the kit's elements with square ends is worked out pixel by
/// pixel (its corners and its outline), in pixels. One with round ends: half its smaller side more.
pub const APP_CHARGE_CORNER: i64 = 32;
/// `line`: this for each row of the body the line crosses, beside the row's pixels.
pub const APP_CHARGE_LINE_ROW: u64 = 16;
/// `text`, `text_at` and the texts of the kit's elements: for each byte, this for each pixel of the height up
/// to APP_CHARGE_TALL_PX; a taller letter costs by its area, so above that height a byte is charged by the
/// square of the height (the same at APP_CHARGE_TALL_PX exactly).
pub const APP_CHARGE_TEXT: u64 = 32;
pub const APP_CHARGE_TALL_PX: u64 = 64;
/// A text in the bold, the title or the caps face: and, for each byte, one for this much of the cube of its
/// height. Where such a face has no bold of its own (the pixel face of the Gothic theme, a font of the owner's
/// without one), each letter is printed over again and again, more times the taller it is.
pub const APP_CHARGE_BOLD: u64 = 128;
/// An icon: this for each pixel of the square of its side (its glyph is made that large, and laid on the body).
pub const APP_CHARGE_ICON_PX: u64 = 2;
/// A text that is fitted to its room: this for the square of its bytes. A `text` wider than its room is cut
/// short by steps that each measure it again, and so is the one line of a chip, a select, an app tile and a
/// text field: their bytes are counted to APP_CHARGE_SQUARE_BYTES (a text is cut to its first 251 bytes before
/// those steps: font.rs, `fit`). The label of a button or a row may be broken into two lines, by steps that
/// measure all of it: all its bytes are counted.
pub const APP_CHARGE_SQUARE: u64 = 16;
pub const APP_CHARGE_SQUARE_BYTES: u64 = 256;
/// The height the texts of the kit's elements are charged at: the kit draws them at its own sizes, all under it.
pub const APP_CHARGE_LABEL_PX: i64 = 48;
/// The side the kit's icons of one size are charged at: a row's icon, a select's chevron, the mark that clears a
/// text field. (And the least a text field's own icon is charged at: that one is as tall as the field.)
pub const APP_CHARGE_SMALL_ICON_PX: i64 = 64;
/// A row's switch: its two round frames, of one size.
pub const APP_CHARGE_SWITCH: u64 = 30_000;
/// `text_width`: each byte measured.
pub const APP_CHARGE_MEASURE: u64 = 32;
/// `status`, `toast`, `setting`, an icon's name: each byte read from the module's memory, or written to it, as
/// text.
pub const APP_CHARGE_BYTE: u64 = 4;
/// `state_get`, `state_set`, what `http_start` reads, `http_body`, `http_error`: one for this many bytes copied
/// as they are.
pub const APP_CHARGE_COPY_BYTES: u64 = 4;
/// `time_text`, `date_text`, `battery_percent`, `battery_charging`, `http_poll`: the calls that ask the system
/// for something each time.
pub const APP_CHARGE_SYSTEM: u64 = 20_000;
/// `http_start`, for a request that is read (the capability declared, none in flight, its lengths within their
/// limits): the start of its curl, whether or not the request then starts.
pub const APP_CHARGE_REQUEST: u64 = 7_500_000;

/// A drawing command recorded by the module and rendered by the host. Coordinates are body-local.
#[derive(Clone, Debug)]
enum DrawCmd {
    Fill(Rect, u8),
    Outline(Rect, i32, u8),
    Round(Rect, i32, u8, bool, i32),
    Line(f32, f32, f32, f32, f32, u8),
    Text(Rect, String, f32, FontFace, bool, u8),
    TextAt(i32, i32, String, f32, FontFace, u8),
    Icon(String, Rect, u8),
    Button(Rect, String, String, i32),
    Chip(Rect, String, bool),
    Row(Rect, String, String, i32, i32),
    Surface(Rect, bool, i32),
    Select(Rect, String, String),
    AppTile(Rect, String, String, bool),
    Fab(Rect, String),
    Card(Rect, i32),
    TextField(Rect, String, String, String),
    Clip(Rect),
    ClipClear,
    /// one of the package kit's (kithost.rs)
    Kit(KitCmd),
}

struct HostState {
    /// the package's id, for the log
    id: String,
    /// the manifest declares `storage:app-local`: the state is read, kept and saved
    storage: bool,
    /// the size of the body the call draws on (a drawing pass's; another call keeps the last pass's), for the
    /// charge of a shape by the part of it inside the body
    body: (i32, i32),
    commands: Vec<DrawCmd>,
    /// the hit regions of the kit's elements in this call (body-local)
    hits: Vec<(i32, Rect)>,
    /// the bytes of the texts read in this call, and the bytes state_get and state_set moved in it
    text_bytes: usize,
    state_bytes: usize,
    /// the bytes read from the module's memory in this call (`read_mem`)
    read_bytes: usize,
    state: Vec<u8>,
    state_path: String,
    settings: Vec<(String, String)>,
    close_requested: bool,
    status_bar: bool,
    status: String,
    texts: Vec<String>,
    toast: Option<(String, u32)>,
    dirty: bool,
    limits: StoreLimits,
    /// the module's request to be drawn again (when it asked, by the app's clock, and how long after); None while
    /// no request stands. One request stands at a time, the soonest asked for, until the host's tick takes it
    /// (`redraw_taken`).
    redraw_at: Option<Redraw>,
    screen_w: i32,
    screen_h: i32,
    clock_12h: bool,
    /// The manifest's `network:http` capability. Without it HTTP imports refuse.
    network_http: bool,
    /// The one asynchronous curl request an app may have in flight.
    http: Option<HttpJob>,
    /// the points of the DRAG_MOVE `kit_event` is being told of (`event_points`); none in another call
    points: Vec<(i32, i32)>,
    /// Tests: the program started in curl's place and its first arguments (empty: curl itself).
    #[cfg(test)]
    http_stand_in: Vec<String>,
}

/// What the package kit's calls (kithost.rs) are given by this host: its body, its limits and its records. An app
/// may always draw.
impl KitHost for HostState {
    fn kit_body(&self) -> (i32, i32) {
        self.body
    }
    fn kit_may_draw(&self) -> bool {
        true
    }
    fn kit_room(&self) -> Result<(), wasmi::Error> {
        room_for_command(self)
    }
    fn kit_hit_room(&self) -> Result<(), wasmi::Error> {
        if self.hits.len() >= APP_CALL_HITS_MAX {
            return Err(past(self, "the hit regions of a call", APP_CALL_HITS_MAX));
        }
        Ok(())
    }
    fn kit_text(&mut self, n: usize) -> Result<(), wasmi::Error> {
        call_text_room(self, n)
    }
    fn kit_push(&mut self, cmd: Option<KitCmd>, hit: Option<(i32, Rect)>) {
        self.commands.extend(cmd.map(DrawCmd::Kit));
        self.hits.extend(hit);
    }
    fn kit_read(&mut self, n: usize) {
        self.read_bytes += n;
    }
    fn kit_points(&self) -> &[(i32, i32)] {
        &self.points
    }
    fn kit_status_bar_h(&self) -> i32 {
        if self.status_bar {
            gui_status_h()
        } else {
            0
        }
    }
}

/// The limits of one app HTTP request (docs/APPS.md, HTTP). The answer's limit is given to curl,
/// set on the files curl may write, watched by `http_watch` while curl runs, and held again when
/// the answer is read.
const HTTP_ADDRESS_MAX: usize = 4096;
/// The method is at most HTTP_METHOD_MAX letters; its text as the module gives it, with any white
/// space around the method, is at most HTTP_METHOD_TEXT_MAX bytes.
const HTTP_METHOD_MAX: usize = 16;
const HTTP_METHOD_TEXT_MAX: usize = 64;
const HTTP_HEADERS_MAX: usize = 32 * 1024;
const HTTP_UPLOAD_MAX: usize = 2 * 1024 * 1024;
const HTTP_HEADER_LINES_MAX: usize = 32;
const HTTP_HEADER_LINE_MAX: usize = 2048;
const HTTP_REDIRECTS_MAX: u32 = 5;
const HTTP_ANSWER_MAX: u64 = 8 * 1024 * 1024;
const HTTP_TOO_LARGE: &str = "The answer is too large.";
/// The User-Agent of a request that has no `User-Agent` line of its own.
const HTTP_USER_AGENT: &str = "Grimoire-App/2.3";
/// The address space a request's curl may take beyond Grimoire's own at the request's start
/// (childcaps.rs): the margin a child with no number of its own has there.
const HTTP_CHILD_MARGIN: u64 = 256 << 20;

/// The variables curl takes a proxy from for an http or https address. A request's curl starts
/// without them (`http_no_proxy`): a request goes to the address it names, whatever Grimoire's
/// own environment holds.
const HTTP_PROXY_VARIABLES: [&str; 5] = ["http_proxy", "https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY"];

/// The kinds of a request's files: the endings of their names.
const HTTP_FILE_KINDS: [&str; 4] = ["body", "status", "err", "request"];

/// One request's files under the app's `state/http` folder, named by their common start: the
/// answer (`.body`), curl's status code (`.status`) and error text (`.err`), and the upload
/// (`.request`). They are removed when this is dropped, so no way out of a request keeps them.
struct HttpFiles {
    base: String,
}

impl HttpFiles {
    fn path(&self, kind: &str) -> String {
        format!("{}.{kind}", self.base)
    }

    fn remove(&self) {
        for kind in HTTP_FILE_KINDS {
            let _ = std::fs::remove_file(self.path(kind));
        }
    }
}

/// True for a name `http_start` gives a request's file: the process's number, a hyphen, the
/// start's time as a number, a dot and one of the four kinds.
fn http_file_name(name: &str) -> bool {
    let number = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    name.rsplit_once('.').is_some_and(|(start, kind)| {
        HTTP_FILE_KINDS.contains(&kind) && start.split_once('-').is_some_and(|(pid, tick)| number(pid) && number(tick))
    })
}

/// Makes an app's state folder `state` and its request folder `folder` (`state/http`) when they
/// are missing, and says whether both are real folders: folders themselves, not links to one.
/// Neither name is followed when it is a link, so nothing is made behind one.
fn http_folders(state: &str, folder: &str) -> bool {
    let real = |path: &str| std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir());
    let missing = |path: &str| std::fs::symlink_metadata(path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
    (real(state) || missing(state)) && (real(folder) || missing(folder)) && mkdir_p(folder) == 0 && real(state) && real(folder)
}

/// Removes the request files an earlier run left in `folder`, an app's `state/http`: a Grimoire
/// that stopped while a request ran removed none of them. Only plain files with a request file's
/// name go. A link in the folder is neither followed nor removed, and every other name stays.
/// When `folder`'s own name is a link, or anything but a folder, nothing is removed; the names
/// before it in the path are followed as any path's are, and are the caller's to check
/// (`http_start` does, with `http_folders`). For when no request of the host is in flight.
fn http_sweep(folder: &str) {
    if !std::fs::symlink_metadata(folder).is_ok_and(|m| m.is_dir()) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    let left: Vec<std::path::PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()) && e.file_name().to_str().is_some_and(http_file_name))
        .map(|e| e.path())
        .collect();
    for path in left {
        let _ = std::fs::remove_file(path);
    }
}

impl Drop for HttpFiles {
    fn drop(&mut self) {
        self.remove();
    }
}

/// One app HTTP request, run by curl in the background. The app polls it from
/// `request_redraw` frames, so no network contact ever blocks drawing or touch. The host's tick
/// looks after it between the app's polls, and when the app does not poll (`http_watch`).
struct HttpJob {
    child: std::process::Child,
    files: HttpFiles,
    done: bool,
    code: i32,
    body: Vec<u8>,
    error: String,
}

impl HttpJob {
    /// Ends a curl that still runs and reaps it.
    fn stop(&mut self) {
        if !self.done {
            let _ = self.child.kill();
            let _ = self.child.wait();
            self.done = true;
        }
    }

    /// The answer file's size so far: 0 until curl has made it.
    fn answer_len(&self) -> u64 {
        std::fs::symlink_metadata(self.files.path("body")).map_or(0, |m| m.len())
    }

    /// The outcome of an answer over HTTP_ANSWER_MAX: no body, no status code, and the reason.
    fn too_large(&mut self) {
        self.code = -1;
        self.body = Vec::new();
        self.error = HTTP_TOO_LARGE.to_string();
    }
}

impl Drop for HttpJob {
    fn drop(&mut self) {
        // (curl ends first; `files` is dropped after this and removes what curl was writing)
        self.stop();
    }
}

/// One open app's WebAssembly instance and its last frame.
pub struct AppRuntime {
    _engine: Engine,
    store: Store<HostState>,
    instance: wasmi::Instance,
    status: String,
    texts: Vec<String>,
    error: Option<String>,
    /// what the last call took from its budget
    used: u64,
    /// the drawing commands of the last drawing pass
    drawn: usize,
    /// the hit regions of the last drawing pass, the part of each inside its body (body-local)
    hits: Vec<(i32, Rect)>,
}

impl std::fmt::Debug for AppRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppRuntime").field("error", &self.error).finish()
    }
}

/// The one running app. Its state lives on App because the Apps launcher is the Library.
#[derive(Default, Debug)]
pub struct AppRun {
    pub open: bool,
    pub id: String,
    pub name: String,
    pub icon: IconKind,
    pub status_bar: bool,
    pub error: String,
    pub body: Rect,
    runtime: Option<AppRuntime>,
    /// Two-finger swipe tracking is the recognizer's Gesture event; this remembers the last
    /// body size so a rotation or a status-bar change redraws into the new one.
    last_body: Rect,
    /// when the last drawing pass was, by the app's clock (`now_ms`): a pass's `elapsed_ms` is the time since
    last_draw: Option<u32>,
    launched_at: Option<Instant>,
    /// the body's contact recognised (the kit's events) and the drawing pass input owes (kithost.rs)
    input: KitInput,
}

impl AppRun {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn status(&self) -> &str {
        self.runtime.as_ref().map_or("", |r| r.status.as_str())
    }

    pub fn text(&self) -> String {
        self.runtime.as_ref().map_or_else(String::new, |r| r.texts.join("|"))
    }

    pub fn error(&self) -> &str {
        self.runtime.as_ref().and_then(|r| r.error.as_deref()).unwrap_or(self.error.as_str())
    }

    /// The hit regions of the last drawing pass, for the runner's probes.
    pub fn hit_count(&self) -> usize {
        self.runtime.as_ref().map_or(0, |r| r.hits.len())
    }

    fn take_close(&mut self) -> bool {
        self.runtime.as_mut().is_some_and(|r| std::mem::take(&mut r.store.data_mut().close_requested))
    }

    fn take_status_bar(&mut self) -> Option<bool> {
        let want = self.runtime.as_ref().map(|r| r.store.data().status_bar)?;
        if want == self.status_bar {
            None
        } else {
            self.status_bar = want;
            Some(want)
        }
    }

    fn take_toast(&mut self) -> Option<(String, u32)> {
        self.runtime.as_mut().and_then(|r| r.store.data_mut().toast.take())
    }

    /// The app's settings page saved them: the module reads the new values from now on, and is told so (a
    /// SETTINGS event), which owes a drawing pass when it says its picture changed.
    pub fn reload_settings(&mut self) {
        if let Some(m) = crate::apppkg::app_package_installed_manifest(&self.id) {
            if let Some(r) = self.runtime.as_mut() {
                r.store.data_mut().settings = crate::apppkg::app_settings_load(&self.id, &m.settings);
                if r.event(&KitEvent::new(EV_SETTINGS, 0, 0, 0, 0)) == Some(true) {
                    self.input.owe();
                }
            }
        }
    }

    /// A drawing pass. Its `elapsed_ms` is the time since the last one by the app's clock (`now_ms`, which a
    /// scenario's clock holds still), and it answers the pass input owed.
    fn draw(&mut self, c: &mut Canvas, body: Rect) {
        let now = now_ms();
        let elapsed = self.last_draw.map_or(0, |at| now.wrapping_sub(at).min(i32::MAX as u32) as i32);
        self.last_draw = Some(now);
        self.input.answered();
        let Some(r) = self.runtime.as_mut() else { return };
        r.draw(c, body, elapsed);
    }

    fn tick(&mut self) {
        if let Some(r) = self.runtime.as_mut() {
            r.tick();
        }
    }

    fn last_error(&self) -> Option<String> {
        self.runtime.as_ref().and_then(|r| r.error.clone())
    }
}

/// The body of the app: the screen under the shared status bar when it is on.
pub fn app_body(a: &App) -> Rect {
    let top = if a.app_view.status_bar { gui_status_h().max(0) } else { 0 };
    Rect::new(0, top, a.w, (a.h - top).max(0))
}

fn set_body(a: &mut App) {
    a.app_view.body = app_body(a);
}

/// Opens an installed app. False (with a toast) when it is gone or its module cannot start.
pub fn apphost_launch(a: &mut App, id: &str) -> bool {
    if a.app_view.open {
        apphost_close(a);
    }
    if crate::apppkg::app_package_installed_manifest(id).is_none() {
        app_toast(a, "That app is no longer installed.", 3000);
        return false;
    }
    // (the app's module, once its file has been checked again: apppkg.rs)
    let (it, m, module) = match crate::apppkg::app_package_artifact(id) {
        Ok(loaded) => loaded,
        Err(e) => {
            app_toast(a, &e, 5000);
            return false;
        }
    };
    match AppRuntime::new(&it, &m, &module, a.w, a.h, a.settings.clock_12h) {
        Ok(r) => {
            let icon = if m.spec.icon.trim().is_empty() { IconKind::Grid } else { widget_icon(&m.spec.icon) };
            a.app_view = AppRun {
                open: true,
                id: it.id.clone(),
                name: it.name.clone(),
                icon,
                status_bar: m.spec.status_bar,
                error: String::new(),
                body: RECT_EMPTY,
                runtime: Some(r),
                last_body: RECT_EMPTY,
                last_draw: Some(now_ms()),
                launched_at: Some(Instant::now()),
                input: KitInput::default(),
            };
            // An app may override the manifest preference once, before its first frame.
            if let Some(want) = a.app_view.runtime.as_mut().and_then(|r| r.export_status_bar()) {
                a.app_view.status_bar = want;
                if let Some(r) = a.app_view.runtime.as_mut() {
                    r.store.data_mut().status_bar = want;
                }
            }
            set_body(a);
            a.app_view.last_body = a.app_view.body;
            crate::library::library_app_launched(a);
            a.app_view.last_draw = Some(now_ms());
            let App { screen: c, app_view, .. } = a;
            app_view.draw(c, app_view.body);
            // (a first drawing pass that stopped the module: the app is not left open with nothing drawn)
            if apphost_failed(a) {
                return false;
            }
            app_invalidate_all(a, RefreshMode::Full);
            true
        }
        Err(e) => {
            app_toast(a, &e, 5000);
            false
        }
    }
}

/// The safe close path: `app_stop` runs, state is saved, then Home is drawn again. Every way out
/// (the system swipe, `close()`, update, removal) goes through here.
pub fn apphost_close(a: &mut App) {
    if !a.app_view.open {
        return;
    }
    if let Some(mut r) = a.app_view.runtime.take() {
        r.stop();
    }
    a.app_view.open = false;
    a.app_view.id.clear();
    a.app_view.name.clear();
    a.app_view.error.clear();
    a.app_view.body = RECT_EMPTY;
    a.app_view.last_body = RECT_EMPTY;
    a.app_view.launched_at = None;
    // The app owned the whole surface; put the Library back now.
    crate::library::library_app_closed(a);
}

/// True when an app currently owns the screen.
pub fn apphost_open(a: &App) -> bool {
    a.app_view.open
}

/// Draw the app and, when it asks for it, the shared status bar over the top.
pub fn apphost_draw(a: &mut App) {
    if !a.app_view.open {
        return;
    }
    let (aw, ah) = (a.w, a.h);
    let body = app_body(a);
    let App { screen: c, app_view, .. } = a;
    canvas_fill(c, Rect::new(0, 0, aw, ah), 255);
    if app_view.status_bar {
        gui_status_bar(c, aw, sys_wall_clock());
    }
    if !rect_is_empty(body) {
        app_view.body = body;
        app_view.draw(c, body);
    }
    if let Some(err) = app_view.runtime.as_ref().and_then(|r| r.error.as_deref()) {
        app_view.error = err.to_string();
    }
    a.toast_floor = if app_view.status_bar { gui_status_h() } else { 0 };
}

/// Handle a Library event while an app is open. Returns true when the app took it.
pub fn apphost_event(a: &mut App, ev: &InEvent) -> bool {
    if !a.app_view.open {
        return false;
    }
    if ev.type_ == InType::Gesture {
        if app_exit_gesture(a, ev) {
            return true;
        }
        return true;
    }
    if ev.type_ == InType::Key || ev.type_ == InType::KeyUp {
        apphost_key(a, ev.value, ev.key_mods as i32, ev.type_ == InType::Key);
        return true;
    }
    let ph = app_pointer_phase(ev);
    if ph < 0 {
        return false;
    }
    let body = app_body(a);
    if !rect_contains(body, ev.x.round_ties_even() as i32, ev.y.round_ties_even() as i32) {
        // The shared status bar is chrome, not app content. A lift still reaches the app so a
        // contact that began in the body can end cleanly.
        if ph != 2 {
            return true;
        }
    }
    let x = ev.x.round_ties_even() as i32 - body.x;
    let y = ev.y.round_ties_even() as i32 - body.y;
    let source = if matches!(ev.type_, InType::PenDown | InType::PenMove | InType::PenUp) { 1 } else { 0 };
    // (a finger the panel calls palm-sized, or one the platform cancelled, is no tap: kithost.rs KitPointer)
    let palm = source == 0 && ev.value & TOUCH_F_BROAD != 0;
    let slop = mm_to_px(TAP_SLOP_MM, a.dpi) as i32;
    let Some(mut r) = a.app_view.runtime.take() else { return true };
    // Every phase still calls `app_pointer`, and the events the contact makes call `kit_event`, at once; the
    // drawing pass either owes is drawn by the tick (apphost_tick).
    let mut owe = r.pointer(ph, x, y, source);
    let p = &mut a.app_view.input.pointer;
    let events = match ph {
        0 => p.down(x, y, source, r.hit_at(x, y), now_ms(), slop, palm),
        1 => p.moved(x, y, palm),
        _ => p.up(x, y, touch_cancelled(ev)),
    };
    for e in &events {
        owe |= r.event(e) == Some(true);
    }
    a.app_view.runtime = Some(r);
    if owe {
        a.app_view.input.owe();
    }
    apphost_after_input(a);
    true
}

/// A key pressed (`down`) or let go, for the open app: `app_key`, and a KEY event for `kit_event` (the key's code,
/// its modifiers, 1 down or 0 up). The drawing pass either owes is drawn by the tick.
pub fn apphost_key(a: &mut App, code: i32, mods: i32, down: bool) {
    if !a.app_view.open {
        return;
    }
    let Some(mut r) = a.app_view.runtime.take() else { return };
    let mut owe = r.key(code, mods, down);
    owe |= r.event(&KitEvent::new(EV_KEY, code, mods, i32::from(down), 0)) == Some(true);
    a.app_view.runtime = Some(r);
    if owe {
        a.app_view.input.owe();
    }
    apphost_after_input(a);
}

/// The two-finger swipe up from the bottom to the middle returns Home. It is a system gesture:
/// it works whatever gestures.ini and the Gestures setting say, and the app cannot disable it.
fn app_exit_gesture(a: &mut App, ev: &InEvent) -> bool {
    if ev.value != G_2_UP {
        return false;
    }        let (aw, ah) = (a.w.max(1), a.h.max(1));
        if gesture_matches(ev.x, ev.y, ev.x2, ev.y2, aw, ah) {
        apphost_close(a);
        return true;
    }
    false
}

/// After calls of the module for input (a touch, a key, a press, a drag's points, settings): what they asked for
/// besides drawing (to show or hide the status bar, a toast, to close), and a call that stopped the module, which
/// closes the app. The drawing pass they may owe is the tick's (apphost_tick). Returns whether the app was closed.
fn apphost_after_input(a: &mut App) -> bool {
    let status_change = a.app_view.take_status_bar();
    let toast = a.app_view.take_toast();
    let close = a.app_view.take_close();
    let error = a.app_view.last_error();
    if status_change.is_some() {
        set_body(a);
        a.app_view.last_body = a.app_view.body;
        apphost_draw(a);
        app_invalidate_all(a, RefreshMode::Full);
    }
    if let Some((text, ms)) = toast {
        app_toast(a, &text, ms.max(500) as i32);
    }
    if error.is_some() && apphost_failed(a) {
        return true;
    }
    if close {
        apphost_close(a);
        return true;
    }
    false
}

/// The drawing pass input owed: the app drawn once, and only the part of the screen the pass changed refreshed, in
/// `mode` (nothing, when it changed nothing).
fn apphost_pass(a: &mut App, mode: RefreshMode) {
    let screen = Rect::new(0, 0, a.w, a.h);
    let kept = kit_keep(&a.screen, screen);
    apphost_draw(a);
    let changed = kept.map_or(screen, |(k, at)| canvas_changed_rect(&k, &a.screen, at));
    if !rect_is_empty(changed) {
        app_invalidate(a, changed, mode);
    }
}

/// An app whose module was stopped (a call that failed, used up its budget or went past a limit) is closed,
/// and a toast says why. After a touch or a key that is at once; for a drawing pass, which nothing waits for,
/// the launch and the tick ask this. Returns whether the app was closed.
fn apphost_failed(a: &mut App) -> bool {
    let Some(e) = a.app_view.last_error() else { return false };
    a.app_view.error = e;
    app_toast(a, &a.app_view.error.clone(), 5000);
    apphost_close(a);
    true
}

/// Ticks an app's timers and its requested redraws. Called from the Library tick.
pub fn apphost_tick(a: &mut App, now: u32) {
    if !a.app_view.open {
        return;
    }
    a.app_view.tick();
    // (a drawing pass stopped the module since the last tick)
    if apphost_failed(a) {
        return;
    }
    // Input: a contact held still is a press; on the pace's turn a drag's gathered samples go to the module, and the
    // drawing pass input owes is drawn once for all the input since the last pass (kithost.rs KitInput).
    let fast_busy = display_fast_busy();
    let mut events: Vec<KitEvent> = a.app_view.input.pointer.tick(now).into_iter().collect();
    let turn = a.app_view.input.turn(now, fast_busy);
    if turn {
        events.extend(a.app_view.input.pointer.take_moves());
    }
    if !events.is_empty() {
        if let Some(mut r) = a.app_view.runtime.take() {
            let owe = events.iter().fold(false, |owe, e| r.event(e) == Some(true) || owe);
            a.app_view.runtime = Some(r);
            if owe {
                a.app_view.input.owe();
            }
        }
        if apphost_after_input(a) {
            return;
        }
    }
    if let Some(mode) = a.app_view.input.pass_now(turn, fast_busy) {
        apphost_pass(a, mode);
        if apphost_failed(a) {
            return;
        }
    }
    // A request that is due is taken, and the app is drawn and the screen refreshed once for it. A pass that
    // wants another asks again, and gets it no sooner than APP_REDRAW_MIN_MS later; while no request is due
    // nothing is drawn and nothing is refreshed.
    if a.app_view.runtime.as_mut().is_some_and(|r| r.redraw_taken(now)) {
        apphost_draw(a);
        app_invalidate_all(a, RefreshMode::Fast);
    }
    // The status bar's clock is the app's too when it is shown.
    if a.app_view.status_bar {
        if crate::library::library_app_minute(a) {
            apphost_draw(a);
            app_invalidate(a, Rect::new(0, 0, a.w, gui_status_h()), RefreshMode::Ui);
        }
    }
}

/// The runner's control points for an app: the body's centre (apps receive absolute taps too), and `app.hit.N`
/// the centre of the last drawing pass's hit region N (an element of the kit given the id N).
pub fn apphost_control_point(a: &App, name: &str) -> Option<(f32, f32)> {
    if !a.app_view.open {
        return None;
    }
    let b = app_body(a);
    if let Some(id) = name.strip_prefix("app.hit.") {
        let id: i32 = id.parse().ok()?;
        let r = a.app_view.runtime.as_ref()?.hit(id)?;
        return Some(((b.x + r.x + r.w / 2) as f32, (b.y + r.y + r.h / 2) as f32));
    }
    (name == "body").then(|| ((b.x + b.w / 2) as f32, (b.y + b.h / 2) as f32))
}

// ---- the runtime ----

impl AppRuntime {
    /// Loads an installed app's module and starts it, for a screen of `screen_w` by `screen_h` pixels: what a
    /// launch does, without the screen (the host's tests drive a module through this). The module's file is
    /// checked again before it is loaded (`app_package_artifact`: a plain file inside the app's folder, with the
    /// SHA-256 recorded at the install), once for each launch.
    pub fn open(id: &str, screen_w: i32, screen_h: i32, clock_12h: bool) -> Result<Self, String> {
        let (it, m, module) = crate::apppkg::app_package_artifact(id)?;
        Self::new(&it, &m, &module, screen_w, screen_h, clock_12h)
    }

    /// Compiles and instantiates an installed app's module, whose bytes `app_package_artifact` read.
    fn new(it: &WidgetInstall, m: &WidgetPackage, bytes: &[u8], screen_w: i32, screen_h: i32, clock_12h: bool) -> Result<Self, String> {
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, bytes).map_err(|e| format!("The app's WebAssembly is invalid: {e}"))?;
        if let Some(call) = kit_missing_import(&module) {
            // TRANSLATORS: a toast: the app uses a call of Grimoire's that this version does not have (%s is the
            // call's name, such as grimoire_kit.row).
            return Err(cformat!(tr("This app needs a newer Grimoire (%s)."), &call));
        }
        let state_path = format!("{}/{STATE_FILE}", crate::apppkg::app_package_dir(&it.id));
        let storage = m.capabilities.iter().any(|c| c == APP_CAP_STORAGE);
        // (a state file left by a version that declared the capability is not read, and stays as it is)
        let state = if storage { read_file_max(&state_path, APP_STATE_MAX).unwrap_or_default() } else { Vec::new() };
        let settings = crate::apppkg::app_settings_load(&it.id, &m.settings);
        // (the body from the start on: the screen under the status bar the manifest asks for; an app that hides or
        // shows the bar before its first pass has its body from that pass on)
        let bar = if m.spec.status_bar { gui_status_h().max(0) } else { 0 };
        let mut store = Store::new(&engine, HostState {
            id: it.id.clone(),
            storage,
            body: (screen_w, (screen_h - bar).max(0)),
            commands: Vec::new(),
            hits: Vec::new(),
            text_bytes: 0,
            state_bytes: 0,
            read_bytes: 0,
            state,
            state_path,
            settings,
            close_requested: false,
            status_bar: m.spec.status_bar,
            status: String::new(),
            texts: Vec::new(),
            toast: None,
            dirty: false,
            limits: crate::wasmhost::store_limits(MEMORY_MAX, APP_TABLE_MAX),
            redraw_at: None,
            screen_w,
            screen_h,
            clock_12h,
            network_http: m.capabilities.iter().any(|c| c == APP_CAP_NETWORK),
            http: None,
            points: Vec::new(),
            #[cfg(test)]
            http_stand_in: Vec::new(),
        });
        store.limiter(|st| &mut st.limits);
        // The start section runs here, before the app can draw: it has the budget of a call.
        let _ = store.set_fuel(APP_CALL_FUEL);
        let mut linker = Linker::new(&engine);
        install_imports(&mut linker)?;
        kit_link(&mut linker)?;
        let instance = linker.instantiate_and_start(&mut store, &module).map_err(|e| format!("The app could not start: {e}"))?;
        if instance.get_memory(&store, "memory").is_none() {
            return Err("The app has no memory export.".into());
        }
        Ok(AppRuntime {
            _engine: engine,
            store,
            instance,
            status: String::new(),
            texts: Vec::new(),
            error: None,
            used: 0,
            drawn: 0,
            hits: Vec::new(),
        })
    }

    /// Calls `app_draw` and renders its commands into `body`, and nowhere else on the canvas.
    pub fn draw(&mut self, c: &mut Canvas, body: Rect, elapsed_ms: i32) {
        if self.error.is_some() {
            return;
        }
        let draw = match self.instance.get_typed_func::<(i32, i32, i32, i32), ()>(&self.store, "app_draw") {
            Ok(f) => f,
            Err(e) => {
                self.error = Some(format!("The app has no app_draw export: {e}"));
                return;
            }
        };
        self.store.data_mut().status.clear();
        self.store.data_mut().texts.clear();
        // (a redraw request whose time has come is answered by this pass, whatever brought it)
        if self.redraw_due(now_ms()) {
            self.store.data_mut().redraw_at = None;
        }
        self.begin_call();
        self.store.data_mut().body = (body.w, body.h);
        let scale = ui_px(1000.0);
        let called = draw.call(&mut self.store, (body.w, body.h, scale, elapsed_ms));
        self.end_call();
        if let Err(e) = called {
            self.error = Some(format!("The app stopped: {e}"));
            return;
        }
        self.save();
        let commands = std::mem::take(&mut self.store.data_mut().commands);
        self.drawn = commands.len();
        let hits = std::mem::take(&mut self.store.data_mut().hits);
        self.hits = hits.into_iter().filter_map(|(id, r)| in_body(body, r).map(|r| (id, r))).collect();
        self.status = std::mem::take(&mut self.store.data_mut().status);
        self.texts = std::mem::take(&mut self.store.data_mut().texts);
        // Nothing below draws outside the body: the canvas is clipped to it (and to what the caller already
        // clipped to) until the commands are drawn, and a `clip` of the module's is a part of that.
        let saved_clip = c.clip;
        let in_body = rect_intersect(saved_clip, body);
        canvas_set_clip(c, in_body);
        canvas_fill(c, body, 255);
        // A module passes any numbers. Each rectangle's edges are held within a reach of the body
        // (`on_screen`), a place and a line's ends likewise, a thickness within THICK_REACH, a text's height
        // and a line's width within their limits (when they were recorded), and no icon is made larger than
        // APP_ICON_PX_MAX: no sum the drawing makes overflows, and nothing is drawn at a size without bound.
        let at = |r: Rect| on_screen(body, r.x, r.y, r.w, r.h);
        let texts = &mut self.texts;
        font_with_icon_side_max(icon_px_max(), || {
            for cmd in commands {
                match cmd {
                    DrawCmd::Fill(r, gray) => canvas_fill(c, at(r), gray),
                    DrawCmd::Outline(r, t, gray) => canvas_rect_outline(c, at(r), thick(t), gray),
                    DrawCmd::Round(r, radius, gray, filled, t) => canvas_round_rect(c, at(r), radius, gray, filled, thick(t)),
                    DrawCmd::Line(x0, y0, x1, y1, width, gray) => {
                        if let Some((x0, y0, x1, y1)) = line_in_reach(body, x0, y0, x1, y1) {
                            canvas_line_aa(c, body.x as f32 + x0, body.y as f32 + y0, body.x as f32 + x1, body.y as f32 + y1, width, gray)
                        }
                    }
                    DrawCmd::Text(r, text, px, face, center, gray) => {
                        ui_text(c, face, px, px, at(r), &text, 1, center, gray);
                        texts.push(text);
                    }
                    DrawCmd::TextAt(x, y, text, px, face, gray) => {
                        let p = at(Rect::new(x, y, 0, 0));
                        font_draw(c, face, px, p.x, p.y, &text, gray);
                        texts.push(text);
                    }
                    DrawCmd::Icon(name, r, gray) => ui_icon(c, widget_icon(&name), at(r), gray),
                    DrawCmd::Button(r, label, icon, style) => {
                        ui_button(c, at(r), if icon.is_empty() { IconKind::None } else { widget_icon(&icon) }, &label, button_style(style));
                        texts.push(label);
                    }
                    DrawCmd::Chip(r, label, selected) => {
                        gui_chip(c, at(r), &label, selected);
                        texts.push(label);
                    }
                    DrawCmd::Row(r, icon, label, style, switch) => {
                        let mut row = Row::new(if icon.is_empty() { IconKind::None } else { widget_icon(&icon) }, &label);
                        row.style = button_style(style);
                        if switch >= 0 {
                            row.switch = Some(switch != 0);
                        }
                        ui_row(c, at(r), &row);
                        texts.push(label);
                    }
                    DrawCmd::Surface(r, filled, t) => gui_surface(c, at(r), filled, thick(t)),
                    DrawCmd::Select(r, caption, value) => {
                        gui_select(c, at(r), &caption, &value);
                        texts.push(value);
                    }
                    DrawCmd::AppTile(r, icon, label, selected) => {
                        let icon = AppIcon::glyph(widget_icon(&icon));
                        gui_app_tile(c, at(r), &icon, &label, selected);
                        texts.push(label);
                    }
                    DrawCmd::Fab(r, icon) => gui_fab(c, at(r), widget_icon(&icon)),
                    DrawCmd::Card(r, style) => ui_card(c, at(r), button_style(style)),
                    DrawCmd::TextField(r, icon, text, hint) => {
                        gui_text_field(c, at(r), if icon.is_empty() { IconKind::None } else { widget_icon(&icon) }, &text, &hint);
                        texts.push(text);
                        texts.push(hint);
                    }
                    DrawCmd::Clip(r) => canvas_set_clip(c, rect_intersect(at(r), in_body)),
                    DrawCmd::ClipClear => canvas_set_clip(c, in_body),
                    DrawCmd::Kit(k) => kit_render(c, body, in_body, k, texts),
                }
            }
        });
        c.clip = saved_clip;
    }

    /// Calls `app_pointer`, when the module has one: a pointer phase at a place of the body. Whether it was called
    /// (a call says nothing of the picture, so it owes a drawing pass).
    pub fn pointer(&mut self, phase: i32, x: i32, y: i32, source: i32) -> bool {
        if self.error.is_some() {
            return false;
        }
        let Some(f) = self.instance.get_typed_func::<(i32, i32, i32, i32), ()>(&self.store, "app_pointer").ok() else { return false };
        self.begin_call();
        let called = f.call(&mut self.store, (phase, x, y, source));
        self.end_call();
        if called.is_err() {
            self.error = Some("The app stopped while handling a touch.".into());
            return true;
        }
        self.save();
        true
    }

    /// Calls `app_key`, when the module has one. Whether it was called.
    pub fn key(&mut self, code: i32, mods: i32, down: bool) -> bool {
        if self.error.is_some() {
            return false;
        }
        let Some(f) = self.instance.get_typed_func::<(i32, i32, i32), ()>(&self.store, "app_key").ok() else { return false };
        self.begin_call();
        let called = f.call(&mut self.store, (code, mods, i32::from(down)));
        self.end_call();
        if called.is_err() {
            self.error = Some("The app stopped while handling a key.".into());
            return true;
        }
        self.save();
        true
    }

    /// Calls `kit_event`, when the module has one, with the event's numbers; a DRAG_MOVE's points are what
    /// `event_points` copies during the call. None for a module without it; else whether its picture changed
    /// (what it returned was not 0). A call that stopped the module changed it: the app is closed with the reason.
    pub fn event(&mut self, ev: &KitEvent) -> Option<bool> {
        if self.error.is_some() {
            return None;
        }
        let f = self.instance.get_typed_func::<(i32, i32, i32, i32, i32), i32>(&self.store, KIT_EVENT).ok()?;
        self.begin_call();
        self.store.data_mut().points = ev.points.clone();
        let called = f.call(&mut self.store, (ev.kind, ev.id, ev.a, ev.b, ev.c));
        self.store.data_mut().points = Vec::new();
        self.end_call();
        match called {
            Ok(v) => {
                self.save();
                Some(v != 0)
            }
            Err(_) => {
                self.error = Some("The app stopped while handling an event.".into());
                Some(true)
            }
        }
    }

    /// The hit region of the last drawing pass under a place of the body (the first drawn that holds it), -1 for
    /// none.
    pub fn hit_at(&self, x: i32, y: i32) -> i32 {
        self.hits.iter().find(|(_, r)| rect_contains(*r, x, y)).map_or(-1, |(id, _)| *id)
    }

    fn export_status_bar(&mut self) -> Option<bool> {
        let f = self.instance.get_typed_func::<(), i32>(&self.store, "app_status_bar").ok()?;
        self.begin_call();
        let called = f.call(&mut self.store, ());
        self.end_call();
        called.ok().map(|v| v != 0)
    }

    /// Calls `app_stop`, when the module has one and has not stopped with an error, and saves its state.
    pub fn stop(&mut self) {
        if self.error.is_none() {
            if let Ok(f) = self.instance.get_typed_func::<(), ()>(&self.store, "app_stop") {
                self.begin_call();
                let _ = f.call(&mut self.store, ());
                self.end_call();
            }
        }
        self.save();
    }

    /// Starts a call of the module: its instruction budget, and nothing recorded and nothing counted yet. What
    /// an earlier call left recorded (a touch that drew) is dropped: a call holds its own commands only, so
    /// its limits are the most the host ever keeps.
    fn begin_call(&mut self) {
        let st = self.store.data_mut();
        st.commands.clear();
        st.hits.clear();
        st.text_bytes = 0;
        st.state_bytes = 0;
        st.read_bytes = 0;
        let _ = self.store.set_fuel(APP_CALL_FUEL);
    }

    /// After a call: what it took from its budget, whether it ended or was stopped.
    fn end_call(&mut self) {
        self.used = APP_CALL_FUEL.saturating_sub(self.store.get_fuel().unwrap_or(0));
    }

    /// What the last call of the module took from its budget: its own instructions and the charges for the
    /// host's work. At most APP_CALL_FUEL, which a call that was stopped for it has used up.
    pub fn fuel_used(&self) -> u64 {
        self.used
    }

    /// The drawing commands the last drawing pass recorded and the host drew. At most APP_CALL_COMMANDS_MAX.
    pub fn commands(&self) -> usize {
        self.drawn
    }

    /// The bytes the host read from the module's memory in the last call, all its imports together. Never more
    /// than the limits let an import read, whatever lengths the module passed.
    pub fn bytes_read(&self) -> usize {
        self.store.data().read_bytes
    }

    fn tick(&mut self) {
        // A request in flight is looked after whether or not the app polls it, or still runs.
        self.store.data_mut().http_watch();
        if self.error.is_some() {
            return;
        }
        // The host has no autonomous app callbacks yet; redraw requests are polled by AppRun.
    }

    /// Writes the state file after a call that changed the state. An app without `storage:app-local` has no
    /// state to change, and no file is made for it.
    fn save(&mut self) {
        let st = self.store.data_mut();
        if !st.storage || !st.dirty {
            return;
        }
        st.dirty = false;
        if let Some(dir) = st.state_path.rsplit_once('/').map(|(d, _)| d) {
            let _ = mkdir_p(dir);
        }
        if write_file_atomic(&st.state_path, &st.state) != 0 {
            st.dirty = true;
        }
    }

    /// Whether the module asked for a drawing pass that is due at `now`, by the app's clock (`now_ms`: the time
    /// since it asked, counted across the clock's wrap, wasmhost.rs Redraw). Nothing is due for a module that has
    /// stopped.
    pub fn redraw_due(&self, now: u32) -> bool {
        self.error.is_none() && self.store.data().redraw_at.is_some_and(|r| r.due(now))
    }

    /// `redraw_due`, and the request is taken: the host's tick asks this once and then draws the app, so one
    /// request brings one drawing pass and one refresh, whatever the pass does.
    pub fn redraw_taken(&mut self, now: u32) -> bool {
        let due = self.redraw_due(now);
        if due {
            self.store.data_mut().redraw_at = None;
        }
        due
    }

    /// What the app shows in place of going on, once a call of its module has failed.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The status line of the last drawing pass, for tests and a debug view.
    pub fn status(&self) -> &str {
        &self.status
    }

    /// The text drawn in the last drawing pass, joined by `|`, for generic scenario probes.
    pub fn text(&self) -> String {
        self.texts.join("|")
    }

    /// The last drawing pass's hit region `id` (body-local), for the runner's taps by name.
    pub fn hit(&self, id: i32) -> Option<Rect> {
        self.hits.iter().find(|(hit, _)| *hit == id).map(|(_, r)| *r)
    }

    /// The hit regions of the last drawing pass: the parts inside its body, at most APP_CALL_HITS_MAX.
    pub fn hits(&self) -> &[(i32, Rect)] {
        &self.hits
    }
}

#[cfg(test)]
thread_local! {
    /// Tests: the host's own greatest sizes of a text and an icon are set aside (`text_px_max`, `icon_px_max`), so
    /// that a module's sizes reach the font as given and meet the font's own greatest glyph.
    static OWN_SIZES_ASIDE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The height a text is drawn at most, in pixels: APP_TEXT_PX_MAX.
fn text_px_max() -> i32 {
    #[cfg(test)]
    if OWN_SIZES_ASIDE.get() {
        return i32::MAX / 1000;
    }
    APP_TEXT_PX_MAX
}

/// The side an icon is drawn at most, in pixels: APP_ICON_PX_MAX.
fn icon_px_max() -> i32 {
    #[cfg(test)]
    if OWN_SIZES_ASIDE.get() {
        return i32::MAX;
    }
    APP_ICON_PX_MAX
}

/// The height a text is drawn and measured at, in pixels, from the thousandths a module passes: from one
/// thousandth to APP_TEXT_PX_MAX, a value outside taken as the nearer end.
fn text_px(px_milli: i32) -> f32 {
    px_milli.clamp(1, text_px_max() * 1000) as f32 / 1000.0
}

fn button_style(v: i32) -> ButtonStyle {
    match v {
        1 => ButtonStyle::Selected,
        2 => ButtonStyle::Disabled,
        3 => ButtonStyle::Primary,
        _ => ButtonStyle::Plain,
    }
}

fn face(v: i32) -> FontFace {
    match v {
        1 => FontFace::Bold,
        2 => FontFace::Title,
        3 => FontFace::Caps,
        4 => FontFace::Fixed,
        5 => FontFace::FixedBold,
        _ => FontFace::Regular,
    }
}

fn install_imports(linker: &mut Linker<HostState>) -> Result<(), String> {
    let e = |x: wasmi::errors::LinkerError| x.to_string();

    // Every import pays the fixed charge first, whatever it is given and whether or not the app has the
    // capability for it (`called`; a drawing call also needs room for one more command: `drawing`), and then for
    // its work where that grows with what it is given: `pay_shape`, `pay_icon`, `call_text` and the bytes moved.

    // ---- drawing ----
    // (each records one command, once the call has paid for drawing it)
    linker.func_wrap(ABI, "fill_rect", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, gray: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        pay_shape(&mut caller, x, y, w, h, 0, 1)?;
        caller.data_mut().commands.push(DrawCmd::Fill(Rect::new(x, y, w, h), gray.clamp(0, 255) as u8));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "outline_rect", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, t: i32, gray: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        // (an outline is four bands of rows: no more of them than the rectangle has)
        pay_shape(&mut caller, x, y, w, h, 0, 1)?;
        caller.data_mut().commands.push(DrawCmd::Outline(Rect::new(x, y, w, h), t, gray.clamp(0, 255) as u8));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "round_rect", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, radius: i32, gray: i32, filled: i32, t: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        // (the pixels within its corners' radius, or its outline's thickness, of its sides are worked out one by one)
        let r = drawn(caller.data(), x, y, w, h);
        let edge = radius.clamp(0, (r.w.min(r.h) / 2).max(0)).max(thick(t)) as i64 + 2;
        pay_shape(&mut caller, x, y, w, h, edge, 1)?;
        caller.data_mut().commands.push(DrawCmd::Round(Rect::new(x, y, w, h), radius, gray.clamp(0, 255) as u8, filled != 0, t));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "line", |mut caller: Caller<'_, HostState>, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, gray: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        let width = width.max(0.1).min(APP_LINE_WIDTH_MAX);
        let n = line_charge(caller.data(), x0, y0, x1, y1, width);
        charge(&mut caller, n)?;
        caller.data_mut().commands.push(DrawCmd::Line(x0, y0, x1, y1, width, gray.clamp(0, 255) as u8));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "text", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, ptr: i32, len: i32, px_milli: i32, face_: i32, center: i32, gray: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        let px = text_px(px_milli);
        let text = call_text(&mut caller, ptr, len, |n| drawn_text_charge(n, px, face(face_)))?;
        // (a text wider than its room is cut short, by steps that each measure it again)
        if font_measure(face(face_), px, &text) > drawn(caller.data(), x, y, w, h).w {
            charge(&mut caller, cut_text_charge(len))?;
        }
        caller.data_mut().commands.push(DrawCmd::Text(Rect::new(x, y, w, h), text, px, face(face_), center != 0, gray.clamp(0, 255) as u8));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "text_at", |mut caller: Caller<'_, HostState>, x: i32, baseline: i32, ptr: i32, len: i32, px_milli: i32, face_: i32, gray: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        let px = text_px(px_milli);
        let text = call_text(&mut caller, ptr, len, |n| drawn_text_charge(n, px, face(face_)))?;
        caller.data_mut().commands.push(DrawCmd::TextAt(x, baseline, text, px, face(face_), gray.clamp(0, 255) as u8));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "text_width", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32, px_milli: i32, face_: i32| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        let text = call_text(&mut caller, ptr, len, |n| n as u64 * APP_CHARGE_MEASURE)?;
        Ok(font_measure(face(face_), text_px(px_milli), &text))
    }).map_err(e)?;
    linker.func_wrap(ABI, "text_height", |mut caller: Caller<'_, HostState>, px_milli: i32, face_: i32| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        let px = text_px(px_milli);
        Ok(font_ascent(face(face_), px) + font_descent(face(face_), px))
    }).map_err(e)?;
    linker.func_wrap(ABI, "icon", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32, x: i32, y: i32, w: i32, h: i32, gray: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        let name = icon_name(&mut caller, ptr, len)?;
        pay_icon(&mut caller, x, y, w, h, 1)?;
        caller.data_mut().commands.push(DrawCmd::Icon(name, Rect::new(x, y, w, h), gray.clamp(0, 255) as u8));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "button", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, label_ptr: i32, label_len: i32, icon_ptr: i32, icon_len: i32, style: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        // (its fill, its outline and its inner line; a label alone may take two lines)
        pay_shape(&mut caller, x, y, w, h, APP_CHARGE_CORNER, 3)?;
        let label = call_text(&mut caller, label_ptr, label_len, |n| label_charge(n, true))?;
        let icon = icon_name(&mut caller, icon_ptr, icon_len)?;
        if !icon.is_empty() {
            pay_icon(&mut caller, x, y, w, h, 1)?;
        }
        caller.data_mut().commands.push(DrawCmd::Button(Rect::new(x, y, w, h), label, icon, style));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "chip", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, ptr: i32, len: i32, selected: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        let edge = round_end(caller.data(), x, y, w, h);
        pay_shape(&mut caller, x, y, w, h, edge, 2)?;
        let label = call_text(&mut caller, ptr, len, |n| label_charge(n, false))?;
        caller.data_mut().commands.push(DrawCmd::Chip(Rect::new(x, y, w, h), label, selected != 0));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "row", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, icon_ptr: i32, icon_len: i32, label_ptr: i32, label_len: i32, style: i32, switch: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        // (its card; its icon and its switch are of one size whatever the row's; its label may take two lines)
        pay_shape(&mut caller, x, y, w, h, APP_CHARGE_CORNER, 1)?;
        let icon = icon_name(&mut caller, icon_ptr, icon_len)?;
        let small = if icon.is_empty() { 0 } else { icon_charge(APP_CHARGE_SMALL_ICON_PX) } + if switch >= 0 { APP_CHARGE_SWITCH } else { 0 };
        charge(&mut caller, small)?;
        let label = call_text(&mut caller, label_ptr, label_len, |n| label_charge(n, true))?;
        caller.data_mut().commands.push(DrawCmd::Row(Rect::new(x, y, w, h), icon, label, style, switch));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "surface", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, filled: i32, t: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        // (its fill, and its outline or its inner line: the outline is as thick as the module says)
        pay_shape(&mut caller, x, y, w, h, APP_CHARGE_CORNER, 1)?;
        pay_shape(&mut caller, x, y, w, h, APP_CHARGE_CORNER + if filled != 0 { 0 } else { thick(t).max(0) as i64 }, 1)?;
        caller.data_mut().commands.push(DrawCmd::Surface(Rect::new(x, y, w, h), filled != 0, t));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "select", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, cap_ptr: i32, cap_len: i32, val_ptr: i32, val_len: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        let edge = round_end(caller.data(), x, y, w, h);
        pay_shape(&mut caller, x, y, w, h, edge, 2)?;
        // (its chevron is of one size)
        charge(&mut caller, icon_charge(APP_CHARGE_SMALL_ICON_PX))?;
        let caption = call_text(&mut caller, cap_ptr, cap_len, |n| label_charge(n, false))?;
        let value = call_text(&mut caller, val_ptr, val_len, |n| label_charge(n, false))?;
        caller.data_mut().commands.push(DrawCmd::Select(Rect::new(x, y, w, h), caption, value));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "app_tile", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, icon_ptr: i32, icon_len: i32, label_ptr: i32, label_len: i32, selected: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        pay_shape(&mut caller, x, y, w, h, APP_CHARGE_CORNER, 2)?;
        let icon = icon_name(&mut caller, icon_ptr, icon_len)?;
        pay_icon(&mut caller, x, y, w, h, 1)?;
        let label = call_text(&mut caller, label_ptr, label_len, |n| label_charge(n, false))?;
        caller.data_mut().commands.push(DrawCmd::AppTile(Rect::new(x, y, w, h), icon, label, selected != 0));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "fab", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, icon_ptr: i32, icon_len: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        let edge = round_end(caller.data(), x, y, w, h);
        pay_shape(&mut caller, x, y, w, h, edge, 1)?;
        let icon = icon_name(&mut caller, icon_ptr, icon_len)?;
        // (its icon is under half its size)
        pay_icon(&mut caller, x, y, w, h, 2)?;
        caller.data_mut().commands.push(DrawCmd::Fab(Rect::new(x, y, w, h), icon));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "card", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, style: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        pay_shape(&mut caller, x, y, w, h, APP_CHARGE_CORNER, 1)?;
        caller.data_mut().commands.push(DrawCmd::Card(Rect::new(x, y, w, h), style));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "text_field", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, icon_ptr: i32, icon_len: i32, text_ptr: i32, text_len: i32, hint_ptr: i32, hint_len: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        let edge = round_end(caller.data(), x, y, w, h);
        pay_shape(&mut caller, x, y, w, h, edge, 2)?;
        let icon = icon_name(&mut caller, icon_ptr, icon_len)?;
        if !icon.is_empty() {
            // (its icon is as tall as the field, whatever the field's width, and never smaller than the kit's
            // small icons: globalui.rs, gui_text_field)
            let tall = drawn(caller.data(), x, y, w, h).h as i64;
            charge(&mut caller, icon_charge(tall.max(APP_CHARGE_SMALL_ICON_PX)))?;
        }
        // (the mark that clears a field with text in it is of one size)
        charge(&mut caller, icon_charge(APP_CHARGE_SMALL_ICON_PX))?;
        let text = call_text(&mut caller, text_ptr, text_len, |n| label_charge(n, false))?;
        let hint = call_text(&mut caller, hint_ptr, hint_len, |n| label_charge(n, false))?;
        caller.data_mut().commands.push(DrawCmd::TextField(Rect::new(x, y, w, h), icon, text, hint));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "clip", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        caller.data_mut().commands.push(DrawCmd::Clip(Rect::new(x, y, w, h)));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "clip_clear", |mut caller: Caller<'_, HostState>| -> Result<(), wasmi::Error> {
        drawing(&mut caller)?;
        caller.data_mut().commands.push(DrawCmd::ClipClear);
        Ok(())
    }).map_err(e)?;

    // ---- state, settings, system ----
    // (state_get and state_set need `storage:app-local`: without it the one gives nothing and the other keeps
    // nothing)
    linker.func_wrap(ABI, "state_get", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        if !caller.data().storage {
            return Ok(0);
        }
        let n = caller.data().state.len().min(cap.max(0) as usize);
        state_moved(caller.data_mut(), n)?;
        charge(&mut caller, copy_charge(n))?;
        let state = caller.data().state[..n].to_vec();
        Ok(write_mem(&mut caller, ptr, cap, &state))
    }).map_err(e)?;
    linker.func_wrap(ABI, "state_set", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<(), wasmi::Error> {
        called(&mut caller)?;
        if !caller.data().storage {
            return Ok(());
        }
        // (the first APP_STATE_MAX bytes are the state: no more than those are read)
        let n = (len.max(0) as usize).min(APP_STATE_MAX);
        state_moved(caller.data_mut(), n)?;
        charge(&mut caller, copy_charge(n))?;
        let state = read_mem(&mut caller, ptr, n as i32);
        let st = caller.data_mut();
        st.state = state;
        st.dirty = true;
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "setting", |mut caller: Caller<'_, HostState>, key_ptr: i32, key_len: i32, out_ptr: i32, out_cap: i32| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        // (no setting has a longer key: nothing is read for one)
        if key_len.max(0) as usize > crate::widgetpkg::WIDGET_SETTINGS_KEY_MAX {
            return Ok(0);
        }
        charge(&mut caller, key_len.max(0) as u64 * APP_CHARGE_BYTE)?;
        let key = read_mem(&mut caller, key_ptr, key_len);
        let value = caller.data().settings.iter().find(|(k, _)| k.as_bytes() == key.as_slice()).map(|(_, v)| v.as_bytes().to_vec()).unwrap_or_default();
        charge(&mut caller, value.len().min(out_cap.max(0) as usize) as u64 * APP_CHARGE_BYTE)?;
        Ok(write_mem(&mut caller, out_ptr, out_cap, &value))
    }).map_err(e)?;
    linker.func_wrap(ABI, "status", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<(), wasmi::Error> {
        called(&mut caller)?;
        if len.max(0) as usize > APP_STATUS_MAX {
            return Err(past(caller.data(), "the bytes of the status line", APP_STATUS_MAX));
        }
        charge(&mut caller, len.max(0) as u64 * APP_CHARGE_BYTE)?;
        caller.data_mut().status = String::from_utf8_lossy(&read_mem(&mut caller, ptr, len)).into_owned();
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "request_redraw", |mut caller: Caller<'_, HostState>, after_ms: i32| -> Result<(), wasmi::Error> {
        called(&mut caller)?;
        let after = after_ms.clamp(APP_REDRAW_MIN_MS, APP_REDRAW_MAX_MS) as u32;
        let st = caller.data_mut();
        // (one request stands, the soonest asked for: a later call does not put an earlier one off)
        st.redraw_at = Some(Redraw::asked(st.redraw_at, now_ms(), after));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "close", |mut caller: Caller<'_, HostState>| -> Result<(), wasmi::Error> {
        called(&mut caller)?;
        caller.data_mut().close_requested = true;
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "show_status_bar", |mut caller: Caller<'_, HostState>, on: i32| -> Result<(), wasmi::Error> {
        called(&mut caller)?;
        caller.data_mut().status_bar = on != 0;
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "toast", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32, ms: i32| -> Result<(), wasmi::Error> {
        called(&mut caller)?;
        // (a toast shows the first 255 bytes of its text: no more than APP_TOAST_MAX are read)
        let n = len.clamp(0, APP_TOAST_MAX as i32);
        charge(&mut caller, n as u64 * APP_CHARGE_BYTE)?;
        let text = String::from_utf8_lossy(&read_mem(&mut caller, ptr, n)).into_owned();
        caller.data_mut().toast = Some((text, ms.clamp(500, 30_000) as u32));
        Ok(())
    }).map_err(e)?;
    linker.func_wrap(ABI, "now", |mut caller: Caller<'_, HostState>| -> Result<i64, wasmi::Error> {
        called(&mut caller)?;
        Ok(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64))
    }).map_err(e)?;
    linker.func_wrap(ABI, "now_ms", |mut caller: Caller<'_, HostState>| -> Result<i64, wasmi::Error> {
        called(&mut caller)?;
        Ok(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64))
    }).map_err(e)?;
    // (the calls below that ask the system for something pay APP_CHARGE_SYSTEM beside the fixed charge: the
    // time and the date as text, the battery's state, and a request's state)
    linker.func_wrap(ABI, "time_text", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> Result<i32, wasmi::Error> {
        asked_of_the_system(&mut caller)?;
        let mut s = String::new();
        format_clock_short(wall_time(), caller.data().clock_12h, &mut s, 64);
        Ok(write_mem(&mut caller, ptr, cap, s.as_bytes()))
    }).map_err(e)?;
    linker.func_wrap(ABI, "date_text", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> Result<i32, wasmi::Error> {
        asked_of_the_system(&mut caller)?;
        let tm = local_tm(wall_time());
        let s = format!("{:04}-{:02}-{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday);
        Ok(write_mem(&mut caller, ptr, cap, s.as_bytes()))
    }).map_err(e)?;
    linker.func_wrap(ABI, "battery_percent", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        asked_of_the_system(&mut caller)?;
        Ok(battery_from_sysfs("/sys/class/power_supply").map_or(-1, |b| b.percent))
    }).map_err(e)?;
    linker.func_wrap(ABI, "battery_charging", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        asked_of_the_system(&mut caller)?;
        Ok(battery_from_sysfs("/sys/class/power_supply").is_some_and(|b| b.charging) as i32)
    }).map_err(e)?;
    linker.func_wrap(ABI, "wifi_connected", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        Ok(wifi_status().and_then(|s| s.connected).map_or(-1, i32::from))
    }).map_err(e)?;
    linker.func_wrap(ABI, "wifi_ssid", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        let mut name = wifi_current().unwrap_or_default();
        name.truncate(APP_SSID_MAX);
        Ok(write_mem(&mut caller, ptr, cap, &name))
    }).map_err(e)?;
    linker.func_wrap(ABI, "screen_width", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        Ok(caller.data().screen_w)
    }).map_err(e)?;
    linker.func_wrap(ABI, "screen_height", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        Ok(caller.data().screen_h)
    }).map_err(e)?;
    linker.func_wrap(ABI, "ui_scale_milli", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        Ok(ui_px(1000.0))
    }).map_err(e)?;
    linker.func_wrap(ABI, "theme", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        Ok(matches!(ui_theme(), UiTheme::Classic) as i32)
    }).map_err(e)?;
    linker.func_wrap(ABI, "status_bar_height", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        Ok(gui_status_h())
    }).map_err(e)?;

    // ---- HTTP (network:http) ----
    // An app starts one request and polls it. The request runs in a child curl while the app keeps
    // drawing; `request_redraw` is how it asks to be called again.
    linker.func_wrap(ABI, "http_start", |mut caller: Caller<'_, HostState>, method_ptr: i32, method_len: i32, url_ptr: i32, url_len: i32, headers_ptr: i32, headers_len: i32, body_ptr: i32, body_len: i32| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        // Nothing of the request is read from the module's memory before the capability, the
        // request in flight and the four lengths have been checked.
        let lengths = [method_len, url_len, headers_len, body_len].map(|len| len.max(0) as usize);
        let before = caller.data().http_start_before_read(lengths);
        if before != 0 {
            return Ok(before);
        }
        // (a request that is read is paid for as one that starts: its bytes, and the start of a curl)
        charge(&mut caller, APP_CHARGE_REQUEST + copy_charge(lengths.iter().sum()))?;
        let method = String::from_utf8_lossy(&read_mem(&mut caller, method_ptr, method_len)).into_owned();
        let url = String::from_utf8_lossy(&read_mem(&mut caller, url_ptr, url_len)).into_owned();
        let headers = String::from_utf8_lossy(&read_mem(&mut caller, headers_ptr, headers_len)).into_owned();
        let body = read_mem(&mut caller, body_ptr, body_len);
        Ok(caller.data_mut().http_start(&method, &url, &headers, &body))
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_poll", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        asked_of_the_system(&mut caller)?;
        Ok(caller.data_mut().http_poll())
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_status", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        Ok(caller.data().http.as_ref().map_or(-1, |j| j.code))
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_body_len", |mut caller: Caller<'_, HostState>| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        Ok(caller.data().http.as_ref().map_or(0, |j| j.body.len() as i32))
    }).map_err(e)?;
    // The answer and the error text are written from where the host holds them: each is lent to
    // the write and put back, and is not copied for the call.
    linker.func_wrap(ABI, "http_body", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        let held = caller.data().http.as_ref().map_or(0, |j| j.body.len());
        charge(&mut caller, copy_charge(held.min(cap.max(0) as usize)))?;
        let Some(body) = caller.data_mut().http.as_mut().map(|j| std::mem::take(&mut j.body)) else { return Ok(0) };
        let n = write_mem(&mut caller, ptr, cap, &body);
        if let Some(j) = caller.data_mut().http.as_mut() {
            j.body = body;
        }
        Ok(n)
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_error", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> Result<i32, wasmi::Error> {
        called(&mut caller)?;
        let held = caller.data().http.as_ref().map_or(0, |j| j.error.len());
        charge(&mut caller, copy_charge(held.min(cap.max(0) as usize)))?;
        let Some(text) = caller.data_mut().http.as_mut().map(|j| std::mem::take(&mut j.error)) else { return Ok(0) };
        let n = write_mem(&mut caller, ptr, cap, text.as_bytes());
        if let Some(j) = caller.data_mut().http.as_mut() {
            j.error = text;
        }
        Ok(n)
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_cancel", |mut caller: Caller<'_, HostState>| -> Result<(), wasmi::Error> {
        called(&mut caller)?;
        caller.data_mut().http_cancel();
        Ok(())
    }).map_err(e)?;
    Ok(())
}

/// True for the address of a request: it begins with `http://` or `https://`, is at most
/// HTTP_ADDRESS_MAX bytes, and has no control character and no space.
fn http_address_ok(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() <= HTTP_ADDRESS_MAX
        && !url.chars().any(|c| c.is_control() || c == ' ')
}

/// True for a header line of a request: a name, a colon and a value. The name is one or more
/// HTTP token characters (RFC 9110's tchar) and begins the line, the colon follows it directly,
/// the value has no control character, and the line is at most HTTP_HEADER_LINE_MAX bytes.
fn http_header_line_ok(line: &str) -> bool {
    let Some((name, value)) = line.split_once(':') else { return false };
    line.len() <= HTTP_HEADER_LINE_MAX
        && !name.is_empty()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        && !value.chars().any(char::is_control)
}

/// The header lines of a request, one per line of `headers`, without the white space around
/// each; an empty line is skipped. None when a line is not a header line or there are more than
/// HTTP_HEADER_LINES_MAX of them: the request is then refused whole.
fn http_header_lines(headers: &str) -> Option<Vec<&str>> {
    let mut lines = Vec::new();
    for line in headers.lines() {
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        if lines.len() == HTTP_HEADER_LINES_MAX || !http_header_line_ok(line) {
            return None;
        }
        lines.push(line);
    }
    Some(lines)
}

/// The arguments curl is started with for one request. `-q` is first, so no configuration file
/// is read. The address and every redirect are http or https, at most HTTP_REDIRECTS_MAX
/// redirects are followed, the address is taken as written (`-g`), and the answer is at most
/// HTTP_ANSWER_MAX bytes. The default User-Agent is `-A`'s, so a `User-Agent` line of the request
/// takes its place and is not sent beside it. Each header line is one `-H`; the address is last.
///
/// A `HEAD` request is curl's own (`--head`), not a method put in another request's place: curl
/// then expects no body behind an answer's header lines, and writes those lines as the answer.
/// The length such an answer states is not its own, so `--max-filesize` is not given: the
/// answer file's other limits (`http_child_rules`, `http_watch`) hold as for any request.
fn http_args(method: &str, url: &str, header_lines: &[&str], body_path: &str, request_path: Option<&str>) -> Vec<String> {
    let redirects = HTTP_REDIRECTS_MAX.to_string();
    let answer = HTTP_ANSWER_MAX.to_string();
    let mut args: Vec<String> = [
        "-q", "-sS", "-g",
        "--proto", "=http,https", "--proto-redir", "=http,https",
        "-L", "--max-redirs", &redirects,
        "--max-time", "30",
    ].map(String::from).to_vec();
    if method == "HEAD" {
        args.push("--head".to_string());
    } else {
        args.extend(["--max-filesize", &answer, "-X", method].map(String::from));
    }
    args.extend(["-o", body_path, "-w", "%{http_code}", "-A", HTTP_USER_AGENT].map(String::from));
    for line in header_lines {
        args.extend(["-H".to_string(), line.to_string()]);
    }
    if let Some(request) = request_path {
        args.extend(["--data-binary".to_string(), format!("@{request}")]);
    }
    args.push(url.to_string());
    args
}

/// Takes the proxy variables out of the environment the child of `cmd` starts with. The rest of
/// Grimoire's environment is passed on as it is.
fn http_no_proxy(cmd: &mut Command) {
    for name in HTTP_PROXY_VARIABLES {
        cmd.env_remove(name);
    }
}

/// The rules a request's child starts under, set in the child between the fork and curl's start,
/// as the tree's other children carry them (wifi.rs, sandbox.rs, childcaps.rs). Everything it uses
/// was worked out before the fork: here there are only system calls, no allocation and no lock.
///
/// - None of Grimoire's descriptors: 0, 1 and 2 are the request's own by now, and every other one
///   closes when curl starts.
/// - curl ends when Grimoire does: the signal is asked for, then Grimoire is looked for, as it may
///   have ended before the signal was asked for.
/// - A background child's caps (`caps`, and `oom` its `oom_score_adj` as text): best effort, and
///   of the address space the soft limit only, never above the hard one nor over a lower one
///   already set, as child_caps_apply sets them.
/// - No file curl writes is longer than the answer's limit and one byte, and a write past that is
///   an error to curl, not a signal. `http_watch` reads an answer file of that length as too
///   large.
///
/// An error here keeps curl from starting.
fn http_child_rules(parent: libc::pid_t, caps: ChildCaps, oom: &[u8]) -> std::io::Result<()> {
    // SAFETY: fcntl, prctl, getppid, open, write, close, getrlimit, setrlimit and signal are system
    // calls here: they take plain numbers, a path that ends with its NUL, `oom`'s bytes and the
    // rlimits this function owns.
    unsafe {
        for fd in 3..1024 {
            let flags = libc::fcntl(fd, libc::F_GETFD);
            if flags >= 0 {
                libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
            }
        }
        #[cfg(target_os = "linux")]
        if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL as libc::c_ulong, 0, 0, 0) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        if libc::getppid() != parent {
            return Err(std::io::Error::from_raw_os_error(libc::ESRCH));
        }
        #[cfg(target_os = "linux")]
        {
            let fd = libc::open(c"/proc/self/oom_score_adj".as_ptr(), libc::O_WRONLY | libc::O_CLOEXEC);
            if fd >= 0 {
                libc::write(fd, oom.as_ptr().cast(), oom.len());
                libc::close(fd);
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = oom; // (only Linux has the file)
        if let Some(cap) = caps.as_bytes {
            let cap = cap as libc::rlim_t;
            let mut was = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            if libc::getrlimit(libc::RLIMIT_AS, &mut was) == 0 {
                let lower = was.rlim_cur != libc::RLIM_INFINITY && was.rlim_cur <= cap;
                let over = was.rlim_max != libc::RLIM_INFINITY && cap > was.rlim_max;
                if !lower && !over {
                    libc::setrlimit(libc::RLIMIT_AS, &libc::rlimit { rlim_cur: cap, rlim_max: was.rlim_max });
                }
            }
        }
        libc::signal(libc::SIGXFSZ, libc::SIG_IGN);
        // (a lower limit already set stays as it is)
        let most = (HTTP_ANSWER_MAX + 1) as libc::rlim_t;
        let mut size = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        if libc::getrlimit(libc::RLIMIT_FSIZE, &mut size) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        size = libc::rlimit { rlim_cur: size.rlim_cur.min(most), rlim_max: size.rlim_max.min(most) };
        if libc::setrlimit(libc::RLIMIT_FSIZE, &size) != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

impl HostState {
    /// What `http_start` answers from a request's four lengths alone, in bytes (the method's text,
    /// the address, the header text and the upload), before anything of the request is read: -2
    /// without the capability, -3 while a request is in flight, -1 when a length is over its limit,
    /// and 0 when the request may be read.
    fn http_start_before_read(&self, [method, url, headers, body]: [usize; 4]) -> i32 {
        if !self.network_http {
            return -2;
        }
        if self.http.as_ref().is_some_and(|j| !j.done) {
            return -3;
        }
        if method > HTTP_METHOD_TEXT_MAX || url > HTTP_ADDRESS_MAX || headers > HTTP_HEADERS_MAX || body > HTTP_UPLOAD_MAX {
            return -1;
        }
        0
    }

    fn http_start(&mut self, method: &str, url: &str, headers: &str, body: &[u8]) -> i32 {
        let before = self.http_start_before_read([method.len(), url.len(), headers.len(), body.len()]);
        if before != 0 {
            return before;
        }
        let method = method.trim().to_ascii_uppercase();
        if method.is_empty() || method.len() > HTTP_METHOD_MAX || !method.bytes().all(|b| b.is_ascii_uppercase()) || !http_address_ok(url) {
            return -1;
        }
        // (a HEAD request has no upload)
        if method == "HEAD" && !body.is_empty() {
            return -1;
        }
        let Some(header_lines) = http_header_lines(headers) else { return -1 };
        // (an ended request stays as it is until this one has started: a start that fails from
        // here on leaves its outcome readable, as a refused one does)
        let Some(dir) = self.state_path.rsplit_once('/').map(|(d, _)| d) else { return -1 };
        let folder = format!("{dir}/http");
        // The request's files are made in the app's state folder and its `http` folder, which are
        // real folders: with a link at either name the start fails, and makes and removes nothing.
        if !http_folders(dir, &folder) {
            return -1;
        }
        // No request of this host is in flight now: what an earlier run left in the folder goes.
        http_sweep(&folder);
        let tick = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        // (from here every return drops `files`, or the job that holds it: the files go with it)
        let files = HttpFiles { base: format!("{folder}/{}-{tick}", std::process::id()) };
        let request_path = files.path("request");
        if !body.is_empty() && std::fs::write(&request_path, body).is_err() {
            return -1;
        }
        let Ok(status_file) = std::fs::File::create(files.path("status")) else { return -1 };
        let Ok(err_file) = std::fs::File::create(files.path("err")) else { return -1 };
        let args = http_args(&method, url, &header_lines, &files.path("body"), (!body.is_empty()).then_some(request_path.as_str()));
        let mut cmd = Command::new("curl");
        #[cfg(test)]
        if let Some((program, first)) = self.http_stand_in.split_first() {
            cmd = Command::new(program);
            cmd.args(first);
        }
        cmd.args(&args).stdin(std::process::Stdio::null()).stdout(status_file).stderr(err_file);
        http_no_proxy(&mut cmd);
        let caps = child_caps_plan(child_vm_base(child_vm_bytes()), HTTP_CHILD_MARGIN);
        let oom = caps.oom_adj.to_string();
        // SAFETY: getpid has no preconditions.
        let parent = unsafe { libc::getpid() };
        // SAFETY: the closure runs in the child, between the fork and curl's start, and makes only
        // system calls (http_child_rules) with what was made here, before the fork.
        unsafe {
            cmd.pre_exec(move || http_child_rules(parent, caps, oom.as_bytes()));
        }
        match cmd.spawn() {
            Ok(child) => {
                self.http = Some(HttpJob {
                    child,
                    files,
                    done: false,
                    code: -1,
                    body: Vec::new(),
                    error: String::new(),
                });
                0
            }
            Err(_) => -1,
        }
    }

    /// Looks after the request in flight: reaps a curl that has ended, reads its outcome once and
    /// removes its files, and ends a curl whose answer file has grown past the limit. The host's
    /// tick calls this, so a request the app never polls leaves no child and no file behind, and
    /// `http_poll` calls it before it answers.
    fn http_watch(&mut self) {
        let Some(job) = self.http.as_mut() else { return };
        if job.done {
            return;
        }
        match job.child.try_wait() {
            Ok(Some(status)) => {
                job.done = true;
                job.code = read_file_max(&job.files.path("status"), 32)
                    .and_then(|v| String::from_utf8_lossy(&v).trim().parse::<i32>().ok())
                    .unwrap_or(-1);
                // (63 is curl's exit code for an answer over --max-filesize. A curl that the size
                // limit of its start stopped ended with a write error, or by that limit's signal,
                // and left an answer file one byte over the limit.)
                if status.code() == Some(63) || job.answer_len() > HTTP_ANSWER_MAX {
                    job.too_large();
                } else if status.success() {
                    job.body = read_file_max(&job.files.path("body"), HTTP_ANSWER_MAX as usize).unwrap_or_default();
                } else {
                    // A curl that ended with a failure brought no answer, whatever part of one it
                    // had written: the request has no status code, and curl's line is its error.
                    let detail = read_file_max(&job.files.path("err"), 4096)
                        .map(|v| String::from_utf8_lossy(&v).split_whitespace().collect::<Vec<_>>().join(" "))
                        .unwrap_or_default();
                    job.error = if detail.is_empty() { format!("curl exited {:?}", status.code()) } else { detail };
                    job.code = -1;
                }
                // The answer is in memory now: its files are not kept until the next request.
                job.files.remove();
            }
            // An answer whose length was not stated is watched as it grows.
            Ok(None) if job.answer_len() > HTTP_ANSWER_MAX => {
                job.stop();
                job.too_large();
                job.files.remove();
            }
            Ok(None) => {}
            Err(e) => {
                job.done = true;
                job.error = format!("Could not wait for curl: {e}");
                job.code = -1;
                job.files.remove();
            }
        }
    }

    /// What an app's poll is told: -1 with no request, 0 while it runs, 1 once it has ended.
    fn http_poll(&mut self) -> i32 {
        self.http_watch();
        self.http.as_ref().map_or(-1, |job| i32::from(job.done))
    }

    fn http_cancel(&mut self) {
        self.http = None;
    }
}

/// Ends a call that went past a limit the way a call that used up its instructions ends: with the trap the
/// host reports as the app having stopped. The log says which limit it was.
fn past(st: &HostState, limit: &str, max: usize) -> wasmi::Error {
    qlog!("app %s: stopped, a call went past a limit: %s (%d)", &st.id, limit, max as i32);
    crate::wasmhost::out_of_fuel()
}

/// The fixed charge of a call of an import: every import pays it first, whatever it is given.
fn called(caller: &mut Caller<'_, HostState>) -> Result<(), wasmi::Error> {
    charge(caller, APP_CHARGE_CALL)
}

/// The same for a call that asks the system for something beside the host's own work.
fn asked_of_the_system(caller: &mut Caller<'_, HostState>) -> Result<(), wasmi::Error> {
    charge(caller, APP_CHARGE_CALL + APP_CHARGE_SYSTEM)
}

/// The start of a drawing call: the fixed charge, and room in the call for one more command.
fn drawing(caller: &mut Caller<'_, HostState>) -> Result<(), wasmi::Error> {
    called(caller)?;
    room_for_command(caller.data())
}

/// A command's rectangle as it is drawn (`on_screen`), relative to the top left of the body the call draws on.
fn drawn(st: &HostState, x: i32, y: i32, w: i32, h: i32) -> Rect {
    crate::kithost::drawn(st.body, Rect::new(x, y, w, h))
}

/// A thickness as it is drawn: within THICK_REACH of nothing.
fn thick(t: i32) -> i32 {
    t.clamp(-THICK_REACH, THICK_REACH)
}

/// How far from its sides an element with round ends is worked out pixel by pixel (`kithost::round_end`).
fn round_end(st: &HostState, x: i32, y: i32, w: i32, h: i32) -> i64 {
    crate::kithost::round_end(st.body, Rect::new(x, y, w, h))
}

/// The charge for one frame of a shape on the body the call draws on (`kithost::shape_charge`).
fn shape_charge(st: &HostState, x: i32, y: i32, w: i32, h: i32, edge: i64) -> u64 {
    crate::kithost::shape_charge(st.body, Rect::new(x, y, w, h), edge)
}

/// Takes the charge for a shape of `frames` frames from the call's budget (`shape_charge`).
fn pay_shape(caller: &mut Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, edge: i64, frames: u64) -> Result<(), wasmi::Error> {
    let n = shape_charge(caller.data(), x, y, w, h, edge) * frames;
    charge(caller, n)
}

/// The charge for a line on the body the call draws on (`kithost::line_charge`).
fn line_charge(st: &HostState, x0: f32, y0: f32, x1: f32, y1: f32, width: f32) -> u64 {
    crate::kithost::line_charge(st.body, x0, y0, x1, y1, width)
}

/// The charge for cutting a text of `len` bytes short to fit its room (`kithost::cut_text_charge`).
fn cut_text_charge(len: i32) -> u64 {
    crate::kithost::cut_text_charge(len.max(0) as usize)
}

/// Takes the charge for the icon of a command from the call's budget: an icon as large as the smaller side of
/// the command's rectangle, or the `part`th of it. (A text field's icon is charged by the field's height, where
/// the import is: the kit draws it that tall.)
fn pay_icon(caller: &mut Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, part: i32) -> Result<(), wasmi::Error> {
    let r = drawn(caller.data(), x, y, w, h);
    charge(caller, icon_charge((r.w.min(r.h) / part) as i64))
}

/// The charge for `bytes` bytes copied as they are (the saved state, a request's bytes, an answer).
fn copy_charge(bytes: usize) -> u64 {
    bytes as u64 / APP_CHARGE_COPY_BYTES
}

/// Counts a text of `n` bytes about to be read in the call, within the limits of one text and of the call's texts.
fn call_text_room(st: &mut HostState, n: usize) -> Result<(), wasmi::Error> {
    if n > APP_TEXT_MAX {
        return Err(past(st, "the bytes of one text", APP_TEXT_MAX));
    }
    if st.text_bytes + n > APP_CALL_TEXT_MAX {
        return Err(past(st, "the bytes of the texts of a call", APP_CALL_TEXT_MAX));
    }
    st.text_bytes += n;
    Ok(())
}

/// A text of a call, read once its length is within the limit of one text and of the call's texts together and
/// the call's budget has paid `cost` of its length for what the host does with it.
fn call_text(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32, cost: impl FnOnce(usize) -> u64) -> Result<String, wasmi::Error> {
    let n = len.max(0) as usize;
    call_text_room(caller.data_mut(), n)?;
    charge(caller, cost(n))?;
    Ok(String::from_utf8_lossy(&read_mem(caller, ptr, len)).into_owned())
}

/// What an icon's name longer than any icon's is taken as: a name no icon has (it is drawn as one is).
const NO_ICON: &str = "?";

/// An icon's name, read when it is no longer than a name can be. A longer one is not read.
fn icon_name(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Result<String, wasmi::Error> {
    if len.max(0) as usize > APP_ICON_NAME_MAX {
        return Ok(NO_ICON.to_string());
    }
    charge(caller, len.max(0) as u64 * APP_CHARGE_BYTE)?;
    Ok(String::from_utf8_lossy(&read_mem(caller, ptr, len)).into_owned())
}

/// Whether the call may record one more drawing command.
fn room_for_command(st: &HostState) -> Result<(), wasmi::Error> {
    if st.commands.len() >= APP_CALL_COMMANDS_MAX {
        return Err(past(st, "the drawing commands of a call", APP_CALL_COMMANDS_MAX));
    }
    Ok(())
}

/// Counts `n` bytes of state about to be copied in this call.
fn state_moved(st: &mut HostState, n: usize) -> Result<(), wasmi::Error> {
    if st.state_bytes + n > APP_CALL_STATE_MAX {
        return Err(past(st, "the bytes of state moved in a call", APP_CALL_STATE_MAX));
    }
    st.state_bytes += n;
    Ok(())
}

/// `len` bytes of the module's memory from `ptr`, or as many as the memory has there. Every read of the
/// module's memory is this one, and the bytes it copies are counted (`AppRuntime::bytes_read`).
fn read_mem(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Vec<u8> {
    let bytes = crate::wasmhost::read_mem(caller, ptr, len);
    caller.data_mut().read_bytes += bytes.len();
    bytes
}

fn write_mem(caller: &mut Caller<'_, HostState>, ptr: i32, cap: i32, bytes: &[u8]) -> i32 {
    crate::wasmhost::write_mem(caller, ptr, cap, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn only_a_bottom_to_middle_two_finger_swipe_closes() {
        assert!(gesture_matches(900.0, 2300.0, 930.0, 1200.0, 1860, 2480));
        assert!(!gesture_matches(900.0, 1500.0, 930.0, 1200.0, 1860, 2480));
        assert!(!gesture_matches(900.0, 2300.0, 1700.0, 1200.0, 1860, 2480));
    }

    // A line's ends are held within DRAW_REACH of the body: a line inside that reach is kept as given, bit for
    // bit; one that reaches farther is cut where it leaves the reach, on the line it is on; one with no part
    // there, or with an end that is not a number, is no line.
    #[test]
    fn a_line_is_cut_where_it_leaves_the_reach_of_the_body() {
        let body = Rect::new(0, 78, 1860, 2402);
        let reach = crate::wasmhost::DRAW_REACH as f32;
        assert_eq!(reach, 65_536.0);
        let (far_x, far_y) = (1860.0 + reach, 2402.0 + reach);
        for line in [(40.5, 200.25, 800.0, 260.0), (0.0, 0.0, 0.0, 0.0), (-reach, -reach, far_x, far_y), (far_x, 0.0, -reach, far_y), (-5000.0, 3000.0, 9000.0, -70.5)] {
            assert_eq!(line_in_reach(body, line.0, line.1, line.2, line.3), Some(line), "{line:?} is kept as given");
        }
        // Level, upright and at a slant: each end outside the reach is moved to its edge, along the line.
        assert_eq!(line_in_reach(body, -1e9, 500.0, 1e9, 500.0), Some((-reach, 500.0, far_x, 500.0)));
        assert_eq!(line_in_reach(body, 700.0, 1e9, 700.0, -1e9), Some((700.0, far_y, 700.0, -reach)));
        assert_eq!(line_in_reach(body, 100.0, 100.0, 100.0 + 2e6, 100.0 + 1e6), Some((100.0, 100.0, far_x, 100.0 + (far_x - 100.0) / 2.0)));
        let (x0, y0, x1, y1) = line_in_reach(body, -3e6, -3e6, 3e6, 3e6).unwrap();
        assert_eq!((x0, y0, x1, y1), (-reach, -reach, far_x, far_x), "a diagonal ends where it first leaves the reach, each way");
        for v in [f32::MAX, f32::MIN, 1e30, -1e30] {
            let cut = line_in_reach(body, v, 300.0, 900.0, 300.0).unwrap();
            assert_eq!((cut.1, cut.2, cut.3), (300.0, 900.0, 300.0), "{v}");
            assert_eq!(cut.0, if v > 0.0 { far_x } else { -reach }, "{v}");
        }
        // No part in the reach: beside it, and passing it by at a slant.
        assert_eq!(line_in_reach(body, -1e9, 500.0, -reach - 1.0, 500.0), None);
        assert_eq!(line_in_reach(body, 0.0, far_y + 1.0, 1860.0, far_y + 9.0), None);
        assert_eq!(line_in_reach(body, -reach - 10.0, -reach + 5.0, -reach + 5.0, -reach - 10.0), None);
        // (and one that crosses a corner of the reach is the part of it there)
        assert_eq!(line_in_reach(body, -reach - 10.0, 0.0, 0.0, -reach - 10.0), Some((-reach, -10.0, -10.0, -reach)));
        // An end that is not a number.
        for v in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for at in 0..4 {
                let mut ends = [100.0, 100.0, 900.0, 800.0];
                ends[at] = v;
                assert_eq!(line_in_reach(body, ends[0], ends[1], ends[2], ends[3]), None, "{v} as number {at}");
            }
        }
    }

    /// A module with one page of memory, `data` from its address 0, the one import `grimoire.<import>` (as many
    /// numbers in as `args` has, none out), and an `app_draw` that calls the import once with `args`: how a test
    /// has an app draw one command with any numbers.
    fn module_drawing(import: &str, args: &[i32], data: &[u8]) -> Vec<u8> {
        let named = |name: &str, kind: &[u8]| [&[name.len() as u8][..], name.as_bytes(), kind].concat();
        let section = |id: u8, content: Vec<u8>| [&[id, content.len() as u8][..], &content[..]].concat();
        // (i32.const: the number as a signed LEB128)
        let number = |mut v: i32| {
            let mut out = vec![0x41];
            loop {
                let b = (v & 0x7f) as u8;
                v >>= 7;
                if (v == 0 && b & 0x40 == 0) || (v == -1 && b & 0x40 != 0) {
                    out.push(b);
                    return out;
                }
                out.push(b | 0x80);
            }
        };
        let n = args.len();
        // (the import's type and app_draw's; the import, function 0; app_draw, function 1; the memory; the two
        // exports; app_draw's code: each number, the call, the end; the data)
        let kinds = [&[2, 0x60, n as u8][..], &vec![0x7f; n][..], &[0, 0x60, 4, 0x7f, 0x7f, 0x7f, 0x7f, 0][..]].concat();
        let imported = [&[1][..], &named(ABI, &[])[..], &named(import, &[0, 0])[..]].concat();
        let exports = [&[2][..], &named("memory", &[2, 0])[..], &named("app_draw", &[0, 1])[..]].concat();
        let numbers: Vec<u8> = args.iter().flat_map(|a| number(*a)).collect();
        let code = [&[1, numbers.len() as u8 + 4, 0][..], &numbers[..], &[0x10, 0, 0x0b][..]].concat();
        let placed = [&[1, 0, 0x41, 0, 0x0b, data.len() as u8][..], data].concat();
        [b"\0asm\x01\0\0\0".to_vec(), section(1, kinds), section(2, imported), section(3, vec![1, 1]), section(5, vec![1, 0, 1]), section(7, exports), section(10, code), section(11, placed)].concat()
    }

    // The font makes no glyph past its greatest size, whoever draws (font.rs, GLYPH_SIDE_MAX), and the host
    // holds an app's text and icons to sizes of its own far under it. Here the host's own sizes are set aside,
    // so that a module's numbers reach the font as given: an icon in a square of 100,000 px and a letter
    // 150,000 px high, the sizes between and the greatest numbers, through each command that draws an icon or
    // a text. No glyph is made of any of them and stb is not asked for one; the pass ends well, with nothing
    // drawn outside the body. With sizes a screen has, setting the host's own aside changes no pixel.
    #[test]
    fn with_the_hosts_own_sizes_aside_the_fonts_greatest_glyph_holds() {
        let dir = scratch("glyphs");
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let screen = Rect::new(0, 0, 1860, 2480);
        let body = Rect::new(0, 78, 1860, 2402);
        // (one pass of a module that draws one command, with the host's own sizes or with them aside: the canvas
        // after it, and how many bitmaps the font asked stb for)
        let pass = |import: &str, args: &[i32], data: &[u8], aside: bool| -> (Canvas, u32) {
            let mut rt = runtime_of(host(&dir), &module_drawing(import, args, data));
            let mut c = Canvas::default();
            assert_eq!(canvas_init(&mut c, screen.w, screen.h), 0);
            c.px.fill(200);
            let asked = crate::font::font_bitmaps_asked();
            OWN_SIZES_ASIDE.set(aside);
            rt.draw(&mut c, body, 0);
            OWN_SIZES_ASIDE.set(false);
            assert_eq!(rt.error(), None, "{import} {args:?}");
            assert!(c.px[..(body.y * c.stride) as usize].iter().all(|p| *p == 200), "{import} {args:?}: drawn above the body");
            (c, crate::font::font_bitmaps_asked() - asked)
        };
        let inked = |c: &Canvas| c.px[(body.y * c.stride) as usize..].iter().filter(|p| **p != 255).count();
        let most = i32::MAX;
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            // An icon by itself, and a letter at a place: drawn by the host at its own greatest sizes, and not
            // at all once those are aside.
            for side in [6000, 12_000, 58_000, 90_000, 100_000, 120_000, 133_000, most] {
                // (about the body's middle, where the host draws the icon of a larger rectangle)
                let args = [0, 4, 930 - side / 2, 1200 - side / 2, side, side, 0];
                assert!(inked(&pass("icon", &args, b"gear", false).0) > 100_000, "{theme:?}: the host draws the icon of a square of {side} px at its own greatest size");
                let (c, asked) = pass("icon", &args, b"gear", true);
                assert_eq!((inked(&c), asked), (0, 0), "{theme:?}: an icon in a square of {side} px");
            }
            for px_milli in [8_192_000, 20_000_000, 100_000_000, 150_000_000, 1_000_000_000, most] {
                for face in 0..6 {
                    let args = [0, 1000, 0, 1, px_milli, face, 0];
                    assert!(inked(&pass("text_at", &args, b"W", false).0) > 10_000, "{theme:?}: the host draws a letter of {px_milli} thousandths at its own greatest height");
                    let (c, asked) = pass("text_at", &args, b"W", true);
                    // (a pixel face asks stb for a letter's design pixels, whatever the height: one small bitmap)
                    assert!(inked(&c) == 0 && asked <= 1, "{theme:?}: a letter of {px_milli} thousandths in face {face}: {} pixels, {asked} asked", inked(&c));
                    let args = [0, 0, 1860, 2402, 0, 1, px_milli, face, 1, 0];
                    let (c, asked) = pass("text", &args, b"W", true);
                    assert!(inked(&c) == 0 && asked <= 1, "{theme:?}: a text of {px_milli} thousandths in face {face}: {} pixels, {asked} asked", inked(&c));
                }
            }
            // The kit's elements that draw an icon as large as they are, far larger than the body: each is the
            // element with no icon, pixel for pixel, and no bitmap is asked for its icon.
            for (import, with_icon, without) in [
                ("fab", vec![-49_000, -49_000, 100_000, 100_000, 0, 4], None),
                ("button", vec![-49_000, -49_000, 100_000, 100_000, 0, 0, 0, 4, 0], Some(vec![-49_000, -49_000, 100_000, 100_000, 0, 0, 0, 0, 0])),
                ("text_field", vec![-200, -49_000, 2400, 100_000, 0, 4, 0, 0, 0, 0], Some(vec![-200, -49_000, 2400, 100_000, 0, 0, 0, 0, 0, 0])),
            ] {
                let (c, asked) = pass(import, &with_icon, b"gear", true);
                assert_eq!(asked, 0, "{theme:?}: {import} {with_icon:?}");
                if let Some(without) = without {
                    assert!(c.px == pass(import, &without, b"gear", true).0.px, "{theme:?}: {import} {with_icon:?} is the element with no icon");
                }
            }
            // Sizes a screen has: the same picture with the host's own sizes and with them aside.
            for (import, args, data) in [
                ("icon", vec![0, 4, 40, 40, 400, 400, 0], &b"gear"[..]),
                ("text_at", vec![40, 900, 0, 5, 300_000, 1, 0], &b"12:34"[..]),
                ("text", vec![40, 40, 1700, 200, 0, 5, 120_000, 0, 1, 0], &b"12:34"[..]),
                ("button", vec![40, 40, 400, 400, 0, 0, 0, 4, 0], &b"gear"[..]),
            ] {
                let (held, aside) = (pass(import, &args, data, false).0, pass(import, &args, data, true).0);
                assert!(inked(&held) > 1000 && held.px == aside.px, "{theme:?}: {import} {args:?}");
            }
        }
        ui_set_theme(UiTheme::Classic);
    }

    // ---- HTTP ----

    // A header line is a name of token characters, a colon and a value without control characters.
    #[test]
    fn a_header_line_is_a_name_a_colon_and_a_value() {
        let longest = format!("X-Long: {}", "a".repeat(HTTP_HEADER_LINE_MAX - 8));
        assert_eq!(longest.len(), HTTP_HEADER_LINE_MAX);
        for line in [
            "Accept: application/json",
            "X-A:1",
            "X-A:    spaces after the colon",
            "X-A:",
            "X-A: a value: with colons",
            "X-A: caf\u{e9} \u{65e5}\u{672c}",
            "x!#$%&'*+-.^_`|~9: every kind of token character",
            longest.as_str(),
        ] {
            assert!(http_header_line_ok(line), "{line:?}");
        }
        let too_long = format!("X-Long: {}", "a".repeat(HTTP_HEADER_LINE_MAX - 7));
        for line in [
            "",
            "no colon",
            "Accept",
            "X-A;",
            ":",
            ": no name",
            "X A: a space in the name",
            "X-A : a space before the colon",
            " X-A: a space first",
            "@X-A: 1",
            "@",
            "=X-A: 1",
            "=",
            "(X-A): 1",
            "(",
            "<X-A>: 1",
            "<",
            "\"X-A\": 1",
            "\"",
            "Caf\u{e9}: 1",
            "X-A: a\ttab",
            "X-A: a\u{0}b",
            "X-A: a\rb",
            "X-A: a\u{1b}b",
            "X-A: a\u{7f}b",
            "X-A: a\u{85}b",
            "X\tA: 1",
            too_long.as_str(),
        ] {
            assert!(!http_header_line_ok(line), "{line:?}");
        }
        // Of the ASCII characters, only the token characters are part of a name, first or later.
        for b in 0u8..128 {
            let c = b as char;
            let token = c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c);
            assert_eq!(http_header_line_ok(&format!("{c}X: 1")), token, "{c:?} first");
            if c != ':' {
                assert_eq!(http_header_line_ok(&format!("X{c}: 1")), token, "{c:?} later");
            }
        }
    }

    // The lines of a request: white space around a line and empty lines are not part of them, and one
    // line that is not a header line, or one line too many, refuses them all.
    #[test]
    fn the_header_lines_of_a_request_are_taken_whole_or_not_at_all() {
        assert_eq!(http_header_lines(""), Some(vec![]));
        assert_eq!(http_header_lines("\r\n\n  \n"), Some(vec![]));
        assert_eq!(http_header_lines("Accept: text/plain\r\n\r\n  X-A: 1 \t\nX-B:2"), Some(vec!["Accept: text/plain", "X-A: 1", "X-B:2"]));
        assert_eq!(http_header_lines("Accept: text/plain\nno colon\nX-A: 1"), None);
        assert_eq!(http_header_lines("Accept: text/plain\n@X-A: 1"), None);
        for line in ["=X-A: 1", "(X-A): 1", "<X-A>: 1", "\"X-A\": 1"] {
            assert_eq!(http_header_lines(&format!("Accept: text/plain\n{line}")), None, "{line:?}");
        }
        assert_eq!(http_header_lines("X-A: a\rb\n"), None);
        let most = "X-A: 1\n\n".repeat(HTTP_HEADER_LINES_MAX);
        assert_eq!(http_header_lines(&most).map(|l| l.len()), Some(HTTP_HEADER_LINES_MAX));
        assert_eq!(http_header_lines(&format!("{most}X-A: 1")), None);
        let longest = format!("X-Long: {}", "a".repeat(HTTP_HEADER_LINE_MAX - 8));
        assert_eq!(http_header_lines(&format!("  {longest}  \r\n")), Some(vec![longest.as_str()]));
        assert_eq!(http_header_lines(&format!("{longest}a")), None);
    }

    // The whole list curl is started with: -q first, each rule's option, each header line behind
    // its own -H, and the address last.
    #[test]
    fn curl_is_started_with_the_rules_of_a_request_and_the_address_last() {
        let url = "https://example.org/a?b=[1]&c={x,y}";
        let args = http_args("POST", url, &["Accept: text/plain", "X-A: 1"], "/s/http/1.body", Some("/s/http/1.request"));
        assert_eq!(args, [
            "-q", "-sS", "-g",
            "--proto", "=http,https", "--proto-redir", "=http,https",
            "-L", "--max-redirs", "5",
            "--max-time", "30", "--max-filesize", "8388608",
            "-X", "POST", "-o", "/s/http/1.body", "-w", "%{http_code}",
            "-A", "Grimoire-App/2.3",
            "-H", "Accept: text/plain",
            "-H", "X-A: 1",
            "--data-binary", "@/s/http/1.request",
            url,
        ]);
        let plain = http_args("GET", "http://example.org/", &[], "/s/http/2.body", None);
        assert_eq!(plain, [
            "-q", "-sS", "-g",
            "--proto", "=http,https", "--proto-redir", "=http,https",
            "-L", "--max-redirs", "5",
            "--max-time", "30", "--max-filesize", "8388608",
            "-X", "GET", "-o", "/s/http/2.body", "-w", "%{http_code}",
            "-A", "Grimoire-App/2.3",
            "http://example.org/",
        ]);
        // A HEAD request: --head where another request has --max-filesize and its method.
        let head = http_args("HEAD", "http://example.org/", &["X-A: 1"], "/s/http/3.body", None);
        assert_eq!(head, [
            "-q", "-sS", "-g",
            "--proto", "=http,https", "--proto-redir", "=http,https",
            "-L", "--max-redirs", "5",
            "--max-time", "30",
            "--head", "-o", "/s/http/3.body", "-w", "%{http_code}",
            "-A", "Grimoire-App/2.3",
            "-H", "X-A: 1",
            "http://example.org/",
        ]);
        assert_eq!(HTTP_ANSWER_MAX, 8_388_608);
    }

    /// A scratch folder of one test's own (under GRIMOIRE_TEST_TMP when the Makefile sets it).
    fn scratch(name: &str) -> String {
        let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let dir = base.join(format!("grimoire-apphost-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.to_str().unwrap().to_string()
    }

    /// The host's state for an app with `network:http` whose state folder is `dir/state`.
    fn host(dir: &str) -> HostState {
        HostState {
            id: "test.app".to_string(),
            storage: true,
            body: (0, 0),
            commands: Vec::new(),
            hits: Vec::new(),
            text_bytes: 0,
            state_bytes: 0,
            read_bytes: 0,
            state: Vec::new(),
            state_path: format!("{dir}/{STATE_FILE}"),
            settings: Vec::new(),
            close_requested: false,
            status_bar: false,
            status: String::new(),
            texts: Vec::new(),
            toast: None,
            dirty: false,
            limits: crate::wasmhost::store_limits(MEMORY_MAX, APP_TABLE_MAX),
            redraw_at: None,
            screen_w: 0,
            screen_h: 0,
            clock_12h: false,
            network_http: true,
            http: None,
            points: Vec::new(),
            http_stand_in: Vec::new(),
        }
    }

    /// At the head of a stand-in's script: puts the answer file's path, which follows `-o` among
    /// curl's arguments, in `$out`.
    const FIND_ANSWER: &str = "while [ $# -gt 1 ]; do [ \"$1\" = -o ] && out=$2; shift; done; ";

    /// A host whose requests start `sh -c script` in curl's place, by the same start and with
    /// curl's arguments as the script's.
    fn host_standing_in(dir: &str, script: &str) -> HostState {
        let mut st = host(dir);
        st.http_stand_in = ["sh", "-c", script, "curl"].map(String::from).to_vec();
        st
    }

    /// Waits until the child has ended and leaves it unreaped, as an app that does not poll does.
    fn wait_unreaped(pid: u32) {
        loop {
            // SAFETY: waitid fills the siginfo this owns; WNOWAIT leaves the child to be reaped.
            let r = unsafe {
                let mut info: libc::siginfo_t = std::mem::zeroed();
                libc::waitid(libc::P_PID, pid as libc::id_t, &mut info, libc::WEXITED | libc::WNOWAIT)
            };
            if r == 0 {
                return;
            }
            assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::EINTR));
        }
    }

    /// The names of the files in the app's request folder.
    fn request_files(dir: &str) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(format!("{dir}/state/http"))
            .map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default();
        names.sort();
        names
    }

    /// The kinds (the names' endings) of the files in the app's request folder.
    fn request_kinds(dir: &str) -> Vec<String> {
        request_files(dir).iter().map(|n| n.rsplit('.').next().unwrap_or("").to_string()).collect()
    }

    /// True when no process has this id any more: the child was ended and reaped.
    fn gone(pid: u32) -> bool {
        // SAFETY: signal 0 sends nothing: it asks whether the process is there.
        let r = unsafe { libc::kill(pid as libc::pid_t, 0) };
        r == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
    }

    fn child_pid(st: &HostState) -> u32 {
        st.http.as_ref().unwrap().child.id()
    }

    /// Polls a request until it has ended, as an app does from its redraws.
    fn poll_to_end(st: &mut HostState) {
        let end = Instant::now() + Duration::from_secs(45);
        while st.http_poll() == 0 {
            assert!(Instant::now() < end, "the request did not end");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(st.http_poll(), 1);
    }

    /// What an app reads of an ended request: its status code, its answer and its error text.
    fn outcome(st: &HostState) -> (i32, Vec<u8>, String) {
        let job = st.http.as_ref().unwrap();
        (job.code, job.body.clone(), job.error.clone())
    }

    /// A local address that takes a connection and never answers it: a request to it stays in
    /// flight. (The listener is returned to be kept: the address is its.)
    fn silent_address() -> (std::net::TcpListener, String) {
        let l = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let url = format!("http://127.0.0.1:{}/", l.local_addr().unwrap().port());
        (l, url)
    }

    const NEEDS_CURL: &str = "this test starts curl: it must be on the PATH";

    // The address of a request is http or https, at most 4096 bytes, and has no control character
    // and no space. A request with another address starts nothing and makes no file or folder.
    #[test]
    fn a_request_with_a_refused_address_starts_nothing() {
        let longest = format!("http://example.org/{}", "a".repeat(HTTP_ADDRESS_MAX - 19));
        assert_eq!(longest.len(), HTTP_ADDRESS_MAX);
        for url in [
            "http://example.org/",
            "https://example.org/a?b=[1]&c={x,y}#d",
            "http://[::1]:8080/",
            "https://example.org/a%20b",
            "https://example.org/caf\u{e9}",
            longest.as_str(),
        ] {
            assert!(http_address_ok(url), "{url:?}");
        }
        let too_long = format!("{longest}a");
        let refused = [
            "",
            "example.org",
            "ftp://example.org/",
            " http://example.org/",
            "http://example.org/a b",
            "http://example.org/ ",
            "http://example.org/a\tb",
            "http://example.org/a\nb",
            "http://example.org/a\rb",
            "http://example.org/a\u{0}b",
            "http://example.org/a\u{1b}b",
            "http://example.org/a\u{7f}b",
            "http://example.org/a\u{85}b",
            too_long.as_str(),
        ];
        let dir = scratch("refused-address");
        let mut st = host(&dir);
        for url in refused {
            assert!(!http_address_ok(url), "{url:?}");
            assert_eq!(st.http_start("POST", url, "X-A: 1", b"upload"), -1, "{url:?}");
            assert!(st.http.is_none(), "{url:?}");
            assert!(!std::path::Path::new(&format!("{dir}/state")).exists(), "{url:?}");
        }
        assert_eq!(st.http_poll(), -1);
    }

    // A refused request starts nothing and makes no file: every case here is one line that is not a
    // header line, a line over the length limit, or more lines than the limit.
    #[test]
    fn a_request_with_a_refused_header_line_starts_nothing() {
        let dir = scratch("refused");
        let mut st = host(&dir);
        let many = "X-A: 1\n".repeat(HTTP_HEADER_LINES_MAX + 1);
        let long = format!("X-A: {}", "a".repeat(HTTP_HEADER_LINE_MAX));
        for headers in [
            "Accept: text/plain\nno colon",
            "Accept: text/plain\n: no name",
            "X A: 1",
            "@X-A: 1",
            "=X-A: 1",
            "(X-A): 1",
            "<X-A>: 1",
            "\"X-A\": 1",
            "X-A: a\u{1}b",
            many.as_str(),
            long.as_str(),
        ] {
            assert_eq!(st.http_start("POST", "http://127.0.0.1:9/", headers, b"upload"), -1, "{headers:?}");
            assert!(st.http.is_none(), "{headers:?}");
            assert_eq!(request_files(&dir), Vec::<String>::new(), "{headers:?}");
        }
        assert_eq!(st.http_poll(), -1);
    }

    // The method of a request is 1 to 16 ASCII letters, in either case, with white space around
    // it or none; the upload is at most 2 MiB and the header text at most 32 KiB. A request at a
    // limit starts, and one past it starts nothing.
    #[test]
    fn a_request_is_held_to_its_method_and_its_limits() {
        let dir = scratch("limits");
        // (a stand-in that gives the method it was started with as its answer)
        let script = "while [ $# -gt 1 ]; do [ \"$1\" = -o ] && out=$2; [ \"$1\" = -X ] && method=$2; shift; done; printf %s \"$method\" > \"$out\"; printf 200";
        let mut st = host_standing_in(&dir, script);
        let at = "http://127.0.0.1:9/";
        let mut starts = |method: &str, headers: &str, upload: &[u8]| {
            let r = st.http_start(method, at, headers, upload);
            if r != 0 {
                assert!(st.http.is_none());
                assert_eq!(request_files(&dir), Vec::<String>::new());
                return (r, String::new());
            }
            poll_to_end(&mut st);
            let (code, body, error) = outcome(&st);
            assert_eq!((code, error), (200, String::new()));
            st.http_cancel();
            (0, String::from_utf8(body).unwrap())
        };
        let sent = |method: &str| (0, method.to_string());
        let refused = (-1, String::new());

        // The method.
        assert_eq!(starts("GET", "", b""), sent("GET"));
        assert_eq!(starts("get", "", b""), sent("GET"));
        assert_eq!(starts(" \tPatch\r\n", "", b""), sent("PATCH"));
        assert_eq!(starts("A", "", b""), sent("A"));
        assert_eq!(starts("ABCDEFGHIJKLMNOP", "", b""), sent("ABCDEFGHIJKLMNOP"));
        assert_eq!(starts("ABCDEFGHIJKLMNOPQ", "", b""), refused);
        for method in ["", " ", "M-SEARCH", "GET2", "GET POST", "G\u{c9}T", "GET\u{0}", "GET."] {
            assert_eq!(starts(method, "", b""), refused, "{method:?}");
        }
        assert_eq!(starts(&format!("{:^1$}", "GET", HTTP_METHOD_TEXT_MAX), "", b""), sent("GET"));
        assert_eq!(starts(&format!("{:^1$}", "GET", HTTP_METHOD_TEXT_MAX + 1), "", b""), refused);

        // The upload.
        assert_eq!(HTTP_UPLOAD_MAX, 2 * 1024 * 1024);
        assert_eq!(starts("POST", "", &vec![b'u'; HTTP_UPLOAD_MAX]), sent("POST"));
        assert_eq!(starts("POST", "", &vec![b'u'; HTTP_UPLOAD_MAX + 1]), refused);

        // The header text: 32 KiB in 32 lines and in 16, and one byte more.
        assert_eq!(HTTP_HEADERS_MAX, 32 * 1024);
        let short = format!("X-A: {}\n", "a".repeat(1018)).repeat(32);
        let long = format!("X-A: {}\n", "a".repeat(2042)).repeat(16);
        assert_eq!((short.len(), long.len()), (HTTP_HEADERS_MAX, HTTP_HEADERS_MAX));
        assert_eq!(starts("GET", &short, b""), sent("GET"));
        assert_eq!(starts("GET", &long, b""), sent("GET"));
        assert_eq!(starts("GET", &format!("{short} "), b""), refused);
        assert_eq!(starts("GET", &format!("{long}\n"), b""), refused);
    }

    // A start that is refused leaves the outcome of an ended request as it was: the app still
    // reads its status code, its answer and its error text.
    #[test]
    fn a_refused_start_leaves_an_ended_requests_outcome_readable() {
        let dir = scratch("refused-keeps");
        let mut st = host(&dir);
        stand_in(&mut st, &dir, "exit 0", "200", Some(5));
        poll_to_end(&mut st);
        let ended = (200, vec![0u8; 5], String::new());
        assert_eq!(outcome(&st), ended);
        let at = "http://127.0.0.1:9/";
        let too_long = "a".repeat(HTTP_HEADERS_MAX + 1);
        for (method, url, headers, upload) in [
            ("GET", "ftp://example.org/", "", &b""[..]),
            ("GET", "http://example.org/a b", "", b""),
            ("GET", at, "no colon", b""),
            ("GET", at, too_long.as_str(), b""),
            ("GET2", at, "", b""),
            ("", at, "", b""),
            ("HEAD", at, "", b"upload"),
            ("POST", at, "", &vec![b'u'; HTTP_UPLOAD_MAX + 1][..]),
        ] {
            assert_eq!(st.http_start(method, url, headers, upload), -1, "{method:?} {url:?}");
            assert!(st.http.is_some(), "{method:?} {url:?}");
            assert_eq!(outcome(&st), ended, "{method:?} {url:?}");
            assert_eq!(st.http_poll(), 1);
        }
        st.network_http = false;
        assert_eq!(st.http_start("GET", at, "", b""), -2);
        assert_eq!(outcome(&st), ended);
        assert_eq!(st.http_poll(), 1);
        assert_eq!(request_files(&dir), Vec::<String>::new());
    }

    // A start that fails after its arguments were accepted (the request folder is not a real
    // folder, the program cannot be started) leaves the outcome of an ended request as it was
    // too. The next request that starts takes its place.
    #[test]
    fn a_start_that_fails_leaves_an_ended_requests_outcome_readable() {
        let dir = scratch("failed-keeps");
        let mut st = host_standing_in(&dir, "printf 201");
        stand_in(&mut st, &dir, "exit 0", "200", Some(5));
        poll_to_end(&mut st);
        let ended = (200, vec![0u8; 5], String::new());
        assert_eq!(outcome(&st), ended);
        let at = "http://127.0.0.1:9/";

        // The program to start is not there: the start fails after the request's files were made.
        let stand_in_program = std::mem::replace(&mut st.http_stand_in, vec![format!("{dir}/no-such-program")]);
        assert_eq!(st.http_start("POST", at, "X-A: 1", b"upload"), -1);
        assert_eq!(outcome(&st), ended);
        assert_eq!(st.http_poll(), 1);
        assert_eq!(request_files(&dir), Vec::<String>::new());
        st.http_stand_in = stand_in_program;

        // The request folder is a link.
        let folder = format!("{dir}/state/http");
        std::fs::remove_dir(&folder).unwrap();
        std::os::unix::fs::symlink(&dir, &folder).unwrap();
        assert_eq!(st.http_start("POST", at, "X-A: 1", b"upload"), -1);
        assert_eq!(outcome(&st), ended);
        assert_eq!(st.http_poll(), 1);
        std::fs::remove_file(&folder).unwrap();

        // A request that starts takes the ended one's place.
        assert_eq!(st.http_start("GET", at, "", b""), 0);
        assert_eq!(st.http.as_ref().unwrap().code, -1);
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), (201, Vec::new(), String::new()));
        assert_eq!(request_files(&dir), Vec::<String>::new());
    }

    /// Puts a request in flight whose child is `sh -c script`, standing in for curl, with the status
    /// and answer files a curl would have written by then. Returns the child's process id.
    fn stand_in(st: &mut HostState, dir: &str, script: &str, status: &str, answer_len: Option<u64>) -> u32 {
        // (an earlier stand-in goes first: its files have the same names)
        st.http_cancel();
        std::fs::create_dir_all(format!("{dir}/state/http")).unwrap();
        let files = HttpFiles { base: format!("{dir}/state/http/stand-in") };
        std::fs::write(files.path("status"), status).unwrap();
        std::fs::write(files.path("err"), "").unwrap();
        std::fs::write(files.path("request"), "upload").unwrap();
        if let Some(len) = answer_len {
            // (a file of that length without its bytes written: it reads as zeros)
            std::fs::File::create(files.path("body")).unwrap().set_len(len).unwrap();
        }
        let child = Command::new("sh").arg("-c").arg(script).spawn().unwrap();
        let pid = child.id();
        st.http = Some(HttpJob { child, files, done: false, code: -1, body: Vec::new(), error: String::new() });
        pid
    }

    // While curl runs, http_poll watches the answer file: at the limit the request goes on, one byte
    // past it the child is ended and reaped and the request ends with the reason.
    #[test]
    fn an_answer_that_grows_past_the_limit_ends_the_request() {
        let dir = scratch("grows");
        let mut st = host(&dir);
        let pid = stand_in(&mut st, &dir, "exec sleep 60", "", Some(HTTP_ANSWER_MAX));
        assert_eq!(st.http_poll(), 0);
        assert!(!gone(pid));
        let answer = st.http.as_ref().unwrap().files.path("body");
        std::fs::OpenOptions::new().write(true).open(&answer).unwrap().set_len(HTTP_ANSWER_MAX + 1).unwrap();
        assert_eq!(st.http_poll(), 1);
        assert_eq!(outcome(&st), (-1, Vec::new(), "The answer is too large.".to_string()));
        assert!(gone(pid));
        assert_eq!(request_files(&dir), Vec::<String>::new());
        assert_eq!(st.http_poll(), 1);
    }

    // When curl has ended, the answer is read under the same limit: curl's own exit for an answer over
    // it, or a file over it, ends with the reason; an answer at the limit is read whole. Either way
    // the request's files are gone once http_poll has said the request ended.
    #[test]
    fn an_ended_request_is_read_under_the_limit_and_its_files_removed() {
        let dir = scratch("ended");
        let mut st = host(&dir);
        let too_large = (-1, Vec::new(), "The answer is too large.".to_string());

        stand_in(&mut st, &dir, "exit 0", "200", Some(5));
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), (200, vec![0u8; 5], String::new()));
        assert_eq!(request_files(&dir), Vec::<String>::new());

        stand_in(&mut st, &dir, "exit 63", "200", Some(5));
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), too_large);
        assert_eq!(request_files(&dir), Vec::<String>::new());

        stand_in(&mut st, &dir, "exit 0", "200", Some(HTTP_ANSWER_MAX + 1));
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), too_large);
        assert_eq!(request_files(&dir), Vec::<String>::new());

        stand_in(&mut st, &dir, "exit 0", "200", Some(HTTP_ANSWER_MAX));
        poll_to_end(&mut st);
        let (code, body, error) = outcome(&st);
        assert_eq!((code, body.len() as u64, error), (200, HTTP_ANSWER_MAX, String::new()));
        assert_eq!(request_files(&dir), Vec::<String>::new());

        // A curl that the size limit of its start stopped, by a write error or by the limit's
        // signal, left its answer file one byte over the limit.
        stand_in(&mut st, &dir, "exit 23", "200", Some(HTTP_ANSWER_MAX + 1));
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), too_large);
        assert_eq!(request_files(&dir), Vec::<String>::new());

        let pid = stand_in(&mut st, &dir, "kill -s XFSZ $$", "200", Some(HTTP_ANSWER_MAX + 1));
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), too_large);
        assert_eq!(request_files(&dir), Vec::<String>::new());
        assert!(gone(pid));

        // A curl that failed with no answer gives its error text, as before: a failed connection,
        // and a write error that is not the limit's.
        for (script, text) in [("exit 7", "curl: (7) Failed to connect"), ("exit 23", "curl: (23) Failure writing output to destination")] {
            let pid = stand_in(&mut st, &dir, script, "000", None);
            let err = st.http.as_ref().unwrap().files.path("err");
            std::fs::write(err, format!("{text}\n")).unwrap();
            poll_to_end(&mut st);
            assert_eq!(outcome(&st), (-1, Vec::new(), text.to_string()));
            assert_eq!(request_files(&dir), Vec::<String>::new());
            assert!(gone(pid));
        }
    }

    /// The answer of a listener that states 100 bytes, sends 10 and closes the connection.
    fn part_of_an_answer(_first_line: &str) -> Vec<u8> {
        b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n0123456789".to_vec()
    }

    // A curl that ended with a failure is a failed request whatever it had written: no status
    // code, no answer, and curl's line as the error. The part of an answer that arrived is not
    // given.
    #[test]
    fn a_curl_that_ended_with_a_failure_brings_no_answer() {
        let dir = scratch("failure");
        let mut st = host(&dir);
        let none = Vec::<String>::new();

        // Stand-ins that end as curl does when the answer stops short (18), the time is up (28)
        // and the connection fails while the answer arrives (56), each with ten bytes written.
        for (code, text) in [
            (18, "curl: (18) transfer closed with 90 bytes remaining to read"),
            (28, "curl: (28) Operation timed out after 30004 milliseconds with 10 out of 100 bytes received"),
            (56, "curl: (56) Recv failure: Connection reset by peer"),
        ] {
            let pid = stand_in(&mut st, &dir, &format!("exit {code}"), "200", Some(10));
            let err = st.http.as_ref().unwrap().files.path("err");
            std::fs::write(err, format!("{text}\n")).unwrap();
            poll_to_end(&mut st);
            assert_eq!(outcome(&st), (-1, Vec::new(), text.to_string()));
            assert!(gone(pid));
            assert_eq!(request_files(&dir), none);
        }

        // With no line of curl's, the error names how it ended: an exit status, or none when a
        // signal ended it.
        stand_in(&mut st, &dir, "exit 18", "200", Some(10));
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), (-1, Vec::new(), "curl exited Some(18)".to_string()));
        stand_in(&mut st, &dir, "kill -s TERM $$", "200", Some(10));
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), (-1, Vec::new(), "curl exited None".to_string()));
        assert_eq!(request_files(&dir), none);

        // The same ten bytes from a curl that ended well are the answer.
        stand_in(&mut st, &dir, "exit 0", "200", Some(10));
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), (200, vec![0u8; 10], String::new()));

        // The host's curl and a real answer that stops short of its stated length.
        let mut st = host(&dir);
        let (at, stop, served) = serve_each(part_of_an_answer);
        assert_eq!(st.http_start("GET", &format!("{at}/"), "", b""), 0, "{NEEDS_CURL}");
        let pid = child_pid(&st);
        poll_to_end(&mut st);
        let (code, body, error) = outcome(&st);
        assert_eq!((code, body), (-1, Vec::new()), "{error}");
        assert!(error.starts_with("curl: (18) "), "{error}");
        assert!(gone(pid));
        assert_eq!(request_files(&dir), none);
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(served.join().unwrap(), ["GET / HTTP/1.1"]);
    }

    // The child of a request keeps none of the host's descriptors: one that a child started plainly
    // lists among its own is not among those of a request's child.
    #[test]
    fn the_child_of_a_request_has_none_of_the_hosts_descriptors() {
        use std::os::fd::AsRawFd;
        let dir = scratch("descriptors");
        // A descriptor that is not close-on-exec, numbered above the few a child opens itself.
        let folder = std::fs::File::open(&dir).unwrap();
        // SAFETY: fcntl makes a second descriptor of one this test owns; it is closed below.
        let kept = unsafe { libc::fcntl(folder.as_raw_fd(), libc::F_DUPFD, 64) };
        assert!((64..1024).contains(&kept), "{kept}");
        let lists = |text: &str, fd: i32| text.lines().any(|l| l.trim() == fd.to_string());
        let script = format!("{FIND_ANSWER}ls /dev/fd > \"$out\"; printf 200");

        // Started plainly, the stand-in lists that descriptor among its own.
        let plain = format!("{dir}/plain");
        assert!(Command::new("sh").args(["-c", &script, "curl", "-o", &plain]).stdout(std::process::Stdio::null()).status().unwrap().success());
        let listed = std::fs::read_to_string(&plain).unwrap();
        assert!(lists(&listed, 0) && lists(&listed, kept), "{listed}");

        // Started as a request's child, it lists its own 0, 1 and 2 and not that one.
        let mut st = host_standing_in(&dir, &script);
        assert_eq!(st.http_start("GET", "http://127.0.0.1:9/", "X-A: 1", b"upload"), 0);
        let pid = child_pid(&st);
        poll_to_end(&mut st);
        let (code, body, error) = outcome(&st);
        let listed = String::from_utf8(body).unwrap();
        assert_eq!((code, error), (200, String::new()), "{listed}");
        assert!(lists(&listed, 0) && lists(&listed, 1) && lists(&listed, 2), "{listed}");
        assert!(!lists(&listed, kept), "{listed}");
        assert!(gone(pid));
        // SAFETY: closes the descriptor this test made.
        unsafe { libc::close(kept) };
    }

    // The child of a request starts without the variables curl takes a proxy from, and with the
    // rest of the environment: of a command that was given all five and one more variable, and of
    // a request's own child, whatever this test's environment holds.
    #[test]
    fn the_child_of_a_request_has_no_proxy_variable() {
        let names = |text: &str| -> Vec<String> { text.lines().filter_map(|l| l.split_once('=')).map(|(name, _)| name.to_string()).collect() };
        let proxies = ["http_proxy", "https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY"];
        assert_eq!(HTTP_PROXY_VARIABLES, proxies);

        let mut cmd = Command::new("sh");
        cmd.args(["-c", "env"]);
        for name in proxies {
            cmd.env(name, "http://127.0.0.1:9/");
        }
        cmd.env("GRIMOIRE_TEST_KEPT", "1");
        http_no_proxy(&mut cmd);
        let given = names(&String::from_utf8(cmd.output().unwrap().stdout).unwrap());
        assert!(proxies.iter().all(|p| !given.iter().any(|n| n == p)), "{given:?}");
        assert!(given.iter().any(|n| n == "GRIMOIRE_TEST_KEPT") && given.iter().any(|n| n == "PATH"), "{given:?}");

        let dir = scratch("no-proxy");
        let mut st = host_standing_in(&dir, &format!("{FIND_ANSWER}env > \"$out\"; printf 200"));
        assert_eq!(st.http_start("GET", "http://127.0.0.1:9/", "", b""), 0);
        poll_to_end(&mut st);
        let (code, body, error) = outcome(&st);
        assert_eq!((code, error), (200, String::new()));
        let given = names(&String::from_utf8(body).unwrap());
        assert!(proxies.iter().all(|p| !given.iter().any(|n| n == p)), "{given:?}");
        assert!(given.iter().any(|n| n == "PATH"), "{given:?}");
    }

    // On Linux the child of a request carries what the tree's children carry there, read back
    // from the child itself: its oom_score_adj is 500; no file it writes is longer than the
    // answer's limit and one byte; its address space has a soft limit, between this process's
    // own size and that size with twice the margin; and its parent-death signal is the kill
    // signal. CI's sandbox job runs this test (docs/TESTING.md).
    #[cfg(target_os = "linux")]
    #[test]
    fn on_linux_the_child_of_a_request_carries_its_caps_and_its_parent_death_signal() {
        let limits_of = |resource| {
            let mut own = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            // SAFETY: getrlimit writes one rlimit into `own`, which this owns.
            assert_eq!(unsafe { libc::getrlimit(resource, &mut own) }, 0);
            own
        };
        let shown = |limit: libc::rlim_t| if limit == libc::RLIM_INFINITY { "unlimited".to_string() } else { limit.to_string() };
        let dir = scratch("linux-rules");
        // The stand-in gives its oom_score_adj and its limits as the kernel tells them and then,
        // as the same process, the parent-death signal it carries (PR_GET_PDEATHSIG is 2): the
        // signal is a process's own and is not handed on to a child it makes.
        let asks = "import ctypes; signal = ctypes.c_int(-1); print(ctypes.CDLL(None).prctl(2, ctypes.byref(signal)), signal.value)";
        let script = format!("{FIND_ANSWER}cat /proc/self/oom_score_adj /proc/self/limits > \"$out\"; printf 200; exec python3 -I -c '{asks}' >> \"$out\"");
        let mut st = host_standing_in(&dir, &script);
        let before = child_vm_bytes().unwrap();
        assert_eq!(st.http_start("GET", "http://127.0.0.1:9/", "", b""), 0);
        let after = child_vm_bytes().unwrap();
        let pid = child_pid(&st);
        poll_to_end(&mut st);
        let (code, body, error) = outcome(&st);
        let text = String::from_utf8(body).unwrap();
        assert_eq!((code, error.as_str()), (200, ""), "this test needs python3 on the PATH: {text}");
        assert!(gone(pid));
        let lines: Vec<&str> = text.lines().collect();
        let limit = |name: &str| -> Vec<&str> { lines.iter().find_map(|l| l.strip_prefix(name)).unwrap_or_else(|| panic!("{name}: {text}")).split_whitespace().collect() };

        assert_eq!(lines[0], CHILD_OOM_ADJ.to_string(), "{text}");
        assert_eq!(CHILD_OOM_ADJ, 500);

        let most = (HTTP_ANSWER_MAX + 1) as libc::rlim_t;
        let own = limits_of(libc::RLIMIT_FSIZE);
        assert_eq!(limit("Max file size"), [shown(own.rlim_cur.min(most)).as_str(), shown(own.rlim_max.min(most)).as_str(), "bytes"], "{text}");
        assert_eq!(most, 8_388_609);

        let own = limits_of(libc::RLIMIT_AS);
        let space = limit("Max address space");
        let soft: u64 = space[0].parse().unwrap_or_else(|_| panic!("the address space has no soft limit: {text}"));
        if own.rlim_cur == libc::RLIM_INFINITY && own.rlim_max == libc::RLIM_INFINITY {
            assert!(soft >= before.min(after) && soft <= before.max(after) + 2 * HTTP_CHILD_MARGIN, "{soft} for a process of {before} to {after} bytes: {text}");
            assert_eq!(space[1..], ["unlimited", "bytes"], "{text}");
        } else {
            // (a limit this process already has is never raised)
            assert!(own.rlim_cur == libc::RLIM_INFINITY || soft <= own.rlim_cur as u64, "{text}");
            assert_eq!(space[1], shown(own.rlim_max), "{text}");
        }

        assert_eq!(lines.last(), Some(&format!("0 {}", libc::SIGKILL).as_str()), "{text}");
    }

    // The size limit a request's child starts under holds with no poll at all: the answer file of a
    // child that writes without end stops one byte past the answer's limit, and the request ends
    // as too large. A real answer of exactly the limit still comes back whole.
    #[test]
    fn the_answer_file_stops_a_byte_past_the_limit_without_a_poll() {
        let dir = scratch("unpolled");
        let too_large = (-1, Vec::new(), "The answer is too large.".to_string());

        // A stand-in that writes 10 MiB to its answer file, were nothing to stop it, and then
        // tells how its writer ended: with an error of its own (1), not by a signal.
        let script = format!("{FIND_ANSWER}dd if=/dev/zero of=\"$out\" bs=65536 count=160; printf %s $?");
        let mut st = host_standing_in(&dir, &script);
        assert_eq!(st.http_start("GET", "http://127.0.0.1:9/", "", b""), 0);
        let pid = child_pid(&st);
        wait_unreaped(pid);
        let job = st.http.as_ref().unwrap();
        assert_eq!(job.answer_len(), HTTP_ANSWER_MAX + 1);
        assert_eq!(std::fs::read_to_string(job.files.path("status")).unwrap(), "1");
        assert_eq!(st.http_poll(), 1);
        assert_eq!(outcome(&st), too_large);
        assert!(gone(pid));
        assert_eq!(request_files(&dir), Vec::<String>::new());

        // The host's curl and a real answer with no stated length, one byte over the limit.
        let mut st = host(&dir);
        let mut grows = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
        grows.resize(grows.len() + HTTP_ANSWER_MAX as usize + 1, b'a');
        let (at, served) = serve_once(grows);
        assert_eq!(st.http_start("GET", &format!("{at}/"), "", b""), 0, "{NEEDS_CURL}");
        let pid = child_pid(&st);
        wait_unreaped(pid);
        assert!(st.http.as_ref().unwrap().answer_len() <= HTTP_ANSWER_MAX + 1);
        assert_eq!(st.http_poll(), 1);
        assert_eq!(outcome(&st), too_large);
        assert!(gone(pid));
        assert_eq!(request_files(&dir), Vec::<String>::new());
        served.join().unwrap();

        // A real answer of exactly the limit is under every limit: it comes back whole.
        let mut whole = format!("HTTP/1.1 200 OK\r\nContent-Length: {HTTP_ANSWER_MAX}\r\nConnection: close\r\n\r\n").into_bytes();
        whole.resize(whole.len() + HTTP_ANSWER_MAX as usize, b'a');
        let (at, served) = serve_once(whole);
        assert_eq!(st.http_start("GET", &format!("{at}/"), "", b""), 0);
        poll_to_end(&mut st);
        let (code, body, error) = outcome(&st);
        assert_eq!((code, body.len() as u64, error), (200, HTTP_ANSWER_MAX, String::new()));
        assert_eq!(request_files(&dir), Vec::<String>::new());
        served.join().unwrap();
    }

    // Every way out of a request leaves none of its files and no child: a start that failed, a
    // cancel, the host dropped while curl runs (the app stopped), an answer read, a new request.
    #[test]
    fn a_request_leaves_no_file_and_no_child_on_any_way_out() {
        let dir = scratch("ways-out");
        let none = Vec::<String>::new();

        // A start that fails after the request's files were made: the program to start is not
        // there. (The request folder is: the start came as far as making it.)
        let mut st = host(&dir);
        st.http_stand_in = vec![format!("{dir}/no-such-program")];
        assert_eq!(st.http_start("POST", "http://127.0.0.1:9/", "X-A: 1", b"upload"), -1);
        assert!(st.http.is_none());
        assert!(std::path::Path::new(&format!("{dir}/state/http")).is_dir());
        assert_eq!(request_files(&dir), none);
        st.http_stand_in.clear();

        // A request in flight has its files; another start is refused and changes nothing.
        let (_listener, silent) = silent_address();
        assert_eq!(st.http_start("POST", &silent, "X-A: 1", b"upload"), 0, "{NEEDS_CURL}");
        let pid = child_pid(&st);
        assert_eq!(st.http_poll(), 0);
        assert_eq!(request_kinds(&dir), ["err", "request", "status"]);
        assert_eq!(st.http_start("GET", &silent, "", b""), -3);
        assert_eq!(child_pid(&st), pid);
        assert_eq!(request_kinds(&dir), ["err", "request", "status"]);

        // http_cancel.
        st.http_cancel();
        assert!(gone(pid));
        assert_eq!(request_files(&dir), none);
        assert_eq!(st.http_poll(), -1);

        // The host dropped while curl runs.
        assert_eq!(st.http_start("POST", &silent, "", b"upload"), 0);
        let pid = child_pid(&st);
        assert_eq!(request_kinds(&dir), ["err", "request", "status"]);
        drop(st);
        assert!(gone(pid));
        assert_eq!(request_files(&dir), none);

        // An outcome read (here curl's failure at a listener of the test's own, which closes the
        // connection without an answer): the files are gone once http_poll has said so, and what
        // was read stays until the next request or a cancel.
        let mut st = host(&dir);
        let (at, stop, served) = serve_each(no_answer);
        assert_eq!(st.http_start("GET", &format!("{at}/"), "", b""), 0);
        let pid = child_pid(&st);
        poll_to_end(&mut st);
        let (code, body, error) = outcome(&st);
        assert_eq!((code, body), (-1, Vec::new()));
        assert!(error.starts_with("curl: (52) "), "{error}");
        assert!(gone(pid));
        assert_eq!(request_files(&dir), none);
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(served.join().unwrap(), ["GET / HTTP/1.1"]);

        // A new request after it: only the new one's files are there.
        assert_eq!(st.http_start("GET", &silent, "", b""), 0);
        let pid = child_pid(&st);
        assert_eq!(request_kinds(&dir), ["err", "status"]);
        drop(st);
        assert!(gone(pid));
        assert_eq!(request_files(&dir), none);
    }

    // The names http_start gives a request's files, and no other name.
    #[test]
    fn a_request_files_name_is_two_numbers_and_a_kind() {
        for name in ["1-2.body", "1-2.status", "1-2.err", "1-2.request", "40213-1795000000123456789.body"] {
            assert!(http_file_name(name), "{name:?}");
        }
        for name in [
            "", "body", ".body", "1-2", "1-2.", "1-2.other", "1-2.body.tmp", "1-2.BODY", "12.body", "-2.body", "1-.body",
            "x-2.body", "1-x.body", "1-2-3.body", "1 -2.body", "+1-2.body", "stand-in.body", "runtime.bin", "notes.txt",
        ] {
            assert!(!http_file_name(name), "{name:?}");
        }
    }

    // What an earlier run left in the request folder is removed when the next request starts: the
    // plain files with a request file's name, and nothing else.
    #[test]
    fn the_request_files_of_an_earlier_run_are_removed_at_the_next_start() {
        let dir = scratch("earlier-run");
        let folder = format!("{dir}/state/http");
        let (_listener, silent) = silent_address();

        // An earlier run that stopped with its request in flight: nothing removed the files.
        let mut st = host(&dir);
        assert_eq!(st.http_start("POST", &silent, "", b"upload"), 0, "{NEEDS_CURL}");
        let job = st.http.as_mut().unwrap();
        std::fs::write(job.files.path("body"), "part of an answer").unwrap();
        job.child.kill().unwrap();
        job.child.wait().unwrap();
        std::mem::forget(st);
        let left = request_files(&dir);
        assert_eq!(request_kinds(&dir), ["body", "err", "request", "status"]);
        std::fs::write(format!("{folder}/1-2.status"), "200").unwrap();

        // Other things in the folder: other names, and a folder and a link with such a name.
        let outside = format!("{dir}/outside");
        std::fs::write(&outside, "kept").unwrap();
        let mut others = vec!["1-2.other", "7-7.err", "8-8.body", "notes.txt", "x-2.body"];
        std::fs::write(format!("{folder}/notes.txt"), "kept").unwrap();
        std::fs::write(format!("{folder}/1-2.other"), "kept").unwrap();
        std::fs::write(format!("{folder}/x-2.body"), "kept").unwrap();
        std::fs::create_dir(format!("{folder}/7-7.err")).unwrap();
        std::os::unix::fs::symlink(&outside, format!("{folder}/8-8.body")).unwrap();

        // A request that is refused starts nothing and removes nothing.
        let mut st = host(&dir);
        assert_eq!(st.http_start("GET", &silent, "no colon", b""), -1);
        assert_eq!(request_files(&dir).len(), left.len() + 1 + others.len());

        // The next request: the earlier run's files are gone, the others are there, and the
        // request has its own two.
        assert_eq!(st.http_start("GET", &silent, "", b""), 0);
        let now = request_files(&dir);
        assert!(left.iter().all(|name| !now.contains(name)), "{now:?}");
        assert!(!now.contains(&"1-2.status".to_string()), "{now:?}");
        let own: Vec<&String> = now.iter().filter(|name| !others.contains(&name.as_str())).collect();
        assert_eq!(own.len(), 2, "{now:?}");
        assert!(own[0].ends_with(".err") && own[1].ends_with(".status") && http_file_name(own[0]), "{now:?}");
        assert_eq!(now.len(), others.len() + 2, "{now:?}");
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "kept");
        assert!(std::fs::symlink_metadata(format!("{folder}/8-8.body")).unwrap().file_type().is_symlink());
        assert!(std::fs::symlink_metadata(format!("{folder}/7-7.err")).unwrap().is_dir());

        // A request folder that is a link to a folder is left as it is.
        let elsewhere = format!("{dir}/elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        std::fs::write(format!("{elsewhere}/1-2.body"), "kept").unwrap();
        let link = format!("{dir}/link");
        std::os::unix::fs::symlink(&elsewhere, &link).unwrap();
        http_sweep(&link);
        assert_eq!(std::fs::read_to_string(format!("{elsewhere}/1-2.body")).unwrap(), "kept");
        http_sweep(&elsewhere);
        assert!(!std::path::Path::new(&format!("{elsewhere}/1-2.body")).exists());

        drop(st);
        others.sort();
        assert_eq!(request_files(&dir), others);
    }

    // A request is started only when the app's state folder and its http folder are real folders.
    // With a link at either name, whatever it leads to, or a file there, the start fails: nothing
    // is started, nothing is made and nothing is removed.
    #[test]
    fn a_request_is_started_only_in_real_folders() {
        use std::os::unix::fs::symlink;
        let listed = |folder: &str| {
            let mut names: Vec<String> = std::fs::read_dir(folder).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
            names.sort();
            names
        };
        let refused = |dir: &str| {
            let mut st = host_standing_in(dir, "printf 200");
            assert_eq!(st.http_start("POST", "http://127.0.0.1:9/", "X-A: 1", b"upload"), -1, "{dir}");
            assert!(st.http.is_none(), "{dir}");
        };

        // The state folder is a link to a folder that has a request folder with files in it.
        let dir = scratch("links-state");
        let target = format!("{dir}/target");
        std::fs::create_dir_all(format!("{target}/http")).unwrap();
        std::fs::write(format!("{target}/http/5-5.body"), "kept").unwrap();
        std::fs::write(format!("{target}/http/notes.txt"), "kept").unwrap();
        symlink(&target, format!("{dir}/state")).unwrap();
        refused(&dir);
        assert_eq!(listed(&format!("{target}/http")), ["5-5.body", "notes.txt"]);
        assert_eq!(listed(&dir), ["state", "target"]);

        // The state folder is a link to a folder with nothing in it, and a link that leads nowhere.
        let dir = scratch("links-state-empty");
        let target = format!("{dir}/target");
        std::fs::create_dir(&target).unwrap();
        symlink(&target, format!("{dir}/state")).unwrap();
        refused(&dir);
        assert_eq!(listed(&target), Vec::<String>::new());
        std::fs::remove_file(format!("{dir}/state")).unwrap();
        symlink(format!("{dir}/nowhere"), format!("{dir}/state")).unwrap();
        refused(&dir);
        assert_eq!(listed(&dir), ["state", "target"]);

        // The request folder is a link to a folder with files in it, and a link that leads nowhere.
        let dir = scratch("links-http");
        let elsewhere = format!("{dir}/elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        std::fs::write(format!("{elsewhere}/5-5.body"), "kept").unwrap();
        std::fs::create_dir(format!("{dir}/state")).unwrap();
        symlink(&elsewhere, format!("{dir}/state/http")).unwrap();
        refused(&dir);
        assert_eq!(listed(&elsewhere), ["5-5.body"]);
        std::fs::remove_file(format!("{dir}/state/http")).unwrap();
        symlink(format!("{dir}/nowhere"), format!("{dir}/state/http")).unwrap();
        refused(&dir);
        assert_eq!(listed(&dir), ["elsewhere", "state"]);
        assert_eq!(listed(&format!("{dir}/state")), ["http"]);

        // A file at either name.
        let dir = scratch("links-files");
        std::fs::create_dir(format!("{dir}/state")).unwrap();
        std::fs::write(format!("{dir}/state/http"), "kept").unwrap();
        refused(&dir);
        assert_eq!(std::fs::read_to_string(format!("{dir}/state/http")).unwrap(), "kept");
        std::fs::remove_file(format!("{dir}/state/http")).unwrap();
        std::fs::remove_dir(format!("{dir}/state")).unwrap();
        std::fs::write(format!("{dir}/state"), "kept").unwrap();
        refused(&dir);
        assert_eq!(std::fs::read_to_string(format!("{dir}/state")).unwrap(), "kept");

        // Real folders, there already or made by the start: the request starts, and what an
        // earlier run left in the request folder goes.
        let dir = scratch("links-none");
        let mut st = host_standing_in(&dir, "printf 200");
        assert_eq!(st.http_start("GET", "http://127.0.0.1:9/", "", b""), 0);
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), (200, Vec::new(), String::new()));
        std::fs::write(format!("{dir}/state/http/5-5.body"), "left").unwrap();
        assert_eq!(st.http_start("GET", "http://127.0.0.1:9/", "", b""), 0);
        poll_to_end(&mut st);
        assert_eq!(request_files(&dir), Vec::<String>::new());
    }

    /// An open app's runtime around `st`, with a module that has nothing in it: what the host's
    /// tick needs.
    fn runtime(st: HostState) -> AppRuntime {
        let engine = Engine::default();
        let module = Module::new(&engine, &b"\0asm\x01\0\0\0"[..]).unwrap();
        let mut store = Store::new(&engine, st);
        let instance = Linker::new(&engine).instantiate_and_start(&mut store, &module).unwrap();
        AppRuntime { _engine: engine, store, instance, status: String::new(), texts: Vec::new(), error: None, used: 0, drawn: 0, hits: Vec::new() }
    }

    /// Ticks the host until its request has ended, with no poll of the app's.
    fn tick_to_end(rt: &mut AppRuntime) {
        let end = Instant::now() + Duration::from_secs(45);
        while !rt.store.data().http.as_ref().unwrap().done {
            assert!(Instant::now() < end, "the request did not end");
            std::thread::sleep(Duration::from_millis(5));
            rt.tick();
        }
    }

    // The host's tick looks after a request the app never polls: an ended child is reaped, its
    // outcome read and its files removed, and an answer file past the limit ends the request.
    // A poll after that is told what is known already.
    #[test]
    fn the_hosts_tick_looks_after_a_request_between_the_apps_polls() {
        let dir = scratch("tick");
        let none = Vec::<String>::new();

        // A request that ends by itself, started as curl is.
        let script = format!("{FIND_ANSWER}printf answer > \"$out\"; printf 200");
        let mut rt = runtime(host_standing_in(&dir, &script));
        assert_eq!(rt.store.data_mut().http_start("POST", "http://127.0.0.1:9/", "", b"upload"), 0);
        let pid = child_pid(rt.store.data());
        tick_to_end(&mut rt);
        assert!(gone(pid));
        assert_eq!(request_files(&dir), none);
        let answered = (200, b"answer".to_vec(), String::new());
        assert_eq!(outcome(rt.store.data()), answered);
        assert_eq!(rt.store.data_mut().http_poll(), 1);
        assert_eq!(outcome(rt.store.data()), answered);

        // A request whose answer file grows past the limit while its child runs.
        let pid = stand_in(rt.store.data_mut(), &dir, "exec sleep 60", "", Some(HTTP_ANSWER_MAX));
        rt.tick();
        assert!(!rt.store.data().http.as_ref().unwrap().done && !gone(pid));
        let answer = rt.store.data().http.as_ref().unwrap().files.path("body");
        std::fs::OpenOptions::new().write(true).open(&answer).unwrap().set_len(HTTP_ANSWER_MAX + 1).unwrap();
        rt.tick();
        assert_eq!(outcome(rt.store.data()), (-1, Vec::new(), "The answer is too large.".to_string()));
        assert!(gone(pid));
        assert_eq!(request_files(&dir), none);
        assert_eq!(rt.store.data_mut().http_poll(), 1);

        // The request of an app that has stopped with an error is looked after too.
        rt.error = Some("stopped".to_string());
        let pid = stand_in(rt.store.data_mut(), &dir, "exit 0", "200", Some(5));
        tick_to_end(&mut rt);
        assert!(gone(pid));
        assert_eq!(request_files(&dir), none);
        assert_eq!(outcome(rt.store.data()), (200, vec![0u8; 5], String::new()));

        // With no request there is nothing to look after.
        rt.store.data_mut().http_cancel();
        rt.tick();
        assert_eq!(rt.store.data_mut().http_poll(), -1);
    }

    /// Serves one request at a local address: reads it whole (its head, and the upload its
    /// Content-Length states), writes `answer`, and waits for the other end to close. The thread
    /// gives back the request as it was received.
    fn serve_once(answer: Vec<u8>) -> (String, std::thread::JoinHandle<String>) {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let at = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
        let served = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = s.read(&mut buf).unwrap();
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got);
                if let Some((head, upload)) = text.split_once("\r\n\r\n") {
                    let stated = head.lines().find_map(|l| l.strip_prefix("Content-Length: ")).map_or(0, |v| v.parse::<usize>().unwrap());
                    if upload.len() >= stated {
                        break;
                    }
                }
                assert!(n > 0, "the request ended early: {text:?}");
            }
            s.write_all(&answer).unwrap();
            let _ = s.read(&mut buf);
            String::from_utf8_lossy(&got).into_owned()
        });
        (at, served)
    }

    /// Serves requests at a local address until `stop` is set, one to a connection, which it then
    /// closes: `answer` makes the answer from a request's first line. The thread gives back the
    /// first lines received, in order.
    fn serve_each(answer: fn(&str) -> Vec<u8>) -> (String, std::sync::Arc<std::sync::atomic::AtomicBool>, std::thread::JoinHandle<Vec<String>>) {
        use std::io::{Read, Write};
        use std::sync::atomic::Ordering;
        let l = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        l.set_nonblocking(true).unwrap();
        let at = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stopped = stop.clone();
        let served = std::thread::spawn(move || {
            let mut firsts = Vec::new();
            while !stopped.load(Ordering::SeqCst) {
                let Ok((mut s, _)) = l.accept() else {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                };
                s.set_nonblocking(false).unwrap();
                s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
                let mut got = Vec::new();
                let mut buf = [0u8; 4096];
                while !got.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = s.read(&mut buf).unwrap();
                    assert!(n > 0, "the request ended early: {:?}", String::from_utf8_lossy(&got));
                    got.extend_from_slice(&buf[..n]);
                }
                let first = String::from_utf8_lossy(&got).lines().next().unwrap_or("").to_string();
                s.write_all(&answer(&first)).unwrap();
                firsts.push(first);
            }
            firsts
        });
        (at, stop, served)
    }

    /// The answer of a listener that closes the connection without one.
    fn no_answer(_first_line: &str) -> Vec<u8> {
        Vec::new()
    }

    /// The answers of a listener that redirects: `/hop/N` leads to `/hop/N-1` and `/hop/0` is the
    /// last answer; `/to/ADDRESS` leads to ADDRESS. Every redirect has an answer of its own.
    fn redirecting(first_line: &str) -> Vec<u8> {
        let path = first_line.split(' ').nth(1).unwrap_or("");
        let moved = |to: &str| format!("HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 5\r\nConnection: close\r\n\r\nmoved").into_bytes();
        if let Some(to) = path.strip_prefix("/to/") {
            return moved(to);
        }
        match path.strip_prefix("/hop/").and_then(|n| n.parse::<u32>().ok()) {
            Some(n) if n > 0 => moved(&format!("/hop/{}", n - 1)),
            _ => b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\narrived".to_vec(),
        }
    }

    // The redirects of a real request: five are followed and bring the last answer; a sixth ends
    // the request as a failure with no answer, and so does a redirect to an address that is not
    // http or https, which is not asked for.
    #[test]
    fn a_real_request_follows_five_redirects_and_only_to_http_and_https() {
        let dir = scratch("redirects");
        let mut st = host(&dir);
        let (at, stop, served) = serve_each(redirecting);
        let mut ask = |path: &str| {
            assert_eq!(st.http_start("GET", &format!("{at}{path}"), "", b""), 0, "{NEEDS_CURL}");
            let pid = child_pid(&st);
            poll_to_end(&mut st);
            assert!(gone(pid));
            assert_eq!(request_files(&dir), Vec::<String>::new());
            outcome(&st)
        };
        assert_eq!(ask("/hop/5"), (200, b"arrived".to_vec(), String::new()));
        let (code, body, error) = ask("/hop/6");
        assert_eq!((code, body), (-1, Vec::new()), "{error}");
        assert!(error.starts_with("curl: (47) "), "{error}");
        for to in ["ftp://127.0.0.1:1/a", "file:///dev/null", "gopher://127.0.0.1:1/"] {
            let (code, body, error) = ask(&format!("/to/{to}"));
            assert_eq!((code, body), (-1, Vec::new()), "{to}: {error}");
            assert!(error.starts_with("curl: (1) "), "{to}: {error}");
        }
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        let asked: Vec<String> = [5, 4, 3, 2, 1, 0, 6, 5, 4, 3, 2, 1]
            .map(|n| format!("GET /hop/{n} HTTP/1.1"))
            .into_iter()
            .chain(["ftp://127.0.0.1:1/a", "file:///dev/null", "gopher://127.0.0.1:1/"].map(|to| format!("GET /to/{to} HTTP/1.1")))
            .collect();
        assert_eq!(served.join().unwrap(), asked);
    }

    // A real request to a local listener: the method, the address as written, the two header lines
    // and the upload arrive, and the status code and the answer come back.
    #[test]
    fn a_real_request_carries_its_header_lines_and_brings_its_answer() {
        let dir = scratch("real");
        let mut st = host(&dir);
        let (at, served) = serve_once(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nworld".to_vec());
        let url = format!("{at}/a?b=[1]&c={{x,y}}");
        assert_eq!(st.http_start("post", &url, "Accept: text/plain\r\n\r\n  X-Grimoire-Test: one two  \n", b"hello"), 0, "{NEEDS_CURL}");
        let pid = child_pid(&st);
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), (200, b"world".to_vec(), String::new()));
        assert!(gone(pid));
        assert_eq!(request_files(&dir), Vec::<String>::new());
        let request = served.join().unwrap();
        let (head, upload) = request.split_once("\r\n\r\n").unwrap();
        let mut lines = head.split("\r\n");
        assert_eq!(lines.next(), Some("POST /a?b=[1]&c={x,y} HTTP/1.1"), "{head}");
        let lines: Vec<&str> = lines.collect();
        for line in ["Accept: text/plain", "X-Grimoire-Test: one two", "User-Agent: Grimoire-App/2.3", "Content-Length: 5"] {
            assert_eq!(lines.iter().filter(|l| **l == line).count(), 1, "{line:?} in {head}");
        }
        assert_eq!(upload, "hello");
    }

    /// The answer to a HEAD request of a listener that closes the connection behind it.
    fn head_of_twelve_bytes(_first_line: &str) -> Vec<u8> {
        b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nContent-Type: text/plain\r\n\r\n".to_vec()
    }

    // A real HEAD request ends when the answer's header lines have arrived, whether the listener
    // keeps the connection open behind them or closes it, and those lines are its answer: of
    // each answer in turn when a redirect was followed, and whatever length they state. A HEAD
    // request with an upload is refused.
    #[test]
    fn a_real_head_request_brings_the_answers_header_lines() {
        let dir = scratch("head");
        let mut st = host(&dir);
        let none = Vec::<String>::new();
        let lines = head_of_twelve_bytes("");

        // A listener that keeps the connection open until the other end closes it.
        let (at, served) = serve_once(lines.clone());
        assert_eq!(st.http_start("head", &format!("{at}/a"), "X-A: 1", b""), 0, "{NEEDS_CURL}");
        let pid = child_pid(&st);
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), (200, lines.clone(), String::new()));
        assert!(gone(pid));
        assert_eq!(request_files(&dir), none);
        let request = served.join().unwrap();
        assert!(request.starts_with("HEAD /a HTTP/1.1\r\n"), "{request}");
        assert_eq!(request.split("\r\n").filter(|l| *l == "X-A: 1").count(), 1, "{request}");

        // A listener that closes the connection behind the header lines.
        let (at, stop, served) = serve_each(head_of_twelve_bytes);
        assert_eq!(st.http_start("HEAD", &format!("{at}/b"), "", b""), 0);
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), (200, lines.clone(), String::new()));
        assert_eq!(request_files(&dir), none);
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(served.join().unwrap(), ["HEAD /b HTTP/1.1"]);

        // The length the header lines state is not the answer's: one over the answer's limit.
        let large = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", HTTP_ANSWER_MAX + 1).into_bytes();
        let (at, served) = serve_once(large.clone());
        assert_eq!(st.http_start("HEAD", &format!("{at}/"), "", b""), 0);
        poll_to_end(&mut st);
        assert_eq!(outcome(&st), (200, large, String::new()));
        served.join().unwrap();

        // A redirect that is followed: the header lines of both answers, in turn.
        let (at, stop, served) = serve_each(redirecting);
        assert_eq!(st.http_start("HEAD", &format!("{at}/hop/1"), "", b""), 0);
        poll_to_end(&mut st);
        let (code, body, error) = outcome(&st);
        let text = String::from_utf8(body).unwrap();
        assert_eq!((code, error), (200, String::new()), "{text}");
        let moved = "HTTP/1.1 302 Found\r\nLocation: /hop/0\r\nContent-Length: 5\r\nConnection: close\r\n\r\n";
        let arrived = "HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\n";
        assert_eq!(text, format!("{moved}{arrived}"));
        assert_eq!(request_files(&dir), none);
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(served.join().unwrap(), ["HEAD /hop/1 HTTP/1.1", "HEAD /hop/0 HTTP/1.1"]);

        // A HEAD request has no upload: with one it is refused, and the last outcome stays.
        let before = outcome(&st);
        assert_eq!(st.http_start("HEAD", "http://127.0.0.1:9/", "", b"upload"), -1);
        assert_eq!(st.http_start(" head ", "http://127.0.0.1:9/", "", b"u"), -1);
        assert_eq!(outcome(&st), before);
        assert_eq!(request_files(&dir), none);
    }

    // One Content-Type line arrives with a real upload: curl's default, or the request's own line
    // in its place. A request with no upload has none.
    #[test]
    fn a_real_upload_carries_one_content_type_line_and_its_own_when_it_has_one() {
        let dir = scratch("content-type");
        let mut st = host(&dir);
        let mut kinds = |headers: &str, upload: &[u8]| -> Vec<String> {
            let (at, served) = serve_once(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec());
            assert_eq!(st.http_start("POST", &format!("{at}/"), headers, upload), 0, "{NEEDS_CURL}");
            poll_to_end(&mut st);
            assert_eq!(outcome(&st), (200, b"ok".to_vec(), String::new()));
            let request = served.join().unwrap();
            assert!(request.ends_with(&format!("\r\n\r\n{}", String::from_utf8_lossy(upload))), "{request}");
            request.split("\r\n").filter(|l| l.to_ascii_lowercase().starts_with("content-type")).map(String::from).collect()
        };
        assert_eq!(kinds("", b"a=1"), ["Content-Type: application/x-www-form-urlencoded"]);
        assert_eq!(kinds("Accept: text/plain", b"{}"), ["Content-Type: application/x-www-form-urlencoded"]);
        assert_eq!(kinds("Content-Type: application/json", b"{}"), ["Content-Type: application/json"]);
        assert_eq!(kinds("content-type: text/plain\nX-A: 1", b"hello"), ["content-type: text/plain"]);
        assert_eq!(kinds("", b""), Vec::<String>::new());
    }

    // One User-Agent line arrives with a real request: the host's default, or the request's own
    // line in its place, however its name is written. A line with an empty value sends none.
    #[test]
    fn a_real_request_carries_one_user_agent_line_and_its_own_when_it_has_one() {
        let dir = scratch("user-agent");
        let mut st = host(&dir);
        let mut agents = |headers: &str| -> Vec<String> {
            let (at, served) = serve_once(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec());
            assert_eq!(st.http_start("GET", &format!("{at}/"), headers, b""), 0, "{NEEDS_CURL}");
            poll_to_end(&mut st);
            assert_eq!(outcome(&st), (200, b"ok".to_vec(), String::new()));
            let request = served.join().unwrap();
            request.split("\r\n").filter(|l| l.to_ascii_lowercase().starts_with("user-agent")).map(String::from).collect()
        };
        assert_eq!(agents(""), ["User-Agent: Grimoire-App/2.3"]);
        assert_eq!(agents("Accept: text/plain"), ["User-Agent: Grimoire-App/2.3"]);
        assert_eq!(agents("User-Agent: mine/1"), ["User-Agent: mine/1"]);
        assert_eq!(agents("Accept: text/plain\nuser-agent: mine/2\nX-A: 1"), ["user-agent: mine/2"]);
        assert_eq!(agents("User-Agent:"), Vec::<String>::new());
    }

    // A real answer over the limit, with its length stated and without: the request ends with the
    // reason, no status code and no answer, and nothing of it is left.
    #[test]
    fn a_real_answer_over_the_limit_ends_with_the_reason() {
        let dir = scratch("real-large");
        let mut st = host(&dir);
        let stated = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", HTTP_ANSWER_MAX + 1).into_bytes();
        // (not stated: the answer runs until the connection closes, and is one byte over the limit)
        let mut grows = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
        grows.resize(grows.len() + HTTP_ANSWER_MAX as usize + 1, b'a');
        for answer in [stated, grows] {
            let (at, served) = serve_once(answer);
            assert_eq!(st.http_start("GET", &format!("{at}/"), "", b""), 0, "{NEEDS_CURL}");
            let pid = child_pid(&st);
            poll_to_end(&mut st);
            assert_eq!(outcome(&st), (-1, Vec::new(), "The answer is too large.".to_string()));
            assert!(gone(pid));
            assert_eq!(request_files(&dir), Vec::<String>::new());
            served.join().unwrap();
        }
    }

    /// A module with a memory of `pages` pages, the one import `grimoire.<import>` (`params`
    /// numbers in, one out) and the export `call`, which passes its arguments to the import and
    /// gives back its answer: how a test calls an import as an app does.
    fn module_calling(import: &str, params: u8, pages: u16) -> Vec<u8> {
        let named = |name: &str, kind: &[u8]| [&[name.len() as u8][..], name.as_bytes(), kind].concat();
        let section = |id: u8, content: Vec<u8>| [&[id, content.len() as u8][..], &content[..]].concat();
        // (the function type; the import, function 0; `call`, function 1; the memory, its least
        // and most the same; the two exports; `call`'s code: each argument, the call, the end)
        let kind = [&[1, 0x60, params][..], &vec![0x7f; params as usize][..], &[1, 0x7f][..]].concat();
        let import = [&[1][..], &named(ABI, &[])[..], &named(import, &[0, 0])[..]].concat();
        let size = [(pages & 0x7f) as u8 | 0x80, (pages >> 7) as u8];
        let memory = [&[1, 1][..], &size[..], &size[..]].concat();
        let exports = [&[2][..], &named("memory", &[2, 0])[..], &named("call", &[0, 1])[..]].concat();
        let arguments: Vec<u8> = (0..params).flat_map(|i| [0x20, i]).collect();
        let code = [&[1, arguments.len() as u8 + 4, 0][..], &arguments[..], &[0x10, 0, 0x0b][..]].concat();
        [
            b"\0asm\x01\0\0\0".to_vec(),
            section(1, kind),
            section(2, import),
            section(3, vec![1, 0]),
            section(5, memory),
            section(7, exports),
            section(10, code),
        ]
        .concat()
    }

    /// An open app's runtime around `st` and `wasm`, with the host's imports and an app's limits.
    fn runtime_of(mut st: HostState, wasm: &[u8]) -> AppRuntime {
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, wasm).unwrap();
        st.limits = crate::wasmhost::store_limits(MEMORY_MAX, APP_TABLE_MAX);
        let mut store = Store::new(&engine, st);
        store.limiter(|st| &mut st.limits);
        store.set_fuel(APP_CALL_FUEL).unwrap();
        let mut linker = Linker::new(&engine);
        install_imports(&mut linker).unwrap();
        let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
        AppRuntime { _engine: engine, store, instance, status: String::new(), texts: Vec::new(), error: None, used: 0, drawn: 0, hits: Vec::new() }
    }

    /// The module's memory, to place a request's texts in and to read what a call wrote.
    fn memory_of(rt: &mut AppRuntime) -> &mut [u8] {
        rt.instance.get_memory(&rt.store, "memory").unwrap().data_mut(&mut rt.store)
    }

    /// The quickest of `times` runs of `run`.
    fn quickest(times: usize, mut run: impl FnMut()) -> Duration {
        (0..times)
            .map(|_| {
                let start = Instant::now();
                run();
                start.elapsed()
            })
            .min()
            .unwrap()
    }

    // http_body and http_error write what the host holds of an ended request into the module's
    // memory: as much as the room given and the memory take, and nothing with no request. What
    // the host holds stays where it was, and http_body does not copy the answer for a call: with
    // an answer of the limit's size, the quickest call that asks for one byte takes less than an
    // eighth of the time of the quickest single copy of that answer.
    #[test]
    fn the_answer_and_the_error_are_written_from_what_the_host_holds() {
        type Write = wasmi::TypedFunc<(i32, i32), i32>;
        let dir = scratch("written");
        let pages = (HTTP_ANSWER_MAX >> 16) as u16 + 1;
        let size = pages as usize * 65536;
        let answer: Vec<u8> = (0..HTTP_ANSWER_MAX as usize).map(|i| (i % 251) as u8 + 1).collect();
        let text = "curl: (7) Failed to connect";
        let mut st = host(&dir);
        stand_in(&mut st, &dir, "exit 0", "200", None);
        poll_to_end(&mut st);
        let job = st.http.as_mut().unwrap();
        job.body = answer.clone();
        job.error = text.to_string();

        // Each call's answer and what it wrote, for the rooms an app may give.
        let written = |rt: &mut AppRuntime, write: &Write, held: &[u8]| {
            let len = held.len();
            let rooms = [(16, len + 100, len), (16, len, len), (16, 10, 10), (16, 1, 1), (size - 3, 10, 3)];
            for (at, room, n) in rooms {
                memory_of(rt).fill(0);
                assert_eq!(write.call(&mut rt.store, (at as i32, room as i32)).unwrap(), n as i32, "{at} {room}");
                let memory = memory_of(rt);
                assert!(memory[at..at + n] == held[..n], "{at} {room}");
                assert!(memory[..at].iter().chain(&memory[at + n..]).all(|b| *b == 0), "{at} {room}");
            }
            memory_of(rt).fill(0);
            for (at, room) in [(0, 0), (0, -1), (-1, 10), (size as i32, 10), (i32::MAX, 10)] {
                assert_eq!(write.call(&mut rt.store, (at, room)).unwrap(), 0, "{at} {room}");
            }
            assert!(memory_of(rt).iter().all(|b| *b == 0));
        };
        let held = |st: &HostState| {
            let job = st.http.as_ref().unwrap();
            (job.body.as_ptr(), job.body.len(), job.error.as_ptr(), job.error.clone())
        };
        let before = held(&st);

        let mut rt = runtime_of(st, &module_calling("http_body", 2, pages));
        let body: Write = rt.instance.get_typed_func(&rt.store, "call").unwrap();
        written(&mut rt, &body, &answer);
        let a_copy = quickest(3, || drop(std::hint::black_box(answer.clone())));
        let took = quickest(20, || assert_eq!(body.call(&mut rt.store, (0, 1)).unwrap(), 1));
        assert!(took * 8 < a_copy, "a call for one byte took {took:?}, one copy of the answer {a_copy:?}");
        assert_eq!(held(rt.store.data()), before);
        assert!(rt.store.data().http.as_ref().unwrap().body == answer);

        let mut rt = runtime_of(rt.store.into_data(), &module_calling("http_error", 2, pages));
        let error: Write = rt.instance.get_typed_func(&rt.store, "call").unwrap();
        written(&mut rt, &error, text.as_bytes());
        assert_eq!(held(rt.store.data()), before);

        // With no request there is nothing to write.
        rt.store.data_mut().http_cancel();
        memory_of(&mut rt).fill(0);
        assert_eq!(error.call(&mut rt.store, (16, 100)).unwrap(), 0);
        let mut rt = runtime_of(rt.store.into_data(), &module_calling("http_body", 2, pages));
        let body: Write = rt.instance.get_typed_func(&rt.store, "call").unwrap();
        assert_eq!(body.call(&mut rt.store, (16, 100)).unwrap(), 0);
        assert!(memory_of(&mut rt).iter().all(|b| *b == 0));
    }

    /// What a call of `write` (a place, a room: the import's answer) writes into the module's memory, for the
    /// rooms an app may give, against `held`, what the host has to write: the first bytes of it that the room and
    /// the memory take, and nothing before the place or behind what was written. A place at or past the memory's
    /// end or under nothing, and a room of nothing or under it, write nothing.
    fn keeps_to_its_room(rt: &mut AppRuntime, write: &dyn Fn(&mut AppRuntime, i32, i32) -> i32, held: &[u8], what: &str) {
        let (len, size) = (held.len(), memory_of(rt).len());
        assert!(len >= 3, "{what}");
        let rooms = [(16, len + 100, len), (16, len, len), (16, len - 1, len - 1), (16, 1, 1), (size - 3, 10, 3), (size - 1, len, 1), (size - 1, i32::MAX as usize, 1)];
        for (at, room, n) in rooms {
            let before = memory_of(rt).to_vec();
            assert_eq!(write(rt, at as i32, room as i32), n as i32, "{what}: at {at}, room {room}");
            let after = memory_of(rt);
            assert!(after[at..at + n] == held[..n], "{what}: at {at}, room {room}");
            assert!(after[..at] == before[..at] && after[at + n..] == before[at + n..], "{what}: at {at}, room {room}: a byte beside its room changed");
            after[at..at + n].copy_from_slice(&before[at..at + n]);
        }
        let before = memory_of(rt).to_vec();
        for (at, room) in [(16, 0), (16, -1), (16, i32::MIN), (-1, 10), (i32::MIN, 10), (size as i32, 10), (size as i32 + 1, i32::MAX), (i32::MAX, 10)] {
            assert_eq!(write(rt, at, room), 0, "{what}: at {at}, room {room}");
        }
        assert!(*memory_of(rt) == before[..], "{what}: written with no room or no place");
    }

    // Each import that writes into the module's memory writes no more than the room the module gives and the
    // memory holds (http_body and http_error: the test above): state_get, setting, time_text, date_text and
    // wifi_ssid. wifi_ssid writes the connected network's name, which is 32 bytes at most: the Wi-Fi helper's
    // report of a longer name is not read (wifi.rs), and no name is written for it.
    #[test]
    fn each_import_that_writes_keeps_to_the_room_given_and_the_memory() {
        type Write = wasmi::TypedFunc<(i32, i32), i32>;
        type Setting = wasmi::TypedFunc<(i32, i32, i32, i32), i32>;
        let dir = scratch("writes");
        // (a module of one page calling the import, its memory a byte no text has)
        let calling = |st: HostState, import: &str, params: u8| {
            let mut rt = runtime_of(st, &module_calling(import, params, 1));
            memory_of(&mut rt).fill(0xa5);
            rt
        };

        let state = b"the saved state of an app".to_vec();
        let mut st = host(&dir);
        st.state = state.clone();
        let mut rt = calling(st, "state_get", 2);
        let get: Write = rt.instance.get_typed_func(&rt.store, "call").unwrap();
        keeps_to_its_room(&mut rt, &|rt: &mut AppRuntime, at, room| get.call(&mut rt.store, (at, room)).unwrap(), &state, "state_get");

        let value = "the value of a setting";
        let mut st = host(&dir);
        st.settings = vec![("key".to_string(), value.to_string())];
        let mut rt = calling(st, "setting", 4);
        memory_of(&mut rt)[40_000..40_003].copy_from_slice(b"key");
        let setting: Setting = rt.instance.get_typed_func(&rt.store, "call").unwrap();
        keeps_to_its_room(&mut rt, &|rt: &mut AppRuntime, at, room| setting.call(&mut rt.store, (40_000, 3, at, room)).unwrap(), value.as_bytes(), "setting");

        // (the clock's texts, worked out here as the host works them out, out of the last two seconds of a minute:
        // the calls give the same texts)
        while wall_time() % 60 >= 58 {
            std::thread::sleep(Duration::from_millis(100));
        }
        let mut time = String::new();
        format_clock_short(wall_time(), false, &mut time, 64);
        let tm = local_tm(wall_time());
        let date = format!("{:04}-{:02}-{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday);
        for (import, held) in [("time_text", time.as_bytes()), ("date_text", date.as_bytes())] {
            let mut rt = calling(host(&dir), import, 2);
            let text: Write = rt.instance.get_typed_func(&rt.store, "call").unwrap();
            keeps_to_its_room(&mut rt, &|rt: &mut AppRuntime, at, room| text.call(&mut rt.store, (at, room)).unwrap(), held, import);
        }

        // (the Wi-Fi state, as the helper's report of the network connected to sets it)
        let script = format!("{dir}/wifi-helper.sh");
        let connected_to = |name: &[u8]| {
            let hex: String = name.iter().map(|b| format!("{b:02x}")).collect();
            std::fs::write(&script, format!("printf 'wireless=1\\nenabled=1\\nconnected=1\\ncurrent={hex}\\n'\n")).unwrap();
            wifi_init(Some(&script));
            assert!(wifi_request(WifiAction::Current));
            let end = Instant::now() + Duration::from_secs(20);
            while wifi_busy() {
                assert!(Instant::now() < end, "the Wi-Fi helper did not end");
                wifi_tick();
                std::thread::sleep(Duration::from_millis(5));
            }
            wifi_current()
        };
        let longest: Vec<u8> = (0..APP_SSID_MAX).map(|k| b'a' + (k % 26) as u8).collect();
        assert_eq!((APP_SSID_MAX, connected_to(&longest)), (32, Some(longest.clone())));
        let mut rt = calling(host(&dir), "wifi_ssid", 2);
        let ssid: Write = rt.instance.get_typed_func(&rt.store, "call").unwrap();
        keeps_to_its_room(&mut rt, &|rt: &mut AppRuntime, at, room| ssid.call(&mut rt.store, (at, room)).unwrap(), &longest, "wifi_ssid");
        let longer = [&longest[..], b"z"].concat();
        assert_eq!(connected_to(&longer), None, "a report of a name of 33 bytes is not read");
        assert_eq!(ssid.call(&mut rt.store, (16, 64)).unwrap(), 0);
        assert!(memory_of(&mut rt).iter().all(|b| *b == 0xa5));
        wifi_init(None);
    }

    // The http_start import answers from the capability, the request in flight and the request's
    // four lengths before it reads the request: -2, -3 and -1 in that order, and a refused call
    // copies nothing of the module's memory. The measure is time: the module's whole 48 MiB is
    // named by each of the four lengths, and the quickest refused call takes less time than the
    // quickest single copy of that memory.
    #[test]
    fn a_start_refused_by_its_lengths_reads_nothing_of_the_request() {
        type Start = wasmi::TypedFunc<(i32, i32, i32, i32, i32, i32, i32, i32), i32>;
        let dir = scratch("lengths");
        let wasm = module_calling("http_start", 8, (MEMORY_MAX >> 16) as u16);
        let mut rt = runtime_of(host_standing_in(&dir, "printf 200"), &wasm);
        let start: Start = rt.instance.get_typed_func(&rt.store, "call").unwrap();
        let whole = MEMORY_MAX as i32;
        memory_of(&mut rt).fill(0xff);
        assert_eq!(memory_of(&mut rt).len(), MEMORY_MAX);
        let a_copy = quickest(3, || drop(std::hint::black_box(memory_of(&mut rt).to_vec())));
        let refused = |rt: &mut AppRuntime, lengths: [i32; 4], answer: i32, what: &str| {
            let [method, url, headers, body] = lengths;
            let took = quickest(5, || assert_eq!(start.call(&mut rt.store, (0, method, 0, url, 0, headers, 0, body)).unwrap(), answer, "{what}"));
            assert!(took < a_copy, "{what}: a refused call took {took:?}, one copy of the module's memory {a_copy:?}");
        };

        // Without the capability: -2, whatever the lengths and whether or not a request runs.
        rt.store.data_mut().network_http = false;
        refused(&mut rt, [whole; 4], -2, "no capability");
        let pid = stand_in(rt.store.data_mut(), &dir, "exec sleep 60", "", None);
        refused(&mut rt, [whole; 4], -2, "no capability, a request in flight");

        // With it, while a request is in flight: -3, whatever the lengths.
        rt.store.data_mut().network_http = true;
        refused(&mut rt, [whole; 4], -3, "a request in flight");
        refused(&mut rt, [3, 20, 0, 0], -3, "a request in flight, a request that could start");
        assert_eq!(child_pid(rt.store.data()), pid);
        rt.store.data_mut().http_cancel();
        assert!(gone(pid));

        // With no request in flight: -1 for any one length over its limit.
        let most = [HTTP_METHOD_TEXT_MAX as i32, HTTP_ADDRESS_MAX as i32, HTTP_HEADERS_MAX as i32, HTTP_UPLOAD_MAX as i32];
        refused(&mut rt, [whole; 4], -1, "every length the whole memory");
        refused(&mut rt, [i32::MAX; 4], -1, "every length the largest number");
        for i in 0..4 {
            let mut lengths = [0; 4];
            lengths[i] = whole;
            refused(&mut rt, lengths, -1, &format!("length {i} the whole memory"));
            lengths = most;
            lengths[i] += 1;
            refused(&mut rt, lengths, -1, &format!("length {i} one over its limit"));
        }
        assert!(rt.store.data().http.is_none());
        assert_eq!(request_files(&dir), Vec::<String>::new());

        // A request of exactly the four limits is read and started: the method with white space
        // around it, the longest address, 32 KiB of header lines and the largest upload.
        let method = format!("{:^1$}", "post", HTTP_METHOD_TEXT_MAX);
        let url = format!("http://127.0.0.1:9/{}", "a".repeat(HTTP_ADDRESS_MAX - 19));
        let headers = format!("X-A: {}\n", "a".repeat(1018)).repeat(32);
        let texts = [method.as_bytes(), url.as_bytes(), headers.as_bytes(), &vec![b'u'; HTTP_UPLOAD_MAX][..]];
        assert_eq!(texts.map(<[u8]>::len), most.map(|n| n as usize));
        let mut at = [0usize; 4];
        let mut next = 0;
        for (i, text) in texts.iter().enumerate() {
            at[i] = next;
            memory_of(&mut rt)[next..next + text.len()].copy_from_slice(text);
            next += text.len();
        }
        let request = (at[0] as i32, most[0], at[1] as i32, most[1], at[2] as i32, most[2], at[3] as i32, most[3]);
        assert_eq!(start.call(&mut rt.store, request).unwrap(), 0);
        let pid = child_pid(rt.store.data());
        assert_eq!(std::fs::metadata(rt.store.data().http.as_ref().unwrap().files.path("request")).unwrap().len(), HTTP_UPLOAD_MAX as u64);
        poll_to_end(rt.store.data_mut());
        assert_eq!(outcome(rt.store.data()), (200, Vec::new(), String::new()));
        assert!(gone(pid));
        assert_eq!(request_files(&dir), Vec::<String>::new());
    }
}

/// The pure geometry of the system Home swipe, shared by the host and its test.
pub fn gesture_matches(x: f32, y: f32, x2: f32, y2: f32, w: i32, h: i32) -> bool {
    let (aw, ah) = (w.max(1), h.max(1));
    y >= ah as f32 * 0.72
        && y2 <= ah as f32 * 0.62
        && y2 <= y - ah as f32 * 0.12
        && (x2 - x).abs() <= aw as f32 * 0.22
}
