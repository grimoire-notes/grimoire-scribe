//! Icons for installed apps: a built-in Grimoire glyph, or the `.ico` an app package bundles.
//!
//! An ICO is read once when the launcher asks for it. PNG entries (the usual 256 px entry) and
//! classic BMP/DIB entries (32, 24, 8, 4 and 1 bits) are accepted. The result is a small grayscale
//! bitmap: the e-ink panel has no colour, and compositing against white keeps transparent corners
//! clean on the launcher's white tiles.

use crate::prelude::*;

const ICON_MAX: usize = 256 * 1024;
const ICON_SIDE_MAX: usize = 256;
const ICON_TARGET: u32 = 64;

/// A decoded `.ico` entry, grayscale and already composited over white.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconImage {
    pub w: u16,
    pub h: u16,
    /// row-major, `w * h` bytes, 0 black .. 255 white
    pub gray: Vec<u8>,
}

/// What a launcher draws for an app: one of Grimoire's glyphs, or an app's own `.ico`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppIcon {
    pub glyph: IconKind,
    pub image: Option<IconImage>,
}

impl Default for AppIcon {
    fn default() -> Self {
        Self { glyph: IconKind::Grid, image: None }
    }
}

impl AppIcon {
    pub fn glyph(glyph: IconKind) -> Self {
        Self { glyph, image: None }
    }

    /// The icon of an installed package. A `.ico` that is missing or unreadable falls back to the
    /// manifest's built-in name (or the grid), so an old or hand-installed package still launches.
    pub fn for_install(it: &WidgetInstall) -> Self {
        let glyph = if it.icon.trim().is_empty() { IconKind::Grid } else { widget_icon(&it.icon) };
        if let Some(path) = crate::apppkg::app_package_icon_path(it) {
            if let Some(image) = load_ico(&path) {
                return Self { glyph, image: Some(image) };
            }
        }
        Self { glyph, image: None }
    }

    pub fn draw(&self, c: &mut Canvas, r: Rect, ink: u8) {
        if let Some(image) = &self.image {
            image.draw(c, r);
        } else {
            ui_icon(c, self.glyph, r, ink);
        }
    }
}

impl IconImage {
    fn draw(&self, c: &mut Canvas, r: Rect) {
        if self.w == 0 || self.h == 0 || r.w <= 0 || r.h <= 0 {
            return;
        }
        let iw = self.w as i32;
        let ih = self.h as i32;
        let scale = (r.w as f32 / iw as f32).min(r.h as f32 / ih as f32);
        let tw = ((iw as f32 * scale).round() as i32).max(1).min(r.w);
        let th = ((ih as f32 * scale).round() as i32).max(1).min(r.h);
        let x0 = r.x + (r.w - tw) / 2;
        let y0 = r.y + (r.h - th) / 2;
        for dy in 0..th {
            let sy = (dy as usize * self.h as usize / th as usize).min(self.h as usize - 1);
            let row = sy * self.w as usize;
            let mut run_x = 0;
            let mut run_gray = 0u8;
            for dx in 0..tw {
                let sx = (dx as usize * self.w as usize / tw as usize).min(self.w as usize - 1);
                let gray = self.gray[row + sx];
                if dx == 0 {
                    run_gray = gray;
                } else if gray != run_gray {
                    canvas_fill(c, Rect::new(x0 + run_x, y0 + dy, dx - run_x, 1), run_gray);
                    run_x = dx;
                    run_gray = gray;
                }
            }
            canvas_fill(c, Rect::new(x0 + run_x, y0 + dy, tw - run_x, 1), run_gray);
        }
    }
}

fn load_ico(path: &str) -> Option<IconImage> {
    let data = read_file_max(path, ICON_MAX)?;
    decode_ico(&data)
}

/// Decode an ICO's best small entry. Public for the unit tests and the package manager.
pub fn decode_ico(data: &[u8]) -> Option<IconImage> {
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
        if dim == 0 || dim > ICON_SIDE_MAX as u32 {
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
        if dim <= ICON_TARGET {
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

fn decode_png(data: &[u8]) -> Option<IconImage> {
    let dec = png::Decoder::new(std::io::Cursor::new(data));
    let mut reader = dec.read_info().ok()?;
    let (w, h, color, depth) = {
        let info = reader.info();
        (info.width as usize, info.height as usize, info.color_type, info.bit_depth)
    };
    if depth != png::BitDepth::Eight || w == 0 || h == 0 || w > ICON_SIDE_MAX || h > ICON_SIDE_MAX {
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
    Some(IconImage { w: w as u16, h: h as u16, gray: out })
}

fn decode_dib(data: &[u8]) -> Option<IconImage> {
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
    if h == 0 || w > ICON_SIDE_MAX || h > ICON_SIDE_MAX {
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
    Some(IconImage { w: w as u16, h: h as u16, gray: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ico_is_decoded_to_a_small_gray_image() {
        let data = include_bytes!("../../tests/fixtures/apppkg/packages/apps/com.example.fixture/icon.ico");
        let image = decode_ico(data).expect("the fixture icon decodes");
        assert_eq!((image.w, image.h), (64, 64));
        assert_eq!(image.gray.len(), 64 * 64);
        assert!(image.gray.iter().any(|&g| g < 128));
        assert!(image.gray.iter().any(|&g| g > 128));
    }
}
