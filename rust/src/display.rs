//! display.h: e-ink output. Implemented by display_fbink.c (Kindle) and sim_sdl.c.

use crate::prelude::*;

// display.h is implemented by one platform file per build, as in the C (display_fbink.c on the
// Kindle, sim_sdl.c in the simulator; none on the host: platform_none.rs). These forward to it,
// through the build's platform (platform.rs).
// (the in-tree platforms' methods need the trait in scope; a registered `dyn` platform's do not)
#[cfg_attr(feature = "external-platform", allow(unused_imports))]
use crate::platform::{active, Platform};

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RefreshMode {
    /// DU: black/white, fast, for live ink
    #[default]
    Fast = 0,  // REFRESH_FAST
    /// GL16 partial, no flash
    Ui = 1,  // REFRESH_UI
    /// GC16 full flash
    Full = 2,  // REFRESH_FULL
    /// GC16 flash of the rect alone (KOReader's "flashui"): every pixel in it driven, so what fast DU
    /// refreshes left there is wiped, which a GL16 partial cannot do (it drives only the pixels
    /// whose value changed). The Rust build only: the C tree has no such mode.
    Clean = 3,
}

impl RefreshMode {
    /// Which of two modes asked for one rect wins (app_invalidate keeps the strongest): Fast, Ui,
    /// Clean, then Full, which flashes the whole screen anyway.
    pub fn strength(self) -> i32 {
        match self {
            RefreshMode::Fast => 0,
            RefreshMode::Ui => 1,
            RefreshMode::Clean => 2,
            RefreshMode::Full => 3,
        }
    }
}

// ---- DU wear (issue #11) ----
//
// A fast (DU) refresh drives a pixel to black or white in one short waveform, and one it turns from
// black to white keeps a faint trace of the black (DU ghosting). The grayscale settle that follows
// (GL16, a partial update) does not reach it: a partial update drives only the pixels whose value
// changed, and DU already wrote that pixel's final white. So the traces of erased ink and of the
// toolbar's old places stay until a flashing refresh (a page turn at another than the fitted view, the
// Library and back, or the 30th UI refresh). DuWear
// counts the screen area fast refreshes turned back toward the paper (the eraser's, a dragged
// selection's); past a quarter of the screen the editor cleans where they were with one Clean
// refresh, at a pause. Writing is not counted: DU turning white paper black leaves no trace.

/// The share of the screen past which DU wear is cleaned: a quarter (issue #11's figure). A
/// handwritten word is about 1 % of the Scribe's screen, so small erases add up to a clean only
/// after twenty-odd words, and one erase of a quarter page cleans at once.
pub const DU_WEAR_QUARTERS: i64 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DuWear {
    /// screen px fast refreshes turned toward the paper since the last clean, counted each time
    pub area: i64,
    /// where they were (screen px): the rect a clean refreshes
    pub rect: Rect,
    /// a clean is owed whatever the area (a toolbar drag that could not clean at its release)
    pub owed: bool,
    /// when the last was added (ms), so a clean waits for a pause in what added it
    pub at: u32,
}

/// `area` screen px of `r` were turned toward the paper by a fast refresh (area at most r's).
pub fn du_wear_add(w: &mut DuWear, r: Rect, area: i64, now: u32) {
    if rect_is_empty(r) || area <= 0 {
        return;
    }
    w.area = w.area.saturating_add(area.min(r.w as i64 * r.h as i64));
    w.rect = rect_union(w.rect, r);
    w.at = now;
}

/// A clean of r is owed whatever the area.
pub fn du_wear_owe(w: &mut DuWear, r: Rect, now: u32) {
    if rect_is_empty(r) {
        return;
    }
    w.rect = rect_union(w.rect, r);
    w.owed = true;
    w.at = now;
}

/// Whether a clean is due on a screen of sw x sh: owed, or more than a quarter of it worn.
pub fn du_wear_due(w: &DuWear, sw: i32, sh: i32) -> bool {
    !rect_is_empty(w.rect) && (w.owed || w.area * 4 > DU_WEAR_QUARTERS * sw as i64 * sh as i64)
}

/// A refresh of r in mode m went out: a Full wipes every trace, a Clean wipes those it covers.
pub fn du_wear_presented(w: &mut DuWear, r: Rect, m: RefreshMode) {
    if m == RefreshMode::Full || (m == RefreshMode::Clean && rect_contains_rect(r, w.rect)) {
        *w = DuWear::default();
    }
}

/// Fills portrait width/height and dpi of the panel. 0 on success.
#[inline]
pub fn display_open(w: &mut i32, h: &mut i32, dpi: &mut f32) -> i32 {
    active().display_open(w, h, dpi)
}
/// Copies the rect of `screen` (same size as the display) to the panel and refreshes it.
#[inline]
pub fn display_present(screen: &Canvas, r: Rect, mode: RefreshMode) {
    active().display_present(screen, r, mode)
}
/// Device rotation (quarter turns clockwise): the logical screen the app draws is mapped onto
/// the panel accordingly. Fills the logical width/height (swapped for 1 and 3).
#[inline]
pub fn display_set_orientation(quarter_turns: i32, w: &mut i32, h: &mut i32) {
    active().display_set_orientation(quarter_turns, w, h)
}
/// Show Grimoire's colors inverted (white on black) when on.
#[inline]
pub fn display_set_invert(on: bool) {
    active().display_set_invert(on)
}
/// Low-latency handwriting mode (MTK pen mode on the Scribe). While on, only REFRESH_FAST
/// should be used; any other mode turns it off first. No-op where unsupported. The editor turns it
/// on for a stroke of ink, which marks live ink too (display_live_ink).
#[inline]
pub fn display_pen_mode(on: bool) {
    active().display_pen_mode(on)
}
/// Live ink: a stroke being drawn with the pen (writing, erasing, the pen feel pad, the calibration's
/// check line), on at its start and off at its end. On a colour panel every other fast refresh is
/// followed by a wait until the driver has read the frame (display_fbink.c); live ink never waits,
/// so pen reading never stalls. Changes nothing else (no pen mode).
#[inline]
pub fn display_live_ink(on: bool) {
    active().display_live_ink(on)
}

/// Which fast refreshes are live ink, the rule display_fbink.c follows (the simulator follows it
/// too, and reports it to scripts): those sent while a stroke is drawn, and the first one within
/// 100 ms of its end (the samples that came with the pen lifting go out just after the stroke ends).
#[derive(Clone, Copy, Debug, Default)]
pub struct LiveInk {
    /// a stroke is being drawn
    pub on: bool,
    /// when the last one ended (0: none since the last fast refresh)
    pub ended: u32,
}
#[inline]
pub fn live_ink_mark(l: &mut LiveInk, on: bool, now: u32) {
    if l.on && !on {
        l.ended = now | 1;
    }
    l.on = on;
}
/// For a refresh in `mode` sent at `now`: true when it is a fast refresh of live ink.
#[inline]
pub fn live_ink_frame(l: &mut LiveInk, mode: RefreshMode, now: u32) -> bool {
    if mode != RefreshMode::Fast {
        return false;
    }
    let ink = l.on || (l.ended != 0 && (now.wrapping_sub(l.ended) as i32) < 100);
    l.ended = 0;
    ink
}
/// The screen guard. Both checks read the framebuffer back (8 or 32 bpp) to see whether another
/// process drew over Grimoire; cheap. The simulator never reports an overwrite.
/// This one samples a band of the screen densely: it catches small intruders such as the Kindle's
/// status bar clock. `skip` (may be empty) is excluded, e.g. a visible toast.
#[inline]
pub fn display_band_overwritten(screen: &Canvas, band: Rect, skip: Rect) -> bool {
    active().display_band_overwritten(screen, band, skip)
}
/// This one finds a patch anywhere on screen that another process drew (a 24 px sampling grid, so
/// a strip of the Kindle home window is caught even when it covers little of the screen).
/// True with *where = the bounding box of the mismatching samples (padded).
#[inline]
pub fn display_find_overwrite(screen: &Canvas, skip: Rect, where_: &mut Rect) -> bool {
    active().display_find_overwrite(screen, skip, where_)
}
/// A rect the panel shows over App.screen for good (Live view's chip, app.c): both checks leave it
/// out, as they do a toast. RECT_EMPTY for none.
#[inline]
pub fn display_skip_overlay(r: Rect) {
    active().display_skip_overlay(r)
}
/// A rect the panel shows over App.screen while the clock and battery corner is up (status.rs):
/// both checks leave it out too, apart from the Live view chip's, which has its own. RECT_EMPTY for
/// none.
#[inline]
pub fn display_skip_status(r: Rect) {
    active().display_skip_status(r)
}
/// True while the panel's driver is in pen mode: set by the first stroke's fast frames and
/// kept past the stroke until a grayscale refresh is due (display_fbink.rs, pen_leave), which then
/// waits for the pen's last frame, or until a pause after writing with Ink edges at Solid sends
/// nothing (display_settle_area's empty answer), after the same wait. The simulator answers with
/// what display_pen_mode last asked.
#[inline]
pub fn display_pen_mode_on() -> bool {
    active().display_pen_mode_on()
}
/// True once the first frame and the next 4 refreshes have all failed: the panel is not Grimoire's
/// to drive, and it should leave. A failure after any refresh worked never counts. Polled by the
/// main loop: until one refresh works it also refreshes the whole panel again once a second, so a
/// dead panel is known within seconds even when nothing else draws. Always false in the simulator.
#[inline]
pub fn display_broken() -> bool {
    active().display_broken()
}
/// Blocks until pending refreshes have completed (used before exit / screenshots).
#[inline]
pub fn display_wait() {
    active().display_wait()
}
/// Finish our updates before handing the display to the screensaver/native UI. Pen mode is a
/// device setting, so leave it explicitly even when no sleep image will be drawn.
pub fn display_sleep() {
    display_wait(); // exactly one waiter: the Kindle backend follows the quiet thread's books
    display_wake();
}
/// Reassert normal display mode after native UI or hardware suspend may have changed it.
pub fn display_wake() {
    active().display_power_reset()
}
#[inline]
pub fn display_close() {
    active().display_close()
}
/// Whether the last fast refresh sent may still be on the panel, its waveform not ended: a held delete's
/// next repeat waits for it rather than queue behind it in the driver (keyboard_repeat). No display
/// operation is issued. Never in the simulator, whose refreshes are done when they are sent, unless a
/// test says so (`sim_fast_busy_set`).
pub fn display_fast_busy() -> bool {
    #[cfg(feature = "sim")]
    if SIM_FAST_BUSY.with(|b| b.get()) {
        return true;
    }
    active().display_fast_busy()
}

#[cfg(feature = "sim")]
thread_local! {
    static SIM_FAST_BUSY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Tests, in the simulator's build: the last fast refresh taken as still on the panel (`busy`), as a Kindle's is for a
/// few hundred ms after it is sent, so that what waits for it can be seen to wait; false again for the simulator's own
/// answer. The main thread's alone.
#[cfg(feature = "sim")]
pub fn sim_fast_busy_set(busy: bool) {
    SIM_FAST_BUSY.with(|b| b.set(busy));
}
/// Cached indication for pacing non-ink trails on a colour panel. No display operation is issued.
pub fn display_last_fast_waiting() -> bool {
    active().display_last_fast_waiting()
}

/// The margin display_settle_area gives the pixels it finds (screen px): a tight crop of changed pixels can leave
/// edge artifacts on some panels.
pub const SETTLE_MARGIN: i32 = 3;

/// The settle after writing with Ink edges at Solid (editor.rs settle_solid, 2.3, #320): the part of r (screen px)
/// whose pixels the panel shows at another level (value >> 4) than `screen` holds them now, padded by SETTLE_MARGIN and
/// clipped to r, or empty when every level matches; the caller refreshes what this returns in grays, or nothing.
/// Pixels that got lighter count as well as darker ones (a shape snapped, an undo, the lasso, erasing).
///
/// The contract for a platform: it answers r whole unless it knows what it last gave the panel at every pixel of r.
/// One that keeps no record answers r whole (the default), and so does one whose last frames may not all have
/// reached the panel as they were written: a refresh refused or failed, a live frame the driver may read half done,
/// a rotation or an inversion since; until a whole grayscale frame has gone out again. A platform with a pen mode
/// leaves it when it answers empty, as the grayscale refresh would have made it do.
#[inline]
pub fn display_settle_area(screen: &Canvas, r: Rect) -> Rect {
    active().display_settle_area(screen, r)
}

/// The box of the pixels of r (in `screen`'s pixels) whose level (v >> 4) differs from what `level_of(x, y)` says the
/// panel shows there, padded by SETTLE_MARGIN and clipped to r; RECT_EMPTY when none differs. A platform's
/// display_settle_area with a record of its panel can answer with it.
pub fn settle_area_by(screen: &Canvas, r: Rect, mut level_of: impl FnMut(i32, i32) -> u8) -> Rect {
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: screen.w, h: screen.h });
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, -1, -1);
    for y in r.y..r.y + r.h {
        let row = &screen.px[(y * screen.stride) as usize..];
        for x in r.x..r.x + r.w {
            if row[x as usize] >> 4 != level_of(x, y) {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    if x1 < 0 {
        return RECT_EMPTY;
    }
    settle_area_pad(Rect { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 }, r)
}

/// The pixels found, `found`, padded by SETTLE_MARGIN and clipped to r (empty for none found).
pub fn settle_area_pad(found: Rect, r: Rect) -> Rect {
    if rect_is_empty(found) {
        return RECT_EMPTY;
    }
    rect_intersect(rect_inflate(found, SETTLE_MARGIN), r)
}

/// The rows a display_settle_area that scans its record under a hold of its state takes between two calls of the
/// pump: a full-screen copy's strip (display_fbink.rs copy_rect).
pub const SETTLE_STRIP_ROWS: i32 = 64;

/// display_settle_area's scan of c (clipped already) in strips of SETTLE_STRIP_ROWS rows, top to bottom, every row
/// in exactly one strip. `strip(y0, y1)` scans rows y0..y1 and answers the box of the pixels it found there
/// (RECT_EMPTY for none), or None when it cannot answer for them: the walk ends there with None, and the caller
/// answers its whole rect. `pump()` (util.rs work_pump, which reads the pens unless they were read under 8 ms
/// before: input_evdev.rs input_pump) is called after every strip of SETTLE_STRIP_ROWS rows, as after a full-screen
/// copy's, and once more after a last, shorter strip (or after none) when no pixel was found: an empty answer is
/// followed by leaving pen mode, which may wait, and a wait reads no pens, so a call always comes between the last
/// row and that wait. It never runs inside `strip`, so a strip may hold its platform's state. Some(the box of all
/// that was found, padded by SETTLE_MARGIN and clipped to c): what one pass over c gives.
pub fn settle_area_strips(c: Rect, mut strip: impl FnMut(i32, i32) -> Option<Rect>, mut pump: impl FnMut()) -> Option<Rect> {
    let mut found = RECT_EMPTY;
    let mut pumped = false;
    let mut y = c.y;
    let y_end = c.y + c.h;
    while y < y_end {
        let strip_end = (y + SETTLE_STRIP_ROWS).min(y_end);
        found = rect_union(found, strip(y, strip_end)?);
        pumped = strip_end - y == SETTLE_STRIP_ROWS;
        if pumped {
            pump();
        }
        y = strip_end;
    }
    if rect_is_empty(found) && !pumped {
        pump();
    }
    Some(settle_area_pad(found, c))
}

/// One strip of the scan (settle_area_strips) for a platform whose record is the bytes it last wrote for the panel,
/// one a pixel (`shown`: display_fbink.rs's RAM shadow): rows y0..y1 of c (inside `screen`), each compared level by
/// level with its bytes as the row was written (fbpack_level_span, `invert` as the copy inverts). `row_at(y)` says
/// where the byte of row y's first pixel (c.x, y) is in `shown` and the step from a pixel to the next (a pixel or a
/// whole line, either direction, by the rotation). The box of the pixels found (RECT_EMPTY for none); None when a
/// row's bytes are not all inside `shown`.
pub fn settle_strip_by_bytes(
    screen: &Canvas,
    c: Rect,
    y0: i32,
    y1: i32,
    shown: &[u8],
    invert: bool,
    mut row_at: impl FnMut(i32) -> (isize, isize),
) -> Option<Rect> {
    let mut found = RECT_EMPTY;
    for y in y0..y1 {
        let (at, step) = row_at(y);
        let last = at + step * (c.w - 1) as isize;
        if at < 0 || last < 0 || at.max(last) as usize >= shown.len() {
            return None;
        }
        let src = &screen.px[(y * screen.stride + c.x) as usize..][..c.w as usize];
        if let Some((a, b)) = fbpack_level_span(src, shown, at as usize, step, invert) {
            found = rect_union(found, Rect { x: c.x + a as i32, y, w: (b - a + 1) as i32, h: 1 });
        }
    }
    Some(found)
}

/// Why a display_settle_area that keeps a record answered its rect whole all the same (the Kindle's, for
/// grimoire.log).
#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettleWhole {
    /// No whole grayscale frame has gone out since the display opened, the screen was turned or inverted, or the
    /// panel changed hands.
    NoFrame = 0,
    /// The frame is not known, and a refresh has failed since a settle last found it known (or since the display
    /// opened): a failed refresh makes the frame unknown until a whole grayscale one goes out.
    RefreshFailed = 1,
    /// A colour panel, whose live frames are not waited for and may be read half done.
    ColourPanel = 2,
    /// No panel open, no record of it kept, or one that cannot hold every row (a framebuffer mapping that does not
    /// cover the panel).
    NoRecord = 3,
    /// A row of the rect would lie outside the record.
    RowRange = 4,
    /// What the scan reads of the display's state differed between two of its strips.
    Changed = 5,
}

pub const SETTLE_WHOLE_KINDS: usize = 6;

/// SettleWhole's reasons as the log names them, in its order.
pub static K_SETTLE_WHOLE: [&str; SETTLE_WHOLE_KINDS] =
    ["no frame known", "a refresh failed", "colour panel", "no record of the panel", "row out of range", "state changed during the scan"];

/// What a display_settle_area that keeps a record has answered since the app started, for its log: plain counts,
/// bumped as it answers and written out later, with the platform's refresh counts (settle_counts_line), so the
/// settle itself writes no line for them. None rises with Ink edges at Smooth, whose settle never asks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettleCounts {
    /// every level matched: nothing sent, pen mode left
    pub quiet: u32,
    /// the box of what differed sent, the record trusted
    pub part: u32,
    /// the rect whole (2.2's refresh), by SettleWhole
    pub whole: [u32; SETTLE_WHOLE_KINDS],
    /// a count rose since the line was last written
    pub unlogged: bool,
}

pub fn settle_count_quiet(c: &mut SettleCounts) {
    c.quiet = c.quiet.saturating_add(1);
    c.unlogged = true;
}

pub fn settle_count_part(c: &mut SettleCounts) {
    c.part = c.part.saturating_add(1);
    c.unlogged = true;
}

/// Counts a whole answer; true the first time its reason occurs (the platform's log then says it once).
pub fn settle_count_whole(c: &mut SettleCounts, why: SettleWhole) -> bool {
    let n = &mut c.whole[why as usize];
    *n = n.saturating_add(1);
    c.unlogged = true;
    *n == 1
}

/// The counts as a line for the log, the reasons that occurred in brackets ("display: settles so far: quiet 12,
/// part 3, whole 2 (no frame known 1, a refresh failed 1)"), when a count rose since the last line (taken as written
/// now); None otherwise, so a session that never settles this way logs nothing.
pub fn settle_counts_line(c: &mut SettleCounts) -> Option<String> {
    if !c.unlogged {
        return None;
    }
    c.unlogged = false;
    let whole = c.whole.iter().fold(0u32, |a, &n| a.saturating_add(n));
    let mut line = format!("display: settles so far: quiet {}, part {}, whole {}", c.quiet, c.part, whole);
    let mut sep = " (";
    for (k, &n) in c.whole.iter().enumerate() {
        if n != 0 {
            line.push_str(&format!("{sep}{} {n}", K_SETTLE_WHOLE[k]));
            sep = ", ";
        }
    }
    if whole != 0 {
        line.push(')');
    }
    Some(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: i32 = 1860;
    const H: i32 = 2480;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn strength_orders_the_modes() {
        assert!(RefreshMode::Fast.strength() < RefreshMode::Ui.strength());
        assert!(RefreshMode::Ui.strength() < RefreshMode::Clean.strength());
        assert!(RefreshMode::Clean.strength() < RefreshMode::Full.strength());
    }

    #[test]
    fn small_erases_never_make_a_clean() {
        let mut w = DuWear::default();
        // nineteen word-sized erases (400 x 150 px, 1.3 % of the screen each) stay under a quarter;
        // the twentieth passes it
        for k in 0..19 {
            du_wear_add(&mut w, r(100, 100 + k * 110, 400, 150), 400 * 150, k as u32);
            assert!(!du_wear_due(&w, W, H), "erase {k}");
        }
        assert_eq!(w.rect, r(100, 100, 400, 18 * 110 + 150));
        assert_eq!(w.at, 18);
        du_wear_add(&mut w, r(100, 2200, 400, 150), 400 * 150, 19);
        assert!(du_wear_due(&w, W, H));
    }

    #[test]
    fn a_quarter_of_the_screen_makes_a_clean() {
        let mut w = DuWear::default();
        let quarter = W as i64 * H as i64 / 4;
        du_wear_add(&mut w, r(0, 0, W, H / 4), quarter, 5);
        assert!(!du_wear_due(&w, W, H), "a quarter exactly is not more than one");
        du_wear_add(&mut w, r(0, H / 4, 10, 10), 100, 6);
        assert!(du_wear_due(&w, W, H));
        // the area counted is never more than the rect's (a long thin erase's bbox)
        let mut v = DuWear::default();
        du_wear_add(&mut v, r(0, 0, 10, 10), 1_000_000_000, 1);
        assert_eq!(v.area, 100);
        // nothing for an empty rect or no area
        du_wear_add(&mut v, RECT_EMPTY, 50, 2);
        du_wear_add(&mut v, r(0, 0, 5, 5), 0, 3);
        assert_eq!((v.area, v.at), (100, 1));
    }

    #[test]
    fn a_clean_that_covers_it_starts_again() {
        let mut w = DuWear::default();
        du_wear_add(&mut w, r(100, 100, W / 2, H), W as i64 * H as i64 / 2, 1);
        assert!(du_wear_due(&w, W, H));
        // a grayscale refresh does not reach DU's traces, nor a clean of part of them
        du_wear_presented(&mut w, r(0, 0, W, H), RefreshMode::Ui);
        du_wear_presented(&mut w, r(100, 100, 10, 10), RefreshMode::Clean);
        assert!(du_wear_due(&w, W, H));
        du_wear_presented(&mut w, r(100, 100, W / 2, H), RefreshMode::Clean);
        assert_eq!(w, DuWear::default());
        assert!(!du_wear_due(&w, W, H));
        // a Full wipes it wherever it was
        du_wear_add(&mut w, r(0, 0, W, H), W as i64 * H as i64, 2);
        du_wear_presented(&mut w, r(5, 5, 1, 1), RefreshMode::Full);
        assert_eq!(w, DuWear::default());
    }

    #[test]
    fn an_owed_clean_is_due_whatever_the_area() {
        let mut w = DuWear::default();
        du_wear_owe(&mut w, RECT_EMPTY, 1);
        assert!(!du_wear_due(&w, W, H));
        du_wear_owe(&mut w, r(1700, 200, 128, 1700), 4);
        assert!(du_wear_due(&w, W, H) && w.area == 0 && w.at == 4);
        du_wear_presented(&mut w, r(1600, 0, 260, 2480), RefreshMode::Clean);
        assert!(!du_wear_due(&w, W, H));
    }

    // ---- the settle's scan in strips (settle_area_strips, settle_strip_by_bytes; Ink edges at Solid, #320) ----

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Step {
        /// rows y0..y1 scanned
        Strip(i32, i32),
        /// the pump called (on a device it reads the pens unless they were read under 8 ms before)
        Pump,
    }
    use Step::{Pump, Strip};

    /// The first row of the walks' rect.
    const TOP: i32 = 40;

    /// The walk over a rect of h rows whose strips find one pixel run, in row `hit`, or nothing: what ran, in order,
    /// and the answer.
    fn walk(h: i32, hit: Option<i32>) -> (Vec<Step>, Option<Rect>) {
        let steps = std::cell::RefCell::new(Vec::new());
        let d = settle_area_strips(
            r(10, TOP, 300, h),
            |y0, y1| {
                steps.borrow_mut().push(Strip(y0, y1));
                Some(match hit {
                    Some(y) if y0 <= y && y < y1 => r(50, y, 20, 1),
                    _ => RECT_EMPTY,
                })
            },
            || steps.borrow_mut().push(Pump),
        );
        (steps.into_inner(), d)
    }

    #[test]
    fn the_scan_pumps_after_each_full_strip_and_before_an_empty_answer() {
        // nothing found: the pump is called after each strip of 64 rows, and once more when the last was shorter
        // (or there was none), so that a call comes between the last row and the wait that may follow
        assert_eq!(walk(0, None).0, [Pump]);
        assert_eq!(walk(1, None).0, [Strip(40, 41), Pump]);
        assert_eq!(walk(63, None).0, [Strip(40, 103), Pump]);
        assert_eq!(walk(64, None).0, [Strip(40, 104), Pump]);
        assert_eq!(walk(65, None).0, [Strip(40, 104), Pump, Strip(104, 105), Pump]);
        assert_eq!(walk(128, None).0, [Strip(40, 104), Pump, Strip(104, 168), Pump]);
        // something found: a refresh follows, not a wait, and the pump is called as a full-screen copy calls it
        assert_eq!(walk(1, Some(40)).0, [Strip(40, 41)]);
        assert_eq!(walk(63, Some(102)).0, [Strip(40, 103)]);
        assert_eq!(walk(64, Some(40)).0, [Strip(40, 104), Pump]);
        assert_eq!(walk(65, Some(104)).0, [Strip(40, 104), Pump, Strip(104, 105)]);
        assert_eq!(walk(128, Some(41)).0, [Strip(40, 104), Pump, Strip(104, 168), Pump]);
    }

    #[test]
    fn the_scan_takes_every_row_in_exactly_one_strip() {
        for h in [0, 1, 63, 64, 65, 128, 1000] {
            let hits: Vec<Option<i32>> = if h == 0 { vec![None] } else { vec![None, Some(TOP), Some(TOP + h / 2), Some(TOP + h - 1)] };
            for hit in hits {
                let (steps, d) = walk(h, hit);
                let mut seen = vec![0u32; h as usize];
                let mut next = TOP;
                let mut pumps = 0;
                for (i, s) in steps.iter().enumerate() {
                    match *s {
                        Strip(y0, y1) => {
                            assert_eq!(y0, next, "h {h}: the strips follow one another");
                            assert!(y1 > y0 && y1 - y0 <= SETTLE_STRIP_ROWS, "h {h}: {s:?}");
                            assert!(y1 - y0 == SETTLE_STRIP_ROWS || y1 == TOP + h, "h {h}: only the last strip is shorter");
                            for y in y0..y1 {
                                seen[(y - TOP) as usize] += 1;
                            }
                            next = y1;
                            // a full strip is followed by a call of the pump, at once
                            if y1 - y0 == SETTLE_STRIP_ROWS {
                                assert_eq!(steps.get(i + 1), Some(&Pump), "h {h}: after {s:?}");
                            }
                        }
                        Pump => {
                            pumps += 1;
                            // never two in a row: each follows a strip (or is the only step, for no rows)
                            if i == 0 {
                                assert_eq!(h, 0);
                            } else {
                                assert!(matches!(steps[i - 1], Strip(..)), "h {h}: step {i}");
                            }
                        }
                    }
                }
                assert_eq!(next, TOP + h, "h {h}");
                assert!(seen.iter().all(|&n| n == 1), "h {h}: every row once");
                let last_short = h % SETTLE_STRIP_ROWS != 0 || h == 0;
                assert_eq!(pumps, h / SETTLE_STRIP_ROWS + (hit.is_none() && last_short) as i32, "h {h} hit {hit:?}");
                // the answer: the found pixels padded by the margin and clipped to the rect, or none
                let want = match hit {
                    Some(y) => rect_intersect(r(47, y - 3, 26, 7), r(10, TOP, 300, h)),
                    None => RECT_EMPTY,
                };
                assert_eq!(d, Some(want), "h {h} hit {hit:?}");
            }
        }
    }

    #[test]
    fn a_strip_that_cannot_answer_ends_the_scan() {
        let steps = std::cell::RefCell::new(Vec::new());
        let d = settle_area_strips(
            r(0, 0, 100, 400),
            |y0, y1| {
                steps.borrow_mut().push(Strip(y0, y1));
                if y0 == 128 { None } else { Some(r(5, y0, 1, 1)) }
            },
            || steps.borrow_mut().push(Pump),
        );
        assert_eq!(d, None);
        // no strip after it, and no call of the pump on its account
        assert_eq!(steps.into_inner(), [Strip(0, 64), Pump, Strip(64, 128), Pump, Strip(128, 192)]);
    }

    /// xorshift32
    fn rnd(s: &mut u32) -> u32 {
        *s ^= *s << 13;
        *s ^= *s >> 17;
        *s ^= *s << 5;
        *s
    }

    // A record kept as bytes, one a panel pixel, of a logical screen turned and flipped onto the panel as
    // display_fbink.rs maps it (to_fb), inverted or not: over random pictures and rects the scan in strips answers
    // what one pass over the rect answers (settle_area_by), and what the scan in one strip does.
    #[test]
    fn the_scan_in_strips_answers_what_one_pass_answers() {
        const PW: i32 = 150;
        const PH: i32 = 210;
        let mut seed = 0x0320_2023u32;
        let (mut answers, mut empties, mut tall, mut read) = (0, 0, 0, 0);
        for rot in 0..4 {
            for flip in [false, true] {
                for invert in [false, true] {
                    let (lw, lh) = if rot & 1 != 0 { (PH, PW) } else { (PW, PH) };
                    let to_fb = |lx: i32, ly: i32| {
                        let (px, py) = match rot {
                            1 => (ly, PH - 1 - lx),
                            2 => (PW - 1 - lx, PH - 1 - ly),
                            3 => (PW - 1 - ly, lx),
                            _ => (lx, ly),
                        };
                        if flip { (PW - 1 - px, PH - 1 - py) } else { (px, py) }
                    };
                    let mut screen = Canvas::default();
                    assert_eq!(canvas_init(&mut screen, lw, lh), 0);
                    for p in screen.px.iter_mut() {
                        *p = rnd(&mut seed) as u8;
                    }
                    // the panel was given the picture as it is
                    let mut shown = vec![0u8; (PW * PH) as usize];
                    for ly in 0..lh {
                        for lx in 0..lw {
                            let (fx, fy) = to_fb(lx, ly);
                            let v = screen.px[(ly * lw + lx) as usize];
                            shown[(fy * PW + fx) as usize] = if invert { 255 - v } else { v };
                        }
                    }
                    for round in 0..60 {
                        // since then a few pixels changed (none in some rounds), within their level or out of it
                        if round % 5 == 0 {
                            for ly in 0..lh {
                                for lx in 0..lw {
                                    let (fx, fy) = to_fb(lx, ly);
                                    let b = shown[(fy * PW + fx) as usize];
                                    screen.px[(ly * lw + lx) as usize] = if invert { 255 - b } else { b };
                                }
                            }
                        } else {
                            for _ in 0..rnd(&mut seed) % 4 {
                                let at = (rnd(&mut seed) % (lw * lh) as u32) as usize;
                                screen.px[at] = rnd(&mut seed) as u8;
                            }
                        }
                        // a rect of any place and size, clipped to the screen as display_settle_area clips it
                        let (x, y) = ((rnd(&mut seed) % (lw as u32 + 20)) as i32 - 10, (rnd(&mut seed) % (lh as u32 + 20)) as i32 - 10);
                        let (w, h) = ((rnd(&mut seed) % (lw as u32 + 20)) as i32, (rnd(&mut seed) % (lh as u32 + 20)) as i32);
                        let c = rect_intersect(r(x, y, w, h), r(0, 0, lw, lh));
                        let want = settle_area_by(&screen, c, |x, y| {
                            let (fx, fy) = to_fb(x, y);
                            let b = shown[(fy * PW + fx) as usize];
                            (if invert { 255 - b } else { b }) >> 4
                        });
                        let row_at = |y: i32| {
                            let ((fx0, fy0), (fx1, fy1)) = (to_fb(c.x, y), to_fb(c.x + 1, y));
                            ((fy0 * PW + fx0) as isize, ((fy1 - fy0) * PW + (fx1 - fx0)) as isize)
                        };
                        // (how often each row of the screen is asked for: the strips' row scans read a row once)
                        let rows = std::cell::RefCell::new(vec![0u32; lh as usize]);
                        let got = settle_area_strips(
                            c,
                            |y0, y1| {
                                settle_strip_by_bytes(&screen, c, y0, y1, &shown, invert, |y| {
                                    rows.borrow_mut()[y as usize] += 1;
                                    row_at(y)
                                })
                            },
                            || read += 1,
                        );
                        assert_eq!(got, Some(want), "rot {rot} flip {flip} invert {invert} round {round} rect {c:?}");
                        for (y, &n) in rows.borrow().iter().enumerate() {
                            let inside = c.y <= y as i32 && (y as i32) < c.y + c.h;
                            assert_eq!(n, inside as u32, "rot {rot} flip {flip} invert {invert} round {round} rect {c:?}: row {y} read {n} times");
                        }
                        let one = settle_strip_by_bytes(&screen, c, c.y, c.y + c.h, &shown, invert, row_at);
                        assert_eq!(one.map(|f| settle_area_pad(f, c)), Some(want), "rot {rot} flip {flip} invert {invert} round {round} rect {c:?}");
                        answers += 1;
                        empties += rect_is_empty(want) as i32;
                        tall += (c.h > SETTLE_STRIP_ROWS) as i32;
                    }
                }
            }
        }
        // (the pictures gave both answers, over rects of more than one strip too, with the pump called)
        assert!(empties > 100 && answers - empties > 100 && tall > 100 && read > 100, "{empties} of {answers} empty, {tall} tall, {read} reads");
    }

    // The settle answers' counts (SettleCounts): no line until one rises, each reason's first time told, and the line
    // names the reasons that occurred.
    #[test]
    fn settle_counts_make_a_line_only_when_one_rose() {
        let mut c = SettleCounts::default();
        assert_eq!(settle_counts_line(&mut c), None);
        assert_eq!(c, SettleCounts::default());
        settle_count_quiet(&mut c);
        settle_count_quiet(&mut c);
        settle_count_part(&mut c);
        assert_eq!(settle_counts_line(&mut c).as_deref(), Some("display: settles so far: quiet 2, part 1, whole 0"));
        assert_eq!(settle_counts_line(&mut c), None, "nothing rose since");
        // a whole answer: its reason's first time, once
        assert!(settle_count_whole(&mut c, SettleWhole::RefreshFailed));
        assert!(!settle_count_whole(&mut c, SettleWhole::RefreshFailed));
        assert!(settle_count_whole(&mut c, SettleWhole::NoFrame));
        assert_eq!(
            settle_counts_line(&mut c).as_deref(),
            Some("display: settles so far: quiet 2, part 1, whole 3 (no frame known 1, a refresh failed 2)")
        );
        assert_eq!(settle_counts_line(&mut c), None);
        // every reason has its name, in the enum's order
        for (k, why) in [
            SettleWhole::NoFrame,
            SettleWhole::RefreshFailed,
            SettleWhole::ColourPanel,
            SettleWhole::NoRecord,
            SettleWhole::RowRange,
            SettleWhole::Changed,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(why as usize, k);
            let mut one = SettleCounts::default();
            assert!(settle_count_whole(&mut one, why));
            let line = settle_counts_line(&mut one).unwrap();
            assert_eq!(line, format!("display: settles so far: quiet 0, part 0, whole 1 ({} 1)", K_SETTLE_WHOLE[k]));
        }
        // a count at its top stays there, and is no first time again
        let mut top = SettleCounts { quiet: u32::MAX, ..Default::default() };
        top.whole[SettleWhole::Changed as usize] = u32::MAX;
        settle_count_quiet(&mut top);
        assert!(!settle_count_whole(&mut top, SettleWhole::Changed));
        assert_eq!((top.quiet, top.whole[5]), (u32::MAX, u32::MAX));
    }

    #[test]
    fn a_row_outside_the_record_has_no_answer() {
        let mut screen = Canvas::default();
        assert_eq!(canvas_init(&mut screen, 40, 30), 0);
        let shown = vec![255u8; 40 * 30];
        let c = r(0, 0, 40, 30);
        let plain = |y: i32| ((y * 40) as isize, 1isize);
        assert_eq!(settle_strip_by_bytes(&screen, c, 0, 30, &shown, false, plain), Some(RECT_EMPTY));
        // a record a row short, one whose rows begin before it, and a row walked backward out of it
        assert_eq!(settle_strip_by_bytes(&screen, c, 0, 30, &shown[..40 * 29], false, plain), None);
        assert_eq!(settle_strip_by_bytes(&screen, c, 0, 29, &shown[..40 * 29], false, plain), Some(RECT_EMPTY));
        assert_eq!(settle_strip_by_bytes(&screen, c, 0, 30, &shown, false, |y| ((y * 40 - 1) as isize, 1)), None);
        assert_eq!(settle_strip_by_bytes(&screen, c, 0, 30, &shown, false, |y| ((y * 40 + 38) as isize, -1)), None);
        assert_eq!(settle_strip_by_bytes(&screen, c, 0, 30, &shown, false, |y| ((y * 40 + 39) as isize, -1)), Some(RECT_EMPTY));
        // the edge: a row whose farthest byte would be the first one past the record (byte 1200 of 1200) has no
        // answer, walked forward or backward, and the rows before it have theirs
        assert_eq!(settle_strip_by_bytes(&screen, c, 0, 30, &shown, false, |y| ((y * 40 + 1) as isize, 1)), None);
        assert_eq!(settle_strip_by_bytes(&screen, c, 0, 29, &shown, false, |y| ((y * 40 + 1) as isize, 1)), Some(RECT_EMPTY));
        assert_eq!(settle_strip_by_bytes(&screen, c, 0, 30, &shown, false, |y| ((y * 40 + 40) as isize, -1)), None);
        assert_eq!(settle_strip_by_bytes(&screen, c, 0, 29, &shown, false, |y| ((y * 40 + 40) as isize, -1)), Some(RECT_EMPTY));
        // and the scan in strips answers none then
        assert_eq!(settle_area_strips(c, |y0, y1| settle_strip_by_bytes(&screen, c, y0, y1, &shown[..40 * 29], false, plain), || {}), None);
    }
}
