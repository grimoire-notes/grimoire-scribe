//! fbpack.h: copy one logical grayscale row into an 8/32-bit framebuffer, in either direction.
//! The caller bounds the destination and optional previous-frame row; strides are byte/pixel
//! steps and can be negative. Previous holds logical grays, before inversion or live dithering.

use crate::prelude::*;

/// The 8x8 ordered dither's thresholds (live ink frames are bilevel in DU mode).
static K_BAYER8: [u8; 64] = [
    0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26,
    12, 44, 4, 36, 14, 46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22,
    3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25,
    15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21,
];

/// In Rust the rows are slices and offsets: pixel i of the destination is dst[dst_at + i * step]
/// (step in bytes, negative for a mirrored framebuffer), of previous previous[previous_at + i *
/// previous_step]; None for no previous frame. Out-of-range offsets panic (a bounds check per
/// pixel, which the C leaves to its caller).
pub fn fbpack_row(dst: &mut [u8], dst_at: usize, step: isize, previous: Option<&mut [u8]>, previous_at: usize, previous_step: isize, src: &[u8], width: i32, x: i32, y: i32, bpp: i32, invert: bool, fast: bool, known: bool) {
    fbpack_row_cached(dst, dst_at, step, previous, None, previous_at, previous_step, src, width, x, y, bpp, invert, fast, known);
}

#[inline]
fn dither_toward(v: i32, g_raw: u8, bayer: &[u8], bx: usize, invert: bool) -> Option<u8> {
    if v == 0 {
        let out = if invert { 255 } else { 0 };
        return if g_raw == out { None } else { Some(out) };
    }
    if v == 255 {
        let out = if invert { 0 } else { 255 };
        return if g_raw == out { None } else { Some(out) };
    }
    let threshold = bayer[bx] as i32 * 4 + 2;
    let g = if invert { 255 - g_raw as i32 } else { g_raw as i32 };
    let out = if v < g {
        if v * 255 > threshold * g {
            return None;
        }
        if invert { 255 } else { 0 }
    } else if v > g {
        if (v - g) * 255 <= threshold * (255 - g) {
            return None;
        }
        if invert { 0 } else { 255 }
    } else {
        return None;
    };
    if g_raw == out { None } else { Some(out) }
}

/// Same as `fbpack_row`, with an optional RAM shadow (`shown`, indexed like `previous`) of the
/// physical bytes currently in `dst`. Reading `shown` instead of `dst` avoids uncached reads from
/// an mmap'ed `/dev/fb0` framebuffer on ARM, which otherwise stalls full-screen fast (DU) updates
/// such as pinch zoom by hundreds of milliseconds per frame.
#[allow(clippy::too_many_arguments)]
pub fn fbpack_row_cached(
    dst: &mut [u8],
    dst_at: usize,
    step: isize,
    mut previous: Option<&mut [u8]>,
    mut shown: Option<&mut [u8]>,
    previous_at: usize,
    previous_step: isize,
    src: &[u8],
    width: i32,
    x: i32,
    y: i32,
    bpp: i32,
    invert: bool,
    fast: bool,
    known: bool,
) {
    if width <= 0 || (bpp != 8 && bpp != 32) {
        return;
    }
    let width = width as usize;
    let src = &src[..width];

    // Fast contiguous paths for 8bpp grayscale frames (Ui / Full / Clean).
    // Both portrait orientations (step == 1 and Scribe 2's flipped step == -1) pack through an
    // L1-cached stack buffer and write forward into the write-combining framebuffer.
    if !fast && bpp == 8 && (step == 1 || step == -1) {
        const CHUNK: usize = 2048;
        if step == 1 {
            if !invert {
                dst[dst_at..dst_at + width].copy_from_slice(src);
                if let Some(sh) = shown.as_deref_mut() {
                    if previous_step == 1 {
                        sh[previous_at..previous_at + width].copy_from_slice(src);
                    } else {
                        for i in 0..width {
                            sh[(previous_at as isize + i as isize * previous_step) as usize] = src[i];
                        }
                    }
                }
            } else {
                let mut buf = [0u8; CHUNK];
                let mut off = 0usize;
                while off < width {
                    let len = (width - off).min(CHUNK);
                    for j in 0..len {
                        buf[j] = 255 - src[off + j];
                    }
                    dst[dst_at + off..dst_at + off + len].copy_from_slice(&buf[..len]);
                    if let Some(sh) = shown.as_deref_mut() {
                        if previous_step == 1 {
                            sh[previous_at + off..previous_at + off + len].copy_from_slice(&buf[..len]);
                        } else {
                            for j in 0..len {
                                sh[(previous_at as isize + (off + j) as isize * previous_step) as usize] = buf[j];
                            }
                        }
                    }
                    off += len;
                }
            }
            if let Some(pv) = previous.as_deref_mut() {
                if previous_step == 1 {
                    pv[previous_at..previous_at + width].copy_from_slice(src);
                } else {
                    for i in 0..width {
                        pv[(previous_at as isize + i as isize * previous_step) as usize] = src[i];
                    }
                }
            }
            return;
        } else {
            // step == -1: pixel i writes to dst[dst_at - i], contiguous in [dst_at + 1 - width ..= dst_at].
            let mut buf = [0u8; CHUNK];
            let mut pbuf = [0u8; CHUNK];
            let mut off = 0usize;
            while off < width {
                let len = (width - off).min(CHUNK);
                let end = off + len;
                if invert {
                    for j in 0..len {
                        let s = src[off + j];
                        buf[len - 1 - j] = 255 - s;
                        pbuf[len - 1 - j] = s;
                    }
                } else {
                    for j in 0..len {
                        buf[len - 1 - j] = src[off + j];
                    }
                }
                let fb_lo = dst_at + 1 - end;
                dst[fb_lo..fb_lo + len].copy_from_slice(&buf[..len]);
                if let Some(sh) = shown.as_deref_mut() {
                    if previous_step == -1 {
                        let p_lo = previous_at + 1 - end;
                        sh[p_lo..p_lo + len].copy_from_slice(&buf[..len]);
                    } else {
                        for j in 0..len {
                            sh[(previous_at as isize + (off + j) as isize * previous_step) as usize] = buf[len - 1 - j];
                        }
                    }
                }
                if let Some(pv) = previous.as_deref_mut() {
                    if previous_step == -1 {
                        let p_lo = previous_at + 1 - end;
                        pv[p_lo..p_lo + len].copy_from_slice(if invert { &pbuf[..len] } else { &buf[..len] });
                    } else {
                        for j in 0..len {
                            pv[(previous_at as isize + (off + j) as isize * previous_step) as usize] = src[off + j];
                        }
                    }
                }
                off = end;
            }
            return;
        }
    }

    let bayer = &K_BAYER8[((y & 7) << 3) as usize..][..8];

    // Fast contiguous paths for 8bpp DU (fast) frames with RAM `shown` shadow: zero /dev/fb0 reads,
    // no per-pixel bounds checks, and pure black/white bypasses Bayer arithmetic.
    if fast && known && bpp == 8 {
        if let (Some(pv), Some(sh)) = (previous.as_deref_mut(), shown.as_deref_mut()) {
            if step == 1 && previous_step == 1 {
                let dst_s = &mut dst[dst_at..dst_at + width];
                let pv_s = &mut pv[previous_at..previous_at + width];
                let sh_s = &mut sh[previous_at..previous_at + width];
                for i in 0..width {
                    let v = src[i];
                    if pv_s[i] == v {
                        continue;
                    }
                    pv_s[i] = v;
                    if let Some(out) = dither_toward(v as i32, sh_s[i], bayer, ((x + i as i32) & 7) as usize, invert) {
                        sh_s[i] = out;
                        dst_s[i] = out;
                    }
                }
                return;
            }
            if step == -1 && previous_step == -1 {
                let d_lo = dst_at + 1 - width;
                let p_lo = previous_at + 1 - width;
                let dst_s = &mut dst[d_lo..d_lo + width];
                let pv_s = &mut pv[p_lo..p_lo + width];
                let sh_s = &mut sh[p_lo..p_lo + width];
                for k in 0..width {
                    let i = width - 1 - k;
                    let v = src[i];
                    if pv_s[k] == v {
                        continue;
                    }
                    pv_s[k] = v;
                    if let Some(out) = dither_toward(v as i32, sh_s[k], bayer, ((x + i as i32) & 7) as usize, invert) {
                        sh_s[k] = out;
                        dst_s[k] = out;
                    }
                }
                return;
            }
        }
    }

    for i in 0..width {
        // (the C's uint8_t v, used as an int in every expression below)
        let mut v = src[i] as i32;
        let p = (dst_at as isize + i as isize * step) as usize;
        let pi = (previous_at as isize + i as isize * previous_step) as usize;
        // DU leaves gray target pixels alone. Keep unchanged gray paper gray, and keep earlier
        // dithered ink in its current state, instead of dithering the whole dirty rectangle.
        if fast && known {
            if let Some(pv) = previous.as_deref() {
                if pv[pi] as i32 == v {
                    continue;
                }
            }
        }
        if let Some(pv) = previous.as_deref_mut() {
            pv[pi] = v as u8;
        }
        let out: u8;
        if fast {
            let bx = ((x + i as i32) & 7) as usize;
            if known && previous.is_some() {
                // A DU transition ends in black or white, and a pixel cannot start another until
                // it has. So move what the panel shows only toward the new value: black or not
                // yet when it darkens, white or not yet when it lightens, as often as keeps the
                // average. Ink on gray paper then goes straight to black, instead of white first
                // and black a whole waveform later. Over black and white this is the dither below.
                let g_raw = match shown.as_deref() {
                    Some(sh) => sh[pi],
                    None => dst[p],
                };
                match dither_toward(v, g_raw, bayer, bx, invert) {
                    Some(b) => out = b,
                    None => continue,
                }
            } else {
                if v != 0 && v != 255 {
                    let threshold = bayer[bx] as i32 * 4 + 2;
                    v = if v > threshold { 255 } else { 0 };
                }
                out = if invert { (255 - v) as u8 } else { v as u8 };
            }
        } else {
            out = if invert { (255 - v) as u8 } else { v as u8 };
        }
        if let Some(sh) = shown.as_deref_mut() {
            sh[pi] = out;
        }
        dst[p] = out;
        if bpp == 32 {
            dst[p + 1] = out;
            dst[p + 2] = out;
            dst[p + 3] = 255;
        }
    }
}

/// The first and last pixel i of a logical row `src` whose level (v >> 4, inverted first when `invert`, as the row is
/// written) differs from the level of the byte `shown` holds for it, shown[shown_at + i * shown_step] (the RAM shadow
/// of what was last written to the framebuffer, indexed as fbpack_row_cached's `shown` and `previous`); None when every
/// level matches. Pixels lighter than what is shown count as well as darker ones. (The settle after writing with Ink
/// edges at Solid asks this of every row it would refresh: display_settle_area.)
pub fn fbpack_level_span(src: &[u8], shown: &[u8], shown_at: usize, shown_step: isize, invert: bool) -> Option<(usize, usize)> {
    let mut span: Option<(usize, usize)> = None;
    for (i, &v) in src.iter().enumerate() {
        let v = if invert { 255 - v } else { v };
        let at = (shown_at as isize + i as isize * shown_step) as usize;
        if v >> 4 != shown[at] >> 4 {
            span = Some((span.map_or(i, |(a, _)| a), i));
        }
    }
    span
}

/// Odd-rotation MTK pen updates below 6000 pixels can bypass coordinate processing. Expand the
/// refresh, without changing its pixels, to at least 6400 pixels while staying inside the panel.
pub fn fbpack_pen_region(r: Rect, width: i32, height: i32) -> Rect {
    if width <= 0 || height <= 0 {
        return RECT_EMPTY;
    }
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: width, h: height });
    if rect_is_empty(r) || (r.w as i64) * (r.h as i64) >= 6400 {
        return r;
    }
    let mut w = if r.h >= 80 { (6400 + r.h - 1) / r.h } else { 80 };
    w = clampi(w, r.w, width);
    let h = clampi((6400 + w - 1) / w, r.h, height);
    // Narrow panels may need more width after the height reaches their edge.
    if (w as i64) * (h as i64) < 6400 {
        w = clampi((6400 + h - 1) / h, w, width);
    }
    let x = clampi(r.x - (w - r.w) / 2, 0, width - w);
    let y = clampi(r.y - (h - r.h) / 2, 0, height - h);
    Rect { x, y, w, h }
}

/// fbpack_pen_region, kept clear of `keep` (#337): the panel's rows the app keeps out of every fast refresh (a
/// notebook's bar drawn over the page), in the same coordinates, a strip along one edge of the panel (full height at
/// its left or right, or full width at its top or bottom). The grown region is moved off the strip along the axis
/// across it, keeping its size and r inside it, and staying inside the panel: its pixels are unchanged, and so is the
/// rule it is grown for (6400 pixels at least). An r that meets the strip itself, or a strip on no edge, is grown as
/// fbpack_pen_region grows it.
pub fn fbpack_pen_region_clear(r: Rect, width: i32, height: i32, keep: Rect) -> Rect {
    let q = fbpack_pen_region(r, width, height);
    let keep = rect_intersect(keep, Rect { x: 0, y: 0, w: width, h: height });
    if rect_is_empty(keep) || rect_is_empty(q) || !rect_overlaps(q, keep) || rect_overlaps(r, keep) {
        return q;
    }
    let mut q = q;
    if keep.h == height && keep.x == 0 {
        q.x = q.x.max(keep.x + keep.w);
    } else if keep.h == height && keep.x + keep.w == width {
        q.x = q.x.min(keep.x - q.w);
    } else if keep.w == width && keep.y == 0 {
        q.y = q.y.max(keep.y + keep.h);
    } else if keep.w == width && keep.y + keep.h == height {
        q.y = q.y.min(keep.y - q.h);
    }
    let q = rect_intersect(q, Rect { x: 0, y: 0, w: width, h: height });
    if rect_contains_rect(q, r) { q } else { fbpack_pen_region(r, width, height) }
}
