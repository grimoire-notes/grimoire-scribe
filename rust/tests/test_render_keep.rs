// Dense pages keep their drawing (store_save_page_render): read back only while it is current.
#[path = "testlib/mod.rs"]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn page_with_ink(w: i32, h: i32) -> Box<Page> {
    page_with_ink_on(PAPER_NARROW, w, h)
}

fn page_with_ink_on(paper: PaperKind, w: i32, h: i32) -> Box<Page> {
    let mut p = page_new(w, h, paper);
    for t in [TOOL_MARKER, TOOL_CHARCOAL, TOOL_BALLPOINT] {
        let mut prm = ToolParams::default();
        tool_params_default(t, &mut prm);
        let mut s = stroke_new(t, &prm, 77 + t.0 as u32);
        for k in 0..60 {
            let f = k as f32 / 59.0;
            stroke_append(&mut s, &PenSample { x: 40.0 + f * (w as f32 - 80.0), y: 60.0 + 40.0 * t.0 as f32 + 30.0 * (f * 7.0).sin(), pressure: 0.6, t_ms: k * 5, ..Default::default() }, 300.0);
        }
        page_add(&mut p, s);
    }
    p
}

fn kept_drawings_are_exact_and_only_current() {
    let mut nb = NotebookInfo::default();
    check!(store_create("Dense", "", PAPER_NARROW, &mut nb) == 0);
    let (w, h) = (600, 800);
    let p = page_with_ink(w, h);
    check!(store_save_page(&nb, 0, &p) == 0);
    let pid = nb.page_ids[0];
    let mut drawn = Canvas::default();
    check!(canvas_init(&mut drawn, w, h) == 0);
    page_render(&mut drawn, &p, Rect { x: 0, y: 0, w, h }, 300.0);
    check!(!store_page_render_kept(&nb, pid, w, h, false));
    check!(store_save_page_render(&nb, pid, &drawn, false));
    check!(store_page_render_kept(&nb, pid, w, h, false));
    // Read back byte for byte, into a canvas of the page's size only.
    let mut back = Canvas::default();
    check!(canvas_init(&mut back, w, h) == 0);
    check!(store_load_page_render(&nb, pid, w, h, &mut back, false) && back.px == drawn.px);
    let mut other = Canvas::default();
    check!(canvas_init(&mut other, w, h + 1) == 0);
    check!(!store_load_page_render(&nb, pid, w, h + 1, &mut other, false));
    // A drawing kept with an unlayered tag (as 2.0.0-beta.1 keeps every page's) is not trusted for a
    // layered page, and a layered one ("|L") not for an unlayered page.
    check!(!store_page_render_kept(&nb, pid, w, h, true));
    check!(!store_load_page_render(&nb, pid, w, h, &mut other, true));
    check!(store_save_page_render(&nb, pid, &drawn, true));
    check!(store_page_render_kept(&nb, pid, w, h, true) && !store_page_render_kept(&nb, pid, w, h, false));
    check!(store_save_page_render(&nb, pid, &drawn, false));
    // The page file written again (new ink): the kept drawing no longer counts.
    let mut p2 = page_with_ink(w, h);
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_PENCIL, &mut prm);
    let mut s = stroke_new(TOOL_PENCIL, &prm, 5);
    stroke_append(&mut s, &PenSample { x: 10.0, y: 10.0, pressure: 0.5, ..Default::default() }, 300.0);
    page_add(&mut p2, s);
    std::thread::sleep(std::time::Duration::from_millis(20));
    check!(store_save_page(&nb, 0, &p2) == 0);
    check!(!store_page_render_kept(&nb, pid, w, h, false));
    let mut stale = Canvas::default();
    check!(canvas_init(&mut stale, w, h) == 0);
    check!(!store_load_page_render(&nb, pid, w, h, &mut stale, false));
    // The fingerprint is stable within a process.
    check!(page_render_fingerprint() == page_render_fingerprint() && page_render_fingerprint() != 0);
}

fn dir_of(nb: &NotebookInfo) -> String {
    let mut dir = String::new();
    check!(store_notebook_dir(nb.id.as_str(), &mut dir, 600) == 0);
    dir
}

// A drawing 2.0.0 or 2.0.1 kept: its tag is this build's without the version ("|k2") after the fingerprint.
fn kept_by_2_0_1(nb: &NotebookInfo, pid: u32) {
    let stamp = format!("{}/.page-previews/p{pid}.full.stamp", dir_of(nb));
    let tag = std::fs::read_to_string(&stamp).unwrap();
    check!(tag.contains("|k2"));
    std::fs::write(&stamp, tag.replace("|k2", "")).unwrap();
}

fn drawings_kept_by_2_0_1_are_never_read_and_retired_once() {
    // 2.0.0 and 2.0.1 could keep the old picture of a page just cleared or given a new paper under the new file's
    // stamp (editor.rs page_is_its_file): such a tag cannot be told from a true one, so none is read.
    let mut nb = NotebookInfo::default();
    check!(store_create("Before", "", PAPER_NARROW, &mut nb) == 0);
    let (w, h) = (600, 800);
    let p = page_with_ink(w, h);
    check!(store_save_page(&nb, 0, &p) == 0);
    let pid = nb.page_ids[0];
    let mut drawn = Canvas::default();
    check!(canvas_init(&mut drawn, w, h) == 0);
    page_render(&mut drawn, &p, Rect { x: 0, y: 0, w, h }, 300.0);
    for layered in [false, true] {
        check!(store_save_page_render(&nb, pid, &drawn, layered));
        kept_by_2_0_1(&nb, pid);
        let mut back = Canvas::default();
        check!(canvas_init(&mut back, w, h) == 0);
        check!(!store_page_render_kept(&nb, pid, w, h, layered));
        check!(!store_load_page_render(&nb, pid, w, h, &mut back, layered));
        // It goes, once (stamp and drawing)
        check!(store_retire_page_render(&nb, pid));
        check!(!store_retire_page_render(&nb, pid));
        check!(!std::path::Path::new(&format!("{}/.page-previews/p{pid}.full", dir_of(&nb))).exists());
    }
    // One of the page file as it was before (the file written again since, by an idle save inside the previews'
    // 30 s or a notebook closed before its idle save) goes too: the preview and the card made beside it may show
    // a Clear's old ink all the same.
    check!(store_save_page_render(&nb, pid, &drawn, true));
    kept_by_2_0_1(&nb, pid);
    std::thread::sleep(std::time::Duration::from_millis(20));
    check!(store_save_page(&nb, 0, &p) == 0);
    check!(store_retire_page_render(&nb, pid) && !store_retire_page_render(&nb, pid));
    // This build's drawing is never taken for one, current or not, nor one of a later version
    check!(store_save_page_render(&nb, pid, &drawn, false));
    check!(!store_retire_page_render(&nb, pid));
    check!(store_page_render_kept(&nb, pid, w, h, false));
    std::thread::sleep(std::time::Duration::from_millis(20));
    check!(store_save_page(&nb, 0, &p) == 0);
    check!(!store_retire_page_render(&nb, pid));
    let stamp = format!("{}/.page-previews/p{pid}.full.stamp", dir_of(&nb));
    let tag = std::fs::read_to_string(&stamp).unwrap();
    std::fs::write(&stamp, tag.replace("|k2", "|k3")).unwrap();
    check!(!store_retire_page_render(&nb, pid));
    check!(std::path::Path::new(&format!("{}/.page-previews/p{pid}.full", dir_of(&nb))).exists());
}

// The kept drawing's tag rewritten with another renderer's fingerprint, as that renderer would have written it.
fn kept_under(nb: &NotebookInfo, pid: u32, fp: u64) {
    let stamp = format!("{}/.page-previews/p{pid}.full.stamp", dir_of(nb));
    let tag = std::fs::read_to_string(&stamp).unwrap();
    let fp = format!("{fp:016x}");
    let mut parts: Vec<&str> = tag.split('|').collect();
    check!(parts.len() == 4 && parts[2].len() == 16);
    parts[2] = &fp;
    std::fs::write(&stamp, parts.join("|")).unwrap();
}

fn every_paper_is_in_the_fingerprint() {
    // A drawing is kept under the renderer's fingerprint, which since 2.3.0 hashes every paper after the ink: a
    // build that draws any one paper differently has another, and its drawings are drawn again. (It hashed the ink
    // page alone before, whose paper has no line on it: a paper drawn differently kept its old drawings, #314.)
    let (fp, ink) = (page_render_fingerprint(), page_ink_fingerprint());
    check!(fp != 0 && ink != 0 && fp != ink && page_render_fingerprint_with(ink, &paper_render) == fp);
    for k in paper_every_drawing().into_iter().filter(|&k| k != PAPER_BLANK) {
        let as_blank = move |c: &mut Canvas, kk: PaperKind, r: Rect, dpi: f32| paper_render(c, if kk == k { PAPER_BLANK } else { kk }, r, dpi);
        check!(page_render_fingerprint_with(ink, &as_blank) != fp);
    }
    // A drawing kept under the fingerprint before 2.3.0 (the ink's alone) is not read; the same file under this
    // build's is.
    let mut nb = NotebookInfo::default();
    check!(store_create("Before 2.3.0", "", PAPER_NARROW, &mut nb) == 0);
    let (w, h) = (600, 800);
    let p = page_with_ink(w, h);
    check!(store_save_page(&nb, 0, &p) == 0);
    let pid = nb.page_ids[0];
    let mut drawn = Canvas::default();
    check!(canvas_init(&mut drawn, w, h) == 0);
    page_render(&mut drawn, &p, Rect { x: 0, y: 0, w, h }, 300.0);
    check!(store_save_page_render(&nb, pid, &drawn, false));
    kept_under(&nb, pid, ink);
    let mut back = Canvas::default();
    check!(canvas_init(&mut back, w, h) == 0);
    check!(!store_page_render_kept(&nb, pid, w, h, false) && !store_load_page_render(&nb, pid, w, h, &mut back, false));
    kept_under(&nb, pid, fp);
    check!(store_page_render_kept(&nb, pid, w, h, false) && store_load_page_render(&nb, pid, w, h, &mut back, false) && back.px == drawn.px);
}

// Graph paper and Hexagon as main drew them before 2.3.0 (issue #314): lines of 200 at 1 px with every fifth 170
// at 2 px; edges of 175 at 1.5 px anti-aliased. Every other paper as this build draws it.
fn grid_before(c: &mut Canvas, dpi: f32) {
    let pitch = mm_to_px(5.0, dpi);
    let thick = |px: f32| ((px * dpi / 300.0).round_ties_even() as i32).max(1);
    let (w, h) = (c.w, c.h);
    let at = |i: i32| (pitch * i as f32).round_ties_even() as i32;
    let mut i = 0;
    while at(i) < w {
        let x = at(i);
        canvas_vline(c, x, 0, h, if i % 5 != 0 { thick(1.0) } else { thick(2.0) }, if i % 5 != 0 { 200 } else { 170 });
        i += 1;
    }
    let mut j = 0;
    while at(j) < h {
        let y = at(j);
        canvas_hline(c, 0, w, y, if j % 5 != 0 { thick(1.0) } else { thick(2.0) }, if j % 5 != 0 { 200 } else { 170 });
        j += 1;
    }
}
fn hex_before(c: &mut Canvas, dpi: f32) {
    let a = mm_to_px(4.0, dpi);
    let hh = a * 0.866_025_4;
    let (w, h) = (c.w as f32, c.h as f32);
    let lw = ((2.0 * dpi / 300.0).round_ties_even() as i32).max(1) as f32 * 0.75;
    let cols = (w / (1.5 * a)) as i32 + 2;
    let rows = (h / (2.0 * hh)) as i32 + 2;
    let x0 = (w - (cols - 1) as f32 * 1.5 * a) / 2.0;
    for i in 0..cols {
        let cx = fmla(i as f32, 1.5 * a, x0);
        let shift = if i % 2 != 0 { hh } else { 0.0 };
        for j in -1..rows {
            let cy = fmla(j as f32, 2.0 * hh, shift);
            let (l, top, bot) = (cx - a, cy - hh, cy + hh);
            canvas_line_aa(c, cx - a / 2.0, top, cx + a / 2.0, top, lw, 175);
            canvas_line_aa(c, l, cy, cx - a / 2.0, top, lw, 175);
            canvas_line_aa(c, l, cy, cx - a / 2.0, bot, lw, 175);
        }
    }
}
fn papers_before(c: &mut Canvas, k: PaperKind, region: Rect, dpi: f32) {
    if k != PAPER_GRID && k != PAPER_HEX {
        paper_render(c, k, region, dpi);
        return;
    }
    let saved = c.clip;
    canvas_set_clip(c, rect_intersect(region, saved));
    let clip = c.clip;
    canvas_fill(c, clip, 255);
    if k == PAPER_GRID {
        grid_before(c, dpi);
    } else {
        hex_before(c, dpi);
    }
    c.clip = saved;
}

fn drawings_kept_under_earlier_papers_are_drawn_again() {
    // 2.3.0 draws the ruled, grid, dot and pattern papers darker (#314): a page kept drawn with the lines before
    // must be drawn again. The fingerprint before 2.3.0 hashed the ink page alone, this build's ink half; and a
    // renderer that draws ink as this one does but Graph paper or Hexagon as main did has another fingerprint
    // too, each paper alone.
    let main = page_ink_fingerprint();
    let papers_main = page_render_fingerprint_with(main, &papers_before);
    check!(main != 0 && papers_main != 0);
    check!(page_render_fingerprint() != main && page_render_fingerprint() != papers_main);
    for k in [PAPER_GRID, PAPER_HEX] {
        let one = move |c: &mut Canvas, kk: PaperKind, r: Rect, dpi: f32| {
            if kk == k { papers_before(c, kk, r, dpi) } else { paper_render(c, kk, r, dpi) }
        };
        check!(page_render_fingerprint_with(main, &one) != page_render_fingerprint());
    }
    let (w, h) = (600, 800);
    let whole = Rect { x: 0, y: 0, w, h };
    for paper in [PAPER_GRID, PAPER_HEX] {
        let mut nb = NotebookInfo::default();
        check!(store_create("Earlier paper", "", paper, &mut nb) == 0);
        let p = page_with_ink_on(paper, w, h);
        check!(store_save_page(&nb, 0, &p) == 0);
        let pid = nb.page_ids[0];
        // the page as main drew it: its paper, the ink over it
        let mut old = Canvas::default();
        check!(canvas_init(&mut old, w, h) == 0);
        papers_before(&mut old, paper, whole, 300.0);
        page_render_layers(&mut old, &p, whole, 300.0, PAGE_LAYER_INK);
        let mut back = Canvas::default();
        check!(canvas_init(&mut back, w, h) == 0);
        for fp in [main, papers_main] {
            check!(store_save_page_render(&nb, pid, &old, false));
            kept_under(&nb, pid, fp);
            check!(!store_page_render_kept(&nb, pid, w, h, false));
            check!(!store_load_page_render(&nb, pid, w, h, &mut back, false));
        }
        // (the same file under this build's fingerprint is read: the renderer is all that differs)
        kept_under(&nb, pid, page_render_fingerprint());
        check!(store_page_render_kept(&nb, pid, w, h, false) && store_load_page_render(&nb, pid, w, h, &mut back, false) && back.px == old.px);
        // Drawn again, the page has the darker lines and none of the lines before.
        let mut now = Canvas::default();
        check!(canvas_init(&mut now, w, h) == 0);
        page_render(&mut now, &p, whole, 300.0);
        check!(now.px != old.px);
        let (mut then, mut lines) = (Canvas::default(), Canvas::default());
        check!(canvas_init(&mut then, w, h) == 0 && canvas_init(&mut lines, w, h) == 0);
        papers_before(&mut then, paper, whole, 300.0);
        page_render_layers(&mut lines, &p, whole, 300.0, PAGE_LAYER_PAPER);
        let has = |c: &Canvas, g: u8| c.px.contains(&g);
        let darkest = |c: &Canvas| c.px.iter().copied().min().unwrap_or(255);
        if paper == PAPER_GRID {
            check!(has(&then, 200) && has(&then, 170) && has(&lines, 136) && has(&lines, 85) && !has(&lines, 200) && !has(&lines, 170));
        } else {
            check!(darkest(&then) == 175 && darkest(&lines) == 119);
        }
        check!(store_save_page_render(&nb, pid, &now, false) && store_page_render_kept(&nb, pid, w, h, false));
    }
}

#[test]
fn test_render_keep() {
    let root = format!("{}/render-keep-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    run!(kept_drawings_are_exact_and_only_current);
    run!(drawings_kept_by_2_0_1_are_never_read_and_retired_once);
    run!(every_paper_is_in_the_fingerprint);
    run!(drawings_kept_under_earlier_papers_are_drawn_again);
    let _ = std::fs::remove_dir_all(&root);
    done(); // (a failed check fails the test: it did not, without this)
}
