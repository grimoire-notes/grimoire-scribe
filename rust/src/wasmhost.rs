//! What the two WebAssembly hosts share: widgetwasm.rs (a widget's frame over the notebook) and apphost.rs (a
//! full-screen app). Each runs a module from an installed package with the wasmi interpreter, inside
//! Grimoire's own process, and each holds its module to rules of the same shape with numbers of its own
//! (docs/WIDGETS.md and docs/APPS.md, Limits). The rules are written once, here, so the two hosts keep them
//! alike.

use wasmi::{StoreLimits, StoreLimitsBuilder};

/// A store's limits: one instance of one module, with one linear memory of at most `memory_max` bytes and at
/// most one table of at most `table_max` entries (the function references a module's indirect calls go
/// through). A module that declares a second memory or table, or one larger than its limit, is not started;
/// `memory.grow` and `table.grow` past a limit fail as WebAssembly defines it (they return -1), and the module
/// runs on.
pub fn store_limits(memory_max: usize, table_max: usize) -> StoreLimits {
    StoreLimitsBuilder::new().memory_size(memory_max).memories(1).tables(1).table_elements(table_max).instances(1).build()
}
