//! What the two WebAssembly hosts share: widgetwasm.rs (a widget's frame over the notebook) and apphost.rs (a
//! full-screen app). Each runs a module from an installed package with the wasmi interpreter, inside
//! Grimoire's own process, and each holds its module to rules of the same shape with numbers of its own
//! (docs/WIDGETS.md and docs/APPS.md, Limits). The rules are written once, here, so the two hosts keep them
//! alike: a store's limits, how a call past a limit ends, how a module's memory is read and written, and how
//! the numbers a module draws with are held near its body.

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
