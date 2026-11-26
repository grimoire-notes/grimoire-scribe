//! What the two WebAssembly hosts share: widgetwasm.rs (a widget's frame over the notebook) and apphost.rs (a
//! full-screen app). Each runs a module from an installed package with the wasmi interpreter, inside
//! Grimoire's own process, and each holds its module to rules of the same shape with numbers of its own
//! (docs/WIDGETS.md and docs/APPS.md, Limits). The rules are written once, here, so the two hosts keep them
//! alike: a store's limits, how a call past a limit ends, how a module's memory is read and written, how the
//! numbers a module draws with are held near its body, and how the host's work for an import is paid from the
//! call's instruction budget.

use crate::prelude::Rect;
use wasmi::{Caller, Extern, StoreLimits, StoreLimitsBuilder, TrapCode};

/// A store's limits: one instance of one module, with one linear memory of at most `memory_max` bytes and at
/// most one table of at most `table_max` entries (the function references a module's indirect calls go
/// through). A module that declares a second memory or table, or one larger than its limit, is not started;
/// `memory.grow` and `table.grow` past a limit fail as WebAssembly defines it (they return -1), and the module
/// runs on.
pub fn store_limits(memory_max: usize, table_max: usize) -> StoreLimits {
    StoreLimitsBuilder::new().memory_size(memory_max).memories(1).tables(1).table_elements(table_max).instances(1).build()
}

/// How a call that went past a limit of its host ends: with the interpreter's own trap for a call that used up
/// its instructions, so that the two end alike for the module and for what the host shows.
pub fn out_of_fuel() -> wasmi::Error {
    wasmi::Error::from(TrapCode::OutOfFuel)
}

// ---- the charges: the host's work for an import, paid from the call's instruction budget ----
//
// A call's budget counts the module's instructions. The host's work for an import is no instruction of the
// module's, so each import takes a charge for it from the same budget: a fixed one for every call, and one by
// size where the work grows with what the import is given. The way a charge is taken (`charge`) and the shapes
// of the charges by size (`TextCharge`, `area_in_body`) are here, for both hosts; the numbers are each host's
// own (docs/WIDGETS.md and docs/APPS.md, "What a call's budget pays for").

/// Takes `n` from the call's instruction budget for work the host does for the call. When the budget cannot
/// pay, the call ends exactly as a call that used up its instructions ends: nothing is left of the budget, and
/// the trap is the interpreter's own for that.
pub fn charge<T>(caller: &mut Caller<'_, T>, n: u64) -> Result<(), wasmi::Error> {
    let left = caller.get_fuel().unwrap_or(0);
    if n > left {
        let _ = caller.set_fuel(0);
        return Err(out_of_fuel());
    }
    let _ = caller.set_fuel(left - n);
    Ok(())
}

/// The pixels of a command's rectangle (relative to the body's top left) that lie inside a body of this size.
pub fn area_in_body(body: (i32, i32), x: i32, y: i32, w: i32, h: i32) -> u64 {
    in_body(Rect { x: 0, y: 0, w: body.0, h: body.1 }, Rect { x, y, w, h }).map_or(0, |r| r.w as u64 * r.h as u64)
}

/// A host's numbers for the charge of drawing a text.
///
/// For each byte, `byte_px` for each pixel of the text's height up to `tall_px`; a taller letter costs by its
/// area, so above that height a byte is charged by the square of the height (the same at `tall_px` exactly).
/// And `square` for the square of the text's bytes, where the text is measured again and again to fit it to its
/// room (cut short, or broken into lines): the caller says up to how many bytes those steps look at.
#[derive(Clone, Copy, Debug)]
pub struct TextCharge {
    pub byte_px: u64,
    pub tall_px: u64,
    pub square: u64,
    /// the height a text is drawn at most: a larger one is paid for as this one, as it is drawn
    pub px_max: u64,
}

impl TextCharge {
    /// The charge for a text of `bytes` bytes `px` pixels high (1 to `px_max`), its bytes counted to
    /// `square_max` in the square.
    pub fn of(&self, bytes: usize, px: i64, square_max: u64) -> u64 {
        let (n, h) = (bytes as u64, px.clamp(1, self.px_max as i64) as u64);
        let m = n.min(square_max);
        n * h * h.max(self.tall_px) * self.byte_px / self.tall_px + self.square * m * m
    }
}

/// `len` bytes of the module's memory from `ptr`, or as many as the memory has there; none for a `ptr` or a
/// `len` that is not positive, or a module with no memory. A host compares `len` with its limit before it
/// calls this: what is asked for here is copied.
pub fn read_mem<T>(caller: &Caller<'_, T>, ptr: i32, len: i32) -> Vec<u8> {
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

/// Writes `bytes` into the module's memory at `ptr`: as many as `cap` and the memory take. Returns how many.
pub fn write_mem<T>(caller: &mut Caller<'_, T>, ptr: i32, cap: i32, bytes: &[u8]) -> i32 {
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

/// How far outside the body, in pixels, an edge of a command's rectangle is still kept as given. A farther edge
/// is moved in to this distance, so the sums the drawing makes with the rectangle stay small. A rectangle that
/// reaches no farther is drawn exactly as given; one that does is drawn as the rectangle with its edges there
/// (the same picture in the body for a filled one, and for an outline or corners narrower than the reach).
pub const DRAW_REACH: i64 = 1 << 16;

/// A command's rectangle on the screen, from its numbers relative to the body's top left. A module passes any
/// numbers: the edges are summed in 64 bits and held within DRAW_REACH of the body, so no later sum overflows.
/// A width or height that is not positive stays so (an empty rectangle draws nothing).
pub fn on_screen(body: Rect, x: i32, y: i32, w: i32, h: i32) -> Rect {
    fn edge(origin: i32, len: i32, at: i64) -> i32 {
        (origin as i64 + at).clamp(origin as i64 - DRAW_REACH, origin as i64 + len.max(0) as i64 + DRAW_REACH) as i32
    }
    let (x0, x1) = (edge(body.x, body.w, x as i64), edge(body.x, body.w, x as i64 + w as i64));
    let (y0, y1) = (edge(body.y, body.h, y as i64), edge(body.y, body.h, y as i64 + h as i64));
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// The part of a rectangle (relative to the body's top left) that is inside the body, or None when no part is.
pub fn in_body(body: Rect, r: Rect) -> Option<Rect> {
    let x0 = (r.x as i64).max(0);
    let y0 = (r.y as i64).max(0);
    let x1 = (r.x as i64 + r.w as i64).min(body.w as i64);
    let y1 = (r.y as i64 + r.h as i64).min(body.h as i64);
    (x1 > x0 && y1 > y0).then(|| Rect { x: x0 as i32, y: y0 as i32, w: (x1 - x0) as i32, h: (y1 - y0) as i32 })
}

/// An end of a line farther from the body than this, each way by itself, is first taken at this distance: the
/// cut below is then worked out to a small part of a pixel, whatever numbers the module passed.
const LINE_FAR: f64 = (1u64 << 30) as f64;

/// A line's ends, relative to the body's top left, held within DRAW_REACH of the body: a line inside that reach
/// is kept as given, and one that reaches farther is cut where it leaves the reach, on the line it is on, so
/// what shows in the body is the same. None for a line with no part in that reach, or with an end that is not a
/// number.
pub fn line_in_reach(body: Rect, x0: f32, y0: f32, x1: f32, y1: f32) -> Option<(f32, f32, f32, f32)> {
    if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
        return None;
    }
    let reach = DRAW_REACH as f64;
    let (far_x, far_y) = (body.w.max(0) as f64 + reach, body.h.max(0) as f64 + reach);
    let within = |x: f32, y: f32| (-reach..=far_x).contains(&(x as f64)) && (-reach..=far_y).contains(&(y as f64));
    if within(x0, y0) && within(x1, y1) {
        return Some((x0, y0, x1, y1));
    }
    // (the part of the line between its ends that lies in the reach: from t0 to t1 of the way along it)
    let near = |v: f32| (v as f64).clamp(-LINE_FAR, LINE_FAR);
    let (ax, ay, bx, by) = (near(x0), near(y0), near(x1), near(y1));
    let (dx, dy) = (bx - ax, by - ay);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (step, room) in [(-dx, ax + reach), (dx, far_x - ax), (-dy, ay + reach), (dy, far_y - ay)] {
        if step == 0.0 {
            if room < 0.0 {
                return None;
            }
        } else if step < 0.0 {
            t0 = t0.max(room / step);
        } else {
            t1 = t1.min(room / step);
        }
    }
    // (an end that is in the reach stays the number it is)
    let at = |t: f64| if t <= 0.0 { (ax as f32, ay as f32) } else if t >= 1.0 { (bx as f32, by as f32) } else { ((ax + t * dx) as f32, (ay + t * dy) as f32) };
    (t0 <= t1).then(|| {
        let ((x0, y0), (x1, y1)) = (at(t0), at(t1));
        (x0, y0, x1, y1)
    })
}
