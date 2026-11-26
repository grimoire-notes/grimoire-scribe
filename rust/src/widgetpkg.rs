//! Widget packages (issue #200): manifests, platform checks, installation and the installed index.
//!
//! A package is a `package.yaml` and an artifact under `packages/widgets/<id>/`, with the list of
//! installed packages in `packages/index.json`. The files are the source of truth; a manifest is
//! validated before anything is written and an install is staged and swapped in whole.
//!
//! This module is the package manager's store and its pure parts. Fetching a repository and drawing
//! the Settings → Packages → Widgets page live in widgetpkgui.rs; the WASM host is a separate runtime.

use crate::prelude::*;
use crate::sha256::sha256_of;
use std::cell::RefCell;

pub const WIDGET_PKG_INDEX: &str = "packages/index.json";
pub const WIDGET_PKG_DIR: &str = "packages/widgets";
pub const WIDGET_PKG_MANIFEST_MAX: usize = 256 * 1024;
pub const WIDGET_PKG_ARTIFACT_MAX: u64 = 32 * 1024 * 1024;
pub const WIDGET_PKG_COUNT_MAX: usize = 64;
pub const WIDGET_PKG_URL_MAX: usize = 1024;
pub const WIDGET_SETTINGS_MAX: usize = 32;
pub const WIDGET_SETTINGS_KEY_MAX: usize = 64;
const WIDGET_SETTINGS_VALUE_MAX: usize = 64;
const WIDGET_SETTINGS_CHOICES_MAX: usize = 16;
const WIDGET_SETTINGS_FILE_MAX: usize = 64 * 1024;

// ---- the platform registry ----

/// Every platform a manifest may name. `all` is the sentinel: it matches every device Grimoire supports
/// and is never a device's own id.
pub const WIDGET_PLATFORMS: [(&str, &str); 3] = [
    ("all", "All supported devices"),
    ("kindle-scribe-2022", "Kindle Scribe (2022)"),
    ("kindle-scribe-2024", "Kindle Scribe (2024)"),
];

pub fn widget_platform_known(id: &str) -> bool {
    WIDGET_PLATFORMS.iter().any(|(k, _)| *k == id)
}

pub fn widget_platform_label(id: &str) -> &'static str {
    WIDGET_PLATFORMS.iter().find(|(k, _)| *k == id).map_or("", |(_, v)| *v)
}

/// The current device's platform id, or `unknown` where Grimoire cannot tell (only `all` matches then).
/// `GRIMOIRE_WIDGET_PLATFORM` is the simulator's and a test's way to stand in for a model.
pub fn widget_current_platform() -> &'static str {
    if let Ok(v) = std::env::var("GRIMOIRE_WIDGET_PLATFORM") {
        let v = v.trim();
        if widget_platform_known(v) && v != "all" {
            return match v {
                "kindle-scribe-2022" => "kindle-scribe-2022",
                "kindle-scribe-2024" => "kindle-scribe-2024",
                _ => "unknown",
            };
        }
        return "unknown";
    }
    if cfg!(feature = "kindle") {
        let mut model = String::new();
        if let Ok(raw) = std::fs::read("/proc/device-tree/model") {
            model = raw.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect();
        }
        let lower = model.to_lowercase();
        if lower.contains("scribe") {
            if lower.contains("2024") {
                return "kindle-scribe-2024";
            }
            return "kindle-scribe-2022";
        }
    }
    "unknown"
}

pub fn widget_platform_supported(platforms: &[String]) -> bool {
    if platforms.iter().any(|p| p == "all") {
        return true;
    }
    let current = widget_current_platform();
    current != "unknown" && platforms.iter().any(|p| p == current)
}

// ---- a small YAML reader: the manifest's subset ----
//
// The manifest uses mappings, block and inline sequences, and plain or quoted scalars. A full YAML
// implementation would be more than the format needs; this reader refuses anything else instead of
// guessing, so a manifest that parses is one whose fields were read as written.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Yaml {
    Scalar(String),
    List(Vec<Yaml>),
    Map(Vec<(String, Yaml)>),
}

impl Yaml {
    fn get(&self, key: &str) -> Option<&Yaml> {
        match self {
            Yaml::Map(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn str(&self) -> Option<&str> {
        match self {
            Yaml::Scalar(s) => Some(s),
            _ => None,
        }
    }
    fn list(&self) -> Option<&[Yaml]> {
        match self {
            Yaml::List(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
struct YLine {
    indent: usize,
    text: String,
}

fn yaml_lines(text: &str) -> Result<Vec<YLine>, String> {
    let mut out = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let raw = raw.trim_end_matches('\r');
        let mut s = raw;
        if n == 0 {
            s = s.trim_start_matches('\u{feff}');
        }
        let mut indent = 0usize;
        for c in s.chars() {
            match c {
                ' ' => indent += 1,
                '\t' => return Err("a tab is used for indentation".into()),
                _ => break,
            }
        }
        let body = &s[indent..];
        if body.trim().is_empty() || body.starts_with('#') || body == "---" || body == "..." {
            continue;
        }
        let text = strip_yaml_comment(body);
        if text.trim().is_empty() {
            continue;
        }
        out.push(YLine { indent, text: text.trim_end().to_string() });
    }
    Ok(out)
}

fn strip_yaml_comment(s: &str) -> &str {
    let mut quote = 0u8;
    let mut escaped = false;
    for (i, b) in s.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match quote {
            b'"' if b == b'\\' => escaped = true,
            b'"' if b == b'"' => quote = 0,
            b'\'' if b == b'\'' => quote = 0,
            0 if b == b'"' || b == b'\'' => quote = b,
            0 if b == b'#' && (i == 0 || s.as_bytes()[i - 1].is_ascii_whitespace()) => return &s[..i],
            _ => {}
        }
    }
    s
}

fn yaml_scalar(s: &str) -> Result<Yaml, String> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(Yaml::Scalar(String::new()));
    }
    if t.starts_with('[') {
        if !t.ends_with(']') {
            return Err("an unclosed list".into());
        }
        let inner = &t[1..t.len() - 1];
        let mut v = Vec::new();
        for part in split_inline(inner)? {
            if !part.trim().is_empty() {
                v.push(yaml_scalar(part)?);
            }
        }
        return Ok(Yaml::List(v));
    }
    if t.starts_with('"') {
        if !t.ends_with('"') || t.len() < 2 {
            return Err("an unclosed quoted value".into());
        }
        let mut out = String::new();
        let mut it = t[1..t.len() - 1].chars();
        while let Some(c) = it.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            let Some(e) = it.next() else { return Err("a quoted value ends in a backslash".into()) };
            out.push(match e {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '"' => '"',
                '\\' => '\\',
                _ => return Err(format!("an unknown escape \\{e}")),
            });
        }
        return Ok(Yaml::Scalar(out));
    }
    if t.starts_with('\'') {
        if !t.ends_with('\'') || t.len() < 2 {
            return Err("an unclosed quoted value".into());
        }
        return Ok(Yaml::Scalar(t[1..t.len() - 1].replace("''", "'")));
    }
    Ok(Yaml::Scalar(t.to_string()))
}

fn split_inline(s: &str) -> Result<Vec<&str>, String> {
    let mut v = Vec::new();
    let mut start = 0usize;
    let mut quote = 0u8;
    let mut depth = 0i32;
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if quote != 0 {
            if b == quote {
                if quote == b'\'' && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                quote = 0;
            } else if b == b'\\' && quote == b'"' {
                i += 1;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' | b'\'' => quote = b,
            b'[' => depth += 1,
            b']' => depth -= 1,
            b',' if depth == 0 => {
                v.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    if quote != 0 {
        return Err("an unclosed quote".into());
    }
    if depth != 0 {
        return Err("an unclosed list".into());
    }
    v.push(&s[start..]);
    Ok(v)
}

fn yaml_parse_block(lines: &[YLine], at: &mut usize, indent: usize) -> Result<Yaml, String> {
    if *at >= lines.len() || lines[*at].indent < indent {
        return Ok(Yaml::Map(Vec::new()));
    }
    let seq = lines[*at].text == "-" || lines[*at].text.starts_with("- ");
    if seq {
        let mut v = Vec::new();
        while *at < lines.len() && lines[*at].indent == indent && (lines[*at].text == "-" || lines[*at].text.starts_with("- ")) {
            let rest = lines[*at].text[1..].trim_start().to_string();
            *at += 1;
            if rest.is_empty() {
                let child = if *at < lines.len() && lines[*at].indent > indent {
                    yaml_parse_block(lines, at, lines[*at].indent)?
                } else {
                    Yaml::Scalar(String::new())
                };
                v.push(child);
                continue;
            }
            if let Some((key, value)) = yaml_split_key(&rest) {
                let mut kv = Vec::new();
                if value.is_empty() {
                    let child = if *at < lines.len() && lines[*at].indent > indent {
                        yaml_parse_block(lines, at, lines[*at].indent)?
                    } else {
                        Yaml::Scalar(String::new())
                    };
                    kv.push((key.to_string(), child));
                } else {
                    kv.push((key.to_string(), yaml_scalar(value)?));
                }
                // Further keys of the same sequence item are indented under it.
                while *at < lines.len() && lines[*at].indent > indent {
                    let child_indent = lines[*at].indent;
                    let Yaml::Map(more) = yaml_parse_block(lines, at, child_indent)? else {
                        return Err("a sequence item mixes a value with a mapping".into());
                    };
                    kv.extend(more);
                }
                v.push(Yaml::Map(kv));
            } else {
                v.push(yaml_scalar(&rest)?);
            }
        }
        return Ok(Yaml::List(v));
    }
    let mut kv = Vec::new();
    while *at < lines.len() && lines[*at].indent == indent && !(lines[*at].text == "-" || lines[*at].text.starts_with("- ")) {
        let line = lines[*at].text.clone();
        let Some((key, value)) = yaml_split_key(&line) else {
            return Err(format!("`{line}` is not a key and a value"));
        };
        *at += 1;
        if value.is_empty() {
            let child = if *at < lines.len() && lines[*at].indent > indent {
                yaml_parse_block(lines, at, lines[*at].indent)?
            } else {
                Yaml::Scalar(String::new())
            };
            kv.push((key.to_string(), child));
        } else {
            kv.push((key.to_string(), yaml_scalar(value)?));
        }
    }
    Ok(Yaml::Map(kv))
}

fn yaml_split_key(s: &str) -> Option<(&str, &str)> {
    let mut quote = 0u8;
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if quote != 0 {
            if b == quote {
                quote = 0;
            } else if b == b'\\' && quote == b'"' {
                i += 1;
            }
            i += 1;
            continue;
        }
        if b == b'"' || b == b'\'' {
            quote = b;
        } else if b == b':' && (i + 1 == bytes.len() || bytes[i + 1].is_ascii_whitespace()) {
            let key = s[..i].trim();
            if key.is_empty() {
                return None;
            }
            return Some((key.trim_matches('"').trim_matches('\''), s[i + 1..].trim_start()));
        }
        i += 1;
    }
    None
}

pub fn yaml_parse(text: &str) -> Result<Yaml, String> {
    let lines = yaml_lines(text)?;
    let mut at = 0usize;
    if lines.is_empty() {
        return Ok(Yaml::Map(Vec::new()));
    }
    let indent = lines[0].indent;
    let v = yaml_parse_block(&lines, &mut at, indent)?;
    if at != lines.len() {
        return Err(format!("`{}` is indented under nothing", lines[at].text));
    }
    Ok(v)
}

// ---- manifests ----

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetSizeMode {
    #[default]
    Fixed,
    Resizable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidgetSize {
    pub mode: WidgetSizeMode,
    pub default: (i32, i32),
    pub min: (i32, i32),
    pub max: (i32, i32),
}

impl Default for WidgetSize {
    fn default() -> Self {
        WidgetSize { mode: WidgetSizeMode::Fixed, default: (220, 160), min: (220, 160), max: (220, 160) }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WidgetArtifact {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WidgetRelease {
    pub version: String,
    pub url: String,
    pub sha256: String,
}

/// How a package setting is changed in Settings → Packages → Widgets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetSettingKind {
    /// an on/off switch, its values `true` and `false`
    #[default]
    Toggle,
    /// one of `choices`, changed by tapping the row
    Choice,
}

/// One setting a package declares in its manifest. The host draws it, so a package needs no UI of
/// its own for a switch or a small list of choices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidgetSetting {
    pub key: String,
    pub label: String,
    pub kind: WidgetSettingKind,
    pub default: String,
    pub choices: Vec<String>,
    /// optional icon name, as `spec.icon`; empty means the gear
    pub icon: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WidgetSpec {
    pub runtime: String,
    pub entry: String,
    pub icon: String,
    /// App packages only: the digest and size of a bundled `.ico` icon. Empty/0 for a built-in icon.
    pub icon_sha256: String,
    pub icon_size: u64,
    pub minimizable: bool,
    /// App packages only: show Grimoire's shared status bar over the app. Widget packages ignore it.
    pub status_bar: bool,
    pub size: WidgetSize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WidgetPackage {
    pub name: String,
    pub id: String,
    pub type_: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub license: String,
    pub license_url: String,
    pub min_grimoire_version: String,
    pub platforms: Vec<String>,
    pub repository: String,
    pub artifact: WidgetArtifact,
    pub releases: Vec<WidgetRelease>,
    pub capabilities: Vec<String>,
    pub settings: Vec<WidgetSetting>,
    pub spec: WidgetSpec,
}

fn ystr(v: &Yaml, key: &str) -> String {
    v.get(key).and_then(Yaml::str).unwrap_or("").trim().to_string()
}

fn ybool(v: &Yaml, key: &str, default: bool) -> bool {
    match v.get(key).and_then(Yaml::str) {
        Some("true") => true,
        Some("false") => false,
        _ => default,
    }
}

fn ylist(v: &Yaml, key: &str) -> Result<Vec<String>, String> {
    let Some(x) = v.get(key) else { return Ok(Vec::new()) };
    match x {
        Yaml::Scalar(s) if s.is_empty() => Ok(Vec::new()),
        Yaml::List(items) => items
            .iter()
            .map(|i| i.str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| format!("`{key}` holds a value that is not a string")),
        _ => Err(format!("`{key}` is not a list")),
    }
}

fn ypair(v: Option<&Yaml>, key: &str, out: &mut (i32, i32)) -> Result<(), String> {
    let Some(v) = v else { return Ok(()) };
    let items = v.list().ok_or_else(|| format!("`{key}` is not a [width, height] list"))?;
    if items.len() != 2 {
        return Err(format!("`{key}` needs two numbers"));
    }
    let mut p = [0i32; 2];
    for (i, it) in items.iter().enumerate() {
        let s = it.str().ok_or_else(|| format!("`{key}` holds a value that is not a number"))?;
        p[i] = s.parse::<i32>().map_err(|_| format!("`{key}` holds `{s}`, not a number"))?;
        if p[i] <= 0 {
            return Err(format!("`{key}` must be positive"));
        }
    }
    *out = (p[0], p[1]);
    Ok(())
}

fn ysize(v: Option<&Yaml>) -> Result<WidgetSize, String> {
    let mut size = WidgetSize::default();
    let Some(v) = v else { return Ok(size) };
    if matches!(v, Yaml::Scalar(s) if s.is_empty()) {
        return Ok(size);
    }
    let mode = v.get("mode").and_then(Yaml::str).unwrap_or("fixed");
    size.mode = match mode {
        "fixed" => WidgetSizeMode::Fixed,
        "resizable" => WidgetSizeMode::Resizable,
        _ => return Err(format!("`spec.size.mode` is `{mode}`, not resizable or fixed")),
    };
    ypair(v.get("default"), "spec.size.default", &mut size.default)?;
    ypair(v.get("min"), "spec.size.min", &mut size.min)?;
    ypair(v.get("max"), "spec.size.max", &mut size.max)?;
    if size.mode == WidgetSizeMode::Fixed {
        size.min = size.default;
        size.max = size.default;
    }
    Ok(size)
}

fn setting_key_ok(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= WIDGET_SETTINGS_KEY_MAX
        && key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-' || b == b'_')
        && key.as_bytes()[0] != b'.'
        && key.as_bytes()[key.len() - 1] != b'.'
        && !key.contains("..")
}

fn setting_text_ok(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}

fn setting_value_ok(s: &WidgetSetting, value: &str) -> bool {
    match s.kind {
        WidgetSettingKind::Toggle => value == "true" || value == "false",
        WidgetSettingKind::Choice => s.choices.iter().any(|c| c == value),
    }
}

fn ysettings(v: &Yaml) -> Result<Vec<WidgetSetting>, String> {
    let Some(x) = v.get("settings") else { return Ok(Vec::new()) };
    if matches!(x, Yaml::Scalar(s) if s.is_empty()) {
        return Ok(Vec::new());
    }
    let items = x.list().ok_or("`settings` is not a list")?;
    if items.len() > WIDGET_SETTINGS_MAX {
        return Err(format!("`settings` has more than {WIDGET_SETTINGS_MAX} entries"));
    }
    let mut out: Vec<WidgetSetting> = Vec::new();
    for (n, item) in items.iter().enumerate() {
        let key = ystr(item, "key");
        if !setting_key_ok(&key) {
            return Err(format!("`settings[{n}].key` must be lowercase letters, digits, dots, hyphens or underscores"));
        }
        if out.iter().any(|s| s.key == key) {
            return Err(format!("`settings[{n}].key` (`{key}`) is used twice"));
        }
        let label = ystr(item, "label");
        if !setting_text_ok(&label, 160) {
            return Err(format!("`settings[{n}].label` is missing or too long"));
        }
        let type_ = ystr(item, "type");
        let kind = match type_.as_str() {
            "toggle" => WidgetSettingKind::Toggle,
            "choice" => WidgetSettingKind::Choice,
            _ => return Err(format!("`settings[{n}].type` must be `toggle` or `choice`")),
        };
        let mut choices = if kind == WidgetSettingKind::Choice { ylist(item, "choices")? } else { Vec::new() };
        let mut default = ystr(item, "default");
        if kind == WidgetSettingKind::Toggle {
            if default.is_empty() {
                default = "true".into();
            }
            if default != "true" && default != "false" {
                return Err(format!("`settings[{n}].default` must be true or false"));
            }
        } else {
            if !(2..=WIDGET_SETTINGS_CHOICES_MAX).contains(&choices.len()) {
                return Err(format!("`settings[{n}].choices` needs 2 to {WIDGET_SETTINGS_CHOICES_MAX} values"));
            }
            for (i, c) in choices.iter().enumerate() {
                if !setting_text_ok(c, WIDGET_SETTINGS_VALUE_MAX) {
                    return Err(format!("`settings[{n}].choices[{i}]` is empty or too long"));
                }
                if choices[..i].contains(c) {
                    return Err(format!("`settings[{n}].choices` repeats `{c}`"));
                }
            }
            if default.is_empty() {
                default = choices[0].clone();
            } else if !choices.contains(&default) {
                return Err(format!("`settings[{n}].default` is not one of its choices"));
            }
        }
        out.push(WidgetSetting {
            key,
            label,
            kind,
            default,
            choices: std::mem::take(&mut choices),
            icon: ystr(item, "icon"),
        });
    }
    Ok(out)
}

/// Reads a `package.yaml`. Unknown top-level keys are kept out and are not an error (forward compatibility).
pub fn widget_package_parse(text: &str) -> Result<WidgetPackage, String> {
    if text.len() > WIDGET_PKG_MANIFEST_MAX {
        return Err("the manifest is too large".into());
    }
    if text.as_bytes().contains(&0) {
        return Err("the manifest holds a NUL byte".into());
    }
    let y = yaml_parse(text)?;
    if !matches!(y, Yaml::Map(_)) {
        return Err("the manifest is not a mapping".into());
    }
    let mut p = WidgetPackage {
        name: ystr(&y, "name"),
        id: ystr(&y, "id"),
        type_: ystr(&y, "type"),
        version: ystr(&y, "version"),
        author: ystr(&y, "author"),
        description: ystr(&y, "description"),
        license: ystr(&y, "license"),
        license_url: ystr(&y, "license_url"),
        min_grimoire_version: ystr(&y, "min_grimoire_version"),
        // (omitted means every supported device; an explicit empty list is not the same and is refused)
        platforms: if y.get("platforms").is_none() { vec!["all".to_string()] } else { ylist(&y, "platforms")? },
        repository: ystr(&y, "repository"),
        capabilities: ylist(&y, "capabilities")?,
        settings: ysettings(&y)?,
        ..Default::default()
    };
    if let Some(a) = y.get("artifact") {
        p.artifact = WidgetArtifact {
            path: ystr(a, "path"),
            sha256: ystr(a, "sha256"),
            size: ystr(a, "size").parse().unwrap_or(0),
        };
    }
    if let Some(rs) = y.get("releases") {
        for r in rs.list().ok_or("`releases` is not a list")? {
            p.releases.push(WidgetRelease {
                version: ystr(r, "version"),
                url: ystr(r, "url"),
                sha256: ystr(r, "sha256"),
            });
        }
    }
    if let Some(s) = y.get("spec") {
        p.spec = WidgetSpec {
            runtime: ystr(s, "runtime"),
            entry: ystr(s, "entry"),
            icon: ystr(s, "icon"),
            icon_sha256: ystr(s, "icon_sha256"),
            icon_size: ystr(s, "icon_size").parse().unwrap_or(0),
            minimizable: ybool(s, "minimizable", true),
            status_bar: ybool(s, "status_bar", true),
            size: ysize(s.get("size"))?,
        };
    }
    Ok(p)
}

/// The longest id a package may have, in bytes (docs/WIDGETS.md).
pub const WIDGET_ID_MAX: usize = 128;

/// What an update adds to a package's id to name the folder it keeps the replaced version in
/// (`packages/widgets/<id>.previous`). No id ends in it, so that folder is never another package's.
pub const WIDGET_ID_PREVIOUS: &str = ".previous";

/// Whether `id` is a package's id: lowercase letters, digits, dots and hyphens, at most WIDGET_ID_MAX, no dot
/// first or last and no two in a row, and not ending in `.previous` (WIDGET_ID_PREVIOUS). The one rule for an
/// id wherever one is read: a manifest, the packages index, a package's folder, and a frame's line in
/// widgets.ini (widgets.rs), so a package that installs is one whose open frame is found again. An app
/// package's id follows it too (apppkg.rs).
pub fn widget_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= WIDGET_ID_MAX
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && id.as_bytes()[0] != b'.'
        && id.as_bytes()[id.len() - 1] != b'.'
        && !id.contains("..")
        && !id.ends_with(WIDGET_ID_PREVIOUS)
}

fn semver_ok(v: &str) -> bool {
    let core = v.split_once('-').map_or(v, |(c, _)| c);
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    for p in &parts {
        if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        if p.len() > 1 && p.starts_with('0') {
            return false;
        }
    }
    if let Some((_, pre)) = v.split_once('-') {
        if pre.is_empty() || !pre.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-') {
            return false;
        }
    }
    true
}

fn sha_ok(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn path_ok(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= 240
        && !p.starts_with('/')
        && !p.contains('\\')
        && p.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
}

/// The capabilities a manifest may declare, and what the host gives a module for each (widgetwasm.rs):
/// drawing in its frame over the notebook, and its own saved state. A widget's manifest declares the first
/// (`widget_package_validate` refuses one that does not); the second is for a widget that saves something.
pub const WIDGET_CAP_OVERLAY: &str = "editor:overlay";
pub const WIDGET_CAP_STORAGE: &str = "storage:widget-local";

/// Whether `artifact.path` may name a package's module: a relative path inside the package's folder (`path_ok`)
/// that is neither the manifest nor anything of `state/`, and whose names the storage walk takes
/// (`StoragePath::walked`), since the module is read through that walk when it is loaded. The manifest and the
/// state folder are named without regard to the case of ASCII letters, as a storage that ignores case finds
/// them (`Package.yaml`, `STATE/x`). The manifest check and the load ask this one question, so a path that
/// installs is a path that loads. An app package's module is held to it too (apppkg.rs).
pub(crate) fn artifact_path_ok(p: &str) -> bool {
    path_ok(p)
        && !p.eq_ignore_ascii_case("package.yaml")
        && !p.split('/').next().is_some_and(|first| first.eq_ignore_ascii_case("state"))
        && crate::fsview::StoragePath::new(p.as_bytes()).is_some_and(|sp| sp.walked())
}

fn cap_ok(c: &str) -> bool {
    c == WIDGET_CAP_OVERLAY || c == WIDGET_CAP_STORAGE
}

/// Validates a manifest for this build and this device. `canvas_w` and `canvas_h` are the editor
/// canvas's size, for the absolute maximum (70% either way).
pub fn widget_package_validate(p: &WidgetPackage, canvas_w: i32, canvas_h: i32) -> Result<(), String> {
    if p.type_ != "widget" {
        return Err("This repository is not a widget package".into());
    }
    if p.id.ends_with(WIDGET_ID_PREVIOUS) {
        return Err("`id` cannot end in `.previous`; an update keeps the replaced version's folder under that name".into());
    }
    if !widget_id_ok(&p.id) {
        return Err("`id` must be lowercase letters, digits, dots and hyphens".into());
    }
    if p.name.trim().is_empty() {
        return Err("`name` is missing".into());
    }
    // (the index keeps these four of a package: no longer than its reader takes of a string)
    for (field, text) in [("name", &p.name), ("version", &p.version), ("author", &p.author), ("description", &p.description)] {
        if text.len() > WIDGET_PKG_TEXT_MAX {
            return Err(format!("`{field}` is longer than {WIDGET_PKG_TEXT_MAX} bytes"));
        }
    }
    if !semver_ok(&p.version) {
        return Err(format!("`version` (`{}`) is not a valid version", p.version));
    }
    if !p.min_grimoire_version.is_empty() {
        if !update_version_key(&p.min_grimoire_version, &mut [0; 3]) {
            return Err("`min_grimoire_version` is not a valid version".into());
        }
        if update_version_cmp(GRIMOIRE_VERSION, &p.min_grimoire_version) < 0 {
            return Err(format!("This widget needs Grimoire {} or newer.", p.min_grimoire_version));
        }
    }
    if p.platforms.is_empty() {
        return Err("`platforms` is an empty list (omit it to mean all devices)".into());
    }
    for name in &p.platforms {
        if !widget_platform_known(name) {
            let all: Vec<&str> = WIDGET_PLATFORMS.iter().map(|(k, _)| *k).collect();
            return Err(format!("Unknown platform '{name}'. Supported: {}", all.join(", ")));
        }
    }
    if !p.platforms.iter().any(|x| x == "all") && !widget_platform_supported(&p.platforms) {
        let current = widget_current_platform();
        let label = if current == "unknown" { "this device".to_string() } else { widget_platform_label(current).to_string() };
        return Err(format!("This widget does not support your device ({label})."));
    }
    for c in &p.capabilities {
        if !cap_ok(c) {
            return Err(format!("Unknown capability '{c}'."));
        }
    }
    // (a widget is a frame its module draws in: one that could draw nothing is not installed)
    if !p.capabilities.iter().any(|c| c == WIDGET_CAP_OVERLAY) {
        return Err(format!("`capabilities` must declare `{WIDGET_CAP_OVERLAY}`; without it nothing a widget draws is shown"));
    }
    if p.spec.runtime != "wasm" {
        return Err("`spec.runtime` must be `wasm`; Grimoire does not run native widget code".into());
    }
    if !artifact_path_ok(&p.artifact.path) {
        return Err("`artifact.path` is not a safe relative path (it cannot be package.yaml or inside state/)".into());
    }
    if !sha_ok(&p.artifact.sha256) {
        return Err("`artifact.sha256` is not a SHA-256 digest".into());
    }
    if p.artifact.size == 0 || p.artifact.size > WIDGET_PKG_ARTIFACT_MAX {
        return Err("`artifact.size` is outside the allowed size".into());
    }
    for r in &p.releases {
        if !semver_ok(&r.version) {
            return Err("a `releases` entry has an invalid version".into());
        }
        if !(r.url.starts_with("https://") || r.url.starts_with("http://")) {
            return Err("a `releases` entry has no http or https URL".into());
        }
        if !sha_ok(&r.sha256) {
            return Err("a `releases` entry has an invalid sha256".into());
        }
    }
    // (the smallest frame is the layout's: a title bar, the margins and a body, widgets.rs)
    let (floor_w, floor_h) = crate::widgets::widget_floor();
    let (floor_w, floor_h) = (floor_w.max(1), floor_h.max(1));
    let ceil_w = (canvas_w * 70 / 100).max(floor_w);
    let ceil_h = (canvas_h * 70 / 100).max(floor_h);
    let s = &p.spec.size;
    for (w, h) in [s.default, s.min, s.max] {
        if w < floor_w || h < floor_h {
            return Err(format!("Widget size {w}x{h} is below the minimum of {floor_w}x{floor_h}"));
        }
        if w > ceil_w || h > ceil_h {
            return Err(format!("Widget size {w}x{h} exceeds the maximum of {ceil_w}x{ceil_h}"));
        }
    }
    if s.mode == WidgetSizeMode::Resizable && (s.min.0 > s.max.0 || s.min.1 > s.max.1) {
        return Err("`spec.size.min` is larger than `spec.size.max`".into());
    }
    Ok(())
}

/// The artifact a repository's manifest names for this version: its release entry when there is one,
/// else the repository tree's `artifact.path`.
pub fn widget_package_artifact_url(manifest_url: &str, p: &WidgetPackage) -> Option<String> {
    if let Some(r) = p.releases.iter().find(|r| r.version == p.version) {
        return Some(r.url.clone());
    }
    let base = manifest_url.rsplit_once('/').map_or("", |(b, _)| b);
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/{}", p.artifact.path))
}

/// The expected digest and size of the artifact for this version (a release can carry them separately).
pub fn widget_package_artifact_digest(p: &WidgetPackage) -> (&str, u64) {
    if let Some(r) = p.releases.iter().find(|r| r.version == p.version) {
        if sha_ok(&r.sha256) {
            return (&r.sha256, p.artifact.size);
        }
    }
    (&p.artifact.sha256, p.artifact.size)
}

// ---- installed state ----

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WidgetInstall {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    /// the manifest's spec.icon: a name in widgets::widget_icon, empty for a package without one
    pub icon: String,
    pub path: String,
    pub sha: String,
    pub installed_at: i64,
    pub source_url: String,
}

fn root() -> String {
    store_root()
}

pub fn widget_package_dir(id: &str) -> String {
    format!("{}/{}/{}", root(), WIDGET_PKG_DIR, id)
}

pub fn widget_package_index_path() -> String {
    format!("{}/{}", root(), WIDGET_PKG_INDEX)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn json_escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

pub fn widget_package_index_save(items: &[WidgetInstall]) -> i32 {
    if root().is_empty() {
        return 0;
    }
    let mut s = String::from("{\n  \"version\": 1,\n  \"packages\": [");
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str("\n    {\"id\":");
        json_escape(&it.id, &mut s);
        s.push_str(",\"name\":");
        json_escape(&it.name, &mut s);
        s.push_str(",\"version\":");
        json_escape(&it.version, &mut s);
        s.push_str(",\"author\":");
        json_escape(&it.author, &mut s);
        s.push_str(",\"description\":");
        json_escape(&it.description, &mut s);
        s.push_str(",\"icon\":");
        json_escape(&it.icon, &mut s);
        s.push_str(",\"path\":");
        json_escape(&it.path, &mut s);
        s.push_str(",\"sha\":");
        json_escape(&it.sha, &mut s);
        s.push_str(&format!(",\"installed_at\":{},\"source_url\":", it.installed_at));
        json_escape(&it.source_url, &mut s);
        s.push('}');
    }
    s.push_str("\n  ]\n}\n");
    mkdir_p(&format!("{}/packages", root()));
    write_file_atomic(&widget_package_index_path(), s.as_bytes())
}

/// The bytes the index's reader keeps of one string of an entry (`json_string` keeps one less than the room it
/// is given). The manifest check holds a package's name, version, author and description to it, so what an
/// install writes into the index is what is read back.
pub const WIDGET_PKG_TEXT_MAX: usize = 1023;

fn json_get_str(text: &[u8], toks: &[JsonTok], obj: i32, key: &str) -> String {
    let v = json_get(text, toks, obj, key);
    if v < 0 {
        return String::new();
    }
    let mut b = Vec::new();
    if json_string(text, &toks[v as usize], &mut b, WIDGET_PKG_TEXT_MAX + 1) {
        String::from_utf8_lossy(&b).into_owned()
    } else {
        String::new()
    }
}

pub fn widget_package_index_load() -> Vec<WidgetInstall> {
    let Some(data) = read_file_max(&widget_package_index_path(), 256 * 1024) else {
        return Vec::new();
    };
    let mut toks = vec![JsonTok::default(); 4096];
    let n = json_parse(&data, &mut toks);
    if n < 0 {
        qlog!("packages: index.json did not parse");
        return Vec::new();
    }
    let arr = json_get(&data, &toks[..n as usize], 0, "packages");
    if arr < 0 || toks[arr as usize].type_ != JsonType::Array {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..toks[arr as usize].size.min(WIDGET_PKG_COUNT_MAX as i32) {
        let it = json_item(&toks[..n as usize], arr, i);
        if it < 0 || toks[it as usize].type_ != JsonType::Object {
            continue;
        }
        let id = json_get_str(&data, &toks[..n as usize], it, "id");
        if !widget_id_ok(&id) {
            continue;
        }
        out.push(WidgetInstall {
            name: json_get_str(&data, &toks[..n as usize], it, "name"),
            id,
            version: json_get_str(&data, &toks[..n as usize], it, "version"),
            author: json_get_str(&data, &toks[..n as usize], it, "author"),
            description: json_get_str(&data, &toks[..n as usize], it, "description"),
            icon: json_get_str(&data, &toks[..n as usize], it, "icon"),
            path: json_get_str(&data, &toks[..n as usize], it, "path"),
            sha: json_get_str(&data, &toks[..n as usize], it, "sha"),
            installed_at: {
                let v = json_get(&data, &toks[..n as usize], it, "installed_at");
                let mut n2 = 0i32;
                if v >= 0 && json_int(&data, &toks[v as usize], &mut n2) { n2 as i64 } else { 0 }
            },
            source_url: json_get_str(&data, &toks[..n as usize], it, "source_url"),
        });
    }
    out
}

pub fn widget_package_installed(id: &str) -> Option<WidgetInstall> {
    widget_package_index_load().into_iter().find(|p| p.id == id)
}

/// An installed package's manifest, re-read from its folder (its author and license are not all in
/// the index, and an install from before those fields were kept has only in the manifest).
pub fn widget_package_installed_manifest(id: &str) -> Option<WidgetPackage> {
    parse_installed(&widget_package_installed(id)?)
}

fn parse_installed(p: &WidgetInstall) -> Option<WidgetPackage> {
    let path = widget_package_dir(&p.id);
    let data = read_file_max(&format!("{path}/package.yaml"), WIDGET_PKG_MANIFEST_MAX)?;
    let text = String::from_utf8_lossy(&data);
    let m = widget_package_parse(&text).ok()?;
    (m.id == p.id).then_some(m)
}

/// An installed widget package's manifest and its module's bytes, for a load (widgetwasm.rs). The file is checked
/// again here, each time a frame loads it (`package_artifact`), and it is read only if every check holds. What
/// failed goes to the log; the frame says that the artifact could not be read.
pub fn widget_package_artifact(id: &str) -> Result<(WidgetPackage, Vec<u8>), String> {
    let installed = || "The widget package is not installed.".to_string();
    let it = widget_package_installed(id).ok_or_else(installed)?;
    let m = parse_installed(&it).ok_or_else(installed)?;
    match package_artifact(&widget_package_dir(id), WIDGET_PKG_DIR, &it, &m) {
        Ok(bytes) => Ok((m, bytes)),
        Err(why) => {
            qlog!("widget %s: its module is not loaded: %s", id, why);
            Err("The widget's artifact could not be read.".to_string())
        }
    }
}

/// An installed package's module, for a load: a widget's (above) or an app's (apppkg.rs). `folder` is the
/// package's own folder, `packages` where the packages of its kind are kept (WIDGET_PKG_DIR, APP_PKG_DIR: the
/// index names a module from there), `it` its entry in the index and `m` its installed manifest. The file is
/// read only if every check holds:
///
/// - the installed manifest's `artifact.path` is one the manifest check takes (`artifact_path_ok`), and the
///   index names the same file (`<packages>/<id>/<artifact.path>`);
/// - it is a plain file inside the package's own folder, reached with no link on the way: the folder and each
///   part of the path opened with O_NOFOLLOW from the one before, on the folder's device (fsview's walk);
/// - it is no larger than an artifact may be, and its SHA-256 is the one the index recorded at the install.
///
/// One read and one SHA-256 of the file a load. An error is the check that failed, in words for the log.
pub(crate) fn package_artifact(folder: &str, packages: &str, it: &WidgetInstall, m: &WidgetPackage) -> Result<Vec<u8>, &'static str> {
    use std::io::Read;
    if !artifact_path_ok(&m.artifact.path) {
        return Err("the installed manifest's artifact.path is not a path inside the package's folder");
    }
    if it.path != format!("{packages}/{}/{}", it.id, m.artifact.path) {
        return Err("the index and the installed manifest name different files");
    }
    if !sha_ok(&it.sha) {
        return Err("the index holds no SHA-256 for it");
    }
    let file = crate::fsview::StoragePath::new(m.artifact.path.as_bytes())
        .and_then(|sp| crate::fsview::FsStorage::open(folder, &root()).ok()?.open_file(&sp, false).ok())
        .ok_or("it is not a plain file inside the package's folder, reached without a link")?;
    let mut file = std::fs::File::from(file);
    let len = file.metadata().map_err(|_| "it cannot be read")?.len();
    if len > WIDGET_PKG_ARTIFACT_MAX {
        return Err("it is larger than an artifact may be");
    }
    let mut bytes = Vec::new();
    if bytes.try_reserve_exact(len as usize).is_err() || file.by_ref().take(WIDGET_PKG_ARTIFACT_MAX + 1).read_to_end(&mut bytes).is_err() {
        return Err("it cannot be read");
    }
    if bytes.len() as u64 > WIDGET_PKG_ARTIFACT_MAX {
        return Err("it is larger than an artifact may be");
    }
    if !sha256_of(&bytes).eq_ignore_ascii_case(&it.sha) {
        return Err("its SHA-256 is not the one recorded when the package was installed");
    }
    Ok(bytes)
}

/// The installed packages whose manifest still reads and whose device is supported.
pub fn widget_package_installed_manifests() -> Vec<(WidgetInstall, WidgetPackage)> {
    let mut v = Vec::new();
    for it in widget_package_index_load() {
        let Some(m) = parse_installed(&it) else { continue };
        if m.platforms.is_empty() || !widget_platform_supported(&m.platforms) {
            continue;
        }
        v.push((it, m));
    }
    v
}

/// Installs a validated manifest and its artifact. Existing versions are replaced only after the new
/// directory is complete; the old one is kept as `.previous` for a rollback.
pub fn widget_package_install(manifest: &str, artifact: &[u8], source_url: &str, canvas_w: i32, canvas_h: i32) -> Result<WidgetInstall, String> {
    let p = widget_package_parse(manifest)?;
    widget_package_validate(&p, canvas_w, canvas_h)?;
    let (want_sha, want_size) = widget_package_artifact_digest(&p);
    if want_size != artifact.len() as u64 {
        return Err(format!("The artifact is {} bytes, not {want_size}.", artifact.len()));
    }
    let got = sha256_of(artifact);
    if !got.eq_ignore_ascii_case(want_sha) {
        return Err("The artifact's SHA-256 does not match the manifest.".into());
    }
    let index = widget_package_index_load();
    if index.len() >= WIDGET_PKG_COUNT_MAX && !index.iter().any(|x| x.id == p.id) {
        return Err("No more widget packages can be installed.".into());
    }
    let dir = widget_package_dir(&p.id);
    let stage = format!("{}/{}/.install-{}", root(), WIDGET_PKG_DIR, std::process::id());
    let _ = std::fs::remove_dir_all(&stage);
    if std::fs::create_dir_all(&stage).is_err() {
        return Err("Could not make the package's folder.".into());
    }
    if write_file_atomic(&format!("{stage}/package.yaml"), manifest.as_bytes()) != 0 {
        let _ = std::fs::remove_dir_all(&stage);
        return Err("Could not write the manifest.".into());
    }
    let dest = format!("{stage}/{}", p.artifact.path);
    if let Some(parent) = dest.rsplit_once('/').map(|(d, _)| d) {
        if std::fs::create_dir_all(parent).is_err() {
            let _ = std::fs::remove_dir_all(&stage);
            return Err("Could not make the artifact's folder.".into());
        }
    }
    if write_file_atomic(&dest, artifact) != 0 {
        let _ = std::fs::remove_dir_all(&stage);
        return Err("Could not write the artifact.".into());
    }
    let old = format!("{dir}{WIDGET_ID_PREVIOUS}");
    let _ = std::fs::remove_dir_all(&old);
    if std::fs::metadata(&dir).is_ok() && std::fs::rename(&dir, &old).is_err() {
        let _ = std::fs::remove_dir_all(&stage);
        return Err("Could not move the installed version aside.".into());
    }
    if let Err(e) = std::fs::rename(&stage, &dir) {
        if std::fs::metadata(&old).is_ok() {
            let _ = std::fs::rename(&old, &dir);
        }
        let _ = std::fs::remove_dir_all(&stage);
        return Err(format!("Could not put the package in place: {e}"));
    }
    // The package's own state survives an update; it is never part of the artifact.
    if std::fs::metadata(format!("{old}/state")).is_ok() {
        let _ = std::fs::rename(format!("{old}/state"), format!("{dir}/state"));
    }
    let it = WidgetInstall {
        id: p.id.clone(),
        name: p.name.clone(),
        version: p.version.clone(),
        author: p.author.clone(),
        description: p.description.clone(),
        icon: p.spec.icon.trim().to_string(),
        path: format!("{}/{}/{}", WIDGET_PKG_DIR, p.id, p.artifact.path),
        sha: got,
        installed_at: now_secs(),
        source_url: source_url.to_string(),
    };
    let mut next: Vec<WidgetInstall> = index.into_iter().filter(|x| x.id != it.id).collect();
    next.push(it.clone());
    if widget_package_index_save(&next) != 0 {
        // Put the previous version, and the state that came with it, back.
        if std::fs::metadata(format!("{dir}/state")).is_ok() {
            let _ = std::fs::rename(format!("{dir}/state"), format!("{old}/state"));
        }
        let _ = std::fs::remove_dir_all(&dir);
        if std::fs::metadata(&old).is_ok() {
            let _ = std::fs::rename(&old, &dir);
        }
        return Err("Could not save the package index.".into());
    }
    widget_catalog_invalidate();
    Ok(it)
}

/// Removes an installed package and its files. The `state` folder is kept when `keep_state` is set.
pub fn widget_package_remove(id: &str, keep_state: bool) -> Result<(), String> {
    if !widget_id_ok(id) {
        return Err("That id is not a widget package's.".into());
    }
    let dir = widget_package_dir(id);
    if keep_state {
        let state = format!("{dir}/state");
        if std::fs::metadata(&state).is_ok() {
            let keep = format!("{}/{}/.state-{}", root(), WIDGET_PKG_DIR, std::process::id());
            let _ = std::fs::remove_dir_all(&keep);
            std::fs::rename(&state, &keep).ok();
            let _ = std::fs::remove_dir_all(&dir);
            if std::fs::create_dir_all(&dir).is_ok() {
                let _ = std::fs::rename(&keep, &format!("{dir}/state"));
            }
        } else {
            let _ = std::fs::remove_dir_all(&dir);
        }
    } else {
        let _ = std::fs::remove_dir_all(&dir);
    }
    let mut index = widget_package_index_load();
    index.retain(|p| p.id != id);
    if widget_package_index_save(&index) != 0 {
        return Err("Could not save the package index.".into());
    }
    let mut frames = crate::widgets::widgets_load();
    frames.retain(|f| f.id != id);
    crate::widgets::widgets_save(&frames);
    widget_catalog_invalidate();
    Ok(())
}

/// Rolls an install back to the `.previous` copy.
pub fn widget_package_rollback(id: &str) -> Result<(), String> {
    if !widget_id_ok(id) {
        return Err("That id is not a widget package's.".into());
    }
    let dir = widget_package_dir(id);
    let old = format!("{dir}{WIDGET_ID_PREVIOUS}");
    if std::fs::metadata(&old).is_err() {
        return Err("There is no previous version to roll back to.".into());
    }
    let bad = format!("{dir}.bad-{}", std::process::id());
    let _ = std::fs::remove_dir_all(&bad);
    if std::fs::metadata(&dir).is_ok() && std::fs::rename(&dir, &bad).is_err() {
        return Err("Could not move the current version aside.".into());
    }
    if std::fs::rename(&old, &dir).is_err() {
        let _ = std::fs::rename(&bad, &dir);
        return Err("Could not restore the previous version.".into());
    }
    let _ = std::fs::remove_dir_all(&bad);
    let data = read_file_max(&format!("{dir}/package.yaml"), WIDGET_PKG_MANIFEST_MAX);
    let Some(data) = data else { return Err("The previous version has no manifest.".into()) };
    let p = widget_package_parse(&String::from_utf8_lossy(&data))?;
    let mut index = widget_package_index_load();
    for it in &mut index {
        if it.id == id {
            it.version = p.version.clone();
            it.name = p.name.clone();
            it.author = p.author.clone();
            it.description = p.description.clone();
        }
    }
    widget_package_index_save(&index);
    widget_catalog_invalidate();
    Ok(())
}

// ---- package settings ----

/// Where a package's setting values live. It is inside the package's state folder, so an update and
/// a remove that keeps state carry it along; the file is the host's, not the package's artifact.
pub fn widget_settings_path(id: &str) -> String {
    format!("{}/state/settings.json", widget_package_dir(id))
}

/// Every declared setting at its default. A package with no settings has an empty list.
pub fn widget_settings_defaults(schema: &[WidgetSetting]) -> Vec<(String, String)> {
    schema.iter().map(|s| (s.key.clone(), s.default.clone())).collect()
}

/// The value a tap produces: the other side of a toggle, or the next choice (the first for a value
/// the manifest no longer offers).
pub fn widget_setting_next(s: &WidgetSetting, current: &str) -> String {
    match s.kind {
        WidgetSettingKind::Toggle => (if current == "true" { "false" } else { "true" }).into(),
        WidgetSettingKind::Choice => {
            let at = s.choices.iter().position(|c| c == current);
            match at {
                Some(i) => s.choices[(i + 1) % s.choices.len()].clone(),
                None => s.choices.first().cloned().unwrap_or_default(),
            }
        }
    }
}

/// A package's saved values in its manifest's order. A missing file, a malformed value or a value
/// this version no longer offers reads as the default; an old key is left out.
pub fn widget_settings_load(id: &str, schema: &[WidgetSetting]) -> Vec<(String, String)> {
    let mut out = widget_settings_defaults(schema);
    if !widget_id_ok(id) {
        return out;
    }
    let Some(data) = read_file_max(&widget_settings_path(id), WIDGET_SETTINGS_FILE_MAX) else { return out };
    let mut toks = vec![JsonTok::default(); 2048];
    let n = json_parse(&data, &mut toks);
    if n < 0 {
        return out;
    }
    let values = json_get(&data, &toks[..n as usize], 0, "values");
    if values < 0 || toks[values as usize].type_ != JsonType::Object {
        return out;
    }
    for (i, s) in schema.iter().enumerate() {
        let tok = json_get(&data, &toks[..n as usize], values, &s.key);
        if tok < 0 {
            continue;
        }
        let mut buf = Vec::new();
        if !json_string(&data, &toks[tok as usize], &mut buf, WIDGET_SETTINGS_VALUE_MAX + 1) {
            continue;
        }
        let value = String::from_utf8_lossy(&buf);
        if setting_value_ok(s, &value) {
            out[i].1 = value.into_owned();
        }
    }
    out
}

/// Writes setting values atomically under the package's state folder. Keys that are not valid
/// setting keys are skipped, as are values longer than a choice can hold.
pub fn widget_settings_save(id: &str, values: &[(String, String)]) -> i32 {
    if !widget_id_ok(id) {
        return -1;
    }
    let mut body = String::from("{\n  \"version\": 1,\n  \"values\": {");
    let mut n = 0usize;
    for (key, value) in values {
        if !setting_key_ok(key) || value.len() > WIDGET_SETTINGS_VALUE_MAX || value.chars().any(char::is_control) {
            continue;
        }
        if n > 0 {
            body.push(',');
        }
        body.push_str("\n    ");
        json_escape(key, &mut body);
        body.push_str(": ");
        json_escape(value, &mut body);
        n += 1;
    }
    if n > 0 {
        body.push('\n');
    }
    body.push_str("  }\n}\n");
    let dir = format!("{}/state", widget_package_dir(id));
    if mkdir_p(&dir) != 0 {
        return -1;
    }
    write_file_atomic(&widget_settings_path(id), body.as_bytes())
}

/// Removes the saved values, so every setting reads its manifest default again. A missing file is
/// already reset.
pub fn widget_settings_clear(id: &str) -> i32 {
    if !widget_id_ok(id) {
        return -1;
    }
    match std::fs::remove_file(widget_settings_path(id)) {
        Ok(()) => 0,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(_) => -1,
    }
}

/// The value of one setting from a loaded list, or the fallback.
pub fn widget_setting_value<'a>(values: &'a [(String, String)], key: &str, fallback: &'a str) -> &'a str {
    values.iter().find(|(k, _)| k == key).map_or(fallback, |(_, v)| v)
}

// ---- fetching ----

/// One `curl` run, its answer read as it comes. The package manager has no TLS of its own; curl is
/// the repository fetcher (as KPM is Grimoire's update fetcher), and an install never waits on it on
/// the UI thread. The caller ticks it from the library tick.
pub struct WidgetFetch {
    pid: libc::pid_t,
    fd: i32,
    err_fd: i32,
    data: Vec<u8>,
    err: Vec<u8>,
    cap: usize,
    started: std::time::Instant,
    finished: bool,
}

/// A fetch may take this long; after it the child is killed. (The timeout is ours, not curl's:
/// some Kindles' curl rejects timeout options, and an install must not hang on one.)
const FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);

/// The redirects curl follows for one fetch; an answer that sends one more is a failed fetch.
const FETCH_REDIRECTS_MAX: u32 = 5;

/// Whether the fetcher takes an address: `https://`, at most WIDGET_PKG_URL_MAX bytes, and no space or
/// control character in it.
fn fetch_address_ok(url: &str) -> bool {
    url.len() <= WIDGET_PKG_URL_MAX && url.starts_with("https://") && !url.chars().any(|c| c.is_control() || c == ' ')
}

/// The arguments curl is started with for one fetch, after its own name. `-q` is first, so no
/// configuration file is read (one might name an option this build does not have), and `-fsSL` is older
/// than every curl on a Kindle or a computer. The address and every redirect are https (`--proto`,
/// `--proto-redir`), at most FETCH_REDIRECTS_MAX redirects are followed, and the address is taken as
/// written (`-g`: braces and brackets are part of the one address). The address is last.
/// No timeout option: the parent kills the child after FETCH_TIMEOUT instead, because some Kindles'
/// curl rejects the options real curl uses for one.
fn fetch_args(url: &str) -> Vec<String> {
    let redirects = FETCH_REDIRECTS_MAX.to_string();
    ["-q", "-fsSL", "-g", "--proto", "=https", "--proto-redir", "=https", "--max-redirs", &redirects, url].map(String::from).to_vec()
}

impl WidgetFetch {
    pub fn start(url: &str, cap: usize) -> Result<WidgetFetch, String> {
        if !fetch_address_ok(url) {
            return Err("Only https addresses can be fetched.".into());
        }
        let Ok(curl) = std::ffi::CString::new("curl") else { return Err("Could not start the fetcher.".into()) };
        // (the list is made here, before the fork: the child only sets up its descriptors and execs)
        let Ok(c_args) = fetch_args(url).into_iter().map(std::ffi::CString::new).collect::<Result<Vec<_>, _>>() else {
            return Err("That address has a NUL byte.".into());
        };
        let mut argv: Vec<*const libc::c_char> = Vec::with_capacity(c_args.len() + 2);
        argv.push(curl.as_ptr());
        argv.extend(c_args.iter().map(|arg| arg.as_ptr()));
        argv.push(std::ptr::null());
        let mut p = [0i32; 2];
        // SAFETY: pipe writes the two descriptors into our array.
        if unsafe { libc::pipe(p.as_mut_ptr()) } != 0 {
            return Err("Could not make the fetcher's pipe.".into());
        }
        let mut e = [0i32; 2];
        // SAFETY: as above, a second pipe for curl's own error text.
        if unsafe { libc::pipe(e.as_mut_ptr()) } != 0 {
            unsafe {
                libc::close(p[0]);
                libc::close(p[1]);
            }
            return Err("Could not make the fetcher's pipe.".into());
        }
        // SAFETY: fork has no preconditions; the child only sets up descriptors and execs, or _exits.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            // SAFETY: closing the descriptors the pipes just made.
            unsafe {
                libc::close(p[0]);
                libc::close(p[1]);
                libc::close(e[0]);
                libc::close(e[1]);
            }
            return Err("Could not start the fetcher.".into());
        }
        if pid == 0 {
            // SAFETY: in the child, dup2/close/fcntl on descriptors it owns, execvp with
            // NUL-terminated strings and a NULL-terminated list, _exit if it fails.
            unsafe {
                libc::dup2(p[1], 1);
                libc::dup2(e[1], 2);
                libc::close(p[0]);
                libc::close(p[1]);
                libc::close(e[0]);
                libc::close(e[1]);
                libc::fcntl(1, libc::F_SETFD, 0);
                libc::fcntl(2, libc::F_SETFD, 0);
                // (nothing else of Grimoire's: an inherited descriptor is no use to curl and can be
                // a terminal, a notebook or another child's pipe)
                for fd in 3..1024 {
                    libc::close(fd);
                }
                // curl with fetch_args' list. HTTPS is checked by start and held by the list for every
                // redirect (and the artifact's SHA-256 is checked by the install).
                libc::execvp(curl.as_ptr(), argv.as_ptr());
                libc::_exit(127);
            }
        }
        // SAFETY: closing the write ends the parent does not use; fcntl on the read ends we own.
        unsafe {
            libc::close(p[1]);
            libc::close(e[1]);
            libc::fcntl(p[0], libc::F_SETFL, libc::fcntl(p[0], libc::F_GETFL, 0) | libc::O_NONBLOCK);
            libc::fcntl(e[0], libc::F_SETFL, libc::fcntl(e[0], libc::F_GETFL, 0) | libc::O_NONBLOCK);
        }
        Ok(WidgetFetch { pid, fd: p[0], err_fd: e[0], data: Vec::with_capacity(cap.min(64 * 1024)), err: Vec::new(), cap, started: std::time::Instant::now(), finished: false })
    }

    fn read_into(fd: i32, out: &mut Vec<u8>, cap: usize) {
        let mut buf = [0u8; 16 * 1024];
        while out.len() < cap {
            let room = (cap - out.len()).min(buf.len());
            // SAFETY: read into our stack buffer, room bytes at most; the fd is ours.
            let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, room) };
            if n > 0 {
                out.extend_from_slice(&buf[..n as usize]);
            } else {
                break;
            }
        }
    }

    fn drain(&mut self) {
        Self::read_into(self.fd, &mut self.data, self.cap);
        // (curl's failure text; enough for its message, never the whole page it did not send)
        Self::read_into(self.err_fd, &mut self.err, 4096);
    }

    /// Kills and reaps the child after a failed fetch, then closes its pipes.
    fn stop(&mut self) {
        // SAFETY: signals and reaps the child this fetcher still owns, then closes its pipes.
        unsafe {
            libc::kill(self.pid, libc::SIGKILL);
            libc::waitpid(self.pid, std::ptr::null_mut(), 0);
            libc::close(self.fd);
            libc::close(self.err_fd);
        }
        self.finished = true;
    }

    fn err_text(&self) -> String {
        let text = String::from_utf8_lossy(&self.err);
        let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if one_line.chars().count() > 180 {
            one_line.chars().take(180).collect::<String>() + "..."
        } else {
            one_line
        }
    }

    /// None while curl runs, then its answer or the failure. The caller drops the fetcher after Some.
    pub fn tick(&mut self) -> Option<Result<Vec<u8>, String>> {
        if self.finished {
            return Some(Err("the fetch was already read".into()));
        }
        self.drain();
        let mut status = 0;
        // SAFETY: waitpid writes the status into our int.
        let reaped = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG) } == self.pid;
        self.drain();
        if !reaped {
            if self.data.len() >= self.cap {
                self.stop();
                return Some(Err("The answer is larger than the package manager accepts.".into()));
            }
            if self.started.elapsed() >= FETCH_TIMEOUT {
                self.stop();
                return Some(Err("The repository did not answer in time.".into()));
            }
            return None;
        }
        // SAFETY: closing the read ends this fetch owns.
        unsafe {
            libc::close(self.fd);
            libc::close(self.err_fd);
        }
        self.finished = true;
        let code = if libc::WIFEXITED(status) { libc::WEXITSTATUS(status) } else { -1 };
        if code != 0 {
            let detail = self.err_text();
            let base = if code == 28 {
                "The repository did not answer in time."
            } else if code == 127 {
                "The Kindle has no curl to fetch packages with."
            } else {
                "Could not fetch the repository"
            };
            return Some(Err(if detail.is_empty() { format!("{base} (curl exited {code}).") } else { format!("{base} (curl exited {code}): {detail}") }));
        }
        Some(Ok(std::mem::take(&mut self.data)))
    }
}

impl Drop for WidgetFetch {
    fn drop(&mut self) {
        if !self.finished {
            self.stop();
        }
    }
}

// ---- catalog entries ----

thread_local! {
    static PACKAGE_DEFS: RefCell<Option<Vec<&'static WidgetDef>>> = const { RefCell::new(None) };
}

pub fn widget_catalog_invalidate() {
    PACKAGE_DEFS.with(|d| *d.borrow_mut() = None);
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn package_def(p: &WidgetPackage) -> &'static WidgetDef {
    let size = &p.spec.size;
    let (min_w, min_h, max_w, max_h) = if size.mode == WidgetSizeMode::Resizable {
        (size.min.0 as f32, size.min.1 as f32, size.max.0 as f32, size.max.1 as f32)
    } else {
        let w = size.default.0 as f32;
        let h = size.default.1 as f32;
        (w, h, w, h)
    };
    // A package without an icon gets the generic widget grid.
    let icon = if p.spec.icon.trim().is_empty() {
        IconKind::Grid
    } else {
        crate::widgets::widget_icon(&p.spec.icon)
    };
    let def = WidgetDef {
        id: leak(p.id.clone()),
        name: leak(p.name.clone()),
        icon,
        default_w: size.default.0 as f32,
        default_h: size.default.1 as f32,
        min_w,
        min_h,
        max_w,
        max_h,
        resizable: size.mode == WidgetSizeMode::Resizable,
        version: leak(p.version.clone()),
        author: leak(p.author.clone()),
        description: leak(p.description.clone()),
    };
    Box::leak(Box::new(def))
}

pub fn widget_package_defs() -> Vec<&'static WidgetDef> {
    PACKAGE_DEFS.with(|d| {
        let mut d = d.borrow_mut();
        if d.is_none() {
            let mut v = Vec::new();
            for (_, p) in widget_package_installed_manifests() {
                v.push(package_def(&p));
            }
            *d = Some(v);
        }
        d.as_ref().unwrap().clone()
    })
}

/// Checks a repository input and returns the manifest URL: `user/repo` is GitHub's default branch,
/// else main then master; a custom URL must end in `.yaml`.
pub fn widget_manifest_url(input: &str) -> Result<(String, bool), String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Enter a repository or a .yaml address.".into());
    }
    if input.len() > WIDGET_PKG_URL_MAX {
        return Err("That address is too long.".into());
    }
    if input.starts_with("https://") || input.starts_with("http://") {
        let path = input.split(['?', '#']).next().unwrap_or(input);
        if !path.to_ascii_lowercase().ends_with(".yaml") {
            return Err("The address must end in .yaml.".into());
        }
        return Ok((input.to_string(), false));
    }
    if input.starts_with('/') {
        return Err("Enter user/repo, not a path.".into());
    }
    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() != 2 || parts.iter().any(|p| p.is_empty() || p.contains(' ')) {
        return Err("Enter user/repo or an https .yaml address.".into());
    }
    let user = parts[0];
    let repo = parts[1];
    if !user.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        || !repo.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
    {
        return Err("That GitHub name has a character it cannot have.".into());
    }
    Ok((format!("https://raw.githubusercontent.com/{user}/{repo}/main/package.yaml"), true))
}

/// A GitHub repository's `user/repo`, from a repository address, a `user/repo` shorthand or a raw
/// file URL, or None for anything else (another host, a local path).
pub fn widget_github_repo(source: &str) -> Option<String> {
    fn name_ok(s: &str) -> bool {
        !s.is_empty()
            && s.len() <= 100
            && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
            && s != "."
            && s != ".."
    }
    let s = source.trim();
    let rest = if let Some(r) = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
        .or_else(|| s.strip_prefix("https://raw.githubusercontent.com/"))
        .or_else(|| s.strip_prefix("http://raw.githubusercontent.com/"))
    {
        r
    } else if s.contains("://") || s.starts_with('/') {
        return None;
    } else {
        s
    };
    let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let repo = parts[1].strip_suffix(".git").unwrap_or(parts[1]);
    if !name_ok(parts[0]) || !name_ok(repo) {
        return None;
    }
    Some(format!("{}/{}", parts[0], repo))
}

/// The repository's GitHub page, for its QR code: only a GitHub repository has one.
pub fn widget_github_repo_url(source: &str) -> Option<String> {
    widget_github_repo(source).map(|r| format!("https://github.com/{r}"))
}

/// The GitHub repository API URL for a shorthand input, or None for a custom URL.
pub fn widget_github_api(input: &str) -> Option<String> {
    let parts: Vec<&str> = input.trim().split('/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !input.trim().starts_with("http") {
        Some(format!("https://api.github.com/repos/{}/{}", parts[0], parts[1]))
    } else {
        None
    }
}

/// The default branch named by GitHub's repository answer, if it has one.
pub fn widget_github_branch(json: &str) -> Option<String> {
    let key = "\"default_branch\"";
    let at = json.find(key)? + key.len();
    let rest = &json[at..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    let branch = &rest[..end];
    (!branch.is_empty() && branch.len() <= 128 && !branch.contains('/')).then(|| branch.to_string())
}

/// Every installed package's detail, for a UI list.
pub fn widget_package_list() -> Vec<WidgetInstall> {
    let mut v = widget_package_index_load();
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.id.cmp(&b.id)));
    v
}

/// Packages whose files are no longer there, removed from the index (a manual delete, a failed move).
pub fn widget_package_repair() {
    let mut index = widget_package_index_load();
    let before = index.len();
    index.retain(|p| parse_installed(p).is_some());
    if index.len() != before {
        widget_package_index_save(&index);
        widget_catalog_invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &str) -> String {
        format!(
            r#"name: Test Widget
id: {id}
type: widget
version: 1.2.0
author: Jane
description: A test.
license: MIT
min_grimoire_version: 2.0.0
platforms: [all]
artifact:
  path: dist/widget.bin
  sha256: {sha}
  size: 4
capabilities: [editor:overlay, storage:widget-local]
settings:
  - key: show_percent
    label: Show percent key
    type: toggle
    default: false
  - key: layout
    label: Layout
    type: choice
    default: large
    choices: [compact, large]
spec:
  runtime: wasm
  entry: _start
  minimizable: true
  size:
    mode: resizable
    default: [220, 160]
    min: [120, 120]
    max: [800, 600]
"#,
            sha = sha256_of(b"test")
        )
    }

    #[test]
    fn a_manifest_reads_and_validates() {
        ui_init(300.0);
        let p = widget_package_parse(&manifest("com.example.test")).unwrap();
        assert_eq!(p.name, "Test Widget");
        assert_eq!(p.platforms, ["all"]);
        assert_eq!(p.spec.size.default, (220, 160));
        assert_eq!(p.spec.size.mode, WidgetSizeMode::Resizable);
        widget_package_validate(&p, 1860, 2480).unwrap();
        // (a name that only begins as the state folder's or the manifest's does, and those names deeper in the path)
        for path in ["states/widget.bin", "state.bin", "package.yaml.bin", "dist/package.yaml", "dist/state/widget.bin", "Dist/Widget.BIN"] {
            let p = widget_package_parse(&manifest("com.example.test").replace("path: dist/widget.bin", &format!("path: {path}"))).unwrap();
            assert_eq!(widget_package_validate(&p, 1860, 2480), Ok(()), "{path}");
        }
        // (the capability to draw alone is a whole list: a widget that saves nothing)
        let p = widget_package_parse(&manifest("com.example.test").replace(", storage:widget-local]", "]")).unwrap();
        assert_eq!(p.capabilities, ["editor:overlay"]);
        widget_package_validate(&p, 1860, 2480).unwrap();
    }

    #[test]
    fn validation_names_the_offending_field() {
        ui_init(300.0);
        let good = manifest("com.example.test");
        for (bad, want) in [
            (good.replace("type: widget", "type: plugin"), "not a widget"),
            (good.replace("com.example.test", "Bad Id"), "lowercase"),
            // (the name an update gives the replaced version's folder is no package's)
            (good.replace("com.example.test", "com.example.test.previous"), "`id` cannot end in `.previous`"),
            (good.replace("com.example.test", "a.previous"), "`id` cannot end in `.previous`"),
            (good.replace("platforms: [all]", "platforms: [kindle-oasis]"), "Unknown platform"),
            (good.replace("capabilities: [editor:overlay, storage:widget-local]", "capabilities: [net]"), "Unknown capability"),
            (good.replace("capabilities: [editor:overlay, storage:widget-local]", "capabilities: [editor:overlay, net]"), "Unknown capability"),
            // (a widget draws in its frame: its manifest says so)
            (good.replace("capabilities: [editor:overlay, storage:widget-local]", "capabilities: []"), "`capabilities` must declare `editor:overlay`"),
            (good.replace("capabilities: [editor:overlay, storage:widget-local]", "capabilities: [storage:widget-local]"), "`capabilities` must declare `editor:overlay`"),
            (good.replace("capabilities: [editor:overlay, storage:widget-local]\n", ""), "`capabilities` must declare `editor:overlay`"),
            (good.replace("path: dist/widget.bin", "path: package.yaml"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: state/widget.bin"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: state"), "safe relative path"),
            // (the manifest and the state folder in any case of their letters: a storage that ignores case finds them)
            (good.replace("path: dist/widget.bin", "path: state/runtime.bin"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: STATE/runtime.bin"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: State/settings.json"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: Package.yaml"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: PACKAGE.YAML"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: sTaTe"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: ../widget.bin"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: /dist/widget.bin"), "safe relative path"),
            // (names the storage walk does not take: the module is read through it when a frame loads it)
            (good.replace("path: dist/widget.bin", "path: .dist/widget.bin"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: dist/.widget.bin"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: dist./widget.bin"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: widget.sdr/widget.bin"), "safe relative path"),
            (good.replace("path: dist/widget.bin", "path: system/widget.bin"), "safe relative path"),
            (good.replace("runtime: wasm", "runtime: native"), "must be `wasm`"),
        ] {
            let p = widget_package_parse(&bad).unwrap();
            let e = widget_package_validate(&p, 1860, 2480).unwrap_err();
            assert!(e.contains(want), "{e}");
        }
    }

    #[test]
    fn a_size_below_the_smallest_frame_is_refused() {
        ui_init(300.0);
        let good = manifest("com.example.test");
        // 120x120 is the smallest frame: a title bar, the margins and a body one line high.
        for (size, ok) in [("min: [120, 120]", true), ("min: [120, 119]", false), ("min: [119, 120]", false), ("min: [120, 80]", false)] {
            let p = widget_package_parse(&good.replace("min: [120, 120]", size)).unwrap();
            let got = widget_package_validate(&p, 1860, 2480);
            assert_eq!(got.is_ok(), ok, "{size}: {got:?}");
            if let Err(e) = got {
                assert!(e.contains("below the minimum of 120x120"), "{e}");
            }
        }
        for (size, ok) in [("default: [120, 120]", true), ("default: [220, 119]", false)] {
            let fixed = good.replace("mode: resizable", "mode: fixed").replace("default: [220, 160]", size);
            let p = widget_package_parse(&fixed).unwrap();
            assert_eq!(widget_package_validate(&p, 1860, 2480).is_ok(), ok, "{size}");
        }
    }

    #[test]
    fn a_package_declares_its_settings() {
        let p = widget_package_parse(&manifest("com.example.test")).unwrap();
        assert_eq!(p.settings.len(), 2);
        assert_eq!(p.settings[0].kind, WidgetSettingKind::Toggle);
        assert_eq!(p.settings[0].default, "false");
        assert_eq!(p.settings[1].choices, ["compact", "large"]);
        assert_eq!(p.settings[1].default, "large");
        assert_eq!(widget_setting_next(&p.settings[0], "false"), "true");
        assert_eq!(widget_setting_next(&p.settings[1], "large"), "compact");
        assert_eq!(widget_setting_next(&p.settings[1], "gone"), "compact");

        let good = manifest("com.example.test");
        for (bad, want) in [
            (good.replace("type: toggle", "type: switch"), "`toggle` or `choice`"),
            (good.replace("key: show_percent", "key: Show Percent"), "settings[0].key"),
            (good.replace("key: layout", "key: show_percent"), "used twice"),
            (good.replace("default: large", "default: wide"), "not one of its choices"),
            (good.replace("choices: [compact, large]", "choices: [compact]"), "needs 2"),
        ] {
            let e = widget_package_parse(&bad).unwrap_err();
            assert!(e.contains(want), "{e}");
        }
    }

    #[test]
    fn settings_load_save_and_fall_back_to_defaults() {
        ui_init(300.0);
        let root = std::env::temp_dir().join(format!("grimoire-widget-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let m = manifest("com.example.test");
        let it = widget_package_install(&m, b"test", "https://example.org/p.yaml", 1860, 2480).unwrap();
        let schema = widget_package_installed_manifest(&it.id).unwrap().settings;
        let defaults = widget_settings_defaults(&schema);
        assert_eq!(widget_settings_load(&it.id, &schema), defaults);
        assert_eq!(widget_settings_save(&it.id, &defaults), 0);
        assert_eq!(widget_settings_load(&it.id, &schema), defaults);

        let changed = vec![
            ("show_percent".to_string(), "true".to_string()),
            ("layout".to_string(), "compact".to_string()),
        ];
        assert_eq!(widget_settings_save(&it.id, &changed), 0);
        assert_eq!(widget_settings_load(&it.id, &schema), changed);
        assert_eq!(widget_setting_value(&changed, "layout", "large"), "compact");
        assert_eq!(widget_setting_value(&changed, "missing", "fallback"), "fallback");

        // The state folder carries the values through an update and a remove that keeps state.
        let m2 = m.replace("version: 1.2.0", "version: 1.3.0");
        widget_package_install(&m2, b"test", "https://example.org/p.yaml", 1860, 2480).unwrap();
        assert_eq!(widget_settings_load(&it.id, &schema), changed);
        widget_package_remove(&it.id, true).unwrap();
        widget_package_install(&m2, b"test", "https://example.org/p.yaml", 1860, 2480).unwrap();
        assert_eq!(widget_settings_load(&it.id, &schema), changed);
        assert_eq!(widget_settings_clear(&it.id), 0);
        assert_eq!(widget_settings_load(&it.id, &schema), defaults);

        // A hand-edited value the manifest no longer offers reads as its default.
        std::fs::write(
            widget_settings_path(&it.id),
            br#"{"version":1,"values":{"show_percent":"maybe","layout":"wide","old":"x"}}"#,
        )
        .unwrap();
        assert_eq!(widget_settings_load(&it.id, &schema), defaults);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn repository_inputs_resolve_to_yaml() {
        let (u, short) = widget_manifest_url("jane/widgets").unwrap();
        assert!(short);
        assert_eq!(u, "https://raw.githubusercontent.com/jane/widgets/main/package.yaml");
        let (u, short) = widget_manifest_url("https://example.org/w.yaml").unwrap();
        assert!(!short);
        assert_eq!(u, "https://example.org/w.yaml");
        assert!(widget_manifest_url("https://example.org/w.zip").is_err());
        assert!(widget_manifest_url("jane").is_err());
        assert_eq!(widget_github_branch(r#"{"default_branch":"trunk"}"#).as_deref(), Some("trunk"));
    }

    #[test]
    fn only_github_addresses_name_a_repository() {
        for (source, want) in [
            ("jane/widgets", Some("jane/widgets")),
            ("https://github.com/jane/widgets", Some("jane/widgets")),
            ("https://github.com/jane/widgets.git", Some("jane/widgets")),
            ("https://github.com/jane/widgets/tree/main", Some("jane/widgets")),
            ("https://raw.githubusercontent.com/jane/widgets/main/package.yaml", Some("jane/widgets")),
            ("https://example.org/widgets", None),
            ("https://example.org/w.yaml", None),
            ("/home/me/widgets", None),
            ("jane", None),
        ] {
            assert_eq!(widget_github_repo(source).as_deref(), want, "{source}");
        }
        assert_eq!(widget_github_repo_url("jane/widgets").as_deref(), Some("https://github.com/jane/widgets"));
        assert!(widget_github_repo_url("https://example.org/widgets").is_none());
    }

    // The whole list curl is started with: -q first, -fsSL, each rule's option, and the address last.
    #[test]
    fn curl_is_started_with_the_rules_of_a_fetch_and_the_address_last() {
        let url = "https://example.org/w/package.yaml?a=[1-3]&b={x,y}";
        assert_eq!(fetch_args(url), ["-q", "-fsSL", "-g", "--proto", "=https", "--proto-redir", "=https", "--max-redirs", "5", url]);
        assert_eq!(FETCH_REDIRECTS_MAX, 5);
    }

    // The fetcher starts curl with that list: a stand-in named curl, first on PATH for the start,
    // answers with the arguments it was given, one a line.
    #[test]
    fn the_fetcher_starts_curl_with_that_list() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("grimoire-widget-fetch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let stand_in = dir.join("curl");
        std::fs::write(&stand_in, "#!/bin/sh\nfor arg in \"$@\"; do printf '%s\\n' \"$arg\"; done\n").unwrap();
        std::fs::set_permissions(&stand_in, std::fs::Permissions::from_mode(0o755)).unwrap();
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut first = std::ffi::OsString::from(&dir);
        first.push(":");
        first.push(&path);
        let url = "https://example.org/w/package.yaml?a=[1-3]&b={x,y}";
        std::env::set_var("PATH", &first);
        let started = WidgetFetch::start(url, 4096);
        std::env::set_var("PATH", &path);
        let mut fetch = started.unwrap_or_else(|e| panic!("{e}"));
        let answer = loop {
            if let Some(answer) = fetch.tick() {
                break answer;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        let answer = String::from_utf8(answer.unwrap()).unwrap();
        assert_eq!(answer.lines().collect::<Vec<_>>(), fetch_args(url));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // An address is https, no longer than WIDGET_PKG_URL_MAX, and has no space or control character:
    // any other is refused with the one sentence before a fetcher is started.
    #[test]
    fn only_a_plain_https_address_is_fetched() {
        let longest = format!("https://example.org/{}", "a".repeat(WIDGET_PKG_URL_MAX - 20));
        assert_eq!(longest.len(), WIDGET_PKG_URL_MAX);
        for url in ["https://example.org/package.yaml", "https://example.org/w/package.yaml?a=[1-3]&b={x,y}#c", "https://example.org/%20/p.yaml", longest.as_str()] {
            assert!(fetch_address_ok(url), "{url}");
        }
        let too_long = format!("{longest}a");
        for url in [
            "",
            "example.org/package.yaml",
            "http://example.org/package.yaml",
            "ftp://example.org/package.yaml",
            "file:///mnt/us/package.yaml",
            "HTTPS://example.org/package.yaml",
            " https://example.org/package.yaml",
            "https://example.org/my package.yaml",
            "https://example.org/package.yaml ",
            "https://example.org/a\tb.yaml",
            "https://example.org/a\nb.yaml",
            "https://example.org/a\r\nb.yaml",
            "https://example.org/a\0b.yaml",
            "https://example.org/a\u{7f}b.yaml",
            "https://example.org/a\u{85}b.yaml",
            too_long.as_str(),
        ] {
            assert!(!fetch_address_ok(url), "{url:?}");
            assert_eq!(WidgetFetch::start(url, 1024).err().as_deref(), Some("Only https addresses can be fetched."), "{url:?}");
        }
    }

    #[test]
    fn a_name_a_version_an_author_and_a_description_are_no_longer_than_the_index_keeps() {
        ui_init(300.0);
        assert_eq!(WIDGET_PKG_TEXT_MAX, 1023);
        let good = manifest("com.example.test");
        // (words, `n` bytes of them, the last a letter)
        let long = |n: usize| {
            let mut s = "word ".repeat((n - 1) / 5);
            s.push_str(&"x".repeat(n - s.len()));
            s
        };
        // (a version, `n` bytes of it: three numbers and a prerelease of letters)
        let version = |n: usize| format!("1.0.0-{}", "x".repeat(n - 6));
        assert_eq!(long(1023).len(), 1023);
        assert_eq!(version(1023).len(), 1023);
        for (line, field) in [("name: Test Widget", "name"), ("version: 1.2.0", "version"), ("author: Jane", "author"), ("description: A test.", "description")] {
            let text = |n: usize| if field == "version" { version(n) } else { long(n) };
            let at = widget_package_parse(&good.replace(line, &format!("{field}: {}", text(1023)))).unwrap();
            assert_eq!(widget_package_validate(&at, 1860, 2480), Ok(()), "{field} of 1,023 bytes");
            let past = widget_package_parse(&good.replace(line, &format!("{field}: {}", text(1024)))).unwrap();
            assert_eq!(widget_package_validate(&past, 1860, 2480), Err(format!("`{field}` is longer than 1023 bytes")), "{field} of 1,024 bytes");
            let far = widget_package_parse(&good.replace(line, &format!("{field}: {}", text(200_000)))).unwrap();
            assert_eq!(widget_package_validate(&far, 1860, 2480), Err(format!("`{field}` is longer than 1023 bytes")), "{field} of 200,000 bytes");
        }
        // What an install writes into the index is what is read back: the four at their longest, whole.
        let root = std::env::temp_dir().join(format!("grimoire-widgetpkg-texts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let m = good
            .replace("name: Test Widget", &format!("name: {}", long(1023)))
            .replace("version: 1.2.0", &format!("version: {}", version(1023)))
            .replace("author: Jane", &format!("author: {}", long(1023)))
            .replace("description: A test.", &format!("description: {}", long(1023)));
        widget_package_install(&m, b"test", "", 1860, 2480).unwrap();
        let read = widget_package_index_load();
        assert_eq!(read.len(), 1);
        assert_eq!((read[0].name.as_str(), read[0].author.as_str(), read[0].description.as_str()), (long(1023).as_str(), long(1023).as_str(), long(1023).as_str()));
        assert_eq!(read[0].version, version(1023));
        // (and a manifest with a longer one leaves the index as it was)
        let e = widget_package_install(&m.replace("id: com.example.test", "id: com.example.other").replace(&format!("description: {}", long(1023)), &format!("description: {}", long(255_000))), b"test", "", 1860, 2480).unwrap_err();
        assert_eq!(e, "`description` is longer than 1023 bytes");
        assert_eq!(widget_package_index_load(), read);
        widget_catalog_invalidate();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn no_id_ends_as_the_folder_of_a_replaced_version_does() {
        ui_init(300.0);
        assert_eq!(WIDGET_ID_PREVIOUS, ".previous");
        for (id, ok) in [
            ("com.example.test", true),
            ("com.example.test.previous", false),
            ("a.previous", false),
            ("x-1.previous", false),
            // (only the end of an id: these name no replaced version's folder)
            ("previous", true),
            ("com.example.previous.test", true),
            ("com.example.previousx", true),
            ("com.example.test-previous", true),
            ("com.example.test.previous.previous", false),
        ] {
            assert_eq!(widget_id_ok(id), ok, "{id}");
        }
        let root = std::env::temp_dir().join(format!("grimoire-widgetpkg-previous-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        // A package, and one whose id is that id with `.previous` after it: the second is not installed.
        let it = widget_package_install(&manifest("com.example.test"), b"test", "", 1860, 2480).unwrap();
        let e = widget_package_install(&manifest("com.example.test.previous"), b"test", "", 1860, 2480).unwrap_err();
        assert!(e.contains("`id` cannot end in `.previous`"), "{e}");
        assert!(!root.join("packages/widgets/com.example.test.previous").exists());
        // An update of the first keeps the replaced version in that folder, which is nobody else's; the index and
        // widgets.ini read no such id, whoever wrote it there.
        let state = format!("{}/state", widget_package_dir(&it.id));
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(format!("{state}/session"), b"keep").unwrap();
        widget_package_install(&manifest("com.example.test").replace("version: 1.2.0", "version: 1.3.0"), b"test", "", 1860, 2480).unwrap();
        let kept = std::fs::read_to_string(root.join("packages/widgets/com.example.test.previous/package.yaml")).unwrap();
        assert!(kept.contains("version: 1.2.0") && kept.contains("id: com.example.test\n"));
        assert_eq!(std::fs::read(format!("{state}/session")).unwrap(), b"keep");
        let index = std::fs::read_to_string(widget_package_index_path()).unwrap();
        std::fs::write(widget_package_index_path(), index.replace("\"id\":\"com.example.test\"", "\"id\":\"com.example.test.previous\"")).unwrap();
        assert!(widget_package_index_load().is_empty(), "an index entry with such an id is skipped");
        assert!(widget_package_remove("com.example.test.previous", false).is_err());
        assert!(root.join("packages/widgets/com.example.test.previous").exists(), "and removing by that id touches nothing");
        std::fs::write(root.join("widgets.ini"), "com.example.test.previous=1,2,300,300,0,1,0\ncom.example.test=5,6,300,300,0,2,0\n").unwrap();
        let lines = crate::widgets::widgets_load();
        assert_eq!(lines.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(), ["com.example.test"]);
        widget_catalog_invalidate();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_package_installs_replaces_and_removes() {
        ui_init(300.0);
        let root = std::env::temp_dir().join(format!("grimoire-widgetpkg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let m = manifest("com.example.test");
        let it = widget_package_install(&m, b"test", "https://example.org/p.yaml", 1860, 2480).unwrap();
        assert_eq!(it.version, "1.2.0");
        assert!(std::fs::metadata(format!("{}/package.yaml", widget_package_dir(&it.id))).is_ok());
        assert_eq!(widget_package_list().len(), 1);
        assert_eq!(widget_package_defs().len(), 1);
        let state = format!("{}/state", widget_package_dir(&it.id));
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(format!("{state}/session"), b"keep").unwrap();
        let m2 = m.replace("version: 1.2.0", "version: 1.3.0");
        let it = widget_package_install(&m2, b"test", "https://example.org/p.yaml", 1860, 2480).unwrap();
        assert_eq!(it.version, "1.3.0");
        assert_eq!(widget_package_list().len(), 1);
        assert_eq!(widget_def(&it.id).unwrap().icon, IconKind::Grid);
        assert_eq!(std::fs::read(format!("{state}/session")).unwrap(), b"keep");
        widget_package_remove(&it.id, true).unwrap();
        assert!(widget_package_list().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
