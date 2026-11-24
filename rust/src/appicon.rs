//! Icons for installed apps: a built-in Grimoire glyph, or the `.ico` an app package bundles.
//!
//! An ICO is read once when the launcher asks for it. PNG entries (the usual 256 px entry) and
//! classic BMP/DIB entries (32, 24, 8, 4 and 1 bits) are accepted. The result is a small grayscale
//! bitmap: the e-ink panel has no colour, and compositing against white keeps transparent corners
//! clean on the launcher's white tiles.

use crate::prelude::*;

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
    let data = read_file_max(path, crate::icofile::ICO_FILE_MAX)?;
    let i = crate::icofile::ico_gray(&data)?;
    Some(IconImage { w: i.w as u16, h: i.h as u16, gray: i.gray })
}
