//! What the two WebAssembly hosts share: widgetwasm.rs (a widget's frame over the notebook) and apphost.rs (a
//! full-screen app). Each runs a module from an installed package with the wasmi interpreter, inside
//! Grimoire's own process, and each holds its module to rules of the same shape with numbers of its own
//! (docs/WIDGETS.md and docs/APPS.md, Limits). The rules are written once, here, so the two hosts keep them
//! alike: a store's limits, how a call past a limit ends, and how a module's memory is read and written.

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
