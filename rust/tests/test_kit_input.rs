// test_kit_input.rs: the package kit's input (kithost.rs, docs/KIT.md, Events): the recogniser of a contact
// (`KitPointer`: a tap, a press at HOLD_MS, the slop of TAP_SLOP_MM each way, a drag's start, gathered samples and
// end, a cancel, the pen and a finger, more samples than a DRAG_MOVE holds), the pace of the passes input owes
// (`KitInput`), the part of the screen a pass changed (`kit_keep`, `canvas_changed_rect`), and `kit_event`, `event_points` and `key_char`
// in both hosts, with modules written for the purpose (tests/kitfix): their numbers, limits and charges, the body
// before the first pass, the budget and stop rules of an event.
mod kitfix;

use grimoire::kithost::*;
use grimoire::prelude::*;
use kitfix::*;

/// A Scribe's screen, the slop at its 300 dpi (TAP_SLOP_MM), and the bodies of test_kit.rs.
const SCREEN: (i32, i32) = (1860, 2480);
const WIDGET_BODY: Rect = Rect { x: 214, y: 192, w: 1272, h: 2200 };
const HOSTS: [Host; 2] = [Host::Widget, Host::App];

fn slop() -> i32 {
    mm_to_px(TAP_SLOP_MM, 300.0) as i32
}

fn app_body() -> Rect {
    let top = grimoire::globalui::gui_status_h();
    Rect { x: 0, y: top, w: SCREEN.0, h: SCREEN.1 - top }
}

fn kinds(evs: &[KitEvent]) -> Vec<i32> {
    evs.iter().map(|e| e.kind).collect()
}

// ---- the recogniser ----

#[test]
fn a_lift_within_the_slop_before_the_hold_is_a_tap_from_a_finger_or_the_pen() {
    for source in [0, 1] {
        let mut p = KitPointer::default();
        assert!(p.down(100, 200, source, 7, 1000, slop(), false).is_empty(), "a contact that comes down is nothing yet");
        assert!(p.is_down() && p.undecided());
        assert!(p.moved(100 + slop(), 200 - slop(), false).is_empty(), "within the slop");
        assert_eq!(p.tick(1000 + HOLD_MS - 1), None, "not yet held");
        assert_eq!(p.up(100 + slop(), 200, false), vec![KitEvent::new(EV_TAP, 7, 100, 200, source)], "a TAP at where it came down, on its region");
        assert!(!p.is_down());
        // (and nothing more for that contact)
        assert_eq!(p.tick(1000 + HOLD_MS), None);
        assert!(p.up(100, 200, false).is_empty());
    }
}

#[test]
fn a_contact_held_still_for_the_hold_is_a_press_once_and_its_lift_is_no_tap() {
    let mut p = KitPointer::default();
    p.down(50, 60, 0, -1, u32::MAX - 100, slop(), false);
    // (the app's clock wraps: the hold is counted across it)
    assert_eq!(p.tick((u32::MAX - 100).wrapping_add(HOLD_MS - 1)), None);
    assert_eq!(p.tick((u32::MAX - 100).wrapping_add(HOLD_MS)), Some(KitEvent::new(EV_PRESS, -1, 50, 60, 0)));
    assert_eq!(p.tick((u32::MAX - 100).wrapping_add(HOLD_MS + 500)), None, "once");
    assert!(!p.undecided() && p.still());
    assert!(p.up(50, 60, false).is_empty(), "a press's lift is no tap");
}

#[test]
fn the_slop_is_tap_slop_mm_each_way_its_bound_still_a_tap_one_pixel_more_a_drag() {
    assert_eq!(slop(), 59, "5 mm at 300 dpi");
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let mut p = KitPointer::default();
        p.down(500, 500, 0, 3, 0, slop(), false);
        assert!(p.moved(500 + dx * slop(), 500 + dy * slop(), false).is_empty(), "at the bound ({dx}, {dy})");
        assert_eq!(kinds(&p.up(500 + dx * slop(), 500 + dy * slop(), false)), [EV_TAP]);
        let mut p = KitPointer::default();
        p.down(500, 500, 0, 3, 0, slop(), false);
        assert_eq!(p.moved(500 + dx * (slop() + 1), 500 + dy * (slop() + 1), false), vec![KitEvent::new(EV_DRAG_START, 3, 500, 500, 0)], "past it ({dx}, {dy})");
        assert_eq!(kinds(&p.up(500, 500, false)), [EV_DRAG_MOVE, EV_DRAG_END], "and its lift, back where it began, ends a drag");
    }
}

#[test]
fn a_drag_starts_once_gathers_its_samples_oldest_first_and_ends_at_its_lift() {
    let mut p = KitPointer::default();
    p.down(100, 100, 1, 2, 0, slop(), false);
    assert_eq!(p.moved(130, 100, false), vec![], "within the slop: nothing gathered");
    assert_eq!(p.moved(200, 110, false), vec![KitEvent::new(EV_DRAG_START, 2, 100, 100, 1)], "DRAG_START at where it came down");
    assert!(p.moved(250, 120, false).is_empty(), "once");
    assert!(p.has_moves());
    let m = p.take_moves().unwrap();
    assert_eq!((m.kind, m.id, m.a, m.b, m.c), (EV_DRAG_MOVE, 2, 250, 120, 2), "at the newest, with how many");
    assert_eq!(m.points, vec![(200, 110), (250, 120)], "oldest first");
    assert_eq!(p.take_moves(), None, "none until more come");
    p.moved(300, 130, false);
    let end = p.up(320, 140, false);
    assert_eq!(kinds(&end), [EV_DRAG_MOVE, EV_DRAG_END]);
    assert_eq!(end[0].points, vec![(300, 130), (320, 140)], "the samples left, and the lift");
    assert_eq!((end[1].a, end[1].b, end[1].c), (320, 140, 0), "DRAG_END at the lift, not cancelled");
    // (a lift where the last sample was is not gathered twice)
    p.down(0, 0, 0, -1, 0, slop(), false);
    p.moved(100, 0, false);
    let end = p.up(100, 0, false);
    assert_eq!(end[0].points, vec![(100, 0)]);
}

#[test]
fn a_press_that_then_moves_past_the_slop_is_a_drag() {
    let mut p = KitPointer::default();
    p.down(10, 10, 0, 4, 0, slop(), false);
    assert_eq!(p.tick(HOLD_MS).map(|e| e.kind), Some(EV_PRESS));
    assert_eq!(kinds(&p.moved(200, 10, false)), [EV_DRAG_START]);
    assert_eq!(kinds(&p.up(200, 10, false)), [EV_DRAG_MOVE, EV_DRAG_END]);
}

#[test]
fn a_lift_past_the_slop_whose_moves_were_not_seen_ends_a_drag() {
    let mut p = KitPointer::default();
    p.down(10, 10, 0, -1, 0, slop(), false);
    let end = p.up(10, 900, false);
    assert_eq!(kinds(&end), [EV_DRAG_START, EV_DRAG_MOVE, EV_DRAG_END], "no TAP");
    assert_eq!(end[1].points, vec![(10, 900)]);
}

#[test]
fn a_cancelled_contact_is_no_tap_and_no_press_and_a_drag_it_was_ends_cancelled() {
    // the platform took the contact (a gesture, the pen came): no TAP
    let mut p = KitPointer::default();
    p.down(10, 10, 0, 1, 0, slop(), false);
    assert!(p.up(-10000, -10000, true).is_empty());
    // nor a PRESS afterwards, nor a TAP after a press
    p.down(10, 10, 0, 1, 0, slop(), false);
    assert!(p.up(-10000, -10000, true).is_empty());
    assert_eq!(p.tick(HOLD_MS), None);
    // a drag ends marked cancelled, at its newest sample (not at the cancel's place); the samples gathered since the
    // last DRAG_MOVE are dropped with it
    p.down(10, 10, 0, 1, 0, slop(), false);
    p.moved(400, 10, false);
    p.moved(500, 20, false);
    assert_eq!(p.up(-10000, -10000, true), vec![KitEvent::new(EV_DRAG_END, 1, 500, 20, 1)]);
    // a palm-sized contact: at its down it is nothing, ever (no press, no tap)
    p.down(10, 10, 0, 1, 0, slop(), true);
    assert_eq!(p.tick(HOLD_MS), None);
    assert!(p.moved(500, 10, false).is_empty(), "no drag");
    assert!(p.up(500, 10, false).is_empty(), "no tap, no end");
    // palm-sized on a move: a tap candidate is dropped, a drag ends cancelled and its later samples are not taken
    p.down(10, 10, 0, 1, 0, slop(), false);
    assert!(p.moved(12, 10, true).is_empty());
    assert!(p.up(12, 10, false).is_empty());
    p.down(10, 10, 0, 1, 0, slop(), false);
    p.moved(400, 10, false);
    assert_eq!(p.moved(420, 10, true), vec![KitEvent::new(EV_DRAG_END, 1, 400, 10, 1)]);
    assert!(p.moved(600, 10, false).is_empty() && !p.has_moves());
    assert!(p.up(600, 10, false).is_empty());
    // a contact that comes down while another was still down (its lift never seen) ends that one first
    p.down(10, 10, 0, 1, 0, slop(), false);
    p.moved(400, 10, false);
    assert_eq!(p.down(50, 50, 0, 2, 0, slop(), false), vec![KitEvent::new(EV_DRAG_END, 1, 400, 10, 1)]);
    assert_eq!(kinds(&p.up(50, 50, false)), [EV_TAP]);
}

#[test]
fn a_drag_move_holds_the_newest_256_samples_however_many_came() {
    let mut p = KitPointer::default();
    p.down(0, 0, 0, -1, 0, slop(), false);
    for k in 1..=100_000 {
        p.moved(k, k % 7, false);
    }
    let m = p.take_moves().unwrap();
    assert_eq!((m.c, m.points.len(), KIT_POINTS_MAX), (256, 256, 256));
    assert_eq!(m.points.first(), Some(&(100_000 - 255, (100_000 - 255) % 7)));
    assert_eq!((m.a, m.b), (100_000, 100_000 % 7));
    // and at the lift, the same bound
    for k in 1..=1000 {
        p.moved(k, 0, false);
    }
    let end = p.up(5000, 0, false);
    assert_eq!(end[0].points.len(), 256);
    assert_eq!(end[0].points.last(), Some(&(5000, 0)));
}

// ---- the pace of the passes input owes ----

#[test]
fn a_pass_input_owes_waits_while_a_tap_may_come_is_paced_while_a_contact_is_down_and_is_at_once_after() {
    assert_eq!(KIT_PASS_MS, 120);
    let mut k = KitInput::default();
    assert_eq!(k.pass_now(false, false), None, "nothing owed");
    k.owe();
    assert_eq!(k.pass_now(false, false), Some(RefreshMode::Ui), "no contact: at once, as the UI");
    assert_eq!(k.pass_now(false, true), None, "never while the last fast refresh is showing");
    k.answered();
    assert!(!k.owed());
    // a contact that may still be a tap: no turn, no pass
    k.pointer.down(0, 0, 0, -1, 1000, slop(), false);
    k.owe();
    assert!(!k.turn(1000, false));
    assert!(!k.turn(1000 + HOLD_MS - 1, false));
    assert_eq!(k.pass_now(false, false), None);
    // a drag: a turn now, then none for KIT_PASS_MS; on each turn the pass is drawn, refreshed fast
    k.pointer.moved(200, 0, false);
    assert!(k.turn(2000, false));
    assert_eq!(k.pass_now(true, false), Some(RefreshMode::Fast));
    assert_eq!(k.pass_now(false, false), None, "not between turns");
    assert!(!k.turn(2000 + KIT_PASS_MS - 1, false));
    assert!(!k.turn(2000 + KIT_PASS_MS, true), "not while the last fast refresh is showing");
    assert!(k.turn(2000 + KIT_PASS_MS, false));
    // a press is decided too
    let mut k = KitInput::default();
    k.pointer.down(0, 0, 0, -1, 0, slop(), false);
    k.owe();
    k.pointer.tick(HOLD_MS);
    assert!(k.turn(HOLD_MS, false));
    assert_eq!(k.pass_now(true, false), Some(RefreshMode::Fast));
    // the lift: at once
    k.pointer.up(0, 0, false);
    assert_eq!(k.pass_now(false, false), Some(RefreshMode::Ui));
    assert!(!k.turn(HOLD_MS + 1000, false), "no contact, no turn");
}

/// Where the app's clock (`now_ms`, a u32 of milliseconds) may stand when a contact begins: soon after a start, 256 ms
/// before 2^31 ms (24.8 days awake), at it and past it, and near the clock's wrap at 2^32 (49.7 days), 451 ms before
/// it, so that a hold or a pace runs across it.
const UPTIMES: [u32; 6] = [1_000_000, 0x7fff_ff00, 0x8000_0000, 0x8000_0000 + 1_000_000, 0xf000_0000, u32::MAX - 450];

#[test]
fn the_hold_the_pace_and_a_redraw_request_are_durations_the_same_at_any_uptime_and_across_the_wrap() {
    for base in UPTIMES {
        let t = |dt: u32| base.wrapping_add(dt);
        // the hold: a PRESS at HOLD_MS after the down, not before
        let mut p = KitPointer::default();
        p.down(50, 60, 0, -1, t(0), slop(), false);
        assert_eq!(p.tick(t(HOLD_MS - 1)), None, "{base:#x}");
        assert_eq!(p.tick(t(HOLD_MS)).map(|e| e.kind), Some(EV_PRESS), "{base:#x}");
        // the pace: a contact that drags has its first turn at once, then one every KIT_PASS_MS
        let mut k = KitInput::default();
        k.pointer.down(0, 0, 0, -1, t(0), slop(), false);
        k.pointer.moved(200, 0, false);
        assert!(k.turn(t(10), false), "{base:#x}: the first turn at once");
        for n in 1..=5u32 {
            assert!(!k.turn(t(10 + n * KIT_PASS_MS - 1), false), "{base:#x}: turn {n} not before its time");
            assert!(k.turn(t(10 + n * KIT_PASS_MS), false), "{base:#x}: turn {n}");
        }
        // a redraw request asked for 300 ms later: due then, not before; the soonest of two stands
        let r = grimoire::wasmhost::Redraw::asked(None, t(0), 300);
        assert!(!r.due(t(299)) && r.due(t(300)) && r.due(t(5000)), "{base:#x}");
        assert_eq!(grimoire::wasmhost::Redraw::asked(Some(r), t(100), 500), r, "{base:#x}: a later ask does not put it off");
        let sooner = grimoire::wasmhost::Redraw::asked(Some(r), t(100), 100);
        assert!(!sooner.due(t(199)) && sooner.due(t(200)), "{base:#x}: a sooner ask stands in its place");
        // and each still so once 2^31 ms have gone (where a signed difference of the clock turns negative)
        let half = 0x8000_0000u32;
        let mut p = KitPointer::default();
        p.down(50, 60, 0, -1, t(0), slop(), false);
        assert_eq!(p.tick(t(half)).map(|e| e.kind), Some(EV_PRESS), "{base:#x}: a hold 2^31 ms long");
        assert!(k.turn(t(10 + 5 * KIT_PASS_MS + half), false), "{base:#x}: a turn 2^31 ms after the last");
        assert!(r.due(t(300 + half)), "{base:#x}: a request whose time came 2^31 ms ago");
    }
}

#[test]
fn what_fast_refreshes_covered_while_a_contact_was_down_is_owed_one_full_depth_refresh_after_its_lift() {
    let mut k = KitInput::default();
    k.pointer.down(0, 0, 0, -1, 0, slop(), false);
    k.refreshed(Rect::new(10, 10, 100, 50), RefreshMode::Fast);
    k.refreshed(Rect::new(300, 200, 20, 20), RefreshMode::Fast);
    k.refreshed(Rect::new(0, 0, 5, 5), RefreshMode::Ui);
    assert_eq!(k.take_worn(), None, "not while the contact is down");
    k.pointer.up(0, 0, false);
    assert_eq!(k.take_worn(), Some(Rect::new(10, 10, 310, 210)), "all the fast refreshes covered, whatever the lift's pass changed");
    assert_eq!(k.take_worn(), None, "taken once");
    // a contact whose passes were refreshed as the UI alone (a tap's) owes nothing
    k.pointer.down(0, 0, 0, -1, 0, slop(), false);
    k.pointer.up(0, 0, false);
    k.refreshed(Rect::new(10, 10, 100, 50), RefreshMode::Ui);
    assert_eq!(k.take_worn(), None);
}

// ---- what a pass changed ----

#[test]
fn the_part_a_pass_changed_is_the_box_of_the_pixels_that_differ_inside_the_part_kept() {
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, 400, 300), 0);
    canvas_fill(&mut c, Rect::new(0, 0, 400, 300), 255);
    let at = Rect::new(100, 50, 200, 200);
    let (kept, cut) = kit_keep(&c, at).unwrap();
    assert_eq!(cut, at);
    assert!(rect_is_empty(canvas_changed_rect(&kept, &c, cut)), "nothing changed");
    canvas_fill(&mut c, Rect::new(150, 60, 10, 20), 0);
    canvas_fill(&mut c, Rect::new(280, 230, 5, 5), 0);
    // (a change outside the part kept is not the pass's)
    canvas_fill(&mut c, Rect::new(10, 10, 5, 5), 0);
    assert_eq!(canvas_changed_rect(&kept, &c, cut), Rect::new(150, 60, 135, 175));
    // a part reaching off the screen is cut to it
    let (kept, cut) = kit_keep(&c, Rect::new(350, 250, 200, 200)).unwrap();
    assert_eq!(cut, Rect::new(350, 250, 50, 50));
    canvas_fill(&mut c, Rect::new(399, 299, 1, 1), 0);
    assert_eq!(canvas_changed_rect(&kept, &c, cut), Rect::new(399, 299, 1, 1));
    assert!(kit_keep(&c, Rect::new(500, 500, 10, 10)).is_none(), "nothing of it on the screen");
    canvas_free(&mut c);
}

// ---- kit_event, event_points and key_char in both hosts ----

/// A fresh data root for one test (the Makefile's test rules set GRIMOIRE_TEST_TMP), the UI at 300 dpi.
fn fresh_root(name: &str) -> std::path::PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let root = base.join(format!("kitinput-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    ui_init(300.0);
    assert_eq!(font_init(), 0);
    ui_set_theme(UiTheme::Gothic);
    root
}

enum Rt {
    W(WidgetRuntime),
    A(AppRuntime),
}

impl Rt {
    fn draw(&mut self) {
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, SCREEN.0, SCREEN.1), 0);
        match self {
            Rt::W(r) => r.draw(&mut c, WIDGET_BODY),
            Rt::A(r) => r.draw(&mut c, app_body(), 0),
        }
        canvas_free(&mut c);
    }
    fn event(&mut self, e: &KitEvent) -> Option<bool> {
        match self {
            Rt::W(r) => r.event(e),
            Rt::A(r) => r.event(e),
        }
    }
    fn status(&self) -> String {
        match self {
            Rt::W(r) => r.status().to_string(),
            Rt::A(r) => r.status().to_string(),
        }
    }
    fn error(&self) -> Option<&str> {
        match self {
            Rt::W(r) => r.error(),
            Rt::A(r) => r.error(),
        }
    }
    fn fuel(&self) -> u64 {
        match self {
            Rt::W(r) => r.fuel_used(),
            Rt::A(r) => r.fuel_used(),
        }
    }
}

/// Installs a module as a package of its host, loads it (a widget for a frame whose body is WIDGET_BODY) and
/// removes the package.
fn loaded(id: &str, kit: &Kit) -> Rt {
    let module = kit.bytes();
    match kit.host {
        Host::Widget => {
            let m = widget_manifest(id, "Test", &WIDGET_BOTH, 400, 300, &module);
            widget_package_install(&m, &module, "", SCREEN.0, SCREEN.1).unwrap();
            let rt = WidgetRuntime::new(id, (WIDGET_BODY.w, WIDGET_BODY.h)).map(Rt::W);
            widget_package_remove(id, false).unwrap();
            rt.unwrap()
        }
        Host::App => {
            let m = app_manifest(id, "Test", &[], true, &module);
            app_package_install(&m, &module, None, "", SCREEN.0, SCREEN.1).unwrap();
            let rt = AppRuntime::open(id, SCREEN.0, SCREEN.1, false).map(Rt::A);
            app_package_remove(id, false).unwrap();
            rt.unwrap()
        }
    }
}

fn tag(host: Host) -> &'static str {
    match host {
        Host::Widget => "w",
        Host::App => "a",
    }
}

/// Where the probe modules keep what their calls answered (a word each), the points `event_points` wrote, and their
/// status line.
const R_AT: i32 = 2048;
const SAID_AT: i32 = 2200;

/// A probe module: its `kit_event` copies the event's points with `event_points(pts_at, cap)`, asks `key_char` of
/// the event's id and `a` (a key's code and modifiers), and the body's size (`metric` BODY_W, BODY_H), and returns
/// 1; its drawing pass asks `event_points` too (where no event is), and says them all in its status line: `n` the
/// points written, `k` the character, `w` and `h` the body, `x` and `y` the first point, `l` the last point's x,
/// `d` what the pass's own `event_points` wrote.
fn probe(host: Host, pts_at: i32, cap: i32) -> Kit {
    let r = |n: i32| R_AT + 4 * n;
    let event = seq(&[
        store(r(0), seq(&[i(pts_at), i(cap), invoke("event_points")])),
        store(r(1), seq(&[arg(1), arg(2), invoke("key_char")])),
        store(r(2), seq(&[i(36), invoke("metric")])),
        store(r(3), seq(&[i(37), invoke("metric")])),
        // (when it wrote one: the first point, and the last, at pts_at + 8 x (n - 1))
        when(
            load(r(0)),
            seq(&[store(r(4), load(pts_at)), store(r(5), load(pts_at + 4)), store(r(6), seq(&[load(r(0)), i(8), vec![MUL], i(pts_at - 8), vec![ADD], vec![0x28, 0x02, 0x00]]))]),
        ),
        i(1),
    ]);
    let fields: [(&str, usize, i32); 8] = [("n", 3, r(0)), ("k", 5, r(1)), ("w", 5, r(2)), ("h", 5, r(3)), ("x", 5, r(4)), ("y", 5, r(5)), ("l", 5, r(6)), ("d", 3, r(7))];
    let mut say = Vec::new();
    let mut template = String::new();
    for (letter, n, from) in fields {
        if !template.is_empty() {
            template.push(' ');
        }
        template.push_str(letter);
        let at = SAID_AT + template.len() as i32;
        template.push_str(&"0".repeat(n));
        say.extend(seq(&[load(from), digits_of(at, n)]));
    }
    let draw = seq(&[store(r(7), seq(&[i(pts_at), i(64), invoke("event_points")])), say, host_status(SAID_AT, template.len() as i32)]);
    let mut k = Kit::new(host, draw);
    k.data = vec![(SAID_AT as u32, template.into_bytes())];
    k.event = Some(event);
    k
}

fn host_status(ptr: i32, len: i32) -> Vec<u8> {
    kitfix::host("status", &[ptr, len])
}

/// A number of a probe's status line, by its letter.
fn said(s: &str, letter: char) -> i32 {
    s.split(' ').find(|f| f.starts_with(letter)).and_then(|f| f[1..].parse().ok()).unwrap_or_else(|| panic!("no {letter} in {s}"))
}

fn drag_move(points: &[(i32, i32)]) -> KitEvent {
    let &(x, y) = points.last().unwrap_or(&(0, 0));
    KitEvent { points: points.to_vec(), ..KitEvent::new(EV_DRAG_MOVE, -1, x, y, points.len() as i32) }
}

#[test]
fn kit_event_is_called_with_the_events_numbers_and_says_whether_the_picture_changed() {
    let root = fresh_root("event");
    for host in HOSTS {
        // every event answered: TAP, PRESS and the drag's, counted by kind; the touch's place and region kept
        let mut rt = loaded(&format!("test.ki.all.{}", tag(host)), &input(host, true, Some(true)));
        for e in [KitEvent::new(EV_TAP, 1, 30, 40, 0), KitEvent::new(EV_PRESS, 2, 50, 60, 1), KitEvent::new(EV_DRAG_START, -1, 1, 2, 0), drag_move(&[(5, 6), (7, 8), (9, 10)]), KitEvent::new(EV_DRAG_END, -1, 9, 10, 1), KitEvent::new(EV_SETTINGS, 0, 0, 0, 0)] {
            assert_eq!(rt.event(&e), Some(true), "{host:?} {e:?}");
        }
        rt.draw();
        assert_eq!(rt.status(), "p001 n000 t01 r01 s01 m01 e01 c01 q003 k00 a00 g01 h0", "{host:?}");
        // TAP alone answered
        let mut rt = loaded(&format!("test.ki.quiet.{}", tag(host)), &input(host, false, Some(false)));
        assert_eq!(rt.event(&KitEvent::new(EV_TAP, 1, 30, 40, 0)), Some(true));
        assert_eq!(rt.event(&drag_move(&[(1, 1)])), Some(false), "{host:?}: a DRAG_MOVE that returned 0 changed nothing");
        assert_eq!(rt.event(&KitEvent::new(EV_KEY, 30, 0, 1, 0)), Some(false));
        // no kit_event: never called
        let mut rt = loaded(&format!("test.ki.none.{}", tag(host)), &input(host, true, None));
        assert_eq!(rt.event(&KitEvent::new(EV_TAP, 1, 30, 40, 0)), None, "{host:?}");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn event_points_writes_whole_points_within_its_room_and_the_memory_and_none_outside_a_drag_move() {
    let root = fresh_root("points");
    let pts: Vec<(i32, i32)> = (0..256).map(|k| (1000 + k, 2000 + k)).collect();
    for host in HOSTS {
        // (cap, the place, the points of the event) -> the points written, the first, the last x
        for (cap, at, n, want) in [
            (2048, 4096, 256, (256, (1000, 2000), 1255)),
            (4096, 4096, 256, (256, (1000, 2000), 1255)), // (more room than 256 points: still 256)
            (2047, 4096, 256, (255, (1000, 2000), 1254)), // (whole points only)
            (8, 4096, 256, (1, (1000, 2000), 1000)),
            (7, 4096, 256, (0, (0, 0), 0)),
            (0, 4096, 256, (0, (0, 0), 0)),
            (-5, 4096, 256, (0, (0, 0), 0)),
            (2048, 4096, 3, (3, (1000, 2000), 1002)),
            (2048, 4096, 0, (0, (0, 0), 0)),
            (2048, 65536 - 20, 256, (2, (1000, 2000), 1001)), // (the memory's end: 20 bytes there, two whole points)
            (2048, 65536, 256, (0, (0, 0), 0)),
            (2048, -8, 256, (0, (0, 0), 0)),
        ] {
            let id = format!("test.ki.pts.{}.{}.{}.{n}", tag(host), cap.max(0), at.max(0));
            let mut rt = loaded(&id, &probe(host, at, cap));
            assert_eq!(rt.event(&drag_move(&pts[..n])), Some(true));
            rt.draw();
            let s = rt.status();
            let (wn, (wx, wy), wl) = want;
            assert_eq!((said(&s, 'n'), said(&s, 'x'), said(&s, 'y'), said(&s, 'l')), (wn, wx, wy, wl), "{host:?} cap {cap} at {at} n {n}: {s}");
            assert_eq!(said(&s, 'd'), 0, "{host:?}: a drawing pass is no DRAG_MOVE: {s}");
            // and in another event, nothing
            assert_eq!(rt.event(&KitEvent::new(EV_TAP, 1, 0, 0, 0)), Some(true));
            rt.draw();
            assert!(rt.status().starts_with("n000"), "{host:?}: a TAP has no points: {}", rt.status());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn key_char_is_the_character_a_hardware_key_types_and_0_for_none() {
    let root = fresh_root("keychar");
    for host in HOSTS {
        let mut rt = loaded(&format!("test.ki.kc.{}", tag(host)), &probe(host, 4096, 0));
        // (code, mods) -> the character: A, Shift+A, 1, Shift+1, Space, keypad 7, Enter, Ctrl+A, an arrow, a far code
        for (code, mods, want) in [(30, 0, 'a' as i32), (30, KEYMOD_SHIFT, 'A' as i32), (2, 0, '1' as i32), (2, KEYMOD_SHIFT, '!' as i32), (57, 0, ' ' as i32), (71, 0, '7' as i32), (28, 0, 0), (30, KEYMOD_CTRL, 0), (106, 0, 0), (100_000, 0, 0), (-1, 0, 0)] {
            assert_eq!(rt.event(&KitEvent::new(EV_KEY, code, mods, 1, 0)), Some(true));
            rt.draw();
            let s = rt.status();
            assert_eq!(said(&s, 'k'), want, "{host:?} key {code} mods {mods}: {s}");
            assert_eq!(keyboard_hw_char(code, mods as u32).map_or(0, |c| c as i32), want, "the same map as Grimoire's keyboards");
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_module_has_its_body_from_its_first_call_on_before_its_first_pass() {
    let root = fresh_root("body");
    for host in HOSTS {
        let mut rt = loaded(&format!("test.ki.body.{}", tag(host)), &probe(host, 4096, 0));
        // (an event before any pass: BODY_W and BODY_H are the frame's body, the app's under its status bar)
        assert_eq!(rt.event(&KitEvent::new(EV_TAP, -1, 0, 0, 0)), Some(true));
        rt.draw();
        let s = rt.status();
        let want = match host {
            Host::Widget => (WIDGET_BODY.w, WIDGET_BODY.h),
            Host::App => (app_body().w, app_body().h),
        };
        assert_eq!((said(&s, 'w'), said(&s, 'h')), want, "{host:?}: {s}");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn an_event_has_the_budget_and_the_stop_rules_of_the_hosts_other_calls() {
    let root = fresh_root("budget");
    for host in HOSTS {
        let mut k = input(host, true, Some(true));
        k.event = Some(seq(&[forever(&[]), i(1)]));
        let mut rt = loaded(&format!("test.ki.loop.{}", tag(host)), &k);
        assert_eq!(rt.event(&KitEvent::new(EV_TAP, 1, 0, 0, 0)), Some(true), "{host:?}: a call that stopped changed the picture");
        let budget = match host {
            Host::Widget => grimoire::widgetwasm::WIDGET_CALL_FUEL,
            Host::App => grimoire::apphost::APP_CALL_FUEL,
        };
        assert_eq!(rt.fuel(), budget, "{host:?}: stopped when its budget was used up");
        let says = match host {
            Host::Widget => "The widget stopped while handling an event.",
            Host::App => "The app stopped while handling an event.",
        };
        assert_eq!(rt.error(), Some(says));
        assert_eq!(rt.event(&KitEvent::new(EV_TAP, 1, 0, 0, 0)), None, "{host:?}: not called again");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn event_points_and_key_char_take_their_charges_from_the_events_budget() {
    let root = fresh_root("charges");
    let pts: Vec<(i32, i32)> = (0..256).map(|k| (k, k)).collect();
    for host in HOSTS {
        // (the same event, its kit_event with and without the call; each the module's second call of it)
        let fuel = |id: &str, event: Vec<u8>| -> u64 {
            let mut k = Kit::new(host, Vec::new());
            k.event = Some(seq(&[event, i(1)]));
            let mut rt = loaded(&format!("test.ki.ch.{}.{id}", tag(host)), &k);
            rt.event(&drag_move(&pts));
            rt.event(&drag_move(&pts));
            assert_eq!(rt.error(), None);
            rt.fuel()
        };
        let none = fuel("none", Vec::new());
        let all = fuel("all", call("event_points", &[4096, 2048]));
        let room = fuel("room", call("event_points", &[4096, 7]));
        let key = fuel("key", call("key_char", &[30, 0]));
        let near = |got: u64, want: u64, what: &str| assert!(got.abs_diff(want) <= 64, "{host:?} {what}: charged {got}, expected {want}");
        near(all - none, KIT_CHARGE_CALL + 2048 / 4, "event_points of 256 points: 500, and 1 for every 4 bytes");
        near(room - none, KIT_CHARGE_CALL, "event_points with no room for a point: 500");
        near(key - none, KIT_CHARGE_CALL, "key_char: 500");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn an_open_app_is_told_its_settings_were_saved_and_drawn_once_for_it() {
    let root = fresh_root("settings");
    let id = "test.ki.settings";
    let module = input(Host::App, true, Some(true)).bytes();
    // (with the status bar hidden: its minute brings passes of its own)
    let m = format!("{}{INPUT_SETTINGS}", app_manifest(id, "Settings", &[], false, &module));
    app_package_install(&m, &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    assert_eq!(a.app_view.status(), "p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0");
    // (what the app's settings page does when the reader changes a value, the app open)
    assert_eq!(app_settings_save(id, &[("mode".to_string(), "false".to_string())]), 0);
    a.app_view.reload_settings();
    assert_eq!(a.app_view.status(), "p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0", "the pass is owed, not drawn at once");
    grimoire::apphost::apphost_tick(&mut a, now_ms());
    assert_eq!(a.app_view.status(), "p002 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g01 h0", "one SETTINGS event, and one pass for it");
    grimoire::apphost::apphost_tick(&mut a, now_ms());
    assert_eq!(a.app_view.status(), "p002 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g01 h0", "and no other");
    grimoire::apphost::apphost_close(&mut a);
    grimoire::library::library_free(&mut a);
    canvas_free(&mut a.screen);
    let _ = std::fs::remove_dir_all(root);
}

/// A module whose start section asks for the body's size (`metric` BODY_W, BODY_H), and whose drawing passes say
/// what it was given there, as `w` and `h`, and their own body, as `x` and `y`.
fn starting(host: Host) -> Kit {
    let r = |n: i32| R_AT + 4 * n;
    let mut k = Kit::new(host, Vec::new());
    k.start = Some(seq(&[store(r(0), seq(&[i(36), invoke("metric")])), store(r(1), seq(&[i(37), invoke("metric")]))]));
    let template = "w00000 h00000 x00000 y00000";
    let mut say = Vec::new();
    for (at, from) in [(1, r(0)), (8, r(1))] {
        say.extend(seq(&[load(from), digits_of(SAID_AT + at, 5)]));
    }
    for (at, id) in [(15, 36), (22, 37)] {
        say.extend(seq(&[i(id), invoke("metric"), digits_of(SAID_AT + at, 5)]));
    }
    k.code[0] = seq(&[say, host_status(SAID_AT, template.len() as i32)]);
    k.data = vec![(SAID_AT as u32, template.as_bytes().to_vec())];
    k
}

#[test]
fn a_frames_module_and_an_apps_have_their_body_in_their_start_section() {
    let root = fresh_root("start");
    // a frame loads its module with the frame's body (WidgetFrame::ensure_runtime), which its start section is given
    let id = "test.ki.start.w";
    let module = starting(Host::Widget).bytes();
    widget_package_install(&widget_manifest(id, "Start", &WIDGET_BOTH, 400, 300, &module), &module, "", SCREEN.0, SCREEN.1).unwrap();
    let mut f = grimoire::widgets::WidgetFrame::new(grimoire::widgets::widget_def(id).unwrap());
    f.layout = grimoire::widgets::WidgetLayout { x: 100, y: 200, w: 700, h: 900, minimized: false, z: 0 };
    let body = f.body_rect();
    f.ensure_runtime();
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, SCREEN.0, SCREEN.1), 0);
    let rt = f.runtime.as_mut().unwrap();
    rt.draw(&mut c, body);
    assert_eq!(rt.status(), format!("w{:05} h{:05} x{:05} y{:05}", body.w, body.h, body.w, body.h), "the frame's body, from the start");
    widget_package_remove(id, false).unwrap();
    // an app's start section is given the screen under the status bar its manifest asks for
    let mut rt = loaded("test.ki.start.a", &starting(Host::App));
    rt.draw();
    let b = app_body();
    assert_eq!(rt.status(), format!("w{:05} h{:05} x{:05} y{:05}", b.w, b.h, b.w, b.h));
    canvas_free(&mut c);
    let _ = std::fs::remove_dir_all(root);
}

/// An app's `elapsed_ms` is the time since its last drawing pass by the app's clock, which the script holds here (the
/// machine's clock moves on meanwhile), across the clock's wrap too.
#[cfg(feature = "sim")]
#[test]
fn an_apps_elapsed_time_is_counted_on_the_apps_clock() {
    let root = fresh_root("elapsed");
    let id = "test.ki.elapsed";
    let mut k = Kit::new(Host::App, seq(&[arg(3), digits_of(SAID_AT, 6), host_status(SAID_AT, 6)]));
    k.data = vec![(SAID_AT as u32, b"000000".to_vec())];
    let module = k.bytes();
    app_package_install(&app_manifest(id, "Elapsed", &[], false, &module), &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    sim_clock_set(Some(5000));
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    assert_eq!(a.app_view.status(), "000000", "the first pass, at the launch");
    std::thread::sleep(std::time::Duration::from_millis(30));
    sim_clock_set(Some(5000 + 1234));
    grimoire::apphost::apphost_draw(&mut a);
    assert_eq!(a.app_view.status(), "001234", "1,234 ms of the app's clock, whatever the machine's did");
    sim_clock_set(Some(5000 + 1234));
    grimoire::apphost::apphost_draw(&mut a);
    assert_eq!(a.app_view.status(), "000000", "none since");
    sim_clock_set(Some(u32::MAX - 99));
    grimoire::apphost::apphost_draw(&mut a);
    sim_clock_set(Some(400));
    grimoire::apphost::apphost_draw(&mut a);
    assert_eq!(a.app_view.status(), "000500", "across the clock's wrap");
    sim_clock_set(None);
    grimoire::apphost::apphost_close(&mut a);
    grimoire::library::library_free(&mut a);
    canvas_free(&mut a.screen);
    let _ = std::fs::remove_dir_all(root);
}

/// A drawing pass that input brings when a redraw request's time has come answers the request too: one pass, not
/// two, in that tick; a request whose time has not come stands through it.
#[cfg(feature = "sim")]
#[test]
fn an_input_pass_drawn_when_a_request_is_due_answers_it() {
    let root = fresh_root("dueinput");
    let id = "test.ki.dueinput";
    // (each pass counts itself in the status line and asks to be drawn again 100 ms later; app_key is there, empty)
    let count = seq(&[bump(R_AT, i(1)), load(R_AT), digits_of(SAID_AT, 6), host_status(SAID_AT, 6)]);
    let mut k = Kit::new(Host::App, Vec::new());
    k.imports.push(("grimoire", "request_redraw", "i", 'v'));
    k.code[0] = seq(&[count, extra(&k, 0, &[100])]);
    k.code.extend([Vec::new(), Vec::new()]);
    k.data = vec![(SAID_AT as u32, b"000000".to_vec())];
    let module = k.bytes();
    app_package_install(&app_manifest(id, "Due", &[], false, &module), &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    sim_clock_set(Some(10_000));
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    assert_eq!(a.app_view.status(), "000001");
    // a key 50 ms later: its pass, and the request (due at 10,100) stands
    sim_clock_set(Some(10_050));
    grimoire::apphost::apphost_key(&mut a, 30, 0, true);
    grimoire::apphost::apphost_tick(&mut a, 10_050);
    assert_eq!(a.app_view.status(), "000002");
    // a key when the request is due: one pass answers both
    sim_clock_set(Some(10_150));
    grimoire::apphost::apphost_key(&mut a, 30, 0, false);
    grimoire::apphost::apphost_tick(&mut a, 10_150);
    assert_eq!(a.app_view.status(), "000003", "one pass for the key and the request");
    // and the request that pass asked for comes 100 ms later, once
    sim_clock_set(Some(10_249));
    grimoire::apphost::apphost_tick(&mut a, 10_249);
    assert_eq!(a.app_view.status(), "000003");
    sim_clock_set(Some(10_250));
    grimoire::apphost::apphost_tick(&mut a, 10_250);
    assert_eq!(a.app_view.status(), "000004");
    sim_clock_set(None);
    grimoire::apphost::apphost_close(&mut a);
    grimoire::library::library_free(&mut a);
    canvas_free(&mut a.screen);
    let _ = std::fs::remove_dir_all(root);
}

/// In an open app, a contact the panel calls palm-sized, or one the platform cancels (a gesture took it, the pen came),
/// is no TAP; app_pointer is still called for each of its phases, and its pass drawn.
#[test]
fn an_apps_palm_and_its_cancelled_contact_are_no_tap() {
    let root = fresh_root("palm");
    let id = "test.ki.palm";
    let module = input(Host::App, true, Some(true)).bytes();
    app_package_install(&app_manifest(id, "Palm", &[], false, &module), &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    let touch = |a: &mut App, type_: InType, x: f32, y: f32, value: i32| {
        assert!(grimoire::apphost::apphost_event(a, &InEvent { type_, x, y, value, t_ms: now_ms(), ..Default::default() }));
    };
    // a palm: its down and its lift
    touch(&mut a, InType::TouchDown, 600.0, 1000.0, TOUCH_F_BROAD);
    touch(&mut a, InType::TouchUp, 600.0, 1000.0, TOUCH_F_BROAD);
    grimoire::apphost::apphost_tick(&mut a, now_ms());
    assert_eq!(a.app_view.status(), "p002 n002 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0", "no TAP for a palm");
    // a finger the platform cancels: the off-screen lift
    touch(&mut a, InType::TouchDown, 600.0, 1000.0, 0);
    touch(&mut a, InType::TouchUp, -10000.0, -10000.0, TOUCH_F_CANCEL);
    grimoire::apphost::apphost_tick(&mut a, now_ms());
    assert_eq!(a.app_view.status(), "p003 n004 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0", "no TAP for a cancelled contact");
    // and a finger that lifts where it came down: a TAP
    touch(&mut a, InType::TouchDown, 600.0, 1000.0, 0);
    touch(&mut a, InType::TouchUp, 600.0, 1000.0, 0);
    grimoire::apphost::apphost_tick(&mut a, now_ms());
    assert_eq!(a.app_view.status(), "p004 n006 t01 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0");
    grimoire::apphost::apphost_close(&mut a);
    grimoire::library::library_free(&mut a);
    canvas_free(&mut a.screen);
    let _ = std::fs::remove_dir_all(root);
}

/// A widget's redraw request stands across its other calls, as an app's does: a tap (`widget_tap`) or an event does not
/// drop the one a pass asked for, and a later call's later request does not put it off (the soonest stands).
#[test]
fn a_widgets_redraw_request_stands_across_its_other_calls_the_soonest_kept() {
    let root = fresh_root("request");
    for events in [false, true] {
        let id = format!("test.ki.request.{events}");
        let mut k = Kit::new(Host::Widget, Vec::new());
        k.imports.push(("grimoire", "request_redraw", "i", 'v'));
        k.code[0] = extra(&k, 0, &[1000]);
        let asks_later = extra(&k, 0, &[5000]);
        k = k.touch(asks_later);
        if events {
            k.event = Some(seq(&[extra(&k, 0, &[5000]), i(0)]));
        }
        let module = k.bytes();
        widget_package_install(&widget_manifest(&id, "Request", &WIDGET_BOTH, 400, 300, &module), &module, "", SCREEN.0, SCREEN.1).unwrap();
        let mut rt = WidgetRuntime::new(&id, (WIDGET_BODY.w, WIDGET_BODY.h)).unwrap();
        widget_package_remove(&id, false).unwrap();
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, SCREEN.0, SCREEN.1), 0);
        let t = now_ms();
        rt.draw(&mut c, WIDGET_BODY);
        // (a tap, or a TAP event, that asks for a pass 5 s later)
        if events {
            assert_eq!(rt.event(&KitEvent::new(EV_TAP, -1, 0, 0, 0)), Some(false));
        } else {
            assert!(rt.tap(5, 5));
        }
        assert!(!rt.redraw_due(t.wrapping_add(900)), "events {events}: not before its time");
        assert!(rt.redraw_due(t.wrapping_add(1000 + 500)), "events {events}: the pass's request stands, the soonest");
        canvas_free(&mut c);
    }
    let _ = std::fs::remove_dir_all(root);
}

/// An app's passes while a finger drags and while it is held are the same whatever the app's clock reads when the
/// contact begins (UPTIMES: past 2^31 ms too, and across its wrap at 2^32): a drag's first turn at once and its
/// DRAG_MOVEs at the pace, a press at HOLD_MS, a pass on each.
#[cfg(feature = "sim")]
#[test]
fn an_apps_passes_while_a_finger_drags_or_is_held_are_the_same_at_any_uptime() {
    let root = fresh_root("uptime");
    let id = "test.ki.uptime";
    let module = input(Host::App, true, Some(true)).bytes();
    app_package_install(&app_manifest(id, "Uptime", &[], false, &module), &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut seen = Vec::new();
    for base in UPTIMES {
        sim_clock_set(Some(base));
        let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        grimoire::library::library_init(&mut a);
        assert!(grimoire::apphost::apphost_launch(&mut a, id));
        // (each touch at the app's clock as it stands; each tick `dt` ms after the first touch)
        let touch = |a: &mut App, type_: InType, x: f32, y: f32| {
            assert!(grimoire::apphost::apphost_event(a, &InEvent { type_, x, y, t_ms: now_ms(), ..Default::default() }));
        };
        let tick = |a: &mut App, dt: u32| {
            sim_clock_set(Some(base.wrapping_add(dt)));
            grimoire::apphost::apphost_tick(a, base.wrapping_add(dt));
        };
        touch(&mut a, InType::TouchDown, 300.0, 1000.0);
        tick(&mut a, 100);
        touch(&mut a, InType::TouchMove, 500.0, 1000.0);
        tick(&mut a, 200);
        touch(&mut a, InType::TouchMove, 600.0, 1000.0);
        tick(&mut a, 400);
        touch(&mut a, InType::TouchMove, 700.0, 1000.0);
        tick(&mut a, 600);
        let during = a.app_view.status().to_string();
        touch(&mut a, InType::TouchUp, 700.0, 1000.0);
        tick(&mut a, 700);
        let after = a.app_view.status().to_string();
        touch(&mut a, InType::TouchDown, 900.0, 1500.0);
        tick(&mut a, 800);
        tick(&mut a, 700 + HOLD_MS - 1);
        let not_yet = a.app_view.status().to_string();
        tick(&mut a, 700 + HOLD_MS);
        let held = a.app_view.status().to_string();
        touch(&mut a, InType::TouchUp, 900.0, 1500.0);
        tick(&mut a, 1500);
        grimoire::apphost::apphost_close(&mut a);
        grimoire::library::library_free(&mut a);
        canvas_free(&mut a.screen);
        seen.push((base, [during, after, not_yet, held]));
    }
    sim_clock_set(None);
    // (a pass on each of the three turns; at the lift a DRAG_MOVE of its place and DRAG_END, and a pass; held under
    // HOLD_MS no pass, the status line still the last pass's; at HOLD_MS a PRESS and a pass)
    let want = [
        "p004 n004 t00 r00 s01 m03 e00 c00 q003 k00 a00 g00 h0",
        "p005 n005 t00 r00 s01 m04 e01 c00 q004 k00 a00 g00 h0",
        "p005 n005 t00 r00 s01 m04 e01 c00 q004 k00 a00 g00 h0",
        "p006 n006 t00 r01 s01 m04 e01 c00 q004 k00 a00 g00 h0",
    ];
    for (base, got) in seen {
        assert_eq!(got, want, "the clock from {base:#x}: during the drag, after its lift, held under HOLD_MS, held HOLD_MS");
    }
    let _ = app_package_remove(id, false);
    let _ = std::fs::remove_dir_all(root);
}

/// The scenarios of the kit's input run again with app time started past 2^31 ms and across its wrap at 2^32
/// (z_kit_input_app_half.qs and _wrap.qs, z_kit_input_widget_half.qs and _wrap.qs) are their bases line for line
/// but for the start (GRIMOIRE_SIM_UPTIME_MS) and a note: they expect the same passes and refreshes.
#[test]
fn the_kit_input_scenarios_at_other_uptimes_are_their_bases_but_for_the_start() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/scenarios");
    for (base, tag, ms) in [("z_kit_input_app", "half", 2147480000u32), ("z_kit_input_app", "wrap", 4294961000), ("z_kit_input_widget", "half", 2147480000), ("z_kit_input_widget", "wrap", 4294961000)] {
        let b = std::fs::read_to_string(dir.join(format!("{base}.qs"))).unwrap();
        let c = std::fs::read_to_string(dir.join(format!("{base}_{tag}.qs"))).unwrap();
        let (b, c): (Vec<&str>, Vec<&str>) = (b.lines().collect(), c.lines().collect());
        let env = format!("{} GRIMOIRE_SIM_UPTIME_MS={ms}", b[2]);
        assert_eq!((c[0], c[1], c[2], c[3]), (b[0], b[1], env.as_str(), b[3]), "{base}_{tag}.qs: its first lines");
        let rest = &c[c.len() - (b.len() - 4)..];
        assert_eq!(rest, &b[4..], "{base}_{tag}.qs: the lines after its note are {base}.qs's from its fifth on");
        assert!(c[4..c.len() - rest.len()].iter().all(|l| l.starts_with("# ")), "{base}_{tag}.qs: a note of comment lines");
    }
}

/// A frame's pass answers a redraw request whose time has come only when it draws the whole body: one drawn with the
/// canvas clipped to part of the body (a region of another frame's, or of the page, that the editor composes) leaves
/// the request standing for the editor's tick, which draws the frame whole.
#[cfg(feature = "sim")]
#[test]
fn a_frames_due_request_is_answered_only_by_a_pass_of_its_whole_body() {
    let root = fresh_root("partial");
    let id = "test.ki.partial";
    let mut k = Kit::new(Host::Widget, Vec::new());
    k.imports.push(("grimoire", "request_redraw", "i", 'v'));
    k.code[0] = extra(&k, 0, &[1000]);
    let module = k.bytes();
    widget_package_install(&widget_manifest(id, "Partial", &WIDGET_BOTH, 400, 300, &module), &module, "", SCREEN.0, SCREEN.1).unwrap();
    let mut rt = WidgetRuntime::new(id, (WIDGET_BODY.w, WIDGET_BODY.h)).unwrap();
    widget_package_remove(id, false).unwrap();
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, SCREEN.0, SCREEN.1), 0);
    sim_clock_set(Some(50_000));
    rt.draw(&mut c, WIDGET_BODY);
    sim_clock_set(Some(51_500));
    assert!(rt.redraw_due(51_500), "the pass's request, due");
    // a pass of the body's lower half
    let whole = c.clip;
    c.clip = Rect { y: WIDGET_BODY.y + WIDGET_BODY.h / 2, h: WIDGET_BODY.h / 2, ..WIDGET_BODY };
    rt.draw(&mut c, WIDGET_BODY);
    assert!(rt.redraw_due(51_500), "a pass of part of the body leaves it standing");
    c.clip = whole;
    rt.draw(&mut c, WIDGET_BODY);
    assert!(!rt.redraw_due(51_500) && rt.redraw_due(52_500), "a pass of the whole body answers it, and asks again");
    sim_clock_set(None);
    canvas_free(&mut c);
    let _ = std::fs::remove_dir_all(root);
}

/// In an open app the recogniser takes a contact's moves and its lift only from the source that began it, as the
/// editor does: the pen coming down while a finger drags ends the finger's drag (cancelled) and begins the pen's
/// contact, which the finger's later samples do not move; app_pointer is still told of every phase.
#[test]
fn an_apps_contact_takes_its_moves_and_lift_from_the_source_that_began_it() {
    let root = fresh_root("source");
    let id = "test.ki.source";
    let module = input(Host::App, true, Some(true)).bytes();
    app_package_install(&app_manifest(id, "Source", &[], false, &module), &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    let ev = |a: &mut App, type_: InType, x: f32, y: f32| {
        assert!(grimoire::apphost::apphost_event(a, &InEvent { type_, x, y, t_ms: now_ms(), ..Default::default() }));
    };
    let tick = |a: &mut App, ms: u32| grimoire::apphost::apphost_tick(a, now_ms().wrapping_add(ms));
    // a finger drags; the pen comes down elsewhere; the finger moves and lifts; the pen taps where it came down
    ev(&mut a, InType::TouchDown, 300.0, 1000.0);
    ev(&mut a, InType::TouchMove, 500.0, 1000.0);
    tick(&mut a, 200);
    ev(&mut a, InType::PenDown, 1200.0, 2000.0);
    ev(&mut a, InType::TouchMove, 700.0, 1000.0);
    ev(&mut a, InType::TouchUp, 700.0, 1000.0);
    ev(&mut a, InType::PenMove, 1210.0, 2005.0);
    ev(&mut a, InType::PenUp, 1210.0, 2005.0);
    tick(&mut a, 400);
    assert_eq!(a.app_view.status(), "p003 n007 t01 r00 s01 m01 e01 c01 q001 k00 a00 g00 h0", "the finger's drag ended cancelled, the pen's tap a TAP");
    // a finger resting while the pen taps: the pen's TAP, and the finger's lift is nothing
    ev(&mut a, InType::TouchDown, 300.0, 1000.0);
    ev(&mut a, InType::PenDown, 1200.0, 2000.0);
    ev(&mut a, InType::PenUp, 1200.0, 2000.0);
    ev(&mut a, InType::TouchUp, 300.0, 1000.0);
    tick(&mut a, 600);
    assert_eq!(a.app_view.status(), "p004 n011 t02 r00 s01 m01 e01 c01 q001 k00 a00 g00 h0");
    grimoire::apphost::apphost_close(&mut a);
    grimoire::library::library_free(&mut a);
    canvas_free(&mut a.screen);
    let _ = app_package_remove(id, false);
    let _ = std::fs::remove_dir_all(root);
}

/// A contact that began in an app's body is seen whole by the recogniser, also where it leaves the body (into the
/// status bar over it), as a frame's is: one that leaves the body and comes back to lift within the slop before
/// HOLD_MS is a drag in both hosts, no TAP, its samples outside the body among its points. app_pointer is told of
/// what it always was: the samples inside the body, and the lift.
#[test]
fn an_apps_drag_is_seen_whole_where_it_leaves_the_body_and_app_pointer_only_inside() {
    let root = fresh_root("outside");
    let id = "test.ki.outside";
    let module = input(Host::App, true, Some(true)).bytes();
    app_package_install(&app_manifest(id, "Outside", &[], true, &module), &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    let bar = grimoire::globalui::gui_status_h();
    assert!(bar > 30, "the app shows the status bar over its body");
    // (the status bar's first minute: a pass of its own)
    grimoire::apphost::apphost_tick(&mut a, now_ms());
    let ev = |a: &mut App, type_: InType, x: f32, y: f32| {
        assert!(grimoire::apphost::apphost_event(a, &InEvent { type_, x, y, t_ms: now_ms(), ..Default::default() }));
    };
    ev(&mut a, InType::TouchDown, 600.0, 1000.0);
    ev(&mut a, InType::TouchMove, 600.0, 10.0);
    ev(&mut a, InType::TouchMove, 610.0, 1000.0);
    ev(&mut a, InType::TouchUp, 610.0, 1000.0);
    grimoire::apphost::apphost_tick(&mut a, now_ms());
    assert_eq!(a.app_view.status(), "p003 n003 t00 r00 s01 m01 e01 c00 q002 k00 a00 g00 h0", "a drag of two points; app_pointer told of three phases");
    // a contact that begins in the status bar is none of the app's
    ev(&mut a, InType::TouchDown, 600.0, 10.0);
    ev(&mut a, InType::TouchMove, 600.0, 1000.0);
    ev(&mut a, InType::TouchUp, 600.0, 1000.0);
    grimoire::apphost::apphost_tick(&mut a, now_ms());
    assert_eq!(a.app_view.status(), "p004 n005 t00 r00 s01 m01 e01 c00 q002 k00 a00 g00 h0", "app_pointer told of the move and the lift inside, as before; no event");
    grimoire::apphost::apphost_close(&mut a);
    grimoire::library::library_free(&mut a);
    canvas_free(&mut a.screen);
    let _ = app_package_remove(id, false);
    let _ = std::fs::remove_dir_all(root);
}

/// What a flood of input makes a module do between two screen updates (KIT.md, How a pass follows input): an input
/// event makes at most three events for `kit_event` (a lift past the slop whose moves were not seen: DRAG_START,
/// DRAG_MOVE and DRAG_END), whatever the contact was, and a tick at most one more (a PRESS or a DRAG_MOVE); the main
/// loop's batch ends at INPUT_BATCH_MAX events. In an app, the review's case: a batch of downs and lifts past the slop.
#[test]
fn an_input_event_makes_at_most_three_events_and_a_tick_one_more() {
    // the recogniser in each of its states, given each kind of sample
    let states: [(&str, fn() -> KitPointer); 6] = [
        ("up", KitPointer::default),
        ("down", || { let mut p = KitPointer::default(); p.down(0, 0, 0, -1, 0, slop(), false); p }),
        ("pressed", || { let mut p = KitPointer::default(); p.down(0, 0, 0, -1, 0, slop(), false); p.tick(HOLD_MS); p }),
        ("dragging", || { let mut p = KitPointer::default(); p.down(0, 0, 0, -1, 0, slop(), false); p.moved(200, 0, false); p }),
        ("dragging, its samples handed over", || { let mut p = KitPointer::default(); p.down(0, 0, 0, -1, 0, slop(), false); p.moved(200, 0, false); p.take_moves(); p }),
        ("cancelled", || { let mut p = KitPointer::default(); p.down(0, 0, 0, -1, 0, slop(), true); p }),
    ];
    let samples: [(&str, fn(&mut KitPointer) -> usize); 8] = [
        ("a down", |p| p.down(500, 500, 0, -1, 0, slop(), false).len()),
        ("a palm's down", |p| p.down(500, 500, 0, -1, 0, slop(), true).len()),
        ("a move within the slop", |p| p.moved(1, 1, false).len()),
        ("a move past it", |p| p.moved(900, 900, false).len()),
        ("a palm's move", |p| p.moved(900, 900, true).len()),
        ("a lift within the slop", |p| p.up(1, 1, false).len()),
        ("a lift past it", |p| p.up(900, 900, false).len()),
        ("a cancel", |p| p.up(-10000, -10000, true).len()),
    ];
    let mut most = 0;
    for (state, make) in states {
        for (sample, give) in samples {
            let n = give(&mut make());
            assert!(n <= 3, "{sample} on a contact {state}: {n} events");
            most = most.max(n);
        }
        let mut p = make();
        let n = p.tick(HOLD_MS * 2).into_iter().count() + p.take_moves().into_iter().count();
        assert!(n <= 1, "a tick of a contact {state}: {n} events");
    }
    assert_eq!(most, 3, "a lift past the slop of a contact not yet a drag makes three");
    // an app given a batch of INPUT_BATCH_MAX events, each lift past the slop, and its tick
    let root = fresh_root("batch");
    let id = "test.ki.batch";
    let module = input(Host::App, true, Some(true)).bytes();
    app_package_install(&app_manifest(id, "Batch", &[], false, &module), &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    let batch = grimoire::mainloop::INPUT_BATCH_MAX as i32;
    assert_eq!((batch, grimoire::mainloop::INPUT_BATCH_MS), (64, 8));
    for k in 0..batch / 2 {
        for (type_, x) in [(InType::TouchDown, 300.0), (InType::TouchUp, 900.0)] {
            assert!(grimoire::apphost::apphost_event(&mut a, &InEvent { type_, x, y: 1000.0 + k as f32, t_ms: now_ms(), ..Default::default() }));
        }
    }
    grimoire::apphost::apphost_tick(&mut a, now_ms());
    let s = a.app_view.status().to_string();
    let events: i32 = ['t', 'r', 's', 'm', 'e'].iter().map(|&l| said(&s, l)).sum();
    assert_eq!((said(&s, 'n'), said(&s, 's'), said(&s, 'm'), said(&s, 'e'), events), (64, 32, 32, 32, 96), "{s}");
    assert!(events <= 3 * batch + 1, "{events} calls of kit_event for {batch} events and a tick");
    assert_eq!(said(&s, 'p'), 2, "one pass for the batch");
    grimoire::apphost::apphost_close(&mut a);
    grimoire::library::library_free(&mut a);
    canvas_free(&mut a.screen);
    let _ = app_package_remove(id, false);
    let _ = std::fs::remove_dir_all(root);
}

/// A module that keeps what its last KEY event and its last `app_key` call said, each counted: its pass says `k` the
/// KEY events, `i` the last one's id, `m` its modifiers, `d` its down flag; `a` the app_key calls, `c` the last one's
/// code, `o` its modifiers, `w` its down flag.
fn key_probe() -> Kit {
    let r = |n: i32| R_AT + 4 * n;
    let fields: [(&str, usize, i32); 8] = [("k", 3, r(0)), ("i", 5, r(1)), ("m", 5, r(2)), ("d", 1, r(3)), ("a", 3, r(4)), ("c", 5, r(5)), ("o", 5, r(6)), ("w", 1, r(7))];
    let mut say = Vec::new();
    let mut template = String::new();
    for (letter, n, from) in fields {
        if !template.is_empty() {
            template.push(' ');
        }
        template.push_str(letter);
        let at = SAID_AT + template.len() as i32;
        template.push_str(&"0".repeat(n));
        say.extend(seq(&[load(from), digits_of(at, n)]));
    }
    let mut k = Kit::new(Host::App, seq(&[say, host_status(SAID_AT, template.len() as i32)]));
    k.data = vec![(SAID_AT as u32, template.into_bytes())];
    k.code.push(Vec::new()); // app_pointer
    k.code.push(seq(&[bump(r(4), i(1)), store(r(5), arg(0)), store(r(6), arg(1)), store(r(7), arg(2))])); // app_key
    k.event = Some(seq(&[
        when(seq(&[arg(0), i(EV_KEY), vec![EQ]]), seq(&[bump(r(0), i(1)), store(r(1), arg(1)), store(r(2), arg(2)), store(r(3), arg(3))])),
        i(1),
    ]));
    k
}

/// An App with an app of `module` open (no status bar), shortcuts on with `shortcuts` as shortcuts.ini.
fn app_open_with(root: &std::path::Path, id: &str, module: &[u8], shortcuts: &str) -> App {
    app_package_install(&app_manifest(id, "Keys", &[], false, module), module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    a.scr = ScreenId::Library;
    a.settings.shortcuts_enabled = true;
    let ini = root.join("shortcuts.ini");
    std::fs::write(&ini, shortcuts).unwrap();
    shortcuts_load_all(&mut a.shortcuts, ini.to_str().unwrap(), "/nonexistent/controller.ini");
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    a
}

fn app_done(mut a: App, id: &str) {
    grimoire::apphost::apphost_close(&mut a);
    grimoire::library::library_free(&mut a);
    canvas_free(&mut a.screen);
    let _ = app_package_remove(id, false);
}

/// A key or a controller's button, pressed or let go, through the App's input (app::app_event), and the tick after.
fn press(a: &mut App, code: i32, mods: u32, pad: bool, down: bool) {
    let type_ = if down { InType::Key } else { InType::KeyUp };
    grimoire::app::app_event(a, &InEvent { type_, value: code, key_mods: mods, gamepad: pad, t_ms: now_ms(), ..Default::default() });
    grimoire::apphost::apphost_tick(a, now_ms());
}

/// While an app is open, a key pressed and let go reaches it twice, through `app_key` and a KEY event: with its code
/// (the KEY's id), its modifiers (`a`) and 1 pressed, 0 let go (`b`). While the Kindle sleeps no key and no release
/// reaches it.
#[test]
fn an_open_apps_keys_reach_it_pressed_and_let_go_with_their_code_and_modifiers_and_none_while_asleep() {
    let root = fresh_root("keys");
    let id = "test.ki.keys";
    let mut a = app_open_with(&root, id, &key_probe().bytes(), "key.Escape=back\n");
    assert_eq!(a.app_view.status(), "k000 i00000 m00000 d0 a000 c00000 o00000 w0");
    press(&mut a, 30, KEYMOD_SHIFT as u32, false, true);
    assert_eq!(a.app_view.status(), "k001 i00030 m00002 d1 a001 c00030 o00002 w1", "Shift+A pressed");
    press(&mut a, 30, KEYMOD_SHIFT as u32, false, false);
    assert_eq!(a.app_view.status(), "k002 i00030 m00002 d0 a002 c00030 o00002 w0", "and let go");
    a.asleep = true;
    press(&mut a, 31, 0, false, true);
    press(&mut a, 31, 0, false, false);
    a.asleep = false;
    grimoire::apphost::apphost_draw(&mut a);
    assert_eq!(a.app_view.status(), "k002 i00030 m00002 d0 a002 c00030 o00002 w0", "nothing while asleep");
    assert!(a.app_view.open);
    app_done(a, id);
    let _ = std::fs::remove_dir_all(root);
}

/// A controller's buttons reach an open app as keys (`app_key` and KEY, with the button's code), and one leaves the
/// app when the controller's bindings bind it to `back`, looked up among the controller's bindings, not the keys'.
#[test]
fn a_controllers_buttons_reach_an_open_app_and_one_bound_to_back_leaves_it() {
    let root = fresh_root("pad");
    let id = "test.ki.pad";
    // (a key with the code of the controller's button is bound to nothing; the button is bound to back)
    let mut a = app_open_with(&root, id, &key_probe().bytes(), "key.Escape=back\npad.305=back\npad.304=undo\n");
    press(&mut a, 304, 0, true, true);
    press(&mut a, 304, 0, true, false);
    assert_eq!(a.app_view.status(), "k002 i00304 m00000 d0 a002 c00304 o00000 w0", "a button bound to undo is the app's");
    press(&mut a, 305, 0, true, true);
    assert!(!a.app_view.open, "a button bound to back leaves the app");
    app_done(a, id);
    let _ = std::fs::remove_dir_all(root);
}

/// A SETTINGS event owes a drawing pass only when `kit_event` says the picture changed: in an app and in a frame, a
/// module that answers SETTINGS with 0 is told and not drawn for it.
#[test]
fn a_settings_event_owes_a_pass_only_when_the_module_says_its_picture_changed() {
    let root = fresh_root("settings0");
    // an app: Quiet's kit_event answers a TAP alone
    let id = "test.ki.settings0";
    let module = input(Host::App, false, Some(false)).bytes();
    let m = format!("{}{INPUT_SETTINGS}", app_manifest(id, "Settings", &[], false, &module));
    app_package_install(&m, &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    assert_eq!(app_settings_save(id, &[("mode".to_string(), "false".to_string())]), 0);
    a.app_view.reload_settings();
    grimoire::apphost::apphost_tick(&mut a, now_ms());
    assert_eq!(a.app_view.status(), "p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0", "told, and not drawn for it");
    grimoire::apphost::apphost_draw(&mut a);
    assert_eq!(a.app_view.status(), "p002 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g01 h0", "the SETTINGS event came");
    app_done(a, id);
    // a frame: Quiet widget owes nothing, Input widget a pass
    for (wid, events, owes) in [("test.ki.settings0w", false, false), ("test.ki.settings1w", true, true)] {
        let module = input(Host::Widget, true, Some(events)).bytes();
        let m = format!("{}{INPUT_SETTINGS}", widget_manifest(wid, "Settings", &WIDGET_BOTH, 400, 300, &module));
        widget_package_install(&m, &module, "", SCREEN.0, SCREEN.1).unwrap();
        widget_catalog_invalidate();
        let mut host = grimoire::widgets::WidgetHost::new();
        host.frames.push(grimoire::widgets::WidgetFrame::new(grimoire::widgets::widget_def(wid).unwrap()));
        host.frames[0].ensure_runtime();
        assert!(host.frames[0].runtime.is_some());
        host.reload_settings(wid);
        assert_eq!(host.frames[0].input.owed(), owes, "{wid}");
        widget_package_remove(wid, false).unwrap();
    }
    let _ = std::fs::remove_dir_all(root);
}

/// An app's pass that input owes waits while the panel's last fast refresh is still showing: a press's PRESS is told
/// at once, its pass comes once the refresh has ended; a drag's turn waits likewise.
#[cfg(feature = "sim")]
#[test]
fn an_apps_input_pass_waits_while_the_last_fast_refresh_shows() {
    let root = fresh_root("fastbusy");
    let id = "test.ki.fastbusy";
    let module = input(Host::App, true, Some(true)).bytes();
    app_package_install(&app_manifest(id, "Busy", &[], false, &module), &module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    sim_clock_set(Some(100_000));
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    let at = |a: &mut App, t: u32| {
        sim_clock_set(Some(t));
        grimoire::apphost::apphost_tick(a, t);
    };
    assert!(grimoire::apphost::apphost_event(&mut a, &InEvent { type_: InType::TouchDown, x: 600.0, y: 1000.0, t_ms: now_ms(), ..Default::default() }));
    sim_fast_busy_set(true);
    at(&mut a, 100_000 + HOLD_MS);
    at(&mut a, 100_000 + HOLD_MS + 200);
    assert_eq!(a.app_view.status(), "p001 n000 t00 r00 s00 m00 e00 c00 q000 k00 a00 g00 h0", "no pass while the fast refresh shows");
    sim_fast_busy_set(false);
    at(&mut a, 100_000 + HOLD_MS + 300);
    assert_eq!(a.app_view.status(), "p002 n001 t00 r01 s00 m00 e00 c00 q000 k00 a00 g00 h0", "the PRESS's pass once it has ended");
    sim_fast_busy_set(false);
    sim_clock_set(None);
    app_done(a, id);
    let _ = std::fs::remove_dir_all(root);
}

/// The refresh each tick of an app sends, if any: the part of the screen and the mode (the clock set to `t` first).
#[cfg(feature = "sim")]
fn asked_tick(a: &mut App, t: u32) -> Option<(Rect, RefreshMode)> {
    sim_clock_set(Some(t));
    a.dirty = RECT_EMPTY;
    grimoire::apphost::apphost_tick(a, t);
    (!rect_is_empty(a.dirty)).then_some((a.dirty, a.dirty_mode))
}

/// An App with the app `id` of `module` open (no status bar), launched at `t` by the app's clock.
#[cfg(feature = "sim")]
fn asked_open(id: &str, module: &[u8], t: u32) -> App {
    app_package_install(&app_manifest(id, "Asked", &[], false, module), module, None, "", SCREEN.0, SCREEN.1).unwrap();
    let mut a = App { w: SCREEN.0, h: SCREEN.1, dpi: 300.0, input_drained: true, ..Default::default() };
    assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
    grimoire::library::library_init(&mut a);
    sim_clock_set(Some(t));
    assert!(grimoire::apphost::apphost_launch(&mut a, id));
    a
}

/// The square of `changing_square` in an app without the status bar: 200 px a side about 200, 200 of the body.
const ASKED_SQUARE: Rect = Rect { x: 100, y: 100, w: 200, h: 200 };

/// A pass an app asks for (`request_redraw`) refreshes only the part of the screen it changed, nothing when it changed
/// nothing. After a rest (nothing owed a clean, no pass asked for and no fast refresh within the pause: 150 ms with
/// the pen away, editor.rs SETTLE_LEFT_MS; 1.5 s with it near, SETTLE_NEAR_MS) as the UI is, at full depth, owing
/// nothing; back to back fast, and what those covered is owed one Clean refresh once the app has asked for no pass for
/// the pause, so passes back to back are cleaned once, after the last.
#[cfg(feature = "sim")]
#[test]
fn a_pass_an_app_asks_for_is_refreshed_at_full_depth_after_a_rest_and_fast_back_to_back_then_cleaned_once() {
    let root = fresh_root("asked");
    let (ui, fast, clean) = (Some((ASKED_SQUARE, RefreshMode::Ui)), Some((ASKED_SQUARE, RefreshMode::Fast)), Some((ASKED_SQUARE, RefreshMode::Clean)));
    // A square of 160 and 96 by turns, a pass asked 100 ms after each of the first three: four passes in all.
    let id = "test.ki.asked";
    let mut a = asked_open(id, &changing_square(Host::App, 200, 100, Some(4)).bytes(), 10_000);
    assert_eq!(asked_tick(&mut a, 10_099), None, "no pass before its time");
    assert_eq!(asked_tick(&mut a, 10_100), ui, "the first after the launch: a rest, so the square at full depth");
    assert_eq!(asked_tick(&mut a, 10_200), fast, "100 ms after a pass asked for: back to back, fast");
    assert_eq!(asked_tick(&mut a, 10_300), fast);
    assert!(a.app_view.status().starts_with("p004 "), "{}", a.app_view.status());
    assert_eq!(asked_tick(&mut a, 10_350), None, "no clean 150 ms after the first fast pass: the app asked again since");
    assert_eq!(asked_tick(&mut a, 10_449), None, "no clean before the pause has passed since the last pass");
    assert_eq!(asked_tick(&mut a, 10_450), clean, "one clean of all the fast passes covered");
    assert_eq!(asked_tick(&mut a, 11_450), None, "and nothing more: the app asked for no other pass");
    app_done(a, id);
    // A pass a second: each after a rest with the pen away, at full depth, nothing to clean. With the pen near the
    // pause is 1.5 s, so a pass a second is back to back: fast, and the clean waits until the passes are the pause
    // apart; once the pen has left, the clean comes 150 ms after the last pass, and the next pass is after a rest.
    let id = "test.ki.asked.second";
    let mut a = asked_open(id, &changing_square(Host::App, 200, 1000, None).bytes(), 10_000);
    assert_eq!(asked_tick(&mut a, 11_000), ui);
    assert_eq!(asked_tick(&mut a, 11_500), None, "no clean: nothing was refreshed fast");
    assert_eq!(asked_tick(&mut a, 12_000), ui);
    grimoire::status::status_pen(&InEvent { type_: InType::PenHover, x: 900.0, y: 1200.0, ..Default::default() });
    assert_eq!(asked_tick(&mut a, 13_000), fast, "the pen near: a second after the last pass is within the pause");
    assert_eq!(asked_tick(&mut a, 14_000), fast);
    assert_eq!(asked_tick(&mut a, 14_999), None, "no clean with the pen near: the next pass comes first");
    assert_eq!(asked_tick(&mut a, 15_000), fast);
    grimoire::status::status_pen(&InEvent { type_: InType::PenLeave, ..Default::default() });
    assert_eq!(asked_tick(&mut a, 15_149), None);
    assert_eq!(asked_tick(&mut a, 15_150), clean, "the pen gone: the clean 150 ms after the last pass");
    assert_eq!(asked_tick(&mut a, 16_000), ui, "and the next pass is after a rest again");
    assert!(a.app_view.status().starts_with("p007 "), "{}", a.app_view.status());
    app_done(a, id);
    // A square that stays as it was, a pass asked 100 ms after each: each pass is drawn, and nothing is refreshed.
    let id = "test.ki.asked.same";
    let mut a = asked_open(id, &input_square(Host::App, 200, 128, Some(100)).bytes(), 10_000);
    assert_eq!(asked_tick(&mut a, 10_100), None, "a pass that changed nothing refreshes nothing");
    assert_eq!(asked_tick(&mut a, 10_200), None);
    assert_eq!(asked_tick(&mut a, 10_500), None, "and owes no clean");
    assert!(a.app_view.status().starts_with("p004 "), "{}", a.app_view.status());
    sim_clock_set(None);
    app_done(a, id);
    let _ = std::fs::remove_dir_all(root);
}

/// The rest before a pass an app asks for, and the pause before its clean, are durations on the u32 clock: the same
/// across its wrap at 2^32 ms.
#[cfg(feature = "sim")]
#[test]
fn the_rest_before_a_pass_an_app_asks_for_and_its_clean_hold_across_the_clocks_wrap() {
    let root = fresh_root("askedwrap");
    let (ui, fast, clean) = (Some((ASKED_SQUARE, RefreshMode::Ui)), Some((ASKED_SQUARE, RefreshMode::Fast)), Some((ASKED_SQUARE, RefreshMode::Clean)));
    // a pass a second, its second one past the wrap: still a second after the first, a rest
    let id = "test.ki.askedwrap.second";
    let mut a = asked_open(id, &changing_square(Host::App, 200, 1000, None).bytes(), u32::MAX - 1500);
    assert_eq!(asked_tick(&mut a, u32::MAX - 500), ui);
    assert_eq!(asked_tick(&mut a, 499), ui, "1,000 ms later across the wrap: after a rest");
    app_done(a, id);
    // passes 100 ms apart across the wrap: back to back, and the clean 150 ms after the last
    let id = "test.ki.askedwrap.run";
    let mut a = asked_open(id, &changing_square(Host::App, 200, 100, Some(4)).bytes(), u32::MAX - 150);
    assert_eq!(asked_tick(&mut a, u32::MAX - 50), ui);
    assert_eq!(asked_tick(&mut a, 49), fast, "100 ms later across the wrap: back to back");
    assert_eq!(asked_tick(&mut a, 149), fast);
    assert_eq!(asked_tick(&mut a, 298), None);
    assert_eq!(asked_tick(&mut a, 299), clean);
    sim_clock_set(None);
    app_done(a, id);
    let _ = std::fs::remove_dir_all(root);
}

/// A pass an app asks for while what fast refreshes covered still waits for its clean is refreshed fast, however long
/// since the last: here a held contact's paced pass refreshed fast, and the request comes 300 ms later with the finger
/// still down. Once the clean has gone out, the next request after a rest is at full depth again.
#[cfg(feature = "sim")]
#[test]
fn a_pass_an_app_asks_for_while_fast_refreshes_wait_for_their_clean_is_fast() {
    let root = fresh_root("askedheld");
    let id = "test.ki.askedheld";
    // (a pass a second; its square follows the touch)
    let mut a = asked_open(id, &changing_square(Host::App, 200, 1000, None).bytes(), 10_000);
    let touch = |a: &mut App, type_: InType, t: u32| {
        sim_clock_set(Some(t));
        assert!(grimoire::apphost::apphost_event(a, &InEvent { type_, x: 900.0, y: 1200.0, t_ms: t, ..Default::default() }));
    };
    let held = Rect::new(800, 1100, 200, 200);
    assert_eq!(asked_tick(&mut a, 11_000), Some((ASKED_SQUARE, RefreshMode::Ui)));
    touch(&mut a, InType::TouchDown, 11_100);
    assert_eq!(asked_tick(&mut a, 11_100), None, "no pass while the contact may be a tap");
    assert_eq!(asked_tick(&mut a, 11_100 + HOLD_MS), Some((Rect::new(100, 100, 900, 1200), RefreshMode::Fast)), "the press's pass: the square moved, fast");
    assert_eq!(asked_tick(&mut a, 12_000), Some((held, RefreshMode::Fast)), "the request with the press's fast refresh owed its clean: fast");
    touch(&mut a, InType::TouchUp, 12_100);
    assert_eq!(asked_tick(&mut a, 12_100), Some((held, RefreshMode::Ui)), "the lift's pass, as the UI");
    assert_eq!(asked_tick(&mut a, 12_249), None);
    assert_eq!(asked_tick(&mut a, 12_250), Some((Rect::new(100, 100, 900, 1200), RefreshMode::Clean)), "the clean of both fast refreshes");
    assert_eq!(asked_tick(&mut a, 13_000), Some((held, RefreshMode::Ui)), "the next request after a rest: full depth");
    sim_clock_set(None);
    app_done(a, id);
    let _ = std::fs::remove_dir_all(root);
}

/// The rest before a pass an app asks for counts from its last fast refresh too, an input pass's among them: with the
/// pen near (a pause of 1.5 s), a request 400 ms after a press's fast refresh is back to back, though its clean has
/// gone out and the app's last request was 2 s before.
#[cfg(feature = "sim")]
#[test]
fn a_pass_an_app_asks_for_within_the_pause_of_an_input_passs_fast_refresh_is_fast() {
    let root = fresh_root("askedinput");
    let id = "test.ki.askedinput";
    // (a pass every 2 s; its square follows the touch)
    let mut a = asked_open(id, &changing_square(Host::App, 200, 2000, None).bytes(), 10_000);
    let touch = |a: &mut App, type_: InType, t: u32| {
        sim_clock_set(Some(t));
        assert!(grimoire::apphost::apphost_event(a, &InEvent { type_, x: 900.0, y: 1200.0, t_ms: t, ..Default::default() }));
    };
    let held = Rect::new(800, 1100, 200, 200);
    assert_eq!(asked_tick(&mut a, 12_000), Some((ASKED_SQUARE, RefreshMode::Ui)));
    touch(&mut a, InType::TouchDown, 13_000);
    assert_eq!(asked_tick(&mut a, 13_000), None);
    assert_eq!(asked_tick(&mut a, 13_000 + HOLD_MS), Some((Rect::new(100, 100, 900, 1200), RefreshMode::Fast)), "the press's pass, fast");
    touch(&mut a, InType::TouchUp, 13_700);
    assert_eq!(asked_tick(&mut a, 13_700), Some((held, RefreshMode::Ui)));
    assert_eq!(asked_tick(&mut a, 13_850), Some((Rect::new(100, 100, 900, 1200), RefreshMode::Clean)), "its clean, the pen away");
    grimoire::status::status_pen(&InEvent { type_: InType::PenHover, x: 300.0, y: 300.0, ..Default::default() });
    assert_eq!(asked_tick(&mut a, 14_000), Some((held, RefreshMode::Fast)), "400 ms after the fast refresh, the pen near: fast");
    grimoire::status::status_pen(&InEvent { type_: InType::PenLeave, ..Default::default() });
    assert_eq!(asked_tick(&mut a, 14_150), Some((held, RefreshMode::Clean)), "cleaned once the pen has left");
    sim_clock_set(None);
    app_done(a, id);
    let _ = std::fs::remove_dir_all(root);
}
