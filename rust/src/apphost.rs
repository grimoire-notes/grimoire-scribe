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
//! `app_stop` is called exactly once when the app is leaving, whether by gesture, `close()`, an
//! update, or removal. It is the safe place to save state.
//!
//! Imports (`grimoire`): drawing and Grimoire's global UI elements, state and settings, time and
//! date, battery and Wi-Fi state, screen geometry, and `close()`. All strings are UTF-8 in the
//! module's memory. See docs/APPS.md for the full list.

use crate::globalui::{gui_app_tile, gui_chip, gui_fab, gui_select, gui_status_bar, gui_status_h, gui_surface, gui_text_field};
use crate::prelude::*;
use std::time::{Duration, Instant};
use std::process::Command;
use wasmi::{Caller, Config, Engine, Extern, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

const ABI: &str = "grimoire";
const ARTIFACT_MAX: usize = 32 * 1024 * 1024;
const STATE_MAX: usize = 256 * 1024;
const STATE_FILE: &str = "state/runtime.bin";
const FUEL_PER_CALL: u64 = 30_000_000;
const MEMORY_MAX: usize = 48 * 1024 * 1024;

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
}

struct HostState {
    commands: Vec<DrawCmd>,
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
    redraw_at: Option<Instant>,
    screen_w: i32,
    screen_h: i32,
    clock_12h: bool,
    /// The manifest's `network:http` capability. Without it HTTP imports refuse.
    network_http: bool,
    /// The one asynchronous curl request an app may have in flight.
    http: Option<HttpJob>,
}

/// The limits of one app HTTP request (docs/APPS.md, HTTP).
const HTTP_HEADER_LINES_MAX: usize = 32;
const HTTP_HEADER_LINE_MAX: usize = 2048;

/// One app HTTP request, run by curl in the background. The app polls it from
/// `request_redraw` frames, so no network contact ever blocks drawing or touch.
struct HttpJob {
    child: std::process::Child,
    body_path: String,
    status_path: String,
    err_path: String,
    done: bool,
    code: i32,
    body: Vec<u8>,
    error: String,
}

impl Drop for HttpJob {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        let _ = std::fs::remove_file(&self.body_path);
        let _ = std::fs::remove_file(&self.status_path);
        let _ = std::fs::remove_file(&self.err_path);
        if let Some(body) = self.body_path.strip_suffix(".body") {
            let _ = std::fs::remove_file(format!("{body}.request"));
        }
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
    last_draw: Option<Instant>,
    launched_at: Option<Instant>,
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

    pub fn reload_settings(&mut self) {
        if let Some(m) = crate::apppkg::app_package_installed_manifest(&self.id) {
            if let Some(r) = self.runtime.as_mut() {
                r.store.data_mut().settings = crate::apppkg::app_settings_load(&self.id, &m.settings);
            }
        }
    }

    fn draw(&mut self, c: &mut Canvas, body: Rect, status_h: i32) {
        let now = Instant::now();
        let elapsed = self.last_draw.map_or(0, |at| now.duration_since(at).as_millis().min(i32::MAX as u128) as i32);
        self.last_draw = Some(now);
        let Some(r) = self.runtime.as_mut() else { return };
        r.draw(c, body, status_h, elapsed);
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
    let Some(m) = crate::apppkg::app_package_installed_manifest(id) else {
        app_toast(a, "That app is no longer installed.", 3000);
        return false;
    };
    let Some(it) = crate::apppkg::app_package_installed(id) else {
        app_toast(a, "That app is no longer installed.", 3000);
        return false;
    };
    if m.type_ != "app" {
        app_toast(a, "That package is not an app.", 3000);
        return false;
    }
    match AppRuntime::new(&it, &m, a.w, a.h, a.settings.clock_12h) {
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
                last_draw: Some(Instant::now()),
                launched_at: Some(Instant::now()),
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
            a.app_view.last_draw = Some(Instant::now());
            let App { screen: c, app_view, .. } = a;
            app_view.draw(c, app_view.body, gui_status_h());
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
        gui_status_bar(c, aw, wall_time());
    }
    if !rect_is_empty(body) {
        app_view.body = body;
        app_view.draw(c, body, gui_status_h());
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
        let mut r = a.app_view.runtime.take();
        if let Some(r) = r.as_mut() {
            r.key(ev.value, ev.key_mods as i32, ev.type_ == InType::Key);
        }
        a.app_view.runtime = r;
        if apphost_after_call(a) {
            return true;
        }
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
    let source = if matches!(ev.type_, InType::PenDown | InType::PenMove | InType::PenUp) { 1 } else { 0 };        let mut r = a.app_view.runtime.take();
        if let Some(r) = r.as_mut() {
            r.pointer(ph, x, y, source);
        }
        a.app_view.runtime = r;
        apphost_after_call(a);
        true
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

fn apphost_after_call(a: &mut App) -> bool {
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
    if let Some(e) = error {
        a.app_view.error = e;
        app_toast(a, &a.app_view.error.clone(), 5000);
        apphost_close(a);
        return true;
    }
    if close {
        apphost_close(a);
        return true;
    }
    // The module's visible work changed: draw it once now. Redraw requests pace the rest.
    apphost_draw(a);
    app_invalidate(a, Rect::new(0, 0, a.w, a.h), RefreshMode::Ui);
    false
}

/// Ticks an app's timers and its requested redraws. Called from the Library tick.
pub fn apphost_tick(a: &mut App, now: u32) {
    if !a.app_view.open {
        return;
    }
    a.app_view.tick();
    let due = a.app_view.runtime.as_ref().is_some_and(|r| r.redraw_due(Instant::now()));
    if due {
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
    let _ = now;
}

/// The runner's control point for an app: the body's centre (apps receive absolute taps too).
pub fn apphost_control_point(a: &App, name: &str) -> Option<(f32, f32)> {
    if name != "body" || !a.app_view.open {
        return None;
    }
    let b = app_body(a);
    Some(((b.x + b.w / 2) as f32, (b.y + b.h / 2) as f32))
}

// ---- the runtime ----

impl AppRuntime {
    fn new(it: &WidgetInstall, m: &WidgetPackage, screen_w: i32, screen_h: i32, clock_12h: bool) -> Result<Self, String> {
        let path = format!("{}/{}", crate::apppkg::app_package_dir(&it.id), m.artifact.path);
        let bytes = read_file_max(&path, ARTIFACT_MAX).ok_or("The app's artifact could not be read.")?;
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, &bytes[..]).map_err(|e| format!("The app's WebAssembly is invalid: {e}"))?;
        let state_path = format!("{}/{STATE_FILE}", crate::apppkg::app_package_dir(&it.id));
        let state = read_file_max(&state_path, STATE_MAX).unwrap_or_default();
        let settings = crate::apppkg::app_settings_load(&it.id, &m.settings);
        let mut store = Store::new(&engine, HostState {
            commands: Vec::new(),
            state,
            state_path,
            settings,
            close_requested: false,
            status_bar: m.spec.status_bar,
            status: String::new(),
            texts: Vec::new(),
            toast: None,
            dirty: false,
            limits: StoreLimitsBuilder::new().memory_size(MEMORY_MAX).build(),
            redraw_at: None,
            screen_w,
            screen_h,
            clock_12h,
            network_http: m.capabilities.iter().any(|c| c == "network:http"),
            http: None,
        });
        store.limiter(|st| &mut st.limits);
        let _ = store.set_fuel(FUEL_PER_CALL);
        let mut linker = Linker::new(&engine);
        install_imports(&mut linker)?;
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
        })
    }

    fn draw(&mut self, c: &mut Canvas, body: Rect, status_h: i32, elapsed_ms: i32) {
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
        self.store.data_mut().commands.clear();
        self.store.data_mut().status.clear();
        self.store.data_mut().texts.clear();
        let _ = self.store.set_fuel(FUEL_PER_CALL);
        let scale = ui_px(1000.0);
        if let Err(e) = draw.call(&mut self.store, (body.w, body.h, scale, elapsed_ms)) {
            self.error = Some(format!("The app stopped: {e}"));
            return;
        }
        self.save();
        let commands = std::mem::take(&mut self.store.data_mut().commands);
        self.status = std::mem::take(&mut self.store.data_mut().status);
        self.texts = std::mem::take(&mut self.store.data_mut().texts);
        let _ = status_h;
        canvas_fill(c, body, 255);
        let saved_clip = c.clip;
        for cmd in commands {
            match cmd {
                DrawCmd::Fill(r, gray) => canvas_fill(c, abs(body, r), gray),
                DrawCmd::Outline(r, t, gray) => canvas_rect_outline(c, abs(body, r), t, gray),
                DrawCmd::Round(r, radius, gray, filled, t) => {
                    canvas_round_rect(c, abs(body, r), radius, gray, filled, t)
                }
                DrawCmd::Line(x0, y0, x1, y1, width, gray) => {
                    canvas_line_aa(c, body.x as f32 + x0, body.y as f32 + y0, body.x as f32 + x1, body.y as f32 + y1, width, gray)
                }
                DrawCmd::Text(r, text, px, face, center, gray) => {
                    let r = abs(body, r);
                    ui_text(c, face, px, px, r, &text, 1, center, gray);
                    self.texts.push(text);
                }
                DrawCmd::TextAt(x, y, text, px, face, gray) => {
                    font_draw(c, face, px, body.x + x, body.y + y, &text, gray);
                    self.texts.push(text);
                }
                DrawCmd::Icon(name, r, gray) => ui_icon(c, widget_icon(&name), abs(body, r), gray),
                DrawCmd::Button(r, label, icon, style) => {
                    ui_button(c, abs(body, r), if icon.is_empty() { IconKind::None } else { widget_icon(&icon) }, &label, button_style(style));
                    self.texts.push(label);
                }
                DrawCmd::Chip(r, label, selected) => {
                    gui_chip(c, abs(body, r), &label, selected);
                    self.texts.push(label);
                }
                DrawCmd::Row(r, icon, label, style, switch) => {
                    let mut row = Row::new(if icon.is_empty() { IconKind::None } else { widget_icon(&icon) }, &label);
                    row.style = button_style(style);
                    if switch >= 0 {
                        row.switch = Some(switch != 0);
                    }
                    ui_row(c, abs(body, r), &row);
                    self.texts.push(label);
                }
                DrawCmd::Surface(r, filled, thick) => gui_surface(c, abs(body, r), filled, thick),
                DrawCmd::Select(r, caption, value) => {
                    gui_select(c, abs(body, r), &caption, &value);
                    self.texts.push(value);
                }
                DrawCmd::AppTile(r, icon, label, selected) => {
                    let icon = AppIcon::glyph(widget_icon(&icon));
                    gui_app_tile(c, abs(body, r), &icon, &label, selected);
                    self.texts.push(label);
                }
                DrawCmd::Fab(r, icon) => gui_fab(c, abs(body, r), widget_icon(&icon)),
                DrawCmd::Card(r, style) => ui_card(c, abs(body, r), button_style(style)),
                DrawCmd::TextField(r, icon, text, hint) => {
                    gui_text_field(c, abs(body, r), if icon.is_empty() { IconKind::None } else { widget_icon(&icon) }, &text, &hint);
                    self.texts.push(text);
                    self.texts.push(hint);
                }
                DrawCmd::Clip(r) => canvas_set_clip(c, abs(body, r)),
                DrawCmd::ClipClear => canvas_set_clip(c, saved_clip),
            }
        }
        c.clip = saved_clip;
    }

    fn pointer(&mut self, phase: i32, x: i32, y: i32, source: i32) {
        if self.error.is_some() {
            return;
        }
        let Some(f) = self.instance.get_typed_func::<(i32, i32, i32, i32), ()>(&self.store, "app_pointer").ok() else { return };
        let _ = self.store.set_fuel(FUEL_PER_CALL);
        if f.call(&mut self.store, (phase, x, y, source)).is_err() {
            self.error = Some("The app stopped while handling a touch.".into());
            return;
        }
        self.save();
    }

    fn key(&mut self, code: i32, mods: i32, down: bool) {
        if self.error.is_some() {
            return;
        }
        let Some(f) = self.instance.get_typed_func::<(i32, i32, i32), ()>(&self.store, "app_key").ok() else { return };
        let _ = self.store.set_fuel(FUEL_PER_CALL);
        if f.call(&mut self.store, (code, mods, i32::from(down))).is_err() {
            self.error = Some("The app stopped while handling a key.".into());
            return;
        }
        self.save();
    }

    fn export_status_bar(&mut self) -> Option<bool> {
        let f = self.instance.get_typed_func::<(), i32>(&self.store, "app_status_bar").ok()?;
        let _ = self.store.set_fuel(FUEL_PER_CALL);
        f.call(&mut self.store, ()).ok().map(|v| v != 0)
    }

    fn stop(&mut self) {
        if self.error.is_none() {
            if let Ok(f) = self.instance.get_typed_func::<(), ()>(&self.store, "app_stop") {
                let _ = self.store.set_fuel(FUEL_PER_CALL);
                let _ = f.call(&mut self.store, ());
            }
        }
        self.save();
    }

    fn tick(&mut self) {
        if self.error.is_some() {
            return;
        }
        // The host has no autonomous app callbacks yet; redraw requests are polled by AppRun.
    }

    fn save(&mut self) {
        let st = self.store.data_mut();
        if !st.dirty {
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

    fn redraw_due(&self, now: Instant) -> bool {
        self.store.data().redraw_at.is_some_and(|at| at <= now)
    }
}

fn abs(body: Rect, r: Rect) -> Rect {
    Rect { x: body.x + r.x, y: body.y + r.y, w: r.w, h: r.h }
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

    // ---- drawing ----
    linker.func_wrap(ABI, "fill_rect", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, gray: i32| {
        caller.data_mut().commands.push(DrawCmd::Fill(Rect::new(x, y, w, h), gray.clamp(0, 255) as u8));
    }).map_err(e)?;
    linker.func_wrap(ABI, "outline_rect", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, t: i32, gray: i32| {
        caller.data_mut().commands.push(DrawCmd::Outline(Rect::new(x, y, w, h), t, gray.clamp(0, 255) as u8));
    }).map_err(e)?;
    linker.func_wrap(ABI, "round_rect", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, radius: i32, gray: i32, filled: i32, t: i32| {
        caller.data_mut().commands.push(DrawCmd::Round(Rect::new(x, y, w, h), radius, gray.clamp(0, 255) as u8, filled != 0, t));
    }).map_err(e)?;
    linker.func_wrap(ABI, "line", |mut caller: Caller<'_, HostState>, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, gray: i32| {
        caller.data_mut().commands.push(DrawCmd::Line(x0, y0, x1, y1, width.max(0.1), gray.clamp(0, 255) as u8));
    }).map_err(e)?;
    linker.func_wrap(ABI, "text", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, ptr: i32, len: i32, px_milli: i32, face_: i32, center: i32, gray: i32| {
        let text = String::from_utf8_lossy(&read_mem(&caller, ptr, len)).into_owned();
        caller.data_mut().commands.push(DrawCmd::Text(Rect::new(x, y, w, h), text, (px_milli.max(1) as f32) / 1000.0, face(face_), center != 0, gray.clamp(0, 255) as u8));
    }).map_err(e)?;
    linker.func_wrap(ABI, "text_at", |mut caller: Caller<'_, HostState>, x: i32, baseline: i32, ptr: i32, len: i32, px_milli: i32, face_: i32, gray: i32| {
        let text = String::from_utf8_lossy(&read_mem(&caller, ptr, len)).into_owned();
        caller.data_mut().commands.push(DrawCmd::TextAt(x, baseline, text, (px_milli.max(1) as f32) / 1000.0, face(face_), gray.clamp(0, 255) as u8));
    }).map_err(e)?;
    linker.func_wrap(ABI, "text_width", |caller: Caller<'_, HostState>, ptr: i32, len: i32, px_milli: i32, face_: i32| -> i32 {
        let text = String::from_utf8_lossy(&read_mem(&caller, ptr, len)).into_owned();
        font_measure(face(face_), (px_milli.max(1) as f32) / 1000.0, &text)
    }).map_err(e)?;
    linker.func_wrap(ABI, "text_height", |_caller: Caller<'_, HostState>, px_milli: i32, face_: i32| -> i32 {
        let px = (px_milli.max(1) as f32) / 1000.0;
        font_ascent(face(face_), px) + font_descent(face(face_), px)
    }).map_err(e)?;
    linker.func_wrap(ABI, "icon", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32, x: i32, y: i32, w: i32, h: i32, gray: i32| {
        let name = String::from_utf8_lossy(&read_mem(&caller, ptr, len)).into_owned();
        caller.data_mut().commands.push(DrawCmd::Icon(name, Rect::new(x, y, w, h), gray.clamp(0, 255) as u8));
    }).map_err(e)?;
    linker.func_wrap(ABI, "button", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, label_ptr: i32, label_len: i32, icon_ptr: i32, icon_len: i32, style: i32| {
        let label = String::from_utf8_lossy(&read_mem(&caller, label_ptr, label_len)).into_owned();
        let icon = String::from_utf8_lossy(&read_mem(&caller, icon_ptr, icon_len)).into_owned();
        caller.data_mut().commands.push(DrawCmd::Button(Rect::new(x, y, w, h), label, icon, style));
    }).map_err(e)?;
    linker.func_wrap(ABI, "chip", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, ptr: i32, len: i32, selected: i32| {
        let label = String::from_utf8_lossy(&read_mem(&caller, ptr, len)).into_owned();
        caller.data_mut().commands.push(DrawCmd::Chip(Rect::new(x, y, w, h), label, selected != 0));
    }).map_err(e)?;
    linker.func_wrap(ABI, "row", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, icon_ptr: i32, icon_len: i32, label_ptr: i32, label_len: i32, style: i32, switch: i32| {
        let icon = String::from_utf8_lossy(&read_mem(&caller, icon_ptr, icon_len)).into_owned();
        let label = String::from_utf8_lossy(&read_mem(&caller, label_ptr, label_len)).into_owned();
        caller.data_mut().commands.push(DrawCmd::Row(Rect::new(x, y, w, h), icon, label, style, switch));
    }).map_err(e)?;
    linker.func_wrap(ABI, "surface", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, filled: i32, thick: i32| {
        caller.data_mut().commands.push(DrawCmd::Surface(Rect::new(x, y, w, h), filled != 0, thick));
    }).map_err(e)?;
    linker.func_wrap(ABI, "select", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, cap_ptr: i32, cap_len: i32, val_ptr: i32, val_len: i32| {
        let caption = String::from_utf8_lossy(&read_mem(&caller, cap_ptr, cap_len)).into_owned();
        let value = String::from_utf8_lossy(&read_mem(&caller, val_ptr, val_len)).into_owned();
        caller.data_mut().commands.push(DrawCmd::Select(Rect::new(x, y, w, h), caption, value));
    }).map_err(e)?;
    linker.func_wrap(ABI, "app_tile", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, icon_ptr: i32, icon_len: i32, label_ptr: i32, label_len: i32, selected: i32| {
        let icon = String::from_utf8_lossy(&read_mem(&caller, icon_ptr, icon_len)).into_owned();
        let label = String::from_utf8_lossy(&read_mem(&caller, label_ptr, label_len)).into_owned();
        caller.data_mut().commands.push(DrawCmd::AppTile(Rect::new(x, y, w, h), icon, label, selected != 0));
    }).map_err(e)?;
    linker.func_wrap(ABI, "fab", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, icon_ptr: i32, icon_len: i32| {
        let icon = String::from_utf8_lossy(&read_mem(&caller, icon_ptr, icon_len)).into_owned();
        caller.data_mut().commands.push(DrawCmd::Fab(Rect::new(x, y, w, h), icon));
    }).map_err(e)?;
    linker.func_wrap(ABI, "card", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, style: i32| {
        caller.data_mut().commands.push(DrawCmd::Card(Rect::new(x, y, w, h), style));
    }).map_err(e)?;
    linker.func_wrap(ABI, "text_field", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, icon_ptr: i32, icon_len: i32, text_ptr: i32, text_len: i32, hint_ptr: i32, hint_len: i32| {
        let icon = String::from_utf8_lossy(&read_mem(&caller, icon_ptr, icon_len)).into_owned();
        let text = String::from_utf8_lossy(&read_mem(&caller, text_ptr, text_len)).into_owned();
        let hint = String::from_utf8_lossy(&read_mem(&caller, hint_ptr, hint_len)).into_owned();
        caller.data_mut().commands.push(DrawCmd::TextField(Rect::new(x, y, w, h), icon, text, hint));
    }).map_err(e)?;
    linker.func_wrap(ABI, "clip", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32| {
        caller.data_mut().commands.push(DrawCmd::Clip(Rect::new(x, y, w, h)));
    }).map_err(e)?;
    linker.func_wrap(ABI, "clip_clear", |mut caller: Caller<'_, HostState>| {
        caller.data_mut().commands.push(DrawCmd::ClipClear);
    }).map_err(e)?;

    // ---- state, settings, system ----
    linker.func_wrap(ABI, "state_get", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> i32 {
        let state = caller.data().state.clone();
        write_mem(&mut caller, ptr, cap, &state)
    }).map_err(e)?;
    linker.func_wrap(ABI, "state_set", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
        let state = read_mem(&caller, ptr, len);
        let st = caller.data_mut();
        st.state = state.into_iter().take(STATE_MAX).collect();
        st.dirty = true;
    }).map_err(e)?;
    linker.func_wrap(ABI, "setting", |mut caller: Caller<'_, HostState>, key_ptr: i32, key_len: i32, out_ptr: i32, out_cap: i32| -> i32 {
        let key = read_mem(&caller, key_ptr, key_len);
        let value = caller.data().settings.iter().find(|(k, _)| k.as_bytes() == key.as_slice()).map(|(_, v)| v.as_bytes().to_vec()).unwrap_or_default();
        write_mem(&mut caller, out_ptr, out_cap, &value)
    }).map_err(e)?;
    linker.func_wrap(ABI, "status", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
        caller.data_mut().status = String::from_utf8_lossy(&read_mem(&caller, ptr, len)).into_owned();
    }).map_err(e)?;
    linker.func_wrap(ABI, "request_redraw", |mut caller: Caller<'_, HostState>, after_ms: i32| {
        let ms = after_ms.clamp(16, 86_400_000) as u64;
        caller.data_mut().redraw_at = Some(Instant::now() + Duration::from_millis(ms));
    }).map_err(e)?;
    linker.func_wrap(ABI, "close", |mut caller: Caller<'_, HostState>| {
        caller.data_mut().close_requested = true;
    }).map_err(e)?;
    linker.func_wrap(ABI, "show_status_bar", |mut caller: Caller<'_, HostState>, on: i32| {
        caller.data_mut().status_bar = on != 0;
    }).map_err(e)?;
    linker.func_wrap(ABI, "toast", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32, ms: i32| {
        let text = String::from_utf8_lossy(&read_mem(&caller, ptr, len)).into_owned();
        caller.data_mut().toast = Some((text, ms.clamp(500, 30_000) as u32));
    }).map_err(e)?;
    linker.func_wrap(ABI, "now", || -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
    }).map_err(e)?;
    linker.func_wrap(ABI, "now_ms", || -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
    }).map_err(e)?;
    linker.func_wrap(ABI, "time_text", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> i32 {
        let mut s = String::new();
        format_clock_short(wall_time(), caller.data().clock_12h, &mut s, 64);
        write_mem(&mut caller, ptr, cap, s.as_bytes())
    }).map_err(e)?;
    linker.func_wrap(ABI, "date_text", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> i32 {
        let tm = local_tm(wall_time());
        let s = format!("{:04}-{:02}-{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday);
        write_mem(&mut caller, ptr, cap, s.as_bytes())
    }).map_err(e)?;
    linker.func_wrap(ABI, "battery_percent", || -> i32 {
        battery_from_sysfs("/sys/class/power_supply").map_or(-1, |b| b.percent)
    }).map_err(e)?;
    linker.func_wrap(ABI, "battery_charging", || -> i32 {
        battery_from_sysfs("/sys/class/power_supply").is_some_and(|b| b.charging) as i32
    }).map_err(e)?;
    linker.func_wrap(ABI, "wifi_connected", || -> i32 {
        wifi_status().and_then(|s| s.connected).map_or(-1, i32::from)
    }).map_err(e)?;
    linker.func_wrap(ABI, "wifi_ssid", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> i32 {
        let name = wifi_current().unwrap_or_default();
        write_mem(&mut caller, ptr, cap, &name)
    }).map_err(e)?;
    linker.func_wrap(ABI, "screen_width", |caller: Caller<'_, HostState>| -> i32 { caller.data().screen_w }).map_err(e)?;
    linker.func_wrap(ABI, "screen_height", |caller: Caller<'_, HostState>| -> i32 { caller.data().screen_h }).map_err(e)?;
    linker.func_wrap(ABI, "ui_scale_milli", || -> i32 { ui_px(1000.0) }).map_err(e)?;
    linker.func_wrap(ABI, "theme", || -> i32 { matches!(ui_theme(), UiTheme::Classic) as i32 }).map_err(e)?;
    linker.func_wrap(ABI, "status_bar_height", || -> i32 { gui_status_h() }).map_err(e)?;

    // ---- HTTP (network:http) ----
    // An app starts one request and polls it. The request runs in a child curl while the app keeps
    // drawing; `request_redraw` is how it asks to be called again.
    linker.func_wrap(ABI, "http_start", |mut caller: Caller<'_, HostState>, method_ptr: i32, method_len: i32, url_ptr: i32, url_len: i32, headers_ptr: i32, headers_len: i32, body_ptr: i32, body_len: i32| -> i32 {
        let method = String::from_utf8_lossy(&read_mem(&caller, method_ptr, method_len)).into_owned();
        let url = String::from_utf8_lossy(&read_mem(&caller, url_ptr, url_len)).into_owned();
        let headers = String::from_utf8_lossy(&read_mem(&caller, headers_ptr, headers_len)).into_owned();
        let body = read_mem(&caller, body_ptr, body_len);
        caller.data_mut().http_start(&method, &url, &headers, &body)
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_poll", |mut caller: Caller<'_, HostState>| -> i32 {
        caller.data_mut().http_poll()
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_status", |caller: Caller<'_, HostState>| -> i32 {
        caller.data().http.as_ref().map_or(-1, |j| j.code)
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_body_len", |caller: Caller<'_, HostState>| -> i32 {
        caller.data().http.as_ref().map_or(0, |j| j.body.len() as i32)
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_body", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> i32 {
        let body = caller.data().http.as_ref().map_or_else(Vec::new, |j| j.body.clone());
        write_mem(&mut caller, ptr, cap, &body)
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_error", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> i32 {
        let text = caller.data().http.as_ref().map_or_else(String::new, |j| j.error.clone());
        write_mem(&mut caller, ptr, cap, text.as_bytes())
    }).map_err(e)?;
    linker.func_wrap(ABI, "http_cancel", |mut caller: Caller<'_, HostState>| {
        caller.data_mut().http = None;
    }).map_err(e)?;
    Ok(())
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

impl HostState {
    fn http_start(&mut self, method: &str, url: &str, headers: &str, body: &[u8]) -> i32 {
        if !self.network_http {
            return -2;
        }
        if self.http.as_ref().is_some_and(|j| !j.done) {
            return -3;
        }
        let method = method.trim().to_ascii_uppercase();
        if method.is_empty()
            || method.len() > 16
            || !method.bytes().all(|b| b.is_ascii_uppercase())
            || !(url.starts_with("https://") || url.starts_with("http://"))
            || url.len() > 4096
            || headers.len() > 32 * 1024
            || body.len() > 2 * 1024 * 1024
        {
            return -1;
        }
        let Some(header_lines) = http_header_lines(headers) else { return -1 };
        self.http = None;
        let Some(dir) = self.state_path.rsplit_once('/').map(|(d, _)| d) else { return -1 };
        if mkdir_p(&format!("{dir}/http")) != 0 {
            return -1;
        }
        let tick = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let base = format!("{dir}/http/{}-{tick}", std::process::id());
        let body_path = format!("{base}.body");
        let status_path = format!("{base}.status");
        let err_path = format!("{base}.err");
        let request_path = format!("{base}.request");
        if !body.is_empty() && std::fs::write(&request_path, body).is_err() {
            return -1;
        }
        let Ok(status_file) = std::fs::File::create(&status_path) else { return -1 };
        let Ok(err_file) = std::fs::File::create(&err_path) else { return -1 };
        let mut cmd = Command::new("curl");
        cmd.arg("-q").arg("-sS").arg("-L").arg("--max-time").arg("30")
            .arg("-X").arg(&method)
            .arg("-o").arg(&body_path)
            .arg("-w").arg("%{http_code}")
            .arg("-H").arg("User-Agent: Grimoire-App/2.3");
        for line in &header_lines {
            cmd.arg("-H").arg(line);
        }
        if !body.is_empty() {
            cmd.arg("--data-binary").arg(format!("@{request_path}"));
        }
        cmd.arg(url).stdin(std::process::Stdio::null()).stdout(status_file).stderr(err_file);
        match cmd.spawn() {
            Ok(child) => {
                self.http = Some(HttpJob {
                    child,
                    body_path,
                    status_path,
                    err_path,
                    done: false,
                    code: -1,
                    body: Vec::new(),
                    error: String::new(),
                });
                0
            }
            Err(e) => {
                let _ = std::fs::remove_file(&body_path);
                let _ = std::fs::remove_file(&status_path);
                let _ = std::fs::remove_file(&err_path);
                let _ = std::fs::remove_file(&request_path);
                self.http = None;
                let _ = e;
                -1
            }
        }
    }

    fn http_poll(&mut self) -> i32 {
        let Some(job) = self.http.as_mut() else { return -1 };
        if job.done {
            return 1;
        }
        match job.child.try_wait() {
            Ok(Some(status)) => {
                job.done = true;
                job.code = read_file_max(&job.status_path, 32)
                    .and_then(|v| String::from_utf8_lossy(&v).trim().parse::<i32>().ok())
                    .unwrap_or(-1);
                job.body = read_file_max(&job.body_path, 8 * 1024 * 1024).unwrap_or_default();
                if !status.success() && job.body.is_empty() {
                    let detail = read_file_max(&job.err_path, 4096)
                        .map(|v| String::from_utf8_lossy(&v).split_whitespace().collect::<Vec<_>>().join(" "))
                        .unwrap_or_default();
                    job.error = if detail.is_empty() { format!("curl exited {:?}", status.code()) } else { detail };
                    job.code = -1;
                }
                1
            }
            Ok(None) => 0,
            Err(e) => {
                job.done = true;
                job.error = format!("Could not wait for curl: {e}");
                job.code = -1;
                1
            }
        }
    }
}

fn read_mem(caller: &Caller<'_, HostState>, ptr: i32, len: i32) -> Vec<u8> {
    if ptr < 0 || len <= 0 {
        return Vec::new();
    }
    let Some(memory) = caller.get_export("memory").and_then(Extern::into_memory) else { return Vec::new() };
    let data = memory.data(caller);
    let start = ptr as usize;
    let end = start.saturating_add(len as usize).min(data.len());
    if start >= end {
        return Vec::new();
    }
    data[start..end].to_vec()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_bottom_to_middle_two_finger_swipe_closes() {
        assert!(gesture_matches(900.0, 2300.0, 930.0, 1200.0, 1860, 2480));
        assert!(!gesture_matches(900.0, 1500.0, 930.0, 1200.0, 1860, 2480));
        assert!(!gesture_matches(900.0, 2300.0, 1700.0, 1200.0, 1860, 2480));
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
            "(X-A): 1",
            "\"X-A\": 1",
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
        assert_eq!(http_header_lines("X-A: a\rb\n"), None);
        let most = "X-A: 1\n\n".repeat(HTTP_HEADER_LINES_MAX);
        assert_eq!(http_header_lines(&most).map(|l| l.len()), Some(HTTP_HEADER_LINES_MAX));
        assert_eq!(http_header_lines(&format!("{most}X-A: 1")), None);
        let longest = format!("X-Long: {}", "a".repeat(HTTP_HEADER_LINE_MAX - 8));
        assert_eq!(http_header_lines(&format!("  {longest}  \r\n")), Some(vec![longest.as_str()]));
        assert_eq!(http_header_lines(&format!("{longest}a")), None);
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
            commands: Vec::new(),
            state: Vec::new(),
            state_path: format!("{dir}/{STATE_FILE}"),
            settings: Vec::new(),
            close_requested: false,
            status_bar: false,
            status: String::new(),
            texts: Vec::new(),
            toast: None,
            dirty: false,
            limits: StoreLimitsBuilder::new().build(),
            redraw_at: None,
            screen_w: 0,
            screen_h: 0,
            clock_12h: false,
            network_http: true,
            http: None,
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
            "(X-A): 1",
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
}

/// The pure geometry of the system Home swipe, shared by the host and its test.
pub fn gesture_matches(x: f32, y: f32, x2: f32, y2: f32, w: i32, h: i32) -> bool {
    let (aw, ah) = (w.max(1), h.max(1));
    y >= ah as f32 * 0.72
        && y2 <= ah as f32 * 0.62
        && y2 <= y - ah as f32 * 0.12
        && (x2 - x).abs() <= aw as f32 * 0.22
}
