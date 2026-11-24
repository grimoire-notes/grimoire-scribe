// test_picwork_small.rs: the picture worker's small gray pictures (2.3, issue #236; picwork.rs `T`,
// `PicPool::small`): a package's icon and the Package Store's pictures. Every picture goes through a real worker, a
// fresh exec of the headless runner (GRIMOIRE_WORK_EXE), as in the app: an `.ico` with a BMP entry and one with a
// PNG entry (picwork/icofile.rs's reader, which only the worker runs), a PNG and a JPEG, one over the side it was asked at
// fitted inside it, and the ones the worker refuses. Then workers that are stand-ins (a shell script with the right
// handshake) and answer with a size, a channel count or a length the writer must not take. That no small picture is
// ever decoded in the writer, with no worker to be had, is test_picwork_fallback.rs's (its own binary: the state
// lasts the process).
#![cfg(feature = "sim")]
mod testlib;
use grimoire::picwork::*;
use grimoire::prelude::*;
use testlib::*;

const HOOKS: [&str; 6] = [
    "GRIMOIRE_PIC_TEST_CRASH",
    "GRIMOIRE_PIC_TEST_HANG",
    "GRIMOIRE_PIC_TEST_TRAP",
    "GRIMOIRE_PIC_WATCHDOG_MS",
    "GRIMOIRE_WORK_FAIL_STEP",
    "GRIMOIRE_SANDBOX",
];

fn clean_env() {
    for h in HOOKS {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
}

fn tmp(name: &str) -> String {
    let d = format!("{}/picsmall-{}-{name}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn gradient(w: i32, h: i32, ch: i32) -> BgPicture {
    let mut px = Vec::new();
    for y in 0..h {
        for x in 0..w {
            for c in 0..ch {
                px.push(((x * 3 + y * 5 + c * 40) % 251) as u8);
            }
        }
    }
    BgPicture { px, w, h, ch }
}

fn png_bytes(p: &BgPicture, dir: &str) -> Vec<u8> {
    let path = format!("{dir}/gen.png");
    assert_eq!(bgimage_write_png(&path, p), 0);
    std::fs::read(&path).unwrap()
}

/// An `.ico` file of these entries: each the directory's width and height bytes and the entry's own bytes.
fn ico_of(entries: &[(u8, u8, Vec<u8>)]) -> Vec<u8> {
    let mut out = vec![0, 0, 1, 0];
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    let mut at = 6 + 16 * entries.len();
    for (w, h, d) in entries {
        out.extend_from_slice(&[*w, *h, 0, 0, 1, 0, 32, 0]);
        out.extend_from_slice(&(d.len() as u32).to_le_bytes());
        out.extend_from_slice(&(at as u32).to_le_bytes());
        at += d.len();
    }
    for (_, _, d) in entries {
        out.extend_from_slice(d);
    }
    out
}

/// A classic 32-bit BMP entry (a 40-byte header, the rows bottom first as B G R A, then the 1-bit mask, all clear).
fn dib32(w: usize, h: usize, px: impl Fn(usize, usize) -> [u8; 4]) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&40u32.to_le_bytes());
    d.extend_from_slice(&(w as i32).to_le_bytes());
    d.extend_from_slice(&(2 * h as i32).to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&32u16.to_le_bytes());
    d.extend_from_slice(&[0u8; 24]);
    for y in (0..h).rev() {
        for x in 0..w {
            let [r, g, b, a] = px(x, y);
            d.extend_from_slice(&[b, g, r, a]);
        }
    }
    d.extend(std::iter::repeat(0u8).take(w.div_ceil(32) * 4 * h));
    d
}

/// What picwork/icofile.rs makes of a colour over white: the launcher's gray.
fn ico_gray_of(r: u8, g: u8, b: u8, a: u8) -> u8 {
    let over = |v: u8| (v as u32 * a as u32 + 255 * (255 - a as u32)) / 255;
    ((over(r) * 77 + over(g) * 150 + over(b) * 29) >> 8) as u8
}

fn luma_of(bytes: &[u8]) -> Canvas {
    let mut p = BgPicture::default();
    let mut why: &'static str = "";
    assert_eq!(bgimage_decode(bytes, &mut p, &mut why), 0, "{why}");
    let mut c = Canvas::default();
    assert_eq!(bgimage_luma(&p, &mut c), 0);
    c
}

fn small(pool: &mut PicPool, data: &[u8], what: usize, max: i32) -> Option<(i32, i32, BgPicture)> {
    match pool.small(data, what, max) {
        PicOut::Pic { src_w, src_h, fit } => Some((src_w, src_h, fit)),
        o => {
            eprintln!("  not a picture: {o:?}");
            None
        }
    }
}

// An `.ico` is read by the worker: a BMP entry (made here, and the fixture's, the launcher's own test icon) and a PNG
// entry, each the gray the launcher drew before, in one channel.
fn an_ico_with_a_bmp_entry_and_one_with_a_png_entry() {
    clean_env();
    let dir = tmp("ico");
    let mut pool = PicPool::worker_only();
    // a BMP entry: 20 x 10, a gray ramp, its first column clear (alpha 0: white)
    let pixel = |x: usize, y: usize| -> [u8; 4] {
        let v = (x * 11 + y * 7) as u8;
        [v, v / 2, 255 - v, if x == 0 { 0 } else { 255 }]
    };
    let bmp = ico_of(&[(20, 10, dib32(20, 10, pixel))]);
    let got = small(&mut pool, &bmp, SMALL_ICO, 256);
    check!(got.as_ref().is_some_and(|(sw, sh, p)| (*sw, *sh, p.w, p.h, p.ch) == (20, 10, 20, 10, 1) && p.px.len() == 200));
    if let Some((_, _, p)) = &got {
        let want: Vec<u8> = (0..10).flat_map(|y| (0..20).map(move |x| (x, y))).map(|(x, y)| { let [r, g, b, a] = pixel(x, y); ico_gray_of(r, g, b, a) }).collect();
        check!(p.px == want);
        check!(p.px[0] == 255 && p.px[20] == 255); // (the clear column is white)
    }
    // the fixture's (five BMP entries): the 64 px one
    let fixture = include_bytes!("../../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico");
    let got = small(&mut pool, fixture, SMALL_ICO, 256);
    check!(got.as_ref().is_some_and(|(sw, sh, p)| (*sw, *sh, p.w, p.h, p.ch) == (64, 64, 64, 64, 1) && p.px.len() == 64 * 64));
    check!(got.as_ref().is_some_and(|(_, _, p)| p.px.iter().any(|&g| g < 128) && p.px.iter().any(|&g| g > 128)));
    // a PNG entry, gray and RGB: the PNG's own pixels, as gray
    for ch in [1, 3] {
        let src = gradient(48, 40, ch);
        let png = ico_of(&[(48, 40, png_bytes(&src, &dir))]);
        let got = small(&mut pool, &png, SMALL_ICO, 256);
        check!(got.as_ref().is_some_and(|(sw, sh, p)| (*sw, *sh, p.w, p.h, p.ch) == (48, 40, 48, 40, 1)));
        if let Some((_, _, p)) = &got {
            let want: Vec<u8> = src.px.chunks(ch as usize).map(|q| if ch == 1 { ico_gray_of(q[0], q[0], q[0], 255) } else { ico_gray_of(q[0], q[1], q[2], 255) }).collect();
            check!(p.px == want);
        }
    }
    // the entry of at most 64 px is the one taken, the largest of those; with none, the smallest
    let both = ico_of(&[(128, 128, dib32(128, 128, |_, _| [0, 0, 0, 255])), (32, 32, dib32(32, 32, |_, _| [200, 200, 200, 255])), (16, 16, dib32(16, 16, |_, _| [90, 90, 90, 255]))]);
    check!(small(&mut pool, &both, SMALL_ICO, 256).is_some_and(|(_, _, p)| (p.w, p.h) == (32, 32) && p.px[0] == 200));
    let big = ico_of(&[(128, 128, dib32(128, 128, |_, _| [10, 10, 10, 255])), (96, 96, dib32(96, 96, |_, _| [70, 70, 70, 255]))]);
    check!(small(&mut pool, &big, SMALL_ICO, 256).is_some_and(|(_, _, p)| (p.w, p.h) == (96, 96) && p.px[0] == 70));
    // "either" takes an `.ico` as an `.ico`
    check!(small(&mut pool, fixture, SMALL_ICO | SMALL_PICTURE, 256).is_some_and(|(_, _, p)| (p.w, p.h) == (64, 64)));
}

// A PNG and a JPEG: the luma of the writer's own decode of 2.0, byte for byte, at the picture's own size.
fn a_png_and_a_jpeg() {
    clean_env();
    let dir = tmp("pic");
    let mut pool = PicPool::worker_only();
    for ch in [1, 3] {
        let bytes = png_bytes(&gradient(121, 77, ch), &dir);
        let want = luma_of(&bytes);
        for what in [SMALL_PICTURE, SMALL_ICO | SMALL_PICTURE] {
            let got = small(&mut pool, &bytes, what, 256);
            check!(got.as_ref().is_some_and(|(sw, sh, p)| (*sw, *sh, p.w, p.h, p.ch) == (121, 77, 121, 77, 1) && p.px == want.px));
        }
    }
    let jpeg = include_bytes!("../../tests/fixtures/photos/IMG_0001.jpg");
    let want = luma_of(jpeg);
    let got = small(&mut pool, jpeg, SMALL_PICTURE, 2560);
    check!(got.as_ref().is_some_and(|(sw, sh, p)| (*sw, *sh, p.w, p.h, p.ch) == (400, 300, 400, 300, 1) && p.px == want.px));
    // a Scribe page's screenshot is within the screenshots' side: kept pixel for pixel
    let shot = include_bytes!("../../tests/fixtures/package-store/package-store/assets/packages/com.example.fixture/screens/01-app.png");
    let want = luma_of(shot);
    check!(small(&mut pool, shot, SMALL_PICTURE, 2560).is_some_and(|(sw, sh, p)| (sw, sh, p.w, p.h) == (1860, 2480, 1860, 2480) && p.px == want.px));
}

// One over the side asked for comes back smaller: fitted inside max x max, its aspect kept, the source's size said.
fn one_over_the_side_is_fitted_inside_it() {
    clean_env();
    let dir = tmp("fit");
    let mut pool = PicPool::worker_only();
    let wide = png_bytes(&gradient(900, 300, 3), &dir);
    let got = small(&mut pool, &wide, SMALL_PICTURE, 256);
    // (255 wide: 85 rows of a 3:1 picture are 255 px across, and no column of the fit's white page comes with it)
    check!(got.as_ref().is_some_and(|(sw, sh, p)| (*sw, *sh, p.w, p.h, p.ch) == (900, 300, 255, 85, 1) && p.px.len() == 255 * 85));
    let tall = png_bytes(&gradient(300, 1200, 1), &dir);
    check!(small(&mut pool, &tall, SMALL_ICO | SMALL_PICTURE, 256).is_some_and(|(sw, sh, p)| (sw, sh, p.w, p.h) == (300, 1200, 64, 256)));
    // an even field stays even through the fit (an area filter: no edge of white comes in)
    let flat = png_bytes(&BgPicture { px: vec![90u8; 700 * 500], w: 700, h: 500, ch: 1 }, &dir);
    check!(small(&mut pool, &flat, SMALL_PICTURE, 256).is_some_and(|(_, _, p)| (p.w, p.h) == (256, 183) && p.px.iter().all(|&v| v == 90)));
    // at the side exactly: as it is
    let exact = png_bytes(&gradient(256, 256, 1), &dir);
    let want = luma_of(&exact);
    check!(small(&mut pool, &exact, SMALL_PICTURE, 256).is_some_and(|(_, _, p)| (p.w, p.h) == (256, 256) && p.px == want.px));
}

fn png_header_of(w: u32, h: u32, depth: u8, color: u8) -> Vec<u8> {
    let mut out = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];
    out.extend_from_slice(&[0, 0, 0, 13, b'I', b'H', b'D', b'R']);
    out.extend_from_slice(&w.to_be_bytes());
    out.extend_from_slice(&h.to_be_bytes());
    out.extend_from_slice(&[depth, color, 0, 0, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&[0, 0, 0, 0, b'I', b'D', b'A', b'T', 0, 0, 0, 0]);
    out
}

// What the worker refuses, each the picture's fault (`Bad`: the same bytes give the same answer), and the worker
// goes on to the next picture: files cut short, sides over the limits, a form that was not asked for.
fn refused_pictures_are_bad_and_the_worker_goes_on() {
    clean_env();
    let dir = tmp("bad");
    let mut pool = PicPool::worker_only();
    let fixture = include_bytes!("../../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico");
    let png = png_bytes(&gradient(60, 50, 1), &dir);
    let bad = |pool: &mut PicPool, data: &[u8], what: usize, max: i32| matches!(pool.small(data, what, max), PicOut::Bad(_));
    // cut short: an `.ico` whose entries are past its end, one cut inside its entry, a PNG cut inside its pixels
    check!(bad(&mut pool, &fixture[..100], SMALL_ICO, 256));
    let one = ico_of(&[(20, 10, dib32(20, 10, |_, _| [1, 2, 3, 255]))]);
    check!(bad(&mut pool, &one[..one.len() - 30], SMALL_ICO, 256));
    check!(bad(&mut pool, &png[..png.len() / 2], SMALL_PICTURE, 256));
    check!(bad(&mut pool, &png[..png.len() / 2], SMALL_ICO | SMALL_PICTURE, 256));
    // sides over the limits: a PNG entry of 300 px inside an `.ico` (256 at most), a BMP entry that says 300 px,
    // a picture whose header says more than 8192 px a side, an `.ico` of more than 256 KiB
    let inside = ico_of(&[(0, 0, png_bytes(&gradient(300, 300, 1), &dir))]);
    check!(bad(&mut pool, &inside, SMALL_ICO, 256));
    let mut wide = dib32(20, 10, |_, _| [9, 9, 9, 255]);
    wide[4..8].copy_from_slice(&300i32.to_le_bytes());
    check!(bad(&mut pool, &ico_of(&[(20, 10, wide)]), SMALL_ICO, 256));
    check!(bad(&mut pool, &png_header_of(9000, 9000, 8, 0), SMALL_PICTURE, 2560));
    let mut long = fixture.to_vec();
    long.resize(256 * 1024 + 1, 0);
    check!(bad(&mut pool, &long, SMALL_ICO, 256));
    // a PNG entry of 16 bits a channel, and one with a palette: the `.ico` reader took neither while the png crate
    // decoded its entries, and takes neither now that stb_image does (which would decode both)
    let png_entry = |color: png::ColorType, depth: png::BitDepth, samples: &[u8]| {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, 8, 8);
            e.set_color(color);
            e.set_depth(depth);
            if color == png::ColorType::Indexed {
                e.set_palette(vec![0, 0, 0, 255, 255, 255]);
            }
            e.write_header().unwrap().write_image_data(samples).unwrap();
        }
        out
    };
    let deep = png_entry(png::ColorType::Rgb, png::BitDepth::Sixteen, &[90u8; 8 * 8 * 6]);
    let indexed = png_entry(png::ColorType::Indexed, png::BitDepth::Eight, &[1u8; 64]);
    for entry in [&deep, &indexed] {
        check!(bad(&mut pool, &ico_of(&[(8, 8, entry.clone())]), SMALL_ICO, 256));
        check!(bad(&mut pool, &ico_of(&[(8, 8, entry.clone())]), SMALL_ICO | SMALL_PICTURE, 256));
        // (as a picture by itself each is one `D` and a screenshot take)
        check!(small(&mut pool, entry, SMALL_PICTURE, 256).is_some_and(|(_, _, p)| (p.w, p.h) == (8, 8)));
    }
    // an entry larger than the side asked for is no picture for that request
    check!(bad(&mut pool, fixture, SMALL_ICO, 32));
    // a form that was not asked for: a PNG as an `.ico`, an `.ico` as a picture; and no picture at all
    check!(bad(&mut pool, &png, SMALL_ICO, 256));
    check!(bad(&mut pool, fixture, SMALL_PICTURE, 256));
    check!(bad(&mut pool, b"no picture at all", SMALL_ICO | SMALL_PICTURE, 256));
    // what the writer does not even send: nothing, a form or a side outside the command's
    check!(bad(&mut pool, b"", SMALL_PICTURE, 256));
    check!(bad(&mut pool, &png, 0, 256) && bad(&mut pool, &png, 4, 256));
    check!(bad(&mut pool, &png, SMALL_PICTURE, 0) && bad(&mut pool, &png, SMALL_PICTURE, SMALL_SIDE_MAX as i32 + 1));
    // the same worker still serves a good one
    check!(small(&mut pool, &png, SMALL_PICTURE, 256).is_some_and(|(_, _, p)| (p.w, p.h) == (60, 50)));
    check!(!in_fallback());
}

// A worker that ends or hangs on a small picture: `Dead`, by one worker run (no second worker tries it), within the
// pool's own time; the next picture gets a fresh worker.
fn a_worker_that_ends_or_hangs_is_dead_within_the_pools_time() {
    clean_env();
    let dir = tmp("dead");
    let png = png_bytes(&gradient(60, 50, 1), &dir);
    std::env::set_var("GRIMOIRE_PIC_TEST_CRASH", "1");
    let mut pool = PicPool::for_preview(grimoire::sandbox::mono_ms() + 20_000);
    check!(matches!(pool.small(&png, SMALL_PICTURE, 256), PicOut::Dead(_)));
    std::env::remove_var("GRIMOIRE_PIC_TEST_CRASH");
    std::env::set_var("GRIMOIRE_PIC_TEST_HANG", "1");
    let t0 = std::time::Instant::now();
    let mut pool = PicPool::for_preview(grimoire::sandbox::mono_ms() + 1_500);
    let out = pool.small(&png, SMALL_PICTURE, 256);
    let waited = t0.elapsed().as_millis();
    eprintln!("  a hang: {out:?} after {waited} ms");
    check!(matches!(out, PicOut::Dead(why) if why == STALLED));
    check!((1_000..8_000).contains(&waited));
    std::env::remove_var("GRIMOIRE_PIC_TEST_HANG");
    let mut pool = PicPool::worker_only();
    check!(small(&mut pool, &png, SMALL_PICTURE, 256).is_some());
    check!(!in_fallback());
}

/// A stand-in worker (a shell script): the right handshake, the command line read, then `reply` (a printf format)
/// and `bytes` zero bytes, and it stays.
fn stand_in_worker(dir: &str, reply: &str, bytes: usize) -> String {
    use std::os::unix::fs::PermissionsExt;
    let path = format!("{dir}/worker.sh");
    let script = format!(
        "#!/bin/sh\nPATH=/usr/bin:/bin\nprintf 'B %s\\n' '{}' >&4\nprintf 'S 1\\n' >&4\nIFS= read -r line <&5\nprintf '{reply}\\n' >&4\nhead -c {bytes} /dev/zero >&4\nsleep 3\n",
        grimoire::sandbox::build_id()
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

// What the writer takes of a worker's answer: one channel, a size within the side it asked at and never larger
// than the source, the source's own bounds, and exactly w * h bytes. Anything else is no picture (`Dead`), and
// nothing of it is kept.
fn an_answer_the_writer_does_not_take_gives_no_picture() {
    clean_env();
    let dir = tmp("answers");
    let png = png_bytes(&gradient(60, 50, 1), &dir);
    for (reply, bytes, what) in [
        ("G 300 300 300 300 1", 90_000, "a side over the one asked for"),
        ("G 60 50 60 50 3", 9_000, "three channels"),
        ("G 60 50 60 50 2", 6_000, "two channels"),
        ("G 10 10 20 20 1", 400, "larger than its source"),
        ("G 9000 50 60 50 1", 3_000, "a source over 8192 px a side"),
        ("G 8000 8000 60 50 1", 3_000, "a source over 40 megapixels"),
        ("G 60 50 0 50 1", 0, "no width"),
        ("G 60 50 60 50 1", 2_999, "a byte short of w * h"),
        ("G 60 50 60 50", 3_000, "a record cut short"),
        ("I 60 50", 0, "another command's answer"),
        ("E sandbox chroot", 0, "a sandbox step said to fail after the bytes"),
    ] {
        std::env::set_var("GRIMOIRE_WORK_EXE", stand_in_worker(&dir, reply, bytes));
        let mut pool = PicPool::for_preview(grimoire::sandbox::mono_ms() + 1_500);
        let out = pool.small(&png, SMALL_PICTURE, 256);
        eprintln!("  {what}: {out:?}");
        check!(matches!(out, PicOut::Dead(_)));
    }
    // and the true answer from that stand-in is taken: the checks above refused the answers, not the stand-in
    std::env::set_var("GRIMOIRE_WORK_EXE", stand_in_worker(&dir, "G 60 50 60 50 1", 3_000));
    let mut pool = PicPool::for_preview(grimoire::sandbox::mono_ms() + 5_000);
    check!(small(&mut pool, &png, SMALL_PICTURE, 256).is_some_and(|(_, _, p)| (p.w, p.h, p.px.len()) == (60, 50, 3_000)));
    check!(!in_fallback());
    clean_env();
}

#[test]
fn test_picwork_small() {
    run!(an_ico_with_a_bmp_entry_and_one_with_a_png_entry);
    run!(a_png_and_a_jpeg);
    run!(one_over_the_side_is_fitted_inside_it);
    run!(refused_pictures_are_bad_and_the_worker_goes_on);
    run!(a_worker_that_ends_or_hangs_is_dead_within_the_pools_time);
    run!(an_answer_the_writer_does_not_take_gives_no_picture);
    done();
}
