//! The WebAssembly widget host (2.2): a generic frame body, nothing about any one widget.
//!
//! A package's artifact is a WebAssembly module with a `memory` export and two exports the host
//! calls:
//!
//! ```text
//! widget_draw(width: i32, height: i32, scale_milli: i32)
//! widget_tap(id: i32)
//! ```
//!
//! `widget_draw` draws through the `grimoire` imports below. Coordinates are screen pixels inside
//! the frame's body; `scale_milli` is the UI scale times 1000, so a package can keep 300 dpi sizes.
//! `widget_tap` receives the id of the hit region under the tap, or -1 for a tap elsewhere in the
//! body. A package keeps its state through `state_get` / `state_set`, which the host saves under
//! `packages/widgets/<id>/state/runtime.bin`.
//!
//! The body is all a module draws on and is tapped in: every command is drawn with the canvas
//! clipped to the body, whatever coordinates and sizes it carries, and a hit region is the part of
//! it inside the body (`WidgetRuntime::draw`).
//!
//! Imports (`grimoire`):
//!
//! ```text
//! rect(x, y, w, h, style)
//! text(x, y, w, h, ptr, len, px, center)
//! button(id, x, y, w, h, ptr, len, style)
//! hit(id, x, y, w, h)
//! status(ptr, len)
//! request_redraw(after_ms)
//! setting(key_ptr, key_len, out_ptr, out_cap) -> i32
//! state_get(ptr, cap) -> i32
//! state_set(ptr, len)
//! now() -> i64
//! ```
//!
//! `style` is 0 (normal), 1 (selected) or 2 (disabled). Strings are UTF-8 in the module's memory.
//! `setting` and `state_get` return the number of bytes written, at most `out_cap` / `cap`.
//!
//! A module has the capabilities its manifest declares and no other (docs/WIDGETS.md):
//! `editor:overlay` is the drawing in the frame (`rect`, `text`, `button`, `hit`, and `request_redraw`,
//! which asks for a drawing pass), and `storage:widget-local` the saved state (`state_get`,
//! `state_set`, the state file). Without the capability the call is still there to link against, and
//! does nothing. `status`, `setting` and `now` need none.

use crate::prelude::*;
use crate::widgetpkg::{WIDGET_CAP_OVERLAY, WIDGET_CAP_STORAGE};
use wasmi::{Caller, Config, Engine, Extern, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, TrapCode};

const ABI: &str = "grimoire";
const STATE_FILE: &str = "state/runtime.bin";

// ---- the limits: what one package, one call and one drawing pass may ask for (docs/WIDGETS.md, Limits) ----
//
// The host calls a module's `widget_draw` (a drawing pass) and `widget_tap`. These are the numbers by
// which the host counts what a call asks of it, and what a module may hold. The counts start at
// nothing with each call, a count or a length is compared with its limit before the module's memory
// is read, and a call that goes past one is stopped as a call that used up its instructions is
// (`past`).

// (The module's file is at most WIDGET_PKG_ARTIFACT_MAX, widgetpkg.rs.)
/// A module has one linear memory, and it may not grow past this.
const MEMORY_MAX: usize = 32 * 1024 * 1024;
/// A module has at most one table, of at most this many entries (the function references its indirect
/// calls go through), and a store holds one instance of one module. A module that declares a second
/// memory or table, or one larger than its limit, is not started; `memory.grow` and `table.grow` past
/// a limit fail as WebAssembly defines it (they return -1), and the module runs on.
pub const WIDGET_TABLE_MAX: usize = 65_536;
const MEMORIES_MAX: usize = 1;
const TABLES_MAX: usize = 1;
const INSTANCES_MAX: usize = 1;
/// A call's instruction budget: the module's own instructions and the host's work for its imports
/// (the charges below) are paid from it. A call that needs more is stopped.
pub const WIDGET_CALL_FUEL: u64 = 20_000_000;
/// The saved state: `state_set` keeps the first this many bytes.
pub const WIDGET_STATE_MAX: usize = 64 * 1024;
/// The bytes `state_get` and `state_set` may move in one call, all their uses together: the whole
/// state read and written several times over.
pub const WIDGET_CALL_STATE_MAX: usize = 16 * WIDGET_STATE_MAX;
/// The drawing commands (`rect`, `text`, `button`) of one pass.
pub const WIDGET_PASS_COMMANDS_MAX: usize = 512;
/// The hit regions (`button`, `hit`) of one pass.
pub const WIDGET_PASS_HITS_MAX: usize = 512;
/// The bytes of one text: a `text`'s, or a `button`'s label.
pub const WIDGET_TEXT_MAX: usize = 1024;
/// The bytes of all the texts of one pass together.
pub const WIDGET_PASS_TEXT_MAX: usize = 16 * 1024;
/// The bytes of the status line.
pub const WIDGET_STATUS_MAX: usize = 256;
/// The delay `request_redraw` may name, in milliseconds; one outside is taken as the nearer end.
/// The shortest is how often a package can have itself drawn again.
pub const WIDGET_REDRAW_MIN_MS: i32 = 100;
pub const WIDGET_REDRAW_MAX_MS: i32 = 86_400_000;
/// A text is drawn at most this many pixels high; a larger `px` is drawn at this size.
pub const WIDGET_TEXT_PX_MAX: i32 = 512;
// ---- the charges: what the host's work for an import takes from the call's budget (docs/WIDGETS.md, Limits) ----
//
// The budget counts the module's instructions. The host's work for an import is no instruction of the
// module's, so each import takes a charge for it from the same budget (`charge`): a fixed one for every
// call, and one by size where the work grows with what the import is given. The numbers are in
// instructions: about what the work costs beside a module's own, as measured in a release build on a
// desktop machine, where the longest drawing a budget pays for takes about three times as long as the
// same budget spent on instructions alone (not measured on a Kindle). When the budget cannot pay, the
// call ends exactly as a call that used up its instructions ends.

/// Every call of an import, whatever it is given and whether or not the package has the capability for it.
pub const WIDGET_CHARGE_CALL: u64 = 500;
/// `rect`: one for this many pixels of the part of the rectangle inside the body (it is filled and outlined).
pub const WIDGET_CHARGE_RECT_PX: u64 = 4;
/// `button`: one for this many pixels of the part of the button inside the body (a button is drawn in more
/// steps than a rectangle), and its label as a text WIDGET_CHARGE_LABEL_PX high.
pub const WIDGET_CHARGE_BUTTON_PX: u64 = 2;
pub const WIDGET_CHARGE_LABEL_PX: i32 = 48;
/// `text`, and a label: for each byte, this for each pixel of its height up to WIDGET_CHARGE_TALL_PX; a
/// taller letter costs by its area, so above that height a byte is charged by the square of the height
/// (the same at WIDGET_CHARGE_TALL_PX exactly).
pub const WIDGET_CHARGE_TEXT: u64 = 32;
pub const WIDGET_CHARGE_TALL_PX: u64 = 64;
/// `text`, and a label: this for the square of its bytes. A text that does not fit its line is cut short,
/// and a label is broken into two lines, by steps that each measure the text again. A `text` is cut to
/// its first 251 bytes before those steps (font.rs, `fit`), so its bytes are counted to
/// WIDGET_CHARGE_SQUARE_BYTES at most; a label's are all counted.
pub const WIDGET_CHARGE_SQUARE: u64 = 16;
pub const WIDGET_CHARGE_SQUARE_BYTES: u64 = 256;
/// `status`, `setting`, `state_get`, `state_set`: each byte read from the module's memory or written to it.
pub const WIDGET_CHARGE_BYTE: u64 = 4;

/// How far outside the body, in pixels, an edge of a command's rectangle is still kept as given. A
/// farther edge is moved in to this distance: what shows in the body is the same, and the sums the
/// drawing makes with the rectangle stay small.
const DRAW_REACH: i64 = 1 << 16;

/// A drawing command recorded by the module and rendered by the host. Coordinates are body-local.
#[derive(Clone, Debug)]
enum DrawCmd {
    Rect { x: i32, y: i32, w: i32, h: i32, style: i32 },
    Text { x: i32, y: i32, w: i32, h: i32, text: String, px: i32, center: bool },
    Button { x: i32, y: i32, w: i32, h: i32, text: String, style: i32 },
}

struct HostState {
    /// the package's id, for the log
    id: String,
    /// the manifest declares `editor:overlay`: the drawing calls and hit regions are recorded, and a
    /// redraw may be asked for
    overlay: bool,
    /// the manifest declares `storage:widget-local`: the state is read, kept and saved
    storage: bool,
    /// the size of the body the call draws on (a drawing pass's; a tap keeps the last pass's), for the
    /// charge of a rectangle by the part of it inside the body
    body: (i32, i32),
    commands: Vec<DrawCmd>,
    hits: Vec<(i32, Rect)>,
    /// the bytes of the texts in `commands`, and the bytes state_get and state_set moved in this call
    text_bytes: usize,
    state_bytes: usize,
    /// the bytes read from the module's memory in this call (`read_mem`)
    read_bytes: usize,
    state: Vec<u8>,
    state_path: String,
    settings: Vec<(String, String)>,
    status: String,
    dirty: bool,
    limits: StoreLimits,
    /// when the module asked to be drawn again, by the app's clock (`now_ms`); None when it did not
    redraw_at: Option<u32>,
}

/// One open package's WebAssembly instance, its last draw and its saved state.
pub struct WidgetRuntime {
    _engine: Engine,
    store: Store<HostState>,
    instance: wasmi::Instance,
    hits: Vec<(i32, Rect)>,
    texts: Vec<String>,
    status: String,
    error: Option<String>,
    /// what the last call took from its budget
    used: u64,
}

impl std::fmt::Debug for WidgetRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetRuntime").field("hits", &self.hits.len()).field("error", &self.error).finish()
    }
}

impl WidgetRuntime {
    /// Compiles and instantiates the installed package's artifact, once its file has been checked again
    /// (`widget_package_artifact`: a plain file inside the package's folder, with the SHA-256 recorded at
    /// the install). A frame loads its module once, when it first draws.
    pub fn new(id: &str) -> Result<Self, String> {
        let (m, bytes) = crate::widgetpkg::widget_package_artifact(id)?;
        if m.spec.runtime != "wasm" {
            return Err("This package is not a WebAssembly widget.".into());
        }
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, &bytes[..]).map_err(|e| format!("The widget's WebAssembly is invalid: {e}"))?;
        let state_path = format!("{}/{STATE_FILE}", crate::widgetpkg::widget_package_dir(id));
        let overlay = m.capabilities.iter().any(|c| c == WIDGET_CAP_OVERLAY);
        let storage = m.capabilities.iter().any(|c| c == WIDGET_CAP_STORAGE);
        if !overlay {
            qlog!("widget %s: its manifest does not declare %s, so nothing it draws is shown", id, WIDGET_CAP_OVERLAY);
        }
        // (a state file left by a version that declared the capability is not read, and stays as it is)
        let state = if storage { read_file_max(&state_path, WIDGET_STATE_MAX).unwrap_or_default() } else { Vec::new() };
        let settings = crate::widgetpkg::widget_settings_load(id, &m.settings);
        let mut store = Store::new(&engine, HostState {
            id: id.to_string(),
            overlay,
            storage,
            body: (0, 0),
            commands: Vec::new(),
            hits: Vec::new(),
            text_bytes: 0,
            state_bytes: 0,
            read_bytes: 0,
            state,
            state_path,
            settings,
            status: String::new(),
            dirty: false,
            limits: StoreLimitsBuilder::new()
                .memory_size(MEMORY_MAX)
                .memories(MEMORIES_MAX)
                .tables(TABLES_MAX)
                .table_elements(WIDGET_TABLE_MAX)
                .instances(INSTANCES_MAX)
                .build(),
            redraw_at: None,
        });
        store.limiter(|st| &mut st.limits);
        // The start section runs here, before a frame can draw: give it the same budget as a call.
        let _ = store.set_fuel(WIDGET_CALL_FUEL);
        let mut linker = Linker::new(&engine);
        linker.func_wrap(ABI, "rect", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, style: i32| -> Result<(), wasmi::Error> {
            charge(&mut caller, WIDGET_CHARGE_CALL)?;
            if !caller.data().overlay {
                return Ok(());
            }
            room_for_command(caller.data())?;
            let area = area_in_body(caller.data(), x, y, w, h);
            charge(&mut caller, area / WIDGET_CHARGE_RECT_PX)?;
            caller.data_mut().commands.push(DrawCmd::Rect { x, y, w, h, style });
            Ok(())
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "text", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, ptr: i32, len: i32, px: i32, center: i32| -> Result<(), wasmi::Error> {
            charge(&mut caller, WIDGET_CHARGE_CALL)?;
            if !caller.data().overlay {
                return Ok(());
            }
            room_for_command(caller.data())?;
            let text = pass_text(&mut caller, ptr, len, px, WIDGET_CHARGE_SQUARE_BYTES)?;
            caller.data_mut().commands.push(DrawCmd::Text { x, y, w, h, text, px, center: center != 0 });
            Ok(())
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "button", |mut caller: Caller<'_, HostState>, id: i32, x: i32, y: i32, w: i32, h: i32, ptr: i32, len: i32, style: i32| -> Result<(), wasmi::Error> {
            charge(&mut caller, WIDGET_CHARGE_CALL)?;
            if !caller.data().overlay {
                return Ok(());
            }
            room_for_command(caller.data())?;
            room_for_hit(caller.data())?;
            let area = area_in_body(caller.data(), x, y, w, h);
            charge(&mut caller, area / WIDGET_CHARGE_BUTTON_PX)?;
            let text = pass_text(&mut caller, ptr, len, WIDGET_CHARGE_LABEL_PX, u64::MAX)?;
            let st = caller.data_mut();
            st.commands.push(DrawCmd::Button { x, y, w, h, text, style });
            st.hits.push((id, Rect { x, y, w, h }));
            Ok(())
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "hit", |mut caller: Caller<'_, HostState>, id: i32, x: i32, y: i32, w: i32, h: i32| -> Result<(), wasmi::Error> {
            charge(&mut caller, WIDGET_CHARGE_CALL)?;
            let st = caller.data_mut();
            if st.overlay {
                room_for_hit(st)?;
                st.hits.push((id, Rect { x, y, w, h }));
            }
            Ok(())
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "status", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<(), wasmi::Error> {
            charge(&mut caller, WIDGET_CHARGE_CALL)?;
            if len.max(0) as usize > WIDGET_STATUS_MAX {
                return Err(past(caller.data(), "the bytes of the status line", WIDGET_STATUS_MAX));
            }
            charge(&mut caller, len.max(0) as u64 * WIDGET_CHARGE_BYTE)?;
            caller.data_mut().status = String::from_utf8_lossy(&read_mem(&mut caller, ptr, len)).into_owned();
            Ok(())
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "request_redraw", |mut caller: Caller<'_, HostState>, after_ms: i32| -> Result<(), wasmi::Error> {
            charge(&mut caller, WIDGET_CHARGE_CALL)?;
            // (a pass is drawing: a package that may not draw is not drawn again for the asking)
            if caller.data().overlay {
                let after_ms = after_ms.clamp(WIDGET_REDRAW_MIN_MS, WIDGET_REDRAW_MAX_MS) as u32;
                caller.data_mut().redraw_at = Some(now_ms().wrapping_add(after_ms));
            }
            Ok(())
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "setting", |mut caller: Caller<'_, HostState>, key_ptr: i32, key_len: i32, out_ptr: i32, out_cap: i32| -> Result<i32, wasmi::Error> {
            charge(&mut caller, WIDGET_CHARGE_CALL)?;
            // (no setting has a longer key: nothing is read for one)
            if key_len.max(0) as usize > crate::widgetpkg::WIDGET_SETTINGS_KEY_MAX {
                return Ok(0);
            }
            charge(&mut caller, key_len.max(0) as u64 * WIDGET_CHARGE_BYTE)?;
            let key = read_mem(&mut caller, key_ptr, key_len);
            let value = caller.data().settings.iter().find(|(k, _)| k.as_bytes() == key.as_slice()).map(|(_, v)| v.as_bytes().to_vec()).unwrap_or_default();
            charge(&mut caller, value.len().min(out_cap.max(0) as usize) as u64 * WIDGET_CHARGE_BYTE)?;
            Ok(write_mem(&mut caller, out_ptr, out_cap, &value))
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "state_get", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> Result<i32, wasmi::Error> {
            charge(&mut caller, WIDGET_CHARGE_CALL)?;
            if !caller.data().storage {
                return Ok(0);
            }
            let n = caller.data().state.len().min(cap.max(0) as usize);
            state_moved(caller.data_mut(), n)?;
            charge(&mut caller, n as u64 * WIDGET_CHARGE_BYTE)?;
            let state = caller.data().state[..n].to_vec();
            Ok(write_mem(&mut caller, ptr, cap, &state))
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "state_set", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> Result<(), wasmi::Error> {
            charge(&mut caller, WIDGET_CHARGE_CALL)?;
            if !caller.data().storage {
                return Ok(());
            }
            // (the first WIDGET_STATE_MAX bytes are the state: no more than those are read)
            let n = (len.max(0) as usize).min(WIDGET_STATE_MAX);
            state_moved(caller.data_mut(), n)?;
            charge(&mut caller, n as u64 * WIDGET_CHARGE_BYTE)?;
            let state = read_mem(&mut caller, ptr, n as i32);
            let st = caller.data_mut();
            st.state = state;
            st.dirty = true;
            Ok(())
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "now", |mut caller: Caller<'_, HostState>| -> Result<i64, wasmi::Error> {
            charge(&mut caller, WIDGET_CHARGE_CALL)?;
            Ok(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64))
        }).map_err(|e| e.to_string())?;
        let instance = linker.instantiate_and_start(&mut store, &module).map_err(|e| format!("The widget could not start: {e}"))?;
        if instance.get_memory(&store, "memory").is_none() {
            return Err("The widget has no memory export.".into());
        }
        Ok(WidgetRuntime { _engine: engine, store, instance, hits: Vec::new(), texts: Vec::new(), status: String::new(), error: None, used: 0 })
    }

    /// Calls `widget_draw` and renders its commands into `body`, and nowhere else on the canvas.
    pub fn draw(&mut self, c: &mut Canvas, body: Rect) {
        if self.error.is_some() {
            return;
        }
        let draw = match self.instance.get_typed_func::<(i32, i32, i32), ()>(&self.store, "widget_draw") {
            Ok(f) => f,
            Err(e) => { self.stopped(format!("The widget has no widget_draw export: {e}")); return; }
        };
        self.store.data_mut().status.clear();
        self.begin_call();
        self.store.data_mut().body = (body.w, body.h);
        let scale = ui_px(1000.0);
        let called = draw.call(&mut self.store, (body.w, body.h, scale));
        self.end_call();
        if let Err(e) = called {
            self.stopped(format!("The widget stopped: {e}"));
            return;
        }
        self.save();
        let commands = std::mem::take(&mut self.store.data_mut().commands);
        let hits = std::mem::take(&mut self.store.data_mut().hits);
        self.hits = hits.into_iter().filter_map(|(id, r)| hit_in_body(body, r).map(|r| (id, r))).collect();
        self.status = std::mem::take(&mut self.store.data_mut().status);
        self.texts.clear();
        // Nothing below draws outside the body: the canvas is clipped to it (and to what the caller
        // already clipped to) until the commands are drawn.
        let clip = c.clip;
        canvas_set_clip(c, rect_intersect(clip, body));
        canvas_fill(c, body, 255);
        for cmd in commands {
            match cmd {
                DrawCmd::Rect { x, y, w, h, style } => {
                    let r = on_screen(body, x, y, w, h);
                    let fill = match style { 1 => 210, _ => 255 };
                    ui_frame(c, r, ui_px(8.0), fill, true, 0);
                    ui_frame(c, r, ui_px(8.0), 0, false, ui_px(2.0));
                }
                DrawCmd::Text { x, y, w, h, text, px, center } => {
                    let r = on_screen(body, x, y, w, h);
                    let px = px.clamp(1, WIDGET_TEXT_PX_MAX) as f32;
                    ui_text(c, FontFace::Regular, px, px, r, &text, 1, center, 0);
                    self.texts.push(text);
                }
                DrawCmd::Button { x, y, w, h, text, style } => {
                    let r = on_screen(body, x, y, w, h);
                    let style = match style { 1 => ButtonStyle::Selected, 2 => ButtonStyle::Disabled, _ => ButtonStyle::Plain };
                    ui_button(c, r, IconKind::None, &text, style);
                    self.texts.push(text);
                }
            }
        }
        c.clip = clip;
    }

    /// Sends a tap in body-local coordinates. Returns whether the module was called.
    pub fn tap(&mut self, x: i32, y: i32) {
        if self.error.is_some() {
            return;
        }
        let id = self.hits.iter().find(|(_, r)| rect_contains(*r, x, y)).map_or(-1, |(id, _)| *id);
        let tap = match self.instance.get_typed_func::<(i32,), ()>(&self.store, "widget_tap") {
            Ok(f) => f,
            Err(_) => return,
        };
        self.begin_call();
        let called = tap.call(&mut self.store, (id,));
        self.end_call();
        if called.is_err() {
            self.stopped("The widget stopped while handling a tap.".into());
            return;
        }
        self.save();
    }

    /// Starts a call of the module: its instruction budget, and a pass with nothing recorded and
    /// nothing counted yet. What an earlier call left recorded (a tap that drew) is dropped: a pass
    /// holds its own commands only, so its limits are the most the host ever keeps.
    fn begin_call(&mut self) {
        let st = self.store.data_mut();
        st.redraw_at = None;
        st.commands.clear();
        st.hits.clear();
        st.text_bytes = 0;
        st.state_bytes = 0;
        st.read_bytes = 0;
        let _ = self.store.set_fuel(WIDGET_CALL_FUEL);
    }

    /// After a call: what it took from its budget, whether it ended or was stopped.
    fn end_call(&mut self) {
        self.used = WIDGET_CALL_FUEL.saturating_sub(self.store.get_fuel().unwrap_or(0));
    }

    /// What the last call of the module took from its budget: its own instructions and the charges for the
    /// host's work. At most WIDGET_CALL_FUEL, which a call that was stopped for it has used up.
    pub fn fuel_used(&self) -> u64 {
        self.used
    }

    /// The bytes the host read from the module's memory in the last call, all its imports together: a text's,
    /// a label's, the status line's, a setting's key, the state `state_set` kept. Never more than the limits let
    /// an import read, whatever lengths the module passed.
    pub fn bytes_read(&self) -> usize {
        self.store.data().read_bytes
    }

    /// A call that trapped (its instructions used up, a limit passed, a fault of the module's own):
    /// the frame shows `error` from now on, the module is not called again, what the call recorded
    /// is dropped, and the redraw it may have asked for is not kept waiting.
    fn stopped(&mut self, error: String) {
        self.error = Some(error);
        let st = self.store.data_mut();
        st.commands = Vec::new();
        st.hits = Vec::new();
        st.redraw_at = None;
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Reads the package's settings again after the settings page saved them.
    pub fn reload_settings(&mut self, id: &str) {
        let Some(m) = crate::widgetpkg::widget_package_installed_manifest(id) else { return };
        self.store.data_mut().settings = crate::widgetpkg::widget_settings_load(id, &m.settings);
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    /// The text drawn in the last frame, joined by `|`, for generic scenario probes.
    pub fn text(&self) -> String {
        self.texts.join("|")
    }

    /// A drawn hit region, for generic scenario taps.
    pub fn hit(&self, id: i32) -> Option<Rect> {
        self.hits.iter().find(|(hit, _)| *hit == id).map(|(_, r)| *r)
    }

    pub fn hit_count(&self) -> usize {
        self.hits.len()
    }

    /// Whether the module asked for a drawing pass that is due at `now`, by the app's clock (`now_ms`,
    /// which wraps: the longest delay is far under half its range).
    pub fn redraw_due(&self, now: u32) -> bool {
        self.error.is_none() && self.store.data().redraw_at.is_some_and(|at| now.wrapping_sub(at) as i32 >= 0)
    }

    /// `redraw_due`, and the request is taken: the editor's tick asks this once for a frame it then
    /// draws, so one request brings one drawing pass and one refresh, whatever the pass does.
    pub fn redraw_taken(&mut self, now: u32) -> bool {
        let due = self.redraw_due(now);
        if due {
            self.store.data_mut().redraw_at = None;
        }
        due
    }

    /// Writes the state file after a call that changed the state. A package without
    /// `storage:widget-local` has no state to change, and no file is made for it.
    fn save(&mut self) {
        if !self.store.data().storage || !self.store.data().dirty {
            return;
        }
        let st = self.store.data_mut();
        st.dirty = false;
        if let Some(dir) = st.state_path.rsplit_once('/').map(|(d, _)| d) {
            let _ = mkdir_p(dir);
        }
        if write_file_atomic(&st.state_path, &st.state) != 0 {
            st.dirty = true;
        }
    }
}

/// A command's rectangle on the screen. A module passes any numbers: the edges are summed in 64
/// bits and held within DRAW_REACH of the body, so no later sum overflows. A width or height that
/// is not positive stays so (an empty rectangle draws nothing).
fn on_screen(body: Rect, x: i32, y: i32, w: i32, h: i32) -> Rect {
    fn edge(origin: i32, len: i32, at: i64) -> i32 {
        (origin as i64 + at).clamp(origin as i64 - DRAW_REACH, origin as i64 + len.max(0) as i64 + DRAW_REACH) as i32
    }
    let (x0, x1) = (edge(body.x, body.w, x as i64), edge(body.x, body.w, x as i64 + w as i64));
    let (y0, y1) = (edge(body.y, body.h, y as i64), edge(body.y, body.h, y as i64 + h as i64));
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// The part of a hit region (body-local) that is inside the body, or None when no part is.
fn hit_in_body(body: Rect, r: Rect) -> Option<Rect> {
    let x0 = (r.x as i64).max(0);
    let y0 = (r.y as i64).max(0);
    let x1 = (r.x as i64 + r.w as i64).min(body.w as i64);
    let y1 = (r.y as i64 + r.h as i64).min(body.h as i64);
    (x1 > x0 && y1 > y0).then(|| Rect { x: x0 as i32, y: y0 as i32, w: (x1 - x0) as i32, h: (y1 - y0) as i32 })
}

/// Ends a call that went past a limit the way a call that used up its instructions ends: with the
/// trap `draw` and `tap` report as the widget having stopped. The log says which limit it was.
fn past(st: &HostState, limit: &str, max: usize) -> wasmi::Error {
    qlog!("widget %s: stopped, a call went past a limit: %s (%d)", &st.id, limit, max as i32);
    wasmi::Error::from(TrapCode::OutOfFuel)
}

/// Takes `n` from the call's instruction budget for work the host does for the call. When the budget cannot
/// pay, the call ends exactly as a call that used up its instructions ends: nothing is left of the budget,
/// and the trap is the interpreter's own for that.
fn charge(caller: &mut Caller<'_, HostState>, n: u64) -> Result<(), wasmi::Error> {
    let left = caller.get_fuel().unwrap_or(0);
    if n > left {
        let _ = caller.set_fuel(0);
        return Err(wasmi::Error::from(TrapCode::OutOfFuel));
    }
    let _ = caller.set_fuel(left - n);
    Ok(())
}

/// The pixels of a command's rectangle (body-local) that lie inside the body the call draws on.
fn area_in_body(st: &HostState, x: i32, y: i32, w: i32, h: i32) -> u64 {
    let body = Rect { x: 0, y: 0, w: st.body.0, h: st.body.1 };
    hit_in_body(body, Rect { x, y, w, h }).map_or(0, |r| r.w as u64 * r.h as u64)
}

/// The charge for drawing a text of `bytes` bytes `px` high, its bytes counted to `square_max` in the square.
fn text_charge(bytes: usize, px: i32, square_max: u64) -> u64 {
    let (n, h) = (bytes as u64, px.clamp(1, WIDGET_TEXT_PX_MAX) as u64);
    let m = n.min(square_max);
    n * h * h.max(WIDGET_CHARGE_TALL_PX) * WIDGET_CHARGE_TEXT / WIDGET_CHARGE_TALL_PX + WIDGET_CHARGE_SQUARE * m * m
}

fn room_for_command(st: &HostState) -> Result<(), wasmi::Error> {
    if st.commands.len() >= WIDGET_PASS_COMMANDS_MAX {
        return Err(past(st, "the drawing commands of a pass", WIDGET_PASS_COMMANDS_MAX));
    }
    Ok(())
}

fn room_for_hit(st: &HostState) -> Result<(), wasmi::Error> {
    if st.hits.len() >= WIDGET_PASS_HITS_MAX {
        return Err(past(st, "the hit regions of a pass", WIDGET_PASS_HITS_MAX));
    }
    Ok(())
}

/// A command's text, read once its length is within the limits of one text and of the pass and the call's
/// budget has paid for drawing it `px` high (`text_charge`).
fn pass_text(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32, px: i32, square_max: u64) -> Result<String, wasmi::Error> {
    let n = len.max(0) as usize;
    let st = caller.data_mut();
    if n > WIDGET_TEXT_MAX {
        return Err(past(st, "the bytes of one text", WIDGET_TEXT_MAX));
    }
    if st.text_bytes + n > WIDGET_PASS_TEXT_MAX {
        return Err(past(st, "the bytes of the texts of a pass", WIDGET_PASS_TEXT_MAX));
    }
    st.text_bytes += n;
    charge(caller, text_charge(n, px, square_max))?;
    Ok(String::from_utf8_lossy(&read_mem(caller, ptr, len)).into_owned())
}

/// Counts `n` bytes of state about to be copied in this call.
fn state_moved(st: &mut HostState, n: usize) -> Result<(), wasmi::Error> {
    if st.state_bytes + n > WIDGET_CALL_STATE_MAX {
        return Err(past(st, "the bytes of state moved in a call", WIDGET_CALL_STATE_MAX));
    }
    st.state_bytes += n;
    Ok(())
}

/// `len` bytes of the module's memory from `ptr`, or as many as the memory has there. Every read of the
/// module's memory is this one, and the bytes it copies are counted (`WidgetRuntime::bytes_read`).
fn read_mem(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Vec<u8> {
    if ptr < 0 || len <= 0 {
        return Vec::new();
    }
    let Some(memory) = caller.get_export("memory").and_then(Extern::into_memory) else { return Vec::new() };
    let data = memory.data(&*caller);
    let start = ptr as usize;
    let end = start.saturating_add(len as usize).min(data.len());
    if start >= end { return Vec::new() }
    let bytes = data[start..end].to_vec();
    caller.data_mut().read_bytes += bytes.len();
    bytes
}

fn write_mem(caller: &mut Caller<'_, HostState>, ptr: i32, cap: i32, bytes: &[u8]) -> i32 {
    if ptr < 0 || cap <= 0 {
        return 0;
    }
    let Some(memory) = caller.get_export("memory").and_then(Extern::into_memory) else { return 0 };
    let data = memory.data_mut(caller);
    let start = ptr as usize;
    if start >= data.len() {
        return 0;
    }
    let n = bytes.len().min(cap as usize).min(data.len() - start);
    data[start..start + n].copy_from_slice(&bytes[..n]);
    n as i32
}
