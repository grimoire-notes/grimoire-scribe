//! The global quick settings panel (2.3): a one-finger pull down from the top of any screen opens
//! brightness and warmth, Settings, rotation lock and auto brightness over whatever is in front.
//!
//! The panel is the app's, not a screen's: the pull and the panel take their pointer events before
//! the screen sees them, and app_flush overlays the panel for the present, putting the screen's own
//! pixels back afterwards (as the toast and the staying notice do). So it works on the home screen,
//! in an app, in Settings, and over a notebook alike.

use crate::globalui::gui_status_h;
use crate::prelude::*;

const Q_BRIGHT: i32 = 1;
const Q_WARM: i32 = 2;
const Q_SETTINGS: i32 = 3;
const Q_ROTATE: i32 = 4;
const Q_AUTO: i32 = 5;
const Q_CLOSE: i32 = 6;

/// The quick settings panel and the finger pulling it down.
#[derive(Default)]
pub struct Quick {
    pub open: bool,
    pub rect: Rect,
    hits: HitList,
    press: Press,
    /// the slider being dragged (Q_BRIGHT or Q_WARM), 0 none
    active: i32,
    active_row: Rect,
    brightness: i32,
    warmth: i32,
    /// the Kindle's own auto brightness: 1 on, 0 off, -1 where there is no sensor
    auto: i32,
    pressed: Option<Rect>,
    /// a finger that came down in the top strip, and has not pulled far enough yet
    tracking: bool,
    /// the pixels the panel covers for the present under way
    under: Canvas,
}

struct QuickLayout {
    rect: Rect,
    title: Rect,
    close: Rect,
    buttons: [(i32, Rect); 3],
    bright: Rect,
    warm: Rect,
}

/// The top strip a pull starts in: the bar where it is drawn, and the same system strip when the
/// bar is hidden (quick settings must stay reachable then).
fn top_strip() -> i32 {
    gui_status_h()
}

/// Where the panel starts: under the bar when it is there, at the top otherwise.
fn panel_top(a: &App) -> i32 {
    if crate::status::status_bar_visible(a) {
        top_strip()
    } else {
        0
    }
}

fn quick_layout(a: &App) -> QuickLayout {
    let top = panel_top(a);
    let m = ui_px(40.0);
    let top_pad = ui_px(28.0);
    let title_h = ui_px(78.0);
    let title_gap = ui_px(36.0);
    let button_h = ui_px(112.0);
    let button_gap = ui_px(30.0);
    let slider_h = ui_px(112.0);
    let slider_gap = ui_px(18.0);
    let bottom_pad = ui_px(30.0);
    let h = (top_pad + title_h + title_gap + button_h + button_gap + slider_h * 2 + slider_gap + bottom_pad)
        .min((a.h - top).max(1));
    let rect = Rect::new(0, top, a.w, h);
    let title_y = top + top_pad;
    let title = Rect::new(m, title_y, (a.w - 2 * m - ui_px(90.0)).max(0), title_h);
    let close_sz = ui_px(70.0);
    let close = Rect::new(a.w - m - close_sz, title_y + ui_px(4.0), close_sz, close_sz);
    let gap = ui_px(24.0);
    let by = title_y + title_h + title_gap;
    let bw = ((a.w - 2 * m - 2 * gap) / 3).max(1);
    let buttons = [
        (Q_SETTINGS, Rect::new(m, by, bw, button_h)),
        (Q_ROTATE, Rect::new(m + bw + gap, by, bw, button_h)),
        (Q_AUTO, Rect::new(m + 2 * (bw + gap), by, bw, button_h)),
    ];
    let sy = by + button_h + button_gap;
    let bright = Rect::new(m, sy, (a.w - 2 * m).max(0), slider_h);
    let warm = Rect::new(m, sy + slider_h + slider_gap, (a.w - 2 * m).max(0), slider_h);
    QuickLayout { rect, title, close, buttons, bright, warm }
}

fn quick_slider(value: i32, label: &'static str, row: Rect) -> Slider {
    let mut s = Slider { min: 0.0, max: LIGHT_MAX as f32, step: 1.0, value: value.max(0) as f32, label, ..Default::default() };
    ui_slider_layout(&mut s, row);
    s
}

fn quick_draw(a: &mut App) {
    let lay = quick_layout(a);
    let (brightness, warmth, auto, pressed) = {
        let q = &a.quick;
        (q.brightness, q.warmth, q.auto, q.pressed)
    };
    let mut hits = HitList::default();
    hits_add(&mut hits, lay.close, Q_CLOSE, 0);
    for (id, r) in lay.buttons {
        hits_add(&mut hits, r, id, 0);
    }
    hits_add(&mut hits, lay.bright, Q_BRIGHT, 0);
    hits_add(&mut hits, lay.warm, Q_WARM, 0);
    canvas_fill(&mut a.screen, lay.rect, 255);
    canvas_fill(
        &mut a.screen,
        Rect::new(lay.rect.x, lay.rect.y + lay.rect.h - ui_px(2.0), lay.rect.w, ui_px(2.0)),
        0,
    );
    crate::globalui::gui_page_title(&mut a.screen, lay.title, tr("Quick settings"));
    crate::globalui::gui_icon_button(&mut a.screen, lay.close, IconKind::Close, ButtonStyle::Plain);
    let labels = [tr("Settings"), tr("Rotation lock"), tr("Auto brightness")];
    let icons = [IconKind::Gear, IconKind::Collapse, IconKind::Brightness];
    let on = [false, a.settings.rotation_lock != 0, auto > 0];
    for i in 0..3 {
        let st = if on[i] { ButtonStyle::Selected } else { ButtonStyle::Plain };
        crate::globalui::gui_button(&mut a.screen, lay.buttons[i].1, icons[i], labels[i], st);
    }
    for (row, value, label) in [(lay.bright, brightness, tr("Brightness")), (lay.warm, warmth, tr("Warmth"))] {
        let s = quick_slider(value, label, row);
        let mut text = String::new();
        csnprintf!(&mut text, 16, "%d", value.max(0));
        ui_slider_draw(&mut a.screen, &s, Some(&text));
    }
    if let Some(r) = pressed {
        canvas_invert(&mut a.screen, r);
    }
    let q = &mut a.quick;
    q.rect = lay.rect;
    q.hits = hits;
}

pub fn is_open(a: &App) -> bool {
    a.quick.open
}

pub fn quick_open(a: &mut App) {
    let mut q = Quick::default();
    light_get(&mut q.brightness, &mut q.warmth);
    q.auto = light_auto();
    q.open = true;
    q.rect = quick_layout(a).rect;
    a.quick = q;
    app_invalidate(a, a.quick.rect, RefreshMode::Ui);
}

pub fn quick_close(a: &mut App) {
    if !a.quick.open {
        return;
    }
    let r = a.quick.rect;
    canvas_free(&mut a.quick.under);
    a.quick.open = false;
    a.quick.tracking = false;
    a.quick.press.down = false;
    a.quick.active = 0;
    a.quick.pressed = None;
    hits_clear(&mut a.quick.hits);
    app_invalidate(a, r, RefreshMode::Ui);
}

/// Closes it without a refresh: a screen switch or a redraw draws the whole screen anyway.
pub fn quick_close_silent(a: &mut App) {
    if !a.quick.open {
        return;
    }
    canvas_free(&mut a.quick.under);
    a.quick.open = false;
    a.quick.tracking = false;
    a.quick.press.down = false;
    a.quick.active = 0;
    a.quick.pressed = None;
    hits_clear(&mut a.quick.hits);
}

pub fn quick_free(a: &mut App) {
    canvas_free(&mut a.quick.under);
}

/// A drag on a slider row: its value follows x (whole steps), applied to the front light at once.
fn quick_drag(a: &mut App, id: i32, row: Rect, x: f32) -> bool {
    let q = &mut a.quick;
    let value = if id == Q_BRIGHT { q.brightness } else { q.warmth };
    let mut sl = quick_slider(value, if id == Q_BRIGHT { tr("Brightness") } else { tr("Warmth") }, row);
    ui_slider_set_from_x(&mut sl, x);
    let v = sl.value as i32;
    if v == value {
        return false;
    }
    if id == Q_BRIGHT {
        q.brightness = v;
        light_set_intensity(v);
    } else {
        q.warmth = v;
        light_set_warmth(v);
    }
    true
}

fn quick_settings_open(a: &mut App) {
    match a.scr {
        ScreenId::Editor => {
            crate::editor::editor_settings_open(a);
        }
        ScreenId::Library => crate::library::library_settings_open(a),
        _ => {
            app_switch(a, ScreenId::Library);
            crate::library::library_settings_open(a);
        }
    }
}

fn quick_tap(a: &mut App, id: i32) {
    match id {
        Q_SETTINGS => {
            quick_close(a);
            quick_settings_open(a);
        }
        Q_ROTATE => {
            a.settings.rotation_lock = if a.settings.rotation_lock == 0 { 1 + a.rotation } else { 0 };
            if a.settings.rotation_lock == 0 && a.settings.follow_rotation {
                let caps = input_caps();
                if caps.has_orientation {
                    a.pending_rotation = caps.orientation;
                }
            } else {
                a.pending_rotation = -1;
            }
            settings_save(&a.settings);
            app_invalidate(a, a.quick.rect, RefreshMode::Ui);
        }
        Q_AUTO => {
            let (mut intensity, mut warmth) = (0, 0);
            light_get(&mut intensity, &mut warmth);
            let on = a.quick.auto == 0;
            light_set_auto(on);
            if on {
                light_get(&mut intensity, &mut warmth);
            }
            let rect = {
                let q = &mut a.quick;
                q.brightness = intensity;
                q.warmth = warmth;
                q.auto = light_auto();
                q.rect
            };
            app_invalidate(a, rect, RefreshMode::Ui);
        }
        Q_CLOSE => quick_close(a),
        _ => {}
    }
}

fn quick_cancel(a: &mut App) {
    a.quick.active = 0;
    a.quick.pressed = None;
    a.quick.press.down = false;
    app_invalidate(a, a.quick.rect, RefreshMode::Ui);
}

fn quick_pointer(a: &mut App, ph: i32, ev: &InEvent) {
    if ph == 0 {
        let (active, row, hit) = {
            let q = &mut a.quick;
            press_begin(&mut q.press, ev.x, ev.y, ev.t_ms);
            q.pressed = None;
            if !rect_contains(q.rect, ev.x as i32, ev.y as i32) {
                return;
            }
            let hit = hits_find(&q.hits, ev.x, ev.y).copied();
            q.active = match hit {
                Some(h) if h.id == Q_BRIGHT || h.id == Q_WARM => h.id,
                _ => 0,
            };
            if q.active != 0 {
                if let Some(h) = hit {
                    q.active_row = h.r;
                }
            }
            (q.active, q.active_row, hit)
        };
        if active != 0 {
            if quick_drag(a, active, row, ev.x) {
                app_invalidate(a, row, RefreshMode::Ui);
            }
            return;
        }
        if let Some(h) = hit {
            a.quick.pressed = Some(h.r);
            app_invalidate(a, h.r, RefreshMode::Ui);
        }
        return;
    }
    let (active, row, down) = {
        let q = &a.quick;
        (q.active, q.active_row, q.press.down)
    };
    if active != 0 {
        if ph == 2 {
            press_move(&mut a.quick.press, ev.x, ev.y);
            a.quick.active = 0;
            app_invalidate(a, a.quick.rect, RefreshMode::Ui);
        } else if quick_drag(a, active, row, ev.x) {
            app_invalidate(a, row, RefreshMode::Ui);
        }
        return;
    }
    if ph != 2 || !down {
        return;
    }
    press_move(&mut a.quick.press, ev.x, ev.y);
    a.quick.pressed = None;
    let (outside, hit, tap) = {
        let q = &a.quick;
        let hit = hits_find(&q.hits, ev.x, ev.y).copied();
        (
            !rect_contains(q.rect, q.press.x0 as i32, q.press.y0 as i32),
            hit,
            hit.is_some_and(|h| press_is_tap(&q.press, h.r)),
        )
    };
    app_invalidate(a, a.quick.rect, RefreshMode::Ui);
    if outside {
        quick_close(a);
        return;
    }
    if tap {
        if let Some(h) = hit {
            quick_tap(a, h.id);
        }
    }
}

/// A one-finger pull down from the top strip, or the open panel's own contacts. Returns whether the
/// event was the system panel's (the screen must not also see it).
pub fn quick_event(a: &mut App, ev: &InEvent) -> bool {
    if a.asleep {
        return false;
    }
    if a.quick.open {
        let ph = app_pointer_phase(ev);
        if ph >= 0 {
            if touch_cancelled(ev) {
                quick_cancel(a);
            } else {
                quick_pointer(a, ph, ev);
            }
            return true;
        }
        return matches!(ev.type_, InType::PinchBegin | InType::PinchMove | InType::PinchEnd);
    }
    if ev.type_ == InType::TouchDown && ev.y < top_strip() as f32 {
        // A notebook control that reaches into the top strip (its floating panel, a link) keeps its
        // contact; the bar itself has no controls, so a pull anywhere else in it is the system's.
        if a.scr == ScreenId::Editor && crate::editor::editor_control_at(a, ev.x, ev.y) {
            return false;
        }
        let q = &mut a.quick;
        if q.tracking {
            q.tracking = false;
            q.press.down = false;
            return true;
        }
        press_begin(&mut q.press, ev.x, ev.y, ev.t_ms);
        q.tracking = true;
        return true;
    }
    if a.quick.tracking {
        match ev.type_ {
            InType::TouchMove => {
                let (dx, dy) = {
                    let q = &mut a.quick;
                    press_move(&mut q.press, ev.x, ev.y);
                    (ev.x - q.press.x0, ev.y - q.press.y0)
                };
                if dy >= ui_px(64.0) as f32 && dy > dx.abs() * 1.2 {
                    a.quick.tracking = false;
                    quick_open(a);
                } else if dy < -ui_px(8.0) as f32 || dx.abs() > ui_px(160.0) as f32 {
                    a.quick.tracking = false;
                }
                return true;
            }
            InType::TouchUp => {
                let tap = {
                    let q = &mut a.quick;
                    press_move(&mut q.press, ev.x, ev.y);
                    q.tracking = false;
                    !q.press.moved && ev.y < top_strip() as f32
                };
                if tap && crate::status::status_bar_visible(a) {
                    crate::status::status_toggle(a);
                }
                return true;
            }
            InType::TouchDown | InType::PinchBegin | InType::PinchMove | InType::PinchEnd => {
                a.quick.tracking = false;
                return true;
            }
            InType::PenDown | InType::PenMove | InType::PenHover | InType::PenLeave => {
                a.quick.tracking = false;
                return false;
            }
            _ => {}
        }
    }
    false
}

/// The runner's tap target for a panel control (quick.close, quick.brightness...), on any screen.
pub fn control_point(a: &mut App, name: &str) -> Option<(f32, f32)> {
    if !a.quick.open {
        return None;
    }
    let n = name.strip_prefix("quick.")?;
    let id = match n {
        "brightness" => Q_BRIGHT,
        "warmth" => Q_WARM,
        "settings" => Q_SETTINGS,
        "rotate" => Q_ROTATE,
        "auto" => Q_AUTO,
        "close" => Q_CLOSE,
        _ => return None,
    };
    a.quick.hits.h.iter().find(|h| h.id == id).map(|h| ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32))
}

/// The overlay for the present: copies the panel's pixels, draws it, and leaves the screen's own
/// pixels for quick_uncover after the present.
pub fn quick_cover(a: &mut App, dirty: &mut Rect) -> bool {
    if !a.quick.open {
        return false;
    }
    let old = a.quick.rect;
    let now = quick_layout(a).rect;
    if now != old {
        *dirty = rect_union(*dirty, old);
        a.quick.rect = now;
    }
    let r = now;
    if !rect_overlaps(*dirty, r) {
        return false;
    }
    *dirty = rect_union(*dirty, r);
    canvas_free(&mut a.quick.under);
    if canvas_init(&mut a.quick.under, r.w, r.h) != 0 {
        return false;
    }
    canvas_blit(&mut a.quick.under, &a.screen, r, 0, 0);
    quick_draw(a);
    true
}

pub fn quick_uncover(a: &mut App) {
    if a.quick.under.px.is_empty() {
        return;
    }
    let (w, h) = (a.quick.under.w, a.quick.under.h);
    let r = a.quick.rect;
    canvas_blit(&mut a.screen, &a.quick.under, Rect::new(0, 0, w, h), r.x, r.y);
}

/// The runner's quick.* keys (and, through editor.*, the old editor.quick.* names).
pub fn quick_probe(a: &App, key: &str, out: &mut String, cap: usize) -> i32 {
    let q = &a.quick;
    match key {
        "quick" | "quick.open" => csnprintf!(out, cap, "%d", q.open as i32),
        "quick.rect" => {
            if q.open {
                let r = q.rect;
                csnprintf!(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h)
            } else {
                csnprintf!(out, cap, "%s", "")
            }
        }
        "quick.brightness" => csnprintf!(out, cap, "%d", if q.open { q.brightness } else { -1 }),
        "quick.warmth" => csnprintf!(out, cap, "%d", if q.open { q.warmth } else { -1 }),
        "quick.auto" => csnprintf!(out, cap, "%d", if q.open { q.auto } else { -1 }),
        _ => return -1,
    };
    0
}
