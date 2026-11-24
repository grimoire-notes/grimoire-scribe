//! The `.ico` container's reader (2.3, issue #236): an icon file's directory, its PNG entries (through the png
//! crate) and its classic BMP/DIB entries (32, 24, 8, 4 and 1 bits). An `.ico` comes from outside Grimoire (an app
//! package's bundled icon, the Package Store's catalog), so this reader runs in the safe loader's picture worker
//! only (picwork.rs, the `T` command): nothing in the UI process or in a background job's writer calls it
//! (rust/tests/test_sec_invariants.rs holds that). It was appicon.rs's until then.
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

fn decode_png(data: &[u8]) -> Option<IcoGray> {
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
        let data = include_bytes!("../../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico");
        let image = ico_gray(data).expect("the fixture icon decodes");
        assert_eq!((image.w, image.h), (64, 64));
        assert_eq!(image.gray.len(), 64 * 64);
        assert!(image.gray.iter().any(|&g| g < 128));
        assert!(image.gray.iter().any(|&g| g > 128));
    }

    // The limits appicon.rs held: a file over 256 KiB is not read at all, and an entry over 256 px a side is no
    // entry (its directory byte 0 says 256, which is taken; a PNG entry that is larger inside is refused).
    #[test]
    fn the_limits_are_the_ones_the_launcher_had() {
        assert_eq!((ICO_FILE_MAX, ICO_SIDE_MAX, ICO_TARGET), (256 * 1024, 256, 64));
        let data = include_bytes!("../../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico");
        let mut long = data.to_vec();
        long.resize(ICO_FILE_MAX + 1, 0);
        assert!(ico_gray(&long).is_none());
        // (the directory alone: every entry it names lies past the end)
        assert!(ico_gray(&data[..100]).is_none());
        assert!(ico_gray(b"").is_none());
        assert!(ico_gray(&[0, 0, 1, 0, 0, 0]).is_none());
    }
}
