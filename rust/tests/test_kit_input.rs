// test_kit_input.rs: the package kit's input (kithost.rs, docs/KIT.md, Events): the recogniser of a contact
// (`KitPointer`: a tap, a press at HOLD_MS, the slop of TAP_SLOP_MM each way, a drag's start, gathered samples and
// end, a cancel, the pen and a finger, more samples than a DRAG_MOVE holds), the pace of the passes input owes
// (`KitInput`), the part of the screen a pass changed (`kit_changed`), and `kit_event`, `event_points` and `key_char`
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

// ---- what a pass changed ----

#[test]
fn the_part_a_pass_changed_is_the_box_of_the_pixels_that_differ_inside_the_part_kept() {
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, 400, 300), 0);
    canvas_fill(&mut c, Rect::new(0, 0, 400, 300), 255);
    let at = Rect::new(100, 50, 200, 200);
    let (kept, cut) = kit_keep(&c, at).unwrap();
    assert_eq!(cut, at);
    assert!(rect_is_empty(kit_changed(&kept, cut, &c)), "nothing changed");
    canvas_fill(&mut c, Rect::new(150, 60, 10, 20), 0);
    canvas_fill(&mut c, Rect::new(280, 230, 5, 5), 0);
    // (a change outside the part kept is not the pass's)
    canvas_fill(&mut c, Rect::new(10, 10, 5, 5), 0);
    assert_eq!(kit_changed(&kept, cut, &c), Rect::new(150, 60, 135, 175));
    // a part reaching off the screen is cut to it
    let (kept, cut) = kit_keep(&c, Rect::new(350, 250, 200, 200)).unwrap();
    assert_eq!(cut, Rect::new(350, 250, 50, 50));
    canvas_fill(&mut c, Rect::new(399, 299, 1, 1), 0);
    assert_eq!(kit_changed(&kept, cut, &c), Rect::new(399, 299, 1, 1));
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
