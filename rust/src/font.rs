//! font.h: TrueType text via stb_truetype, fonts embedded at build time
//! (build/gen/fonts.c, from assets/*.ttf via xxd -i). Glyph bitmaps are cached per size.
//! Classic's text can be drawn in one of FONT_FAMILIES (2.1, Rust only): Terminess, or a bundled
//! family the Font sheet offers (settings.ini font=). Or in a font of the owner's own (2.1.0-beta.2,
//! font=file:...): a file of grimoire/fonts that passed the check, which stb never opens: skrifa reads
//! it and stb's rasterizer draws the outlines (fontcheck.rs's SafeFace), glyph by glyph, with Terminess
//! for whatever it lacks.

use crate::assets::{
    FONT_ATKINSON_BOLD, FONT_ATKINSON_REGULAR, FONT_DOTGOTHIC16, FONT_JACQUARD24, FONT_JERSEY10, FONT_LITERATA_BOLD,
    FONT_LITERATA_REGULAR, FONT_NOTO_SANS_BOLD, FONT_NOTO_SANS_REGULAR, FONT_TERMINESS_BOLD, FONT_TERMINESS_REGULAR,
};
use crate::ffi_stb::*;
use crate::fontcheck::{Outliner, SafeFace, FONT_CJK_EM, FONT_PX_MAX, FONT_PX_MIN};
use crate::prelude::*;
use std::cell::RefCell;

/// What the text is for; the theme picks the typeface. Classic: Terminess, or the family the Font
/// sheet picked (its Regular for text, its Bold for bold, titles and caps). Gothic: pixel
/// faces, Jersey 10 for text and labels (overprinted one design pixel for bold, except where
/// that would fill in an accent or turn a letter into another), Jacquard 24 (pixel blackletter)
/// for titles. FIXED faces are Terminess in every theme and with every font: paper labels, the
/// clock.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FontFace {
    #[default]
    Regular = 0,  // FONT_REGULAR
    Bold = 1,  // FONT_BOLD
    Title = 2,  // FONT_TITLE
    Caps = 3,  // FONT_CAPS
    Fixed = 4,  // FONT_FIXED
    FixedBold = 5,  // FONT_FIXED_BOLD
}
impl FontFace {
    /// FONT_FACE_COUNT
    pub const COUNT: usize = 6;
}

// font.c: TrueType text through stb_truetype with a small glyph cache. Pixel fonts (drawn on
// a grid of `unit` font units) are rendered one bitmap pixel per design pixel, without
// smoothing, then blown up to whole k x k blocks, so they stay razor sharp at any size.

// The embedded TrueType files. Terminess (with the Nerd Font icons) is always there; the
// pixel faces belong to the gothic theme: Jacquard 24 (pixel blackletter) for titles, Jersey
// 10 for text and labels. DotGothic16 is the Japanese fallback, in the same pixel style. Then
// the Classic families the Font sheet offers (2.1), each a Regular and a Bold (assets/README.md
// says how they were made: static, Latin only; what they lack comes from Terminess).
pub(crate) const TTF_TERM: usize = 0;
pub(crate) const TTF_TERM_BOLD: usize = 1;
pub(crate) const TTF_JACQUARD: usize = 2;
pub(crate) const TTF_JERSEY: usize = 3;
const TTF_JP: usize = 4;
const TTF_ATKINSON: usize = 5;
const TTF_ATKINSON_BOLD: usize = 6;
const TTF_NOTO: usize = 7;
const TTF_NOTO_BOLD: usize = 8;
const TTF_LITERATA: usize = 9;
const TTF_LITERATA_BOLD: usize = 10;
const TTF_COUNT: usize = 11;
// After the embedded files, the faces of the owner's own fonts (2.1.0-beta.2), which are no file of the
// binary and never stb's to open: the font in use (its regular, and its bold when it has one), then the
// one the Font sheet shows a sample of. A glyph's cache slot names its face by these numbers too.
const TTF_OWN: usize = TTF_COUNT;
const TTF_TRY: usize = TTF_COUNT + 2;
const OWN_SLOTS: usize = 4;

/// The embedded file of a TTF_* index.
fn ttf_data(ttf: usize) -> &'static [u8] {
    match ttf {
        TTF_TERM => FONT_TERMINESS_REGULAR,
        TTF_TERM_BOLD => FONT_TERMINESS_BOLD,
        TTF_JACQUARD => FONT_JACQUARD24,
        TTF_JERSEY => FONT_JERSEY10,
        TTF_JP => FONT_DOTGOTHIC16,
        TTF_ATKINSON => FONT_ATKINSON_REGULAR,
        TTF_ATKINSON_BOLD => FONT_ATKINSON_BOLD,
        TTF_NOTO => FONT_NOTO_SANS_REGULAR,
        TTF_NOTO_BOLD => FONT_NOTO_SANS_BOLD,
        TTF_LITERATA => FONT_LITERATA_REGULAR,
        _ => FONT_LITERATA_BOLD,
    }
}

/// A family Classic's text can be drawn in (2.1): Terminess, the default, or one the Font sheet
/// offers beside it. Font names are never translated.
#[derive(Clone, Copy, Debug)]
pub struct FontFamily {
    /// settings.ini's font= value
    pub id: &'static str,
    /// its name: the Font sheet's row, drawn in the family itself
    pub name: &'static str,
    /// the short name Settings' Font chip shows ("Font: Atkinson")
    pub short: &'static str,
    /// its Regular and Bold (TTF_*)
    regular: usize,
    bold: usize,
    /// How much bigger than asked its text is drawn (FaceMap.scale; a size is a line height, ascent
    /// to descent, and each family puts a different share of its line in its letters): Terminess's
    /// sizes stay the sizes every layout was made for, and each family's letters come out about
    /// Terminess's size while every screen still fits in English and Spanish, upright and sideways
    /// (make l10n-check runs every screen in each family).
    pub scale: f32,
    /// the share pages' names for its Regular and Bold, under /font/ (httpd.rs serves the files)
    pub web: [&'static str; 2],
    /// the CSS generic family the share pages name after it
    pub css_generic: &'static str,
}

/// The families, Terminess first (the default; settings.ini font=terminess). The share pages serve the
/// one in use (httpd.rs), and only in Classic: Gothic's look is its pixel faces.
pub const FONT_FAMILIES: [FontFamily; 4] = [
    FontFamily {
        id: "terminess",
        name: "Terminess",
        short: "Terminess",
        regular: TTF_TERM,
        bold: TTF_TERM_BOLD,
        scale: 1.0,
        web: ["terminess.ttf", "terminess-bold.ttf"],
        css_generic: "monospace",
    },
    // Atkinson Hyperlegible Next (Braille Institute): made for readers with low vision; I, l and 1,
    // O and 0 cannot be mistaken.
    FontFamily {
        id: "atkinson",
        name: "Atkinson Hyperlegible Next",
        short: "Atkinson",
        regular: TTF_ATKINSON,
        bold: TTF_ATKINSON_BOLD,
        scale: 1.10,
        web: ["atkinson.ttf", "atkinson-bold.ttf"],
        css_generic: "sans-serif",
    },
    // Noto Sans: neutral, its digits all one width (a number that changes keeps its place).
    FontFamily {
        id: "noto-sans",
        name: "Noto Sans",
        short: "Noto Sans",
        regular: TTF_NOTO,
        bold: TTF_NOTO_BOLD,
        scale: 1.065,
        web: ["noto-sans.ttf", "noto-sans-bold.ttf"],
        css_generic: "sans-serif",
    },
    // Literata: a serif made for reading on screens (Google Play Books' typeface), at its 12 pt
    // optical size, the sturdy one.
    FontFamily {
        id: "literata",
        name: "Literata",
        short: "Literata",
        regular: TTF_LITERATA,
        bold: TTF_LITERATA_BOLD,
        scale: 1.15,
        web: ["literata.ttf", "literata-bold.ttf"],
        css_generic: "serif",
    },
];

/// The family settings.ini's font= names (its id), None for one this version does not have.
pub fn font_family_find(id: &[u8]) -> Option<usize> {
    FONT_FAMILIES.iter().position(|f| f.id.as_bytes() == id)
}

/// A family's Regular (bold false) or Bold file, as embedded: what the share pages serve.
pub fn font_family_file(family: usize, bold: bool) -> &'static [u8] {
    let f = &FONT_FAMILIES[family.min(FONT_FAMILIES.len() - 1)];
    ttf_data(if bold { f.bold } else { f.regular })
}

/// Font units per design pixel (0 = an outline font, drawn smooth). Measured: every outline
/// point of these files sits on this grid.
const K_UNIT: [i32; TTF_COUNT] = {
    let mut k = [0; TTF_COUNT];
    k[TTF_JACQUARD] = 30;
    k[TTF_JERSEY] = 75;
    k
};

/// K_UNIT for any face: a custom font is an outline font.
fn k_unit(ttf: usize) -> i32 {
    if ttf < TTF_COUNT {
        K_UNIT[ttf]
    } else {
        0
    }
}

/// What a FontFace means under the current theme and font: which file, how much bigger than asked,
/// whether to embolden by overprinting (one design pixel for pixel faces), and the Terminess weight a
/// glyph the file lacks comes from (the bold faces' from its Bold).
#[derive(Clone, Copy)]
pub struct FaceMap {
    pub(crate) ttf: usize,
    pub(crate) scale: f32,
    pub(crate) embolden: bool,
    pub(crate) fallback: usize,
}

pub(crate) const fn fm(ttf: usize, scale: f32, embolden: bool) -> FaceMap {
    FaceMap { ttf, scale, embolden, fallback: if embolden { TTF_TERM_BOLD } else { TTF_TERM } }
}

/// The same for a face of a family's own Bold (it needs no overprint, and its fallback is bold).
pub(crate) const fn fm_bold(ttf: usize, scale: f32) -> FaceMap {
    FaceMap { ttf, scale, embolden: false, fallback: TTF_TERM_BOLD }
}

// The glyph cache: one slot per (face, size, code point), probed from a hash.
#[derive(Default)]
struct Glyph {
    used: bool,
    /// TTF_* index
    face: u8,
    /// size * 4, rounded; pixel faces: -k
    size4: i32,
    cp: u32,
    w: i32,
    h: i32,
    xoff: i32,
    yoff: i32,
    /// unscaled advance (font units)
    advance: i32,
    /// empty where the C has NULL (a glyph with no ink, a space)
    bmp: Vec<u8>,
    /// pixel glyphs above U+007F: the bold weight, w + k wide (see pixel_bold); empty until made
    bold: Vec<u8>,
}

const CACHE_SLOTS: usize = 1024;
/// The bitmaps the cache holds at most, all together: past it, it is flushed whole, as when its slots
/// run out. (A screen of text is a few hundred KB; a custom font's large glyphs are what could grow it.)
const CACHE_BYTES_MAX: usize = 4 << 20;
/// A glyph's bitmap has a greatest size: this many pixels each way, and no more bytes (one a pixel) than the
/// cache holds, CACHE_BYTES_MAX: a square of 2,048 px, or 4,096 by 1,024. It holds for every glyph of an
/// embedded face, whoever draws it and at whatever size: the size is worked out first (`outline_bitmap_ok`,
/// `make_pixel_glyph`), and no bitmap is made of a larger glyph. Its place in a text stays empty and keeps its
/// advance, so a line is laid out as it is measured; an icon is not drawn. What the screen shows is far under
/// it: a letter reaches it at a text height of about 3,000 px, an icon in a square of about 2,000. (A font of
/// the owner's own has its sizes from fontcheck.rs: FONT_PX_MAX, `rasterize`.)
const GLYPH_SIDE_MAX: i64 = 4096;
/// An edge of a glyph's box, in pixels from the glyph's origin, lies no farther out than this, or the glyph is
/// not made: far past GLYPH_SIDE_MAX, and a number of pixels that an f32 and stb's own whole numbers both hold
/// exactly.
const GLYPH_EDGE_MAX: f32 = (1u32 << 30) as f32;

/// Whether a glyph may have a bitmap of `w` by `h` pixels (GLYPH_SIDE_MAX): neither side over the greatest
/// or under nothing, and their product, worked out in 64 bits, no more than CACHE_BYTES_MAX. (A side of
/// nothing is a glyph with no ink, which has no bitmap.)
fn glyph_size_ok(w: i64, h: i64) -> bool {
    (0..=GLYPH_SIDE_MAX).contains(&w) && (0..=GLYPH_SIDE_MAX).contains(&h) && w.saturating_mul(h) <= CACHE_BYTES_MAX as i64
}

/// How a custom font maps onto Classic's faces: its scale, and whether its bold is a face of its own (its
/// weight axis, or a Bold file paired with it) or the regular overprinted.
#[derive(Clone, Copy)]
struct OwnMap {
    scale: f32,
    bold_face: bool,
}

/// A font of the owner's own, ready to draw: its regular face, and its bold when it has one (None: bold is
/// the regular overprinted, as an outline face's always was).
pub struct OwnFont {
    pub regular: SafeFace,
    pub bold: Option<SafeFace>,
}

struct State {
    /// one stbtt_fontinfo per embedded file, boxed so it keeps its address once stbtt_InitFont has
    /// filled it
    font: [Box<stbtt_fontinfo>; TTF_COUNT],
    ready: bool,
    map: [FaceMap; FontFace::COUNT],
    /// the theme's own faces (Look::faces): it takes no family and no custom font
    theme_faces: Option<[FaceMap; FontFace::COUNT]>,
    /// the Classic family (FONT_FAMILIES)
    family: usize,
    cache: Vec<Glyph>,
    cache_used: i32,
    /// the bytes of the bitmaps in the cache
    cache_bytes: usize,
    overflow: Option<FontOverflowHook>,
    /// the custom faces (TTF_OWN + i): the font in use, then the Font sheet's sample
    own: [Option<SafeFace>; OWN_SLOTS],
    /// how each of the two maps (None: there is none)
    own_map: [Option<OwnMap>; 2],
    /// which of the two Classic's text is drawn in: 0 the font in use, 1 the sample
    own_pick: usize,
    /// Classic's text in the family whatever custom font there is (the Font sheet's built-in rows and its
    /// own words)
    own_off: bool,
    /// glyphs of custom faces drawn so far (the crash guard waits for a screen with some)
    own_drawn: u32,
    /// how many times the custom font in use was set (font_look: another font, another layout)
    own_sets: usize,
    /// the one buffer custom faces' outlines are read into (256 KB, there from the first custom glyph on)
    own_out: Option<Outliner>,
    /// the largest an icon is drawn, each way, in pixels (font_with_icon_side_max; i32::MAX: as large as
    /// its rectangle)
    icon_side_max: i32,
}

impl State {
    fn new() -> State {
        State {
            font: std::array::from_fn(|_| Box::new(stbtt_fontinfo::default())),
            ready: false,
            map: [fm(0, 0.0, false); FontFace::COUNT],
            theme_faces: None,
            family: 0,
            cache: (0..CACHE_SLOTS).map(|_| Glyph::default()).collect(),
            cache_used: 0,
            cache_bytes: 0,
            overflow: None,
            own: std::array::from_fn(|_| None),
            own_map: [None; 2],
            own_pick: 0,
            own_off: false,
            own_drawn: 0,
            own_sets: 0,
            own_out: None,
            icon_side_max: i32::MAX,
        }
    }
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::new());
}
// Each borrow is one primitive's: nothing here calls the overflow hook, another module or itself
// while holding it.
fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

// ---- stb_truetype, one safe function per call ----

fn init_font(info: &mut stbtt_fontinfo, data: &'static [u8]) -> bool {
    // SAFETY: data is one of the embedded TrueType files, 'static, so the pointer stb keeps in info
    // stays valid; info is our own boxed storage, larger than stb's struct (ffi_stb.rs).
    unsafe { stbtt_InitFont(info, data.as_ptr(), stbtt_GetFontOffsetForIndex(data.as_ptr(), 0)) != 0 }
}

fn vmetrics(fi: &stbtt_fontinfo) -> (i32, i32, i32) {
    let (mut a, mut d, mut l) = (0, 0, 0);
    // SAFETY: fi was filled by stbtt_InitFont; the three outputs are our locals.
    unsafe { stbtt_GetFontVMetrics(fi, &mut a, &mut d, &mut l) };
    (a, d, l)
}

fn scale_for_pixel_height(fi: &stbtt_fontinfo, px: f32) -> f32 {
    // SAFETY: fi was filled by stbtt_InitFont.
    unsafe { stbtt_ScaleForPixelHeight(fi, px) }
}

fn find_glyph_index(fi: &stbtt_fontinfo, cp: u32) -> i32 {
    // SAFETY: fi was filled by stbtt_InitFont.
    unsafe { stbtt_FindGlyphIndex(fi, cp as i32) }
}

/// (advance, left side bearing) in font units
fn hmetrics(fi: &stbtt_fontinfo, cp: u32) -> (i32, i32) {
    let (mut adv, mut lsb) = (0, 0);
    // SAFETY: fi was filled by stbtt_InitFont; the outputs are our locals.
    unsafe { stbtt_GetCodepointHMetrics(fi, cp as i32, &mut adv, &mut lsb) };
    (adv, lsb)
}

fn kern_advance(fi: &stbtt_fontinfo, a: u32, b: u32) -> i32 {
    // SAFETY: fi was filled by stbtt_InitFont.
    unsafe { stbtt_GetCodepointKernAdvance(fi, a as i32, b as i32) }
}

/// The glyph's ink box (x0, y0, x1, y1) in font units, None if the font has no such glyph.
fn codepoint_box(fi: &stbtt_fontinfo, cp: u32) -> Option<(i32, i32, i32, i32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (0, 0, 0, 0);
    // SAFETY: fi was filled by stbtt_InitFont; the outputs are our locals.
    let ok = unsafe { stbtt_GetCodepointBox(fi, cp as i32, &mut x0, &mut y0, &mut x1, &mut y1) };
    if ok == 0 {
        None
    } else {
        Some((x0, y0, x1, y1))
    }
}

/// Whether the bitmap stb makes of cp's outline at this scale is one a glyph may have (`glyph_size_ok`). Its
/// sides are worked out here as stb works them out (stbtt_GetGlyphBitmapBoxSubpixel: the glyph's box in font
/// units, scaled, and out to whole pixels), before anything is made and in floating point, where a box of any
/// size is a number. A scale under nothing, or one that is not a number, is of no glyph. A glyph with no outline
/// has no bitmap, and passes.
fn outline_bitmap_ok(fi: &stbtt_fontinfo, sx: f32, sy: f32, cp: u32) -> bool {
    // (no comparison holds for a NaN)
    if !(sx >= 0.0 && sy >= 0.0) {
        return false;
    }
    let Some((x0, y0, x1, y1)) = codepoint_box(fi, cp) else { return true };
    // (a scale of nothing one way is the other way's, as stb takes it)
    let (sx, sy) = (if sx == 0.0 { sy } else { sx }, if sy == 0.0 { sx } else { sy });
    let (ix0, iy0) = ((x0 as f32 * sx).floor(), (-(y1 as f32) * sy).floor());
    let (ix1, iy1) = ((x1 as f32 * sx).ceil(), (-(y0 as f32) * sy).ceil());
    // (an edge that is not a number, or lies farther out than a glyph's may, fails here)
    if ![ix0, iy0, ix1, iy1].iter().all(|e| e.abs() <= GLYPH_EDGE_MAX) {
        return false;
    }
    glyph_size_ok(ix1 as i64 - ix0 as i64, iy1 as i64 - iy0 as i64)
}

#[cfg(test)]
thread_local! {
    /// Tests: how many bitmaps stb has been asked for.
    static BITMAPS_ASKED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Tests: how many bitmaps stb has been asked for so far, on this thread (a glyph past the greatest size is not
/// asked for).
#[cfg(test)]
pub(crate) fn font_bitmaps_asked() -> u32 {
    BITMAPS_ASKED.with(|n| n.get())
}

/// stbtt_GetCodepointBitmap: the 8-bit coverage (empty where stb returns NULL: a glyph with no
/// ink), and its width, height and offsets. The bitmap stb allocated is copied and freed here, so
/// no raw pointer outlives this call. A glyph whose bitmap would be past the greatest size
/// (GLYPH_SIDE_MAX) is not made: the answer is that of a glyph with no ink, and stb is not asked.
fn codepoint_bitmap(fi: &stbtt_fontinfo, sx: f32, sy: f32, cp: u32) -> (Vec<u8>, i32, i32, i32, i32) {
    if !outline_bitmap_ok(fi, sx, sy, cp) {
        return (Vec::new(), 0, 0, 0, 0);
    }
    #[cfg(test)]
    BITMAPS_ASKED.with(|n| n.set(n.get() + 1));
    let (mut w, mut h, mut xo, mut yo) = (0, 0, 0, 0);
    // SAFETY: fi was filled by stbtt_InitFont; the four outputs are our locals.
    let p = unsafe { stbtt_GetCodepointBitmap(fi, sx, sy, cp as i32, &mut w, &mut h, &mut xo, &mut yo) };
    let mut bmp = Vec::new();
    if !p.is_null() {
        // (the size stb says is the one asked about above: read as its bytes only when it is one a glyph has)
        let sized = w > 0 && h > 0 && glyph_size_ok(w as i64, h as i64);
        if sized {
            // SAFETY: on success stb returns w * h bytes (it returns NULL when either is 0).
            bmp.extend_from_slice(unsafe { std::slice::from_raw_parts(p, w as usize * h as usize) });
        }
        // SAFETY: p came from stbtt_GetCodepointBitmap (malloc), is not used again, and no
        // userdata was given to the allocator.
        unsafe { stbtt_FreeBitmap(p, std::ptr::null_mut()) };
        if !sized {
            return (Vec::new(), 0, 0, 0, 0);
        }
    }
    (bmp, w, h, xo, yo)
}

// ---- a face, embedded or the owner's own ----

fn own_face(st: &mut State, ttf: usize) -> Option<&mut SafeFace> {
    st.own.get_mut(ttf.wrapping_sub(TTF_COUNT))?.as_mut()
}

/// Whether the face has its own glyph for cp (a custom face: one that passed the check).
fn face_has(st: &mut State, ttf: usize, cp: u32) -> bool {
    if ttf < TTF_COUNT {
        find_glyph_index(&st.font[ttf], cp) != 0
    } else {
        own_face(st, ttf).is_some_and(|f| f.glyph(cp).is_some())
    }
}

/// cp's advance in the face's font units.
fn face_advance(st: &mut State, ttf: usize, cp: u32) -> i32 {
    if ttf < TTF_COUNT {
        hmetrics(&st.font[ttf], cp).0
    } else {
        own_face(st, ttf).and_then(|f| f.glyph(cp)).map_or(0, |g| g.1)
    }
}

/// The face's ascent and descent in font units (a custom face's as the check settled them).
fn face_vmetrics(st: &State, ttf: usize) -> (i32, i32) {
    if ttf < TTF_COUNT {
        let (a, d, _l) = vmetrics(&st.font[ttf]);
        return (a, d);
    }
    match st.own.get(ttf - TTF_COUNT).and_then(|f| f.as_ref()) {
        Some(f) => (f.facts().ascent, f.facts().descent),
        None => (1, 0),
    }
}

/// Pixels a font unit of the face at a line height of px: stbtt_ScaleForPixelHeight, for any face.
fn face_scale(st: &State, ttf: usize, px: f32) -> f32 {
    if ttf < TTF_COUNT {
        return scale_for_pixel_height(&st.font[ttf], px);
    }
    let (a, d) = face_vmetrics(st, ttf);
    px / (a - d).max(1) as f32
}

/// The line height a custom face's glyph for cp is drawn at when its text is drawn at fpx (px asked, times
/// the font's scale): fpx, but kana and kanji at the size whose em is DotGothic16's, so Japanese takes the
/// room it takes in every other face. None when that is a size the check did not draw: the glyph then
/// comes from Terminess.
fn own_px(st: &State, ttf: usize, px: f32, fpx: f32, cp: u32) -> Option<f32> {
    let f = st.own.get(ttf.wrapping_sub(TTF_COUNT))?.as_ref()?.facts();
    let g = if cjk(cp) { px * FONT_CJK_EM * f.line() as f32 / f.upem.max(1) as f32 } else { fpx };
    (FONT_PX_MIN..=FONT_PX_MAX).contains(&g).then_some(g)
}

/// Whether the custom font in use draws cp in Classic's text (a name's letters count as drawable then).
fn own_text_has(st: &mut State, cp: u32) -> bool {
    // (the Bold face draws most of the chrome: a Bold file that lacks the letter would show the missing-glyph
    // box where the regular has it, so both must have it)
    let Some(m) = st.own_map[0] else { return false };
    st.theme_faces.is_none() && !st.own_off && face_has(st, TTF_OWN, cp) && (!m.bold_face || face_has(st, TTF_OWN + 1, cp))
}

/// Tests (the simulator's build): GRIMOIRE_FONT_TEST_DRAW_CRASH ends the app at the first glyph a custom
/// font draws, as a font that crashed its first screen would.
#[cfg(feature = "sim")]
fn own_draw_hook() {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *ON.get_or_init(|| std::env::var_os("GRIMOIRE_FONT_TEST_DRAW_CRASH").is_some()) {
        std::process::abort();
    }
}

/// A custom face's glyph for the cache: (advance, bitmap, w, h, xoff, yoff). A glyph that does not draw
/// (it cannot after the check, but the caps are applied again) leaves its room empty.
fn own_glyph(st: &mut State, ttf: usize, px: f32, cp: u32) -> (i32, Vec<u8>, i32, i32, i32, i32) {
    #[cfg(feature = "sim")]
    own_draw_hook();
    if st.own_out.is_none() {
        st.own_out = Outliner::new();
    }
    let State { own, own_out, .. } = st;
    let (Some(f), Some(out)) = (own.get_mut(ttf.wrapping_sub(TTF_COUNT)).and_then(|f| f.as_mut()), own_out.as_mut()) else {
        return (0, Vec::new(), 0, 0, 0, 0);
    };
    let Some((gid, adv)) = f.glyph(cp) else { return (0, Vec::new(), 0, 0, 0, 0) };
    let bm = f.bitmap(out, gid, px);
    st.own_drawn = st.own_drawn.wrapping_add(1);
    match bm {
        Some(b) => (adv, b.px, b.w, b.h, b.xoff, b.yoff),
        None => (adv, Vec::new(), 0, 0, 0, 0),
    }
}

// ---- faces ----

fn map_faces(st: &mut State) {
    // Classic: the family's Regular for text, its Bold for bold, titles and caps (Terminess's are the
    // classic faces of every version before 2.1); FIXED always Terminess.
    let f = &FONT_FAMILIES[st.family];
    let mut classic: [FaceMap; FontFace::COUNT] = [
        fm(f.regular, f.scale, false), // FONT_REGULAR
        fm_bold(f.bold, f.scale),      // FONT_BOLD
        fm_bold(f.bold, f.scale),      // FONT_TITLE
        fm_bold(f.bold, f.scale),      // FONT_CAPS
        fm(TTF_TERM, 1.0, false),      // FONT_FIXED
        fm_bold(TTF_TERM_BOLD, 1.0),   // FONT_FIXED_BOLD
    ];
    // A font of the owner's own takes the family's place in the four text faces: its bold face where it
    // has one, else its regular overprinted. What it lacks comes from Terminess, glyph by glyph.
    if let (false, Some(m)) = (st.own_off, st.own_map[st.own_pick]) {
        let reg = if st.own_pick == 0 { TTF_OWN } else { TTF_TRY };
        let bold = if m.bold_face { fm_bold(reg + 1, m.scale) } else { fm(reg, m.scale, true) };
        classic[FontFace::Regular as usize] = fm(reg, m.scale, false);
        classic[FontFace::Bold as usize] = bold;
        classic[FontFace::Title as usize] = bold;
        classic[FontFace::Caps as usize] = bold;
    }
    st.map = st.theme_faces.unwrap_or(classic);
}

/// Pixel faces: how many screen pixels one design pixel becomes at a requested line height.
fn pixel_k(st: &State, ttf: usize, px: f32) -> i32 {
    let (a, d, _l) = vmetrics(&st.font[ttf]);
    let line = (a - d) as f32 / k_unit(ttf) as f32;
    let k = (px / line).round_ties_even() as i32;
    if k < 1 {
        1
    } else {
        k
    }
}

fn cache_flush(st: &mut State) {
    // (the bitmaps are Vecs: dropping a slot frees what stbtt_FreeBitmap and free free in the C)
    for g in st.cache.iter_mut() {
        *g = Glyph::default();
    }
    st.cache_used = 0;
    st.cache_bytes = 0;
}

/// 0 on success
pub fn font_init() -> i32 {
    let failed = with(|st| {
        if st.ready {
            return None;
        }
        for i in 0..TTF_COUNT {
            if !init_font(&mut st.font[i], ttf_data(i)) {
                return Some(i);
            }
        }
        map_faces(st);
        st.ready = true;
        None
    });
    match failed {
        Some(i) => {
            qlog!("font: init failed for face %d", i as i32);
            -1
        }
        None => 0,
    }
}

/// The theme's own faces from now on (ui_set_theme: Look::faces), or None: Classic's family, or the
/// owner's own font.
pub fn font_set_faces(faces: Option<[FaceMap; FontFace::COUNT]>) {
    with(|st| {
        st.theme_faces = faces;
        map_faces(st);
    });
}

/// Whether the theme in use draws in faces of its own (no family, no custom font).
pub fn font_theme_faces() -> bool {
    with(|st| st.theme_faces.is_some())
}

/// Classic's text from now on in FONT_FAMILIES[family] (one past the list: Terminess). Gothic keeps
/// its pixel faces whatever the family; FIXED faces and icons are Terminess in every family. Nothing
/// cached is lost: the glyph cache keys each glyph by its file.
pub fn font_set_family(family: usize) {
    with(|st| {
        st.family = if family < FONT_FAMILIES.len() { family } else { 0 };
        map_faces(st);
    });
}

/// The Classic family in use (FONT_FAMILIES).
pub fn font_family() -> usize {
    with(|st| st.family)
}

/// f run with Classic's text in another family, the one in use again after it: the Font sheet draws
/// each family's row, and the sample of the one highlighted, in that family. The family itself: a custom
/// font in use is set aside for the run, so the sheet's own words can be drawn in Terminess whatever font
/// was picked (a pick that went wrong can always be undone).
pub fn font_with_family<R>(family: usize, f: impl FnOnce() -> R) -> R {
    let (was, was_off) = with(|st| (st.family, std::mem::replace(&mut st.own_off, true)));
    font_set_family(family);
    let r = f();
    with(|st| st.own_off = was_off);
    font_set_family(was);
    r
}

fn set_own_slot(st: &mut State, which: usize, f: Option<OwnFont>) {
    let (reg, bold, map) = match f {
        Some(o) => {
            let map = OwnMap { scale: o.regular.facts().scale_pm as f32 / 1000.0, bold_face: o.bold.is_some() };
            (Some(o.regular), o.bold, Some(map))
        }
        None => (None, None, None),
    };
    st.own[which * 2] = reg;
    st.own[which * 2 + 1] = bold;
    st.own_map[which] = map;
    // (a slot's number names another font now: nothing cached for it holds)
    cache_flush(st);
    map_faces(st);
}

/// Classic's text from now on in a font of the owner's own (None: in the family again). It passed the
/// check: userfonts.rs loads it. Gothic keeps its pixel faces; FIXED faces and icons stay Terminess.
pub fn font_set_own(f: Option<OwnFont>) {
    with(|st| {
        st.own_sets += 1;
        set_own_slot(st, 0, f)
    });
}

/// A number for what Classic's text is drawn in now, for anything that keeps a layout it measured: the
/// family (FONT_FAMILIES), or past them a number of its own for each custom font set.
pub fn font_look() -> usize {
    with(|st| if st.own_map[0].is_some() && !st.own_off { FONT_FAMILIES.len() + st.own_sets } else { st.family })
}

/// Whether a custom font is in use for Classic's text.
pub fn font_own() -> bool {
    with(|st| st.own_map[0].is_some())
}

/// The font the Font sheet shows a sample of (None: none), held beside the one in use and drawn only by
/// font_with_own.
pub fn font_set_sample(f: Option<OwnFont>) {
    with(|st| set_own_slot(st, 1, f));
}

/// f run with Classic's text in a font of the owner's own: the sample (font_set_sample), or the font in use
/// (inside font_with_family, which sets it aside); what drew before draws again after it.
pub fn font_with_own<R>(sample: bool, f: impl FnOnce() -> R) -> R {
    let (was_pick, was_off) = with(|st| {
        let was = (st.own_pick, st.own_off);
        st.own_pick = sample as usize;
        st.own_off = false;
        map_faces(st);
        was
    });
    let r = f();
    with(|st| {
        st.own_pick = was_pick;
        st.own_off = was_off;
        map_faces(st);
    });
    r
}

/// The bytes of font files the custom faces hold (a bold through the weight axis shares its regular's).
pub fn font_own_bytes() -> usize {
    with(|st| {
        let mut seen: Vec<*const Vec<u8>> = Vec::new();
        let mut n = 0;
        for f in st.own.iter().flatten() {
            let p = std::rc::Rc::as_ptr(f.data());
            if !seen.contains(&p) {
                seen.push(p);
                n += f.data().len();
            }
        }
        n
    })
}

/// How many glyphs custom faces have drawn so far: the crash guard takes its mark away after a screen
/// that drew some (userfonts.rs).
pub fn font_own_drawn() -> u32 {
    with(|st| st.own_drawn)
}

pub fn font_shutdown() {
    with(|st| {
        cache_flush(st);
        st.ready = false;
    });
}

/// Design pixels a pixel face's glyph is set below where the font has it. Jersey 10 draws the
/// inverted exclamation mark exactly as its i, a dot over a stem standing on the baseline, so a
/// Spanish exclamation would open with what reads as an i ("iHola!"). Lowered by the depth of
/// the face's descenders (g j p q y reach two design pixels below the baseline), it hangs as type
/// usually sets it: the dot at the x-height, the stem below the baseline.
fn pixel_drop(ttf: usize, cp: u32) -> i32 {
    if ttf == TTF_JERSEY && cp == 0xA1 {
        2
    } else {
        0
    }
}

/// A pixel face's glyph at k screen pixels per design pixel: hard-edged k x k blocks. Returns the
/// bitmap (empty for none) and the glyph's w, h, xoff, yoff. A glyph with no ink, and one whose blocks
/// would make a bitmap past the greatest size (GLYPH_SIDE_MAX: its sides are worked out in 64 bits, k being
/// as large as the size asked for makes it), has no bitmap and no size.
fn make_pixel_glyph(st: &State, ttf: usize, k: i32, cp: u32) -> (Vec<u8>, i32, i32, i32, i32) {
    let s = 1.0 / K_UNIT[ttf] as f32;
    let (small, w, h, xo, yo) = codepoint_bitmap(&st.font[ttf], s, s, cp);
    let (wide, high) = (w as i64 * k as i64, h as i64 * k as i64);
    if small.is_empty() || w <= 0 || h <= 0 || k <= 0 || !glyph_size_ok(wide, high) {
        return (Vec::new(), 0, 0, 0, 0);
    }
    let (gw, gh) = (wide as i32, high as i32);
    let mut bmp = vec![0u8; gw as usize * gh as usize];
    for y in 0..gh {
        for x in 0..gw {
            bmp[(y * gw + x) as usize] = if small[((y / k) * w + x / k) as usize] >= 128 { 255 } else { 0 };
        }
    }
    (bmp, gw, gh, xo * k, (yo + pixel_drop(ttf, cp)) * k)
}

/// The cache slot of the glyph, made on first use.
fn glyph_get(st: &mut State, ttf: usize, px: f32, cp: u32) -> usize {
    // (an outline face's size in quarter pixels, never under nothing: a negative number here is a pixel face's,
    // below, and a text drawn at a size under nothing has no glyphs to show)
    let size4 = if k_unit(ttf) != 0 { -pixel_k(st, ttf, px) } else { (px.max(0.0) * 4.0).round_ties_even() as i32 };
    let hsh = cp.wrapping_mul(2654435761) ^ (size4 as u32).wrapping_mul(40503) ^ (ttf as u32).wrapping_mul(977);
    for probe in 0..16u32 {
        let slot = (hsh.wrapping_add(probe) % CACHE_SLOTS as u32) as usize;
        let g = &st.cache[slot];
        if g.used && g.cp == cp && g.size4 == size4 && g.face as usize == ttf {
            return slot;
        }
        if !g.used {
            if st.cache_used > (CACHE_SLOTS * 3 / 4) as i32 || st.cache_bytes > CACHE_BYTES_MAX {
                cache_flush(st);
                return glyph_get(st, ttf, px, cp);
            }
            let (adv, bmp, w, h, xoff, yoff);
            if ttf >= TTF_COUNT {
                (adv, bmp, w, h, xoff, yoff) = own_glyph(st, ttf, size4 as f32 / 4.0, cp);
            } else if size4 < 0 {
                adv = hmetrics(&st.font[ttf], cp).0;
                (bmp, w, h, xoff, yoff) = make_pixel_glyph(st, ttf, -size4, cp);
            } else {
                adv = hmetrics(&st.font[ttf], cp).0;
                let scale = scale_for_pixel_height(&st.font[ttf], size4 as f32 / 4.0);
                (bmp, w, h, xoff, yoff) = codepoint_bitmap(&st.font[ttf], scale, scale, cp);
            }
            st.cache_bytes += bmp.len();
            st.cache[slot] =
                Glyph { used: true, face: ttf as u8, size4, cp, w, h, xoff, yoff, advance: adv, bmp, bold: Vec::new() };
            st.cache_used += 1;
            return slot;
        }
    }
    cache_flush(st);
    glyph_get(st, ttf, px, cp)
}

fn face_of(st: &State, f: FontFace) -> FaceMap {
    // (the C clamps an out-of-range face to FONT_REGULAR; the enum cannot be out of range)
    st.map[f as usize]
}

// ---- text as the C reads it: bytes up to a NUL ----

/// The string a C function sees: the text up to a NUL byte, if it has one.
fn cstr(s: &str) -> &str {
    match s.find('\0') {
        Some(n) => &s[..n],
        None => s,
    }
}

/// Reads s[i] as C reads a terminated string: the byte past the end is the NUL.
#[inline]
fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// Decodes one UTF-8 code point at s[*i..]; advances *i. Invalid bytes become U+FFFD.
fn utf8_next(s: &[u8], i: &mut usize) -> u32 {
    let mut p = *i;
    let mut c = at(s, p) as u32;
    p += 1;
    let mut extra = 0;
    if (0xF0..0xF8).contains(&c) {
        c &= 0x07;
        extra = 3;
    } else if c >= 0xE0 {
        c &= 0x0F;
        extra = 2;
    } else if c >= 0xC0 {
        c &= 0x1F;
        extra = 1;
    } else if c >= 0x80 {
        *i = p;
        return 0xFFFD;
    }
    while extra > 0 {
        extra -= 1;
        if (at(s, p) & 0xC0) != 0x80 {
            *i = p;
            return 0xFFFD;
        }
        c = (c << 6) | (at(s, p) & 0x3F) as u32;
        p += 1;
    }
    *i = p;
    c
}

fn blit(c: &mut Canvas, bmp: &[u8], w: i32, h: i32, gx: i32, gy: i32, gray: u8) {
    let gr = rect_intersect(Rect { x: gx, y: gy, w, h }, c.clip);
    for yy in gr.y..gr.y + gr.h {
        let src = ((yy - gy) * w) as usize;
        let dst = (yy * c.stride) as usize;
        for xx in gr.x..gr.x + gr.w {
            let a = bmp[src + (xx - gx) as usize] as i32;
            if a != 0 {
                let d = &mut c.px[dst + xx as usize];
                *d = (*d as i32 + ((gray as i32 - *d as i32) * a + 127) / 255) as u8;
            }
        }
    }
}

fn blit_glyph(c: &mut Canvas, g: &Glyph, gx: i32, gy: i32, gray: u8) {
    blit(c, &g.bmp, g.w, g.h, gx, gy, gray);
}

/// Letters the overprint would turn into others: sharp s (U+00DF) closes into a B, the notch
/// between its stem and its bowls being all that tells them apart, and o slash (U+00F8, U+00D8)
/// fills its counter around the slash until it reads as a theta.
fn bold_keeps_gaps(cp: u32) -> bool {
    cp == 0xDF || cp == 0xF8 || cp == 0xD8
}

/// The bold weight of a pixel glyph above U+007F, made on first use. Bold is the glyph printed
/// again one design pixel to the right, which closes every one-pixel gap: the dots of u-umlaut
/// run together into a macron, the tilde and the circumflex fill in. So here the letter takes the
/// overprint as before, but a detached mark (a component wholly above or below the letter's
/// largest part: an accent, a cedilla that does not touch) is drawn as designed when the
/// overprint would close a one-pixel gap in it. Marks it cannot harm (an acute, the dot of an
/// inverted exclamation mark) keep the overprint. In the letters of bold_keeps_gaps a one-pixel
/// gap moves one design pixel right instead of closing: the stroke on its left takes the
/// overprint, the one on its right keeps its width. ASCII never comes here: it draws exactly as
/// it always has. Returns whether g.bold holds the bold weight (false: plain overprint).
fn pixel_bold(g: &mut Glyph) -> bool {
    if !g.bold.is_empty() || g.bmp.is_empty() || g.size4 >= 0 {
        return !g.bold.is_empty();
    }
    let k = -g.size4;
    let (w, h) = (g.w / k, g.h / k);
    let n = (w * h) as usize;
    // Per design pixel: its component (0 = paper), then a flood-fill stack; per component: its
    // first and last row, its size and its zone (0 the letter, 1 a mark above, 2 a mark below).
    let mut mem = vec![0i32; n * 6];
    let mut out = vec![0u8; (w as usize + 1) * h as usize];
    let (lab, rest) = mem.split_at_mut(n);
    let (stack, rest) = rest.split_at_mut(n);
    let (top, rest) = rest.split_at_mut(n);
    let (bot, rest) = rest.split_at_mut(n);
    let (size, zone) = rest.split_at_mut(n);
    let gw = g.w;
    let bmp = &g.bmp;
    let ink = |x: i32, y: i32| bmp[(y * k * gw + x * k) as usize] != 0;
    let mut nc = 0usize;
    let mut big = 0usize;
    for i in 0..n {
        if lab[i] != 0 || !ink(i as i32 % w, i as i32 / w) {
            continue;
        }
        let id = nc;
        nc += 1;
        let mut sp = 0usize;
        top[id] = h;
        bot[id] = -1;
        lab[i] = id as i32 + 1;
        stack[sp] = i as i32;
        sp += 1;
        while sp != 0 {
            sp -= 1;
            let p = stack[sp];
            let (px, py) = (p % w, p / w);
            size[id] += 1;
            if py < top[id] {
                top[id] = py;
            }
            if py > bot[id] {
                bot[id] = py;
            }
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (qx, qy) = (px + dx, py + dy);
                    if qx < 0 || qy < 0 || qx >= w || qy >= h {
                        continue;
                    }
                    let q = (qy * w + qx) as usize;
                    if lab[q] == 0 && ink(qx, qy) {
                        lab[q] = id as i32 + 1;
                        stack[sp] = q as i32;
                        sp += 1;
                    }
                }
            }
        }
        if size[id] > size[big] {
            big = id;
        }
    }
    for i in 0..nc {
        zone[i] = if bot[i] < top[big] {
            1
        } else if top[i] > bot[big] {
            2
        } else {
            0
        };
    }
    // Marks (above, below) keep their designed shape if the overprint would fill a gap one
    // design pixel wide in them: paper between two inked pixels of a row.
    let mut plain = [false; 3];
    for y in 0..h {
        for x in 0..(w - 2).max(0) {
            let l = lab[(y * w + x) as usize];
            if l != 0 && zone[l as usize - 1] != 0 && lab[(y * w + x + 1) as usize] == 0 && lab[(y * w + x + 2) as usize] != 0 {
                plain[zone[l as usize - 1] as usize] = true;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let l = lab[(y * w + x) as usize];
            if l == 0 {
                continue;
            }
            out[(y * (w + 1) + x) as usize] = 1;
            if !plain[zone[l as usize - 1] as usize] {
                out[(y * (w + 1) + x + 1) as usize] = 1;
            }
        }
    }
    if bold_keeps_gaps(g.cp) {
        for y in 0..h {
            for x in 1..(w - 1).max(1) {
                if lab[(y * w + x) as usize] == 0 && lab[(y * w + x - 1) as usize] != 0 && lab[(y * w + x + 1) as usize] != 0 {
                    out[(y * (w + 1) + x + 1) as usize] = 0;
                }
            }
        }
    }
    let bw = (w + 1) * k;
    let mut bold = vec![0u8; bw as usize * g.h as usize];
    for y in 0..g.h {
        for x in 0..bw {
            bold[(y * bw + x) as usize] = if out[((y / k) * (w + 1) + x / k) as usize] != 0 { 255 } else { 0 };
        }
    }
    g.bold = bold;
    true
}

/// Horizontal font units -> screen pixels for a face at a size (pixel faces: whole design
/// pixels, so every glyph starts on the grid).
fn units_to_px(st: &State, ttf: usize, gpx: f32, units: i32) -> f32 {
    if k_unit(ttf) != 0 {
        return (((units as f32) / (k_unit(ttf) as f32)).round_ties_even() as i64 * pixel_k(st, ttf, gpx) as i64) as f32;
    }
    face_scale(st, ttf, gpx) * units as f32
}

/// The face and size cp is drawn in under m (px asked, fpx px times m's scale), and whether that face has
/// it. Glyphs a theme face or a family lacks (symbols, arrows, Greek, Japanese) come from Terminess at the
/// asked size (a bold face's from its Bold), or, when Terminess lacks them too, from DotGothic16. A custom
/// font draws the glyphs it has and that passed its check, at the sizes the check drew; every other one is
/// borrowed the same way, ASCII too. False: no face has it (the face returned is Terminess, which lacks
/// it). Printable ASCII is taken as there in every embedded face.
fn pick_face(st: &mut State, m: FaceMap, px: f32, fpx: f32, cp: u32) -> (usize, f32, bool) {
    let mut ttf = m.ttf;
    let mut gpx = fpx;
    let lacks = if ttf >= TTF_COUNT {
        match own_px(st, ttf, px, fpx, cp) {
            Some(g) if face_has(st, ttf, cp) => {
                gpx = g;
                false
            }
            _ => true,
        }
    } else {
        !(0x20..0x7F).contains(&cp) && find_glyph_index(&st.font[ttf], cp) == 0
    };
    if !lacks {
        return (ttf, gpx, true);
    }
    if ttf != TTF_TERM && ttf != TTF_TERM_BOLD {
        ttf = m.fallback;
        gpx = px;
    }
    if find_glyph_index(&st.font[ttf], cp) != 0 {
        return (ttf, gpx, true);
    }
    if find_glyph_index(&st.font[TTF_JP], cp) != 0 {
        return (TTF_JP, px, true);
    }
    (ttf, gpx, false)
}

/// What a character no face has shows as (2.1.0-beta.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Missing {
    /// whitespace: the room of a space (U+0020), drawn as one
    Space,
    /// a control, format or default-ignorable character, a combining mark: nothing, and no room
    Nothing,
    /// anything else (a letter, an emoji, a symbol): the box (missing_box), one for each code point
    Box,
}

/// Unicode 17.0's general categories Cc, Cf, Mn and Me (controls, format characters, nonspacing and
/// enclosing marks: the variation selectors are Mn, the tags Cf) and its Default_Ignorable_Code_Point
/// (besides those, the Hangul fillers and the unassigned rest of U+E0000 to U+E0FFF), as sorted ranges,
/// first and last. Made with Python's unicodedata (unidata_version 17.0.0): every code point whose
/// category() is one of the four, with the 17 Default_Ignorable_Code_Point ranges of
/// DerivedCoreProperties.txt (which unicodedata does not carry) added by hand, merged where they touch.
const UNSEEN: [(u32, u32); 378] = [
    (0x0, 0x1F), (0x7F, 0x9F), (0xAD, 0xAD), (0x300, 0x36F), (0x483, 0x489), (0x591, 0x5BD), (0x5BF, 0x5BF),
    (0x5C1, 0x5C2), (0x5C4, 0x5C5), (0x5C7, 0x5C7), (0x600, 0x605), (0x610, 0x61A), (0x61C, 0x61C), (0x64B, 0x65F),
    (0x670, 0x670), (0x6D6, 0x6DD), (0x6DF, 0x6E4), (0x6E7, 0x6E8), (0x6EA, 0x6ED), (0x70F, 0x70F), (0x711, 0x711),
    (0x730, 0x74A), (0x7A6, 0x7B0), (0x7EB, 0x7F3), (0x7FD, 0x7FD), (0x816, 0x819), (0x81B, 0x823), (0x825, 0x827),
    (0x829, 0x82D), (0x859, 0x85B), (0x890, 0x891), (0x897, 0x89F), (0x8CA, 0x902), (0x93A, 0x93A), (0x93C, 0x93C),
    (0x941, 0x948), (0x94D, 0x94D), (0x951, 0x957), (0x962, 0x963), (0x981, 0x981), (0x9BC, 0x9BC), (0x9C1, 0x9C4),
    (0x9CD, 0x9CD), (0x9E2, 0x9E3), (0x9FE, 0x9FE), (0xA01, 0xA02), (0xA3C, 0xA3C), (0xA41, 0xA42), (0xA47, 0xA48),
    (0xA4B, 0xA4D), (0xA51, 0xA51), (0xA70, 0xA71), (0xA75, 0xA75), (0xA81, 0xA82), (0xABC, 0xABC), (0xAC1, 0xAC5),
    (0xAC7, 0xAC8), (0xACD, 0xACD), (0xAE2, 0xAE3), (0xAFA, 0xAFF), (0xB01, 0xB01), (0xB3C, 0xB3C), (0xB3F, 0xB3F),
    (0xB41, 0xB44), (0xB4D, 0xB4D), (0xB55, 0xB56), (0xB62, 0xB63), (0xB82, 0xB82), (0xBC0, 0xBC0), (0xBCD, 0xBCD),
    (0xC00, 0xC00), (0xC04, 0xC04), (0xC3C, 0xC3C), (0xC3E, 0xC40), (0xC46, 0xC48), (0xC4A, 0xC4D), (0xC55, 0xC56),
    (0xC62, 0xC63), (0xC81, 0xC81), (0xCBC, 0xCBC), (0xCBF, 0xCBF), (0xCC6, 0xCC6), (0xCCC, 0xCCD), (0xCE2, 0xCE3),
    (0xD00, 0xD01), (0xD3B, 0xD3C), (0xD41, 0xD44), (0xD4D, 0xD4D), (0xD62, 0xD63), (0xD81, 0xD81), (0xDCA, 0xDCA),
    (0xDD2, 0xDD4), (0xDD6, 0xDD6), (0xE31, 0xE31), (0xE34, 0xE3A), (0xE47, 0xE4E), (0xEB1, 0xEB1), (0xEB4, 0xEBC),
    (0xEC8, 0xECE), (0xF18, 0xF19), (0xF35, 0xF35), (0xF37, 0xF37), (0xF39, 0xF39), (0xF71, 0xF7E), (0xF80, 0xF84),
    (0xF86, 0xF87), (0xF8D, 0xF97), (0xF99, 0xFBC), (0xFC6, 0xFC6), (0x102D, 0x1030), (0x1032, 0x1037),
    (0x1039, 0x103A), (0x103D, 0x103E), (0x1058, 0x1059), (0x105E, 0x1060), (0x1071, 0x1074), (0x1082, 0x1082),
    (0x1085, 0x1086), (0x108D, 0x108D), (0x109D, 0x109D), (0x115F, 0x1160), (0x135D, 0x135F), (0x1712, 0x1714),
    (0x1732, 0x1733), (0x1752, 0x1753), (0x1772, 0x1773), (0x17B4, 0x17B5), (0x17B7, 0x17BD), (0x17C6, 0x17C6),
    (0x17C9, 0x17D3), (0x17DD, 0x17DD), (0x180B, 0x180F), (0x1885, 0x1886), (0x18A9, 0x18A9), (0x1920, 0x1922),
    (0x1927, 0x1928), (0x1932, 0x1932), (0x1939, 0x193B), (0x1A17, 0x1A18), (0x1A1B, 0x1A1B), (0x1A56, 0x1A56),
    (0x1A58, 0x1A5E), (0x1A60, 0x1A60), (0x1A62, 0x1A62), (0x1A65, 0x1A6C), (0x1A73, 0x1A7C), (0x1A7F, 0x1A7F),
    (0x1AB0, 0x1ADD), (0x1AE0, 0x1AEB), (0x1B00, 0x1B03), (0x1B34, 0x1B34), (0x1B36, 0x1B3A), (0x1B3C, 0x1B3C),
    (0x1B42, 0x1B42), (0x1B6B, 0x1B73), (0x1B80, 0x1B81), (0x1BA2, 0x1BA5), (0x1BA8, 0x1BA9), (0x1BAB, 0x1BAD),
    (0x1BE6, 0x1BE6), (0x1BE8, 0x1BE9), (0x1BED, 0x1BED), (0x1BEF, 0x1BF1), (0x1C2C, 0x1C33), (0x1C36, 0x1C37),
    (0x1CD0, 0x1CD2), (0x1CD4, 0x1CE0), (0x1CE2, 0x1CE8), (0x1CED, 0x1CED), (0x1CF4, 0x1CF4), (0x1CF8, 0x1CF9),
    (0x1DC0, 0x1DFF), (0x200B, 0x200F), (0x202A, 0x202E), (0x2060, 0x206F), (0x20D0, 0x20F0), (0x2CEF, 0x2CF1),
    (0x2D7F, 0x2D7F), (0x2DE0, 0x2DFF), (0x302A, 0x302D), (0x3099, 0x309A), (0x3164, 0x3164), (0xA66F, 0xA672),
    (0xA674, 0xA67D), (0xA69E, 0xA69F), (0xA6F0, 0xA6F1), (0xA802, 0xA802), (0xA806, 0xA806), (0xA80B, 0xA80B),
    (0xA825, 0xA826), (0xA82C, 0xA82C), (0xA8C4, 0xA8C5), (0xA8E0, 0xA8F1), (0xA8FF, 0xA8FF), (0xA926, 0xA92D),
    (0xA947, 0xA951), (0xA980, 0xA982), (0xA9B3, 0xA9B3), (0xA9B6, 0xA9B9), (0xA9BC, 0xA9BD), (0xA9E5, 0xA9E5),
    (0xAA29, 0xAA2E), (0xAA31, 0xAA32), (0xAA35, 0xAA36), (0xAA43, 0xAA43), (0xAA4C, 0xAA4C), (0xAA7C, 0xAA7C),
    (0xAAB0, 0xAAB0), (0xAAB2, 0xAAB4), (0xAAB7, 0xAAB8), (0xAABE, 0xAABF), (0xAAC1, 0xAAC1), (0xAAEC, 0xAAED),
    (0xAAF6, 0xAAF6), (0xABE5, 0xABE5), (0xABE8, 0xABE8), (0xABED, 0xABED), (0xFB1E, 0xFB1E), (0xFE00, 0xFE0F),
    (0xFE20, 0xFE2F), (0xFEFF, 0xFEFF), (0xFFA0, 0xFFA0), (0xFFF0, 0xFFFB), (0x101FD, 0x101FD), (0x102E0, 0x102E0),
    (0x10376, 0x1037A), (0x10A01, 0x10A03), (0x10A05, 0x10A06), (0x10A0C, 0x10A0F), (0x10A38, 0x10A3A),
    (0x10A3F, 0x10A3F), (0x10AE5, 0x10AE6), (0x10D24, 0x10D27), (0x10D69, 0x10D6D), (0x10EAB, 0x10EAC),
    (0x10EFA, 0x10EFF), (0x10F46, 0x10F50), (0x10F82, 0x10F85), (0x11001, 0x11001), (0x11038, 0x11046),
    (0x11070, 0x11070), (0x11073, 0x11074), (0x1107F, 0x11081), (0x110B3, 0x110B6), (0x110B9, 0x110BA),
    (0x110BD, 0x110BD), (0x110C2, 0x110C2), (0x110CD, 0x110CD), (0x11100, 0x11102), (0x11127, 0x1112B),
    (0x1112D, 0x11134), (0x11173, 0x11173), (0x11180, 0x11181), (0x111B6, 0x111BE), (0x111C9, 0x111CC),
    (0x111CF, 0x111CF), (0x1122F, 0x11231), (0x11234, 0x11234), (0x11236, 0x11237), (0x1123E, 0x1123E),
    (0x11241, 0x11241), (0x112DF, 0x112DF), (0x112E3, 0x112EA), (0x11300, 0x11301), (0x1133B, 0x1133C),
    (0x11340, 0x11340), (0x11366, 0x1136C), (0x11370, 0x11374), (0x113BB, 0x113C0), (0x113CE, 0x113CE),
    (0x113D0, 0x113D0), (0x113D2, 0x113D2), (0x113E1, 0x113E2), (0x11438, 0x1143F), (0x11442, 0x11444),
    (0x11446, 0x11446), (0x1145E, 0x1145E), (0x114B3, 0x114B8), (0x114BA, 0x114BA), (0x114BF, 0x114C0),
    (0x114C2, 0x114C3), (0x115B2, 0x115B5), (0x115BC, 0x115BD), (0x115BF, 0x115C0), (0x115DC, 0x115DD),
    (0x11633, 0x1163A), (0x1163D, 0x1163D), (0x1163F, 0x11640), (0x116AB, 0x116AB), (0x116AD, 0x116AD),
    (0x116B0, 0x116B5), (0x116B7, 0x116B7), (0x1171D, 0x1171D), (0x1171F, 0x1171F), (0x11722, 0x11725),
    (0x11727, 0x1172B), (0x1182F, 0x11837), (0x11839, 0x1183A), (0x1193B, 0x1193C), (0x1193E, 0x1193E),
    (0x11943, 0x11943), (0x119D4, 0x119D7), (0x119DA, 0x119DB), (0x119E0, 0x119E0), (0x11A01, 0x11A0A),
    (0x11A33, 0x11A38), (0x11A3B, 0x11A3E), (0x11A47, 0x11A47), (0x11A51, 0x11A56), (0x11A59, 0x11A5B),
    (0x11A8A, 0x11A96), (0x11A98, 0x11A99), (0x11B60, 0x11B60), (0x11B62, 0x11B64), (0x11B66, 0x11B66),
    (0x11C30, 0x11C36), (0x11C38, 0x11C3D), (0x11C3F, 0x11C3F), (0x11C92, 0x11CA7), (0x11CAA, 0x11CB0),
    (0x11CB2, 0x11CB3), (0x11CB5, 0x11CB6), (0x11D31, 0x11D36), (0x11D3A, 0x11D3A), (0x11D3C, 0x11D3D),
    (0x11D3F, 0x11D45), (0x11D47, 0x11D47), (0x11D90, 0x11D91), (0x11D95, 0x11D95), (0x11D97, 0x11D97),
    (0x11EF3, 0x11EF4), (0x11F00, 0x11F01), (0x11F36, 0x11F3A), (0x11F40, 0x11F40), (0x11F42, 0x11F42),
    (0x11F5A, 0x11F5A), (0x13430, 0x13440), (0x13447, 0x13455), (0x1611E, 0x16129), (0x1612D, 0x1612F),
    (0x16AF0, 0x16AF4), (0x16B30, 0x16B36), (0x16F4F, 0x16F4F), (0x16F8F, 0x16F92), (0x16FE4, 0x16FE4),
    (0x1BC9D, 0x1BC9E), (0x1BCA0, 0x1BCA3), (0x1CF00, 0x1CF2D), (0x1CF30, 0x1CF46), (0x1D167, 0x1D169),
    (0x1D173, 0x1D182), (0x1D185, 0x1D18B), (0x1D1AA, 0x1D1AD), (0x1D242, 0x1D244), (0x1DA00, 0x1DA36),
    (0x1DA3B, 0x1DA6C), (0x1DA75, 0x1DA75), (0x1DA84, 0x1DA84), (0x1DA9B, 0x1DA9F), (0x1DAA1, 0x1DAAF),
    (0x1E000, 0x1E006), (0x1E008, 0x1E018), (0x1E01B, 0x1E021), (0x1E023, 0x1E024), (0x1E026, 0x1E02A),
    (0x1E08F, 0x1E08F), (0x1E130, 0x1E136), (0x1E2AE, 0x1E2AE), (0x1E2EC, 0x1E2EF), (0x1E4EC, 0x1E4EF),
    (0x1E5EE, 0x1E5EF), (0x1E6E3, 0x1E6E3), (0x1E6E6, 0x1E6E6), (0x1E6EE, 0x1E6EF), (0x1E6F5, 0x1E6F5),
    (0x1E8D0, 0x1E8D6), (0x1E944, 0x1E94A), (0xE0000, 0xE0FFF),
];

fn missing_as(cp: u32) -> Missing {
    if char::from_u32(cp).is_some_and(char::is_whitespace) {
        return Missing::Space;
    }
    let unseen = UNSEEN
        .binary_search_by(|&(a, b)| {
            if b < cp {
                std::cmp::Ordering::Less
            } else if a > cp {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok();
    if unseen {
        Missing::Nothing
    } else {
        Missing::Box
    }
}

/// The box a visible character no face has is drawn as (2.1.0-beta.2): a hollow rectangle in a cell of
/// Terminess at the asked size (the room the font's own missing-glyph box took before), inset 0.15 of the
/// cell on each side, standing on the baseline, 0.6 of the line tall, its sides a 24th of the size (at least
/// a pixel). The same in every face, weight and theme: never overprinted, whatever the face's bold. Returns
/// the cell's advance, the box (x from the pen, y from the baseline) and its sides' width.
fn missing_box(st: &State, px: f32) -> (f32, Rect, i32) {
    let adv = face_scale(st, TTF_TERM, px) * hmetrics(&st.font[TTF_TERM], '0' as u32).0 as f32;
    let inset = (adv * 0.15).round_ties_even() as i32;
    let h = ((px * 0.6).round_ties_even() as i32).max(1);
    let t = ((px / 24.0).round_ties_even() as i32).max(1);
    (adv, Rect { x: inset, y: -h, w: (adv.round_ties_even() as i32 - 2 * inset).max(1), h }, t)
}

/// One character of a text as `layout` measures it: what it adds to the pen, so that a part of the text can be
/// measured again by adding the same numbers in the same order (`Word`), without laying it out again.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// a character that takes no room: the kerning goes on from the character before it
    Unseen,
    /// the box drawn for a character no face has: its advance; nothing is kerned against it
    Boxed(f32),
    /// a glyph: the kerning added before it when it follows a character of its face, and its advance
    Glyph(Option<f32>, f32),
}

/// Lays out (and with a canvas, draws) the text from s[from] up to s[end] (None: its terminator).
/// Measuring (no canvas) reads the advances straight from the font: no glyph is rendered or cached
/// for it. With `steps`, each character's end and what it adds to the pen are kept there.
#[allow(clippy::too_many_arguments)]
fn layout(
    st: &mut State,
    mut c: Option<&mut Canvas>,
    f: FontFace,
    px: f32,
    x: i32,
    baseline: i32,
    s: &[u8],
    from: usize,
    end: Option<usize>,
    gray: u8,
    mut steps: Option<&mut Vec<(usize, Step)>>,
) -> i32 {
    if !st.ready {
        return 0;
    }
    let m = face_of(st, f);
    let fpx = px * m.scale;
    // Overprinting a second copy to the right reads as bold on e-ink: by one design pixel for a
    // pixel face, by a size-relative amount for an outline face.
    let bold = if !m.embolden {
        0
    } else if k_unit(m.ttf) != 0 {
        pixel_k(st, m.ttf, fpx)
    } else {
        (1.0f32).max((fpx / 34.0).round_ties_even() as i64 as f32) as i32
    };
    let mut pen = x as f32;
    let mut prev = 0u32;
    let mut prev_ttf: i32 = -1;
    let mut i = from;
    while at(s, i) != 0 && end.is_none_or(|e| i < e) {
        let mut cp = utf8_next(s, &mut i);
        let (mut ttf, mut gpx, has) = pick_face(st, m, px, fpx, cp);
        // What no face has: whitespace takes a space's room, what is meant to be unseen (a control,
        // format or default-ignorable character, a combining mark) none, anything else the box, measured
        // as it is drawn.
        if !has {
            match missing_as(cp) {
                Missing::Nothing => {
                    if let Some(v) = steps.as_deref_mut() {
                        v.push((i, Step::Unseen));
                    }
                    continue;
                }
                Missing::Space => {
                    cp = 0x20;
                    (ttf, gpx, _) = pick_face(st, m, px, fpx, cp);
                }
                Missing::Box => {
                    let (adv, r, t) = missing_box(st, px);
                    if let Some(cv) = c.as_deref_mut() {
                        let r = Rect { x: pen.round_ties_even() as i32 + r.x, y: baseline + r.y, ..r };
                        canvas_rect_outline(cv, r, t, gray);
                    }
                    pen += adv;
                    if let Some(v) = steps.as_deref_mut() {
                        v.push((i, Step::Boxed(adv)));
                    }
                    prev = 0;
                    prev_ttf = -1;
                    continue;
                }
            }
        }
        // (kerning is stb's, inside one embedded face; a custom font has none)
        let mut kern = None;
        if prev != 0 && prev_ttf == ttf as i32 && ttf < TTF_COUNT {
            let k = units_to_px(st, ttf, gpx, kern_advance(&st.font[ttf], prev, cp));
            pen += k;
            kern = Some(k);
        }
        let advance;
        if let Some(cv) = c.as_deref_mut() {
            let gi = glyph_get(st, ttf, gpx, cp);
            let g = &mut st.cache[gi];
            advance = g.advance;
            if !g.bmp.is_empty() {
                let gx = pen.round_ties_even() as i32 + g.xoff;
                let gy = baseline + g.yoff;
                // Accented letters in a pixel face's bold spare their marks (pixel_bold); that
                // bitmap already holds the overprint of one design pixel (bold == k px).
                let bb = bold != 0 && cp > 0x7F && k_unit(ttf) != 0 && bold == -g.size4 && pixel_bold(g);
                if bb {
                    blit(cv, &g.bold, g.w + bold, g.h, gx, gy, gray);
                } else {
                    blit_glyph(cv, g, gx, gy, gray);
                    for b in 1..=bold {
                        blit_glyph(cv, g, gx + b, gy, gray);
                    }
                }
            }
        } else {
            advance = face_advance(st, ttf, cp);
        }
        let step = units_to_px(st, ttf, gpx, advance) + bold as f32;
        pen += step;
        if let Some(v) = steps.as_deref_mut() {
            v.push((i, Step::Glyph(kern, step)));
        }
        prev = cp;
        prev_ttf = ttf as i32;
    }
    pen.round_ties_even() as i32 - x
}

/// The width of s[from..end] (end: None, its terminator) at px, drawing nothing.
fn measure_n(f: FontFace, px: f32, s: &[u8], from: usize, end: Option<usize>) -> i32 {
    with(|st| layout(st, None, f, px, 0, 0, s, from, end, 0, None))
}

fn measure(f: FontFace, px: f32, s: &[u8]) -> i32 {
    measure_n(f, px, s, 0, None)
}

#[allow(clippy::too_many_arguments)]
fn draw(c: &mut Canvas, f: FontFace, px: f32, x: i32, baseline_y: i32, s: &[u8], from: usize, end: Option<usize>, gray: u8) -> i32 {
    with(|st| layout(st, Some(c), f, px, x, baseline_y, s, from, end, gray, None))
}

/// Draws UTF-8 text with its baseline at y. Returns advance width in px.
pub fn font_draw(c: &mut Canvas, f: FontFace, px: f32, x: i32, baseline_y: i32, utf8: &str, gray: u8) -> i32 {
    draw(c, f, px, x, baseline_y, cstr(utf8).as_bytes(), 0, None, gray)
}

pub fn font_measure(f: FontFace, px: f32, utf8: &str) -> i32 {
    measure(f, px, cstr(utf8).as_bytes())
}

/// Whether the face (as the current theme maps it) has its own glyph for cp, without either
/// fallback.
pub fn font_face_has_glyph(f: FontFace, cp: u32) -> bool {
    with(|st| {
        let ttf = face_of(st, f).ttf;
        st.ready && face_has(st, ttf, cp)
    })
}

/// Whether it can draw cp: its own glyph, or the Terminess or DotGothic16 fallback.
pub fn font_has_glyph(f: FontFace, cp: u32) -> bool {
    font_face_has_glyph(f, cp)
        || with(|st| {
            st.ready
                && (find_glyph_index(&st.font[TTF_TERM], cp) != 0
                    || find_glyph_index(&st.font[TTF_JP], cp) != 0)
        })
}

/// utf8 without the characters that show nothing: a control, format or default-ignorable character or a
/// combining mark that no face has (every face takes what it lacks from Terminess or DotGothic16; a custom
/// font in use counts for what it has), and without the spaces that leaves doubled, first or last. A visible
/// character no face has stays (2.1.0-beta.2): it shows as a box (missing_box: most Vietnamese letters,
/// simplified Chinese, emoji), never as nothing, so a name never looks cut short. Returns how many
/// characters it left out: 0, and out is utf8 as it is (cut to cap between characters). Before
/// font_init, none.
pub fn font_drawable_copy(utf8: &str, out: &mut String, cap: usize) -> i32 {
    if cap == 0 {
        return 0;
    }
    let s = cstr(utf8).as_bytes();
    let mut buf: Vec<u8> = Vec::new();
    let mut left_out = 0;
    let mut i = 0;
    while at(s, i) != 0 {
        let c = i;
        let cp = utf8_next(s, &mut i);
        let n = i - c;
        if missing_as(cp) == Missing::Nothing
            && with(|st| {
                st.ready
                    && find_glyph_index(&st.font[TTF_TERM], cp) == 0
                    && find_glyph_index(&st.font[TTF_JP], cp) == 0
                    && !own_text_has(st, cp)
            })
        {
            left_out += 1;
            continue;
        }
        if left_out != 0 && s[c] == b' ' && (buf.is_empty() || buf[buf.len() - 1] == b' ') {
            continue;
        }
        if buf.len() + n >= cap {
            break; // the next character would not fit whole
        }
        buf.extend_from_slice(&s[c..c + n]);
    }
    while left_out != 0 && !buf.is_empty() && buf[buf.len() - 1] == b' ' {
        buf.pop();
    }
    // (every piece is a whole character of a &str: the bytes are UTF-8)
    *out = String::from_utf8(buf).unwrap_or_default();
    left_out
}

/// px above baseline
pub fn font_ascent(f: FontFace, px: f32) -> i32 {
    with(|st| {
        if !st.ready {
            return px as i32;
        }
        let m = face_of(st, f);
        let (a, _d) = face_vmetrics(st, m.ttf);
        if k_unit(m.ttf) != 0 {
            return ((a as f32 / k_unit(m.ttf) as f32).round_ties_even() as i32) * pixel_k(st, m.ttf, px * m.scale);
        }
        (a as f32 * face_scale(st, m.ttf, px * m.scale)).round_ties_even() as i32
    })
}

/// px below baseline (positive)
pub fn font_descent(f: FontFace, px: f32) -> i32 {
    with(|st| {
        if !st.ready {
            return 0;
        }
        let m = face_of(st, f);
        let (_a, d) = face_vmetrics(st, m.ttf);
        if k_unit(m.ttf) != 0 {
            return (((-d) as f32 / k_unit(m.ttf) as f32).round_ties_even() as i32) * pixel_k(st, m.ttf, px * m.scale);
        }
        ((-d) as f32 * face_scale(st, m.ttf, px * m.scale)).round_ties_even() as i32
    })
}

/// Whenever text is cut short to fit (font_draw_centered, font_draw_left, font_draw_wrapped), or
/// font_draw_wrapped breaks a word between its letters, the hook hears of it: the text as given,
/// the width it needs (for wrapped text, its last line's, or the broken word's) and the width it
/// has, in px. NULL takes it away. The headless runner's fit check uses it.
pub type FontOverflowHook = fn(utf8: &str, need: i32, have: i32);
pub fn font_set_overflow_hook(hook: Option<FontOverflowHook>) {
    with(|st| st.overflow = hook);
}

/// f run with no overflow hook, and the hook as it was again after it: for text that is not Grimoire's own (a
/// package's, kithost.rs), which no translation's fit check has anything to say about.
pub fn font_without_overflow_hook<R>(f: impl FnOnce() -> R) -> R {
    let was = with(|st| st.overflow.take());
    let r = f();
    with(|st| st.overflow = was);
    r
}

/// Tells the overflow hook, if there is one, outside the state's borrow: the hook may measure.
fn overflow(utf8: &str, need: i32, have: i32) {
    if let Some(hook) = with(|st| st.overflow) {
        hook(utf8, need, have);
    }
}

/// Text s (bytes up to a NUL), truncated with "..." so it measures <= maxw: None when it fits as
/// it is, else the shortened copy (the C's buf; empty where nothing fits). With report (the
/// original text), tells the overflow hook.
fn fit(f: FontFace, px: f32, s: &[u8], maxw: i32, report: Option<&str>) -> Option<Vec<u8>> {
    let w = measure(f, px, s);
    if w <= maxw {
        return None;
    }
    if let Some(text) = report {
        overflow(text, w, maxw);
    }
    // strlcpy_q(buf, s, cap - 4) with the C's 256-byte buffer: the first 251 bytes, cut wherever.
    let orig: Vec<u8> = s.iter().copied().take_while(|&b| b != 0).take(256 - 4 - 1).collect();
    let mut n = orig.len();
    while n > 0 {
        // Step back over a whole UTF-8 sequence.
        loop {
            n -= 1;
            if !(n > 0 && (orig[n] & 0xC0) == 0x80) {
                break;
            }
        }
        let mut buf = orig[..n].to_vec();
        buf.extend_from_slice(b"...");
        if measure(f, px, &buf) <= maxw {
            return Some(buf);
        }
    }
    Some(Vec::new())
}

/// Centered in r both ways; text longer than r is truncated with an ellipsis.
pub fn font_draw_centered(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8) {
    let t = cstr(utf8);
    let fitted = fit(f, px, t.as_bytes(), r.w, Some(t));
    let s: &[u8] = fitted.as_deref().unwrap_or(t.as_bytes());
    let w = measure(f, px, s);
    let asc = font_ascent(f, px);
    let desc = font_descent(f, px);
    let base = r.y + (r.h - (asc + desc)) / 2 + asc;
    draw(c, f, px, r.x + (r.w - w) / 2, base, s, 0, None, gray);
}

fn draw_left(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8, report: bool) {
    let t = cstr(utf8);
    let fitted = fit(f, px, t.as_bytes(), r.w, if report { Some(t) } else { None });
    let s: &[u8] = fitted.as_deref().unwrap_or(t.as_bytes());
    let asc = font_ascent(f, px);
    let desc = font_descent(f, px);
    let base = r.y + (r.h - (asc + desc)) / 2 + asc;
    draw(c, f, px, r.x, base, s, 0, None, gray);
}

/// Left-aligned, vertically centered in r, truncated with ellipsis.
pub fn font_draw_left(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8) {
    draw_left(c, f, px, r, utf8, gray, true);
}

/// The same for a name someone gave (a notebook's, a folder's), which "..." shortens by design:
/// the overflow hook (below) does not hear of it.
pub fn font_draw_left_name(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, gray: u8) {
    draw_left(c, f, px, r, utf8, gray, false);
}

// ---- several lines ----

const WRAP_MAX: usize = 16;

/// A line: the bytes [s, e) of the text.
#[derive(Clone, Copy, Default)]
struct Span {
    s: usize,
    e: usize,
}

fn skip_spaces(t: &[u8], mut s: usize) -> usize {
    while at(t, s) == b' ' {
        s += 1;
    }
    s
}

/// True for a character Japanese breaks lines between: kana, kanji, CJK punctuation and fullwidth
/// forms. (Latin words, including accented ones, must stay whole.) about.rs breaks a version's notes
/// there too.
pub(crate) fn cjk(cp: u32) -> bool {
    (0x2E80..=0x9FFF).contains(&cp) || (0xF900..=0xFAFF).contains(&cp) || (0xFE30..=0xFE4F).contains(&cp) || (0xFF00..=0xFFEF).contains(&cp)
}

/// Where the last CJK character of t[s..e] starts, if it has one: a part of it from a character at or before
/// there has a CJK character, a part from after it has none.
fn last_cjk(t: &[u8], mut s: usize, e: usize) -> Option<usize> {
    let mut last = None;
    while s < e {
        let at = s;
        if cjk(utf8_next(t, &mut s)) {
            last = Some(at);
        }
    }
    last
}

/// A word wider than its line, laid out once: each of its characters' steps (`Step`), from which where each of its
/// lines ends and how wide the rest of it is from a line are summed as `layout` sums them, the same numbers in the
/// same order, so the widths are the ones measuring the text again gives.
struct Word {
    steps: Vec<(usize, Step)>,
    /// every kerning and advance in it is 0 or more: a part of the word is never narrower than a part of that part
    rising: bool,
}

impl Word {
    /// t[s..e], a word of at least one character.
    fn new(f: FontFace, px: f32, t: &[u8], s: usize, e: usize) -> Word {
        let mut steps = Vec::new();
        with(|st| layout(st, None, f, px, 0, 0, t, s, Some(e), 0, Some(&mut steps)));
        if steps.is_empty() {
            // (no font is ready: everything measures 0)
            let mut i = s;
            while i < e {
                utf8_next(t, &mut i);
                steps.push((i, Step::Glyph(None, 0.0)));
            }
        }
        let rising = steps.iter().all(|&(_, step)| match step {
            Step::Unseen => true,
            Step::Boxed(a) => a >= 0.0,
            Step::Glyph(k, a) => k.unwrap_or(0.0) >= 0.0 && a >= 0.0,
        });
        Word { steps, rising }
    }

    /// The pen after a character's step, as `layout` moves it; `first` while no character that takes room has come
    /// yet in the part measured (the first is kerned against nothing).
    fn on(pen: f32, first: &mut bool, step: Step) -> f32 {
        match step {
            Step::Unseen => pen,
            Step::Boxed(a) => {
                *first = false;
                pen + a
            }
            Step::Glyph(k, a) => {
                let pen = match k {
                    Some(k) if !*first => pen + k,
                    _ => pen,
                };
                *first = false;
                pen + a
            }
        }
    }

    /// The width of the word from its character `k` to its end.
    fn width(&self, k: usize) -> i32 {
        let (mut pen, mut first) = (0.0f32, true);
        for &(_, step) in &self.steps[k..] {
            pen = Self::on(pen, &mut first, step);
        }
        pen.round_ties_even() as i32
    }

    /// Whether the word from its character `k` to its end is wider than `w`. In a rising word the sum stops at the
    /// first character that takes it past `w`, since nothing after can bring it back.
    fn wider(&self, k: usize, w: i32) -> bool {
        if !self.rising {
            return self.width(k) > w;
        }
        let (mut pen, mut first) = (0.0f32, true);
        for &(_, step) in &self.steps[k..] {
            pen = Self::on(pen, &mut first, step);
            if pen.round_ties_even() as i32 > w {
                return true;
            }
        }
        false
    }

    /// Where a line of the word from its character `k` ends: after as many characters as fit `w`, one at least (the
    /// character after the last on it).
    fn line_end(&self, k: usize, w: i32) -> usize {
        let mut first = true;
        let mut pen = Self::on(0.0, &mut first, self.steps[k].1);
        let mut j = k + 1;
        while j < self.steps.len() {
            let mut then = first;
            let next = Self::on(pen, &mut then, self.steps[j].1);
            if next.round_ties_even() as i32 > w {
                break;
            }
            (pen, first) = (next, then);
            j += 1;
        }
        j
    }
}

/// As many whole words on each line as fit w; a word wider than w on its own is broken between its
/// characters (at least one to a line), and *broken gets the widest such word's width (else 0).
/// A CJK run is not reported: Japanese has no spaces, and breaks between its characters by rule.
/// Keeps up to max lines; returns how many the text needs.
fn wrap_greedy(f: FontFace, px: f32, t: &[u8], w: i32, out: &mut [Span], max: usize, broken: &mut i32) -> usize {
    let mut n = 0;
    *broken = 0;
    let mut s = skip_spaces(t, 0);
    while at(t, s) != 0 {
        let mut fits: Option<usize> = None; // the end of the longest run of words that fits
        let mut first_w = 0; // the width of the line's first word, when it alone does not fit
        let mut e = s;
        while at(t, skip_spaces(t, e)) != 0 {
            let mut we = skip_spaces(t, e);
            while at(t, we) != 0 && at(t, we) != b' ' {
                we += 1;
            }
            let lw = measure_n(f, px, t, s, Some(we));
            if lw > w {
                first_w = lw;
                break;
            }
            fits = Some(we);
            e = we;
        }
        if let Some(fits) = fits {
            if n < max {
                out[n] = Span { s, e: fits };
            }
            n += 1;
            s = skip_spaces(t, fits);
            continue;
        }
        // A word wider than the line on its own is laid out once (`Word`) and broken between its characters into
        // lines of as many as fit, one at least, until the rest of it fits a line, where the words after it go on.
        // *broken takes the width of the word, or of the rest of it from one of its lines, without a CJK character.
        let mut we = s;
        while at(t, we) != 0 && at(t, we) != b' ' {
            we += 1;
        }
        let word = Word::new(f, px, t, s, we);
        let cjk_at = last_cjk(t, s, we);
        let (mut k, mut ls, mut rest_w) = (0, s, Some(first_w));
        // (in a rising word the rest from a later line is never wider: the first rest without CJK is the widest)
        let mut reported = false;
        loop {
            if !cjk_at.is_some_and(|c| c >= ls) && !(word.rising && reported) {
                let ww = rest_w.unwrap_or_else(|| word.width(k));
                if ww > *broken {
                    *broken = ww;
                }
                reported = true;
            }
            let j = word.line_end(k, w);
            let le = word.steps[j - 1].0;
            if n < max {
                out[n] = Span { s: ls, e: le };
            }
            n += 1;
            if j == word.steps.len() {
                s = skip_spaces(t, le);
                break;
            }
            (k, ls, rest_w) = (j, le, None);
            if !word.wider(k, w) {
                s = ls;
                break;
            }
        }
    }
    n
}

/// The two-line break that keeps the longer line shortest, or one right after punctuation, where a
/// clause ends ("Narrow," over "no margin"), whose longer line is at most half again as long: a
/// first clause much shorter than the rest stays on the line with it. False if no break makes two
/// lines that fit.
fn wrap_two(f: FontFace, px: f32, t: &[u8], w: i32, out: &mut [Span]) -> bool {
    let s = skip_spaces(t, 0);
    let mut end = t.len(); // s + strlen(s)
    while end > s && t[end - 1] == b' ' {
        end -= 1;
    }
    let mut best = 0.0f32;
    let mut found = false;
    let mut b = s;
    while b < end {
        if at(t, b) != b' ' || t[b - 1] == b' ' {
            b += 1;
            continue;
        }
        let s2 = skip_spaces(t, b);
        let w1 = measure_n(f, px, t, s, Some(b));
        let w2 = measure_n(f, px, t, s2, Some(end));
        if w1 > w || w2 > w {
            b += 1;
            continue;
        }
        let score = (if w1 > w2 { w1 } else { w2 }) as f32 * (if b",.;:!?)".contains(&t[b - 1]) { 2.0f32 / 3.0 } else { 1.0 });
        if !found || score < best {
            best = score;
            out[0] = Span { s, e: b };
            out[1] = Span { s: s2, e: end };
            found = true;
        }
        b += 1;
    }
    found
}

/// The lines font_draw_wrapped breaks text into for width w: how many it needs, up to max kept;
/// *broken as for wrap_greedy (two lines broken at a space never break a word: each fits w).
fn wrap(f: FontFace, px: f32, t: &[u8], w: i32, out: &mut [Span], max: usize, broken: &mut i32) -> usize {
    let n = wrap_greedy(f, px, t, w, out, max, broken);
    if n == 2 && max >= 2 {
        wrap_two(f, px, t, w, out);
    }
    n
}

/// How many lines font_draw_wrapped needs for the text in width w (it draws at most its
/// max_lines), and the width of the widest of them in *widest (may be NULL).
pub fn font_wrap_lines(f: FontFace, px: f32, utf8: &str, w: i32, widest: Option<&mut i32>) -> i32 {
    // (the C writes through *widest only if it is not NULL; a local takes the writes otherwise)
    let mut unused = 0;
    let widest = widest.unwrap_or(&mut unused);
    *widest = 0;
    if !with(|st| st.ready) {
        return 0;
    }
    let t = cstr(utf8).as_bytes();
    let one = measure(f, px, t);
    if one <= w {
        *widest = one;
        return if at(t, skip_spaces(t, 0)) != 0 { 1 } else { 0 };
    }
    let mut ln = [Span::default(); WRAP_MAX];
    let mut broken = 0;
    let n = wrap(f, px, t, w, &mut ln, WRAP_MAX, &mut broken);
    for l in ln.iter().take(n.min(WRAP_MAX)) {
        let lw = measure_n(f, px, t, l.s, Some(l.e));
        if lw > *widest {
            *widest = lw;
        }
    }
    n as i32
}

/// Up to max_lines lines broken at spaces (a word wider than r on its own is broken between its
/// characters), each centered in r or left-aligned, in slots line_h px apart (0: the face's own
/// height) with the block centered vertically in r. Text that fits on one line is drawn exactly as
/// font_draw_centered or font_draw_left draws it. Two lines are balanced: the break that keeps the
/// longer line shortest, unless one right after punctuation makes it at most half again as long
/// ("Wide, no margin" breaks after its comma). Only text that still does not fit ends in "...".
/// Returns the lines drawn.
pub fn font_draw_wrapped(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, max_lines: i32, line_h: i32, center: bool, gray: u8) -> i32 {
    draw_wrapped(c,f,px,r,utf8,max_lines,line_h,center,gray,true)
}
/// Wrapped user-provided names may be ellipsized without reporting a translation overflow.
pub fn font_draw_wrapped_name(c:&mut Canvas,f:FontFace,px:f32,r:Rect,text:&str,max_lines:i32,gray:u8)->i32 {
    draw_wrapped(c,f,px,r,text,max_lines,0,false,gray,false)
}
fn draw_wrapped(c: &mut Canvas, f: FontFace, px: f32, r: Rect, utf8: &str, max_lines: i32, line_h: i32, center: bool, gray: u8,report:bool) -> i32 {
    if !with(|st| st.ready) {
        return 0;
    }
    let text = cstr(utf8);
    let t = text.as_bytes();
    if max_lines <= 1 || measure(f, px, t) <= r.w {
        if center {
            font_draw_centered(c, f, px, r, utf8, gray);
        } else {
            draw_left(c,f,px,r,utf8,gray,report);
        }
        return 1;
    }
    let max_lines = if max_lines > WRAP_MAX as i32 { WRAP_MAX as i32 } else { max_lines };
    let mut ln = [Span::default(); WRAP_MAX];
    let mut broken = 0;
    let n = wrap(f, px, t, r.w, &mut ln, WRAP_MAX, &mut broken) as i32;
    let shown = if n < max_lines { n } else { max_lines };
    // A word wider than the line, split between two of its letters with no hyphen: the hook hears
    // of it as of a cut (a translation should take a shorter word, or two).
    if report && broken != 0 {
        overflow(text, broken, r.w);
    }
    let asc = font_ascent(f, px);
    let desc = font_descent(f, px);
    let lh = if line_h > 0 { line_h } else { asc + desc };
    let top = r.y + (r.h - shown * lh) / 2;
    for i in 0..shown {
        let base = top + i * lh + (lh - (asc + desc)) / 2 + asc;
        let (s, mut e) = (ln[i as usize].s, Some(ln[i as usize].e));
        let mut fitted: Option<Vec<u8>> = None;
        if n > max_lines && i == shown - 1 {
            // The last line holds the rest of the text, cut short: the hook hears of the whole.
            if report {overflow(text, measure_n(f, px, t, s, None), r.w);}
            fitted = fit(f, px, &t[s..], r.w, None);
            e = None;
        }
        // (the line's text: the original from s, or the shortened copy)
        let (line, from): (&[u8], usize) = match (&fitted, e) {
            (Some(buf), _) => (buf, 0),
            (None, None) => (&t[s..], 0),
            (None, Some(_)) => (t, s),
        };
        let w = measure_n(f, px, line, from, e);
        draw(c, f, px, if center { r.x + (r.w - w) / 2 } else { r.x }, base, line, from, e, gray);
    }
    shown
}

/// The largest size from px down to min_px at which the text fits maxw on one line, among the
/// sizes the face really has: a pixel face changes only at whole design pixels (Jersey 10 looks the
/// same from 27 to 43 px), an outline face in quarter pixels. px if it fits as it is; 0 if even
/// the smallest does not.
pub fn font_fit_px(f: FontFace, px: f32, min_px: f32, utf8: &str, maxw: i32) -> f32 {
    if font_measure(f, px, utf8) <= maxw {
        return px;
    }
    if !with(|st| st.ready) || min_px >= px {
        return 0.0;
    }
    let m = with(|st| face_of(st, f));
    if k_unit(m.ttf) != 0 {
        // Each smaller whole step, at the size it is drawn for, or at min_px inside it.
        let (a, d, _l) = with(|st| vmetrics(&st.font[m.ttf]));
        let line = (a - d) as f32 / k_unit(m.ttf) as f32;
        let mut k = with(|st| pixel_k(st, m.ttf, px * m.scale)) - 1;
        while k >= 1 {
            let size = (k as f32 * line / m.scale).max(min_px);
            if with(|st| pixel_k(st, m.ttf, size * m.scale)) != k {
                break; // min_px lies above this step
            }
            if font_measure(f, size, utf8) <= maxw {
                return size;
            }
            k -= 1;
        }
        return 0.0;
    }
    // An outline face: the largest quarter pixel that fits.
    let mut lo = (min_px * 4.0).ceil() / 4.0;
    let mut hi = px;
    if lo >= hi || font_measure(f, lo, utf8) > maxw {
        return 0.0;
    }
    loop {
        let mid = ((lo + hi) * 2.0).floor() / 4.0;
        if mid <= lo || mid >= hi {
            break;
        }
        if font_measure(f, mid, utf8) <= maxw {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// f run with no icon drawn larger than `side` pixels each way, and the size as it was again after it. An icon
/// whose rectangle is larger is drawn at that size in the middle of the rectangle (font_draw_icon). For a
/// caller that draws rectangles of someone else's choosing and shows an icon in each, whatever its size:
/// outside such a run, the icon of a rectangle so large that its glyph would be past the greatest size
/// (GLYPH_SIDE_MAX) is not drawn.
pub fn font_with_icon_side_max<R>(side: i32, f: impl FnOnce() -> R) -> R {
    let was = with(|st| std::mem::replace(&mut st.icon_side_max, side.max(1)));
    let r = f();
    with(|st| st.icon_side_max = was);
    r
}

/// Draws one glyph (e.g. a Nerd Font icon code point) scaled so its ink box fits r, centered.
pub fn font_draw_icon(c: &mut Canvas, f: FontFace, cp: u32, r: Rect, gray: u8) {
    if !with(|st| st.ready) || rect_is_empty(r) {
        return;
    }
    // (no larger than the caller's largest, in the middle of r: font_with_icon_side_max)
    let side = with(|st| st.icon_side_max);
    let r = if r.w > side || r.h > side {
        let (w, h) = (r.w.min(side), r.h.min(side));
        Rect { x: r.x + (r.w - w) / 2, y: r.y + (r.h - h) / 2, w, h }
    } else {
        r
    };
    // Icons are Terminess Nerd Font glyphs in every theme.
    let ttf = if f == FontFace::Bold || f == FontFace::FixedBold { TTF_TERM_BOLD } else { TTF_TERM };
    with(|st| {
        let fi = &st.font[ttf];
        let Some((x0, y0, x1, y1)) = codepoint_box(fi, cp) else { return };
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        // Pick the pixel height whose scale fits the glyph's ink box into r.
        let unit = scale_for_pixel_height(fi, 100.0) / 100.0; // scale per px of height
        let sx = r.w as f32 / ((x1 - x0) as f32 * unit);
        let sy = r.h as f32 / ((y1 - y0) as f32 * unit);
        let px = (sx.min(sy) * 4.0).floor() / 4.0;
        let gi = glyph_get(st, ttf, px, cp);
        let g = &st.cache[gi];
        if g.bmp.is_empty() {
            return;
        }
        blit_glyph(c, g, r.x + (r.w - g.w) / 2, r.y + (r.h - g.h) / 2, gray);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn up() {
        assert_eq!(font_init(), 0);
        ui_set_theme(UiTheme::Classic);
    }

    thread_local! {
        static SEEN: RefCell<Vec<(String, i32, i32)>> = const { RefCell::new(Vec::new()) };
    }
    fn hear(text: &str, need: i32, have: i32) {
        SEEN.with(|s| s.borrow_mut().push((text.to_string(), need, have)));
    }

    // font_with_icon_side_max: within it an icon in a rectangle larger than the size is the icon of a square
    // of that size in the rectangle's middle, pixel for pixel; one in a smaller rectangle is as ever; and after
    // it an icon is as large as its rectangle again.
    #[test]
    fn an_icon_is_no_larger_than_the_callers_largest() {
        up();
        let cp = 0xF0493; // md-cog
        let icon = |r: Rect, side: Option<i32>| {
            let mut c = Canvas::default();
            assert_eq!(canvas_init(&mut c, 900, 900), 0);
            match side {
                Some(side) => font_with_icon_side_max(side, || font_draw_icon(&mut c, FontFace::Regular, cp, r, 0)),
                None => font_draw_icon(&mut c, FontFace::Regular, cp, r, 0),
            }
            c.px
        };
        let large = Rect { x: 50, y: 100, w: 800, h: 700 };
        let middle = Rect { x: 50 + (800 - 200) / 2, y: 100 + (700 - 200) / 2, w: 200, h: 200 };
        let small = Rect { x: 300, y: 300, w: 120, h: 90 };
        assert!(icon(large, Some(200)) == icon(middle, None));
        assert!(icon(large, Some(200)) != icon(large, None));
        assert!(icon(small, Some(200)) == icon(small, None), "a smaller icon is drawn as ever");
        assert!(icon(Rect { x: 0, y: 0, w: 2_000_000, h: 2_000_000 }, Some(200)) == icon(Rect { x: 999_900, y: 999_900, w: 200, h: 200 }, None));
        // (one way larger: the size is each way's)
        assert!(icon(Rect { x: 50, y: 100, w: 800, h: 90 }, Some(200)) == icon(Rect { x: 350, y: 100, w: 200, h: 90 }, None));
        // The size is the one before again after the run, also when runs are inside one another.
        let nested = font_with_icon_side_max(300, || {
            let inner = font_with_icon_side_max(200, || icon(large, None));
            (inner, icon(large, None))
        });
        assert!(nested.0 == icon(middle, None));
        assert!(nested.1 == icon(Rect { x: 50 + 250, y: 100 + 200, w: 300, h: 300 }, None));
        assert!(icon(large, None) != nested.1, "after the run an icon is as large as its rectangle");
    }

    // An outline face at a size under nothing has no glyphs to show: the text draws nothing, in every face of
    // Classic, at every such size a caller reaches that works a label's size out from its rectangle's height
    // (globalui.rs's chip and select, ui.rs's button: a rectangle a few pixels high, or one whose height is far
    // under nothing).
    #[test]
    fn an_outline_face_at_a_size_under_nothing_draws_nothing() {
        up();
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, 600, 200), 0);
        for f in [FontFace::Regular, FontFace::Bold, FontFace::Title, FontFace::Caps, FontFace::Fixed, FontFace::FixedBold] {
            for px in [-0.25, -0.46, -1.0, -30.0, -60_000.0, -80_000.0, 0.0] {
                font_draw(&mut c, f, px, 20, 120, "Label 12:34", 0);
                font_draw_centered(&mut c, f, px, Rect { x: 0, y: 0, w: 600, h: 200 }, "Label 12:34", 0);
                assert!(c.px.iter().all(|p| *p == 255), "{f:?} at {px}");
            }
            font_draw(&mut c, f, 30.0, 20, 120, "Label 12:34", 0);
            assert!(c.px.iter().any(|p| *p != 255), "{f:?} at 30 px draws");
            c.px.fill(255);
        }
    }

    // ---- a glyph's bitmap has a greatest size (GLYPH_SIDE_MAX, CACHE_BYTES_MAX) ----

    /// The glyphs the cache holds: each one's code point, size key, sides and bytes.
    fn cached() -> Vec<(u32, i32, i32, i32, usize)> {
        with(|st| st.cache.iter().filter(|g| g.used).map(|g| (g.cp, g.size4, g.w, g.h, g.bmp.len())).collect())
    }

    /// What holds of the cache whatever was drawn: a glyph's bytes are the product of its sides, every glyph is
    /// of a size a glyph may have, and all of them together are no more than the cache's bytes and one glyph's.
    fn the_cache_is_within_its_sizes(what: &str) {
        let glyphs = cached();
        for (cp, size4, w, h, len) in &glyphs {
            assert_eq!(*len as i64, *w as i64 * *h as i64, "{what}: U+{cp:04X} at {size4}");
            assert!(glyph_size_ok(*w as i64, *h as i64), "{what}: U+{cp:04X} at {size4} is {w} by {h}");
        }
        let all: usize = glyphs.iter().map(|g| g.4).sum();
        assert_eq!(all, with(|st| st.cache_bytes), "{what}");
        assert!(all <= 2 * CACHE_BYTES_MAX, "{what}: {all} bytes of glyphs");
    }

    /// How many bitmaps stb has been asked for so far (a glyph past the greatest size is not asked for).
    fn asked() -> u32 {
        font_bitmaps_asked()
    }

    /// How many pixels of the canvas are not paper, and how many of those lie outside `keep`.
    fn ink(c: &Canvas, keep: Rect) -> (usize, usize) {
        let (mut all, mut outside) = (0, 0);
        for y in 0..c.h {
            for x in 0..c.w {
                if c.px[(y * c.stride + x) as usize] != 255 {
                    all += 1;
                    outside += usize::from(!rect_contains(keep, x, y));
                }
            }
        }
        (all, outside)
    }

    // The rule itself, by its numbers: a side is at most 4,096 px, and the product of the two sides, worked out
    // in 64 bits, at most 4 MiB. At the size, one past it and far past it; squares, one long side, the greatest
    // numbers, and sides of nothing or under it.
    #[test]
    fn a_glyphs_bitmap_has_a_greatest_size() {
        assert_eq!((GLYPH_SIDE_MAX, CACHE_BYTES_MAX), (4096, 4_194_304));
        let (most, least) = (i64::MAX, i64::MIN);
        let made: [(i64, i64); 14] =
            [(1, 1), (64, 64), (1024, 1024), (2047, 2049), (2048, 2048), (4096, 1024), (1024, 4096), (4096, 1), (1, 4096), (3000, 1398), (0, 0), (0, 4096), (4096, 0), (0, 1)];
        let not_made: [(i64, i64); 34] = [
            // squares
            (2049, 2049),
            (2896, 2896),
            (4096, 4096),
            (4097, 4097),
            (133_000, 133_000),
            (2_000_000_000, 2_000_000_000),
            // one pixel past, either way
            (2049, 2048),
            (2048, 2049),
            (4096, 1025),
            (1025, 4096),
            (3000, 1399),
            // one long side
            (4097, 1),
            (1, 4097),
            (4097, 1024),
            (133_000, 1),
            (1, 133_000),
            (133_000, 1024),
            (1024, 133_000),
            (0, 4097),
            (4097, 0),
            // sides far past the greatest, whose product is told in 64 bits
            (46_341, 46_341),
            (65_536, 65_536),
            (65_536, 65_537),
            (100_000, 90_000),
            (1 << 31, 2),
            (1 << 32, 1 << 32),
            // the greatest numbers
            (i32::MAX as i64, i32::MAX as i64),
            (most, most),
            (most, 1),
            (1, most),
            // a side under nothing
            (-1, 10),
            (10, -1),
            (-65_536, -65_536),
            (least, least),
        ];
        for (w, h) in made {
            assert!(glyph_size_ok(w, h), "{w} by {h} is a size a glyph may have");
        }
        for (w, h) in not_made {
            assert!(!glyph_size_ok(w, h), "{w} by {h} is no size a glyph has");
        }
    }

    // An outline face's glyph is made up to the greatest size and no larger. For letters and an icon of several
    // embedded files: the tallest text, to the quarter pixel, at which the glyph is made is of a size a glyph may
    // have and fills it (a side, or the bytes); a quarter pixel taller, and at any height past that, no bitmap is
    // made and the answer is that of a glyph with no ink. So is it for a scale that is nothing, under nothing or
    // not a number.
    #[test]
    fn an_outline_glyph_is_made_up_to_the_greatest_size_and_no_larger() {
        up();
        // (the bitmap of cp in the file at a text height, as glyph_get makes it, and whether stb was asked for it)
        let at = |ttf: usize, px: f32, cp: u32| {
            let before = asked();
            let made = with(|st| {
                let scale = scale_for_pixel_height(&st.font[ttf], px);
                codepoint_bitmap(&st.font[ttf], scale, scale, cp)
            });
            (made, asked() != before)
        };
        // (the answer for a glyph that is not made: no bitmap, no size, no place, and stb was not asked)
        let none = |(made, asked): ((Vec<u8>, i32, i32, i32, i32), bool)| made.0.is_empty() && (made.1, made.2, made.3, made.4) == (0, 0, 0, 0) && !asked;
        let glyphs =
            [(TTF_TERM, 'W' as u32), (TTF_TERM, 'l' as u32), (TTF_TERM, '-' as u32), (TTF_TERM, '|' as u32), (TTF_TERM, 0xF0493), (TTF_TERM_BOLD, 'W' as u32), (TTF_ATKINSON, 'W' as u32), (TTF_LITERATA_BOLD, 'm' as u32), (TTF_JP, 0x6F22)];
        let (mut by_side, mut by_bytes) = (0, 0);
        for (ttf, cp) in glyphs {
            let what = format!("file {ttf}, U+{cp:04X}");
            // (quarter pixels: made at 100 px, not at 12,000 px, and a taller glyph is no smaller either way)
            let (mut lo, mut hi) = (4 * 100, 4 * 12_000);
            assert!(!at(ttf, lo as f32 / 4.0, cp).0 .0.is_empty(), "{what}");
            assert!(none(at(ttf, hi as f32 / 4.0, cp)), "{what}");
            while hi - lo > 1 {
                let mid = (lo + hi) / 2;
                let tried = at(ttf, mid as f32 / 4.0, cp);
                if tried.0 .0.is_empty() {
                    assert!(none(tried), "{what} at {} px", mid as f32 / 4.0);
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            let ((bmp, w, h, _, _), _) = at(ttf, lo as f32 / 4.0, cp);
            assert_eq!(bmp.len(), w as usize * h as usize, "{what}");
            assert!(w <= 4096 && h <= 4096 && w * h <= 4_194_304, "{what}: {w} by {h} at {} px", lo as f32 / 4.0);
            // (the greatest: its longer side is the greatest side to a few pixels, or its bytes the most to 1%)
            let (side, bytes) = (w.max(h) >= 4090, w * h > 4_150_000);
            assert!(side || bytes, "{what}: {w} by {h} at {} px", lo as f32 / 4.0);
            by_side += i32::from(side);
            by_bytes += i32::from(bytes);
            assert!(none(at(ttf, hi as f32 / 4.0, cp)), "{what}: a quarter pixel taller");
            for px in [hi as f32 / 4.0 + 1.0, hi as f32 / 2.0, 150_000.0, 2_147_483.0, 1e9, 1e20, f32::MAX, f32::INFINITY, f32::NAN] {
                assert!(none(at(ttf, px, cp)), "{what} at {px} px");
            }
        }
        assert!(by_side >= 1 && by_bytes >= 1, "both halves of the rule were met: {by_side} by a side, {by_bytes} by the bytes");
        // (a scale of nothing has no bitmap, as ever; one under nothing, past any size or not a number is not asked
        // about)
        for scale in [0.0, -0.0, -0.001, -1.0, -1e9, f32::MIN, f32::MAX, 1e9, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            let before = asked();
            let made = with(|st| codepoint_bitmap(&st.font[TTF_TERM], scale, scale, 'W' as u32));
            assert!(none((made, scale != 0.0 && asked() != before)), "a scale of {scale}");
        }
        // (a glyph with no outline has no bitmap at any size)
        for px in [20.0, 150_000.0, f32::MAX] {
            assert!(at(TTF_TERM, px, ' ' as u32).0 == (Vec::new(), 0, 0, 0, 0), "a space at {px} px");
        }
    }

    // An icon in a rectangle of any size: its glyph is made up to the greatest size and no larger, nothing is
    // drawn outside the rectangle or outside the canvas's clip, and the cache holds no more than its bytes.
    // Squares at the size, past it and far past it, rectangles with one long side, and the greatest numbers.
    #[test]
    fn an_icon_is_drawn_up_to_the_greatest_glyph_and_not_past_it() {
        up();
        let cp = 0xF0493; // md-cog
        let (cx, cy) = (1200, 1200);
        let clip = Rect { x: 60, y: 90, w: 2280, h: 2250 };
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, 2400, 2400), 0);
        // (an icon in a rectangle of w by h about the canvas's middle: the ink it left)
        let mut icon = |w: i32, h: i32| -> usize {
            c.px.fill(255);
            canvas_set_clip(&mut c, clip);
            let r = Rect { x: cx - w / 2, y: cy - h / 2, w, h };
            with(cache_flush);
            let before = asked();
            font_draw_icon(&mut c, FontFace::Regular, cp, r, 0);
            // (a glyph that is made was asked of stb once; one that is not, never)
            let made = cached().into_iter().any(|g| g.0 == cp && g.4 != 0);
            assert_eq!(asked() - before, u32::from(made), "an icon in {w} by {h}");
            // (stb rounds the bitmap out to whole pixels: a pixel over at most, as icons_fit_their_box has it; a
            // rectangle far larger than the canvas holds all of the clip)
            let keep = if w > 1_000_000 || h > 1_000_000 { clip } else { rect_intersect(rect_inflate(r, 1), clip) };
            let (all, outside) = ink(&c, keep);
            assert_eq!(outside, 0, "an icon in {w} by {h}: ink outside its rectangle or the clip");
            the_cache_is_within_its_sizes(&format!("an icon in {w} by {h}"));
            all
        };
        // Squares: drawn up to a side, to the pixel, and not past it.
        for side in [24, 96, 1023, 1024, 1025, 1280, 1281, 2000] {
            assert_ne!(icon(side, side), 0, "an icon in a square of {side} px is drawn");
        }
        let (mut lo, mut hi) = (2000, 6000);
        assert_eq!(icon(hi, hi), 0);
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if icon(mid, mid) == 0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        assert_ne!(icon(lo, lo), 0);
        let (_, _, w, h, len) = cached().into_iter().find(|g| g.0 == cp).unwrap();
        assert_eq!(len, w as usize * h as usize);
        assert!(w.max(h) >= 4090 || w * h > 4_150_000, "the largest icon made, in a square of {lo} px, is {w} by {h}: the greatest glyph");
        for side in [hi, hi + 1, 4096, 4097, 8192, 12_000, 30_000, 58_000, 66_000, 90_000, 100_000, 120_000, 133_000, 2_000_000_000, i32::MAX] {
            assert_eq!(icon(side, side), 0, "an icon in a square of {side} px is not drawn");
        }
        // One long side: the icon is as large as the shorter side lets it be, in the rectangle's middle.
        for (w, h) in [(133_000, 1024), (1024, 133_000), (2_000_000_000, 1280), (1280, i32::MAX), (i32::MAX, 96)] {
            assert_ne!(icon(w, h), 0, "an icon in {w} by {h} is drawn");
        }
        for (w, h) in [(133_000, 1), (1, 133_000), (i32::MAX, 1), (1, i32::MAX)] {
            icon(w, h);
        }
        for (w, h) in [(100_000, 90_000), (90_000, 100_000), (65_536, 65_537), (65_537, 65_536), (133_000, 5000), (5000, 133_000), (i32::MAX, 100_000), (46_341, 46_341)] {
            assert_eq!(icon(w, h), 0, "an icon in {w} by {h} is not drawn");
        }
    }

    /// A text height at which a pixel face draws k screen pixels to a design pixel.
    fn px_of(f: FontFace, k: i32) -> f32 {
        let (ttf, scale) = with(|st| (face_of(st, f).ttf, face_of(st, f).scale));
        let (a, d, _) = with(|st| vmetrics(&st.font[ttf]));
        let px = k as f32 * ((a - d) as f32 / k_unit(ttf) as f32) / scale;
        assert_eq!(with(|st| pixel_k(st, ttf, px * scale)), k, "{f:?} at {px} px");
        px
    }

    /// The glyph of `cp` the cache holds for a size key (an outline face's: quarter pixels of the text's height;
    /// a pixel face's: minus its k): its sides and its bytes.
    fn glyph_of(cp: u32, size4: i32) -> Option<(i32, i32, usize)> {
        cached().into_iter().find(|g| g.0 == cp && g.1 == size4).map(|g| (g.2, g.3, g.4))
    }

    // A text at any height, in an outline face (Classic) and in a pixel face (Gothic): its letters' glyphs are
    // made up to the greatest size and no larger; a letter that is not made leaves its place empty and keeps its
    // advance, so the line is as wide drawn as measured; nothing is drawn outside the canvas's clip; and the
    // cache holds no more than its bytes.
    #[test]
    fn a_text_is_drawn_up_to_the_greatest_glyph_and_keeps_its_layout_past_it() {
        up();
        let clip = Rect { x: 0, y: 30, w: 1100, h: 800 };
        let mut c = Canvas::default();
        assert_eq!(canvas_init(&mut c, 1200, 900), 0);
        // (a text drawn from the canvas's left edge on a baseline near its foot: its ink, and its width)
        let mut drawn = |f: FontFace, px: f32, text: &str| -> (usize, i32) {
            c.px.fill(255);
            canvas_set_clip(&mut c, clip);
            let before = asked();
            let width = font_draw(&mut c, f, px, 0, 780, text, 0);
            // (stb is asked for each glyph that is made, and for a pixel face's design pixels: never for more
            // than the text has letters)
            assert!(asked() - before <= text.len() as u32, "{f:?} at {px} px");
            let (all, outside) = ink(&c, clip);
            assert_eq!(outside, 0, "{f:?} at {px} px: ink outside the clip");
            assert_eq!(width, font_measure(f, px, text), "{f:?} at {px} px: {text:?} is as wide drawn as measured");
            the_cache_is_within_its_sizes(&format!("{f:?} at {px} px"));
            (all, width)
        };
        let far = [8192.0, 150_000.0, 1_000_000.0, 2_147_483.0];

        // An outline face. (Terminess, at its own scale, draws a W about half as wide as the text is high.)
        font_set_family(0);
        for f in [FontFace::Regular, FontFace::Bold, FontFace::Fixed, FontFace::FixedBold] {
            assert_eq!(with(|st| (k_unit(face_of(st, f).ttf), face_of(st, f).scale)), (0, 1.0), "{f:?} is an outline face");
            for px in [12.0, 44.0, 512.0, 768.0, 2000.0] {
                assert_ne!(drawn(f, px, "W"), (0, 0), "{f:?} at {px} px");
                assert!(drawn(f, px, "Wide").0 > 0, "{f:?} at {px} px");
            }
            // (quarter pixels: the W is made at 2,000 px and not at 8,000 px)
            let (mut lo, mut hi) = (4 * 2000, 4 * 8000);
            let made = |px4: i32| glyph_of('W' as u32, px4).is_some_and(|g| g.2 != 0);
            let before = asked();
            assert_eq!((drawn(f, 8000.0, "W").0, asked() - before), (0, 0), "{f:?}");
            assert!(made(lo) && !made(hi), "{f:?}");
            while hi - lo > 1 {
                let mid = (lo + hi) / 2;
                with(cache_flush);
                drawn(f, mid as f32 / 4.0, "W");
                if made(mid) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            with(cache_flush);
            let (_, width) = drawn(f, lo as f32 / 4.0, "W");
            let (w, h, len) = glyph_of('W' as u32, lo).unwrap();
            assert!(len != 0 && (w.max(h) >= 4090 || w * h > 4_150_000), "{f:?}: the tallest W made, at {} px, is {w} by {h}: the greatest glyph", lo as f32 / 4.0);
            // A quarter pixel taller the W is not made: nothing of it is drawn, and it is as wide as it was to a pixel.
            with(cache_flush);
            let before = asked();
            let (ink_past, width_past) = drawn(f, hi as f32 / 4.0, "W");
            assert_eq!((glyph_of('W' as u32, hi), asked() - before), (Some((0, 0, 0)), 0), "{f:?}");
            assert_eq!(ink_past, 0, "{f:?} at {} px", hi as f32 / 4.0);
            assert!((width_past - width).abs() <= 1 && width > 1000, "{f:?}: {width} px wide, then {width_past}");
            for px in far {
                let before = asked();
                assert_eq!(drawn(f, px, "W").0, 0, "{f:?} at {px} px");
                let (ink_far, width_far) = drawn(f, px, "Tenletters");
                assert_eq!((ink_far, asked() - before), (0, 0), "{f:?} at {px} px");
                assert!(width_far as f32 > px, "{f:?} at {px} px: ten letters are {width_far} px wide");
            }
        }

        // A pixel face: a glyph is its design pixels as blocks of k screen pixels, so its sides are whole
        // multiples of k. The largest k at which a letter is made keeps both halves of the rule, and one more
        // breaks one.
        ui_set_theme(UiTheme::Gothic);
        for (f, letter) in [(FontFace::Regular, 'W'), (FontFace::Bold, 'W'), (FontFace::Caps, 'l'), (FontFace::Title, 'G')] {
            let ttf = with(|st| face_of(st, f).ttf);
            assert_ne!(k_unit(ttf), 0, "{f:?} is a pixel face in Gothic");
            let text = letter.to_string();
            let cp = letter as u32;
            // (the letter's design pixels: its glyph at one screen pixel each)
            with(cache_flush);
            assert!(drawn(f, px_of(f, 1), &text).1 > 0);
            let (dw, dh, _) = glyph_of(cp, -1).unwrap();
            assert!(dw >= 1 && dh >= 1);
            let fits = |k: i64| dw as i64 * k <= 4096 && dh as i64 * k <= 4096 && dw as i64 * k * dh as i64 * k <= 4_194_304;
            let most = (1..5000).take_while(|k| fits(*k)).last().unwrap() as i32;
            for k in [1, 2, 7, most - 1, most] {
                with(cache_flush);
                let (ink_at, width) = drawn(f, px_of(f, k), &text);
                assert_eq!(glyph_of(cp, -k), Some((dw * k, dh * k, (dw * k * dh * k) as usize)), "{f:?} at k {k}");
                assert!(ink_at > 0 && width > 0, "{f:?} at k {k}");
            }
            for k in [most + 1, most + 2, 2 * most, 4097, 65_536, 100_000] {
                with(cache_flush);
                let (ink_at, width) = drawn(f, px_of(f, k), &text);
                assert_eq!(glyph_of(cp, -k), Some((0, 0, 0)), "{f:?} at k {k}");
                assert_eq!(ink_at, 0, "{f:?} at k {k}");
                assert!(width >= k, "{f:?} at k {k}: the letter keeps its advance, {width} px");
            }
            for px in far {
                assert_eq!(drawn(f, px, &text).0, 0, "{f:?} at {px} px");
                let (ink_far, width_far) = drawn(f, px, "Tenletters");
                assert_eq!(ink_far, 0, "{f:?} at {px} px");
                assert!(width_far as f32 > px, "{f:?} at {px} px: ten letters are {width_far} px wide");
            }
        }
        // A text that is fitted to a rectangle at such a height draws nothing, in either theme.
        for theme in [UiTheme::Gothic, UiTheme::Classic] {
            ui_set_theme(theme);
            for px in far {
                c.px.fill(255);
                canvas_set_clip(&mut c, clip);
                let r = Rect { x: 100, y: 100, w: 900, h: 600 };
                font_draw_centered(&mut c, FontFace::Bold, px, r, "Wide words", 0);
                font_draw_left(&mut c, FontFace::Regular, px, r, "Wide words", 0);
                font_draw_wrapped(&mut c, FontFace::Title, px, r, "Wide words in two lines", 3, 0, true, 0);
                assert_eq!(ink(&c, clip), (0, 0), "{theme:?} at {px} px");
                the_cache_is_within_its_sizes(&format!("{theme:?} at {px} px, fitted"));
            }
        }
        ui_set_theme(UiTheme::Classic);
    }

    #[test]
    fn broken_utf8_decodes_to_the_replacement_character() {
        let mut i = 0;
        assert_eq!(utf8_next(b"A", &mut i), 0x41);
        assert_eq!(i, 1);
        let mut i = 0;
        assert_eq!(utf8_next(b"\xc3\xb1", &mut i), 0xF1);
        assert_eq!(i, 2);
        // a lead byte with nothing after it, a stray continuation byte, a cut sequence
        let mut i = 0;
        assert_eq!(utf8_next(b"\xc3", &mut i), 0xFFFD);
        assert_eq!(i, 1);
        let mut i = 0;
        assert_eq!(utf8_next(b"\x80x", &mut i), 0xFFFD);
        assert_eq!(i, 1);
        let mut i = 0;
        assert_eq!(utf8_next(b"\xe2\x82", &mut i), 0xFFFD);
        assert_eq!(i, 2);
        let mut i = 0;
        assert_eq!(utf8_next(b"\xf0\x9f\x98\x80", &mut i), 0x1F600);
        assert_eq!(i, 4);
    }

    #[test]
    fn nothing_is_measured_before_font_init() {
        font_shutdown();
        assert_eq!(font_measure(FontFace::Regular, 44.0, "abc"), 0);
        assert_eq!(font_ascent(FontFace::Regular, 44.4), 44);
        assert_eq!(font_descent(FontFace::Regular, 44.0), 0);
        assert!(!font_has_glyph(FontFace::Regular, 0x41));
        assert_eq!(font_wrap_lines(FontFace::Regular, 44.0, "abc", 1, None), 0);
        let mut out = String::new();
        assert_eq!(font_drawable_copy("a\u{65e5}b", &mut out, 20), 0);
        assert_eq!(out, "a\u{65e5}b");
    }

    #[test]
    fn drawable_copy_leaves_out_what_shows_nothing() {
        up();
        let mut out = String::new();
        assert_eq!(font_drawable_copy("Hello", &mut out, 20), 0);
        assert_eq!(out, "Hello");
        // Japanese comes from DotGothic16; an emoji neither it nor Terminess has stays (it is a box,
        // 2.1.0-beta.2), and so do whitespace and a zero-width space Terminess has
        assert_eq!(font_drawable_copy("\u{65e5}\u{672c} a \u{8a9e} b \u{3042}\u{1f600}", &mut out, 40), 0);
        assert_eq!(out, "\u{65e5}\u{672c} a \u{8a9e} b \u{3042}\u{1f600}");
        assert_eq!(font_drawable_copy("a\tb\u{200b}c", &mut out, 40), 0);
        // what shows nothing and no face has goes (a control, a format character, a combining mark, a
        // variation selector), and the spaces that leaves doubled, first or last go with it
        assert_eq!(font_drawable_copy("\u{feff}\u{7} a \u{2063} b\u{323}\u{2764}\u{fe0f} \u{e0067}", &mut out, 40), 6);
        assert_eq!(out, "a b\u{2764}");
        // cut between two characters, never inside one
        assert_eq!(font_drawable_copy("\u{f1}\u{f1}\u{f1}", &mut out, 5), 0);
        assert_eq!(out, "\u{f1}\u{f1}");
        assert_eq!(font_drawable_copy("abc", &mut out, 1), 0);
        assert_eq!(out, "");
        // (cap 0 touches nothing)
        out = "kept".to_string();
        assert_eq!(font_drawable_copy("abc", &mut out, 0), 0);
        assert_eq!(out, "kept");
    }

    #[test]
    fn text_cut_short_ends_in_dots_and_the_hook_hears_of_it() {
        up();
        let mut c = Canvas::default();
        canvas_init(&mut c, 400, 100);
        let text = "A notebook with a name too long for the box";
        let need = font_measure(FontFace::Regular, 34.0, text);
        SEEN.with(|s| s.borrow_mut().clear());
        font_set_overflow_hook(Some(hear));
        font_draw_centered(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 0, w: 200, h: 50 }, text, 0);
        font_draw_left(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 50, w: 200, h: 50 }, text, 0);
        // a name someone gave is shortened by design: the hook does not hear of it
        font_draw_left_name(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 50, w: 200, h: 50 }, text, 0);
        font_set_overflow_hook(None);
        font_draw_centered(&mut c, FontFace::Regular, 34.0, Rect { x: 0, y: 0, w: 200, h: 50 }, text, 0);
        SEEN.with(|s| {
            let s = s.borrow();
            assert_eq!(*s, vec![(text.to_string(), need, 200), (text.to_string(), need, 200)]);
        });
        let cut = fit(FontFace::Regular, 34.0, text.as_bytes(), 200, None).unwrap();
        assert!(cut.ends_with(b"..."));
        assert!(measure(FontFace::Regular, 34.0, &cut) <= 200);
        assert!(fit(FontFace::Regular, 34.0, b"short", 200, None).is_none());
        assert_eq!(fit(FontFace::Regular, 34.0, b"abc", 1, None).unwrap(), b"");
    }

    #[test]
    fn two_lines_break_after_the_comma() {
        up();
        // Terminess is monospaced: "Wide, no" is 8 cells, "no margin" 9
        let text = "Wide, no margin";
        let w = font_measure(FontFace::Regular, 34.0, "no margin");
        let mut widest = 0;
        assert_eq!(font_wrap_lines(FontFace::Regular, 34.0, text, w, Some(&mut widest)), 2);
        assert_eq!(widest, w);
        let mut ln = [Span::default(); WRAP_MAX];
        let mut broken = 0;
        assert_eq!(wrap(FontFace::Regular, 34.0, text.as_bytes(), w, &mut ln, WRAP_MAX, &mut broken), 2);
        assert_eq!((ln[0].s, ln[0].e, ln[1].s, ln[1].e), (0, 5, 6, 15));
        assert_eq!(broken, 0);
        // a word wider than the line is broken between its letters
        let n = wrap(FontFace::Regular, 34.0, b"abcdefghij k", font_measure(FontFace::Regular, 34.0, "abcd"), &mut ln, WRAP_MAX, &mut broken);
        assert!(n >= 3);
        assert_eq!(broken, font_measure(FontFace::Regular, 34.0, "abcdefghij"));
        assert_eq!((ln[0].s, ln[0].e), (0, 4));
        // text that fits is one line; nothing but spaces is none
        assert_eq!(font_wrap_lines(FontFace::Regular, 34.0, text, 10000, Some(&mut widest)), 1);
        assert_eq!(font_wrap_lines(FontFace::Regular, 34.0, "   ", 10000, None), 0);
    }

    fn next_char(t: &[u8], mut s: usize) -> usize {
        if at(t, s) != 0 {
            s += 1;
        }
        while (at(t, s) & 0xC0) == 0x80 {
            s += 1;
        }
        s
    }

    /// The lines of wrap_greedy as measuring each line from its start gives them: a word wider than the line
    /// measured again from each of its lines, and the characters of each line measured one more at a time.
    fn wrap_greedy_measuring_each_line(f: FontFace, px: f32, t: &[u8], w: i32, out: &mut [Span], max: usize, broken: &mut i32) -> usize {
        let has_cjk = |mut s: usize, e: usize| {
            while s < e {
                if cjk(utf8_next(t, &mut s)) {
                    return true;
                }
            }
            false
        };
        let mut n = 0;
        *broken = 0;
        let mut s = skip_spaces(t, 0);
        while at(t, s) != 0 {
            let mut fits: Option<usize> = None;
            let mut e = s;
            while at(t, skip_spaces(t, e)) != 0 {
                let mut we = skip_spaces(t, e);
                while at(t, we) != 0 && at(t, we) != b' ' {
                    we += 1;
                }
                if measure_n(f, px, t, s, Some(we)) > w {
                    break;
                }
                fits = Some(we);
                e = we;
            }
            let fits = match fits {
                Some(x) => x,
                None => {
                    let mut we = s;
                    while at(t, we) != 0 && at(t, we) != b' ' {
                        we += 1;
                    }
                    let ww = measure_n(f, px, t, s, Some(we));
                    if ww > *broken && !has_cjk(s, we) {
                        *broken = ww;
                    }
                    let mut fits = next_char(t, s);
                    while at(t, fits) != 0 && at(t, fits) != b' ' && measure_n(f, px, t, s, Some(next_char(t, fits))) <= w {
                        fits = next_char(t, fits);
                    }
                    fits
                }
            };
            if n < max {
                out[n] = Span { s, e: fits };
            }
            n += 1;
            s = skip_spaces(t, fits);
        }
        n
    }

    // A word wider than its line is laid out once and its lines summed from that: the lines, their count and the
    // width the overflow report hears are the ones measuring each line again gives, in every face and size, both
    // themes and each family, for words of kerned pairs, CJK, marks, controls, boxes and spaces of every kind.
    #[test]
    fn a_long_word_is_broken_where_measuring_each_line_again_breaks_it() {
        up();
        let words = [
            "i".repeat(300),
            "W".repeat(120),
            "AVAVAWAYATAVTo".repeat(12),
            format!("\u{6f22}\u{5b57}{}", "abcdefghij".repeat(12)),
            format!("{}\u{3072}\u{3089}", "abcdefghij".repeat(12)),
            "e\u{301}\u{200b}\u{200d}x\u{202e}".repeat(30),
            // (two private-use characters, at the areas' bounds, and a symbol no face has)
            format!("a{}b{}\t\u{3000}\u{a0}c\u{1d11e}", char::from_u32(0x10FFFD).unwrap(), char::from_u32(0xE000).unwrap()).repeat(20),
            "\u{1f600}\u{fffd}\u{0}z".repeat(25),
            "\u{3072}\u{3089}\u{304c}\u{306a}\u{30ab}\u{30bf}".repeat(20),
        ];
        let texts: Vec<String> = words
            .iter()
            .flat_map(|wd| [wd.clone(), format!("a {wd} b c"), format!("one two {wd}  {wd} three"), format!(" {wd} ")])
            .chain(["Wide, no margin".to_string(), "x".to_string(), String::new()])
            .collect();
        let looks = [(UiTheme::Gothic, 0), (UiTheme::Classic, 0), (UiTheme::Classic, 1), (UiTheme::Classic, 2), (UiTheme::Classic, 3)];
        let faces = [(FontFace::Regular, 34.0), (FontFace::Bold, 44.0), (FontFace::Title, 70.0), (FontFace::Fixed, 34.0), (FontFace::Caps, 30.0)];
        let mut cases = 0;
        for (theme, family) in looks {
            ui_set_theme(theme);
            font_set_family(family);
            for (f, px) in faces {
                for text in &texts {
                    let t = cstr(text).as_bytes();
                    for w in [-3, 0, 1, 5, 17, 40, 77, 150, 301, 1000] {
                        let (mut a, mut b) = ([Span::default(); WRAP_MAX], [Span::default(); WRAP_MAX]);
                        let (mut ba, mut bb) = (-1, -1);
                        let na = wrap_greedy(f, px, t, w, &mut a, WRAP_MAX, &mut ba);
                        let nb = wrap_greedy_measuring_each_line(f, px, t, w, &mut b, WRAP_MAX, &mut bb);
                        assert_eq!((na, ba), (nb, bb), "{theme:?} {family} {f:?} {px} w {w}: {text:?}");
                        let kept = na.min(WRAP_MAX);
                        assert!(a[..kept].iter().zip(&b[..kept]).all(|(x, y)| (x.s, x.e) == (y.s, y.e)), "{theme:?} {family} {f:?} w {w}: {text:?}");
                        cases += 1;
                    }
                }
            }
        }
        assert_eq!(cases, 5 * 5 * 39 * 10);
        font_set_family(0);
        ui_set_theme(UiTheme::Classic);
    }

    #[test]
    fn wrapped_text_draws_the_lines_it_returns() {
        up();
        let mut c = Canvas::default();
        canvas_init(&mut c, 300, 300);
        let text = "one two three four five six seven eight nine ten";
        let r = Rect { x: 0, y: 0, w: 200, h: 200 };
        let one = font_draw_wrapped(&mut c, FontFace::Regular, 34.0, r, text, 1, 0, false, 0);
        assert_eq!(one, 1);
        let shown = font_draw_wrapped(&mut c, FontFace::Regular, 34.0, r, text, 3, 0, true, 0);
        assert_eq!(shown, 3);
        let needed = font_wrap_lines(FontFace::Regular, 34.0, text, 200, None);
        assert!(needed > 3);
        let all = font_draw_wrapped(&mut c, FontFace::Regular, 34.0, r, text, 99, 40, false, 0);
        assert_eq!(all, needed.min(WRAP_MAX as i32));
    }

    #[test]
    fn fit_px_finds_the_largest_size_that_fits() {
        up();
        let text = "Rename the notebook";
        let full = font_measure(FontFace::Regular, 44.0, text);
        assert_eq!(font_fit_px(FontFace::Regular, 44.0, 20.0, text, full), 44.0);
        let px = font_fit_px(FontFace::Regular, 44.0, 20.0, text, full - 40);
        assert!((20.0..44.0).contains(&px));
        assert!(font_measure(FontFace::Regular, px, text) <= full - 40);
        // quarter pixels in an outline face
        assert_eq!(px * 4.0, (px * 4.0).floor());
        assert_eq!(font_fit_px(FontFace::Regular, 44.0, 20.0, text, 10), 0.0);
        // a pixel face changes only at whole design pixels
        ui_set_theme(UiTheme::Gothic);
        let g_full = font_measure(FontFace::Regular, 44.0, text);
        let g = font_fit_px(FontFace::Regular, 44.0, 20.0, text, g_full - 30);
        assert!(g > 0.0 && g < 44.0);
        assert!(font_measure(FontFace::Regular, g, text) <= g_full - 30);
    }

    #[test]
    fn glyph_cache_keeps_working_past_its_size() {
        up();
        // every code point of a block at several sizes: more than the cache holds, so it flushes
        let mut c = Canvas::default();
        canvas_init(&mut c, 120, 120);
        for size in [20.0f32, 21.0, 22.0, 23.0] {
            let mut cp = 0x20u32;
            while cp < 0x2200 {
                if let Some(ch) = char::from_u32(cp) {
                    font_draw(&mut c, FontFace::Regular, size, 10, 60, &ch.to_string(), 0);
                }
                cp += 1;
            }
        }
        assert!(font_measure(FontFace::Regular, 20.0, "Hello") > 0);
        font_shutdown();
        assert_eq!(font_measure(FontFace::Regular, 20.0, "Hello"), 0);
        assert_eq!(font_init(), 0);
        assert!(font_measure(FontFace::Regular, 20.0, "Hello") > 0);
    }

    #[test]
    fn the_face_table() {
        up();
        // Terminess first, the default, at the sizes every layout was made for
        let t = &FONT_FAMILIES[0];
        assert_eq!((t.id, t.regular, t.bold, t.scale), ("terminess", TTF_TERM, TTF_TERM_BOLD, 1.0));
        let mut files = vec![TTF_TERM, TTF_TERM_BOLD];
        for (i, f) in FONT_FAMILIES.iter().enumerate() {
            assert_eq!(font_family_find(f.id.as_bytes()), Some(i));
            assert!(FONT_FAMILIES[..i].iter().all(|g| g.id != f.id && g.web != f.web));
            assert!(f.short.len() <= f.name.len() && f.name.starts_with(f.short.split(' ').next().unwrap()));
            assert!((1.0..=1.25).contains(&f.scale), "{}", f.id);
            assert_eq!(font_family_file(i, false), ttf_data(f.regular));
            assert_eq!(font_family_file(i, true), ttf_data(f.bold));
            if i > 0 {
                files.extend([f.regular, f.bold]);
            }
        }
        // every embedded file is a family's, Gothic's or the Japanese one, once
        files.extend([TTF_JACQUARD, TTF_JERSEY, TTF_JP]);
        files.sort();
        assert_eq!(files, (0..TTF_COUNT).collect::<Vec<_>>());
        assert_eq!(font_family_find(b"comic-sans"), None);
        assert_eq!(font_family_find(b""), None);
        // a family past the list is Terminess; with_family puts the one in use back
        font_set_family(99);
        assert_eq!(font_family(), 0);
        let lit = font_family_find(b"literata").unwrap();
        let w = font_with_family(lit, || {
            assert_eq!(font_family(), lit);
            font_measure(FontFace::Regular, 44.0, "Literata")
        });
        assert_eq!(font_family(), 0);
        assert_ne!(w, font_measure(FontFace::Regular, 44.0, "Literata"));
        // its scale: the line is that much taller than asked
        let asc = font_with_family(lit, || font_ascent(FontFace::Regular, 100.0) + font_descent(FontFace::Regular, 100.0));
        assert!((asc - (100.0 * FONT_FAMILIES[lit].scale).round() as i32).abs() <= 1, "{asc}");
        // kerning inside a family (its GPOS pairs, read by stb)
        let atk = font_family_find(b"atkinson").unwrap();
        font_with_family(atk, || {
            let pair = font_measure(FontFace::Regular, 100.0, "AV");
            let apart = font_measure(FontFace::Regular, 100.0, "A") + font_measure(FontFace::Regular, 100.0, "V");
            assert!(pair < apart, "{pair} {apart}");
        });
    }

    // A font of the owner's own is read by skrifa and drawn by stb's rasterizer (fontcheck.rs), never opened by
    // stb. For a TrueType font the two roads meet: the outline skrifa hands over is the very vertex list stb's
    // own parser makes, so the bitmap is stb's own, byte for byte, with its size and its offsets. Checked here
    // on every glyph every embedded text file maps, at five sizes: the pixel faces, DotGothic16's 3,340 glyphs,
    // the three families' Regular and Bold, and Terminess's text. Terminess's icons (the private use areas) are
    // left out: some are built of scaled components, where stb's parser cuts a coordinate short and skrifa
    // rounds it, a unit apart; they are never drawn through this road (icons are stb's, always).
    #[test]
    fn skrifa_outlines_drawn_by_stb_are_stbs_own_bitmaps() {
        use crate::fontcheck::*;
        use skrifa::MetadataProvider;
        up();
        let mut out = Outliner::new().unwrap();
        let mut compared = 0;
        for ttf in 0..TTF_COUNT {
            let font = skrifa::raw::FontRef::from_index(ttf_data(ttf), 0).unwrap();
            let glyphs = font.outline_glyphs();
            let (a, d, _) = with(|st| vmetrics(&st.font[ttf]));
            let line = a - d;
            for (cp, gid) in font.charmap().mappings() {
                if (ttf == TTF_TERM || ttf == TTF_TERM_BOLD) && ((0xE000..=0xF8FF).contains(&cp) || cp >= 0xF0000) {
                    continue;
                }
                for px in [12.0f32, 34.0, 44.0, 64.0, 100.0] {
                    let scale = with(|st| scale_for_pixel_height(&st.font[ttf], px));
                    let (bmp, w, h, xo, yo) = with(|st| codepoint_bitmap(&st.font[ttf], scale, scale, cp));
                    let ours = match out.outline(&glyphs, gid.to_u32(), &[], line) {
                        Ok(Some(b)) => rasterize(&mut out.verts, b, scale).unwrap(),
                        Ok(None) => Bitmap::default(),
                        Err(e) => panic!("face {ttf} U+{cp:04X}: {e:?}"),
                    };
                    if bmp.is_empty() {
                        // (stb gives no bitmap for a glyph with no ink)
                        assert!(ours.px.is_empty(), "face {ttf} U+{cp:04X} at {px}");
                        continue;
                    }
                    assert_eq!((ours.w, ours.h, ours.xoff, ours.yoff), (w, h, xo, yo), "face {ttf} U+{cp:04X} at {px}");
                    assert!(ours.px == bmp, "face {ttf} U+{cp:04X} at {px}: the bitmaps differ");
                    compared += 1;
                }
            }
        }
        assert!(compared > 30_000, "{compared}");
    }

    // The same road end to end: Noto Sans as a font of the owner's own (the file's bytes, read by the check and
    // drawn as a custom font) draws each character as the bundled Noto Sans does, at the same scale: the same
    // pixels, the same advance, regular and bold. (Whole strings differ by design: a custom font has no kerning.)
    #[test]
    fn a_custom_font_draws_as_the_same_file_embedded() {
        use crate::fontcheck::*;
        use std::rc::Rc;
        up();
        let noto = font_family_find(b"noto-sans").unwrap();
        let own = |bold: bool| {
            let data = Rc::new(font_family_file(noto, bold).to_vec());
            let mut facts = facts_in_process(&data).unwrap();
            // (the bundled family's scale, and stb's line for the file)
            facts.scale_pm = (FONT_FAMILIES[noto].scale * 1000.0).round() as i32;
            let (a, d, _) = with(|st| vmetrics(&st.font[if bold { TTF_NOTO_BOLD } else { TTF_NOTO }]));
            assert_eq!((facts.ascent, facts.descent), (a, d));
            SafeFace::new(data, Rc::new(facts), false).unwrap()
        };
        font_set_own(Some(OwnFont { regular: own(false), bold: Some(own(true)) }));
        assert!(font_own());
        let shot = |f: FontFace, px: f32, text: &str| {
            let mut c = Canvas::default();
            canvas_init(&mut c, 260, 220);
            let w = font_draw(&mut c, f, px, 40, 150, text, 0);
            (c.px, w, font_ascent(f, px), font_descent(f, px))
        };
        let mut seen = 0;
        for f in [FontFace::Regular, FontFace::Bold, FontFace::Title, FontFace::Caps] {
            for px in [20.0f32, 34.0, 44.0, 64.0] {
                for cp in (0x21u32..0x7F).chain(0xA1..0x17F).chain([0x2026, 0x20AC]) {
                    let text = char::from_u32(cp).unwrap().to_string();
                    let mine = shot(f, px, &text);
                    let theirs = font_with_family(noto, || shot(f, px, &text));
                    assert!(mine == theirs, "{f:?} at {px}: U+{cp:04X}");
                    seen += 1;
                }
            }
        }
        assert!(seen > 4000);
        // what the font lacks comes from Terminess, then DotGothic16, as for a family
        for text in ["\u{2192}", "\u{65e5}\u{672c}"] {
            assert!(shot(FontFace::Regular, 44.0, text) == font_with_family(noto, || shot(FontFace::Regular, 44.0, text)), "{text}");
        }
        // a size the check never drew: Terminess's glyph, not the font's (the line stays the font's)
        let big = shot(FontFace::Regular, 400.0, "A");
        let term = font_with_family(0, || shot(FontFace::Regular, 400.0, "A"));
        assert!(big.0 == term.0 && big.1 == term.1);
        assert!(font_own_drawn() > 0 && font_own_bytes() > 50_000);
        // Gothic keeps its faces; FIXED is Terminess whatever the font
        let fixed = shot(FontFace::Fixed, 44.0, "Ag");
        assert!(fixed == font_with_family(0, || shot(FontFace::Fixed, 44.0, "Ag")));
        ui_set_theme(UiTheme::Gothic);
        let gothic = shot(FontFace::Regular, 44.0, "Ag");
        font_set_own(None);
        assert!(!font_own() && font_own_bytes() == 0);
        assert!(gothic == shot(FontFace::Regular, 44.0, "Ag"));
        ui_set_theme(UiTheme::Classic);
    }

    // A custom font whose bold is its regular overprinted (no weight axis, no Bold file): bold text is wider by
    // the overprint, as an outline face's always was; and the sample slot draws beside the font in use.
    #[test]
    fn a_custom_font_without_a_bold_is_overprinted_and_the_sample_stands_beside_it() {
        use crate::fontcheck::*;
        use std::rc::Rc;
        up();
        let face = |family: &[u8]| {
            let data = Rc::new(font_family_file(font_family_find(family).unwrap(), false).to_vec());
            let facts = facts_in_process(&data).unwrap();
            SafeFace::new(data, Rc::new(facts), false).unwrap()
        };
        font_set_own(Some(OwnFont { regular: face(b"literata"), bold: None }));
        let reg = font_measure(FontFace::Regular, 44.0, "Hamburg");
        let bold = font_measure(FontFace::Bold, 44.0, "Hamburg");
        assert!(bold > reg, "{bold} {reg}");
        // the sample: another font, only inside font_with_sample
        font_set_sample(Some(OwnFont { regular: face(b"atkinson"), bold: None }));
        assert_eq!(font_measure(FontFace::Regular, 44.0, "Hamburg"), reg);
        let sample = font_with_own(true, || font_measure(FontFace::Regular, 44.0, "Hamburg"));
        assert_ne!(sample, reg);
        assert_eq!(font_measure(FontFace::Regular, 44.0, "Hamburg"), reg);
        // the sheet's own words: the family, whatever font is in use; the font in use again inside it
        let term = font_with_family(0, || font_measure(FontFace::Regular, 44.0, "Hamburg"));
        assert_ne!(term, reg);
        assert_eq!(font_with_family(0, || font_with_own(false, || font_measure(FontFace::Regular, 44.0, "Hamburg"))), reg);
        assert_eq!(font_with_family(0, || font_with_own(true, || font_measure(FontFace::Regular, 44.0, "Hamburg"))), sample);
        font_set_sample(None);
        font_set_own(None);
        assert_eq!(font_measure(FontFace::Regular, 44.0, "Hamburg"), term);
    }

    // The glyph cache has a byte cap beside its slots: big glyphs flush it before it grows past it, and text
    // draws the same after a flush.
    #[test]
    fn the_glyph_cache_is_capped_in_bytes() {
        up();
        let mut c = Canvas::default();
        canvas_init(&mut c, 900, 900);
        for i in 0..40 {
            // an icon 700 px high is about half a megabyte of coverage
            font_draw_icon(&mut c, FontFace::Regular, 0xF064F, Rect { x: 0, y: 0, w: 700 - i, h: 700 - i }, 0);
            assert!(with(|st| st.cache_bytes) <= CACHE_BYTES_MAX + (1 << 20));
        }
        assert!(with(|st| st.cache_used) < 20);
        assert!(font_measure(FontFace::Regular, 20.0, "Hello") > 0);
    }

    /// The boxes drawn in c at px on the baseline: the left x of each one's cell, found by its exact pixels
    /// (ink on its sides, paper inside, paper beside it).
    fn boxes_at(c: &Canvas, px: f32, baseline: i32) -> Vec<i32> {
        let (_, r, t) = with(|st| missing_box(st, px));
        let at = |x: i32, y: i32| c.px[(y * c.stride + x) as usize];
        let mut found = Vec::new();
        for x in 1..c.w - r.w - 1 {
            let ok = (0..r.h).all(|dy| {
                let y = baseline + r.y + dy;
                at(x - 1, y) == 255
                    && at(x + r.w, y) == 255
                    && (0..r.w).all(|dx| {
                        let side = dx < t || dx >= r.w - t || dy < t || dy >= r.h - t;
                        at(x + dx, y) == if side { 0 } else { 255 }
                    })
            });
            if ok {
                found.push(x - r.x);
            }
        }
        found
    }

    // A visible character no face has is the box (2.1.0-beta.2), measured as it is drawn: the owner's
    // notebook 会议记录 (imported from 会议记录.pdf) showed 会 alone in the Library, as 议, 记 and 录 are
    // simplified Chinese no face has (DotGothic16 has 会). Now it is 会 and three boxes, wider than 会 alone.
    #[test]
    fn a_character_no_face_has_is_a_box_measured_as_drawn() {
        up();
        let name = "\u{4f1a}\u{8bae}\u{8bb0}\u{5f55}";
        assert!(font_has_glyph(FontFace::Regular, 0x4f1a));
        assert!(name.chars().skip(1).all(|ch| !font_has_glyph(FontFace::Regular, ch as u32)));
        for px in [26.0f32, 34.0, 44.0, 61.0] {
            let (adv, _, _) = with(|st| missing_box(st, px));
            let hui = with(|st| {
                let a = face_advance(st, TTF_JP, 0x4f1a);
                units_to_px(st, TTF_JP, px, a)
            });
            let w = font_measure(FontFace::Regular, px, name);
            assert!(w > font_measure(FontFace::Regular, px, "\u{4f1a}"), "{px}");
            assert_eq!(w, (hui + 3.0 * adv).round_ties_even() as i32, "{px}");
            let mut c = Canvas::default();
            canvas_init(&mut c, 400, 120);
            assert_eq!(font_draw(&mut c, FontFace::Regular, px, 10, 90, name, 0), w);
            let want: Vec<i32> = (0..3).map(|k| (10.0 + hui + k as f32 * adv).round_ties_even() as i32).collect();
            assert_eq!(boxes_at(&c, px, 90), want, "{px}");
            // 会 as it is drawn alone, nothing past the last box
            let mut one = Canvas::default();
            canvas_init(&mut one, 400, 120);
            font_draw(&mut one, FontFace::Regular, px, 10, 90, "\u{4f1a}", 0);
            for y in 0..120 {
                for x in 0..400 {
                    let i = (y * 400 + x) as usize;
                    if x < want[0] {
                        assert_eq!(c.px[i], one.px[i], "{px}: {x},{y}");
                    } else if x >= 10 + w {
                        assert_eq!(c.px[i], 255, "{px}: {x},{y}");
                    }
                }
            }
            // cut short to fit, the box is measured as drawn: the dots come after whole boxes
            let long = format!("{name}\u{8bb0}\u{5f55}\u{8bb0}\u{5f55}\u{8bb0}\u{5f55}");
            let fits = font_measure(FontFace::Regular, px, &format!("{name}..."));
            assert!(fits < font_measure(FontFace::Regular, px, &long));
            let mut cut = Canvas::default();
            canvas_init(&mut cut, 400, 120);
            font_draw_left(&mut cut, FontFace::Regular, px, Rect { x: 10, y: 0, w: fits, h: 120 }, &long, 0);
            let (asc, desc) = (font_ascent(FontFace::Regular, px), font_descent(FontFace::Regular, px));
            assert_eq!(boxes_at(&cut, px, (120 - (asc + desc)) / 2 + asc), want, "{px}");
        }
    }

    // The box is drawn the same in every face, weight and theme, a custom font's too (never overprinted),
    // and measured as drawn: a hollow rectangle in a cell of Terminess at the size asked.
    #[test]
    fn the_box_is_the_same_in_every_face_and_theme() {
        use crate::fontcheck::*;
        use std::rc::Rc;
        up();
        let own = || {
            let data = Rc::new(font_family_file(font_family_find(b"literata").unwrap(), false).to_vec());
            let facts = facts_in_process(&data).unwrap();
            SafeFace::new(data, Rc::new(facts), false).unwrap()
        };
        let faces = [FontFace::Regular, FontFace::Bold, FontFace::Title, FontFace::Caps, FontFace::Fixed, FontFace::FixedBold];
        let check = |what: &str| {
            for f in faces {
                for px in [12.0f32, 20.0, 34.0, 44.0, 64.0, 100.0] {
                    let (adv, r, t) = with(|st| missing_box(st, px));
                    assert!(t >= 1 && 2 * t < r.w && 2 * t < r.h && r.x >= 1 && r.x + r.w < adv.round() as i32, "{px}");
                    let mut want = Canvas::default();
                    canvas_init(&mut want, 200, 200);
                    canvas_rect_outline(&mut want, Rect { x: 20 + r.x, y: 150 + r.y, ..r }, t, 0);
                    for text in ["\u{8bae}", "\u{1f600}"] {
                        let mut c = Canvas::default();
                        canvas_init(&mut c, 200, 200);
                        let w = font_draw(&mut c, f, px, 20, 150, text, 0);
                        assert_eq!(w, adv.round_ties_even() as i32, "{what} {f:?} {px}");
                        assert_eq!(font_measure(f, px, text), w, "{what} {f:?} {px}");
                        assert!(c.px == want.px, "{what} {f:?} {px} {text}");
                    }
                }
            }
        };
        for (i, fam) in FONT_FAMILIES.iter().enumerate() {
            font_set_family(i);
            check(fam.id);
        }
        font_set_family(0);
        font_set_own(Some(OwnFont { regular: own(), bold: None }));
        check("own font, overprinted bold");
        font_set_own(None);
        ui_set_theme(UiTheme::Gothic);
        check("gothic");
        ui_set_theme(UiTheme::Classic);
    }

    // What is meant to be unseen never shows as a box, even where no face has it: an emoji's ZWJ sequence
    // is a box for each pictograph (its joiners are Terminess's, blank as before), a variation selector and a
    // flag's tags are nothing, as are a combining mark no face has (on a letter that is drawn) and a
    // control; whitespace no face has is a space. Measured as drawn. (A theme's own face may have the mark,
    // U+0323: there it is drawn, as any letter the face has.)
    #[test]
    fn what_is_meant_to_be_unseen_shows_nothing() {
        up();
        let px = 44.0;
        let name = "e\u{323}\u{200b}\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} \u{1f3f3}\u{fe0f}\u{200d}\u{1f308} \
                    \u{1f3f4}\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}\u{7}";
        let seen = "e\u{200b}\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} \u{1f3f3}\u{200d}\u{1f308} \u{1f3f4}";
        for cp in [0x323, 0xfe0f, 0xe0067, 0x7, 0x1f468, 0x1f469, 0x1f467, 0x1f3f3, 0x1f308, 0x1f3f4] {
            assert!(!font_has_glyph(FontFace::Regular, cp), "U+{cp:04X}");
        }
        for theme in UiTheme::all() {
            ui_set_theme(theme);
            for f in [FontFace::Regular, FontFace::Bold, FontFace::Title] {
                let shot = |text: &str| {
                    let mut c = Canvas::default();
                    canvas_init(&mut c, 900, 120);
                    let w = font_draw(&mut c, f, px, 10, 90, text, 0);
                    assert_eq!(w, font_measure(f, px, text), "{theme:?} {f:?}");
                    (c, w)
                };
                let (a, wa) = shot(name);
                let mark = if font_has_glyph(f, 0x323) { seen.replacen('e', "e\u{323}", 1) } else { seen.to_string() };
                let (b, wb) = shot(&mark);
                assert!(wa == wb && a.px == b.px, "{theme:?} {f:?}: {wa} {wb}");
                assert_eq!(boxes_at(&a, px, 90).len(), 6, "{theme:?} {f:?}");
                // a control is nothing; whitespace no face has (a line separator) is a space
                assert_eq!(font_measure(f, px, "a\u{7}\u{1}b"), font_measure(f, px, "ab"), "{theme:?} {f:?}");
                assert_eq!(font_measure(f, px, "a\u{2028}b"), font_measure(f, px, "a b"), "{theme:?} {f:?}");
            }
        }
        ui_set_theme(UiTheme::Classic);
    }

    // The table of what is meant to be unseen: sorted, its ranges apart, and what it should and should not hold.
    #[test]
    fn the_unseen_table() {
        assert!(UNSEEN.iter().all(|&(a, b)| a <= b && b <= 0x10FFFF));
        assert!(UNSEEN.windows(2).all(|w| w[0].1 + 1 < w[1].0));
        for cp in [0x0, 0x7, 0x7F, 0x9F, 0xAD, 0x300, 0x323, 0x34F, 0x20DD, 0x200B, 0x200D, 0x2060, 0x2064, 0x3164, 0xFE00, 0xFE0F, 0xFEFF, 0xE0001, 0xE0020, 0xE007F, 0xE0100, 0xE01EF, 0xE0FFF] {
            assert_eq!(missing_as(cp), Missing::Nothing, "U+{cp:04X}");
        }
        for cp in [0x9, 0xA, 0x20, 0xA0, 0x1680, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000] {
            assert_eq!(missing_as(cp), Missing::Space, "U+{cp:04X}");
        }
        for cp in [0x41, 0xE9, 0x93E, 0x1EBF, 0x4F1A, 0x8BAE, 0xFFFD, 0x1F600, 0x1F468, 0x1F3F4, 0xD800, 0x110000] {
            assert_eq!(missing_as(cp), Missing::Box, "U+{cp:04X}");
        }
    }

    #[test]
    fn icons_fit_their_box() {
        up();
        let mut c = Canvas::default();
        canvas_init(&mut c, 100, 100);
        let r = Rect { x: 20, y: 20, w: 60, h: 60 };
        font_draw_icon(&mut c, FontFace::Regular, 0xF064F, r, 0);
        let mut inked = Rect::default();
        for y in 0..100 {
            for x in 0..100 {
                if c.px[(y * c.stride + x) as usize] < 255 {
                    inked = rect_union(inked, Rect { x, y, w: 1, h: 1 });
                }
            }
        }
        assert!(!rect_is_empty(inked));
        // (stb rounds the bitmap out to whole pixels: a pixel over at most)
        assert!(rect_contains_rect(rect_inflate(r, 1), inked));
        assert!(inked.w >= r.w - 2 && inked.h >= r.h - 2);
        // an empty box: nothing drawn
        let mut d = Canvas::default();
        canvas_init(&mut d, 100, 100);
        font_draw_icon(&mut d, FontFace::Regular, 0xF064F, Rect { x: 0, y: 0, w: 0, h: 10 }, 0);
        assert!(d.px.iter().all(|&p| p == 255));
    }
}
