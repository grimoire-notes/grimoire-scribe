//! Kindle Scribe notebooks: the `.notebooks/<id>/nbk` files imported as Grimoire notebooks.
//!
//! An `nbk` is a SQLite database wrapped in 1 KiB "fingerprint" records every 1 MiB (Amazon's
//! wrapping: SQLite sees a corrupt file until they are removed). Its tables hold Ion fragments:
//! `$ion_symbol_table` names the local symbols, and each other row is one annotated Ion value.
//! A notebook's pages are `$260` fragments (`nmdl.canvas_width/height`), their ink hangs off the
//! page's content (`$141`) and the stories it refers to (`$176`), and each stroke is a struct with
//! `nmdl.type = "nmdl.stroke"`: bounds, thickness, brush, and the point arrays in Amazon's
//! delta-of-delta encoding (docs/KINDLE.md, Kindle notebooks).
//!
//! This module is pure: bytes in, pages out. `bgwork_template.rs` writes the notebook in the
//! import child (the import-folder pipeline).

use crate::prelude::*;
use std::collections::HashMap;

/// the fingerprint record at offset 1024 and after every 1 MiB + 1 KiB of file
const FP_OFFSET: usize = 1024;
const FP_RECORD_LEN: usize = 1024;
const FP_DATA_LEN: usize = 1024 * 1024;
const FP_SIG: [u8; 4] = [0xFA, 0x50, 0x0A, 0x5F];

/// an `nbk` bigger than this is not read (the Scribe's notebooks are far smaller: 512 MiB is the
/// PDF import's own ceiling, and this is held in memory whole)
pub const NBK_MAX_BYTES: usize = 256 << 20;
/// a page keeps no more ink than this from an import (a page of the Scribe's own holds far fewer)
const NBK_MAX_STROKES: usize = 20_000;
const NBK_MAX_POINTS: usize = 400_000;
/// the recursion through a page's stories is stopped here
const NBK_MAX_DEPTH: usize = 12;

/// The Scribe's canvas: 15624 x 20832 at 2520 units per inch (a page of ours is that at the
/// device's dpi: 1860 x 2480 at 300).
const NBK_CANVAS_W: f64 = 15624.0;
const NBK_CANVAS_H: f64 = 20832.0;
const NBK_PPI: f64 = 2520.0;

#[derive(Clone, Debug, Default)]
pub struct NbkStroke {
    /// canvas units, absolute (bounds already added)
    pub points: Vec<(f64, f64)>,
    /// 0..1, one per point (empty when the notebook stores none)
    pub pressure: Vec<f32>,
    /// canvas units
    pub thickness: f64,
    /// Amazon's brush: 0 original pen, 1 highlighter, 5 pencil, 6 fountain pen, 7 pen/marker, 9 shader
    pub brush: i64,
}

#[derive(Clone, Debug, Default)]
pub struct NbkPage {
    pub canvas_w: f64,
    pub canvas_h: f64,
    pub strokes: Vec<NbkStroke>,
}

#[derive(Clone, Debug, Default)]
pub struct NbkNotebook {
    /// the notebook's own title, when its metadata holds one
    pub title: String,
    pub pages: Vec<NbkPage>,
    pub strokes: usize,
    pub points: usize,
}

// ---- the container ----

/// Strips Amazon's KDF fingerprint records. A file that is already a plain SQLite database (or is
/// too short to carry a fingerprint) is returned as it is.
pub fn unwrap_fingerprints(data: &[u8]) -> Vec<u8> {
    if data.len() < FP_OFFSET + FP_RECORD_LEN || data[FP_OFFSET..FP_OFFSET + 4] != FP_SIG {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..FP_OFFSET]);
    let mut off = FP_OFFSET;
    while data.len() >= off + FP_RECORD_LEN {
        if data[off..off + 4] != FP_SIG {
            // not the wrapping this build knows: leave the file alone rather than cut good bytes
            return data.to_vec();
        }
        let next = off + FP_DATA_LEN + FP_RECORD_LEN;
        let end = next.min(data.len());
        out.extend_from_slice(&data[off + FP_RECORD_LEN..end]);
        off = next;
    }
    out
}

// ---- a read-only SQLite reader (just what a table of blobs needs) ----

struct Sqlite<'a> {
    b: &'a [u8],
    page: usize,
    usable: usize,
}

fn be16(b: &[u8], at: usize) -> Option<usize> {
    Some((*b.get(at)? as usize) << 8 | *b.get(at + 1)? as usize)
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some((*b.get(at)? as u32) << 24 | (*b.get(at + 1)? as u32) << 16 | (*b.get(at + 2)? as u32) << 8 | *b.get(at + 3)? as u32)
}

impl<'a> Sqlite<'a> {
    fn new(b: &'a [u8]) -> Option<Sqlite<'a>> {
        if b.get(..16)? != b"SQLite format 3\0" {
            return None;
        }
        let mut page = be16(b, 16)?;
        if page == 1 {
            page = 65536;
        }
        if page < 512 || page > 65536 {
            return None;
        }
        let reserved = *b.get(20)? as usize;
        if reserved >= page {
            return None;
        }
        Some(Sqlite { b, page, usable: page - reserved })
    }

    fn page_off(&self, n: u32) -> Option<usize> {
        let off = (n as usize).checked_sub(1)?.checked_mul(self.page)?;
        if off + self.page > self.b.len() {
            return None;
        }
        Some(off)
    }

    /// SQLite's big-endian 7-bit varint (up to nine bytes).
    fn varint(&self, at: &mut usize) -> Option<u64> {
        let mut v: u64 = 0;
        for i in 0..9 {
            let byte = *self.b.get(*at)?;
            *at += 1;
            if i == 8 {
                v = (v << 8) | byte as u64;
                return Some(v);
            }
            v = (v << 7) | (byte & 0x7F) as u64;
            if byte & 0x80 == 0 {
                return Some(v); // SQLite: the high bit says more bytes follow
            }
        }
        None
    }

    /// A cell's payload, local bytes plus its overflow chain.
    fn payload(&self, raw: &[u8], payload_len: usize) -> Option<Vec<u8>> {
        if payload_len > self.b.len() {
            return None;
        }
        let max_local = self.usable.checked_sub(35)?;
        let mut local = payload_len;
        if payload_len > max_local {
            let min_local = ((self.usable - 12) * 32 / 255).saturating_sub(23);
            let k = min_local + (payload_len - min_local) % (self.usable - 4);
            local = if k <= max_local { k } else { min_local };
        }
        let mut out = Vec::with_capacity(payload_len);
        out.extend_from_slice(raw.get(..local.min(raw.len()))?);
        if payload_len <= max_local {
            return Some(out);
        }
        // an overflow page: its next page number, then usable-4 bytes of the payload
        let mut next = be32(raw, local)?;
        let mut guard = 0;
        while out.len() < payload_len {
            guard += 1;
            if guard > 1_000_000 {
                return None;
            }
            let off = self.page_off(next)?;
            let avail = (self.usable - 4).min(payload_len - out.len());
            out.extend_from_slice(self.b.get(off + 4..off + 4 + avail)?);
            if out.len() >= payload_len {
                break;
            }
            next = be32(self.b, off)?;
        }
        Some(out)
    }

    fn walk_leaf(&self, base: usize, hdr: usize, out: &mut Vec<Vec<u8>>, max_rows: usize) -> Option<()> {
        let ncell = be16(self.b, hdr + 3)?;
        let ptr = hdr + 8;
        let page_end = (base + self.usable).min(self.b.len());
        for i in 0..ncell {
            if out.len() >= max_rows {
                return Some(());
            }
            // (a cell's pointer is from the start of its page, the file header included on page 1)
            let cell = base + be16(self.b, ptr + 2 * i)?;
            let mut at = cell;
            let payload_len = self.varint(&mut at)? as usize;
            let _rowid = self.varint(&mut at)?;
            let local = self.payload(&self.b[at.min(page_end)..page_end], payload_len)?;
            out.push(local);
        }
        Some(())
    }

    fn walk(&self, page: u32, out: &mut Vec<Vec<u8>>, max_rows: usize, depth: usize) -> Option<()> {
        if depth > 64 {
            return None;
        }
        let base = self.page_off(page)?;
        // page 1 holds SQLite's 100-byte file header before its B-tree page header
        let hdr = base + if page == 1 { 100 } else { 0 };
        let kind = *self.b.get(hdr)?;
        match kind {
            0x0D => self.walk_leaf(base, hdr, out, max_rows),
            0x05 => {
                let ncell = be16(self.b, hdr + 3)?;
                for i in 0..ncell {
                    let cell = base + be16(self.b, hdr + 12 + 2 * i)?;
                    let child = be32(self.b, cell)?;
                    self.walk(child, out, max_rows, depth + 1)?;
                }
                let right = be32(self.b, hdr + 8)?;
                self.walk(right, out, max_rows, depth + 1)
            }
            _ => None,
        }
    }

    fn rows_of(&self, root: u32, max_rows: usize) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        if self.walk(root, &mut out, max_rows, 0).is_none() {
            out.clear();
        }
        out
    }

    /// The root page and CREATE statement of the table called `name`.
    fn table(&self, name: &str) -> Option<(u32, String)> {
        // sqlite_master: type, name, tbl_name, rootpage, sql
        for rec in self.rows_of(1, 64) {
            let r = Record::new(&rec);
            if r.text(0)? == "table" && r.text(1)? == name {
                return Some((r.int(3)? as u32, r.text(4)?.to_string()));
            }
        }
        None
    }

    /// The root page of the table called `name`.
    fn root_of(&self, name: &str) -> Option<u32> {
        self.table(name).map(|(root, _)| root)
    }
}

struct Record<'a> {
    b: &'a [u8],
    hdr: usize,
    body: usize,
}

impl<'a> Record<'a> {
    fn new(b: &'a [u8]) -> Record<'a> {
        let mut r = Record { b, hdr: 0, body: 0 };
        let mut at = 0usize;
        let hdr_len = varint_slice(b, &mut at).unwrap_or(0) as usize;
        r.hdr = at.min(b.len());
        r.body = hdr_len.min(b.len());
        r
    }

    /// the serial type of column i
    fn serial(&self, i: usize) -> Option<u64> {
        let mut at = 0usize;
        let _hdr_len = varint_slice(self.b, &mut at)?;
        for _ in 0..i {
            let _ = varint_slice(self.b, &mut at)?;
        }
        varint_slice(self.b, &mut at)
    }

    fn offset_of(&self, i: usize) -> Option<usize> {
        let mut at = 0usize;
        let _hdr_len = varint_slice(self.b, &mut at)?;
        let mut off = self.body;
        for _ in 0..i {
            let t = varint_slice(self.b, &mut at)?;
            off += serial_len(t)?;
        }
        Some(off)
    }

    fn int(&self, i: usize) -> Option<i64> {
        let t = self.serial(i)?;
        let off = self.offset_of(i)?;
        let n = serial_len(t)?;
        let d = self.b.get(off..off + n)?;
        Some(match t {
            8 => 0,
            9 => 1,
            1..=6 => {
                let mut v: i64 = 0;
                for &x in d {
                    v = (v << 8) | x as i64;
                }
                let bits = n * 8;
                if bits < 64 && d.first().is_some_and(|x| x & 0x80 != 0) {
                    v |= -1 << bits;
                }
                v
            }
            7 => 0,
            _ => 0,
        })
    }

    fn text(&self, i: usize) -> Option<&'a str> {
        let t = self.serial(i)?;
        if t < 13 || t % 2 == 0 {
            return None;
        }
        let off = self.offset_of(i)?;
        let n = serial_len(t)?;
        std::str::from_utf8(self.b.get(off..off + n)?).ok()
    }

    fn blob(&self, i: usize) -> Option<&'a [u8]> {
        let t = self.serial(i)?;
        if t < 12 || t % 2 != 0 {
            return None;
        }
        let off = self.offset_of(i)?;
        let n = serial_len(t)?;
        self.b.get(off..off + n)
    }
}

fn serial_len(t: u64) -> Option<usize> {
    Some(match t {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 6,
        6 => 8,
        7 => 8,
        8 | 9 => 0,
        10 | 11 => 0,
        n => ((n - 12) / 2) as usize,
    })
}

fn varint_slice(b: &[u8], at: &mut usize) -> Option<u64> {
    let mut v: u64 = 0;
    for i in 0..9 {
        let byte = *b.get(*at)?;
        *at += 1;
        if i == 8 {
            return Some((v << 8) | byte as u64);
        }
        v = (v << 7) | (byte & 0x7F) as u64;
        if byte & 0x80 == 0 {
            return Some(v); // SQLite's varint: its high bit says another byte follows
        }
    }
    None
}

// ---- the Kindle's notebook catalog ----

/// A standalone notebook in the Kindle's `ksdk.content.db` catalog. Its `id` is the notebook's
/// UUID and the name of its folder under `.notebooks/`; the title is the name the Kindle shows.
/// Notebooks that exist only in Amazon's cloud stay in this catalog without an `nbk` on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindleCatalogNotebook {
    pub id: String,
    pub title: String,
}

/// Reads the Kindle's `Nodes` catalog. `None` when it is not a catalog this build understands.
pub fn catalog_notebooks(data: &[u8]) -> Option<Vec<KindleCatalogNotebook>> {
    let sql = Sqlite::new(data)?;
    let (root, schema) = sql.table("Nodes")?;
    let cols = create_columns(&schema);
    let at = |name: &str| cols.iter().position(|c| c == name);
    let (i_uuid, i_title, i_type) = (at("uuid")?, at("title_collation")?, at("node_type")?);
    let i_extra = at("additional_data");
    let mut out = Vec::new();
    for rec in sql.rows_of(root, 100_000) {
        let r = Record::new(&rec);
        let Some(id) = r.text(i_uuid).filter(|s| !s.is_empty()) else { continue };
        if r.text(i_type) != Some("STANDALONE_NOTEBOOK") {
            continue; // (a book's or personal document's annotation notebook is not one of these)
        }
        let extra = i_extra.and_then(|i| r.text(i)).unwrap_or("");
        if json_number(extra, "delete_state").unwrap_or(0) != 0 {
            continue;
        }
        let title = r
            .text(i_title)
            .filter(|t| !t.trim().is_empty())
            .map(str::to_string)
            .or_else(|| json_string(extra, "title"))
            .unwrap_or_default();
        out.push(KindleCatalogNotebook { id: id.to_string(), title });
    }
    Some(out)
}

/// Column names of a simple `CREATE TABLE` statement, in order.
fn create_columns(sql: &str) -> Vec<String> {
    let (Some(open), Some(close)) = (sql.find('('), sql.rfind(')')) else { return Vec::new() };
    if close <= open {
        return Vec::new();
    }
    sql[open + 1..close]
        .split(',')
        .filter_map(|part| {
            let name = part.trim().split_whitespace().next()?.trim_matches(['"', '`', '[', ']']);
            let lower = name.to_ascii_lowercase();
            if matches!(lower.as_str(), "constraint" | "primary" | "unique" | "check" | "foreign") {
                return None;
            }
            (!lower.is_empty()).then_some(lower)
        })
        .collect()
}

/// A top-level `"key"` value in the catalog's flat JSON `additional_data`.
fn json_after_key<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let at = json.find(&needle)? + needle.len();
    Some(json[at..].trim_start().strip_prefix(':')?.trim_start())
}

fn json_string(json: &str, key: &str) -> Option<String> {
    let mut chars = json_after_key(json, key)?.strip_prefix('"')?.chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?).unwrap_or('\u{FFFD}'));
                }
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

fn json_number(json: &str, key: &str) -> Option<i64> {
    let rest = json_after_key(json, key)?;
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '-')).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

// ---- Amazon Ion (binary), the subset a notebook holds ----

#[derive(Clone, Debug)]
enum Ion {
    Null,
    Bool,
    Int(i64),
    Float(f64),
    Symbol(String),
    Str(String),
    Blob(Vec<u8>),
    List(Vec<Ion>),
    Struct(Vec<(String, Ion)>),
}

impl Ion {
    fn get(&self, key: &str) -> Option<&Ion> {
        match self {
            Ion::Struct(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    /// A field that names another fragment: kfx writes it as a symbol or as a string.
    fn as_ref_name(&self) -> Option<&str> {
        match self {
            Ion::Symbol(s) | Ion::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_int(&self) -> Option<i64> {
        match self {
            Ion::Int(v) => Some(*v),
            _ => None,
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            Ion::Int(v) => Some(*v as f64),
            Ion::Float(v) => Some(*v),
            _ => None,
        }
    }
    fn as_symbol(&self) -> Option<&str> {
        match self {
            Ion::Symbol(s) => Some(s),
            Ion::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_blob(&self) -> Option<&[u8]> {
        match self {
            Ion::Blob(b) => Some(b),
            _ => None,
        }
    }
    fn as_list(&self) -> Option<&[Ion]> {
        match self {
            Ion::List(v) => Some(v),
            _ => None,
        }
    }
}

/// symbols 1..=9 are Ion's, 10..=imported_max are `$<id>` (KFX's shared YJ_symbols table), and the
/// rest are the local table's names in order.
struct SymTab {
    imported_max: u32,
    local: Vec<Option<String>>,
}

impl SymTab {
    fn new() -> SymTab {
        SymTab { imported_max: 0, local: Vec::new() }
    }

    fn name(&self, id: u32) -> String {
        const SYSTEM: [&str; 9] = [
            "$ion",
            "$ion_1_0",
            "$ion_symbol_table",
            "name",
            "version",
            "imports",
            "symbols",
            "max_id",
            "$ion_shared_symbol_table",
        ];
        if (1..=9).contains(&id) {
            return SYSTEM[id as usize - 1].to_string();
        }
        if id <= 9 + self.imported_max {
            return format!("${}", id);
        }
        let i = (id - 10 - self.imported_max) as usize;
        self.local.get(i).and_then(|s| s.clone()).unwrap_or_else(|| format!("${}", id))
    }

    fn apply(&mut self, table: &Ion) -> bool {
        let Ion::Struct(_) = table else {
            return false;
        };
        let mut imported = 0u32;
        if let Some(Ion::List(imports)) = table.get("imports") {
            for im in imports {
                let name = im.get("name").and_then(Ion::as_symbol).unwrap_or("");
                let max = im.get("max_id").and_then(Ion::as_int).unwrap_or(0).max(0) as u32;
                if name == "YJ_symbols" {
                    imported = imported.max(max);
                }
            }
        }
        let mut local = Vec::new();
        if let Some(Ion::List(symbols)) = table.get("symbols") {
            for s in symbols {
                local.push(s.as_symbol().map(|x| x.to_string()));
            }
        }
        self.imported_max = imported;
        self.local = local;
        true
    }

    fn is_ready(&self) -> bool {
        self.imported_max != 0 || !self.local.is_empty()
    }
}

struct IonReader<'a> {
    b: &'a [u8],
    i: usize,
    sym: &'a SymTab,
}

impl<'a> IonReader<'a> {
    fn new(b: &'a [u8], sym: &'a SymTab) -> IonReader<'a> {
        IonReader { b, i: 0, sym }
    }

    fn byte(&mut self) -> Option<u8> {
        let b = *self.b.get(self.i)?;
        self.i += 1;
        Some(b)
    }

    fn var_uint(&mut self) -> Option<u64> {
        let mut v: u64 = 0;
        for _ in 0..10 {
            let byte = self.byte()?;
            v = (v << 7) | (byte & 0x7F) as u64;
            if byte & 0x80 != 0 {
                return Some(v);
            }
            if v > (1 << 50) {
                return None;
            }
        }
        None
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.i.checked_add(n)?;
        let s = self.b.get(self.i..end)?;
        self.i = end;
        Some(s)
    }

    fn read_all(&mut self) -> Vec<(Vec<String>, Ion)> {
        let mut out = Vec::new();
        while self.i < self.b.len() {
            if self.b.get(self.i) == Some(&0xE0) {
                if self.b.get(self.i..self.i + 4) != Some(&[0xE0, 0x01, 0x00, 0xEA]) {
                    break;
                }
                self.i += 4;
                continue;
            }
            match self.read() {
                Some(v) => out.push(v),
                None => break,
            }
        }
        out
    }

    fn read(&mut self) -> Option<(Vec<String>, Ion)> {
        loop {
            let d = self.byte()?;
            let t = d >> 4;
            let l = d & 0x0F;
            if d == 0x00 {
                continue; // NOP padding
            }
            return match t {
                0 => Some((Vec::new(), Ion::Null)),
                1 => Some((Vec::new(), Ion::Bool)),
                2 | 3 => {
                    if l == 15 {
                        return Some((Vec::new(), Ion::Null));
                    }
                    let n = if l == 14 { self.var_uint()? as usize } else { l as usize };
                    let data = self.take(n)?;
                    let mut v: i128 = 0;
                    for &x in data {
                        v = (v << 8) | x as i128;
                    }
                    let v = if t == 3 { -v } else { v };
                    Some((Vec::new(), Ion::Int(v.clamp(i64::MIN as i128, i64::MAX as i128) as i64)))
                }
                4 => {
                    let n = if l == 14 { self.var_uint()? as usize } else { l as usize };
                    let data = self.take(n)?;
                    let v = match n {
                        0 => 0.0,
                        4 => f32::from_be_bytes([data[0], data[1], data[2], data[3]]) as f64,
                        8 => f64::from_be_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]),
                        _ => return None,
                    };
                    Some((Vec::new(), Ion::Float(v)))
                }
                5 | 6 => {
                    let n = if l == 14 { self.var_uint()? as usize } else { l as usize };
                    self.take(n)?;
                    Some((Vec::new(), Ion::Null))
                }
                7 => {
                    if l == 15 {
                        return Some((Vec::new(), Ion::Null));
                    }
                    let id = if l == 0 {
                        0
                    } else {
                        let data = self.take(l as usize)?;
                        let mut v: u32 = 0;
                        for &x in data {
                            v = (v << 8) | x as u32;
                        }
                        v
                    };
                    Some((Vec::new(), Ion::Symbol(self.sym.name(id))))
                }
                8 | 9 | 10 => {
                    let n = if l == 14 { self.var_uint()? as usize } else { l as usize };
                    let data = self.take(n)?;
                    match t {
                        8 => Some((Vec::new(), Ion::Str(String::from_utf8_lossy(data).into_owned()))),
                        _ => Some((Vec::new(), Ion::Blob(data.to_vec()))),
                    }
                }
                11 | 12 => {
                    let n = if l == 14 { self.var_uint()? as usize } else { l as usize };
                    let end = self.i.checked_add(n)?;
                    if end > self.b.len() {
                        return None;
                    }
                    let mut v = Vec::new();
                    while self.i < end {
                        if self.b.get(self.i) == Some(&0x00) {
                            self.i += 1;
                            continue;
                        }
                        v.push(self.read()?.1);
                    }
                    Some((Vec::new(), Ion::List(v)))
                }
                13 => {
                    if l == 15 {
                        return Some((Vec::new(), Ion::Null));
                    }
                    let n = if l == 14 || l == 1 { self.var_uint()? as usize } else { l as usize };
                    let end = self.i.checked_add(n)?;
                    if end > self.b.len() {
                        return None;
                    }
                    let mut fields = Vec::new();
                    while self.i < end {
                        if self.b.get(self.i) == Some(&0x00) {
                            self.i += 1;
                            continue;
                        }
                        let id = self.var_uint()? as u32;
                        let name = self.sym.name(id);
                        let (_, v) = self.read()?;
                        fields.push((name, v));
                    }
                    Some((Vec::new(), Ion::Struct(fields)))
                }
                14 => {
                    let n = if l == 14 { self.var_uint()? as usize } else { l as usize };
                    let end = self.i.checked_add(n)?;
                    if end > self.b.len() {
                        return None;
                    }
                    let alen = self.var_uint()? as usize;
                    let adata = self.take(alen)?;
                    let mut a = IonReader { b: adata, i: 0, sym: self.sym };
                    let mut anns = Vec::new();
                    while a.i < a.b.len() {
                        anns.push(a.sym.name(a.var_uint()? as u32));
                    }
                    let (_, v) = self.read()?;
                    if self.i != end {
                        return None;
                    }
                    Some((anns, v))
                }
                _ => Some((Vec::new(), Ion::Null)),
            };
        }
    }
}

// ---- fragments and pages ----

struct Frag {
    id: String,
    ftype: String,
    value: Ion,
}

fn fragments_from_db(db: &[u8]) -> Option<(SymTab, Vec<Frag>)> {
    let sql = Sqlite::new(db)?;
    let mut sym = SymTab::new();

    // (id -> (payload_type, blob, deleted)): the deltas over the fragments they replace
    let mut rows: Vec<(String, String, Vec<u8>, bool)> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (table, delta) in [("fragments", false), ("local_delta_fragments", true)] {
        let Some(root) = sql.root_of(table) else {
            continue;
        };
        for rec in sql.rows_of(root, 1_000_000) {
            let r = Record::new(&rec);
            let Some(id) = r.text(0) else { continue };
            let Some(ptype) = r.text(1) else { continue };
            let Some(blob) = r.blob(2) else { continue };
            let deleted = delta && r.int(3).unwrap_or(0) != 0;
            if let Some(&i) = seen.get(id) {
                if delta {
                    rows[i] = (id.to_string(), ptype.to_string(), blob.to_vec(), deleted);
                }
            } else {
                seen.insert(id.to_string(), rows.len());
                rows.push((id.to_string(), ptype.to_string(), blob.to_vec(), deleted));
            }
        }
    }

    // the symbol table first (by its id, wherever it sits), then every fragment in the file's order
    let sym_at = rows.iter().position(|r| r.0 == "$ion_symbol_table");
    if let Some(i) = sym_at {
        if rows[i].1 == "blob" {
            let mut rd = IonReader::new(&rows[i].2, &sym);
            if let Some((_, v)) = rd.read_all().into_iter().next() {
                sym.apply(&v);
            }
        }
    }
    if !sym.is_ready() {
        return None;
    }

    let mut frags = Vec::new();
    for (id, ptype, blob, deleted) in &rows {
        if *deleted || id == "$ion_symbol_table" || ptype != "blob" {
            continue;
        }
        if !blob.starts_with(&[0xE0, 0x01, 0x00, 0xEA]) {
            continue;
        }
        let mut rd = IonReader::new(blob, &sym);
        let Some((anns, value)) = rd.read_all().into_iter().next() else { continue };
        let Some(ftype) = anns.first() else { continue };
        frags.push(Frag { id: id.clone(), ftype: ftype.clone(), value });
    }
    Some((sym, frags))
}

fn find_by_name<'a>(frags: &'a [Frag], name: &str) -> Option<&'a Frag> {
    frags.iter().find(|f| f.id == name).or_else(|| frags.iter().find(|f| f.value.get("$176").and_then(Ion::as_ref_name) == Some(name)))
}

fn stroke_from_ion(s: &Ion) -> Option<NbkStroke> {
    let t = s.get("nmdl.thickness").and_then(Ion::as_f64).unwrap_or(23.625);
    let brush = s.get("nmdl.brush_type").and_then(Ion::as_int).unwrap_or(7);
    let bounds = s.get("nmdl.stroke_bounds").and_then(Ion::as_list)?;
    let bx = bounds.first().and_then(Ion::as_int).unwrap_or(0) as f64;
    let by = bounds.get(1).and_then(Ion::as_int).unwrap_or(0) as f64;
    let pts = s.get("nmdl.stroke_points")?;
    let n = pts.get("nmdl.num_points").and_then(Ion::as_int).unwrap_or(0);
    if n <= 0 || n as usize > NBK_MAX_POINTS {
        return None;
    }
    let n = n as usize;
    let xs = decode_values(pts.get("nmdl.position_x")?.as_blob()?, n)?;
    let ys = decode_values(pts.get("nmdl.position_y")?.as_blob()?, n)?;
    let pressure = pts
        .get("nmdl.pressure")
        .and_then(Ion::as_blob)
        .and_then(|b| decode_values(b, n))
        .map(|v| v.iter().map(|&p| (p as f32 / 4096.0).clamp(0.0, 1.0)).collect())
        .unwrap_or_default();
    let mut points = Vec::with_capacity(n);
    for i in 0..n {
        points.push((xs[i] + bx, ys[i] + by));
    }
    Some(NbkStroke { points, pressure, thickness: t, brush })
}

/// Amazon's delta-of-delta point arrays: `01 01`, a count, packed 4-bit instructions, then the
/// operands. Each instruction's bit 3 negates, bit 2 makes the low two bits the step, and the low
/// two bits select 0, 1, 2 or 3 operand bytes (little-endian).
fn decode_values(data: &[u8], n: usize) -> Option<Vec<f64>> {
    if data.len() < 6 || data[..2] != [0x01, 0x01] {
        return None;
    }
    let stored = u32::from_le_bytes([data[2], data[3], data[4], data[5]]) as usize;
    if stored == 0 && n != 0 {
        return None;
    }
    let mut at = 6usize;
    let mut instrs: Vec<u8> = Vec::with_capacity(n);
    while instrs.len() < n {
        let b = *data.get(at)?;
        at += 1;
        instrs.push(b >> 4);
        if instrs.len() < n {
            instrs.push(b & 0x0F);
        }
    }
    let mut vals = Vec::with_capacity(n);
    let mut change: i64 = 0;
    let mut value: i64 = 0;
    for (i, &instr) in instrs.iter().enumerate() {
        let nib = instr & 3;
        let mut inc: i64 = if instr & 4 != 0 {
            nib as i64
        } else {
            match nib {
                0 => 0,
                1 => {
                    let v = *data.get(at)? as i64;
                    at += 1;
                    v
                }
                2 => {
                    let v = u16::from_le_bytes([*data.get(at)?, *data.get(at + 1)?]) as i64;
                    at += 2;
                    v
                }
                _ => {
                    let v = *data.get(at)? as i64 | ((u16::from_le_bytes([*data.get(at + 1)?, *data.get(at + 2)?]) as i64) << 8);
                    at += 3;
                    v
                }
            }
        };
        if instr & 8 != 0 {
            inc = -inc;
        }
        if i == 0 {
            change = 0;
            value = inc;
        } else {
            change += inc;
            value += change;
        }
        vals.push(value as f64);
    }
    Some(vals)
}

fn collect_strokes(v: &Ion, frags: &[Frag], out: &mut Vec<NbkStroke>, points: &mut usize, depth: usize) {
    if depth > NBK_MAX_DEPTH || out.len() >= NBK_MAX_STROKES {
        return;
    }
    if v.get("nmdl.type").and_then(Ion::as_symbol) == Some("nmdl.stroke") || v.get("nmdl.stroke_points").is_some() {
        if *points < NBK_MAX_POINTS {
            if let Some(s) = stroke_from_ion(v) {
                *points += s.points.len();
                out.push(s);
            }
        }
        return;
    }
    // a content entry names the story fragment its ink lives in (`$176`, a symbol or a string)
    if let Some(name) = v.get("$176").and_then(Ion::as_ref_name) {
        if let Some(f) = find_by_name(frags, name) {
            if let Some(Ion::List(children)) = f.value.get("$146") {
                for c in children {
                    collect_strokes(c, frags, out, points, depth + 1);
                }
            }
        }
    }
    for key in ["$146", "$141"] {
        if let Some(Ion::List(children)) = v.get(key) {
            for c in children {
                collect_strokes(c, frags, out, points, depth + 1);
            }
        }
    }
}

fn reading_order(frags: &[Frag]) -> Vec<String> {
    let Some(f) = frags.iter().find(|f| f.ftype == "$538").or_else(|| frags.iter().find(|f| f.ftype == "$258")) else {
        return Vec::new();
    };
    let Some(Ion::List(entries)) = f.value.get("$169") else { return Vec::new() };
    for e in entries {
        // (the template collection is a string-named entry; the notebook's is a symbol)
        if matches!(e.get("$178"), Some(Ion::Symbol(_))) {
            if let Some(Ion::List(ids)) = e.get("$170") {
                return ids.iter().filter_map(|v| v.as_ref_name().map(str::to_string)).collect();
            }
        }
    }
    Vec::new()
}

fn page_title(frags: &[Frag]) -> String {
    // book_metadata: a `kindle_title_metadata` entry's `title` (when the Scribe holds one)
    let Some(f) = frags.iter().find(|f| f.ftype == "$490") else { return String::new() };
    // (the metadata hangs off an empty-named field of book_metadata)
    let root = if f.value.get("$491").is_some() { &f.value } else { f.value.get("").unwrap_or(&f.value) };
    let Some(Ion::List(entries)) = root.get("$491") else { return String::new() };
    for e in entries {
        if e.get("$495").and_then(Ion::as_ref_name) != Some("kindle_title_metadata") {
            continue;
        }
        let Some(Ion::List(kv)) = e.get("$258") else { continue };
        for pair in kv {
            if pair.get("$492").and_then(Ion::as_ref_name) == Some("title") {
                if let Some(t) = pair.get("$307").and_then(Ion::as_ref_name) {
                    return t.trim().to_string();
                }
            }
        }
    }
    String::new()
}

/// Reads an `nbk` file (as it lies on the Kindle, fingerprint wrapping included) into its pages.
pub fn nbk_notebook(data: &[u8]) -> Result<NbkNotebook, &'static str> {
    if data.len() > NBK_MAX_BYTES {
        return Err("the notebook is too big");
    }
    let db = unwrap_fingerprints(data);
    let (_sym, frags) = fragments_from_db(&db).ok_or("its fragments do not read")?;
    if frags.is_empty() {
        return Err("it holds no fragments");
    }
    // `$169` gives the notebook's reading order; `note_template_collection` holds blank templates,
    // not pages the owner wrote. Without it, the database's own order is all there is.
    let mut ordered: Vec<&Frag> = Vec::new();
    for id in reading_order(&frags) {
        if let Some(f) = frags.iter().find(|f| f.id == id && f.ftype == "$260") {
            ordered.push(f);
        }
    }
    if ordered.is_empty() {
        ordered.extend(frags.iter().filter(|f| f.ftype == "$260"));
    }

    let mut pages = Vec::new();
    let mut total_strokes = 0;
    let mut total_points = 0;
    for f in ordered {
        let cw = f.value.get("nmdl.canvas_width").and_then(Ion::as_f64).unwrap_or(NBK_CANVAS_W);
        let ch = f.value.get("nmdl.canvas_height").and_then(Ion::as_f64).unwrap_or(NBK_CANVAS_H);
        let mut strokes = Vec::new();
        let mut points = 0;
        if let Some(Ion::List(content)) = f.value.get("$141") {
            for c in content {
                collect_strokes(c, &frags, &mut strokes, &mut points, 0);
            }
        }
        total_strokes += strokes.len();
        total_points += points;
        pages.push(NbkPage {
            canvas_w: if cw > 0.0 { cw } else { NBK_CANVAS_W },
            canvas_h: if ch > 0.0 { ch } else { NBK_CANVAS_H },
            strokes,
        });
    }
    if pages.is_empty() {
        return Err("it holds no pages");
    }
    Ok(NbkNotebook { title: page_title(&frags), pages, strokes: total_strokes, points: total_points })
}

/// The Grimoire tool and ink settings for Amazon's brush and thickness, at the device's dpi.
pub fn brush_for(brush: i64, thickness: f64) -> (ToolKind, ToolParams) {
    let t = match brush {
        1 => TOOL_HIGHLIGHTER,
        5 => TOOL_PENCIL,
        6 => TOOL_FOUNTAIN,
        7 => TOOL_BALLPOINT,
        9 => TOOL_MARKER,
        _ => TOOL_BALLPOINT,
    };
    let mut p = ToolParams::default();
    tool_params_default(t, &mut p);
    // Amazon's thickness is in the canvas's units: 2520 to the inch
    let mm = (thickness / NBK_PPI * 25.4).clamp(0.05, 12.0) as f32;
    p.size_min_mm = mm;
    p.size_max_mm = mm;
    (t, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_are_removed_and_a_plain_file_is_left_alone() {
        let plain = vec![7u8; 4096];
        assert_eq!(unwrap_fingerprints(&plain), plain);
        let mut wrapped = vec![1u8; 1024];
        wrapped.extend_from_slice(&FP_SIG);
        wrapped.resize(2048, 2);
        wrapped.extend_from_slice(&[3u8; 100]);
        let out = unwrap_fingerprints(&wrapped);
        assert_eq!(out.len(), 1024 + 100);
        assert_eq!(&out[1024..], &[3u8; 100]);
    }

    #[test]
    fn reading_order_takes_the_notebook_and_skips_template_pages() {
        let page = |id: &str| Frag { id: id.into(), ftype: "$260".into(), value: Ion::Struct(Vec::new()) };
        let meta = Frag {
            id: "metadata".into(),
            ftype: "$258".into(),
            value: Ion::Struct(vec![(
                "$169".into(),
                Ion::List(vec![
                    Ion::Struct(vec![
                        ("$178".into(), Ion::Str("note_template_collection".into())),
                        ("$170".into(), Ion::List(vec![Ion::Str("t1".into()), Ion::Str("t2".into())])),
                    ]),
                    Ion::Struct(vec![
                        ("$178".into(), Ion::Symbol("$351".into())),
                        ("$170".into(), Ion::List(vec![Ion::Str("p2".into()), Ion::Str("p1".into())])),
                    ]),
                ]),
            )]),
        };
        let frags = vec![meta, page("p1"), page("p2"), page("t1"), page("t2")];
        assert_eq!(reading_order(&frags), vec!["p2".to_string(), "p1".to_string()]);
    }

    #[test]
    fn catalog_schema_and_json_fields_are_read() {
        assert_eq!(
            create_columns("CREATE TABLE Nodes (uuid TEXT PRIMARY KEY, title_collation TEXT, node_type TEXT, additional_data TEXT)"),
            ["uuid", "title_collation", "node_type", "additional_data"]
        );
        let j = r#"{"title":"Caf\u00e9 \"notes\"","delete_state":0}"#;
        assert_eq!(json_string(j, "title").as_deref(), Some("Caf\u{e9} \"notes\""));
        assert_eq!(json_number(j, "delete_state"), Some(0));
        assert_eq!(json_string(j, "missing"), None);
    }

    #[test]
    fn stroke_values_decode_a_delta_of_delta_stream() {
        // 3 values: 10, 10+3=13, 13+(3-1)=15 (nibbles 1, 1, 9, packed 11 90)
        let mut d = vec![1, 1, 3, 0, 0, 0];
        d.extend_from_slice(&[0x11, 0x90]);
        d.extend_from_slice(&[10, 3, 1]);
        assert_eq!(decode_values(&d, 3), Some(vec![10.0, 13.0, 15.0]));
    }
}
