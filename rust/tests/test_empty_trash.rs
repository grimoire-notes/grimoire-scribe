// Files' Empty trash (2.3, issue #330): fsview.rs's FsStorage::empty_trash, the job's work, on a tree of the host's
// own. It deletes exactly the entries named (the Trash place's rows when the user confirmed) and nothing outside the
// trash; a link in the trash, to a file or to a folder outside it, goes as a link and its target stays; an entry a
// restore is at work on (its lock held) stays, and a restore leaves one Empty trash is at work on; an entry that came
// into the trash during the job stays; a run whose time is up stops inside an entry with that entry out of the
// listing, and the next run removes it first; names that are not one of the trash's own, and a trash that is a link,
// are refused; a job that finds the trash held by another Empty trash deletes nothing and its toast says so. Then the
// times for 10, 1,000 and 5,000 entries (printed: docs/TESTING.md has the device check).
#![cfg(feature = "sim")]
mod testlib;
use grimoire::fsview::*;
use grimoire::prelude::*;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{symlink, MetadataExt};
use testlib::*;

fn sp(s: &str) -> StoragePath {
    StoragePath::new(s.as_bytes()).unwrap_or_else(|| panic!("{s}"))
}

/// A tree made afresh: the storage root and Grimoire's folder in it (its store set up there), a notebook, a template,
/// and an empty trash.
fn tree() -> (String, String) {
    let root = format!("{}/empty-storage", tl_tmp());
    let _ = std::fs::remove_dir_all(&root);
    let g = format!("{root}/grimoire");
    for d in ["documents", "grimoire/templates", "grimoire/photos/Trip", "grimoire/notebooks/20261101-000000-0009", "grimoire/trash"] {
        std::fs::create_dir_all(format!("{root}/{d}")).unwrap();
    }
    for (f, body) in [
        ("documents/a.pdf", &b"%PDF-1.4 a\n"[..]),
        ("grimoire/templates/a.png", b"template a"),
        ("grimoire/photos/Trip/p1.png", b"one"),
        ("grimoire/notebooks/20261101-000000-0009/notebook.ini", b"name=Live\nfolder=\n"),
    ] {
        std::fs::write(format!("{root}/{f}"), body).unwrap();
    }
    std::env::set_var("GRIMOIRE_STORAGE", &root);
    store_init(&g);
    (root, g)
}

fn open(root: &str, g: &str) -> FsStorage {
    FsStorage::open(root, g).unwrap()
}

fn names(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    v.sort();
    v
}

/// What the Trash place lists: the trash's own listing, read to its end, by name.
fn listed(st: &FsStorage) -> Vec<Vec<u8>> {
    let mut l = FsListing::start_trash(st).unwrap();
    while !l.step(FS_SLICE_MS) {}
    fsview_trash_entries(&l.entries).into_iter().map(|e| e.name).collect()
}

/// A notebook as store_delete leaves it in the trash: its folder, notebook.ini, `pages` page files of `size` bytes,
/// and a picture in bg/.
fn trashed_notebook(t: &str, name: &str, pages: usize, size: usize) {
    std::fs::create_dir_all(format!("{t}/{name}/bg")).unwrap();
    std::fs::write(format!("{t}/{name}/notebook.ini"), b"name=Gone\nfolder=\n").unwrap();
    for p in 1..=pages {
        std::fs::write(format!("{t}/{name}/p{p}.qpg"), vec![b'q'; size]).unwrap();
    }
    std::fs::write(format!("{t}/{name}/bg/index.ini"), b"1=a.png\n").unwrap();
    std::fs::write(format!("{t}/{name}/bg/1.pgm"), vec![b'g'; size]).unwrap();
}

fn all_time() -> impl FnMut() -> u32 {
    || 0
}

/// A folder of the test's own beside the storage root, with a file in it: what a link out of the trash points at.
fn outside() -> String {
    let o = format!("{}/empty-outside", tl_tmp());
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(format!("{o}/sub")).unwrap();
    std::fs::write(format!("{o}/hosts"), b"canary").unwrap();
    std::fs::write(format!("{o}/sub/keep"), b"kept").unwrap();
    o
}

fn outside_intact(o: &str) -> bool {
    std::fs::read(format!("{o}/hosts")).ok().as_deref() == Some(&b"canary"[..]) && std::fs::read(format!("{o}/sub/keep")).ok().as_deref() == Some(&b"kept"[..])
}

// Exactly the entries named go: a notebook's folder, a page's file, a file trashed from Grimoire's folders (its
// `files-` folder with its record), and a name of no known form; an entry not named stays, and so does everything
// outside the trash (the Library's notebooks, the templates, the storage's own files). The trash itself stays (the
// same folder), the staging folder goes, and the bytes said are the files' removed.
fn only_the_named_entries_go() {
    let (root, g) = tree();
    let t = format!("{g}/trash");
    let st = open(&root, &g);
    let trash_ino = std::fs::metadata(&t).unwrap().ino();
    trashed_notebook(&t, "20261101-000000-0001-1790000000", 3, 1000);
    std::fs::write(format!("{t}/20261101-000000-0001-p3-1790000500"), vec![b'p'; 700]).unwrap();
    let entry = st.trash(&sp("grimoire/photos/Trip"), &st.stat(&sp("grimoire/photos/Trip"), false).unwrap()).unwrap();
    std::fs::write(format!("{t}/notes"), vec![b'n'; 5]).unwrap();
    trashed_notebook(&t, "20261101-000000-0002-1790000900", 1, 10);
    let all = listed(&st);
    check!(all.len() == 5);
    let named: Vec<Vec<u8>> = all.iter().filter(|n| n.as_slice() != b"20261101-000000-0002-1790000900").cloned().collect();
    let bytes: u64 = named.iter().map(|n| files_bytes(&format!("{t}/{}", String::from_utf8_lossy(n)))).sum();
    check!(bytes == 18 + 3 * 1000 + 8 + 1000 + 700 + 3 + 5 + files_bytes(&format!("{t}/{}/.grimoire-trash", String::from_utf8_lossy(&entry))));
    let done = st.empty_trash(&named, 0, u32::MAX, &mut all_time()).unwrap();
    check!(done.entries == 4 && done.skipped == 0 && !done.more);
    check!(done.bytes == bytes);
    check!(names(&t) == ["20261101-000000-0002-1790000900"]);
    check!(std::fs::metadata(&t).unwrap().ino() == trash_ino);
    check!(names(&format!("{g}/notebooks/20261101-000000-0009")) == ["notebook.ini"]);
    check!(std::fs::read(format!("{root}/grimoire/templates/a.png")).ok().as_deref() == Some(&b"template a"[..]));
    check!(std::fs::read(format!("{root}/documents/a.pdf")).is_ok());
    check!(!std::path::Path::new(&format!("{root}/grimoire/photos/Trip")).exists());
    // (a second run with the same names finds nothing to do: every name is passed over)
    let again = st.empty_trash(&named, 0, u32::MAX, &mut all_time()).unwrap();
    check!(again == Emptied { next: named.len(), ..Emptied::default() });
    check!(names(&t) == ["20261101-000000-0002-1790000900"]);
}

/// The bytes of the regular files at `p` and under it (no link followed): what removing it frees.
fn files_bytes(p: &str) -> u64 {
    let Ok(m) = std::fs::symlink_metadata(p) else { return 0 };
    if m.is_file() {
        return m.len();
    }
    if !m.is_dir() {
        return 0;
    }
    std::fs::read_dir(p).map(|rd| rd.flatten().map(|e| files_bytes(&e.path().to_string_lossy())).sum()).unwrap_or(0)
}

// A link in the trash goes as a link, its target untouched: inside a notebook's folder, one to a file outside the
// storage and one to a folder outside it; and an entry of the trash that is itself a link (to a folder outside: what
// a swap leaves where a folder was listed). Nothing outside changes.
fn links_go_as_links() {
    let (root, g) = tree();
    let o = outside();
    let t = format!("{g}/trash");
    let st = open(&root, &g);
    trashed_notebook(&t, "20261101-000000-0001-1790000000", 1, 100);
    symlink(format!("{o}/hosts"), format!("{t}/20261101-000000-0001-1790000000/p2.qpg")).unwrap();
    symlink(&o, format!("{t}/20261101-000000-0001-1790000000/bg/pictures")).unwrap();
    symlink(&o, format!("{t}/20261101-000000-0003-1790000100")).unwrap();
    symlink(format!("{o}/hosts"), format!("{t}/20261101-000000-0003-p1-1790000200")).unwrap();
    let named: Vec<Vec<u8>> = ["20261101-000000-0001-1790000000", "20261101-000000-0003-1790000100", "20261101-000000-0003-p1-1790000200"].iter().map(|n| n.as_bytes().to_vec()).collect();
    let done = st.empty_trash(&named, 0, u32::MAX, &mut all_time()).unwrap();
    check!(done.entries == 3 && done.skipped == 0);
    // (only the notebook's own files are counted: a link frees nothing of what it points at)
    check!(done.bytes == 100 + 100 + 18 + 8);
    check!(names(&t).is_empty());
    check!(outside_intact(&o));
    check!(names(&o) == ["hosts", "sub"]);
}

// An entry a restore is at work on (its lock held, as FsStorage::restore holds it) stays: the others go, it is counted
// as left, and a run after the lock is let go takes it. The lock is the same both ways: a restore leaves an entry
// Empty trash holds.
fn a_busy_entry_stays() {
    let (root, g) = tree();
    let t = format!("{g}/trash");
    let st = open(&root, &g);
    trashed_notebook(&t, "20261101-000000-0001-1790000000", 1, 10);
    trashed_notebook(&t, "20261101-000000-0002-1790000100", 1, 10);
    let entry = st.trash(&sp("grimoire/templates/a.png"), &st.stat(&sp("grimoire/templates/a.png"), false).unwrap()).unwrap();
    let named = listed(&st);
    check!(named.len() == 3);
    // (a restore at work on the notebook, and another on the file's folder)
    let hold = |p: &str| {
        let f = std::fs::File::open(p).unwrap();
        // SAFETY: flock on the descriptor the File owns.
        check!(unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0);
        f
    };
    let held = hold(&format!("{t}/20261101-000000-0001-1790000000"));
    let held_file = hold(&format!("{t}/{}", String::from_utf8_lossy(&entry)));
    let done = st.empty_trash(&named, 0, u32::MAX, &mut all_time()).unwrap();
    check!(done.entries == 1 && done.skipped == 2);
    check!(names(&t).len() == 2 && names(&t).contains(&"20261101-000000-0001-1790000000".to_string()));
    check!(names(&format!("{t}/20261101-000000-0001-1790000000")).contains(&"notebook.ini".to_string()));
    check!(names(&format!("{t}/{}", String::from_utf8_lossy(&entry))) == [".grimoire-trash", "a.png"]);
    // the other way: a restore leaves an entry whose lock is held, and puts it back once it is let go
    check!(st.restore(b"20261101-000000-0001-1790000000") == Err(FsWriteErr::Io(libc::EWOULDBLOCK)));
    check!(st.restore(&entry) == Err(FsWriteErr::Io(libc::EWOULDBLOCK)));
    drop(held_file);
    check!(st.restore(&entry) == Ok(Restored::File(sp("grimoire/templates/a.png"))));
    drop(held);
    let done = st.empty_trash(&named, 0, u32::MAX, &mut all_time()).unwrap();
    check!(done.entries == 1 && done.skipped == 0);
    check!(names(&t).is_empty());
    check!(std::fs::read(format!("{root}/grimoire/templates/a.png")).ok().as_deref() == Some(&b"template a"[..]));
}

// An entry that comes into the trash while the job runs stays: a run whose time is up after its first entry (a clock
// that moves on a second a look), a delete into the trash meanwhile (a file from Grimoire's folders, a page, a
// notebook), then the runs that follow with the same names, to the end.
fn an_entry_that_comes_meanwhile_stays() {
    let (root, g) = tree();
    let t = format!("{g}/trash");
    let st = open(&root, &g);
    for i in 0..4 {
        std::fs::write(format!("{t}/20261101-000000-0001-p{}-17900000{i}0", i + 1), b"page").unwrap();
    }
    let named = listed(&st);
    let mut now = 0u32;
    let mut clock = || {
        now += 1000;
        now
    };
    let first = st.empty_trash(&named, 0, 500, &mut clock).unwrap();
    check!(first.more && first.entries == 1 && first.next == 1);
    check!(names(&t).len() == 3 + 1);
    // (deleted meanwhile, as the Library and Files delete)
    let came = st.trash(&sp("grimoire/templates/a.png"), &st.stat(&sp("grimoire/templates/a.png"), false).unwrap()).unwrap();
    std::fs::write(format!("{t}/20261101-000000-0002-p1-1790009999"), b"new page").unwrap();
    trashed_notebook(&t, "20261101-000000-0009-1790009999", 1, 10);
    let (mut runs, mut from) = (1, first.next);
    loop {
        let r = st.empty_trash(&named, from, 500, &mut clock).unwrap();
        from = r.next;
        runs += 1;
        check!(runs < 20);
        if !r.more || runs >= 20 {
            break;
        }
    }
    check!(runs >= 3);
    let left = names(&t);
    check!(left == ["20261101-000000-0002-p1-1790009999".to_string(), "20261101-000000-0009-1790009999".into(), String::from_utf8_lossy(&came).into_owned()]);
    check!(names(&format!("{t}/20261101-000000-0009-1790009999")).contains(&"notebook.ini".to_string()));
}

// A run whose time is up inside an entry leaves that entry in the staging folder: out of the trash's listing (so no
// row lists it and Restore cannot reach it), the rest of the trash listed whole. The next run removes it first, then
// goes on; at the end the staging folder is gone.
fn a_run_cut_short() {
    let (root, g) = tree();
    let t = format!("{g}/trash");
    let st = open(&root, &g);
    trashed_notebook(&t, "20261101-000000-0001-1790000500", 40, 10);
    trashed_notebook(&t, "20261101-000000-0002-1790000100", 1, 10);
    let named = listed(&st);
    check!(named[0] == b"20261101-000000-0001-1790000500");
    let mut now = 0u32;
    let mut clock = || {
        now += 1;
        now
    };
    let cut = st.empty_trash(&named, 0, 10, &mut clock).unwrap();
    check!(cut.more && cut.entries == 1 && cut.next == 1);
    // the big notebook is out of the listing, half removed in the staging folder; the other is listed whole
    check!(listed(&st) == [b"20261101-000000-0002-1790000100".to_vec()]);
    check!(names(&t) == [TRASH_EMPTYING, "20261101-000000-0002-1790000100"]);
    let staged = names(&format!("{t}/{TRASH_EMPTYING}"));
    check!(staged.len() == 1);
    let inside = names(&format!("{t}/{TRASH_EMPTYING}/{}", staged[0]));
    check!(!inside.is_empty() && inside.len() < 42);
    check!(st.restore(b"20261101-000000-0001-1790000500").is_err());
    check!(names(&format!("{g}/notebooks")) == ["20261101-000000-0009"]);
    // the next run, with all the time it needs
    let rest = st.empty_trash(&named, cut.next, u32::MAX, &mut all_time()).unwrap();
    check!(!rest.more && rest.entries == 1 && rest.next == 2);
    check!(names(&t).is_empty());
}

// Only names of the trash's own are acted on: a path, `.`, `..`, an empty name, the staging folder and any other
// dot-name are left (counted as left), and nothing outside the trash changes. A trash that is a link (to a folder of
// the test's own outside the storage) is not Grimoire's trash: nothing is done, and what it points at stays.
fn only_inside_the_trash() {
    let (root, g) = tree();
    let o = outside();
    let t = format!("{g}/trash");
    let st = open(&root, &g);
    std::fs::write(format!("{t}/.hidden"), b"h").unwrap();
    let bad: Vec<Vec<u8>> = ["../notebooks/20261101-000000-0009", "..", ".", "", ".hidden", TRASH_EMPTYING, "a/b"].iter().map(|n| n.as_bytes().to_vec()).collect();
    let done = st.empty_trash(&bad, 0, u32::MAX, &mut all_time()).unwrap();
    check!(done.entries == 0 && done.skipped == bad.len());
    check!(names(&t) == [".hidden"]);
    check!(names(&format!("{g}/notebooks/20261101-000000-0009")) == ["notebook.ini"]);
    // the trash swapped for a link
    std::fs::remove_dir_all(&t).unwrap();
    symlink(&o, &t).unwrap();
    check!(st.empty_trash(&[b"hosts".to_vec(), b"sub".to_vec()], 0, u32::MAX, &mut all_time()).is_err());
    check!(outside_intact(&o));
    std::fs::remove_file(&t).unwrap();
    std::fs::create_dir_all(&t).unwrap();
}

/// Polls the background jobs until none is queued or running (at most 20 s): true when they all ended.
fn jobs_end() -> bool {
    let t0 = std::time::Instant::now();
    while bg_busy() {
        if t0.elapsed().as_secs() > 20 {
            return false;
        }
        let mut kind = -1;
        if bg_poll_kind(Some(&mut kind)) == 0 && kind < 0 {
            sleep_ms(2);
        }
    }
    true
}

/// The staging folder's lock, held as another Empty trash at work on the trash holds it (the detached child of a
/// session before): the folder made, then flock(LOCK_EX) through a descriptor of the test's own.
fn hold_staging(t: &str) -> std::fs::File {
    let staging = format!("{t}/{TRASH_EMPTYING}");
    let _ = std::fs::create_dir(&staging);
    let f = std::fs::File::open(&staging).unwrap();
    // SAFETY: flock on the descriptor the File owns.
    check!(unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0);
    f
}

// An Empty trash that finds the trash's staging folder held by another (the detached child of a session before, at
// work on the same trash) deletes nothing: the job, asked as the Trash place asks it and forked as the app runs it,
// ends 1, every entry stays, and its toast says the trash was not emptied, never "Deleted 0 entries".
fn a_job_that_finds_the_trash_held() {
    let (root, g) = tree();
    let t = format!("{g}/trash");
    trashed_notebook(&t, "20261101-000000-0001-1790000000", 1, 10);
    trashed_notebook(&t, "20261101-000000-0002-1790000100", 1, 10);
    let named = listed(&open(&root, &g));
    let held = hold_staging(&t);
    check!(grimoire::bgwork_empty::empty_request(named));
    check!(jobs_end());
    check!(bg_last_exit() == 1);
    check!(names(&t) == [TRASH_EMPTYING, "20261101-000000-0001-1790000000", "20261101-000000-0002-1790000100"]);
    check!(grimoire::bgwork_empty::empty_finished(bg_last_exit() != 0) == "Couldn't empty the trash.");
    drop(held);
}

/// A trash of `n` entries as a long use leaves it: four in five a page's file (2 KB), about one in seven a notebook's
/// folder (four pages of 2 KB and a picture), the rest a file trashed from Grimoire's folders (its folder, its record
/// and a 3 KB picture).
fn big_trash(t: &str, n: usize) {
    for i in 0..n {
        match i % 20 {
            0..=15 => std::fs::write(format!("{t}/20261101-000000-0001-p{}-{}", i + 1, 1_700_000_000 + i), vec![b'p'; 2048]).unwrap(),
            16..=18 => trashed_notebook(t, &format!("20261101-{:06}-0001-{}", i, 1_700_000_000 + i), 4, 2048),
            _ => {
                let w = format!("{t}/files-20261101-000000-{:04x}", i & 0xffff);
                std::fs::create_dir_all(&w).unwrap();
                std::fs::write(format!("{w}/.grimoire-trash"), b"# Grimoire\nfrom=grimoire/photos/x.png\ntime=1700000000\n").unwrap();
                std::fs::write(format!("{w}/x.png"), vec![b'x'; 3072]).unwrap();
            }
        }
    }
}

// The times (printed, never checked: the machine decides them): a trash of 10, 1,000 and 5,000 entries emptied in one
// run with all the time it needs, and how many runs of the job's own slice (EMPTY_SLICE_MS) that takes.
fn the_times() {
    for n in [10, 1000, 5000] {
        let (root, g) = tree();
        let t = format!("{g}/trash");
        big_trash(&t, n);
        let st = open(&root, &g);
        let named = listed(&st);
        check!(named.len() == n.min(FS_LIST_MAX));
        let t0 = std::time::Instant::now();
        let done = st.empty_trash(&named, 0, u32::MAX, &mut all_time()).unwrap();
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        check!(done.entries == n && !done.more);
        check!(names(&t).is_empty());
        // the same trash again, in the job's slices
        big_trash(&t, n);
        let named = listed(&st);
        let t0 = std::time::Instant::now();
        let (mut runs, mut from) = (0, 0);
        loop {
            runs += 1;
            let r = st.empty_trash(&named, from, grimoire::bgwork_empty::EMPTY_SLICE_MS, &mut real_now_ms).unwrap();
            from = r.next;
            if !r.more || runs > 1000 {
                break;
            }
        }
        check!(names(&t).is_empty());
        eprintln!("  empty trash: {n} entries in {ms:.1} ms (one run); in slices: {runs} run(s), {:.1} ms", t0.elapsed().as_secs_f64() * 1000.0);
    }
}

#[test]
fn test_empty_trash() {
    run!(only_the_named_entries_go);
    run!(links_go_as_links);
    run!(a_busy_entry_stays);
    run!(an_entry_that_comes_meanwhile_stays);
    run!(a_run_cut_short);
    run!(only_inside_the_trash);
    run!(a_job_that_finds_the_trash_held);
    run!(the_times);
    std::env::remove_var("GRIMOIRE_STORAGE");
    let _ = std::fs::remove_dir_all(format!("{}/empty-storage", tl_tmp()));
    let _ = std::fs::remove_dir_all(format!("{}/empty-outside", tl_tmp()));
    done();
}
