// The Pages view's previews job (store_build_page_previews, 2.0.0-beta.2): the light pages before a dense one,
// a current kept drawing read back instead of drawn, a stale one never used (nor taken for a quick page), a
// dense page's drawing kept by the job and no light page's however slow (none with its picture missing), a
// preview cut short made again, what counts as a dense page (store_page_dense), and what counts as a slow one
// (store_page_slow, "Loading page..."'s, from 256 KB since the owner's decision of 2026-11-04).
#[path = "testlib/mod.rs"]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

const W: i32 = 600;
const H: i32 = 800;

fn fresh_root(tag: &str) -> String {
    let root = format!("{}/page-previews-{tag}-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    check!(store_init(&root) == 0);
    store_set_device(300.0, W, H);
    root
}

fn dir_of(nb: &NotebookInfo) -> String {
    let mut d = String::new();
    check!(store_notebook_dir(nb.id.as_str(), &mut d, 700) == 0);
    d
}

fn notebook(name: &str, n: i32) -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    check!(store_create(name, "", PAPER_NARROW, &mut nb) == 0);
    while nb.npages() < n {
        let at = nb.npages();
        check!(store_add_page(&mut nb, at, PAPER_NARROW) == 0);
    }
    nb
}

fn seeded(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (*seed >> 8) as f32 / (1 << 24) as f32
}

// A page of `strokes` ballpoint strokes of `pts` points each, wandering from seeded places.
fn page_of(strokes: usize, pts: usize, mut seed: u32) -> Box<Page> {
    let mut p = page_new(W, H, PAPER_NARROW);
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_BALLPOINT, &mut prm);
    for k in 0..strokes {
        let mut s = stroke_new(TOOL_BALLPOINT, &prm, 1000 + k as u32);
        let (mut x, mut y) = (seeded(&mut seed) * W as f32, seeded(&mut seed) * H as f32);
        for i in 0..pts {
            x = (x + seeded(&mut seed) * 8.0 - 4.0).clamp(0.0, W as f32);
            y = (y + seeded(&mut seed) * 8.0 - 4.0).clamp(0.0, H as f32);
            stroke_append(&mut s, &PenSample { x, y, pressure: 0.6, t_ms: i as u32 * 6, ..Default::default() }, 300.0);
        }
        page_add(&mut p, s);
    }
    p
}

fn preview_of(nb: &NotebookInfo, pid: u32) -> Option<Canvas> {
    let mut c = Canvas::default();
    store_load_page_preview(nb, pid, &mut c).then_some(c)
}

fn mean(c: &Canvas) -> f64 {
    c.px[..(c.w * c.h) as usize].iter().map(|&v| v as f64).sum::<f64>() / (c.w * c.h) as f64
}

fn modified(path: &str) -> std::time::SystemTime {
    std::fs::metadata(path).and_then(|m| m.modified()).unwrap()
}

// A dense page among light ones (first in the notebook's order) is made last: the light pages' previews are
// written before its own.
fn light_pages_first() {
    let _root = fresh_root("order");
    let nb = notebook("Order", 4);
    check!(store_save_page(&nb, 0, &page_of(1100, 60, 7)) == 0);
    for i in 1..4 {
        check!(store_save_page(&nb, i, &page_of(2, 40, 100 + i as u32)) == 0);
    }
    let dir = dir_of(&nb);
    let size = |i: usize| std::fs::metadata(format!("{dir}/p{}.qpg", nb.page_ids[i])).unwrap().len();
    check!(size(0) >= STORE_DENSE_PAGE_BYTES && size(1) < STORE_DENSE_PAGE_BYTES);
    check!(store_page_dense(&nb, nb.page_ids[0]) && !store_page_dense(&nb, nb.page_ids[1]));
    check!(store_build_page_previews(nb.id.as_str(), "0:4", 300.0) == 0);
    let pgm = |i: usize| format!("{dir}/.page-previews/p{}.pgm", nb.page_ids[i]);
    for i in 0..4 {
        check!(preview_of(&nb, nb.page_ids[i]).is_some());
    }
    let dense = modified(&pgm(0));
    for i in 1..4 {
        check!(modified(&pgm(i)) < dense);
    }
}

// A page with a current kept drawing gets its preview from that drawing, read back, not drawn: here a drawing
// all black, which no drawing of the page (a stroke on ruled paper) would give. Its kept drawing does not make
// the light page a dense one.
fn kept_drawing_read_back() {
    let _root = fresh_root("kept");
    let nb = notebook("Kept", 1);
    let pid = nb.page_ids[0];
    check!(store_save_page(&nb, 0, &page_of(2, 40, 3)) == 0);
    let mut black = Canvas::default();
    check!(canvas_init(&mut black, W, H) == 0);
    canvas_fill(&mut black, Rect::new(0, 0, W, H), 0);
    check!(store_save_page_render(&nb, pid, &black, false));
    check!(store_page_render_kept(&nb, pid, W, H, false) && !store_page_dense(&nb, pid));
    check!(store_build_page_previews(nb.id.as_str(), "0:1", 300.0) == 0);
    let p = preview_of(&nb, pid);
    check!(p.as_ref().is_some_and(|p| mean(p) < 1.0));
}

// A kept drawing of the page file as it was before a later save is never used: the page is drawn again.
fn stale_kept_drawing_ignored() {
    let _root = fresh_root("stale");
    let nb = notebook("Stale", 1);
    let pid = nb.page_ids[0];
    check!(store_save_page(&nb, 0, &page_of(2, 40, 3)) == 0);
    let mut black = Canvas::default();
    check!(canvas_init(&mut black, W, H) == 0);
    canvas_fill(&mut black, Rect::new(0, 0, W, H), 0);
    check!(store_save_page_render(&nb, pid, &black, false));
    std::thread::sleep(std::time::Duration::from_millis(20));
    check!(store_save_page(&nb, 0, &page_of(3, 40, 4)) == 0);
    check!(!store_page_render_kept(&nb, pid, W, H, false));
    check!(store_build_page_previews(nb.id.as_str(), "0:1", 300.0) == 0);
    let p = preview_of(&nb, pid);
    check!(p.as_ref().is_some_and(|p| mean(p) > 200.0));
}

// The job keeps the drawing of a dense page (store_page_dense: a page file of 512 KB or more), exactly what
// drawing it gives, so it opens from it and its next preview is read back; a page under
// 512 KB gets its preview and no kept drawing, however slow (store_keep_render_ms() 0 here: every drawing counts
// as slow, as the editor would count it). A page under 512 KB that kept a drawing before (the editor's, for a
// page it drew slowly, or an earlier build's job's) is no dense page either: the job makes its preview and keeps
// no drawing of it again, and its old one, gone stale, stays unused.
fn dense_drawing_kept_by_the_job() {
    let _root = fresh_root("keep");
    let nb = notebook("Keep", 3);
    check!(store_save_page(&nb, 0, &page_of(1100, 60, 11)) == 0);
    check!(store_save_page(&nb, 1, &page_of(400, 60, 12)) == 0);
    check!(store_save_page(&nb, 2, &page_of(30, 50, 13)) == 0);
    let dir = dir_of(&nb);
    let size = |i: usize| std::fs::metadata(format!("{dir}/p{}.qpg", nb.page_ids[i])).unwrap().len();
    check!(size(0) >= STORE_DENSE_PAGE_BYTES && size(1) < STORE_DENSE_PAGE_BYTES && size(1) > STORE_DENSE_PAGE_BYTES / 2);
    let (dense, light, kept_before) = (nb.page_ids[0], nb.page_ids[1], nb.page_ids[2]);
    // (page 3 kept a drawing before, then changed: its stamp is there, not current)
    let mut black = Canvas::default();
    check!(canvas_init(&mut black, W, H) == 0);
    canvas_fill(&mut black, Rect::new(0, 0, W, H), 0);
    check!(store_save_page_render(&nb, kept_before, &black, false));
    std::thread::sleep(std::time::Duration::from_millis(20));
    check!(store_save_page(&nb, 2, &page_of(31, 50, 14)) == 0);
    check!(!store_page_render_kept(&nb, kept_before, W, H, false));
    check!(store_page_dense(&nb, dense) && !store_page_dense(&nb, light) && !store_page_dense(&nb, kept_before));
    let stamp = format!("{dir}/.page-previews/p{kept_before}.full.stamp");
    let old_stamp = std::fs::read(&stamp).unwrap();
    STORE_TEST_KEEP_RENDER_MS.store(0, std::sync::atomic::Ordering::Relaxed);
    check!(store_build_page_previews(nb.id.as_str(), "0:3", 300.0) == 0);
    STORE_TEST_KEEP_RENDER_MS.store(u32::MAX, std::sync::atomic::Ordering::Relaxed);
    for pid in [dense, light, kept_before] {
        check!(preview_of(&nb, pid).is_some());
    }
    check!(!store_page_render_kept(&nb, light, W, H, false) && !store_page_render_kept(&nb, light, W, H, true));
    check!(!std::path::Path::new(&format!("{dir}/.page-previews/p{light}.full")).exists());
    check!(!store_page_dense(&nb, light));
    check!(!store_page_render_kept(&nb, kept_before, W, H, false) && !store_page_render_kept(&nb, kept_before, W, H, true));
    check!(std::fs::read(&stamp).unwrap() == old_stamp && !store_page_dense(&nb, kept_before));
    check!(store_page_render_kept(&nb, dense, W, H, false));
    let mut kept = Canvas::default();
    check!(canvas_init(&mut kept, W, H) == 0);
    check!(store_load_page_render(&nb, dense, W, H, &mut kept, false));
    let mut page = None;
    check!(store_read_page(&nb, 0, &mut page) == 0);
    let mut drawn = Canvas::default();
    check!(canvas_init(&mut drawn, W, H) == 0);
    page_render(&mut drawn, page.as_deref().unwrap(), Rect::new(0, 0, W, H), 300.0);
    check!(kept.px == drawn.px);
}

// A slow page under 512 KB (store_page_slow: a page file of 256 KB or more, which says "Loading page..." as it
// opens with no current kept drawing, the owner's decision of 2026-11-04) is no dense page: the job makes its
// preview and keeps no drawing of it, however slow (store_keep_render_ms() 0 here), so it still says so at its
// next open; the job's rule stays at 512 KB. A page under 256 KB is neither, and a dense page is both. A kept
// drawing (the editor's) changes neither: the notice asks store_page_render_kept as well.
fn slow_page_not_dense() {
    let _root = fresh_root("slow");
    let nb = notebook("Slow", 3);
    check!(store_save_page(&nb, 0, &page_of(1100, 60, 41)) == 0);
    check!(store_save_page(&nb, 1, &page_of(400, 60, 42)) == 0);
    check!(store_save_page(&nb, 2, &page_of(30, 50, 43)) == 0);
    let dir = dir_of(&nb);
    let size = |i: usize| std::fs::metadata(format!("{dir}/p{}.qpg", nb.page_ids[i])).unwrap().len();
    check!(STORE_SLOW_PAGE_BYTES == 256 * 1024 && STORE_DENSE_PAGE_BYTES == 512 * 1024);
    check!(size(0) >= STORE_DENSE_PAGE_BYTES);
    check!(size(1) >= STORE_SLOW_PAGE_BYTES && size(1) < STORE_DENSE_PAGE_BYTES);
    check!(size(2) < STORE_SLOW_PAGE_BYTES);
    let (dense, slow, light) = (nb.page_ids[0], nb.page_ids[1], nb.page_ids[2]);
    check!(store_page_slow(&nb, dense) && store_page_dense(&nb, dense));
    check!(store_page_slow(&nb, slow) && !store_page_dense(&nb, slow));
    check!(!store_page_slow(&nb, light) && !store_page_dense(&nb, light));
    STORE_TEST_KEEP_RENDER_MS.store(0, std::sync::atomic::Ordering::Relaxed);
    check!(store_build_page_previews(nb.id.as_str(), "0:3", 300.0) == 0);
    STORE_TEST_KEEP_RENDER_MS.store(u32::MAX, std::sync::atomic::Ordering::Relaxed);
    for pid in [dense, slow, light] {
        check!(preview_of(&nb, pid).is_some());
    }
    check!(store_page_render_kept(&nb, dense, W, H, false));
    check!(!store_page_render_kept(&nb, slow, W, H, false) && !store_page_render_kept(&nb, slow, W, H, true));
    check!(!std::path::Path::new(&format!("{dir}/.page-previews/p{slow}.full")).exists());
    check!(!store_page_render_kept(&nb, light, W, H, false) && !store_page_render_kept(&nb, light, W, H, true));
    check!(store_page_slow(&nb, slow) && !store_page_dense(&nb, slow) && !store_page_slow(&nb, light));
    // (the editor's kept drawing of each, as it keeps one of a page it drew slowly)
    let mut black = Canvas::default();
    check!(canvas_init(&mut black, W, H) == 0);
    canvas_fill(&mut black, Rect::new(0, 0, W, H), 0);
    check!(store_save_page_render(&nb, slow, &black, false) && store_save_page_render(&nb, light, &black, false));
    check!(store_page_render_kept(&nb, slow, W, H, false) && store_page_render_kept(&nb, light, W, H, false));
    check!(store_page_slow(&nb, slow) && !store_page_dense(&nb, slow));
    check!(!store_page_slow(&nb, light) && !store_page_dense(&nb, light));
}

// A dense page whose kept drawing is of its file as it is now but by another renderer (a build that draws ink
// otherwise: another fingerprint) is drawn again, and so is made after the light pages, as its size says: its kept
// drawing never puts it first. (Here a drawing all black, which is not used.)
fn foreign_kept_drawing_not_first() {
    let _root = fresh_root("foreign");
    let nb = notebook("Foreign", 4);
    check!(store_save_page(&nb, 0, &page_of(1100, 60, 7)) == 0);
    for i in 1..4 {
        check!(store_save_page(&nb, i, &page_of(2, 40, 100 + i as u32)) == 0);
    }
    let dense = nb.page_ids[0];
    let mut black = Canvas::default();
    check!(canvas_init(&mut black, W, H) == 0);
    canvas_fill(&mut black, Rect::new(0, 0, W, H), 0);
    check!(store_save_page_render(&nb, dense, &black, false));
    let dir = dir_of(&nb);
    let stamp = format!("{dir}/.page-previews/p{dense}.full.stamp");
    let tag = std::fs::read_to_string(&stamp).unwrap();
    // (the page file's stamp, |WxH|, the fingerprint, then the kept drawings' version: |k2 since 2.1.0)
    let size = format!("|{W}x{H}|");
    let (head, rest) = tag.split_once(size.as_str()).unwrap();
    let (fingerprint, version) = rest.split_once('|').unwrap();
    check!(fingerprint.len() == 16 && version == "k2");
    let other = if fingerprint == "0123456789abcdef" { "fedcba9876543210" } else { "0123456789abcdef" };
    std::fs::write(&stamp, format!("{head}{size}{other}|{version}")).unwrap();
    check!(!store_page_render_kept(&nb, dense, W, H, false));
    check!(store_build_page_previews(nb.id.as_str(), "0:4", 300.0) == 0);
    let pgm = |i: usize| format!("{dir}/.page-previews/p{}.pgm", nb.page_ids[i]);
    let p = preview_of(&nb, dense);
    check!(p.as_ref().is_some_and(|p| mean(p) > 1.0));
    let last = modified(&pgm(0));
    for i in 1..4 {
        check!(preview_of(&nb, nb.page_ids[i]).is_some() && modified(&pgm(i)) < last);
    }
}

// A dense page on picture paper whose picture is missing draws it white: the job keeps no drawing of it and
// writes no preview of it either, as the editor writes neither then; a dense page beside it is made and kept as
// always. (Kept, the white drawing would open the page with nothing said, and stay so once the picture is there.)
fn missing_picture_keeps_nothing() {
    let _root = fresh_root("picture");
    let nb = notebook("Picture", 2);
    let mut p = page_of(1100, 60, 31);
    p.paper = paper_image(PAPER_ROLE_TEMPLATE, 7);
    check!(store_save_page(&nb, 0, &p) == 0);
    check!(store_save_page(&nb, 1, &page_of(1100, 60, 32)) == 0);
    let dir = dir_of(&nb);
    check!(!std::path::Path::new(&format!("{dir}/bg/000007.png")).exists());
    let (pid, other) = (nb.page_ids[0], nb.page_ids[1]);
    check!(store_page_dense(&nb, pid) && store_page_dense(&nb, other));
    check!(store_build_page_previews(nb.id.as_str(), "0:2", 300.0) == 0);
    check!(!store_page_render_kept(&nb, pid, W, H, false) && !store_page_render_kept(&nb, pid, W, H, true));
    check!(preview_of(&nb, pid).is_none());
    check!(store_page_render_kept(&nb, other, W, H, false) && preview_of(&nb, other).is_some());
    check!(!bgimage_take_missing());
}

// A preview whose image was cut short (a cache write is never synced) under a current stamp is made again.
fn cut_short_preview_made_again() {
    let _root = fresh_root("cut");
    let nb = notebook("Cut", 2);
    for i in 0..2 {
        check!(store_save_page(&nb, i, &page_of(2, 40, 21 + i as u32)) == 0);
    }
    check!(store_build_page_previews(nb.id.as_str(), "0:2", 300.0) == 0);
    let pid = nb.page_ids[1];
    check!(store_page_preview_ok(&nb, pid));
    let pgm = format!("{}/.page-previews/p{pid}.pgm", dir_of(&nb));
    let bytes = std::fs::read(&pgm).unwrap();
    std::fs::write(&pgm, &bytes[..bytes.len() / 2]).unwrap();
    check!(store_page_preview_fresh(&nb, pid).is_some() && !store_page_preview_ok(&nb, pid));
    check!(store_build_page_previews(nb.id.as_str(), "0:2", 300.0) == 0);
    check!(store_page_preview_ok(&nb, pid) && preview_of(&nb, pid).is_some());
}

// Previews and cards drawn by another renderer (a version before 2.3.0, whose papers' lines were lighter, #314) go
// out of date at once as the app starts (store_previews_follow_renderer, from app_init): the job draws the previews
// again, and a card noted as drawn from its page's file is no longer of it (editor.rs card_page_source). Once this
// renderer is noted it leaves them alone; a later one puts them out of date again. A notebook with no previews and
// no card is left as it is, and so is one whose previews folder is a link (nothing is written behind it).
fn previews_of_another_renderer_drawn_again() {
    let root = fresh_root("renderer");
    let nb = notebook("Renderer", 2);
    let (p1, p2) = (nb.page_ids[0], nb.page_ids[1]);
    for i in 0..2 {
        check!(store_save_page(&nb, i, &page_of(2, 40, 50 + i as u32)) == 0);
    }
    // as an earlier version left them: the previews made, a card drawn from page 1, no renderer noted
    check!(store_build_page_previews(nb.id.as_str(), "0:2", 300.0) == 0);
    check!(store_page_preview_ok(&nb, p1) && store_page_preview_ok(&nb, p2));
    let mut card = Canvas::default();
    check!(canvas_init(&mut card, W / 3, H / 3) == 0);
    canvas_fill(&mut card, Rect::new(0, 0, W / 3, H / 3), 255);
    check!(store_save_thumb(nb.id.as_str(), &card) == 0);
    let source = store_page_source_identity(&nb, p1);
    check!(source.is_some());
    let bare = notebook("Bare", 1);
    let linked = notebook("Linked", 1);
    check!(store_save_page(&linked, 0, &page_of(2, 40, 60)) == 0);
    let outside = format!("{root}-outside");
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, format!("{}/.page-previews", dir_of(&linked))).unwrap();
    let noted = format!("{root}/.previews-renderer");
    let exists = |p: &str| std::fs::symlink_metadata(p).is_ok();
    check!(!exists(&noted) && !exists(&format!("{}/thumb.pgm", dir_of(&bare))));
    let fp = page_render_fingerprint();
    check!(store_previews_follow_renderer(fp) == 1);
    check!(!store_page_preview_ok(&nb, p1) && !store_page_preview_ok(&nb, p2) && preview_of(&nb, p1).is_none());
    check!(store_page_source_identity(&nb, p1).is_some_and(|s| Some(s) != source));
    check!(std::fs::read_to_string(&noted).ok() == Some(format!("{fp:016x}\n")));
    check!(std::fs::read_dir(&outside).unwrap().count() == 0 && !exists(&format!("{}/.page-previews", dir_of(&bare))));
    // drawn again by the job, then left alone by the same renderer
    check!(store_build_page_previews(nb.id.as_str(), "0:2", 300.0) == 0);
    check!(store_page_preview_ok(&nb, p1) && store_page_preview_ok(&nb, p2));
    let again = store_page_source_identity(&nb, p1);
    check!(store_previews_follow_renderer(fp) == 0);
    check!(store_page_preview_ok(&nb, p1) && store_page_preview_ok(&nb, p2) && store_page_source_identity(&nb, p1) == again);
    // a later renderer: out of date again
    check!(store_previews_follow_renderer(fp ^ 1) == 1);
    check!(!store_page_preview_ok(&nb, p1) && !store_page_preview_ok(&nb, p2));
    let _ = std::fs::remove_dir_all(&outside);
}

#[test]
fn test_page_previews() {
    run!(light_pages_first);
    run!(kept_drawing_read_back);
    run!(stale_kept_drawing_ignored);
    run!(dense_drawing_kept_by_the_job);
    run!(slow_page_not_dense);
    run!(foreign_kept_drawing_not_first);
    run!(missing_picture_keeps_nothing);
    run!(cut_short_preview_made_again);
    run!(previews_of_another_renderer_drawn_again);
    done();
}
