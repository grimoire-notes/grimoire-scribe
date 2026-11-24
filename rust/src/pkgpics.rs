//! Packages' pictures (2.3, issue #236; docs/DESIGN.md, "Packages' pictures"): an installed app's bundled `.ico` and
//! the Package Store's catalog icons and screenshots come from outside Grimoire, so the safe loader's picture worker
//! decodes them, never the UI and never a job's writer, and what a screen draws is a gray picture read back from a
//! cache and checked. As Files' previews (bgwork_previews.rs), whose pool, key, try rule and prune this uses.
//!
//! - What a screen asks (UI thread): `PicSrc::of` stats the file (no byte of it is read), `picture_cached` reads
//!   what the cache holds for it, and with nothing there `picture_request` queues a job of its own
//!   (BgKind::PackagePictures; path: `<form>.<size>.<mtime>:<the file's path from Grimoire's folder>`), at most
//!   PICTURES_AT_ONCE of them queued or running at a time, and none in a session in which a sandbox step failed.
//!   It ranks with Files' previews (bgjob.rs `rank`): never a job the user waits on by name.
//! - The job (a background child, `bg_package_picture`): takes only a file of the form's own folder
//!   (`packages/apps/` for an app's icon, `package-store/assets/` for the Store's), opens it from Grimoire's folder
//!   one part at a time, never through a link, checks it is still the file asked for (its size and time), and has
//!   ONE run of the picture worker make the picture (picwork.rs `T`, a pool that never decodes in the writer,
//!   held to a preview's time): an `.ico`'s best small entry, or a PNG, JPEG or PNM picture, gray, one over the
//!   form's side fitted inside it. The writer checks the answer (`PicPool::small`) and writes the cache.
//! - What a job ends with, by previews' rule for a try: a picture; a "no picture" mark at once when the worker
//!   refuses the bytes (the same bytes give the same answer); a try on record (`once`) the first time a worker
//!   ends, lies or runs out of time, and the mark only the second time; or nothing at all when no worker can be
//!   had, memory or room is short, or the file changed or is gone.
//! - The cache: grimoire/.cache/packages/icons/ and grimoire/.cache/packages/shots/, `<key>.pgm` and `<key>.info`
//!   (docs/FORMAT.md), `<key>` the FNV-1a 64 hash of the file's path, size and time (`preview_key`). Written
//!   atomically by the job alone, the picture first and then the `.info` that says it is whole; read by the UI
//!   with the plain-file read and a size limit, the `.info` parsed strictly and held to the file asked for (its
//!   path, size, time and form), the picture to the `.info`: its header and exactly w * h bytes, within the
//!   form's side. After a job that wrote, the folder past its limits loses its oldest entries (`prune_plan`).
//!   An app's entry goes with the app (apppkg.rs). Deleting the folder loses nothing.
//!
//! grimoire.log never has a file's name here: forms, sizes, timings and reasons.

use crate::bgwork_previews::{num_i64, num_u64, preview_key, preview_limit_ms, prune_plan, CacheEntry, Made, NoPreview, PreviewKind};
use crate::picwork::{PicOut, PicPool, SMALL_ICO, SMALL_PICTURE, STALLED};
use crate::prelude::*;
use crate::sandbox::mono_ms;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::MetadataExt;

/// At most this many picture jobs are queued or running at a time (the job queue holds 16): a screen with more
/// pictures asks for the rest as these end.
pub const PICTURES_AT_ONCE: usize = 4;
/// The number in a `.info`'s first line: it stands for the text and for the rules a mark was made by. An entry of
/// another number is no entry.
pub const INFO_NUMBER: u32 = 1;
/// The longest path from Grimoire's folder a picture may have (the Store's folder and its longest key).
const REL_MAX: usize = 540;
/// A `.info` file is at most this long: four short lines and the path.
const INFO_MAX: usize = 160 + REL_MAX;
/// An icon is at most this many pixels a side (the `.ico` reader's entry limit, and the Store's icons').
pub const ICON_SIDE: i32 = 256;
/// A screenshot is kept at most this many pixels a side (a Scribe page, 1860 x 2480, is within it).
pub const SHOT_SIDE: i32 = 2560;
/// The largest catalog screenshot file read (packagestore.rs downloads none larger).
pub const SHOT_FILE_MAX: usize = 8 * 1024 * 1024;

/// What a picture is, which says where its file may be, what the worker reads it as and how big it may come back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PicForm {
    /// an installed app's bundled `.ico`, under `packages/apps/`
    AppIcon,
    /// a Package Store catalog icon (an `.ico`, a PNG or a JPEG), under `package-store/assets/`
    StoreIcon,
    /// a Package Store screenshot (a PNG or a JPEG), under `package-store/assets/`
    Screenshot,
}

impl PicForm {
    const ALL: [PicForm; 3] = [PicForm::AppIcon, PicForm::StoreIcon, PicForm::Screenshot];

    /// Its letter in a job's path.
    fn letter(self) -> &'static str {
        match self {
            PicForm::AppIcon => "a",
            PicForm::StoreIcon => "i",
            PicForm::Screenshot => "s",
        }
    }
    /// Its word in a `.info` and in the log.
    pub fn word(self) -> &'static str {
        match self {
            PicForm::AppIcon => "app-icon",
            PicForm::StoreIcon => "icon",
            PicForm::Screenshot => "screenshot",
        }
    }
    /// The folder of Grimoire's its file is in.
    fn under(self) -> &'static str {
        match self {
            PicForm::AppIcon => "packages/apps/",
            PicForm::StoreIcon | PicForm::Screenshot => "package-store/assets/",
        }
    }
    /// What the worker reads it as (picwork.rs `T`).
    fn what(self) -> usize {
        match self {
            PicForm::AppIcon => SMALL_ICO,
            PicForm::StoreIcon => SMALL_ICO | SMALL_PICTURE,
            PicForm::Screenshot => SMALL_PICTURE,
        }
    }
    /// The most pixels a side its picture has.
    pub fn side_max(self) -> i32 {
        match self {
            PicForm::AppIcon | PicForm::StoreIcon => ICON_SIDE,
            PicForm::Screenshot => SHOT_SIDE,
        }
    }
    /// The largest file of it that is read.
    pub fn file_max(self) -> usize {
        match self {
            PicForm::AppIcon | PicForm::StoreIcon => crate::picwork::ICO_FILE_MAX,
            PicForm::Screenshot => SHOT_FILE_MAX,
        }
    }
    /// Its cache folder's name under `.cache/packages/`, and that folder's limits: files, bytes.
    fn cache(self) -> (&'static str, usize, u64) {
        match self {
            PicForm::AppIcon | PicForm::StoreIcon => ("icons", 1200, 24 << 20),
            PicForm::Screenshot => ("shots", 32, 48 << 20),
        }
    }
}

/// A picture's file as a screen asked for it: its path from Grimoire's folder, its size and time, and its form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PicSrc {
    pub rel: String,
    pub size: u64,
    pub mtime: i64,
    pub form: PicForm,
}

/// Whether `rel` is a path a picture of this form may have: text without a control character or a backslash, no
/// empty, `.` or `..` part, in the form's own folder, within REL_MAX.
fn rel_ok(rel: &str, form: PicForm) -> bool {
    rel.len() <= REL_MAX
        && !rel.chars().any(|c| c.is_control() || c == '\\')
        && rel.strip_prefix(form.under()).is_some_and(|rest| !rest.is_empty())
        && rel.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

impl PicSrc {
    /// The file at `path` (inside Grimoire's folder) as it is now, None when its path is not one a picture of this
    /// form may have, nothing or no plain file stands there (a link is not followed), or it is over the form's
    /// size. One stat: no byte of the file is read.
    pub fn of(path: &str, form: PicForm) -> Option<PicSrc> {
        let root = store_root();
        let rel = path.strip_prefix(root.trim_end_matches('/'))?.strip_prefix('/')?;
        if !rel_ok(rel, form) {
            return None;
        }
        let md = std::fs::symlink_metadata(path).ok()?;
        if !md.is_file() || md.len() > form.file_max() as u64 {
            return None;
        }
        Some(PicSrc { rel: rel.to_string(), size: md.len(), mtime: md.mtime(), form })
    }

    fn key(&self) -> u64 {
        preview_key(self.rel.as_bytes(), self.size, self.mtime)
    }

    /// Its job's path, None when the whole would not fit a job's 600 bytes.
    pub fn job_path(&self) -> Option<String> {
        let p = format!("{}.{}.{}:{}", self.form.letter(), self.size, self.mtime, self.rel);
        (p.len() < 600).then_some(p)
    }

    /// A job's path read strictly: the form's letter, the size and the time as plain decimals, and a path the
    /// form may have.
    fn job_parse(path: &str) -> Option<PicSrc> {
        let (head, rel) = path.split_once(':')?;
        let f: Vec<&str> = head.split('.').collect();
        let [form, size, mtime] = f[..] else { return None };
        let form = PicForm::ALL.into_iter().find(|k| k.letter() == form)?;
        let src = PicSrc { rel: rel.to_string(), size: num_u64(size)?, mtime: num_i64(mtime)?, form };
        rel_ok(rel, form).then_some(src)
    }
}

/// What a `.info` says: the file it is of (its path, size and time), its form, and what came of its picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PicInfo {
    pub size: u64,
    pub mtime: i64,
    pub form: PicForm,
    pub made: Made,
    pub path: String,
}

impl PicInfo {
    /// The file's text: five lines (docs/FORMAT.md).
    pub fn to_text(&self) -> String {
        let made = match self.made {
            Made::Preview(w, h) => format!("made {w} {h}"),
            Made::None(n) => format!("none {}", n.word()),
            Made::Once(n) => format!("once {}", n.word()),
        };
        format!("grimoire-package-picture {INFO_NUMBER}\nof {} {}\nform {}\n{made}\npath {}\n", self.size, self.mtime, self.form.word(), self.path)
    }

    /// The text read strictly: at most INFO_MAX bytes of UTF-8, exactly the five lines in their order, each ended
    /// by a line break, single spaces, decimal numbers without a sign (but a time's minus) or a leading zero, a
    /// picture within the form's side, `once` never `refused`, a path the form may have. And only the very bytes
    /// `to_text` writes for what they say. Anything else is None (the entry is made again).
    pub fn parse(text: &[u8]) -> Option<PicInfo> {
        if text.len() > INFO_MAX {
            return None;
        }
        let whole = std::str::from_utf8(text).ok()?;
        let s = whole.strip_suffix('\n')?;
        let lines: Vec<&str> = s.split('\n').collect();
        let [head, of, form, last, path] = lines[..] else { return None };
        if head != format!("grimoire-package-picture {INFO_NUMBER}") {
            return None;
        }
        let of: Vec<&str> = of.split(' ').collect();
        let ["of", size, mtime] = of[..] else { return None };
        let (size, mtime) = (num_u64(size)?, num_i64(mtime)?);
        let form = form.strip_prefix("form ").and_then(|w| PicForm::ALL.into_iter().find(|k| k.word() == w))?;
        let t: Vec<&str> = last.split(' ').collect();
        let made = match t[..] {
            ["made", w, h] => {
                let (w, h) = (num_u64(w)?, num_u64(h)?);
                if !(1..=form.side_max() as u64).contains(&w) || !(1..=form.side_max() as u64).contains(&h) {
                    return None;
                }
                Made::Preview(w as i32, h as i32)
            }
            ["none", why] => Made::None(NoPreview::from_word(why)?),
            ["once", why] => Made::Once(NoPreview::from_word(why).filter(|n| *n != NoPreview::Refused)?),
            _ => return None,
        };
        let path = path.strip_prefix("path ").filter(|p| rel_ok(p, form))?;
        let info = PicInfo { size, mtime, form, made, path: path.to_string() };
        (info.to_text() == whole).then_some(info)
    }
}

/// The cache folder of a form's pictures.
pub fn picture_cache_dir(form: PicForm) -> String {
    format!("{}/.cache/packages/{}", store_root(), form.cache().0)
}

/// The cache file of `src` with this extension (`pgm`, `info`).
pub fn picture_file(src: &PicSrc, ext: &str) -> String {
    format!("{}/{:016x}.{ext}", picture_cache_dir(src.form), src.key())
}

/// A picture's file in the cache at most: its P5 header and its pixels.
fn pgm_max(form: PicForm) -> usize {
    32 + (form.side_max() as usize) * (form.side_max() as usize)
}

/// What the cache holds for a picture.
pub enum Cached {
    /// nothing to show: not made yet, an entry that does not read (it is made again), or one try on record
    Nothing,
    /// its "no picture" mark: the worker refused the file, or ended on it twice
    None,
    /// its picture, gray
    Picture(Canvas),
}

/// The `.info` of `src`: read with the plain-file read (at most INFO_MAX bytes), parsed strictly, and only when it
/// is of that very file: its path (two paths whose keys meet never share an entry), its size, its time, its form.
pub fn picture_info(src: &PicSrc) -> Option<PicInfo> {
    let info = PicInfo::parse(&read_file_max(&picture_file(src, "info"), INFO_MAX)?)?;
    (info.size == src.size && info.mtime == src.mtime && info.form == src.form && info.path == src.rel).then_some(info)
}

/// What the cache holds for `src`: its `.info`, then its picture, read with the plain-file read and a size limit,
/// the picture's header and length exactly what the `.info` says (w and h within the form's side, by the `.info`'s
/// own rule). A cache folder reached through a link is not read. The UI's read, and the job's own test of "it is
/// there". Nothing here parses a picture format: the pixels are the worker's, checked by the writer.
pub fn picture_cached(src: &PicSrc) -> Cached {
    if !no_link_below(&store_root(), &picture_cache_dir(src.form)) {
        return Cached::Nothing;
    }
    let Some(info) = picture_info(src) else { return Cached::Nothing };
    let (w, h) = match info.made {
        Made::Preview(w, h) => (w, h),
        Made::None(_) => return Cached::None,
        Made::Once(_) => return Cached::Nothing,
    };
    let head = format!("P5\n{w} {h}\n255\n");
    let n = w as usize * h as usize;
    let Some(d) = read_file_max(&picture_file(src, "pgm"), head.len() + n) else { return Cached::Nothing };
    if d.len() != head.len() + n || !d.starts_with(head.as_bytes()) {
        return Cached::Nothing;
    }
    let mut c = Canvas::default();
    if canvas_init(&mut c, w, h) != 0 {
        return Cached::Nothing;
    }
    c.px[..n].copy_from_slice(&d[head.len()..]);
    Cached::Picture(c)
}

/// What `picture_request` did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// its job is queued or running: its path (`bg_job_pending` says when it has ended)
    Job(String),
    /// PICTURES_AT_ONCE are queued or running, or the job queue is full: ask again once one has ended
    Later,
    /// nothing is asked for: a path that does not fit a job, or a session in which a sandbox step already failed
    /// (no worker can be had: nothing is forked for it)
    No,
}

/// Asks for the picture of `src`: one job, queued as work nobody waits on by name, unless that very job is queued
/// or running. A picture is asked for later when the job queue is full: its job never pushes another out of it
/// (bg_queue lets the oldest job of the last rank go, whatever comes in).
pub fn picture_request(src: &PicSrc) -> Ask {
    if crate::sandbox::session_broken() {
        return Ask::No;
    }
    let Some(jp) = src.job_path() else { return Ask::No };
    if bg_job_pending(BgKind::PackagePictures, "", &jp) {
        return Ask::Job(jp);
    }
    if bg_kind_count(BgKind::PackagePictures) >= PICTURES_AT_ONCE {
        return Ask::Later;
    }
    if bg_queue_full() {
        return Ask::Later;
    }
    bg_queue(BgKind::PackagePictures, None, Some(&jp), 0.0, false);
    Ask::Job(jp)
}

/// The cache's entry of `src` removed (its package goes, or is replaced): both files, never through a link.
pub fn picture_forget(src: &PicSrc) {
    if !no_link_below(&store_root(), &picture_cache_dir(src.form)) {
        return;
    }
    for ext in ["pgm", "info"] {
        let _ = std::fs::remove_file(picture_file(src, ext));
    }
}

// ---- the job ----

/// How a picture's job ended: made, marked as having none, one try on record, or already there.
enum Outcome {
    Made,
    Marked(NoPreview),
    Once(NoPreview),
    There,
}

/// Why no picture came from a worker.
enum Fail {
    /// the worker says the file is at fault: the mark at once (the same bytes give the same answer)
    Refused,
    /// the worker ended, lied or ran out of time on it: a try. The first is put on record; the second in a row
    /// is the mark
    Strike(NoPreview),
    /// not the file's: nothing is written, and a later look asks again
    Later(&'static str),
}

/// BgKind::PackagePictures, in the background child: the picture of the file `path` names (the job's path), its
/// "no picture" mark or its first try's record, in the cache; then the cache's folder held to its limits. 0 when the
/// job wrote one of them or found the picture or the mark there, -1 when nothing was written (the log says why).
pub fn bg_package_picture(path: &str) -> i32 {
    let t0 = real_now_ms();
    let r = make(path);
    let ms = real_now_ms().wrapping_sub(t0);
    let rc = match &r {
        Ok((form, Outcome::Made)) => {
            qlog!("package pictures: %s made in %u ms", form.word(), ms);
            0
        }
        Ok((form, Outcome::Marked(why))) => {
            qlog!("package pictures: %s has no picture (%s), marked, in %u ms", form.word(), why.word(), ms);
            0
        }
        Ok((form, Outcome::Once(why))) => {
            qlog!("package pictures: %s gave no picture (%s) in %u ms; one more try at another look", form.word(), why.word(), ms);
            0
        }
        Ok((_, Outcome::There)) => 0,
        Err(why) => {
            qlog!("package pictures: nothing made (%s)", why.as_str());
            -1
        }
    };
    if let Ok((form, Outcome::Made | Outcome::Marked(_) | Outcome::Once(_))) = r {
        let (_, files, bytes) = form.cache();
        let gone = picture_cache_tidy(form, files, bytes);
        if gone > 0 {
            qlog!("package pictures: the cache of %s pruned: %d file(s) removed", form.cache().0, gone as i32);
        }
    }
    rc
}

/// The plain file at `rel` below `root`, opened for reading one part at a time from `root`, never through a link
/// (each folder O_NOFOLLOW, the file O_NOFOLLOW and without waiting), and what fstat says of it.
fn open_below(root: &str, rel: &str) -> Result<(std::fs::File, std::fs::Metadata), &'static str> {
    let c = |s: &str| std::ffi::CString::new(s).map_err(|_| "a path that does not read");
    let dir = c(root)?;
    // SAFETY: open reads the NUL-terminated path we own; the descriptor is owned by the OwnedFd.
    let fd = unsafe { libc::open(dir.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC) };
    if fd < 0 {
        return Err("Grimoire's folder does not open");
    }
    // SAFETY: fd was just opened and is owned by no one else.
    let mut at = unsafe { OwnedFd::from_raw_fd(fd) };
    let parts: Vec<&str> = rel.split('/').collect();
    let Some((name, folders)) = parts.split_last() else { return Err("a path that does not read") };
    for part in folders {
        let part = c(part)?;
        // SAFETY: openat reads the NUL-terminated name we own, in the folder we hold open; the descriptor is owned
        // by the OwnedFd.
        let fd = unsafe { libc::openat(at.as_raw_fd(), part.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
        if fd < 0 {
            return Err("a folder on its way is not there, or is a link");
        }
        // SAFETY: fd was just opened and is owned by no one else.
        at = unsafe { OwnedFd::from_raw_fd(fd) };
    }
    let name = c(name)?;
    // SAFETY: as above, for the file itself.
    let fd = unsafe { libc::openat(at.as_raw_fd(), name.as_ptr(), libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC) };
    if fd < 0 {
        return Err("it is not there, or is a link");
    }
    // SAFETY: fd was just opened and is owned by no one else.
    let f = std::fs::File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    let md = f.metadata().map_err(|_| "it does not stat")?;
    if !md.is_file() {
        return Err("it is no plain file");
    }
    Ok((f, md))
}

/// The bytes free for this user where `dir` is (statvfs's f_bavail), None when it cannot tell.
fn free_bytes(dir: &str) -> Option<u64> {
    let c = std::ffi::CString::new(dir).ok()?;
    // SAFETY: sv is written by statvfs before it is read; c is NUL-terminated.
    let mut sv: libc::statvfs = unsafe { std::mem::zeroed() };
    (unsafe { libc::statvfs(c.as_ptr(), &mut sv) } == 0).then(|| sv.f_bavail as u64 * sv.f_frsize as u64)
}

const NO_WORKER: &str = "no sandboxed worker can be had";

fn make(path: &str) -> Result<(PicForm, Outcome), String> {
    let src = PicSrc::job_parse(path).ok_or("a request that does not read")?;
    let form = src.form;
    let root = store_root();
    let dir = picture_cache_dir(form);
    // (read and written as root: never through a planted link, and the folder made where it is missing)
    if !no_link_below(&root, &dir) || mkdir_p(&dir) != 0 {
        return Err("the cache folder cannot be made".into());
    }
    // What the cache holds for it, by the UI's own test: a mark, or a picture that reads, is there (asked twice,
    // and made by the first); one try on record makes this the second; anything else is made (again).
    let struck = match picture_info(&src).map(|i| i.made) {
        Some(Made::None(_)) => return Ok((form, Outcome::There)),
        Some(Made::Preview(..)) if matches!(picture_cached(&src), Cached::Picture(_)) => return Ok((form, Outcome::There)),
        Some(Made::Once(_)) => true,
        _ => false,
    };
    let (f, md) = open_below(&root, &src.rel)?;
    let same = |md: &std::fs::Metadata| md.len() == src.size && md.mtime() == src.mtime;
    if !same(&md) {
        return Err("it changed since it was asked for".into());
    }
    // (a picture never takes the room the notebooks need: under the free floor none is made)
    if free_bytes(&dir).is_some_and(|free| !crate::fsview::fsview_room_for(free, pgm_max(form) as u64)) {
        return Err("the storage is nearly full".into());
    }
    let r = read_whole(f, &src).and_then(|d| worker_picture(&d, form));
    let write = |made| {
        let info = PicInfo { size: src.size, mtime: src.mtime, form, made, path: src.rel.clone() };
        write_file_atomic(&picture_file(&src, "info"), info.to_text().as_bytes()) == 0
    };
    // (no picture beside a mark or a try's record)
    let no_picture = || {
        let _ = std::fs::remove_file(picture_file(&src, "pgm"));
    };
    let failed = || -> Result<(PicForm, Outcome), String> { Err("the cache could not be written".into()) };
    let mark = |why: NoPreview| {
        no_picture();
        if !write(Made::None(why)) {
            return failed();
        }
        Ok((form, Outcome::Marked(why)))
    };
    match r {
        Ok((w, h, px)) => {
            // (the picture first: a .info that says "made" has one beside it, whole)
            let mut d = format!("P5\n{w} {h}\n255\n").into_bytes();
            if d.try_reserve_exact(px.len()).is_err() {
                return Err("out of memory".into());
            }
            d.extend_from_slice(&px);
            if write_file_atomic(&picture_file(&src, "pgm"), &d) != 0 || !write(Made::Preview(w, h)) {
                return failed();
            }
            Ok((form, Outcome::Made))
        }
        Err(Fail::Later(why)) => Err(why.into()),
        Err(Fail::Strike(why)) if !struck => {
            no_picture();
            if !write(Made::Once(why)) {
                return failed();
            }
            Ok((form, Outcome::Once(why)))
        }
        Err(Fail::Refused) => mark(NoPreview::Refused),
        Err(Fail::Strike(why)) => mark(why),
    }
}

/// The whole file, which is `src`'s size (an empty one has no picture), still that size and time once read.
fn read_whole(mut f: std::fs::File, src: &PicSrc) -> Result<Vec<u8>, Fail> {
    if src.size == 0 || src.size > src.form.file_max() as u64 {
        return Err(Fail::Refused);
    }
    // (reads block again, as a file's do)
    // SAFETY: fcntl on a descriptor this function owns.
    unsafe {
        let fl = libc::fcntl(f.as_raw_fd(), libc::F_GETFL);
        libc::fcntl(f.as_raw_fd(), libc::F_SETFL, fl & !libc::O_NONBLOCK);
    }
    let mut buf: Vec<u8> = Vec::new();
    if buf.try_reserve_exact(src.size as usize + 1).is_err() {
        return Err(Fail::Later("out of memory"));
    }
    if f.by_ref().take(src.size + 1).read_to_end(&mut buf).is_err() || buf.len() as u64 != src.size {
        return Err(Fail::Later("the file did not read whole"));
    }
    if !f.metadata().is_ok_and(|now| now.len() == src.size && now.mtime() == src.mtime) {
        return Err(Fail::Later("it changed while it was read"));
    }
    Ok(buf)
}

/// The picture of `data` by one run of the picture worker (never in the writer, and no second worker), held to a
/// preview's time: its size and its gray pixels, w * h of them, as the pool checked them.
fn worker_picture(data: &[u8], form: PicForm) -> Result<(i32, i32, Vec<u8>), Fail> {
    let mut pool = PicPool::for_preview(mono_ms() + preview_limit_ms(PreviewKind::Picture));
    match pool.small(data, form.what(), form.side_max()) {
        PicOut::Pic { fit, .. } if fit.ch == 1 && fit.px.len() == fit.w as usize * fit.h as usize => Ok((fit.w, fit.h, fit.px)),
        PicOut::Pic { .. } => Err(Fail::Strike(NoPreview::Ended)),
        PicOut::Bad(_) => Err(Fail::Refused),
        PicOut::Dead(why) if why == STALLED => Err(Fail::Strike(NoPreview::Slow)),
        PicOut::Dead(_) => Err(Fail::Strike(NoPreview::Ended)),
        PicOut::Retry(_) | PicOut::Info { .. } => Err(Fail::Later("out of memory")),
        PicOut::NoWorker(_) => Err(Fail::Later(NO_WORKER)),
    }
}

/// A form's cache folder held to `max_files` files and `max_bytes` bytes (the job gives the form's own limits):
/// past either, its oldest entries go (both files of an entry together, by their newest time), down to three
/// quarters of each (`prune_plan`). The names alone first: while the folder cannot be past a limit by the most its
/// files can hold, nothing more is read. Entries that are no plain file are left to themselves (never followed,
/// never counted). How many files went.
pub fn picture_cache_tidy(form: PicForm, max_files: usize, max_bytes: u64) -> usize {
    use std::os::unix::ffi::OsStrExt;
    let dir = &picture_cache_dir(form);
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    let mut names: Vec<std::ffi::OsString> = Vec::new();
    for e in rd.flatten() {
        let n = e.file_name();
        if n.as_bytes().first() == Some(&b'.') || !e.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        // (a folder far past its limit: what is not looked at now is looked at once these have gone)
        if names.len() < 4 * max_files.max(64) {
            names.push(n);
        }
    }
    let most: u64 = names.iter().map(|n| if n.as_bytes().ends_with(b".info") { INFO_MAX } else { pgm_max(form) } as u64).sum();
    if names.len() <= max_files && most <= max_bytes {
        return 0;
    }
    let stem_of = |n: &[u8]| n.split(|&c| c == b'.').next().unwrap_or(&[]).to_vec();
    let mut entries: std::collections::HashMap<Vec<u8>, CacheEntry> = std::collections::HashMap::new();
    for n in &names {
        let Ok(m) = std::fs::symlink_metadata(std::path::Path::new(dir).join(n)) else { continue };
        if !m.is_file() {
            continue;
        }
        let stem = stem_of(n.as_bytes());
        let e = entries.entry(stem.clone()).or_insert_with(|| CacheEntry { stem: String::from_utf8_lossy(&stem).into_owned(), files: 0, bytes: 0, mtime: i64::MIN });
        e.files += 1;
        e.bytes += m.len();
        e.mtime = e.mtime.max(m.mtime());
    }
    let entries: Vec<CacheEntry> = entries.into_values().collect();
    let gone: std::collections::HashSet<String> = prune_plan(&entries, max_files, max_bytes).into_iter().collect();
    let mut files = 0;
    for n in &names {
        if gone.contains(String::from_utf8_lossy(&stem_of(n.as_bytes())).as_ref()) && std::fs::remove_file(std::path::Path::new(dir).join(n)).is_ok() {
            files += 1;
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(form: PicForm, made: Made) -> PicInfo {
        let path = match form {
            PicForm::AppIcon => "packages/apps/com.example.app/icon.ico",
            PicForm::StoreIcon => "package-store/assets/packages/com.example.app/icon.png",
            PicForm::Screenshot => "package-store/assets/packages/com.example.app/screens/1 a.png",
        };
        PicInfo { size: 1234, mtime: 1_700_000_000, form, made, path: path.to_string() }
    }

    // A .info is read back as written, and anything else is refused: each line's word and order, the numbers' form,
    // the picture's bounds by its form, a first try that says "refused", a path outside the form's folder, the
    // length, a control character, a missing or extra line break.
    #[test]
    fn the_info_is_parsed_strictly() {
        for i in [
            info(PicForm::AppIcon, Made::Preview(64, 64)),
            info(PicForm::AppIcon, Made::Preview(256, 1)),
            info(PicForm::StoreIcon, Made::Preview(256, 256)),
            info(PicForm::Screenshot, Made::Preview(1860, 2480)),
            info(PicForm::Screenshot, Made::Preview(2560, 2560)),
            info(PicForm::StoreIcon, Made::None(NoPreview::Refused)),
            info(PicForm::AppIcon, Made::None(NoPreview::Slow)),
            info(PicForm::Screenshot, Made::None(NoPreview::Ended)),
            info(PicForm::StoreIcon, Made::Once(NoPreview::Slow)),
            info(PicForm::AppIcon, Made::Once(NoPreview::Ended)),
        ] {
            assert_eq!(PicInfo::parse(i.to_text().as_bytes()), Some(i));
        }
        let good = "grimoire-package-picture 1\nof 1234 1700000000\nform app-icon\nmade 64 64\npath packages/apps/com.example.app/icon.ico\n";
        assert_eq!(info(PicForm::AppIcon, Made::Preview(64, 64)).to_text(), good);
        assert!(PicInfo::parse(good.as_bytes()).is_some());
        for (from, to) in [
            ("grimoire-package-picture 1", "grimoire-package-picture 2"),
            ("grimoire-package-picture 1", "grimoire-package-picture 01"),
            ("grimoire-package-picture 1", "grimoire-preview 1"),
            ("of 1234 1700000000", "of 01234 1700000000"),
            ("of 1234 1700000000", "of +1234 1700000000"),
            ("of 1234 1700000000", "of 1234 -0"),
            ("of 1234 1700000000", "of 1234  1700000000"),
            ("of 1234 1700000000", "of 99999999999999999999 1700000000"),
            ("form app-icon", "form icon"),
            ("form app-icon", "form screenshot"),
            ("form app-icon", "form picture"),
            ("made 64 64", "made 257 64"),
            ("made 64 64", "made 64 257"),
            ("made 64 64", "made 0 64"),
            ("made 64 64", "made 64"),
            ("made 64 64", "made 64 64 1"),
            ("made 64 64", "none tired"),
            ("made 64 64", "none"),
            ("made 64 64", "once refused"),
            ("path packages/apps/", "path package-store/assets/"),
            ("path packages/apps/", "path /packages/apps/"),
            ("path packages/apps/", "path packages/apps/../apps/"),
            ("path packages/apps/", "path packages/apps//"),
            ("path packages/apps/", "path notebooks/"),
            ("icon.ico\n", "icon.ico"),
            ("icon.ico\n", "icon.ico\n\n"),
            ("icon.ico\n", "icon.ico\nmore\n"),
            ("icon.ico\n", "icon\t.ico\n"),
            ("icon.ico\n", "icon\\.ico\n"),
            ("1700000000\nform", "1700000000\r\nform"),
        ] {
            let bad = good.replace(from, to);
            assert_ne!(bad, good, "{from:?}");
            assert_eq!(PicInfo::parse(bad.as_bytes()), None, "{bad:?}");
        }
        assert_eq!(PicInfo::parse(b""), None);
        // a form's side is its own: a screenshot's size is no icon's
        let shot = info(PicForm::Screenshot, Made::Preview(1860, 2480)).to_text();
        assert!(PicInfo::parse(shot.as_bytes()).is_some());
        assert_eq!(PicInfo::parse(shot.replace("made 1860 2480", "made 2561 2480").as_bytes()), None);
        // too long; bytes that are no text
        let long = good.replace("icon.ico", &format!("{}.ico", "i".repeat(INFO_MAX)));
        assert_eq!(PicInfo::parse(long.as_bytes()), None);
        assert_eq!(PicInfo::parse(b"grimoire-package-picture 1\nof 1 2\nform icon\nmade 3 4\npath package-store/assets/a\xff.png\n"), None);
    }

    // A job's path is the form's letter, the size and the time, then the path from Grimoire's folder, and is read
    // back strictly: only a path in the form's own folder, with no part that climbs or is empty.
    #[test]
    fn a_jobs_path_is_read_strictly() {
        let src = PicSrc { rel: "packages/apps/com.example.app/icon.ico".into(), size: 99678, mtime: 1_700_000_000, form: PicForm::AppIcon };
        let jp = src.job_path().unwrap();
        assert_eq!(jp, "a.99678.1700000000:packages/apps/com.example.app/icon.ico");
        assert_eq!(PicSrc::job_parse(&jp), Some(src));
        let shot = PicSrc { rel: "package-store/assets/packages/x/screens/a:b.png".into(), size: 5, mtime: -3, form: PicForm::Screenshot };
        assert_eq!(PicSrc::job_parse(&shot.job_path().unwrap()), Some(shot));
        for bad in [
            "",
            "a.1.2",
            "a.1.2:",
            "x.1.2:packages/apps/a/icon.ico",
            "a.1:packages/apps/a/icon.ico",
            "a.1.2.3:packages/apps/a/icon.ico",
            "a.01.2:packages/apps/a/icon.ico",
            "a.1.+2:packages/apps/a/icon.ico",
            "a.1.2:package-store/assets/a/icon.ico",
            "i.1.2:packages/apps/a/icon.ico",
            "s.1.2:packages/apps/a/icon.ico",
            "a.1.2:packages/apps/",
            "a.1.2:packages/apps/../../settings.ini",
            "a.1.2:packages/apps/a//icon.ico",
            "a.1.2:packages/apps/a/./icon.ico",
            "a.1.2:/packages/apps/a/icon.ico",
            "a.1.2:notebooks/a/p1.qpg",
            "i.1.2:package-store/index.json",
            "i.1.2:package-store/assets/a\nb.png",
        ] {
            assert_eq!(PicSrc::job_parse(bad), None, "{bad:?}");
        }
        // the longest path a picture may have fits a job
        let long = PicSrc { rel: format!("package-store/assets/{}", "k".repeat(REL_MAX - 21)), size: 9_999_999_999_999_999_999, mtime: i64::MIN + 1, form: PicForm::Screenshot };
        assert!(rel_ok(&long.rel, long.form) && long.job_path().is_some_and(|p| p.len() < 600 && PicSrc::job_parse(&p) == Some(long.clone())));
        assert!(!rel_ok(&format!("package-store/assets/{}", "k".repeat(REL_MAX - 20)), PicForm::Screenshot));
    }

    // A picture is asked for later while the job queue is full, and pushes no job out of it: sixteen exports wait
    // (the queue's size) behind one that runs, a picture asked for then is not queued and every export still
    // waits; with one place free it is queued, and asked for again it is that same job. Before, the request went
    // to the full queue, which let its oldest export go for it. (No child runs: bgjob.rs bg_test_hold.)
    #[test]
    fn a_picture_waits_for_room_in_a_full_queue() {
        let src = PicSrc { rel: "packages/apps/com.example.app/icon.ico".into(), size: 99678, mtime: 1_700_000_000, form: PicForm::AppIcon };
        let jp = src.job_path().unwrap();
        crate::bgjob::bg_test_hold(true);
        let export = |k: usize| format!("full-queue-{k:02}");
        for k in 0..16 {
            bg_queue(BgKind::MirrorPdf, Some(&export(k)), Some("/nowhere/PDFs"), 150.0, false);
        }
        let waiting = || (0..16).filter(|&k| bg_job_pending(BgKind::MirrorPdf, &export(k), "/nowhere/PDFs")).count();
        assert!(bg_queue_full() && waiting() == 16);
        assert_eq!(picture_request(&src), Ask::Later);
        assert_eq!(picture_request(&src), Ask::Later);
        assert!(waiting() == 16 && !bg_job_pending(BgKind::PackagePictures, "", &jp) && bg_kind_count(BgKind::PackagePictures) == 0);
        // one export leaves the queue: there is room for the picture's job
        assert!(bg_unqueue(BgKind::MirrorPdf, &export(3), "/nowhere/PDFs") && !bg_queue_full());
        assert_eq!(picture_request(&src), Ask::Job(jp.clone()));
        assert!(waiting() == 15 && bg_job_pending(BgKind::PackagePictures, "", &jp) && bg_queue_full());
        // (asked again while it is queued, the queue full again: the same job, and none pushed out)
        assert_eq!(picture_request(&src), Ask::Job(jp.clone()));
        assert!(waiting() == 15 && bg_kind_count(BgKind::PackagePictures) == 1);
        // another picture: later
        let other = PicSrc { rel: "packages/apps/com.example.other/icon.ico".into(), ..src.clone() };
        assert_eq!(picture_request(&other), Ask::Later);
        assert!(waiting() == 15 && bg_kind_count(BgKind::PackagePictures) == 1);
        crate::bgjob::bg_test_hold(false);
        assert!(!bg_busy());
    }

    // The forms' figures: an icon's side is the `.ico` reader's, its file's limit the bundled icon's; a Scribe page
    // is within a screenshot's side; and each cache folder's limits hold at least a few of its largest pictures.
    #[test]
    fn the_forms_figures() {
        assert_eq!((ICON_SIDE as usize, PicForm::AppIcon.file_max()), (crate::picwork::ICO_SIDE_MAX, crate::apppkg::APP_ICON_MAX));
        assert_eq!(PicForm::StoreIcon.file_max(), crate::apppkg::APP_ICON_MAX);
        assert!(SHOT_SIDE >= 2480 && SHOT_SIDE as usize <= crate::picwork::SMALL_SIDE_MAX);
        for form in PicForm::ALL {
            let (_, files, bytes) = form.cache();
            assert!(files >= 16 && bytes >= 4 * pgm_max(form) as u64, "{form:?}");
        }
        assert_eq!(PicForm::ALL.map(|f| f.letter()), ["a", "i", "s"]);
    }
}
