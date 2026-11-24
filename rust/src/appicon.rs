//! Icons for installed apps: a built-in Grimoire glyph, or the `.ico` an app package bundles.
//!
//! A bundled `.ico` comes from outside Grimoire, so nothing here reads or decodes it (2.3, issue #236): the safe
//! loader's picture worker does (pkgpics.rs, picwork.rs `T`, picwork/icofile.rs), and what the launcher draws is the gray
//! picture kept in grimoire/.cache/packages/icons, read back and checked (`pkgpics::picture_cached`). The result
//! is a small grayscale bitmap: the e-ink panel has no colour, and compositing against white keeps transparent
//! corners clean on the launcher's white tiles.
//!
//! A draw only looks: `AppIcon::for_install` stats the icon's file and takes what a small table of the UI's holds
//! for it (`KEPT`: the icon's picture, read from the cache the first time it is asked for; or nothing yet; or none
//! to be had). The Library's tick (`app_icons_step`) asks the background job for the icons the cache has nothing
//! for, a few at a time, and takes a picture in when its job has ended; the launcher on screen is then drawn again.
//! Until then, and for good when the worker refuses the file, no worker can be had or the session's sandbox is
//! broken, the launcher shows the manifest's built-in icon name (or the grid).

use crate::pkgpics::{picture_cached, picture_request, Ask, Cached, PicForm, PicSrc};
use crate::prelude::*;
use std::cell::RefCell;

/// A bundled icon's picture as the worker made it: one `.ico` entry, grayscale and already composited over white.
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

    /// The icon of an installed package, as it can be drawn now. A `.ico` that is missing, not made into its
    /// picture yet, or one the worker refuses falls back to the manifest's built-in name (or the grid), so an
    /// old or hand-installed package still launches. Nothing of the `.ico` is read here (`bundled_icon`).
    pub fn for_install(it: &WidgetInstall) -> Self {
        let glyph = if it.icon.trim().is_empty() { IconKind::Grid } else { widget_icon(&it.icon) };
        let image = crate::apppkg::app_package_icon_path(it).and_then(|path| bundled_icon(&path));
        Self { glyph, image }
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
    /// A gray picture from the cache (`picture_cached`: at most 256 px a side) as the launcher keeps it.
    fn of(c: &Canvas) -> IconImage {
        let (w, h) = (c.w.max(0) as usize, c.h.max(0) as usize);
        let mut gray = Vec::with_capacity(w * h);
        for y in 0..h {
            gray.extend_from_slice(&c.px[y * c.stride as usize..y * c.stride as usize + w]);
        }
        IconImage { w: w as u16, h: h as u16, gray }
    }

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

/// Where a bundled icon the launcher asked for is.
enum Icon {
    /// the cache has nothing for it: its job is to be asked for (`app_icons_step`)
    ToAsk,
    /// its job is queued or running (the job's path)
    Job(String),
    /// its picture, from the cache
    There(IconImage),
    /// none to be had: the worker refused it (its mark), its job ended with nothing, or none can be asked for.
    /// The built-in icon, until the file changes or Grimoire starts again
    Nothing,
}

struct Kept {
    src: PicSrc,
    state: Icon,
}

/// The table holds at most this many icons (twice the apps that can be installed): past it the oldest goes.
const KEPT_MAX: usize = 2 * crate::apppkg::APP_PKG_COUNT_MAX;

thread_local! {
    /// The bundled icons the launcher asked for, by their file as it was (the UI thread's).
    static KEPT: RefCell<Vec<Kept>> = const { RefCell::new(Vec::new()) };
}

/// What the cache holds for `src`: its picture, its mark, or nothing yet.
fn look(src: &PicSrc) -> Icon {
    match picture_cached(src) {
        Cached::Picture(c) => Icon::There(IconImage::of(&c)),
        Cached::None => Icon::Nothing,
        Cached::Nothing => Icon::ToAsk,
    }
}

/// The picture of the bundled icon at `path` when the table has it: one stat of the file (no byte of it is
/// read), and for a file new to the table, or changed since, one look in the cache. None while its picture is
/// to come and when there is none.
fn bundled_icon(path: &str) -> Option<IconImage> {
    let src = PicSrc::of(path, PicForm::AppIcon)?;
    KEPT.with(|k| {
        let mut k = k.borrow_mut();
        let i = match k.iter().position(|e| e.src.rel == src.rel) {
            Some(i) if k[i].src == src => i,
            Some(i) => {
                k[i] = Kept { state: look(&src), src };
                i
            }
            None => {
                if k.len() >= KEPT_MAX {
                    k.remove(0);
                }
                k.push(Kept { state: look(&src), src });
                k.len() - 1
            }
        };
        match &k[i].state {
            Icon::There(image) => Some(image.clone()),
            _ => None,
        }
    })
}

/// One round, at each tick of the Library: the icons the cache had nothing for are asked for (a job each, a few at
/// a time: `picture_request`), and those whose job has ended are looked up again: the picture, or none for this
/// run. True when a picture came: the launcher on screen is to be drawn again.
pub fn app_icons_step() -> bool {
    KEPT.with(|k| {
        let mut came = false;
        for e in k.borrow_mut().iter_mut() {
            match &e.state {
                Icon::ToAsk => match picture_request(&e.src) {
                    Ask::Job(job) => e.state = Icon::Job(job),
                    Ask::Later => {}
                    Ask::No => e.state = Icon::Nothing,
                },
                Icon::Job(job) if !bg_job_pending(BgKind::PackagePictures, "", job) => {
                    e.state = match picture_cached(&e.src) {
                        Cached::Picture(c) => {
                            came = true;
                            Icon::There(IconImage::of(&c))
                        }
                        _ => Icon::Nothing,
                    };
                }
                _ => {}
            }
        }
        came
    })
}

/// How many bundled icons are still to come: to be asked for, or their job queued or running.
pub fn app_icons_coming() -> usize {
    KEPT.with(|k| k.borrow().iter().filter(|e| matches!(e.state, Icon::ToAsk | Icon::Job(_))).count())
}

/// The table emptied (tests, never the Kindle's build): the next draw looks in the cache again, as after a start.
#[cfg(any(test, feature = "sim"))]
pub fn app_icons_forget() {
    KEPT.with(|k| k.borrow_mut().clear());
}
