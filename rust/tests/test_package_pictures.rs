// test_package_pictures.rs: packages' pictures (2.3, issue #236; src/pkgpics.rs), the job as the background child
// runs it, on a data folder of the host's own. Every picture goes through a real worker, a fresh exec of the headless
// runner (GRIMOIRE_WORK_EXE), as in the app: an installed app's bundled `.ico`, a Package Store icon (an `.ico` with
// a PNG entry, a PNG) and screenshots (a PNG, a JPEG). A file the worker refuses is marked and not tried again; one
// whose worker ends or runs out of time gets a try on record first and the mark at the second; a file changed since
// it was asked for gets nothing until it is asked for again; a file reached through a link, or outside its form's
// folder, is never read; with no worker to be had nothing is written and nothing is decoded in the writer. The
// cache: what the UI takes of an entry (the `.info` strictly, the picture's header and exactly w * h bytes, never
// through a link), an entry that does not read made again, the prune, a few jobs at a time, an app's entry gone with
// its app.
#![cfg(feature = "sim")]
mod testlib;
use grimoire::bgwork_previews::{Made, NoPreview};
use grimoire::pkgpics::*;
use grimoire::prelude::*;
use testlib::*;

const HOOKS: [&str; 8] = [
    "GRIMOIRE_PIC_TEST_HANG",
    "GRIMOIRE_PIC_TEST_CRASH",
    "GRIMOIRE_PIC_TEST_TRAP",
    "GRIMOIRE_PIC_WATCHDOG_MS",
    "GRIMOIRE_PREVIEW_MS",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_SANDBOX",
    "GRIMOIRE_BG_DELAY_MS",
];

const ICO: &[u8] = include_bytes!("../../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico");
const SHOT: &[u8] = include_bytes!("../../tests/fixtures/package-store/package-store/assets/packages/com.example.fixture/screens/01-app.png");
const JPEG: &[u8] = include_bytes!("../../tests/fixtures/photos/IMG_0001.jpg");
const APP_ICON: &str = "packages/apps/com.example.fixture/icon.ico";

fn clean_env() {
    for h in HOOKS {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
}

/// Grimoire's folder made afresh (its store set up there).
fn root() -> String {
    let g = format!("{}/pp-root", tl_tmp());
    let _ = std::fs::remove_dir_all(&g);
    std::fs::create_dir_all(&g).unwrap();
    assert_eq!(store_init(&g), 0);
    g
}

/// The file `rel` of Grimoire's folder written, and its path.
fn put(g: &str, rel: &str, data: &[u8]) -> String {
    let path = format!("{g}/{rel}");
    std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).unwrap();
    std::fs::write(&path, data).unwrap();
    path
}

fn png(w: i32, h: i32) -> Vec<u8> {
    let dir = format!("{}/pp-png", tl_tmp());
    std::fs::create_dir_all(&dir).unwrap();
    let path = format!("{dir}/gen.png");
    let px: Vec<u8> = (0..w * h).map(|i| if (i % w) < w / 2 { 30 } else { 220 }).collect();
    assert_eq!(bgimage_write_png(&path, &BgPicture { px, w, h, ch: 1 }), 0);
    std::fs::read(&path).unwrap()
}

/// An `.ico` whose one entry is this PNG.
fn ico_of_png(w: u8, h: u8, png: &[u8]) -> Vec<u8> {
    let mut out = vec![0, 0, 1, 0, 1, 0, w, h, 0, 0, 1, 0, 32, 0];
    out.extend_from_slice(&(png.len() as u32).to_le_bytes());
    out.extend_from_slice(&22u32.to_le_bytes());
    out.extend_from_slice(png);
    out
}

fn src(path: &str, form: PicForm) -> PicSrc {
    PicSrc::of(path, form).unwrap_or_else(|| panic!("{path}"))
}

fn job(s: &PicSrc) -> i32 {
    bg_package_picture(&s.job_path().unwrap())
}

fn picture(s: &PicSrc) -> Option<Canvas> {
    match picture_cached(s) {
        Cached::Picture(c) => Some(c),
        _ => None,
    }
}

fn made(s: &PicSrc) -> Option<Made> {
    picture_info(s).map(|i| i.made)
}

/// The cache's files of a form, by name.
fn cache_names(form: PicForm) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(picture_cache_dir(form)).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    v.sort();
    v
}

fn set_mtime(path: &str, mtime: i64) -> bool {
    let t = std::ffi::CString::new(path).unwrap();
    let times = [libc::timeval { tv_sec: mtime as libc::time_t, tv_usec: 0 }; 2];
    // SAFETY: utimes reads the path and the two times we own.
    unsafe { libc::utimes(t.as_ptr(), times.as_ptr()) == 0 }
}

/// Runs `f` with this process's log (stderr) kept in a file: what it returned, and the log's text.
fn with_log<R>(name: &str, f: impl FnOnce() -> R) -> (R, String) {
    use std::io::Write;
    let path = format!("{}/pp-{name}.log", tl_tmp());
    let _ = std::io::stderr().flush();
    // SAFETY: dup, open, dup2 and close on descriptors this test owns.
    let saved = unsafe {
        let saved = libc::dup(2);
        let c = std::ffi::CString::new(path.as_str()).unwrap();
        let fd = libc::open(c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        libc::dup2(fd, 2);
        libc::close(fd);
        saved
    };
    let r = f();
    let _ = std::io::stderr().flush();
    // SAFETY: dup2 and close on the descriptor saved above.
    unsafe {
        libc::dup2(saved, 2);
        libc::close(saved);
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    eprint!("{text}");
    (r, text)
}

fn settle() {
    let t0 = grimoire::util::real_now_ms();
    while bg_busy() && grimoire::util::real_now_ms().wrapping_sub(t0) < 60_000 {
        bg_poll();
        sleep_ms(5);
    }
    assert!(!bg_busy());
}

// An installed app's bundled icon, a Store icon (an `.ico` with a PNG entry, and a PNG) and screenshots (a PNG, a
// JPEG), each made by its worker: a gray picture in the form's cache folder under the file's key, with its .info;
// asked again, it is there and no worker runs.
fn an_apps_icon_a_store_icon_and_screenshots_are_made() {
    clean_env();
    let g = root();
    let store = "package-store/assets/packages/com.example.fixture";
    let cases = [
        (APP_ICON.to_string(), PicForm::AppIcon, ICO.to_vec(), "app-icon", (64, 64)),
        (format!("{store}/icon.ico"), PicForm::StoreIcon, ICO.to_vec(), "icon", (64, 64)),
        (format!("{store}/icon-png.ico"), PicForm::StoreIcon, ico_of_png(48, 40, &png(48, 40)), "icon", (48, 40)),
        (format!("{store}/icon.png"), PicForm::StoreIcon, png(120, 90), "icon", (120, 90)),
        (format!("{store}/big.png"), PicForm::StoreIcon, png(512, 384), "icon", (256, 192)),
        (format!("{store}/screens/01-app.png"), PicForm::Screenshot, SHOT.to_vec(), "screenshot", (1860, 2480)),
        (format!("{store}/screens/02.jpg"), PicForm::Screenshot, JPEG.to_vec(), "screenshot", (400, 300)),
    ];
    for (rel, form, data, word, (w, h)) in &cases {
        let s = src(&put(&g, rel, data), *form);
        check!(s.rel == *rel && s.size == data.len() as u64);
        check!(matches!(picture_cached(&s), Cached::Nothing));
        check!(job(&s) == 0);
        let c = picture(&s);
        check!(c.as_ref().is_some_and(|c| (c.w, c.h) == (*w, *h) && c.px.iter().any(|&v| v < 128) && c.px.iter().any(|&v| v > 128)));
        let info = std::fs::read_to_string(picture_file(&s, "info")).unwrap_or_default();
        check!(info == format!("grimoire-package-picture 1\nof {} {}\nform {word}\nmade {w} {h}\npath {rel}\n", s.size, s.mtime));
        check!(made(&s) == Some(Made::Preview(*w, *h)));
        // the picture's file: its header and exactly w * h bytes
        let pgm = std::fs::read(picture_file(&s, "pgm")).unwrap_or_default();
        let head = format!("P5\n{w} {h}\n255\n");
        check!(pgm.starts_with(head.as_bytes()) && pgm.len() == head.len() + (*w * *h) as usize);
        // asked again (a second look's job queued before the first ended): it is there, and no worker is needed
        std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
        check!(job(&s) == 0);
        std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    }
    // the PNG's left half is dark and its right half light: so is its picture
    let s = src(&format!("{g}/{store}/icon.png"), PicForm::StoreIcon);
    let c = picture(&s).unwrap();
    let at = |x: i32, y: i32| c.px[(y * c.stride + x) as usize];
    check!(at(20, 45) < 80 && at(100, 45) > 170);
    // icons and screenshots have a folder each
    check!(cache_names(PicForm::AppIcon).len() == 10 && cache_names(PicForm::Screenshot).len() == 4);
    check!(picture_cache_dir(PicForm::AppIcon) == format!("{g}/.cache/packages/icons") && picture_cache_dir(PicForm::StoreIcon) == picture_cache_dir(PicForm::AppIcon));
    check!(picture_cache_dir(PicForm::Screenshot) == format!("{g}/.cache/packages/shots"));
    // a request that does not read: nothing
    check!(bg_package_picture("not a request") == -1);
    check!(bg_package_picture("a.1.2:notebooks/x/p1.qpg") == -1);
    check!(cache_names(PicForm::AppIcon).len() == 10);
}

// A file the worker refuses gets a "no picture" mark at once (its .info, no picture), kept: the next job for it finds
// the mark and runs no worker. Files cut short, a picture that is no `.ico` where an app's icon is asked for, an
// `.ico` where a screenshot is, sides over the limits, an empty file.
fn a_refused_file_is_marked_and_not_tried_again() {
    clean_env();
    let g = root();
    let store = "package-store/assets/packages/x";
    let good = png(120, 90);
    let mut wide_header = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n', 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
    wide_header.extend_from_slice(&9000u32.to_be_bytes());
    wide_header.extend_from_slice(&9000u32.to_be_bytes());
    wide_header.extend_from_slice(&[8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b'I', b'D', b'A', b'T', 0, 0, 0, 0]);
    let cases = [
        ("packages/apps/a/cut.ico".to_string(), PicForm::AppIcon, ICO[..100].to_vec()),
        ("packages/apps/a/png.ico".to_string(), PicForm::AppIcon, good.clone()),
        ("packages/apps/a/big.ico".to_string(), PicForm::AppIcon, ico_of_png(0, 0, &png(300, 300))),
        ("packages/apps/a/empty.ico".to_string(), PicForm::AppIcon, Vec::new()),
        (format!("{store}/cut.png"), PicForm::StoreIcon, good[..good.len() / 2].to_vec()),
        (format!("{store}/text.png"), PicForm::StoreIcon, b"no picture at all".to_vec()),
        (format!("{store}/screens/icon.ico"), PicForm::Screenshot, ICO.to_vec()),
        (format!("{store}/screens/wide.png"), PicForm::Screenshot, wide_header),
    ];
    for (rel, form, data) in &cases {
        let s = src(&put(&g, rel, data), *form);
        let (rc, log) = with_log("refused", || job(&s));
        check!(rc == 0 && log.contains("has no picture (refused), marked"));
        check!(!log.contains(rel.rsplit('/').next().unwrap())); // (never a name in the log)
        check!(matches!(picture_cached(&s), Cached::None));
        check!(made(&s) == Some(Made::None(NoPreview::Refused)));
        check!(!std::path::Path::new(&picture_file(&s, "pgm")).exists());
        // the mark is kept: the next job runs no worker
        std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
        check!(job(&s) == 0 && matches!(picture_cached(&s), Cached::None));
        std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    }
    // a file over its form's size is not asked for at all (nothing is read of it)
    let mut long = ICO.to_vec();
    long.resize(256 * 1024 + 1, 0);
    check!(PicSrc::of(&put(&g, "packages/apps/a/long.ico", &long), PicForm::AppIcon).is_none());
    long.truncate(256 * 1024);
    check!(PicSrc::of(&put(&g, "packages/apps/a/long.ico", &long), PicForm::AppIcon).is_some());
}

// A worker that ends on a file, or runs out of the picture's time: a try on record the first time (no mark, no
// picture), and the mark the second time in a row; by one worker run a job.
fn a_worker_that_ends_or_takes_too_long_is_a_try_and_then_the_mark() {
    clean_env();
    let g = root();
    for (hook, ms, why, word) in [("GRIMOIRE_PIC_TEST_CRASH", "", NoPreview::Ended, "ended"), ("GRIMOIRE_PIC_TEST_HANG", "1500", NoPreview::Slow, "slow")] {
        let s = src(&put(&g, &format!("packages/apps/{word}/icon.ico"), ICO), PicForm::AppIcon);
        std::env::set_var(hook, "1");
        if !ms.is_empty() {
            std::env::set_var("GRIMOIRE_PREVIEW_MS", ms);
        }
        let t0 = std::time::Instant::now();
        let (rc, log) = with_log("strike", || job(&s));
        check!(rc == 0 && log.contains(&format!("gave no picture ({word})")) && log.contains("one more try at another look"));
        check!(t0.elapsed().as_millis() < 8_000);
        check!(made(&s) == Some(Made::Once(why)) && matches!(picture_cached(&s), Cached::Nothing));
        let (rc, log) = with_log("strike2", || job(&s));
        check!(rc == 0 && log.contains(&format!("has no picture ({word}), marked")));
        check!(made(&s) == Some(Made::None(why)) && matches!(picture_cached(&s), Cached::None));
        check!(!std::path::Path::new(&picture_file(&s, "pgm")).exists());
        clean_env();
        // a try on record and then a good run: the picture, and no mark
        let s = src(&put(&g, &format!("packages/apps/{word}-then-good/icon.ico"), ICO), PicForm::AppIcon);
        std::env::set_var(hook, "1");
        if !ms.is_empty() {
            std::env::set_var("GRIMOIRE_PREVIEW_MS", ms);
        }
        check!(job(&s) == 0 && made(&s) == Some(Made::Once(why)));
        clean_env();
        check!(job(&s) == 0 && made(&s) == Some(Made::Preview(64, 64)) && picture(&s).is_some());
    }
    check!(!grimoire::picwork::in_fallback());
}

// A file changed since it was asked for (its size or its time): nothing is made for the old request, and the file
// asked for again has an entry of its own. A file that is gone: nothing.
fn a_changed_file_gets_nothing_until_it_is_asked_for_again() {
    clean_env();
    let g = root();
    let path = put(&g, APP_ICON, ICO);
    let old = src(&path, PicForm::AppIcon);
    check!(job(&old) == 0 && picture(&old).is_some());
    // written over with another icon (another size): the old request makes nothing
    let other = ico_of_png(48, 40, &png(48, 40));
    put(&g, APP_ICON, &other);
    check!(set_mtime(&path, old.mtime + 10));
    let stale = old.clone();
    std::fs::remove_file(picture_file(&old, "pgm")).unwrap();
    std::fs::remove_file(picture_file(&old, "info")).unwrap();
    let (rc, log) = with_log("changed", || job(&stale));
    check!(rc == -1 && log.contains("nothing made (it changed since it was asked for)"));
    check!(cache_names(PicForm::AppIcon).is_empty());
    let new = src(&path, PicForm::AppIcon);
    check!(new != old && picture_file(&new, "pgm") != picture_file(&old, "pgm"));
    check!(job(&new) == 0 && picture(&new).is_some_and(|c| (c.w, c.h) == (48, 40)));
    // the same size, another time: another entry too
    check!(set_mtime(&path, old.mtime + 20));
    let later = src(&path, PicForm::AppIcon);
    check!(later.size == new.size && later != new && matches!(picture_cached(&later), Cached::Nothing));
    check!(job(&later) == 0 && picture(&later).is_some());
    // gone: nothing to ask for, and a request made before it went makes nothing
    std::fs::remove_file(&path).unwrap();
    check!(PicSrc::of(&path, PicForm::AppIcon).is_none());
    let before = PicSrc { mtime: later.mtime + 5, ..later.clone() };
    let (rc, log) = with_log("gone", || job(&before));
    check!(rc == -1 && log.contains("nothing made (it is not there, or is a link)"));
}

// The job reads only a plain file of its form's own folder, reached from Grimoire's folder with no link on the way:
// a link at the file's name, a folder that is a link, and a path outside the folder are never read, whatever they
// lead to. The UI's own look (`PicSrc::of`) refuses the same.
fn a_file_through_a_link_or_outside_its_folder_is_never_read() {
    clean_env();
    let g = root();
    let elsewhere = format!("{}/pp-elsewhere", tl_tmp());
    let _ = std::fs::remove_dir_all(&elsewhere);
    std::fs::create_dir_all(format!("{elsewhere}/app")).unwrap();
    std::fs::write(format!("{elsewhere}/app/icon.ico"), ICO).unwrap();
    let md = std::fs::metadata(format!("{elsewhere}/app/icon.ico")).unwrap();
    use std::os::unix::fs::MetadataExt;
    // a link at the name
    std::fs::create_dir_all(format!("{g}/packages/apps/linked")).unwrap();
    std::os::unix::fs::symlink(format!("{elsewhere}/app/icon.ico"), format!("{g}/packages/apps/linked/icon.ico")).unwrap();
    check!(PicSrc::of(&format!("{g}/packages/apps/linked/icon.ico"), PicForm::AppIcon).is_none());
    let named = PicSrc { rel: "packages/apps/linked/icon.ico".into(), size: md.len(), mtime: md.mtime(), form: PicForm::AppIcon };
    let (rc, log) = with_log("link", || job(&named));
    check!(rc == -1 && log.contains("nothing made (it is not there, or is a link)"));
    // a folder on the way that is a link
    std::os::unix::fs::symlink(format!("{elsewhere}/app"), format!("{g}/packages/apps/folder")).unwrap();
    let named = PicSrc { rel: "packages/apps/folder/icon.ico".into(), ..named };
    let (rc, log) = with_log("link2", || job(&named));
    check!(rc == -1 && log.contains("nothing made (a folder on its way is not there, or is a link)"));
    check!(cache_names(PicForm::AppIcon).is_empty());
    // outside the form's folder: the UI asks for nothing, and a job's path that names it does not read
    let page = put(&g, "notebooks/n/icon.ico", ICO);
    check!(PicSrc::of(&page, PicForm::AppIcon).is_none() && PicSrc::of(&page, PicForm::StoreIcon).is_none());
    check!(PicSrc::of(&put(&g, "package-store/index.ico", ICO), PicForm::StoreIcon).is_none());
    check!(PicSrc::of(&put(&g, "package-store/assets/a.png", &png(20, 20)), PicForm::AppIcon).is_none());
    check!(PicSrc::of(&format!("{elsewhere}/app/icon.ico"), PicForm::AppIcon).is_none());
    check!(bg_package_picture(&format!("a.{}.{}:notebooks/n/icon.ico", md.len(), md.mtime())) == -1);
    check!(bg_package_picture(&format!("a.{}.{}:packages/apps/../../notebooks/n/icon.ico", md.len(), md.mtime())) == -1);
    check!(cache_names(PicForm::AppIcon).is_empty());
    // the cache folder itself a link: nothing is read through it and nothing written into it
    let s = src(&put(&g, APP_ICON, ICO), PicForm::AppIcon);
    check!(job(&s) == 0 && picture(&s).is_some());
    let kept = format!("{elsewhere}/icons");
    std::fs::rename(picture_cache_dir(PicForm::AppIcon), &kept).unwrap();
    std::os::unix::fs::symlink(&kept, picture_cache_dir(PicForm::AppIcon)).unwrap();
    check!(matches!(picture_cached(&s), Cached::Nothing));
    let other = src(&put(&g, "packages/apps/other/icon.ico", ICO), PicForm::AppIcon);
    let (rc, log) = with_log("cachelink", || job(&other));
    check!(rc == -1 && log.contains("nothing made (the cache folder cannot be made)"));
    check!(std::fs::read_dir(&kept).unwrap().count() == 2);
    picture_forget(&s);
    check!(std::fs::read_dir(&kept).unwrap().count() == 2);
    std::fs::remove_file(picture_cache_dir(PicForm::AppIcon)).unwrap();
}

// What the UI takes of the cache: an entry whose `.info` is exactly the text for the file asked for, and a picture
// that is its header and exactly w * h bytes, each a plain file. Anything else is nothing there, and the next job
// makes the entry again.
fn a_cache_entry_that_does_not_read_is_nothing_and_is_made_again() {
    clean_env();
    let g = root();
    let s = src(&put(&g, APP_ICON, ICO), PicForm::AppIcon);
    check!(job(&s) == 0);
    let (pgm, info) = (picture_file(&s, "pgm"), picture_file(&s, "info"));
    let (good_pgm, good_info) = (std::fs::read(&pgm).unwrap(), std::fs::read_to_string(&info).unwrap());
    let head = "P5\n64 64\n255\n";
    check!(good_pgm.len() == head.len() + 64 * 64);
    let again = |what: &str| {
        check!(matches!(picture_cached(&s), Cached::Nothing));
        let ok = job(&s) == 0 && picture(&s).is_some_and(|c| (c.w, c.h) == (64, 64));
        if !ok {
            eprintln!("  not made again after: {what}");
        }
        check!(ok);
        check!(std::fs::read(&pgm).unwrap() == good_pgm && std::fs::read_to_string(&info).unwrap() == good_info);
    };
    // a picture of a wrong length: a byte short, a byte long, cut to its header, empty
    for len in [good_pgm.len() - 1, head.len(), 0] {
        std::fs::write(&pgm, &good_pgm[..len]).unwrap();
        again("a picture cut short");
    }
    let mut long = good_pgm.clone();
    long.push(0);
    std::fs::write(&pgm, &long).unwrap();
    again("a picture a byte long");
    // a header that is not the `.info`'s: another size with the same count of bytes, another maximum, a comment
    for bad in ["P5\n32 128\n255", "P5\n64 64\n254\n", "P2\n64 64\n255\n", "P5 64 64\n255\n\n"] {
        let mut d = bad.as_bytes().to_vec();
        d.extend_from_slice(&good_pgm[head.len()..]);
        std::fs::write(&pgm, &d).unwrap();
        again(bad);
    }
    // the picture gone
    std::fs::remove_file(&pgm).unwrap();
    again("no picture beside the .info");
    // an `.info` that is not exactly this file's: a side over the form's, another file's path, size, time or form,
    // another number, a line more
    let size = format!("of {} ", s.size);
    for (from, to) in [
        ("made 64 64", "made 257 64"),
        ("made 64 64", "made 64 65"),
        ("com.example.fixture", "com.example.other"),
        (size.as_str(), "of 1 "),
        ("form app-icon", "form icon"),
        ("grimoire-package-picture 1", "grimoire-package-picture 2"),
        ("icon.ico\n", "icon.ico\nmore\n"),
    ] {
        let bad = good_info.replace(from, to);
        check!(bad != good_info);
        std::fs::write(&info, &bad).unwrap();
        again(to);
    }
    // a `.info` that says a larger picture than the file beside it holds: nothing (never bytes past the file's end)
    std::fs::write(&info, good_info.replace("made 64 64", "made 128 64")).unwrap();
    again("an .info of a larger picture");
    // a link at the picture's name, to a picture that would read: not followed
    let elsewhere = format!("{}/pp-elsewhere-pgm", tl_tmp());
    std::fs::write(&elsewhere, &good_pgm).unwrap();
    std::fs::remove_file(&pgm).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &pgm).unwrap();
    check!(matches!(picture_cached(&s), Cached::Nothing));
    std::fs::remove_file(&pgm).unwrap();
    again("a link at the picture's name");
    // and at the `.info`'s
    let elsewhere = format!("{}/pp-elsewhere-info", tl_tmp());
    std::fs::write(&elsewhere, &good_info).unwrap();
    std::fs::remove_file(&info).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &info).unwrap();
    check!(matches!(picture_cached(&s), Cached::Nothing));
    std::fs::remove_file(&info).unwrap();
    again("a link at the .info's name");
}

// The cache's bounds: past a folder's limit of files or of bytes its oldest entries go, both files of an entry
// together, down to three quarters; within the limits nothing goes. The job prunes its form's folder after it wrote.
fn the_cache_is_pruned_oldest_first() {
    clean_env();
    let g = root();
    let mut made: Vec<PicSrc> = Vec::new();
    for k in 0..6 {
        let s = src(&put(&g, &format!("package-store/assets/p{k}/icon.png"), &png(100 + k, 80)), PicForm::StoreIcon);
        check!(job(&s) == 0);
        // (the entries' times a minute apart, the first the oldest)
        for ext in ["pgm", "info"] {
            check!(set_mtime(&picture_file(&s, ext), 1_700_000_000 + 60 * k as i64));
        }
        made.push(s);
    }
    check!(cache_names(PicForm::StoreIcon).len() == 12);
    // within both limits: nothing goes (the form's own limits are far)
    check!(picture_cache_tidy(PicForm::StoreIcon, 12, u64::MAX) == 0 && cache_names(PicForm::StoreIcon).len() == 12);
    // past the files: 12 > 8, down to 6 files: the three oldest entries go, each whole
    check!(picture_cache_tidy(PicForm::StoreIcon, 8, u64::MAX) == 6);
    check!(cache_names(PicForm::StoreIcon).len() == 6);
    check!(made[..3].iter().all(|s| matches!(picture_cached(s), Cached::Nothing) && !std::path::Path::new(&picture_file(s, "info")).exists()));
    check!(made[3..].iter().all(|s| picture(s).is_some()));
    // past the bytes: the three left hold about 8.4 kB each, 25 kB; a limit of 24 kB takes the oldest (down to 18 kB)
    check!(picture_cache_tidy(PicForm::StoreIcon, 100, 24_000) == 2);
    check!(matches!(picture_cached(&made[3]), Cached::Nothing) && picture(&made[4]).is_some() && picture(&made[5]).is_some());
    // a folder or a link in the cache folder is left to itself
    std::fs::create_dir_all(format!("{}/kept", picture_cache_dir(PicForm::StoreIcon))).unwrap();
    std::os::unix::fs::symlink(format!("{g}/settings.ini"), format!("{}/link.pgm", picture_cache_dir(PicForm::StoreIcon))).unwrap();
    check!(picture_cache_tidy(PicForm::StoreIcon, 0, 0) == 4);
    check!(cache_names(PicForm::StoreIcon) == ["kept", "link.pgm"]);
    // screenshots have their own folder, which an icon's job never prunes
    let shot = src(&put(&g, "package-store/assets/p0/screens/1.jpg", JPEG), PicForm::Screenshot);
    check!(job(&shot) == 0 && picture(&shot).is_some());
    check!(picture_cache_tidy(PicForm::StoreIcon, 0, 0) == 0 && picture(&shot).is_some());
}

// The request (UI thread): a job of its own for a file the cache has nothing for, the same job when asked twice, a
// few at a time (the rest are asked for again once these have ended), run by a background child.
fn the_request_queues_a_few_jobs_at_a_time() {
    clean_env();
    let g = root();
    check!(PICTURES_AT_ONCE == 4);
    let all: Vec<PicSrc> = (0..6).map(|k| src(&put(&g, &format!("packages/apps/app{k}/icon.ico"), ICO), PicForm::AppIcon)).collect();
    let asked: Vec<Ask> = all.iter().map(picture_request).collect();
    check!(asked[..4].iter().zip(&all).all(|(a, s)| *a == Ask::Job(s.job_path().unwrap())));
    check!(asked[4..].iter().all(|a| *a == Ask::Later));
    // asked again while it is queued or running: the same job, none more
    check!(picture_request(&all[0]) == asked[0] && picture_request(&all[3]) == asked[3] && picture_request(&all[5]) == Ask::Later);
    check!(bg_kind_count(BgKind::PackagePictures) == 4);
    settle();
    check!(all[..4].iter().all(|s| picture(s).is_some()) && all[4..].iter().all(|s| matches!(picture_cached(s), Cached::Nothing)));
    check!(asked[..4].iter().all(|a| matches!(a, Ask::Job(j) if !bg_job_pending(BgKind::PackagePictures, "", j))));
    // the rest, asked for once those have ended
    check!(all[4..].iter().all(|s| matches!(picture_request(s), Ask::Job(_))));
    settle();
    check!(all.iter().all(|s| picture(s).is_some_and(|c| (c.w, c.h) == (64, 64))));
    check!(bg_kind_count(BgKind::PackagePictures) == 0);
}

// No worker to be had (none starts, or a sandbox step fails): nothing is written, no mark and no try, and nothing is
// decoded in the writer; a later look asks again. A sandbox step refused to a picture's worker, in the job's own
// child as the app runs it: the child's exit status marks this process's session broken too, the pictures still
// queued are dropped, and nothing more is asked for. (Last: it leaves this process without picture workers.)
fn no_worker_writes_nothing() {
    clean_env();
    let g = root();
    let cases = [
        (APP_ICON.to_string(), PicForm::AppIcon, ICO.to_vec()),
        ("package-store/assets/p/icon.png".to_string(), PicForm::StoreIcon, png(120, 90)),
        ("package-store/assets/p/screens/1.jpg".to_string(), PicForm::Screenshot, JPEG.to_vec()),
    ];
    let all: Vec<PicSrc> = cases.iter().map(|(rel, form, data)| src(&put(&g, rel, data), *form)).collect();
    // none starts (in the jobs' own children, as in the app: this process keeps its workers for the next part)
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    let (asked, log) = with_log("noworker", || {
        let asked: Vec<Ask> = all.iter().map(picture_request).collect();
        settle();
        asked
    });
    check!(asked.iter().all(|a| matches!(a, Ask::Job(_))));
    check!(log.matches("nothing made (no sandboxed worker can be had)").count() == 3);
    check!(!log.contains("pictures decode in the writer"));
    check!(all.iter().all(|s| matches!(picture_cached(s), Cached::Nothing)));
    check!(cache_names(PicForm::AppIcon).is_empty() && cache_names(PicForm::Screenshot).is_empty());
    check!(!grimoire::picwork::in_fallback() && !grimoire::sandbox::session_broken());
    // a sandbox step fails (the worker says so before it reads a byte), in the jobs' own children: the three are
    // asked for and queued; the first one's child ends with the status that says so, this process's session is
    // broken by it, and the two still queued never run
    clean_env();
    std::env::set_var("GRIMOIRE_WORK_FAIL_STEP", "seccomp");
    check!(CODE_SANDBOX == 4 && CODE_SANDBOX == grimoire::bgwork_tplpdf::CODE_SANDBOX);
    let (asked, log) = with_log("nosandbox", || {
        let asked: Vec<Ask> = all.iter().map(picture_request).collect();
        settle();
        asked
    });
    check!(asked.iter().all(|a| matches!(a, Ask::Job(_))));
    check!(log.matches("nothing made (no sandboxed worker can be had)").count() == 1);
    check!(log.matches("bg: the sandbox refused a package picture's worker a step").count() == 1);
    check!(log.matches("bg: package pictures of a file").count() == 1 && bg_last_exit() == CODE_SANDBOX);
    check!(!log.contains("pictures decode in the writer"));
    check!(grimoire::sandbox::session_broken());
    check!(all.iter().all(|s| matches!(picture_cached(s), Cached::Nothing)));
    check!(cache_names(PicForm::AppIcon).is_empty() && cache_names(PicForm::Screenshot).is_empty());
    // nothing more is asked for in this session: no job, no child
    check!(all.iter().all(|s| picture_request(s) == Ask::No) && !bg_busy() && bg_kind_count(BgKind::PackagePictures) == 0);
    // (and the job called here, in the session that is broken now: the same status, nothing written)
    for s in &all {
        let (rc, log) = with_log("nosandbox2", || job(s));
        check!(rc == CODE_SANDBOX && log.contains("nothing made (no sandboxed worker can be had)"));
        check!(!log.contains("pictures decode in the writer"));
        check!(matches!(picture_cached(s), Cached::Nothing));
    }
    check!(cache_names(PicForm::AppIcon).is_empty() && cache_names(PicForm::Screenshot).is_empty());
    check!(grimoire::picwork::in_fallback());
    clean_env();
}

#[test]
fn test_package_pictures() {
    run!(an_apps_icon_a_store_icon_and_screenshots_are_made);
    run!(a_refused_file_is_marked_and_not_tried_again);
    run!(a_worker_that_ends_or_takes_too_long_is_a_try_and_then_the_mark);
    run!(a_changed_file_gets_nothing_until_it_is_asked_for_again);
    run!(a_file_through_a_link_or_outside_its_folder_is_never_read);
    run!(a_cache_entry_that_does_not_read_is_nothing_and_is_made_again);
    run!(the_cache_is_pruned_oldest_first);
    run!(the_request_queues_a_few_jobs_at_a_time);
    run!(no_worker_writes_nothing);
    done();
}
