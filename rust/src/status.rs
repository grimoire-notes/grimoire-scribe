//! status.rs: the battery reading and the numbers behind the shared top status bar (globalui.rs).
//!
//! The old floating clock-and-battery corner is retired: a notebook page keeps the one status bar the
//! whole app draws (globalui.rs), showing the time, date, connections and battery at the top of the
//! page. The settings.ini fields (status_on, status_size, status_corner) are still read and written so
//! an owner's file keeps its meaning: `status_on` is whether the bar is shown at all (a tap hides it,
//! and the state survives a restart), `status_size` scales the bar, and the corner has no place on
//! the page. The Display sheet's "Status bar" page changes the size, the 12-hour clock, the bar's
//! visibility and whether a notebook shows it (its background there is transparent).
//!
//! The functions here that app.rs, editor.rs and the shortcuts still call are kept as no-ops
//! (status_pen, status_touch, status_cover and friends) so the editor's event and flush paths do not
//! need a second flag for a feature that no longer exists.

use crate::prelude::*;

/// How many status bar sizes there are: settings.ini status_size 0 small, 1 medium, 2 large.
pub const STATUS_SIZES: i32 = 3;

/// What the power supply says now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Battery {
    /// 0 to 100, or -1 when it cannot be read
    pub percent: i32,
    /// it takes charge now
    pub charging: bool,
}

impl Default for Battery {
    fn default() -> Self {
        Battery { percent: -1, charging: false }
    }
}

// ---- the text ----

/// "Oct 10, 14:32  87%" (or "Oct 10, 2:32 PM  87%" on the 12-hour clock): the time as the app writes
/// times (the language in use, local time), then the percent; just the time where the battery cannot be
/// read. The shared bar draws the same parts, the percent beside the battery art.
pub fn status_text(t: i64, b: &Battery, twelve: bool) -> String {
    let mut when = String::new();
    if twelve {
        format_time_short_12h(t, &mut when, 64);
    } else {
        format_time_short(t, &mut when, 64);
    }
    if b.percent >= 0 {
        cformat!("%s  %d%%", &when, b.percent)
    } else {
        when
    }
}

// ---- the battery, as the kernel tells it ----

/// A whole number in a file of the power supply class ("87\n"): None for anything else.
pub fn battery_parse_number(b: &[u8]) -> Option<i32> {
    let t = std::str::from_utf8(b).ok()?.trim();
    if t.is_empty() || t.len() > 9 {
        return None;
    }
    t.parse::<i32>().ok()
}

/// The supply's `status` file: "Charging" charges; "Discharging", "Full", "Not charging" and
/// "Unknown" do not.
pub fn battery_parse_charging(b: &[u8]) -> bool {
    std::str::from_utf8(b).is_ok_and(|t| t.trim().eq_ignore_ascii_case("charging"))
}

/// The battery among the supplies under root (/sys/class/power_supply): the one whose `type` is
/// Battery (the chargers and the USB port are supplies too, with no capacity), its `capacity` and
/// whether its `status` says it charges. None where there is no such supply or it has no readable
/// capacity.
pub fn battery_from_sysfs(root: &str) -> Option<Battery> {
    let mut names: Vec<String> =
        std::fs::read_dir(root).ok()?.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    for name in names {
        let dir = cformat!("%s/%s", root, &name);
        let is_battery =
            read_system_file(&cformat!("%s/type", &dir)).is_some_and(|t| String::from_utf8_lossy(&t).trim().eq_ignore_ascii_case("battery"));
        if !is_battery {
            continue;
        }
        let Some(percent) = read_system_file(&cformat!("%s/capacity", &dir)).and_then(|c| battery_parse_number(&c)) else {
            continue;
        };
        let charging = read_system_file(&cformat!("%s/status", &dir)).is_some_and(|s| battery_parse_charging(&s));
        return Some(Battery { percent: clampi(percent, 0, 100), charging });
    }
    None
}

// ---- the retired corner ----

/// No status strip is reserved: the shared bar is drawn over the page's top margin, and the page keeps
/// the full screen underneath it.
pub fn status_reserved(_a: &App) -> i32 {
    0
}

/// Compatibility for the retired status-mode action: it still toggles the stored switch, which no
/// longer puts a widget on the page.
pub fn status_cycle_mode(a: &mut App) {
    status_toggle(a);
}

/// Turns the shared status bar on or off. `status_on` is saved, so a tap that hides the bar survives a
/// restart; the Status bar settings page's "Show status bar" row turns it back on.
pub fn status_toggle(a: &mut App) {
    a.settings.status_mode = 1;
    a.settings.status_on = !a.settings.status_on;
    settings_save(&a.settings);
    crate::globalui::gui_status_shown(a.settings.status_on);
    app_redraw(a);
    qlog!("status: the shared bar is %s", if a.settings.status_on { "shown" } else { "hidden" });
}

/// Whether the shared bar is drawn on the screen in front: `status_on`, and in a notebook also the
/// Status bar settings page's "Show status bar in notebooks". An app that asks for no bar gets none.
pub fn status_bar_visible(a: &App) -> bool {
    if !a.settings.status_on {
        return false;
    }
    match a.scr {
        ScreenId::Editor => crate::library::library_settings_active(a) || a.settings.status_in_notebooks,
        ScreenId::Library if a.app_view.open => a.app_view.status_bar,
        _ => true,
    }
}

/// Settings > Sleep & Clock > Status bar > the size row: the next size, small to medium to large and
/// round. The shared bar's height and contents scale with it.
pub fn status_cycle_size(a: &mut App) {
    a.settings.status_size = (a.settings.status_size + 1) % STATUS_SIZES;
    crate::globalui::gui_status_style(a.settings.status_size, a.settings.clock_12h);
    settings_save(&a.settings);
}

/// The corner is retired: the shortcut leaves the stored corner alone too (the Display sheet no longer
/// offers it).
pub fn status_cycle_corner(_a: &mut App) {}

/// The status-corner shortcut: nothing, now that the corner is retired.
pub fn status_corner_shortcut(_a: &mut App) {}

/// The corner's square, empty: nothing on the page takes a tap for it.
pub fn status_square(_a: &App) -> Rect {
    RECT_EMPTY
}

/// The shared bar's room, where the old corner's zone used to be: top-docked cards and keyboards
/// keep below it while it is shown. Hidden, the page has the top to itself.
pub fn status_zone(a: &App) -> Rect {
    if !status_bar_visible(a) {
        return RECT_EMPTY;
    }
    Rect { x: 0, y: 0, w: a.w, h: crate::globalui::gui_status_h() }
}

/// Whether (x, y) is the corner's: never.
pub fn status_hit(_a: &App, _x: f32, _y: f32) -> bool {
    false
}

/// Whether the last touch was the corner's tap: never.
pub fn status_tapped() -> bool {
    false
}

/// Every pen event, for the retired corner: nothing to watch.
pub fn status_pen(_ev: &InEvent) {}

/// A finger's events, for the retired corner: nothing to take.
pub fn status_touch(_a: &mut App, _ev: &InEvent) {}

/// A status setting changed: the shared bar redraws with the next composition; there is no overlay to
/// refresh.
pub(crate) fn ask_change(_a: &App) {}

/// The Kindle went to sleep or woke: the retired overlay held nothing.
pub fn status_reset() {}

/// A setting waits to be saved: nothing waits here now.
pub fn status_save_now(_a: &App) {}

/// The retired corner's minute and battery tick: nothing to draw or ask for.
pub fn status_tick(_a: &mut App, _now: u32) {}

/// The pixels a painted corner covered, for app_flush to put back: the shared bar is already in the
/// screen, so there is nothing to put back.
pub struct StatusCover;

/// Keeps Live view's status layer (the shared bar, when it is shown) in step with the screen: a refresh
/// that reaches the bar, or a bar that is now hidden, tells the layer again. The bar is painted into
/// a.screen by the screen itself, so there is nothing to cover and put back.
pub fn status_cover(a: &mut App, dirty: &mut Rect) -> Option<StatusCover> {
    let zone = status_zone(a);
    if rect_is_empty(zone) {
        crate::live::live_layer(crate::live::LIVE_LAYER_STATUS, &a.screen, RECT_EMPTY);
    } else if rect_overlaps(*dirty, zone) {
        crate::live::live_layer(crate::live::LIVE_LAYER_STATUS, &a.screen, zone);
    }
    None
}

/// Puts back what the retired corner covered: nothing.
pub fn status_uncover(_a: &mut App, _c: StatusCover) {}

/// A refresh was sent: the retired corner was not in it.
pub fn status_presented(_a: &App, _r: Rect) {}

/// Live view starts: the shared bar's layer, where it is shown.
pub fn status_live(a: &mut App) {
    let zone = status_zone(a);
    crate::live::live_layer(crate::live::LIVE_LAYER_STATUS, &a.screen, zone);
}

/// A refresh of the retired corner: nothing.
pub fn status_flush(_a: &mut App) {}

// ---- for the runner ----

/// The runner's status.* keys, kept so existing scripts still resolve. The shared bar has no runner
/// overlay state: "reserved" is 0, "front" still says whether the notebook page is in front, and the
/// retired corner's "shown", "text", "rect", "icon", "fill" and "charging" are empty. The settings
/// themselves are "settings.status_on", "settings.status_size", "settings.status_corner" and
/// "settings.clock_12h".
pub fn status_probe(a: &App, key: &str, out: &mut String, cap: usize) -> i32 {
    match key {
        "reserved" => csnprintf!(out, cap, "%d", status_reserved(a)),
        "live" | "controller" | "shown" => csnprintf!(out, cap, "0"),
        "visible" => csnprintf!(out, cap, "%d", status_bar_visible(a) as i32),
        "text" | "rect" | "icon" | "fill" | "charging" => csnprintf!(out, cap, "%s", ""),
        "zone" => {
            let r = status_zone(a);
            csnprintf!(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h)
        }
        "front" => csnprintf!(out, cap, "%d", editor_page_in_front(a) as i32),
        _ => return -1,
    };
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up() {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        ui_set_theme(UiTheme::Classic);
        i18n_apply("en");
    }

    extern "C" {
        // (the libc crate does not declare it on every target)
        fn tzset();
    }

    /// A time of day in UTC: the text tests set TZ for the process (they run one at a time).
    fn utc() {
        std::env::set_var("TZ", "UTC");
        // SAFETY: tzset takes no arguments and only rereads TZ.
        unsafe { tzset() }
    }

    #[test]
    fn the_text_is_the_time_then_the_percent() {
        utc();
        up();
        // 2023-11-14 22:13:20 UTC
        let b = Battery { percent: 87, charging: false };
        assert_eq!(status_text(1700000000, &b, false), "Nov 14, 22:13  87%");
        // charging does not change the text (the bar draws the battery art)
        assert_eq!(status_text(1700000000, &Battery { percent: 87, charging: true }, false), "Nov 14, 22:13  87%");
        assert_eq!(status_text(1700000000, &Battery { percent: 5, charging: false }, false), "Nov 14, 22:13  5%");
        assert_eq!(status_text(1700000000, &Battery { percent: 100, charging: false }, false), "Nov 14, 22:13  100%");
        // without a battery reading, the time alone
        assert_eq!(status_text(1700000000, &Battery::default(), false), "Nov 14, 22:13");
        // the 12-hour clock: the status bar's own setting, not every time the app writes
        assert_eq!(status_text(1700000000, &b, true), "Nov 14, 10:13 PM  87%");
        assert_eq!(status_text(1700000000, &Battery::default(), true), "Nov 14, 10:13 PM");
        // the language in use: Spanish writes the day first
        i18n_apply("es");
        assert_eq!(status_text(1700000000, &b, false), "14 nov, 22:13  87%");
        i18n_apply("en");
    }

    #[test]
    fn numbers_and_states_of_the_power_supply() {
        assert_eq!(battery_parse_number(b"87\n"), Some(87));
        assert_eq!(battery_parse_number(b" 100 "), Some(100));
        assert_eq!(battery_parse_number(b"0"), Some(0));
        assert_eq!(battery_parse_number(b""), None);
        assert_eq!(battery_parse_number(b"\n"), None);
        assert_eq!(battery_parse_number(b"87%"), None);
        assert_eq!(battery_parse_number(b"99999999999"), None);
        assert_eq!(battery_parse_number(b"\xff"), None);
        assert!(battery_parse_charging(b"Charging\n"));
        assert!(battery_parse_charging(b"charging"));
        assert!(!battery_parse_charging(b"Not charging\n"));
        assert!(!battery_parse_charging(b"Discharging\n"));
        assert!(!battery_parse_charging(b"Full\n"));
        assert!(!battery_parse_charging(b""));
    }

    fn supply(root: &std::path::Path, name: &str, files: &[(&str, &str)]) {
        let d = root.join(name);
        std::fs::create_dir_all(&d).unwrap();
        for (f, v) in files {
            std::fs::write(d.join(f), v).unwrap();
        }
    }

    #[test]
    fn the_battery_is_the_supply_of_that_type() {
        let root = std::env::temp_dir().join(format!("grimoire-status-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let r = root.to_str().unwrap();
        // no supplies, or none that is a battery
        assert_eq!(battery_from_sysfs(r), None);
        assert_eq!(battery_from_sysfs("/nonexistent/power_supply"), None);
        supply(&root, "a_charger", &[("type", "Mains\n"), ("online", "1\n")]);
        supply(&root, "b_usb", &[("type", "USB\n"), ("online", "0\n")]);
        assert_eq!(battery_from_sysfs(r), None);
        // a battery among them, picked by its type whatever the name or the order
        supply(&root, "c_bat", &[("type", "Battery\n"), ("capacity", "64\n"), ("status", "Discharging\n")]);
        assert_eq!(battery_from_sysfs(r), Some(Battery { percent: 64, charging: false }));
        supply(&root, "c_bat", &[("type", "Battery\n"), ("capacity", "65\n"), ("status", "Charging\n")]);
        assert_eq!(battery_from_sysfs(r), Some(Battery { percent: 65, charging: true }));
        // a capacity past 100 reads as 100; one that does not read passes the supply by
        supply(&root, "c_bat", &[("type", "Battery\n"), ("capacity", "140\n"), ("status", "Full\n")]);
        assert_eq!(battery_from_sysfs(r), Some(Battery { percent: 100, charging: false }));
        supply(&root, "c_bat", &[("type", "Battery\n"), ("capacity", "n/a\n")]);
        assert_eq!(battery_from_sysfs(r), None);
        // a battery with no status file is not charging
        supply(&root, "c_bat", &[("type", "Battery\n"), ("capacity", "30\n")]);
        std::fs::remove_file(root.join("c_bat").join("status")).ok();
        assert_eq!(battery_from_sysfs(r), Some(Battery { percent: 30, charging: false }));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_retired_corner_reserves_nothing_and_takes_no_tap() {
        up();
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        a.settings.status_on = true;
        a.settings.status_in_notebooks = true;
        assert_eq!(status_reserved(&a), 0);
        assert!(rect_is_empty(status_square(&a)));
        assert_eq!(status_zone(&a), Rect { x: 0, y: 0, w: 1860, h: crate::globalui::gui_status_h() });
        assert!(!status_hit(&a, 70.0, 70.0));
        assert!(!status_tapped());
        let mut out = String::new();
        assert_eq!(status_probe(&a, "shown", &mut out, 32), 0);
        assert_eq!(out, "0");
        assert_eq!(status_probe(&a, "zone", &mut out, 32), 0);
        assert_eq!(out, format!("0 0 1860 {}", crate::globalui::gui_status_h()));
    }

    #[test]
    fn the_size_row_still_scales_the_shared_bar() {
        up();
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        a.settings.status_size = 0;
        crate::globalui::gui_status_style(0, false);
        let small = crate::globalui::gui_status_h();
        status_cycle_size(&mut a);
        assert_eq!(a.settings.status_size, 1);
        let medium = crate::globalui::gui_status_h();
        status_cycle_size(&mut a);
        assert_eq!(a.settings.status_size, 2);
        let large = crate::globalui::gui_status_h();
        assert!(small < medium && medium < large, "{small} {medium} {large}");
        status_cycle_size(&mut a);
        assert_eq!(a.settings.status_size, 0);
        crate::globalui::gui_status_style(1, false);
    }
}
