//! What's new, the full-screen browser of every version's notes (docs/whatsnew/VERSION.md, built in)
//! and the owner's signature. The Apps page's foot button opens it, and so does the first start
//! after every update, once (the owner's rule of 2026-11-05: "no don't show this again, just a
//! "Dismiss" and every update with changes by default has at least a basic what's new"; store.rs,
//! whatsnew_due). Rust only (2.0.0).
//!
//! The browser is the Settings frame: What's new and its X at the top, every version down the left
//! in Settings' rows, and the selected version's notes in the reader at the right. It opens on this
//! version's notes (GRIMOIRE_VERSION), or the newest when this version has none (current). A tap on
//! a version selects its notes; the notes are set closer first, then smaller (fit_in), to fit the
//! reader whole, with the signature at their end (2.3.0). There is no Read all layer: every version
//! is already in the sidebar, and the X or Esc closes the browser.
//!
//! The notes, a file for each version (2.1.0): build.rs lists docs/whatsnew/*.md, each named for its
//! version (VERSION.md), with the date CHANGELOG.md's "## VERSION (YYYY-MM-DD)" heading gives it, into
//! NOTE_FILES; a new release's notes are a new file there, and no code changes. The archive (notes)
//! puts them newest first by version (update_version_cmp: 2.1.0, 2.0.0, 2.0.0-rc.2, 2.0.0-beta.2,
//! 2.0.0-beta.1, the numbers compared as numbers), one entry for each note: versions whose notes are
//! the same, block for block, share one entry ("2.0.0, 2.0.0-beta.2") at the newest one's place, with
//! its date. 2.0.0-beta.2 has no file: it shipped 2.0.0-beta.1's notes, and its changes are in
//! 2.0.0's.
//!
//! A note is a small Markdown: "# " the title, "## " a heading, "- " a bullet, other lines a
//! paragraph (lines joined until a blank one), and a line "{signature}" where the signature goes.
//! The notes are the owner's text and are not translated; the browser's own words are.

use crate::prelude::*;
use std::cell::RefCell;
use std::sync::OnceLock;

/// One version's notes as built in: docs/whatsnew/VERSION.md, and the date CHANGELOG.md's heading for
/// the version gives it ("2026-11-05"; "unreleased" for "(unreleased)"; "" without one).
#[derive(Clone, Copy, Debug)]
pub struct NoteFile {
    pub version: &'static str,
    pub date: &'static str,
    pub md: &'static str,
}

// NOTE_FILES: build.rs's list of docs/whatsnew, in file name order (never empty).
include!(concat!(env!("OUT_DIR"), "/whatsnew.rs"));

/// One entry of the archive: a note, and every version that has it, newest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub versions: Vec<&'static str>,
    /// the newest one's date (NoteFile::date)
    pub date: &'static str,
    pub md: &'static str,
}

impl Entry {
    /// Its versions as the list names them: "2.0.0", or "2.0.0, 2.0.0-beta.2" for notes two shared.
    pub fn label(&self) -> String {
        self.versions.join(", ")
    }
}

/// The notes newest first by version, one entry for each note: a note that is another's block for
/// block (whatever its line breaks and blank lines) adds its version to the first one's entry.
pub fn archive(files: &[NoteFile]) -> Vec<Entry> {
    let mut f: Vec<&NoteFile> = files.iter().collect();
    f.sort_by(|a, b| update_version_cmp(b.version, a.version).cmp(&0).then(a.version.cmp(b.version)));
    let mut out: Vec<(Vec<Block>, Entry)> = Vec::new();
    for n in f {
        let bl = about_blocks(n.md);
        match out.iter_mut().find(|(b, _)| *b == bl) {
            Some((_, e)) => e.versions.push(n.version),
            None => out.push((bl, Entry { versions: vec![n.version], date: n.date, md: n.md })),
        }
    }
    out.into_iter().map(|(_, e)| e).collect()
}

/// The built-in notes' archive (archive of NOTE_FILES).
pub fn notes() -> &'static [Entry] {
    static ARCHIVE: OnceLock<Vec<Entry>> = OnceLock::new();
    // (the notes call the device a Kindle, as every text does: i18n.rs, the device's name)
    ARCHIVE.get_or_init(|| {
        let mut entries = archive(NOTE_FILES);
        for e in &mut entries {
            e.md = crate::i18n::i18n_named(e.md);
        }
        entries
    })
}

/// The entry with version's notes, else the newest (0): the browser opens a version without notes
/// of its own on the newest there are.
pub fn current(entries: &[Entry], version: &str) -> usize {
    entries.iter().position(|e| e.versions.contains(&version)).unwrap_or(0)
}

/// A note's line in the list: its first heading or first line (its title, when it has one).
pub fn summary(md: &str) -> String {
    about_blocks(md)
        .into_iter()
        .find_map(|b| match b {
            Block::Title(t) | Block::Heading(t) | Block::Bullet(t) | Block::Para(t) => Some(t),
            Block::Signature => None,
        })
        .unwrap_or_default()
}

/// A note's date as the list and the sheet show it: the changelog's, or "unreleased" in the
/// language in use.
pub fn date_label(date: &str) -> &str {
    if date == "unreleased" {
        // TRANSLATORS: What's new's list of versions (Read all), in a version's row and above its
        // notes: in place of the date of a version not released yet, a test build (34 px, at the
        // right: at most 400 px).
        tr("unreleased")
    } else {
        date
    }
}

/// The owner's signature, with the bat drawn beside it: a P5 PGM of the page's ink as Grimoire drew
/// it (the notebook "2.0b1 sig", page 1, cut to its ink), laid over the sheet so that only the ink
/// shows. Empty: a line to sign on shows where it goes.
static SIGNATURE: &[u8] = include_bytes!("../../assets/signature.pgm");

/// The signature's place at most max_w by max_h: the signature's own shape, as large as that holds
/// (never larger than drawn); without one, the whole box (the line to sign on).
fn sig_box(max_w: i32, max_h: i32) -> (i32, i32) {
    match pgm(SIGNATURE) {
        Some((sw, sh, _)) => {
            let k = (max_w as f32 / sw as f32).min(max_h as f32 / sh as f32).min(1.0);
            (((sw as f32 * k) as i32).max(1), ((sh as f32 * k) as i32).max(1))
        }
        None => (max_w, max_h),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Title(String),
    Heading(String),
    Bullet(String),
    Para(String),
    Signature,
}

fn flush(para: &mut String, out: &mut Vec<Block>) {
    if !para.is_empty() {
        out.push(Block::Para(std::mem::take(para)));
    }
}

/// A note's blocks, in order.
pub fn about_blocks(md: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut para = String::new();
    for l in md.lines() {
        let t = l.trim();
        if t.is_empty() {
            flush(&mut para, &mut out);
        } else if let Some(x) = t.strip_prefix("## ") {
            flush(&mut para, &mut out);
            out.push(Block::Heading(x.trim().to_string()));
        } else if let Some(x) = t.strip_prefix("# ") {
            flush(&mut para, &mut out);
            out.push(Block::Title(x.trim().to_string()));
        } else if let Some(x) = t.strip_prefix("- ") {
            flush(&mut para, &mut out);
            out.push(Block::Bullet(x.trim().to_string()));
        } else if t == "{signature}" {
            flush(&mut para, &mut out);
            out.push(Block::Signature);
        } else {
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(t);
        }
    }
    flush(&mut para, &mut out);
    out
}

/// Marks a line never starts with (Japanese's closing marks and small kana), and marks it never
/// ends with (the opening ones): no break before the first, nor after the second.
const NO_BREAK_BEFORE: &str = "、。，．・：；？！ー々ゝゞヽヾ）〕］｝〉》」』】〙〗〟’”｠»ぁぃぅぇぉっゃゅょゎァィゥェォッャュョヮヵヶ";
const NO_BREAK_AFTER: &str = "（〔［｛〈《「『【〘〖〝‘“｟«";

/// The text's pieces a line may break between, each with whether a space comes before it: its
/// words, and in Japanese or Chinese, which have no spaces, each character (font.rs cjk), never a
/// closing mark apart from what comes before it nor an opening one from what follows.
fn pieces(text: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        let (mut cur, mut spaced, mut prev) = (String::new(), true, None::<char>);
        for ch in word.chars() {
            if let Some(p) = prev {
                if (crate::font::cjk(ch as u32) || crate::font::cjk(p as u32)) && !NO_BREAK_BEFORE.contains(ch) && !NO_BREAK_AFTER.contains(p) {
                    out.push((std::mem::take(&mut cur), spaced));
                    spaced = false;
                }
            }
            cur.push(ch);
            prev = Some(ch);
        }
        out.push((cur, spaced));
    }
    out
}

/// Lines of text broken at spaces to fit w, and between the characters of Japanese or Chinese
/// (pieces). A word wider than w on its own (a long link) starts a line and is broken between its
/// characters, as many to a line as fit (at least one), so no line is wider than w and nothing is
/// cut short.
fn wrap(f: FontFace, px: f32, text: &str, w: i32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for (piece, spaced) in pieces(text) {
        let trial = if line.is_empty() { piece.clone() } else if spaced { format!("{line} {piece}") } else { format!("{line}{piece}") };
        if font_measure(f, px, &trial) <= w {
            line = trial;
            continue;
        }
        if !line.is_empty() {
            out.push(std::mem::take(&mut line));
        }
        let mut rest = piece.as_str();
        while font_measure(f, px, rest) > w {
            let mut cut = rest.chars().next().map_or(rest.len(), char::len_utf8);
            while let Some(c) = rest[cut..].chars().next() {
                if font_measure(f, px, &rest[..cut + c.len_utf8()]) > w {
                    break;
                }
                cut += c.len_utf8();
            }
            out.push(rest[..cut].to_string());
            rest = &rest[cut..];
        }
        line = rest.to_string();
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

fn line_h(f: FontFace, px: f32, lead: f32) -> i32 {
    ((font_ascent(f, px) + font_descent(f, px)) as f32 * lead).round() as i32
}

#[derive(Clone, Debug)]
enum Ink {
    Text { s: String, f: FontFace, px: f32 },
    /// a bullet's dot, at this times its size on the card
    Dot(f32),
    Sign,
}

#[derive(Clone, Debug)]
struct Item {
    r: Rect,
    ink: Ink,
}

/// One row of the laid-out notes: its items' rects are relative to the row's top.
struct Row {
    h: i32,
    /// space above it, dropped at a page's top and under a heading
    gap: i32,
    /// a heading's last line: it goes to the next page rather than end one alone
    keep_next: bool,
    items: Vec<Item>,
}

struct Look {
    body: f32,
    head_f: FontFace,
    head_px: f32,
    lead: f32,
    /// a bullet's text starts this far in
    indent: i32,
    head_gap: i32,
    item_gap: i32,
    para_gap: i32,
    sign_w: i32,
    sign_h: i32,
    /// a bullet's dot, at this times the card's
    dot: f32,
}

fn rows(bl: &[Block], w: i32, k: &Look) -> Vec<Row> {
    let body_lh = line_h(FontFace::Regular, k.body, k.lead);
    let head_lh = line_h(k.head_f, k.head_px, k.lead);
    let text = |s: String, f: FontFace, px: f32, x: i32, lh: i32, w: i32| Item {
        r: Rect::new(x, 0, w, lh),
        ink: Ink::Text { s, f, px },
    };
    let mut out: Vec<Row> = Vec::new();
    let mut after_head = false;
    for b in bl.iter() {
        let first = out.len();
        match b {
            Block::Title(_) => continue, // (the sheet's title)
            Block::Heading(t) => {
                let lines = wrap(k.head_f, k.head_px, t, w);
                let n = lines.len();
                for (i, l) in lines.into_iter().enumerate() {
                    out.push(Row {
                        h: head_lh,
                        gap: if i == 0 { k.head_gap } else { 0 },
                        keep_next: i + 1 == n,
                        items: vec![text(l, k.head_f, k.head_px, 0, head_lh, w)],
                    });
                }
            }
            Block::Bullet(t) => {
                for (i, l) in wrap(FontFace::Regular, k.body, t, w - k.indent).into_iter().enumerate() {
                    let mut items = vec![text(l, FontFace::Regular, k.body, k.indent, body_lh, w - k.indent)];
                    if i == 0 {
                        items.push(Item { r: Rect::new(0, 0, k.indent, body_lh), ink: Ink::Dot(k.dot) });
                    }
                    out.push(Row { h: body_lh, gap: if i == 0 { k.item_gap } else { 0 }, keep_next: false, items });
                }
            }
            Block::Para(t) => {
                for (i, l) in wrap(FontFace::Regular, k.body, t, w).into_iter().enumerate() {
                    out.push(Row {
                        h: body_lh,
                        gap: if i == 0 { k.para_gap } else { 0 },
                        keep_next: false,
                        items: vec![text(l, FontFace::Regular, k.body, 0, body_lh, w)],
                    });
                }
            }
            Block::Signature => {
                // (signed under the closing, at the left as a letter is; the closing never ends a page
                // without it)
                if let Some(prev) = out.last_mut() {
                    prev.keep_next = true;
                }
                let (sw, sh) = sig_box(k.sign_w.min(w), k.sign_h);
                out.push(Row { h: sh, gap: k.para_gap, keep_next: false, items: vec![Item { r: Rect::new(0, 0, sw, sh), ink: Ink::Sign }] });
            }
        }
        if after_head {
            if let Some(r) = out.get_mut(first) {
                r.gap = 0; // (what follows a heading sits right under it)
            }
        }
        after_head = matches!(b, Block::Heading(_));
    }
    out
}

/// The rows placed on pages of height h (items' rects relative to the page's top left).
fn paginate(rows: &[Row], h: i32) -> Vec<Vec<Item>> {
    let mut pages: Vec<Vec<Item>> = vec![Vec::new()];
    let mut y = 0;
    for (i, row) in rows.iter().enumerate() {
        let mut gap = if y == 0 { 0 } else { row.gap };
        let next = if row.keep_next { rows.get(i + 1).map_or(0, |n| n.gap + n.h) } else { 0 };
        if y > 0 && y + gap + row.h + next > h {
            pages.push(Vec::new());
            y = 0;
            gap = 0;
        }
        for it in &row.items {
            let mut it = it.clone();
            it.r.y += y + gap;
            pages.last_mut().unwrap().push(it);
        }
        y += gap + row.h;
    }
    pages
}

fn rows_h(rows: &[Row]) -> i32 {
    rows.iter().enumerate().map(|(i, r)| r.h + if i == 0 { 0 } else { r.gap }).sum()
}

fn gothic() -> bool {
    ui_theme() == UiTheme::Gothic
}

fn draw_items(c: &mut Canvas, items: &[Item], at: Rect) {
    for it in items {
        let r = Rect::new(at.x + it.r.x, at.y + it.r.y, it.r.w, it.r.h);
        match &it.ink {
            Ink::Text { s, f, px } => font_draw_left_name(c, *f, *px, r, s, 0),
            Ink::Dot(k) => {
                let (cx, cy) = (r.x as f32 + r.w as f32 * 0.4, r.y as f32 + r.h as f32 / 2.0);
                if gothic() {
                    canvas_lozenge(c, cx, cy, ui_px(10.0 * k) as f32, 0);
                } else {
                    canvas_disc_aa(c, cx, cy, ui_px(8.0 * k) as f32, 0, 1.0);
                }
            }
            Ink::Sign => draw_signature(c, r),
        }
    }
}

/// A P5 PGM's size and pixels (maxval 255, no comments), or None.
fn pgm(d: &[u8]) -> Option<(i32, i32, &[u8])> {
    if !d.starts_with(b"P5") {
        return None;
    }
    let mut at = 2;
    let mut v = [0usize; 3];
    for x in v.iter_mut() {
        while d.get(at)?.is_ascii_whitespace() {
            at += 1;
        }
        let s = at;
        while d.get(at).is_some_and(|c| c.is_ascii_digit()) {
            at += 1;
        }
        *x = std::str::from_utf8(&d[s..at]).ok()?.parse().ok()?;
    }
    let (w, h) = (v[0], v[1]);
    if v[2] != 255 || w == 0 || h == 0 || w > 4096 || h > 4096 {
        return None;
    }
    // (one white space after the maxval, then the pixels)
    let px = d.get(at + 1..at + 1 + w * h)?;
    Some((w as i32, h as i32, px))
}

/// The signature in r: as large as r holds, at its top left, each pixel the average of the ones it
/// covers, laid over what is there (the darker wins). Without one, a line to sign on.
fn draw_signature(c: &mut Canvas, r: Rect) {
    let Some((sw, sh, px)) = pgm(SIGNATURE) else {
        let base = r.y + r.h - ui_px(20.0);
        canvas_hline(c, r.x, r.x + r.w, base, ui_px(3.0), 0);
        let (x, y, s) = (r.x as f32 + ui_px(14.0) as f32, base as f32 - ui_px(28.0) as f32, ui_px(12.0) as f32);
        canvas_line_aa(c, x - s, y - s, x + s, y + s, ui_px(3.0) as f32, 0);
        canvas_line_aa(c, x - s, y + s, x + s, y - s, ui_px(3.0) as f32, 0);
        font_draw_centered(c, FontFace::Regular, ui_small_size(), Rect::new(r.x, base - ui_px(80.0), r.w, ui_px(70.0)), "signature", 150);
        return;
    };
    let k = (r.w as f32 / sw as f32).min(r.h as f32 / sh as f32).min(1.0);
    let (dw, dh) = (((sw as f32 * k) as i32).max(1), ((sh as f32 * k) as i32).max(1));
    let (dx, dy) = (r.x, r.y);
    let (x0c, y0c) = (c.clip.x.max(0), c.clip.y.max(0));
    let (x1c, y1c) = ((c.clip.x + c.clip.w).min(c.w), (c.clip.y + c.clip.h).min(c.h));
    for y in 0..dh {
        let (y0, y1) = (y * sh / dh, ((y + 1) * sh / dh).max(y * sh / dh + 1));
        for x in 0..dw {
            let (cx, cy) = (dx + x, dy + y);
            if cx < x0c || cy < y0c || cx >= x1c || cy >= y1c {
                continue;
            }
            let (x0, x1) = (x * sw / dw, ((x + 1) * sw / dw).max(x * sw / dw + 1));
            let mut sum = 0u32;
            for yy in y0..y1 {
                for xx in x0..x1 {
                    sum += px[(yy * sw + xx) as usize] as u32;
                }
            }
            let g = (sum / ((y1 - y0) * (x1 - x0)) as u32) as u8;
            let i = (cy * c.stride + cx) as usize;
            if g < c.px[i] {
                c.px[i] = g;
            }
        }
    }
}

/// Where What's new's controls are (empty when not shown), and what it shows (docs/SIM.md,
/// library.about.*): the reader's pages and the page shown; the versions sidebar's page and pages,
/// its rows on that page (each with its entry in notes()) and the selected row; whether the reader's
/// page has the signature; its body type's size and leading; and the screen.
#[derive(Clone, Debug, Default)]
pub struct AboutHits {
    pub close: Rect,
    pub prev: Rect,
    pub next: Rect,
    /// the versions sidebar's page arrows, when every version does not fit on one page
    pub sidebar_prev: Rect,
    pub sidebar_next: Rect,
    pub sidebar_page: i32,
    pub sidebar_pages: i32,
    pub rows: Vec<(Rect, usize)>,
    /// the selected row on the sidebar page shown (from 1; 0 none)
    pub marked: i32,
    pub signed: bool,
    pub area: Rect,
    pub pages: i32,
    /// the page shown (from 0), kept in range
    pub page: i32,
    /// the notes' body text size and leading (fit_in); 0 while the browser is not drawn
    pub body_px: f32,
    pub lead: f32,
}

/// Draws What's new as the app's full-screen browser: the title and X of the Settings frame, every
/// version down the left in Settings' rows, and the selected version's notes in the reader at the
/// right. `note` is the selected entry in `notes()`, `page` its reader page and `sidebar_page` the
/// page of versions shown; pages count from 0 and are kept in range.
pub fn about_draw(c: &mut Canvas, aw: i32, ah: i32, note: usize, page: i32, sidebar_page: i32) -> AboutHits {
    draw_browser(c, aw, ah, notes(), note, page, sidebar_page)
}

/// The full-screen What's new browser (`about_draw`): the Settings page's chrome, then the versions
/// sidebar and the reader. Selecting a version changes the reader, never the frame; the X closes
/// What's new.
fn draw_browser(c: &mut Canvas, aw: i32, ah: i32, all: &[Entry], note: usize, page: i32, sidebar_page: i32) -> AboutHits {
    let note = note.min(all.len().saturating_sub(1));
    let page_rect = crate::library::settings_page(c, aw, ah, whats_new(), false, false);
    let content = page_rect.content;
    let side_w = (aw * 30 / 100).clamp(ui_px(320.0), ui_px(560.0));
    let side_area = Rect::new(content.x, content.y, side_w, content.h - ui_px(20.0));
    let mut sidebar = ui_row_page(side_area, all.len(), sidebar_page, ui_px(120.0), ui_px(72.0));
    // The version selected comes on screen when the list was left on another page.
    if !sidebar.rows.is_empty() && !(sidebar.first..sidebar.first + sidebar.rows.len()).contains(&note) {
        let per = sidebar.rows.len().max(1);
        sidebar = ui_row_page(side_area, all.len(), (note / per) as i32, ui_px(120.0), ui_px(72.0));
    }
    let mut hits = AboutHits {
        close: page_rect.close,
        sidebar_page: sidebar.page,
        sidebar_pages: sidebar.pages,
        area: Rect::new(0, 0, aw, ah),
        ..AboutHits::default()
    };
    for (k, i) in (sidebar.first..sidebar.first + sidebar.rows.len()).enumerate() {
        let r = sidebar.rows[k];
        let label = all[i].label();
        let row = crate::ui::Row {
            style: if i == note { ButtonStyle::Selected } else { ButtonStyle::Plain },
            ..crate::ui::Row::new(IconKind::None, &label).bold()
        };
        ui_row(c, r, &row);
        hits.rows.push((r, i));
        if i == note {
            hits.marked = k as i32 + 1;
        }
    }
    ui_row_pager(c, &sidebar);
    hits.sidebar_prev = sidebar.prev;
    hits.sidebar_next = sidebar.next;

    let reader_x = content.x + side_w + ui_px(48.0);
    let reader_w = (content.x + content.w - reader_x).max(1);
    let e = &all[note];
    let date = date_label(e.date);
    let date_w = if date.is_empty() { 0 } else { font_measure(FontFace::Regular, ui_small_size(), date) };
    let label = e.label();
    let head_h = ui_px(78.0);
    if date_w > 0 {
        font_draw_left(
            c,
            FontFace::Regular,
            ui_small_size(),
            Rect::new(reader_x + reader_w - date_w, content.y, date_w, head_h),
            date,
            90,
        );
    }
    let label_w = reader_w - if date_w > 0 { date_w + ui_px(36.0) } else { 0 };
    font_draw_left_name(c, FontFace::Bold, ui_text_size(), Rect::new(reader_x, content.y, label_w, head_h), &label, 0);
    canvas_hline(c, reader_x, reader_x + reader_w, content.y + head_h, ui_px(2.0), 0);

    let body_top = content.y + head_h + ui_px(24.0);
    let mut body = Rect::new(reader_x, body_top, reader_w, (content.y + content.h - body_top).max(0));
    let bl = about_blocks(e.md);
    let mut fit = note_set_in(e.md, &bl, body.w, body.h);
    let mut rs = rows(&bl, body.w, &note_look(fit.px, fit.tight));
    let mut pages = paginate(&rs, body.h);
    if pages.len() > 1 {
        body.h = (body.h - ui_px(130.0)).max(0);
        fit = note_set_in(e.md, &bl, body.w, body.h);
        rs = rows(&bl, body.w, &note_look(fit.px, fit.tight));
        pages = paginate(&rs, body.h);
    }
    hits.pages = pages.len() as i32;
    hits.page = page.clamp(0, hits.pages - 1);
    hits.body_px = fit.px;
    hits.lead = note_look(fit.px, fit.tight).lead;
    let shown = &pages[hits.page as usize];
    hits.signed = shown.iter().any(|it| matches!(it.ink, Ink::Sign));
    draw_items(c, shown, body);
    if hits.pages > 1 {
        let bw = ui_px(130.0);
        let bh = ui_px(90.0);
        let y = content.y + content.h - bh;
        hits.next = Rect::new(reader_x + reader_w - bw, y, bw, bh);
        hits.prev = Rect::new(hits.next.x - ui_px(150.0) - bw, y, bw, bh);
        let st = |on: bool| if on { ButtonStyle::Plain } else { ButtonStyle::Disabled };
        ui_button(c, hits.prev, IconKind::Prev, "", st(hits.page > 0));
        ui_button(c, hits.next, IconKind::Next, "", st(hits.page + 1 < hits.pages));
        font_draw_centered(
            c,
            FontFace::Regular,
            ui_small_size(),
            Rect::new(hits.prev.x + bw, y, ui_px(150.0), bh),
            &format!("{} / {}", hits.page + 1, hits.pages),
            0,
        );
    }
    hits
}

fn whats_new() -> &'static str {
    // TRANSLATORS: the title of the What's new screen (the Settings frame's title type): every
    // version's notes, with the versions down the left.
    tr("What's new")
}

/// The notes' leading at its most open and at its tightest, and their gaps at their tightest as a
/// share of the open look's (the owner, 2026-11-07: "The note reading itself isn't polished
/// enough"): notes too long for the reader tighten these at their size before their type gets
/// smaller (fit_in).
const LEAD: f32 = 1.3;
const LEAD_MIN: f32 = 1.15;
const GAPS_MIN: f32 = 0.6;
/// The steps from the open look (0) to the tightest: the leading a hundredth a step.
const TIGHT_STEPS: i32 = 15;

/// A bullet's dot beside body text at px, as a share of the card's: in the proportion the card's dot
/// has to the card's text (ui_small_size), by the type's height as it is drawn. Notes at the card's
/// size have the card's dots, larger ones larger dots, and Gothic's type, which changes size only at
/// whole design pixels, the dots of the size it shows.
fn dot_scale(px: f32) -> f32 {
    let h = |px: f32| (font_ascent(FontFace::Regular, px) + font_descent(FontFace::Regular, px)) as f32;
    h(px) / h(ui_small_size()).max(1.0)
}

/// The notes' look with their body text at px, tight steps from the open look to the tightest
/// (TIGHT_STEPS): at ui_text_size and 0, the look Read all has always set them in, but for the
/// bullets' dots (dot_scale). Smaller type makes every measure smaller with it (the headings, the
/// leading's lines, the bullets' indent, the gaps, the signature); each step tighter takes the
/// leading a hundredth nearer LEAD_MIN, and the gaps nearer GAPS_MIN of theirs, at the same size.
fn note_look(px: f32, tight: i32) -> Look {
    let k = px / ui_text_size();
    let t = tight.clamp(0, TIGHT_STEPS) as f32 / TIGHT_STEPS as f32;
    let g = k * (1.0 - (1.0 - GAPS_MIN) * t);
    let at = |v: f32| ui_px(v * k);
    let gap = |v: f32| ui_px(v * g);
    Look {
        body: px,
        head_f: FontFace::Bold,
        head_px: px,
        lead: LEAD - (LEAD - LEAD_MIN) * t,
        indent: at(56.0),
        head_gap: gap(44.0),
        item_gap: gap(8.0),
        para_gap: gap(24.0),
        sign_w: at(720.0),
        sign_h: at(190.0),
        dot: dot_scale(px),
    }
}

/// The sizes note_fit tries below ui_text_size, down to the floor (44 px to 34 px at 300 dpi, half a
/// pixel a step).
const FIT_STEPS: i32 = 20;

/// note_fit's size i: ui_text_size at 0, the floor (ui_small_size) at FIT_STEPS.
fn fit_size(i: i32) -> f32 {
    let (top, floor) = (ui_text_size(), ui_small_size());
    top + (floor - top) * i as f32 / FIT_STEPS as f32
}

/// How a version's notes are set (note_look): their body size, and how many steps tighter than the
/// open look.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fit {
    px: f32,
    tight: i32,
}

/// How notes bl are set in a body w wide and room high (note_fit's rules): the largest size at which
/// the tightest look fits, in the most open look that fits there, or the floor in the open look and
/// pages when even that does not fit.
fn fit_in(bl: &[Block], w: i32, room: i32) -> Fit {
    #[cfg(test)]
    FIT_RUNS.set(FIT_RUNS.get() + 1);
    let fits = |px: f32, tight: i32| rows_h(&rows(bl, w, &note_look(px, tight))) <= room;
    for px in (0..=FIT_STEPS).map(fit_size) {
        if !fits(px, TIGHT_STEPS) {
            continue;
        }
        let (mut open, mut tight) = (0, TIGHT_STEPS);
        while open < tight {
            let mid = (open + tight) / 2;
            if fits(px, mid) {
                tight = mid;
            } else {
                open = mid + 1;
            }
        }
        return Fit { px, tight };
    }
    Fit { px: ui_small_size(), tight: 0 }
}

thread_local! {
    /// note_set's answers: the notes (their text's place and length), the screen's width and height,
    /// the look, Classic's font (2.1, Settings > Font; 0 in Gothic, which has one) and the body type's
    /// size, and how note_fit set them.
    static NOTE_FIT: RefCell<Vec<((usize, usize, i32, i32, bool, usize, u32), Fit)>> = const { RefCell::new(Vec::new()) };
}
#[cfg(test)]
thread_local! {
    /// note_fit's runs, for the test that a note's size is found once for each screen
    static FIT_RUNS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// note_set's answer for the notes in a body w wide and room high (the full-screen browser's
/// reader), found once for each note, size, look and font as note_set's is.
fn note_set_in(md: &'static str, bl: &[Block], w: i32, room: i32) -> Fit {
    let family = if gothic() { 0 } else { font_look() };
    let key = (md.as_ptr() as usize, md.len(), w, room, gothic(), family, ui_text_size().to_bits());
    if let Some(f) = NOTE_FIT.with_borrow(|v| v.iter().find(|e| e.0 == key).map(|e| e.1)) {
        return f;
    }
    let f = fit_in(bl, w, room);
    NOTE_FIT.with_borrow_mut(|v| v.push((key, f)));
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up(theme: UiTheme) {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        ui_set_theme(theme);
        i18n_apply("en");
    }

    fn screen(aw: i32, ah: i32) -> Canvas {
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, aw, ah), 0);
        c
    }

    fn inside(o: Rect, r: Rect) -> bool {
        rect_contains_rect(o, r)
    }

    fn note(md: &'static str) -> Entry {
        Entry { versions: vec!["9.9.9"], date: "", md }
    }

    fn entries(n: usize) -> Vec<Entry> {
        (0..n)
            .rev()
            .map(|i| Entry {
                versions: vec![Box::leak(format!("2.{i}.0").into_boxed_str())],
                date: "2026-11-05",
                md: Box::leak(format!("# Notes {i}\n\n- a change\n").into_boxed_str()),
            })
            .collect()
    }

    #[test]
    fn blocks_of_the_notes() {
        let b = about_blocks("# T\n\n## H\n- one\n- two\n\nA para\nthat goes on.\n\n{signature}\n");
        assert_eq!(
            b,
            vec![
                Block::Title("T".into()),
                Block::Heading("H".into()),
                Block::Bullet("one".into()),
                Block::Bullet("two".into()),
                Block::Para("A para that goes on.".into()),
                Block::Signature,
            ]
        );
        for e in notes() {
            assert_eq!(about_blocks(e.md).last(), Some(&Block::Signature), "{}", e.label());
        }
    }

    #[test]
    fn the_archive_is_newest_first_and_one_entry_a_note() {
        let files = [
            NoteFile { version: "2.0.0-beta.1", date: "2026-11-01", md: "Dear reader,\n\n- one\n\n{signature}\n" },
            NoteFile { version: "2.0.0-rc.2", date: "", md: "# Since rc.1\n\n- a fix\n" },
            NoteFile { version: "2.1.0", date: "unreleased", md: "# What's new\n\n- wake\n\n{signature}\n" },
            NoteFile { version: "2.0.0-beta.10", date: "", md: "# Ten\n\n- ten\n" },
            NoteFile { version: "2.0.0", date: "2026-11-05", md: "# What's new\n\n- lots\n\n{signature}\n" },
            NoteFile { version: "2.0.0-beta.2", date: "2026-11-05", md: "Dear\nreader,\n\n\n- one\n{signature}" },
            NoteFile { version: "2.0.0-rc.10", date: "", md: "# What's new\n- wake\n\n{signature}" },
        ];
        let a = archive(&files);
        assert_eq!(
            a.iter().map(|e| e.label()).collect::<Vec<_>>(),
            ["2.1.0, 2.0.0-rc.10", "2.0.0", "2.0.0-rc.2", "2.0.0-beta.10", "2.0.0-beta.2, 2.0.0-beta.1"]
        );
        assert_eq!(a[0].date, "unreleased");
        assert_eq!(a[4].date, "2026-11-05");
        assert!(archive(&[]).is_empty());
        assert_eq!(current(&a, "2.1.0"), 0);
        assert_eq!(current(&a, "2.0.0"), 1);
        assert_eq!(current(&a, "1.2.0"), 0);
        assert!(notes()[current(notes(), GRIMOIRE_VERSION)].versions.contains(&GRIMOIRE_VERSION));
    }

    #[test]
    fn the_browser_lists_every_version_and_reads_the_selected_one() {
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for (aw, ah) in [(1860, 2480), (2480, 1860)] {
                let mut c = screen(aw, ah);
                let all = notes();
                let h = draw_browser(&mut c, aw, ah, all, 2, 0, 0);
                let what = format!("{theme:?} {aw}x{ah}");
                assert_eq!(h.rows.len(), all.len(), "{what}");
                assert_eq!(h.rows[2].1, 2, "{what}");
                assert_eq!(h.marked, 3, "{what}");
                assert_eq!(h.sidebar_page, 0, "{what}");
                assert_eq!(h.sidebar_pages, 1, "{what}");
                assert!(h.pages >= 1 && h.page < h.pages, "{what}");
                assert!(!rect_is_empty(h.close), "{what}");
                assert!(inside(Rect::new(0, 0, aw, ah), h.area), "{what}");
                assert!(h.body_px >= ui_small_size() && h.body_px <= ui_text_size(), "{what}");
                for (r, _) in &h.rows {
                    assert!(inside(h.area, *r), "{what}: {r:?}");
                }
            }
        }
    }

    #[test]
    fn the_sidebar_brings_the_selected_version_into_view() {
        up(UiTheme::Gothic);
        let all = entries(40);
        let mut c = screen(1860, 2480);
        let h = draw_browser(&mut c, 1860, 2480, &all, 39, 0, 0);
        assert!(h.sidebar_pages > 1, "the versions need pages");
        assert_eq!(h.sidebar_page, h.sidebar_pages - 1);
        assert_eq!(h.rows.last().unwrap().1, 39);
        assert!(h.marked > 0);
        assert!(h.pages >= 1);
    }

    #[test]
    fn every_versions_notes_read_whole_and_signed() {
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for (aw, ah) in [(1860, 2480), (2480, 1860)] {
                let mut c = screen(aw, ah);
                for (i, e) in notes().iter().enumerate() {
                    let h = draw_browser(&mut c, aw, ah, notes(), i, 0, 0);
                    let what = format!("{theme:?} {aw}x{ah} {}", e.label());
                    assert!(h.pages >= 1, "{what}");
                    assert!(h.signed, "{what}: its notes end with the signature");
                    assert!(!rect_is_empty(h.close), "{what}");
                    if h.pages > 1 {
                        assert!(!rect_is_empty(h.next) && !rect_is_empty(h.prev), "{what}");
                    } else {
                        assert!(rect_is_empty(h.next) && rect_is_empty(h.prev), "{what}");
                    }
                }
                let h = draw_browser(&mut c, aw, ah, notes(), 0, 99, 0);
                assert_eq!(h.page, h.pages - 1);
            }
        }
    }

    #[test]
    fn notes_tighten_and_shrink_to_fit_the_reader() {
        up(UiTheme::Classic);
        let md = "# Short\n\n- one change\n\n{signature}\n";
        let bl = about_blocks(md);
        let fit = fit_in(&bl, 900, 1200);
        assert_eq!((fit.px, fit.tight), (ui_text_size(), 0));
        let short = note(md);
        let mut c = screen(1860, 2480);
        let h = draw_browser(&mut c, 1860, 2480, &[short], 0, 0, 0);
        assert_eq!((h.body_px, h.pages), (ui_text_size(), 1));
        let long: String = (0..90).map(|i| format!("- change {i}, which takes a line or two to say at any of these sizes\n")).collect();
        let long = note(Box::leak(format!("# Long\n\n{long}\nCheers,\n\n{{signature}}\n").into_boxed_str()));
        let h = draw_browser(&mut c, 1860, 2480, &[long], 0, 0, 0);
        assert_eq!(h.body_px, ui_small_size());
        assert!(h.pages > 1);
        assert!(!rect_is_empty(h.next));
    }

    #[test]
    fn a_word_wider_than_the_reader_is_broken_not_cut() {
        let url = format!("https://example.com/{}", "grimoire-notes/".repeat(12));
        let link = format!("See {url} for the rest.");
        let ja = "グリモアはキンドルスクライブのためのノートです。「すべて読む」を押すと、すべての版のお知らせが一覧になり、タップするとその版のお知らせが一画面に収まります。".repeat(3);
        let strip = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            up(theme);
            for w in [900, 1200] {
                for px in [ui_text_size(), ui_small_size()] {
                    for (text, iw) in [(&link, w - note_look(px, 0).indent), (&ja, w)] {
                        let lines = wrap(FontFace::Regular, px, text, iw);
                        assert!(lines.len() > 1, "{theme:?} {w} {px}: {lines:?}");
                        for l in &lines {
                            assert!(font_measure(FontFace::Regular, px, l) <= iw, "{theme:?} {w} {px}: {l}");
                            assert!(!NO_BREAK_BEFORE.contains(l.chars().next().unwrap()) && !NO_BREAK_AFTER.contains(l.chars().last().unwrap()), "{l}");
                        }
                        assert_eq!(strip(&lines.concat()), strip(text));
                    }
                }
            }
            assert_eq!(wrap(FontFace::Regular, 44.0, "one two  three", 10000), ["one two three"]);
            assert_eq!(pieces("Kindleの「本」。"), [("Kindle".into(), true), ("の".into(), false), ("「本」。".into(), false)]);
        }
    }

    #[test]
    fn signature_pgm() {
        assert_eq!(pgm(b"P5\n3 2\n255\n\x00\x80\xff\x10\x20\x30"), Some((3, 2, &b"\x00\x80\xff\x10\x20\x30"[..])));
        assert_eq!(pgm(b"P5 3 2 255 \x00\x80\xff\x10\x20"), None);
        assert_eq!(pgm(b"P5\n3 2\n65535\n"), None);
        assert_eq!(pgm(b"P6\n1 1\n255\n\x00"), None);
        let (w, h, px) = pgm(SIGNATURE).expect("assets/signature.pgm");
        assert_eq!(px.len(), (w * h) as usize);
        let (bw, bh) = sig_box(560, 130);
        assert!(bw <= 560 && bh <= 130 && (bw as f32 / bh as f32 - w as f32 / h as f32).abs() < 0.1);
    }
}
