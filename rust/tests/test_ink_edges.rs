// Ink edges, Smooth or Solid (2.3, #320; brush.rs brush_ink_solid and the solid deposit, page.rs stroke_solid): with
// Solid the solid pens draw a stroke as the threshold of its Smooth drawing against the pixels before it (a pixel it
// darkens by SOLID_STEP or more becomes 0, any other keeps its value), the other tools as with Smooth, and every way a
// page is drawn gives the same pixels. Smooth is the renderer as before (test_golden's tables and fingerprint).
#![allow(clippy::needless_range_loop)]
#[path = "testlib/mod.rs"]
mod testlib;
use grimoire::prelude::*;
use std::rc::Rc;
use testlib::*;

const DPI: f32 = 300.0;
const SOLID_PENS: [ToolKind; 5] = [TOOL_BALLPOINT, TOOL_FINELINER, TOOL_FOUNTAIN, TOOL_MARKER, TOOL_CALLIGRAPHY];

fn params(t: ToolKind) -> ToolParams {
    let mut p = ToolParams::default();
    tool_params_default(t, &mut p);
    p
}

fn canvas(w: i32, h: i32) -> Canvas {
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    c
}

// A stroke across a w x h canvas: a wave from left to right, its pressure from p0 to p1, the pen tilted (tx, ty).
fn wave(t: ToolKind, prm: &ToolParams, seed: u32, w: i32, h: i32, y: f32, p0: f32, p1: f32, tx: f32, ty: f32) -> Box<Stroke> {
    let mut s = stroke_new(t, prm, seed);
    let n = 48;
    for k in 0..n {
        let f = k as f32 / (n - 1) as f32;
        let q = PenSample {
            x: 12.0 + f * (w as f32 - 24.0),
            y: (y + (h as f32 / 6.0) * (f * 9.0).sin()).clamp(4.0, h as f32 - 4.0),
            pressure: p0 + (p1 - p0) * f,
            tilt_x: tx,
            tilt_y: ty,
            t_ms: k as u32 * 7,
        };
        stroke_append(&mut s, &q, DPI);
    }
    s
}

// s drawn over c sample by sample, as the editor draws it live, with the setting at e (an erase stroke restores toward
// `paper`, a canvas of c's size, or white).
fn draw(c: &mut Canvas, s: &Stroke, e: InkEdges, paper: Option<&Canvas>) {
    ink_edges_as(e, || {
        let mut b = BrushState::default();
        let pp = paper.map(|p| Rc::new(p.px.clone()));
        stroke_brush_begin(&mut b, s, DPI, pp, 0, 0, c.stride);
        for q in &s.pts {
            brush_add(&mut b, c, q);
        }
    });
}

// The backgrounds a stroke is checked over: white, a pencil line, a highlighter band, a ruled paper's gray lines, a
// picture's grays, and an earlier solid stroke.
fn backgrounds(w: i32, h: i32) -> Vec<(&'static str, Canvas)> {
    let mut out = Vec::new();
    out.push(("white", canvas(w, h)));
    let mut c = canvas(w, h);
    let mut pencil = params(TOOL_PENCIL);
    pencil.size_max_mm = 1.2;
    draw(&mut c, &wave(TOOL_PENCIL, &pencil, 11, w, h, h as f32 * 0.45, 0.3, 0.9, 40.0, 30.0), InkEdges::Smooth, None);
    out.push(("pencil", c));
    let mut c = canvas(w, h);
    let mut hl = stroke_new(TOOL_HIGHLIGHTER, &params(TOOL_HIGHLIGHTER), 12);
    stroke_mark_tip(&mut hl);
    for k in 0..30 {
        stroke_append(&mut hl, &PenSample { x: w as f32 * 0.5, y: 6.0 + k as f32 * (h as f32 - 12.0) / 29.0, pressure: 0.8, t_ms: k * 5, ..Default::default() }, DPI);
    }
    draw(&mut c, &hl, InkEdges::Smooth, None);
    out.push(("highlighter", c));
    let mut c = canvas(w, h);
    let mut y = 7;
    while y < h {
        canvas_fill(&mut c, Rect { x: 0, y, w, h: 2 }, 200);
        y += 23;
    }
    out.push(("rule", c));
    let mut c = canvas(w, h);
    for yy in 0..h {
        for xx in 0..w {
            c.px[(yy * c.stride + xx) as usize] = (110 + (xx * 3 + yy * 2) % 140) as u8;
        }
    }
    out.push(("picture", c));
    let mut c = canvas(w, h);
    draw(&mut c, &wave(TOOL_BALLPOINT, &params(TOOL_BALLPOINT), 13, w, h, h as f32 * 0.55, 0.6, 0.6, 0.0, 0.0), InkEdges::Solid, None);
    out.push(("solid ink", c));
    out
}

// Every solid pen, at pressures from light to full and three ways of holding the pen, over every background, as a
// long stroke and as a dot of one or two samples (an i's): the Solid drawing is, pixel for pixel, 0 where the Smooth
// drawing over the same pixels is 16 (of 255, the rule's SOLID_STEP) or more darker than the background, and the
// background everywhere else (nothing lighter, nothing but 0 changed). Every dot has a black pixel.
fn solid_is_the_threshold_of_smooth() {
    let (w, h) = (260, 200);
    let mut crossed = 0i64;
    for (name, bg) in backgrounds(w, h) {
        for (ti, &t) in SOLID_PENS.iter().enumerate() {
            for (pi, &(p0, p1)) in [(0.1, 0.1), (0.3, 0.3), (0.6, 0.6), (1.0, 1.0), (0.05, 0.95)].iter().enumerate() {
                for &(tx, ty) in &[(0.0, 0.0), (40.0, 46.0), (-35.0, 20.0)] {
                    let long = wave(t, &params(t), 100 + ti as u32 * 10 + pi as u32, w, h, h as f32 * 0.5, p0, p1, tx, ty);
                    let mut dots = Vec::new();
                    for n in 1..=2 {
                        let mut d = stroke_new(t, &params(t), 200 + ti as u32 * 10 + pi as u32);
                        for k in 0..n {
                            let q = PenSample { x: 73.4 + k as f32 * 1.5, y: 97.6, pressure: p1, tilt_x: tx, tilt_y: ty, t_ms: k as u32 * 7 };
                            stroke_append(&mut d, &q, DPI);
                        }
                        dots.push(d);
                    }
                    for (si, s) in std::iter::once(&long).chain(dots.iter()).enumerate() {
                        let mut smooth = bg.clone();
                        draw(&mut smooth, s, InkEdges::Smooth, None);
                        let mut solid = bg.clone();
                        draw(&mut solid, s, InkEdges::Solid, None);
                        let mut wrong = 0;
                        for i in 0..(w * h) as usize {
                            let pre = bg.px[i] as i32;
                            let want = if pre - smooth.px[i] as i32 >= 16 { 0 } else { pre };
                            wrong += (solid.px[i] as i32 != want) as i32;
                            crossed += (want == 0 && pre != 0) as i64;
                            check!(solid.px[i] == 0 || solid.px[i] == bg.px[i]);
                        }
                        check!(wrong == 0);
                        if wrong != 0 {
                            eprintln!("  {} over {name}, pressure {p0}..{p1}, tilt {tx} {ty}, stroke {si}: {wrong} pixels", tool_name(t));
                        }
                        if si > 0 && name == "white" {
                            check!(solid.px.contains(&0));
                        }
                    }
                }
            }
        }
    }
    check!(crossed > 100_000);
    check!(SOLID_STEP == 16);
}

// The ballpoint at pressure 0.6 on white, a horizontal stroke: Smooth's column across it is the one the port's
// experiment measured (254 106 42 38 54 186), and Solid draws it 5 px of 0; the fineliner 3 px.
fn solid_widths_are_the_ones_judged() {
    let (w, h) = (240, 120);
    for (t, width) in [(TOOL_BALLPOINT, 5), (TOOL_FINELINER, 3)] {
        let mut s = stroke_new(t, &params(t), 0x1234);
        for k in 0..=60 {
            let f = k as f32 / 60.0;
            stroke_append(&mut s, &PenSample { x: 20.0 + 200.0 * f, y: 60.3, pressure: 0.6, t_ms: k * 8, ..Default::default() }, DPI);
        }
        let mut solid = canvas(w, h);
        draw(&mut solid, &s, InkEdges::Solid, None);
        let mut smooth = canvas(w, h);
        draw(&mut smooth, &s, InkEdges::Smooth, None);
        let column = |c: &Canvas, x: i32| (0..h).map(|y| c.px[(y * c.stride + x) as usize]).collect::<Vec<u8>>();
        let dark = |c: &Canvas, x: i32| column(c, x).iter().filter(|&&v| v == 0).count();
        if t == TOOL_BALLPOINT {
            let col: Vec<u8> = column(&smooth, 120).into_iter().filter(|&v| v != 255).collect();
            check!(col == vec![254, 106, 42, 38, 54, 186]);
        }
        check!((40..200).all(|x| dark(&solid, x) == width));
        check!((40..200).all(|x| column(&solid, x).iter().all(|&v| v == 0 || v == 255)));
    }
}

// The rule at the defaults: exactly the ballpoint, fineliner, fountain pen, marker and calligraphy pen; a pen turned
// down to a light gray, a colour of a palette, the pencils, charcoal, the highlighter (either tip) and the eraser never.
fn the_rule_takes_the_solid_pens() {
    for t in 0..TOOL_COUNT as i32 {
        let t = ToolKind(t);
        for tip in [TIP_FIRST, TIP_HIGHLIGHTER_VERTICAL] {
            check!(brush_ink_solid(t, tip, &params(t), 0) == SOLID_PENS.contains(&t));
        }
        check!(!brush_ink_solid(t, TIP_FIRST, &params(t), 1));
    }
    let mut light = params(TOOL_BALLPOINT);
    light.opacity_min = 0.5;
    light.opacity_max = 0.7;
    check!(!brush_ink_solid(TOOL_BALLPOINT, TIP_FIRST, &light, 0));
    light.opacity_max = 0.8;
    check!(brush_ink_solid(TOOL_BALLPOINT, TIP_FIRST, &light, 0));
    // a pen given grain past the marker's own, or Tilt lightens, is a textured or shaded one: never Solid
    for t in SOLID_PENS {
        let mut grainy = params(t);
        grainy.grain = SOLID_GRAIN_MOST;
        check!(brush_ink_solid(t, TIP_FIRST, &grainy, 0));
        grainy.grain = 0.2;
        check!(!brush_ink_solid(t, TIP_FIRST, &grainy, 0));
        let mut shaded = params(t);
        shaded.tilt_opacity = 0.05;
        check!(!brush_ink_solid(t, TIP_FIRST, &shaded, 0));
    }
    check!(params(TOOL_MARKER).grain == SOLID_GRAIN_MOST);
    check!(!brush_ink_solid(ToolKind(TOOL_COUNT as i32), TIP_FIRST, &params(TOOL_BALLPOINT), 0));
    check!(InkEdges::from_index(1) == InkEdges::Solid && InkEdges::from_index(0) == InkEdges::Smooth && InkEdges::from_index(7) == InkEdges::Smooth);
    check!(InkEdges::NAMES == ["smooth", "solid"] && InkEdges::Solid.name() == "solid");
}

// With Solid, every stroke the rule leaves out draws exactly as with Smooth, over ink, pencil and gray: the pencils,
// charcoal, the highlighter (both tips), a ballpoint in a colour of a palette, a ballpoint turned down to a light
// gray, a grainy marker and a ballpoint shaded by tilt. White ink of every texture and the precise eraser too, but over a solid stroke's black: there a pixel takes
// its paper whole where the Smooth restore lightens it by SOLID_STEP or more, and stays 0 where it does not (never a
// gray between), the mirror of the solid deposit.
fn the_other_tools_draw_as_smooth() {
    let (w, h) = (260, 200);
    let mut bg = backgrounds(w, h).remove(1).1; // the pencil line
    draw(&mut bg, &wave(TOOL_BALLPOINT, &params(TOOL_BALLPOINT), 21, w, h, h as f32 * 0.3, 0.7, 0.7, 0.0, 0.0), InkEdges::Solid, None);
    let mut marker = params(TOOL_MARKER);
    marker.size_max_mm = 3.0;
    marker.size_min_mm = 3.0;
    draw(&mut bg, &wave(TOOL_MARKER, &marker, 22, w, h, h as f32 * 0.7, 0.7, 0.7, 0.0, 0.0), InkEdges::Solid, None);
    let paper = canvas(w, h);
    let mut strokes: Vec<Box<Stroke>> = Vec::new();
    for t in [TOOL_PENCIL, TOOL_MECH_PENCIL, TOOL_CHARCOAL, TOOL_HIGHLIGHTER] {
        for tip in [TIP_FIRST, TIP_HIGHLIGHTER_VERTICAL] {
            let mut s = wave(t, &params(t), 30 + t.0 as u32, w, h, h as f32 * 0.5, 0.2, 1.0, 30.0, 40.0);
            s.ext[2] = tip;
            strokes.push(s);
        }
    }
    for t in 0..TOOL_COUNT as i32 {
        let mut s = wave(TOOL_ERASER, &params(ToolKind(t)), erase_seed(ToolKind(t), 40 + t as u32), w, h, h as f32 * 0.5, 0.4, 1.0, 0.0, 0.0);
        s.params = params(if t == TOOL_ERASER.0 { TOOL_ERASER } else { ToolKind(t) });
        strokes.push(s);
    }
    let mut coloured = wave(TOOL_BALLPOINT, &params(TOOL_BALLPOINT), 50, w, h, h as f32 * 0.5, 0.6, 0.6, 0.0, 0.0);
    coloured.ext[0] = 3;
    strokes.push(coloured);
    let mut light = params(TOOL_BALLPOINT);
    light.opacity_min = 0.5;
    light.opacity_max = 0.5;
    strokes.push(wave(TOOL_BALLPOINT, &light, 51, w, h, h as f32 * 0.5, 0.6, 0.6, 0.0, 0.0));
    let mut grainy = params(TOOL_MARKER);
    grainy.grain = 1.0;
    strokes.push(wave(TOOL_MARKER, &grainy, 52, w, h, h as f32 * 0.5, 0.6, 0.6, 0.0, 0.0));
    let mut shaded = params(TOOL_BALLPOINT);
    shaded.tilt_opacity = 1.0;
    strokes.push(wave(TOOL_BALLPOINT, &shaded, 53, w, h, h as f32 * 0.5, 0.6, 0.6, 50.0, 50.0));
    let mut paper = paper;
    for (i, v) in paper.px.iter_mut().enumerate() {
        if (i as i32 / w) % 23 == 7 {
            *v = 200; // (a ruled paper's line: an erased solid pixel takes the line's gray back)
        }
    }
    let mut erased = 0;
    for s in &strokes {
        check!(!stroke_may_be_solid(s));
        let mut smooth = bg.clone();
        draw(&mut smooth, s, InkEdges::Smooth, Some(&paper));
        let mut solid = bg.clone();
        draw(&mut solid, s, InkEdges::Solid, Some(&paper));
        check!(smooth.px != bg.px);
        if !stroke_is_erase(s) {
            check!(smooth.px == solid.px);
            continue;
        }
        let mut wrong = 0;
        for i in 0..(w * h) as usize {
            let want = if bg.px[i] != 0 { smooth.px[i] } else if smooth.px[i] >= 16 { paper.px[i] } else { 0 };
            wrong += (solid.px[i] != want) as i32;
            erased += (bg.px[i] == 0 && solid.px[i] != 0) as i32;
        }
        check!(wrong == 0);
        if wrong != 0 {
            eprintln!("  erase stroke of texture {} at Solid: {wrong} pixels", s.seed >> 24);
        }
    }
    check!(erased > 10_000);
    // a highlighter or a pencil over a solid word changes none of its pixels (0 stays 0)
    for s in &strokes[..8] {
        let mut over = bg.clone();
        draw(&mut over, s, InkEdges::Solid, None);
        check!((0..(w * h) as usize).all(|i| bg.px[i] != 0 || over.px[i] == 0));
    }
}

// White ink traced along a solid stroke with its own pen and settings, as one corrects a word: with Solid it takes
// the stroke away and leaves no rim (the solid black under its soft edge comes back to the paper whole), where a gray
// one half restored was left before; with Smooth it leaves what it always left. The precise eraser at its default size
// leaves nothing either way.
fn erasing_a_solid_stroke_leaves_no_rim() {
    let (w, h) = (260, 120);
    for t in SOLID_PENS {
        let line = |seed: u32, tool: ToolKind, prm: &ToolParams| {
            let mut s = stroke_new(tool, prm, seed);
            stroke_mark_tip(&mut s);
            for k in 0..=50 {
                let f = k as f32 / 50.0;
                stroke_append(&mut s, &PenSample { x: 30.0 + 200.0 * f, y: 60.3, pressure: 0.6, t_ms: k * 8, ..Default::default() }, DPI);
            }
            s
        };
        let ink = line(0x77, t, &params(t));
        let white = line(erase_seed(t, 0x78), TOOL_ERASER, &params(t));
        let eraser = line(erase_seed(TOOL_ERASER, 0x79), TOOL_ERASER, &params(TOOL_ERASER));
        for e in [InkEdges::Smooth, InkEdges::Solid] {
            for (what, s) in [("white ink", &white), ("the precise eraser", &eraser)] {
                let mut c = canvas(w, h);
                draw(&mut c, &ink, e, None);
                check!(c.px.iter().any(|&v| v < 64));
                draw(&mut c, s, e, None);
                let darkest = *c.px.iter().min().unwrap();
                let left = c.px.iter().filter(|&&v| v < 200).count();
                // (Smooth's own trace: a faint one, no darker than 190 after white ink)
                let ok = if e == InkEdges::Solid || what == "the precise eraser" { left == 0 } else { darkest >= 180 };
                check!(ok);
                if !ok {
                    eprintln!("  {} erased by {what} at {:?}: darkest {darkest}, {left} pixels below 200", tool_name(t), e);
                }
            }
        }
    }
}

// Smooth's fingerprint is the renderer's as before whatever the setting (the PDF cache and every Smooth stamp are kept
// under it): taken first with the thread at Solid, it is the one taken at Smooth. Solid's is another.
fn smooth_fingerprint_whatever_the_setting() {
    let (fp, ink) = ink_edges_as(InkEdges::Solid, || (page_render_fingerprint(), page_ink_fingerprint()));
    check!(ink == page_ink_fingerprint() && ink != 0);
    check!(fp == page_render_fingerprint_with(ink, &paper_render) && fp != 0);
    let solid = page_render_fingerprint_for(InkEdges::Solid);
    check!(solid != 0 && solid != fp && page_render_fingerprint_for(InkEdges::Smooth) == fp);
    check!(page_render_fingerprint_screen() == fp);
    check!(ink_edges_as(InkEdges::Solid, page_render_fingerprint_screen) == solid);
    check!(ink_edges() == InkEdges::Smooth);
}

// A page of every kind of stroke on ruled paper: random strokes of every tool, ink and erase strokes among them.
fn random_page(w: i32, h: i32, seed: u32, n: usize) -> Box<Page> {
    let mut g = seed;
    let mut rnd = move || {
        g ^= g << 13;
        g ^= g >> 17;
        g ^= g << 5;
        g
    };
    let mut p = page_new(w, h, PAPER_MEDIUM);
    for i in 0..n {
        let r = rnd();
        let erase = r % 7 == 0;
        let t = if erase { ToolKind((rnd() % TOOL_COUNT as u32) as i32) } else { [TOOL_BALLPOINT, TOOL_FINELINER, TOOL_FOUNTAIN, TOOL_MARKER, TOOL_CALLIGRAPHY, TOOL_PENCIL, TOOL_HIGHLIGHTER, TOOL_BALLPOINT][(rnd() % 8) as usize] };
        let prm = params(if erase && t == TOOL_ERASER { TOOL_ERASER } else { t });
        let mut s = if erase { stroke_new(TOOL_ERASER, &prm, erase_seed(t, rnd())) } else { stroke_new(t, &prm, rnd() & 0xFFFFFF) };
        stroke_mark_tip(&mut s);
        let (mut x, mut y) = ((rnd() % w as u32) as f32, (rnd() % h as u32) as f32);
        let mut ang = (rnd() % 628) as f32 / 100.0;
        let k = 4 + (rnd() % 60) as usize;
        for j in 0..k {
            stroke_append(&mut s, &PenSample { x, y, pressure: 0.2 + (rnd() % 80) as f32 / 100.0, tilt_x: 20.0, tilt_y: 30.0, t_ms: j as u32 * 6 }, DPI);
            ang += ((rnd() % 140) as f32 - 70.0) / 100.0;
            x += ang.cos() * 9.0;
            y += ang.sin() * 9.0;
        }
        let _ = i;
        page_add(&mut p, s);
    }
    p
}

fn render(p: &Page, e: InkEdges) -> Canvas {
    let mut c = canvas(p.w, p.h);
    ink_edges_as(e, || page_render(&mut c, p, Rect { x: 0, y: 0, w: p.w, h: p.h }, DPI));
    c
}

// With Solid a page draws the same pixels every way it is drawn: stroke by stroke as the pen writes it (with the
// page's paper under the erase strokes), whole, in the stroke eraser's tiles after strokes are taken off, from a
// paste's first stroke on, and in compaction's pieces (a region a long stroke's middle never reaches).
fn solid_draws_the_same_every_way() {
    let (w, h) = (420, 520);
    for seed in [0x1234_5678u32, 0x0BAD_F00D, 0x7777_1111] {
        let p = random_page(w, h, seed, 140);
        let whole_page = Rect { x: 0, y: 0, w, h };
        let fresh = render(&p, InkEdges::Solid);
        check!(fresh.px != render(&p, InkEdges::Smooth).px);
        // live: the paper, then each stroke sample by sample
        let mut paper = canvas(w, h);
        page_paper_render(&mut paper, &p, whole_page, DPI);
        let mut live = paper.clone();
        for s in &p.strokes {
            draw(&mut live, s, InkEdges::Solid, Some(&paper));
        }
        check!(live.px == fresh.px);
        // the stroke eraser: strokes taken off, their tiles drawn again
        ink_edges_as(InkEdges::Solid, || {
            let gone: Vec<usize> = (0..p.strokes.len()).filter(|i| i % 9 == 4).collect();
            let mut q = page_new(w, h, p.paper);
            for (i, s) in p.strokes.iter().enumerate() {
                if !gone.contains(&i) {
                    page_add(&mut q, s.clone());
                }
            }
            let mut m = tile_mask_new(w, h).unwrap();
            let mut d = RECT_EMPTY;
            for &i in &gone {
                stroke_mark_tiles(&mut m, &p.strokes[i], DPI);
                d = rect_union(d, p.strokes[i].bbox);
            }
            tile_mask_seal(&mut m);
            let mut c = fresh.clone();
            page_render_tiles(&mut c, &q, d, &Rc::new(m), DPI);
            let mut want = canvas(w, h);
            page_render(&mut want, &q, whole_page, DPI);
            check!(c.px == want.px);
            // a paste: the last 30 strokes drawn over a drawing of the rest
            let first = p.strokes.len() - 30;
            let mut head = page_new(w, h, p.paper);
            for s in &p.strokes[..first] {
                page_add(&mut head, s.clone());
            }
            let mut c = canvas(w, h);
            page_render(&mut c, &head, whole_page, DPI);
            let pp = Rc::new(paper.px.clone());
            check!(page_render_from(&mut c, &p, whole_page, DPI, first, Some((&pp, paper.stride))));
            check!(c.px == fresh.px);
        });
    }
    // Compaction's pieces: a long ballpoint that goes right and comes back left beside itself (its pieces about 16 mm
    // each), drawn over the left part of the page, which the middle pieces never reach: the pieces that do share the
    // stroke's shadow, and a pixel two of them darken a little each is drawn as the whole stroke draws it.
    // (a few pens, pressures and gaps between the two ways, back along the very line among them: without the shared
    // shadow these differ by 1 to 240 pixels)
    let (w, h) = (700, 160);
    let mut shared_wrong = 0;
    for (t, pressure, gap) in [(TOOL_BALLPOINT, 0.35f32, 3.0f32), (TOOL_BALLPOINT, 0.5, 0.0), (TOOL_BALLPOINT, 0.5, 5.0), (TOOL_MARKER, 0.9, 0.0)] {
        let mut p = page_new(w, h, PAPER_BLANK);
        let mut s = stroke_new(t, &params(t), 9);
        let mut k = 0;
        let mut add = |s: &mut Stroke, x: f32, y: f32| {
            stroke_append(s, &PenSample { x, y, pressure, t_ms: k * 6, ..Default::default() }, DPI);
            k += 1;
        };
        for i in 0..120 {
            add(&mut s, 30.0 + i as f32 * 5.0, 60.0);
        }
        for i in 0..120 {
            add(&mut s, 625.0 - i as f32 * 5.0, 60.0 + gap);
        }
        page_add(&mut p, s);
        for e in [InkEdges::Smooth, InkEdges::Solid] {
            let all = render(&p, e);
            let left = Rect { x: 0, y: 0, w: 150, h };
            let mut c = canvas(w, h);
            ink_edges_as(e, || page_render_in_pieces(&mut c, &p, left, DPI));
            let mut wrong = 0;
            for y in 0..h {
                for x in 0..left.w {
                    let i = (y * w + x) as usize;
                    wrong += (c.px[i] != all.px[i]) as i32;
                }
            }
            check!(wrong == 0);
            shared_wrong += wrong;
            if wrong != 0 {
                eprintln!("  pieces of a {} stroke drawn {:?} at pressure {pressure}, {gap} px apart: {wrong} pixels differ", tool_name(t), e);
            }
            let mut c = canvas(w, h);
            ink_edges_as(e, || page_render_in_pieces(&mut c, &p, Rect { x: 0, y: 0, w, h }, DPI));
            check!(c.px == all.px);
        }
    }
    check!(shared_wrong == 0);
}

// PDF and PNG exports draw with Smooth whatever the setting (export.rs render_page_links): the same bytes with the
// thread at Solid as at Smooth, though the page drawn on screen with Solid differs.
fn exports_draw_smooth() {
    let root = format!("{}/ink-edges-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    let mut nb = NotebookInfo::default();
    check!(store_create("Edges", "", PAPER_MEDIUM, &mut nb) == 0);
    let p = random_page(600, 800, 0x5EED_0320, 60);
    check!(store_save_page(&nb, 0, &p) == 0);
    check!(render(&p, InkEdges::Solid).px != render(&p, InkEdges::Smooth).px);
    let png = |e: InkEdges| {
        let mut b = Buf::default();
        check!(ink_edges_as(e, || export_page_png(&nb, 0, DPI, &mut b)) == 0);
        b.data
    };
    let smooth = png(InkEdges::Smooth);
    check!(!smooth.is_empty() && png(InkEdges::Solid) == smooth);
    let pdf = |e: InkEdges| {
        let mut b = Buf::default();
        check!(ink_edges_as(e, || export_notebook_pdf(&nb, DPI, &mut b)) == 0);
        b.data
    };
    let smooth = pdf(InkEdges::Smooth);
    check!(!smooth.is_empty() && pdf(InkEdges::Solid) == smooth);
    check!(ink_edges() == InkEdges::Smooth);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_ink_edges() {
    check!(font_init() == 0); // (the papers of the fingerprint draw text)
    paper_set_panel_ppi(0.0, DPI);
    run!(smooth_fingerprint_whatever_the_setting); // (first: the fingerprint is taken once per process)
    run!(the_rule_takes_the_solid_pens);
    run!(solid_is_the_threshold_of_smooth);
    run!(solid_widths_are_the_ones_judged);
    run!(the_other_tools_draw_as_smooth);
    run!(erasing_a_solid_stroke_leaves_no_rim);
    run!(solid_draws_the_same_every_way);
    run!(exports_draw_smooth);
    done();
}
