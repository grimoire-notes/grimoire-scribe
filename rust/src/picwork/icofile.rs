//! The `.ico` container's reader (2.3, issue #236): an icon file's directory, its PNG entries (through stb_image,
//! the decoder the worker has for every other picture: bgimage.rs) and its classic BMP/DIB entries (32, 24, 8, 4
//! and 1 bits). An `.ico` comes from outside Grimoire (an app
//! package's bundled icon, the Package Store's catalog), so this reader runs in the safe loader's picture worker
//! only: it is a module private to picwork.rs, whose worker side alone calls it (the `T` command's `small_one`),
//! so nothing in the UI process or in a background job's writer can (rust/tests/test_sec_invariants.rs holds
//! that). It was appicon.rs's until then.
//!
//! The result is a small grayscale picture: the e-ink panel has no colour, and compositing against white keeps
//! transparent corners clean on the launcher's white tiles.

/// The largest `.ico` file read (apppkg.rs `APP_ICON_MAX`, the same figure: a bundled icon's limit).
pub const ICO_FILE_MAX: usize = 256 * 1024;
/// The largest entry taken, a side.
pub const ICO_SIDE_MAX: usize = 256;
/// The entry looked for: the largest of at most this many pixels a side, else the smallest there is.
pub const ICO_TARGET: u32 = 64;

/// A decoded `.ico` entry, grayscale and already composited over white.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IcoGray {
    pub w: usize,
    pub h: usize,
    /// row-major, `w * h` bytes, 0 black .. 255 white
    pub gray: Vec<u8>,
}

/// Decode an ICO's best small entry: None for a file over ICO_FILE_MAX, a directory that does not read, or an
/// entry that does not decode.
pub fn ico_gray(data: &[u8]) -> Option<IcoGray> {
    if data.len() > ICO_FILE_MAX {
        return None;
    }
    let count = u16_at(data, 4)? as usize;
    if count == 0 || count > 64 || data.len() < 6 + count * 16 {
        return None;
    }
    let mut best: Option<Entry> = None;
    let mut smallest: Option<Entry> = None;
    for i in 0..count {
        let at = 6 + i * 16;
        let w = if data[at] == 0 { 256u32 } else { data[at] as u32 };
        let h = if data[at + 1] == 0 { 256u32 } else { data[at + 1] as u32 };
        let dim = w.max(h);
        if dim == 0 || dim > ICO_SIDE_MAX as u32 {
            continue;
        }
        let entry = Entry {
            dim,
            offset: u32_at(data, at + 12)? as usize,
            size: u32_at(data, at + 8)? as usize,
        };
        if entry.offset.checked_add(entry.size)? > data.len() || entry.size == 0 {
            continue;
        }
        if dim <= ICO_TARGET {
            if best.is_none_or(|b| entry.dim > b.dim) {
                best = Some(entry);
            }
        }
        if smallest.is_none_or(|b| entry.dim < b.dim) {
            smallest = Some(entry);
        }
    }
    let entry = best.or(smallest)?;
    let bytes = &data[entry.offset..entry.offset + entry.size];
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        decode_png(bytes)
    } else {
        decode_dib(bytes)
    }
}

#[derive(Clone, Copy)]
struct Entry {
    dim: u32,
    offset: usize,
    size: usize,
}

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn i32_at(data: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn gray(r: u8, g: u8, b: u8, a: u8) -> u8 {
    let a = a as u32;
    let r = (r as u32 * a + 255 * (255 - a)) / 255;
    let g = (g as u32 * a + 255 * (255 - a)) / 255;
    let b = (b as u32 * a + 255 * (255 - a)) / 255;
    ((r * 77 + g * 150 + b * 29) >> 8) as u8
}

/// A PNG entry: 8 bits a channel, gray or RGB, with or without alpha, at most ICO_SIDE_MAX a side. Its header is
/// read here first (the signature, then the IHDR chunk: width, height, bit depth, colour type), so that what
/// this reader never took stays refused whatever the decoder would make of it: a 16-bit entry or one of fewer
/// than 8 bits, a palette entry, a file whose first chunk is not its header. The samples are stb_image's
/// (bgimage.rs `bgimage_decode_samples`), the file's own, composed over white here by `gray`. A colour key
/// (a tRNS chunk beside gray or RGB), which stb_image hands over as alpha, is not applied: such an entry is
/// opaque, as it was while the png crate decoded it.
fn decode_png(data: &[u8]) -> Option<IcoGray> {
    if data.get(8..16)? != [0, 0, 0, 13, b'I', b'H', b'D', b'R'] {
        return None;
    }
    let side = |at: usize| Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?) as usize);
    let (w, h, depth, colour) = (side(16)?, side(20)?, *data.get(24)?, *data.get(25)?);
    // (colour type: 0 gray, 2 RGB, 4 gray and alpha, 6 RGB and alpha; 3 is a palette)
    if depth != 8 || !matches!(colour, 0 | 2 | 4 | 6) || w == 0 || h == 0 || w > ICO_SIDE_MAX || h > ICO_SIDE_MAX {
        return None;
    }
    let (px, pw, ph, n) = crate::bgimage::bgimage_decode_samples(data)?;
    let (colours, alpha) = (if colour & 2 != 0 { 3 } else { 1 }, colour & 4 != 0);
    let n = n as usize;
    // (the file's own channels, and one more only for its alpha or for a colour key)
    if (pw as usize, ph as usize) != (w, h) || n < colours + alpha as usize || n > colours + 1 || px.len() != w * h * n {
        return None;
    }
    let mut out = Vec::with_capacity(w * h);
    for q in px.chunks_exact(n) {
        let a = if alpha { q[colours] } else { 255 };
        out.push(if colours == 3 { gray(q[0], q[1], q[2], a) } else { gray(q[0], q[0], q[0], a) });
    }
    Some(IcoGray { w, h, gray: out })
}

fn decode_dib(data: &[u8]) -> Option<IcoGray> {
    let header = u32_at(data, 0)? as usize;
    if header < 40 || data.len() < header {
        return None;
    }
    let w = i32_at(data, 4)?;
    let h2 = i32_at(data, 8)?;
    if w <= 0 || h2 == 0 {
        return None;
    }
    let w = w as usize;
    let h = (h2.unsigned_abs() as usize) / 2;
    if h == 0 || w > ICO_SIDE_MAX || h > ICO_SIDE_MAX {
        return None;
    }
    let bpp = u16_at(data, 14)?;
    if !matches!(bpp, 1 | 4 | 8 | 24 | 32) || u32_at(data, 16)? != 0 {
        return None;
    }
    let palette_count = if bpp <= 8 {
        let n = u32_at(data, 32)? as usize;
        if n == 0 { 1usize << bpp } else { n }
    } else {
        0
    };
    if palette_count > 256 {
        return None;
    }
    let palette_off = header;
    let pixel_off = palette_off.checked_add(palette_count * 4)?;
    let stride = ((w * bpp as usize + 31) / 32) * 4;
    let mask_stride = ((w + 31) / 32) * 4;
    let mask_off = pixel_off.checked_add(stride.checked_mul(h)?)?;
    if mask_off.checked_add(mask_stride.checked_mul(h)?)? > data.len() {
        return None;
    }
    let alpha_all_zero = bpp == 32 && {
        let mut zero = true;
        'outer: for y in 0..h {
            for x in 0..w {
                if data[pixel_off + y * stride + x * 4 + 3] != 0 {
                    zero = false;
                    break 'outer;
                }
            }
        }
        zero
    };
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        let sy = h - 1 - y;
        let row = pixel_off + sy * stride;
        for x in 0..w {
            let (r, g, b, mut a) = match bpp {
                32 => {
                    let at = row + x * 4;
                    (data[at + 2], data[at + 1], data[at], data[at + 3])
                }
                24 => {
                    let at = row + x * 3;
                    (data[at + 2], data[at + 1], data[at], 255)
                }
                _ => {
                    let idx = match bpp {
                        8 => data[row + x] as usize,
                        4 => {
                            let byte = data[row + x / 2];
                            ((byte >> if x % 2 == 0 { 4 } else { 0 }) & 0x0f) as usize
                        }
                        _ => {
                            let byte = data[row + x / 8];
                            ((byte >> (7 - (x % 8))) & 1) as usize
                        }
                    };
                    let at = palette_off + idx.min(palette_count - 1) * 4;
                    (data[at + 2], data[at + 1], data[at], 255)
                }
            };
            let transparent = data[mask_off + sy * mask_stride + x / 8] & (0x80 >> (x % 8)) != 0;
            if transparent || (bpp == 32 && alpha_all_zero) {
                a = if transparent { 0 } else { 255 };
            }
            out.push(gray(r, g, b, a));
        }
    }
    Some(IcoGray { w, h, gray: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ico_is_decoded_to_a_small_gray_image() {
        let data = include_bytes!("../../../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico");
        let image = ico_gray(data).expect("the fixture icon decodes");
        assert_eq!((image.w, image.h), (64, 64));
        assert_eq!(image.gray.len(), 64 * 64);
        assert!(image.gray.iter().any(|&g| g < 128));
        assert!(image.gray.iter().any(|&g| g > 128));
    }

    // The PNG entry's decoder until issue #236's review: the png crate, kept here for the comparison below (the
    // tests alone build it).
    fn decode_png_before(data: &[u8]) -> Option<IcoGray> {
        let dec = png::Decoder::new(std::io::Cursor::new(data));
        let mut reader = dec.read_info().ok()?;
        let (w, h, color, depth) = {
            let info = reader.info();
            (info.width as usize, info.height as usize, info.color_type, info.bit_depth)
        };
        if depth != png::BitDepth::Eight || w == 0 || h == 0 || w > ICO_SIDE_MAX || h > ICO_SIDE_MAX {
            return None;
        }
        let mut buf = vec![0u8; reader.output_buffer_size()?];
        let frame = reader.next_frame(&mut buf).ok()?;
        let n = color.samples();
        if n == 0 || frame.buffer_size() < w * h * n {
            return None;
        }
        let mut out = Vec::with_capacity(w * h);
        for i in 0..w * h {
            let at = i * n;
            let px = match color {
                png::ColorType::Grayscale => gray(buf[at], buf[at], buf[at], 255),
                png::ColorType::GrayscaleAlpha => gray(buf[at], buf[at], buf[at], buf[at + 1]),
                png::ColorType::Rgb => gray(buf[at], buf[at + 1], buf[at + 2], 255),
                png::ColorType::Rgba => gray(buf[at], buf[at + 1], buf[at + 2], buf[at + 3]),
                png::ColorType::Indexed => return None,
            };
            out.push(px);
        }
        Some(IcoGray { w, h, gray: out })
    }

    // A PNG of these samples (`n` a pixel), written by the png crate's encoder.
    fn png_of(w: u32, h: u32, color: png::ColorType, depth: png::BitDepth, samples: &[u8], key: Option<&[u8]>, palette: Option<&[u8]>) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, w, h);
            e.set_color(color);
            e.set_depth(depth);
            if let Some(p) = palette {
                e.set_palette(p.to_vec());
            }
            if let Some(k) = key {
                e.set_trns(k.to_vec());
            }
            let mut wr = e.write_header().unwrap();
            wr.write_image_data(samples).unwrap();
        }
        out
    }

    // An `.ico` of one entry.
    fn ico_of(w: u8, h: u8, entry: &[u8]) -> Vec<u8> {
        let mut out = vec![0, 0, 1, 0, 1, 0, w, h, 0, 0, 1, 0, 32, 0];
        out.extend_from_slice(&(entry.len() as u32).to_le_bytes());
        out.extend_from_slice(&22u32.to_le_bytes());
        out.extend_from_slice(entry);
        out
    }

    // A 9 x 7 PNG, RGB and alpha, Adam7 interlaced: pixel (x, y) is r (29x + 3y) % 256, g (7x + 31y) % 256,
    // b (13x + 17y + 5) % 256, alpha [0, 255, 128, 77, 200][(x + 2y) % 5].
    const INTERLACED: [u8; 334] = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 9, 0, 0, 0, 7,
        8, 6, 0, 0, 1, 173, 156, 87, 246, 0, 0, 1, 21, 73, 68, 65, 84, 120, 218, 1, 10, 1, 245, 254,
        0, 0, 0, 5, 0, 232, 56, 109, 77, 0, 116, 28, 57, 200, 0, 12, 124, 73, 77, 128, 152, 125, 128, 244,
        180, 177, 255, 0, 58, 14, 31, 128, 174, 42, 83, 255, 0, 70, 138, 99, 0, 186, 166, 151, 200, 0, 6, 62,
        39, 200, 64, 76, 65, 255, 122, 90, 91, 77, 180, 104, 117, 0, 238, 118, 143, 128, 0, 18, 186, 107, 128, 76,
        200, 133, 200, 134, 214, 159, 255, 192, 228, 185, 77, 250, 242, 211, 0, 0, 29, 7, 18, 255, 87, 21, 44, 77,
        145, 35, 70, 0, 203, 49, 96, 128, 0, 35, 69, 52, 0, 93, 83, 78, 128, 151, 97, 104, 200, 209, 111, 130,
        255, 0, 41, 131, 86, 200, 99, 145, 112, 255, 157, 159, 138, 77, 215, 173, 164, 0, 0, 47, 193, 120, 77, 105,
        207, 146, 0, 163, 221, 172, 128, 221, 235, 198, 200, 0, 3, 31, 22, 128, 32, 38, 35, 77, 61, 45, 48, 200,
        90, 52, 61, 0, 119, 59, 74, 255, 148, 66, 87, 128, 177, 73, 100, 77, 206, 80, 113, 200, 235, 87, 126, 0,
        0, 9, 93, 56, 255, 38, 100, 69, 128, 67, 107, 82, 77, 96, 114, 95, 200, 125, 121, 108, 0, 154, 128, 121,
        255, 183, 135, 134, 128, 212, 142, 147, 77, 241, 149, 160, 200, 0, 15, 155, 90, 0, 44, 162, 103, 255, 73, 169,
        116, 128, 102, 176, 129, 77, 131, 183, 142, 200, 160, 190, 155, 0, 189, 197, 168, 255, 218, 204, 181, 128, 247, 211,
        194, 77, 228, 246, 118, 220, 160, 239, 240, 61, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    // A PNG entry's pixels are the ones the png crate gave, byte for byte, now that stb_image decodes it: gray,
    // gray and alpha, RGB, RGB and alpha (every alpha from clear to solid), an interlaced file, and a colour key,
    // which neither applies. Each also against the gray worked out here from the samples.
    #[test]
    fn a_png_entrys_pixels_are_the_ones_the_png_crate_gave() {
        let (w, h) = (37usize, 29usize);
        let sample = |x: usize, y: usize, c: usize| ((x * 7 + y * 13 + c * 59 + (x * y) % 11) % 256) as u8;
        let alpha = |x: usize, y: usize| match (x + y) % 6 {
            0 => 0,
            1 => 255,
            2 => 1,
            3 => 254,
            _ => ((x * 31 + y * 17) % 256) as u8,
        };
        for (color, n) in [(png::ColorType::Grayscale, 1), (png::ColorType::GrayscaleAlpha, 2), (png::ColorType::Rgb, 3), (png::ColorType::Rgba, 4)] {
            let mut samples = Vec::new();
            let mut want = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    let a = if n % 2 == 0 { alpha(x, y) } else { 255 };
                    let (r, g, b) = if n >= 3 { (sample(x, y, 0), sample(x, y, 1), sample(x, y, 2)) } else { (sample(x, y, 0), sample(x, y, 0), sample(x, y, 0)) };
                    let all = [r, g, b, a];
                    samples.extend_from_slice(match n {
                        1 => &all[..1],
                        2 => &all[2..],
                        3 => &all[..3],
                        _ => &all[..],
                    });
                    want.push(gray(r, g, b, a));
                }
            }
            let entry = png_of(w as u32, h as u32, color, png::BitDepth::Eight, &samples, None, None);
            let before = decode_png_before(&entry).expect("the png crate decodes it");
            assert_eq!((before.w, before.h, &before.gray), (w, h, &want), "{color:?}");
            assert_eq!(decode_png(&entry), Some(before.clone()), "{color:?}");
            assert_eq!(ico_gray(&ico_of(w as u8, h as u8, &entry)), Some(before), "{color:?}");
        }
        // a colour key beside gray and beside RGB: not applied (the pixels of that colour stay as they are)
        let grays: Vec<u8> = (0..w * h).map(|i| (i % 7 * 40) as u8).collect();
        let keyed = png_of(w as u32, h as u32, png::ColorType::Grayscale, png::BitDepth::Eight, &grays, Some(&[0, 80]), None);
        let before = decode_png_before(&keyed).unwrap();
        assert!(before.gray.contains(&gray(80, 80, 80, 255)));
        assert_eq!(decode_png(&keyed), Some(before));
        let rgbs: Vec<u8> = (0..w * h).flat_map(|i| if i % 3 == 0 { [10, 20, 30] } else { [200, (i % 256) as u8, 90] }).collect();
        let keyed = png_of(w as u32, h as u32, png::ColorType::Rgb, png::BitDepth::Eight, &rgbs, Some(&[0, 10, 0, 20, 0, 30]), None);
        let before = decode_png_before(&keyed).unwrap();
        assert_eq!(before.gray[0], gray(10, 20, 30, 255));
        assert_eq!(decode_png(&keyed), Some(before));
        // interlaced
        let before = decode_png_before(&INTERLACED).unwrap();
        let want: Vec<u8> = (0..7usize)
            .flat_map(|y| (0..9usize).map(move |x| (x, y)))
            .map(|(x, y)| gray(((x * 29 + y * 3) % 256) as u8, ((x * 7 + y * 31) % 256) as u8, ((x * 13 + y * 17 + 5) % 256) as u8, [0, 255, 128, 77, 200][(x + 2 * y) % 5]))
            .collect();
        assert_eq!((before.w, before.h, &before.gray), (9, 7, &want));
        assert_eq!(decode_png(&INTERLACED), Some(before));
    }

    // What the reader refused of a PNG entry while the png crate decoded it stays refused: 16 bits a channel, fewer
    // than 8, a palette, more than 256 px a side, a file cut short; and a file whose first chunk is not its header.
    #[test]
    fn a_png_entry_refused_before_is_refused() {
        let refused = |entry: &[u8], what: &str| {
            assert_eq!(decode_png_before(entry), None, "{what}: the png crate's reader took it");
            assert_eq!(decode_png(entry), None, "{what}");
        };
        let sixteen = png_of(8, 8, png::ColorType::Rgb, png::BitDepth::Sixteen, &[90u8; 8 * 8 * 6], None, None);
        refused(&sixteen, "16 bits a channel");
        let sixteen_gray = png_of(8, 8, png::ColorType::GrayscaleAlpha, png::BitDepth::Sixteen, &[90u8; 8 * 8 * 4], None, None);
        refused(&sixteen_gray, "16 bits, gray and alpha");
        for (depth, bytes) in [(png::BitDepth::One, 1usize), (png::BitDepth::Two, 2), (png::BitDepth::Four, 4)] {
            let low = png_of(8, 8, png::ColorType::Grayscale, depth, &vec![0x5au8; 8 * bytes], None, None);
            refused(&low, "fewer than 8 bits");
        }
        let palette = png_of(8, 8, png::ColorType::Indexed, png::BitDepth::Eight, &[1u8; 64], None, Some(&[0, 0, 0, 255, 255, 255]));
        refused(&palette, "a palette");
        let keyed_palette = png_of(8, 8, png::ColorType::Indexed, png::BitDepth::Eight, &[1u8; 64], Some(&[0, 128]), Some(&[0, 0, 0, 255, 255, 255]));
        refused(&keyed_palette, "a palette with alpha");
        let wide = png_of(257, 4, png::ColorType::Grayscale, png::BitDepth::Eight, &[7u8; 257 * 4], None, None);
        refused(&wide, "257 px wide");
        let tall = png_of(4, 257, png::ColorType::Grayscale, png::BitDepth::Eight, &[7u8; 257 * 4], None, None);
        refused(&tall, "257 px high");
        let good = png_of(40, 30, png::ColorType::Rgba, png::BitDepth::Eight, &(0..40 * 30 * 4).map(|i| (i * 37 % 256) as u8).collect::<Vec<u8>>(), None, None);
        assert!(decode_png(&good).is_some() && decode_png(&good) == decode_png_before(&good));
        refused(&good[..good.len() / 2], "cut inside its pixels");
        refused(&good[..20], "cut inside its header");
        refused(&good[..8], "the signature alone");
        // a chunk before the header (the files one vendor's tools write): both readers refuse it
        let mut early = good[..8].to_vec();
        early.extend_from_slice(&[0, 0, 0, 4, b'C', b'g', b'B', b'I', 0x50, 0, 0x20, 0x02, 0x2b, 0xd5, 0xb3, 0x7f]);
        early.extend_from_slice(&good[8..]);
        refused(&early, "a chunk before the header");
        // the sides are the header's: 256 is taken
        let full = png_of(256, 2, png::ColorType::Grayscale, png::BitDepth::Eight, &[7u8; 512], None, None);
        assert_eq!(decode_png(&full), decode_png_before(&full));
        assert!(decode_png(&full).is_some_and(|i| (i.w, i.h) == (256, 2)));
    }

    // The fixture icon (five BMP entries, the tree's only `.ico`): its picture is what it was before the PNG
    // entries changed decoders, by the SHA-256 of its 64 x 64 gray bytes recorded then.
    #[test]
    fn the_fixture_icons_pixels_are_what_they_were() {
        let data = include_bytes!("../../../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico");
        let image = ico_gray(data).unwrap();
        assert_eq!((image.w, image.h), (64, 64));
        assert_eq!(crate::sha256::sha256_of(&image.gray), "23dd71159f28f9af5fdd92b193195b01bd8b207ba754a3ce0575af49966803a6");
    }

    // The limits appicon.rs held: a file over 256 KiB is not read at all, and an entry over 256 px a side is no
    // entry (its directory byte 0 says 256, which is taken; a PNG entry that is larger inside is refused).
    #[test]
    fn the_limits_are_the_ones_the_launcher_had() {
        assert_eq!((ICO_FILE_MAX, ICO_SIDE_MAX, ICO_TARGET), (256 * 1024, 256, 64));
        let data = include_bytes!("../../../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico");
        let mut long = data.to_vec();
        long.resize(ICO_FILE_MAX + 1, 0);
        assert!(ico_gray(&long).is_none());
        // (the directory alone: every entry it names lies past the end)
        assert!(ico_gray(&data[..100]).is_none());
        assert!(ico_gray(b"").is_none());
        assert!(ico_gray(&[0, 0, 1, 0, 0, 0]).is_none());
    }
}
