//! Files' Empty trash (BgKind::EmptyTrash, 2.3, issue #330): what the Trash place's question does when the user
//! confirms it. The request (UI thread, `empty_request`) keeps the names of the entries the place listed then and
//! queues the job as one the user waits on. The job (a background child: a fork, so it has those names) deletes them
//! inside the trash through fsview's `FsStorage::empty_trash` (the trash known by its identity, no link followed, an
//! entry a restore or a delete is at work on left, an entry that came since never taken), a slice of at most
//! EMPTY_SLICE_MS a run: a run whose time is up ends CODE_MORE and is queued again behind whatever ranks higher (an
//! export, the mirror), so a big trash never holds the one background child for long. Each run adds what it did to a
//! small result file in Grimoire's own `.cache` folder (`trash-emptied`: the request's tag, the entries it took out,
//! the bytes of their files, the names left, the first name the next run looks at: each name is looked at once), which
//! the UI reads once the last run has ended (`empty_finished`) for its toast. A count under another request's tag
//! (the detached child of a session before, still at work) is never this request's. The log has counts and times,
//! never a name.

use crate::fsview::*;
use crate::prelude::*;

/// The job's path, its only one: one Empty trash at a time.
pub const EMPTY_JOB_PATH: &str = "trash";
/// A run's time, by the machine's clock: then it ends CODE_MORE and the next run goes on (2 s, as an import's slice is
/// held to its time, so an export asked for meanwhile waits at most that).
pub const EMPTY_SLICE_MS: u32 = 2000;
/// A run's exit status when names are left for the next (bgjob.rs queues it again).
pub const CODE_MORE: i32 = 3;
/// The result file's largest size (five short lines).
const RESULT_MAX: usize = 256;

/// The request the UI made last: the names it was given, and its tag (`empty_request`), which the result file carries
/// (`job=`) while the count in it is this request's. A child reads its own copy, as it was when it was forked.
struct Request {
    names: Vec<Vec<u8>>,
    tag: String,
}
static REQUEST: std::sync::Mutex<Request> = std::sync::Mutex::new(Request { names: Vec::new(), tag: String::new() });
/// (the requests made in this process, for their tags)
static REQUESTS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn request() -> std::sync::MutexGuard<'static, Request> {
    REQUEST.lock().unwrap_or_else(|e| e.into_inner())
}

fn result_path() -> String {
    format!("{}/.cache/trash-emptied", store_root())
}

/// Whether an Empty trash is queued or running (between two of its runs too): the Trash place offers none then, and
/// no Restore.
pub fn empty_pending() -> bool {
    bg_kind_count(BgKind::EmptyTrash) > 0
}

/// Empty trash confirmed on these entries of the trash (their names, as the Trash place listed them): the job is
/// queued as one the user waits on, under a tag of its own (this process, the time, the requests made before), and a
/// result an earlier one left is let go. False, with nothing asked, when one is queued or running already or no name
/// is given.
pub fn empty_request(names: Vec<Vec<u8>>) -> bool {
    if empty_pending() || names.is_empty() {
        return false;
    }
    let _ = std::fs::remove_file(result_path());
    let n = REQUESTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    *request() = Request { names, tag: format!("{}.{}.{n}", std::process::id(), real_now_ms()) };
    bg_queue(BgKind::EmptyTrash, None, Some(EMPTY_JOB_PATH), 0.0, true);
    true
}

/// What the result file says, the runs before together, when it is this request's (it carries the request's tag):
/// the entries taken out, their bytes, the names left, and the first name no run has looked at yet. None for no file,
/// one another request's runs wrote, or one that does not read: a run then starts from the first name (one done
/// already is passed over) and counts from nothing.
fn result_read() -> Option<Emptied> {
    let t = read_file_max(&result_path(), RESULT_MAX)?;
    let tag = request().tag.clone();
    let text = String::from_utf8_lossy(&t);
    if tag.is_empty() || !text.lines().any(|l| l.strip_prefix("job=") == Some(tag.as_str())) {
        return None;
    }
    let mut r = Emptied::default();
    for line in text.lines() {
        match line.split_once('=') {
            Some(("entries", v)) => r.entries = v.parse().unwrap_or(0),
            Some(("bytes", v)) => r.bytes = v.parse().unwrap_or(0),
            Some(("left", v)) => r.skipped = v.parse().unwrap_or(0),
            Some(("next", v)) => r.next = v.parse().unwrap_or(0),
            _ => {}
        }
    }
    Some(r)
}

/// The job (in the child): one run over the names the request kept. 0 when every name is done (deleted, gone, or
/// left as busy), CODE_MORE when its time was up first, 1 when the trash could not be emptied at all (no storage, the
/// trash not Grimoire's own, another Empty trash at work on it).
pub fn bg_empty_trash(path: &str) -> i32 {
    if path != EMPTY_JOB_PATH {
        return 1;
    }
    let (names, tag) = {
        let r = request();
        (r.names.clone(), r.tag.clone())
    };
    let t0 = real_now_ms();
    let Some(root) = crate::pdfimport::storage_root() else {
        qlog!("bg: the trash was not emptied: no storage");
        return 1;
    };
    let st = match FsStorage::open(&root, &store_root()) {
        Ok(st) => st,
        Err(e) => {
            qlog!("bg: the trash was not emptied: %s", &format!("{e:?}"));
            return 1;
        }
    };
    #[cfg(feature = "sim")]
    let _held = test_busy();
    let before = result_read().unwrap_or_default();
    match st.empty_trash(&names, before.next, EMPTY_SLICE_MS, &mut real_now_ms) {
        Ok(e) => {
            // (each run adds what it did to the runs' before it, and says where the next goes on, under its request's
            // tag: a count another request's runs left in the file is written over)
            let (entries, bytes, left) = (before.entries + e.entries, before.bytes + e.bytes, before.skipped + e.skipped);
            let _ = crate::util::mkdir_p(&format!("{}/.cache", store_root()));
            let count = format!("job={tag}\nentries={entries}\nbytes={bytes}\nleft={left}\nnext={}\n", e.next);
            if write_file_atomic(&result_path(), count.as_bytes()) != 0 {
                qlog!("bg: the trash's emptying could not keep its count");
            }
            qlog!(
                "bg: the trash's emptying: %d entries, %s KB, %d left, in %d ms%s",
                e.entries as i32,
                &(e.bytes >> 10).to_string(),
                e.skipped as i32,
                real_now_ms().wrapping_sub(t0) as i32,
                if e.more { " (more in the next run)" } else { "" }
            );
            if e.more {
                CODE_MORE
            } else {
                0
            }
        }
        Err(e) => {
            qlog!("bg: the trash was not emptied: %s", &format!("{e:?}"));
            1
        }
    }
}

/// Tests (the headless build's scenarios, never the Kindle's): with GRIMOIRE_EMPTY_TEST_BUSY set, the run holds the
/// staging folder's lock through a descriptor of its own, as another Empty trash at work on the trash holds it (the
/// detached child of a session before), so `FsStorage::empty_trash` finds it held and the run ends 1 (docs/SIM.md).
#[cfg(feature = "sim")]
fn test_busy() -> Option<std::fs::File> {
    std::env::var("GRIMOIRE_EMPTY_TEST_BUSY").ok().filter(|v| !v.is_empty())?;
    let p = format!("{}/trash/{TRASH_EMPTYING}", store_root());
    let _ = std::fs::create_dir(&p);
    let f = std::fs::File::open(&p).ok()?;
    // SAFETY: flock on the descriptor the File owns; the lock goes with it.
    unsafe { libc::flock(std::os::fd::AsRawFd::as_raw_fd(&f), libc::LOCK_EX | libc::LOCK_NB) };
    Some(f)
}

/// What the Empty trash that just ended did, read from its result file (removed here when it is this request's; one
/// another request's runs keep is left to them), the names it was given forgotten: the toast that says it
/// (`empty_said`). `failed`: its last run ended without emptying the trash (its exit status was not 0: the trash could
/// not be opened or written, or another Empty trash held it).
pub fn empty_finished(failed: bool) -> String {
    let mine = result_read();
    if mine.is_some() {
        let _ = std::fs::remove_file(result_path());
    }
    let r = mine.unwrap_or_default();
    request().names.clear();
    qlog!(
        "files: the trash's emptying ended: %d entries, %s KB, %d left%s",
        r.entries as i32,
        &(r.bytes >> 10).to_string(),
        r.skipped as i32,
        if failed { " (its last run failed)" } else { "" }
    );
    empty_said(r.entries, r.bytes, failed)
}

/// The toast at its end: how many entries went and the space freed; or, when its last run failed and no run deleted
/// anything, that the trash was not emptied (never "Deleted 0 entries" for a job that could not work). A job that
/// deleted some entries before its last run failed says what it deleted.
pub fn empty_said(entries: usize, bytes: u64, failed: bool) -> String {
    if failed && entries == 0 {
        // TRANSLATORS: a toast in Files' Trash (or wherever the user is) when Empty trash deleted nothing because it
        // could not work on the trash (it could not be written, or another Empty trash was at work on it). One line
        // of about 1500 px at 44 px.
        return tr("Couldn't empty the trash.").to_string();
    }
    // (nothing freed says 0 KB, as the question does)
    let size = crate::browser::empty_size(bytes);
    // TRANSLATORS: a toast in Files' Trash (or wherever the user is) once Empty trash has ended: %d is how many
    // entries of the trash were deleted for good, %s the space that freed ("Deleted 12 entries, freed 3.4 MB."). One
    // line of about 1500 px at 44 px.
    cformat!(ng_("Deleted %d entry, freed %s.", "Deleted %d entries, freed %s.", entries as i64), entries as i32, size.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    // The toast says how many entries went and the space they freed, 0 KB for nothing (never the "1 KB" a size label
    // gives every small size); a job whose last run failed with nothing deleted says the trash was not emptied, one
    // that deleted some first says what it deleted; and the job takes only its own path.
    #[test]
    fn the_toast_and_the_path() {
        assert_eq!(empty_said(12, 3_565_158, false), "Deleted 12 entries, freed 3.4 MB.");
        assert_eq!(empty_said(1, 2048, false), "Deleted 1 entry, freed 2 KB.");
        assert_eq!(empty_said(3, 0, false), "Deleted 3 entries, freed 0 KB.");
        assert_eq!(empty_said(0, 0, false), "Deleted 0 entries, freed 0 KB.");
        assert_eq!(empty_said(0, 0, true), "Couldn't empty the trash.");
        assert_eq!(empty_said(2, 7168, true), "Deleted 2 entries, freed 7 KB.");
        assert_eq!(bg_empty_trash("elsewhere"), 1);
    }
}
