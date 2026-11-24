// test_app_icons.rs: an installed app's bundled icon as the launcher draws it (2.3, issue #236; src/appicon.rs over
// src/pkgpics.rs). The launcher never reads the `.ico`: its draw looks in a small table and in the cache, the
// Library's tick asks the background job for what is missing, and the picture the sandboxed worker made is drawn
// once it is there. Every picture goes through a real worker, a fresh exec of the headless runner
// (GRIMOIRE_WORK_EXE), as in the app. Until the picture is there, and when the worker refuses the file or none can
// be had, the launcher draws the manifest's built-in icon. An app's picture goes from the cache with the app.
#![cfg(feature = "sim")]
mod testlib;
use grimoire::appicon::*;
use grimoire::pkgpics::*;
use grimoire::prelude::*;
use grimoire::sha256::sha256_of;
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
const ARTIFACT: &[u8] = b"fixture app bytes";

fn clean_env() {
    for h in HOOKS {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
}

/// Grimoire's folder made afresh (its store set up there), and the launcher's table emptied, as after a start.
fn root() -> String {
    ui_init(300.0);
    assert_eq!(font_init(), 0);
    let g = format!("{}/ai-root", tl_tmp());
    let _ = std::fs::remove_dir_all(&g);
    std::fs::create_dir_all(&g).unwrap();
    assert_eq!(store_init(&g), 0);
    app_icons_forget();
    g
}

fn manifest(version: &str, icon: &[u8]) -> String {
    format!(
        "name: Test App\nid: test.app\ntype: app\nversion: {version}\nauthor: Tester\ndescription: An app\nplatforms: [all]\nartifact:\n  path: app.wasm\n  sha256: {}\n  size: {}\nspec:\n  runtime: wasm\n  icon: icon.ico\n  icon_sha256: {}\n  icon_size: {}\n  status_bar: false\n",
        sha256_of(ARTIFACT),
        ARTIFACT.len(),
        sha256_of(icon),
        icon.len()
    )
}

fn install(version: &str, icon: &[u8]) -> WidgetInstall {
    app_package_install(&manifest(version, icon), ARTIFACT, Some(icon), "https://example.org/app.yaml", 1860, 2480).unwrap()
}

fn png(w: i32, h: i32) -> Vec<u8> {
    let dir = format!("{}/ai-png", tl_tmp());
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

/// The Library's ticks until no icon is to come and no job runs: each asks for what is to ask and takes in what
/// came. Whether a picture came.
fn ticks() -> bool {
    let t0 = grimoire::util::real_now_ms();
    let mut came = false;
    loop {
        came |= app_icons_step();
        bg_poll();
        if (app_icons_coming() == 0 && !bg_busy()) || grimoire::util::real_now_ms().wrapping_sub(t0) > 60_000 {
            break;
        }
        sleep_ms(5);
    }
    assert!(app_icons_coming() == 0 && !bg_busy());
    came
}

fn size_of(icon: &AppIcon) -> Option<(u16, u16)> {
    icon.image.as_ref().map(|i| (i.w, i.h))
}

fn cache_names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(picture_cache_dir(PicForm::AppIcon)).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    v.sort();
    v
}

// The launcher's draw of an app whose icon no worker has made yet: the built-in icon at once, nothing read of the
// `.ico`, no worker and no job started by the draw itself. The tick asks for it; once its job has ended the same
// call gives the picture, and a launcher drawn after a start finds it in the cache with no job at all.
fn the_built_in_icon_until_the_workers_picture_is_there() {
    clean_env();
    let _g = root();
    let it = install("1.0.0", ICO);
    // (a worker that would hang, and a job that would take a while: the draw waits for neither)
    std::env::set_var("GRIMOIRE_PIC_TEST_HANG", "1");
    std::env::set_var("GRIMOIRE_PIC_WATCHDOG_MS", "3000");
    let t0 = std::time::Instant::now();
    let first = AppIcon::for_install(&it);
    let waited = t0.elapsed().as_millis();
    std::env::remove_var("GRIMOIRE_PIC_TEST_HANG");
    std::env::remove_var("GRIMOIRE_PIC_WATCHDOG_MS");
    check!(first.image.is_none() && first.glyph == widget_icon("icon.ico"));
    check!(waited < 500 && !bg_busy() && app_icons_coming() == 1);
    // drawn: the built-in glyph (ink in the box)
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, 200, 200) == 0);
    first.draw(&mut c, Rect::new(20, 20, 160, 160), 0);
    let glyph_px = c.px.clone();
    check!(c.px.iter().filter(|&&v| v < 16).count() > 1000);
    // a second draw before the tick asks nothing more
    check!(AppIcon::for_install(&it).image.is_none() && !bg_busy() && app_icons_coming() == 1);
    // the tick asks, the job's worker makes it, and the tick that sees the job ended says a picture came
    check!(ticks());
    let icon = AppIcon::for_install(&it);
    check!(size_of(&icon) == Some((64, 64)) && icon.glyph == first.glyph);
    check!(icon.image.as_ref().is_some_and(|i| i.gray.len() == 64 * 64 && i.gray.iter().any(|&g| g < 128) && i.gray.iter().any(|&g| g > 128)));
    check!(canvas_init(&mut c, 200, 200) == 0);
    icon.draw(&mut c, Rect::new(20, 20, 160, 160), 0);
    check!(c.px != glyph_px && c.px.iter().any(|&v| v != 255));
    check!(cache_names().len() == 2);
    // no more ticks say a picture came, and nothing more is asked
    check!(!ticks() && !app_icons_step() && !bg_busy());
    // after a start (the table empty): the picture is read from the cache by the draw, and no job is asked for
    app_icons_forget();
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    check!(size_of(&AppIcon::for_install(&it)) == Some((64, 64)) && app_icons_coming() == 0);
    check!(!app_icons_step() && !bg_busy());
    clean_env();
    // an app with a built-in icon name has no file to ask for
    let plain = WidgetInstall { icon: "gear".into(), ..it.clone() };
    check!(AppIcon::for_install(&plain) == AppIcon::glyph(widget_icon("gear")) && app_icons_coming() == 0);
    let none = WidgetInstall { icon: String::new(), ..it.clone() };
    check!(AppIcon::for_install(&none) == AppIcon::glyph(IconKind::Grid));
}

// An icon the worker refuses (no `.ico` at all, though its manifest's digest holds): the built-in icon, its mark in
// the cache, and never asked for again, in this run or after a start.
fn a_refused_icon_keeps_the_built_in_icon() {
    clean_env();
    let _g = root();
    let junk = b"this is no icon, whatever its name says".to_vec();
    let it = install("1.0.0", &junk);
    check!(AppIcon::for_install(&it).image.is_none() && app_icons_coming() == 1);
    check!(!ticks());
    check!(AppIcon::for_install(&it).image.is_none() && app_icons_coming() == 0);
    let names = cache_names();
    check!(names.len() == 1 && names[0].ends_with(".info"));
    for _ in 0..2 {
        std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
        check!(AppIcon::for_install(&it).image.is_none() && app_icons_coming() == 0 && !app_icons_step() && !bg_busy());
        app_icons_forget();
    }
    clean_env();
    // a PNG under the name is no `.ico` either: a bundled icon is read as an `.ico` alone, as before
    let it = install("1.0.1", &png(64, 64));
    check!(AppIcon::for_install(&it).image.is_none() && !ticks() && AppIcon::for_install(&it).image.is_none());
}

// An app's picture goes from the cache with the app: an update takes the replaced version's and the new icon is
// made when it is asked for; Remove takes it, with the app's state kept or not. An icon file changed by hand is
// looked up afresh (another size or time is another file).
fn an_apps_picture_goes_with_the_app() {
    clean_env();
    let g = root();
    let it = install("1.0.0", ICO);
    check!(AppIcon::for_install(&it).image.is_none() && ticks() && size_of(&AppIcon::for_install(&it)) == Some((64, 64)));
    let first = cache_names();
    check!(first.len() == 2);
    // an update with another icon
    let other = ico_of_png(48, 40, &png(48, 40));
    let it = install("1.1.0", &other);
    check!(cache_names().is_empty());
    check!(AppIcon::for_install(&it).image.is_none() && ticks() && size_of(&AppIcon::for_install(&it)) == Some((48, 40)));
    let second = cache_names();
    check!(second.len() == 2 && second != first);
    // Remove, keeping the app's state
    app_package_remove(&it.id, true).unwrap();
    check!(cache_names().is_empty() && app_package_list().is_empty());
    check!(AppIcon::for_install(&it).image.is_none() && app_icons_coming() == 0);
    // installed again (after a start: the table empty) and removed whole
    app_icons_forget();
    let it = install("1.1.0", &other);
    check!(AppIcon::for_install(&it).image.is_none() && ticks() && size_of(&AppIcon::for_install(&it)) == Some((48, 40)));
    check!(cache_names().len() == 2);
    app_package_remove(&it.id, false).unwrap();
    check!(cache_names().is_empty());
    // an icon file written over by hand: the launcher shows the built-in icon until the new file's picture is made
    let it = install("1.2.0", ICO);
    check!(AppIcon::for_install(&it).image.is_none() && ticks() && size_of(&AppIcon::for_install(&it)) == Some((64, 64)));
    std::fs::write(format!("{g}/packages/apps/test.app/icon.ico"), &other).unwrap();
    check!(AppIcon::for_install(&it).image.is_none() && app_icons_coming() == 1);
    check!(ticks() && size_of(&AppIcon::for_install(&it)) == Some((48, 40)));
    // a link at the icon's name is not looked at, whatever it leads to
    let elsewhere = format!("{}/ai-elsewhere.ico", tl_tmp());
    std::fs::write(&elsewhere, ICO).unwrap();
    std::fs::remove_file(format!("{g}/packages/apps/test.app/icon.ico")).unwrap();
    std::os::unix::fs::symlink(&elsewhere, format!("{g}/packages/apps/test.app/icon.ico")).unwrap();
    check!(AppIcon::for_install(&it).image.is_none() && app_icons_coming() == 0 && !ticks());
}

// Many apps: every icon is asked for, a few jobs at a time, and all of them come.
fn many_icons_are_asked_for_a_few_at_a_time() {
    clean_env();
    let g = root();
    let apps: Vec<WidgetInstall> = (0..9)
        .map(|k| {
            let dir = format!("{g}/packages/apps/app{k}");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(format!("{dir}/icon.ico"), ICO).unwrap();
            WidgetInstall { id: format!("app{k}"), name: format!("App {k}"), icon: "icon.ico".into(), ..Default::default() }
        })
        .collect();
    check!(apps.iter().all(|it| AppIcon::for_install(it).image.is_none()) && app_icons_coming() == 9 && !bg_busy());
    app_icons_step();
    check!(bg_kind_count(BgKind::PackagePictures) == PICTURES_AT_ONCE && app_icons_coming() == 9);
    check!(ticks());
    check!(apps.iter().all(|it| size_of(&AppIcon::for_install(it)) == Some((64, 64))));
    check!(cache_names().len() == 18);
}

// No worker to be had: the built-in icon, nothing written and nothing decoded here; the icon is not asked for again
// in this run, and is after a start. In a session whose sandbox step failed no job is asked for at all.
fn no_worker_leaves_the_built_in_icon() {
    clean_env();
    let _g = root();
    let it = install("1.0.0", ICO);
    std::env::set_var("GRIMOIRE_WORK_EXE", "/nonexistent/grimoire");
    check!(AppIcon::for_install(&it).image.is_none() && !ticks());
    check!(AppIcon::for_install(&it).image.is_none() && app_icons_coming() == 0 && cache_names().is_empty());
    check!(!app_icons_step() && !bg_busy());
    // after a start, with a worker to be had: asked for again, and made
    clean_env();
    app_icons_forget();
    check!(AppIcon::for_install(&it).image.is_none() && ticks() && size_of(&AppIcon::for_install(&it)) == Some((64, 64)));
    // a broken session: nothing is asked for, the built-in icon stays (the cache's own pictures still draw)
    let _g = root();
    let it = install("1.0.0", ICO);
    grimoire::sandbox::session_set_broken();
    check!(AppIcon::for_install(&it).image.is_none() && app_icons_coming() == 1);
    check!(!app_icons_step() && !bg_busy() && app_icons_coming() == 0);
    check!(AppIcon::for_install(&it).image.is_none() && cache_names().is_empty());
}

#[test]
fn test_app_icons() {
    run!(the_built_in_icon_until_the_workers_picture_is_there);
    run!(a_refused_icon_keeps_the_built_in_icon);
    run!(an_apps_picture_goes_with_the_app);
    run!(many_icons_are_asked_for_a_few_at_a_time);
    run!(no_worker_leaves_the_built_in_icon);
    done();
}
