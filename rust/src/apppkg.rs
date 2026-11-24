//! App packages (2.3): manifests, installs, the apps index and app settings.
//!
//! Apps are a package kind of their own, beside widgets: a `package.yaml` with `type: app` and a
//! WebAssembly artifact under `packages/apps/<id>/`, listed in `packages/apps-index.json`. The
//! manifest parser and the package manager page are shared with widgets; only the folder, index,
//! validation rules and the runtime differ. The full-screen runtime lives in apphost.rs.
//!
//! An app package may use Grimoire's global UI and may request `storage:app-local` and
//! `network:http`. Its state folder survives updates and a remove that keeps state; the artifact
//! never includes it.

use crate::prelude::*;
use crate::sha256::sha256_of;

pub const APP_PKG_INDEX: &str = "packages/apps-index.json";
pub const APP_PKG_DIR: &str = "packages/apps";
pub const APP_PKG_COUNT_MAX: usize = 64;
/// A bundled app icon is one small `.ico`; its pixels are decoded by the safe loader's picture worker (pkgpics.rs,
/// issue #236), never here or in appicon.rs.
pub const APP_ICON_MAX: usize = 256 * 1024;
const APP_SETTINGS_FILE_MAX: usize = 64 * 1024;
const APP_SETTINGS_VALUE_MAX: usize = 64;

fn root() -> String {
    store_root()
}

pub fn app_package_dir(id: &str) -> String {
    format!("{}/{}/{id}", root(), APP_PKG_DIR)
}

pub fn app_package_index_path() -> String {
    format!("{}/{APP_PKG_INDEX}", root())
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && id.as_bytes()[0] != b'.'
        && id.as_bytes()[id.len() - 1] != b'.'
        && !id.contains("..")
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

fn cap_ok(c: &str) -> bool {
    matches!(c, "storage:app-local" | "network:http")
}

/// Whether `spec.icon` names a bundled file rather than one of Grimoire's built-in icon names.
pub fn app_icon_is_file(name: &str) -> bool {
    let name = name.trim();
    name.len() > 4 && name.to_ascii_lowercase().ends_with(".ico")
}

/// Where a repository's bundled `.ico` lives: beside its manifest, like the artifact.
pub fn app_package_icon_url(manifest_url: &str, p: &WidgetPackage) -> Option<String> {
    if !app_icon_is_file(&p.spec.icon) {
        return None;
    }
    let base = manifest_url.rsplit_once('/').map_or("", |(b, _)| b);
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/{}", p.spec.icon.trim()))
}

/// An installed app's `.ico`, or None when its manifest uses a built-in icon name.
pub fn app_package_icon_path(it: &WidgetInstall) -> Option<String> {
    let icon = it.icon.trim();
    if !app_icon_is_file(icon) || !path_ok(icon) || icon == "package.yaml" || icon.starts_with("state/") {
        return None;
    }
    Some(format!("{}/{icon}", app_package_dir(&it.id)))
}

/// The picture the worker made of an installed app's bundled icon goes from its cache (pkgpics.rs) as the app is
/// removed or replaced. The icon's file is looked at (one stat), never read.
fn forget_icon_picture(it: &WidgetInstall) {
    use crate::pkgpics::{picture_forget, PicForm, PicSrc};
    if let Some(src) = app_package_icon_path(it).and_then(|path| PicSrc::of(&path, PicForm::AppIcon)) {
        picture_forget(&src);
    }
}

/// Validates an app manifest for this build and device. `canvas_w`/`canvas_h` are accepted for the
/// package manager's shared call shape; an app owns the whole screen and has no size in its spec.
pub fn app_package_validate(p: &WidgetPackage, _canvas_w: i32, _canvas_h: i32) -> Result<(), String> {
    if p.type_ != "app" {
        return Err("This repository is not an app package".into());
    }
    if !id_ok(&p.id) {
        return Err("`id` must be lowercase letters, digits, dots and hyphens".into());
    }
    if p.name.trim().is_empty() {
        return Err("`name` is missing".into());
    }
    if !semver_ok(&p.version) {
        return Err(format!("`version` (`{}`) is not a valid version", p.version));
    }
    if !p.min_grimoire_version.is_empty() {
        if !update_version_key(&p.min_grimoire_version, &mut [0; 3]) {
            return Err("`min_grimoire_version` is not a valid version".into());
        }
        if update_version_cmp(GRIMOIRE_VERSION, &p.min_grimoire_version) < 0 {
            return Err(format!("This app needs Grimoire {} or newer.", p.min_grimoire_version));
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
        return Err(format!("This app does not support your device ({label})."));
    }
    for c in &p.capabilities {
        if !cap_ok(c) {
            return Err(format!("Unknown app capability '{c}'."));
        }
    }
    if p.spec.runtime != "wasm" {
        return Err("`spec.runtime` must be `wasm`; Grimoire does not run native app code".into());
    }
    let icon = p.spec.icon.trim();
    if app_icon_is_file(icon) {
        if !path_ok(icon)
            || icon == "package.yaml"
            || icon == "state"
            || icon.starts_with("state/")
            || icon == p.artifact.path
        {
            return Err("`spec.icon` is not a safe relative path for the app's icon".into());
        }
        if !sha_ok(&p.spec.icon_sha256) {
            return Err("`spec.icon_sha256` is not a SHA-256 digest".into());
        }
        if p.spec.icon_size == 0 || p.spec.icon_size > APP_ICON_MAX as u64 {
            return Err("`spec.icon_size` is outside the allowed size".into());
        }
    } else if !p.spec.icon_sha256.trim().is_empty() || p.spec.icon_size != 0 {
        return Err("`spec.icon_sha256` and `spec.icon_size` are only for a `.ico` icon".into());
    }
    if !path_ok(&p.artifact.path)
        || p.artifact.path == "package.yaml"
        || p.artifact.path == "state"
        || p.artifact.path.starts_with("state/")
    {
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
    Ok(())
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

pub fn app_package_index_save(items: &[WidgetInstall]) -> i32 {
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
    write_file_atomic(&app_package_index_path(), s.as_bytes())
}

fn json_get_str(text: &[u8], toks: &[JsonTok], obj: i32, key: &str) -> String {
    let v = json_get(text, toks, obj, key);
    if v < 0 {
        return String::new();
    }
    let mut b = Vec::new();
    if json_string(text, &toks[v as usize], &mut b, 1024) {
        String::from_utf8_lossy(&b).into_owned()
    } else {
        String::new()
    }
}

pub fn app_package_index_load() -> Vec<WidgetInstall> {
    let Some(data) = read_file_max(&app_package_index_path(), 256 * 1024) else {
        return Vec::new();
    };
    let mut toks = vec![JsonTok::default(); 4096];
    let n = json_parse(&data, &mut toks);
    if n < 0 {
        qlog!("apps: apps-index.json did not parse");
        return Vec::new();
    }
    let toks = &toks[..n as usize];
    let arr = json_get(&data, toks, 0, "packages");
    if arr < 0 || toks[arr as usize].type_ != JsonType::Array {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..toks[arr as usize].size.min(APP_PKG_COUNT_MAX as i32) {
        let it = json_item(toks, arr, i);
        if it < 0 || toks[it as usize].type_ != JsonType::Object {
            continue;
        }
        let id = json_get_str(&data, toks, it, "id");
        if !id_ok(&id) {
            continue;
        }
        out.push(WidgetInstall {
            name: json_get_str(&data, toks, it, "name"),
            id,
            version: json_get_str(&data, toks, it, "version"),
            author: json_get_str(&data, toks, it, "author"),
            description: json_get_str(&data, toks, it, "description"),
            icon: json_get_str(&data, toks, it, "icon"),
            path: json_get_str(&data, toks, it, "path"),
            sha: json_get_str(&data, toks, it, "sha"),
            installed_at: {
                let v = json_get(&data, toks, it, "installed_at");
                let mut n2 = 0i32;
                if v >= 0 && json_int(&data, &toks[v as usize], &mut n2) { n2 as i64 } else { 0 }
            },
            source_url: json_get_str(&data, toks, it, "source_url"),
        });
    }
    out
}

pub fn app_package_installed(id: &str) -> Option<WidgetInstall> {
    app_package_index_load().into_iter().find(|p| p.id == id)
}

fn parse_installed(p: &WidgetInstall) -> Option<WidgetPackage> {
    let path = app_package_dir(&p.id);
    let data = read_file_max(&format!("{path}/package.yaml"), WIDGET_PKG_MANIFEST_MAX)?;
    let text = String::from_utf8_lossy(&data);
    let m = widget_package_parse(&text).ok()?;
    (m.id == p.id && m.type_ == "app").then_some(m)
}

pub fn app_package_installed_manifest(id: &str) -> Option<WidgetPackage> {
    parse_installed(&app_package_installed(id)?)
}

pub fn app_package_list() -> Vec<WidgetInstall> {
    let mut v = app_package_index_load();
    v.retain(|p| parse_installed(p).is_some());
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.id.cmp(&b.id)));
    v
}

/// Installs a validated app manifest and artifact. Existing versions are swapped in whole; the
/// app's state is carried over and the old directory is kept as `.previous`.
pub fn app_package_install(manifest: &str, artifact: &[u8], icon: Option<&[u8]>, source_url: &str, canvas_w: i32, canvas_h: i32) -> Result<WidgetInstall, String> {
    let p = widget_package_parse(manifest)?;
    app_package_validate(&p, canvas_w, canvas_h)?;
    let (want_sha, want_size) = widget_package_artifact_digest(&p);
    if want_size != artifact.len() as u64 {
        return Err(format!("The artifact is {} bytes, not {want_size}.", artifact.len()));
    }
    let got = sha256_of(artifact);
    if !got.eq_ignore_ascii_case(want_sha) {
        return Err("The artifact's SHA-256 does not match the manifest.".into());
    }
    let icon_bytes = if app_icon_is_file(&p.spec.icon) {
        let bytes = icon.ok_or("The app's icon was not downloaded.")?;
        if bytes.len() as u64 != p.spec.icon_size {
            return Err(format!("The icon is {} bytes, not {}.", bytes.len(), p.spec.icon_size));
        }
        if !sha256_of(bytes).eq_ignore_ascii_case(&p.spec.icon_sha256) {
            return Err("The icon's SHA-256 does not match the manifest.".into());
        }
        Some(bytes)
    } else {
        None
    };
    let index = app_package_index_load();
    if index.len() >= APP_PKG_COUNT_MAX && !index.iter().any(|x| x.id == p.id) {
        return Err("No more app packages can be installed.".into());
    }
    let dir = app_package_dir(&p.id);
    let stage = format!("{}/{}/.install-{}", root(), APP_PKG_DIR, std::process::id());
    let _ = std::fs::remove_dir_all(&stage);
    if std::fs::create_dir_all(&stage).is_err() {
        return Err("Could not make the app's folder.".into());
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
    if let Some(bytes) = icon_bytes {
        let icon_dest = format!("{stage}/{}", p.spec.icon.trim());
        if let Some(parent) = icon_dest.rsplit_once('/').map(|(d, _)| d) {
            if std::fs::create_dir_all(parent).is_err() {
                let _ = std::fs::remove_dir_all(&stage);
                return Err("Could not make the icon's folder.".into());
            }
        }
        if write_file_atomic(&icon_dest, bytes) != 0 {
            let _ = std::fs::remove_dir_all(&stage);
            return Err("Could not write the app's icon.".into());
        }
    }
    let old = format!("{dir}.previous");
    let _ = std::fs::remove_dir_all(&old);
    if let Some(was) = index.iter().find(|x| x.id == p.id) {
        forget_icon_picture(was);
    }
    if std::fs::metadata(&dir).is_ok() && std::fs::rename(&dir, &old).is_err() {
        let _ = std::fs::remove_dir_all(&stage);
        return Err("Could not move the installed version aside.".into());
    }
    if let Err(e) = std::fs::rename(&stage, &dir) {
        if std::fs::metadata(&old).is_ok() {
            let _ = std::fs::rename(&old, &dir);
        }
        let _ = std::fs::remove_dir_all(&stage);
        return Err(format!("Could not put the app in place: {e}"));
    }
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
        path: format!("{}/{}/{}", APP_PKG_DIR, p.id, p.artifact.path),
        sha: got,
        installed_at: now_secs(),
        source_url: source_url.to_string(),
    };
    let mut next: Vec<WidgetInstall> = index.into_iter().filter(|x| x.id != it.id).collect();
    next.push(it.clone());
    if app_package_index_save(&next) != 0 {
        if std::fs::metadata(format!("{dir}/state")).is_ok() {
            let _ = std::fs::rename(format!("{dir}/state"), format!("{old}/state"));
        }
        let _ = std::fs::remove_dir_all(&dir);
        if std::fs::metadata(&old).is_ok() {
            let _ = std::fs::rename(&old, &dir);
        }
        return Err("Could not save the app index.".into());
    }
    Ok(it)
}

/// Removes an installed app and its files. Its `state` folder is kept when `keep_state` is set.
pub fn app_package_remove(id: &str, keep_state: bool) -> Result<(), String> {
    if !id_ok(id) {
        return Err("That id is not an app package's.".into());
    }
    let dir = app_package_dir(id);
    if let Some(was) = app_package_installed(id) {
        forget_icon_picture(&was);
    }
    if keep_state {
        let state = format!("{dir}/state");
        if std::fs::metadata(&state).is_ok() {
            let keep = format!("{}/{}/.state-{}", root(), APP_PKG_DIR, std::process::id());
            let _ = std::fs::remove_dir_all(&keep);
            std::fs::rename(&state, &keep).ok();
            let _ = std::fs::remove_dir_all(&dir);
            if std::fs::create_dir_all(&dir).is_ok() {
                let _ = std::fs::rename(&keep, format!("{dir}/state"));
            }
        } else {
            let _ = std::fs::remove_dir_all(&dir);
        }
    } else {
        let _ = std::fs::remove_dir_all(&dir);
    }
    let mut index = app_package_index_load();
    index.retain(|p| p.id != id);
    if app_package_index_save(&index) != 0 {
        return Err("Could not save the app index.".into());
    }
    Ok(())
}

pub fn app_settings_path(id: &str) -> String {
    format!("{}/state/settings.json", app_package_dir(id))
}

pub fn app_settings_defaults(schema: &[WidgetSetting]) -> Vec<(String, String)> {
    schema.iter().map(|s| (s.key.clone(), s.default.clone())).collect()
}

fn setting_ok(s: &WidgetSetting, value: &str) -> bool {
    match s.kind {
        WidgetSettingKind::Toggle => value == "true" || value == "false",
        WidgetSettingKind::Choice => s.choices.iter().any(|c| c == value),
    }
}

pub fn app_settings_load(id: &str, schema: &[WidgetSetting]) -> Vec<(String, String)> {
    let mut out = app_settings_defaults(schema);
    if !id_ok(id) {
        return out;
    }
    let Some(data) = read_file_max(&app_settings_path(id), APP_SETTINGS_FILE_MAX) else { return out };
    let mut toks = vec![JsonTok::default(); 2048];
    let n = json_parse(&data, &mut toks);
    if n < 0 {
        return out;
    }
    let toks = &toks[..n as usize];
    let values = json_get(&data, toks, 0, "values");
    if values < 0 || toks[values as usize].type_ != JsonType::Object {
        return out;
    }
    for (i, s) in schema.iter().enumerate() {
        let tok = json_get(&data, toks, values, &s.key);
        if tok < 0 {
            continue;
        }
        let mut buf = Vec::new();
        if !json_string(&data, &toks[tok as usize], &mut buf, APP_SETTINGS_VALUE_MAX + 1) {
            continue;
        }
        let value = String::from_utf8_lossy(&buf);
        if setting_ok(s, &value) {
            out[i].1 = value.into_owned();
        }
    }
    out
}

pub fn app_settings_save(id: &str, values: &[(String, String)]) -> i32 {
    if !id_ok(id) {
        return -1;
    }
    let mut body = String::from("{\n  \"version\": 1,\n  \"values\": {");
    let mut n = 0usize;
    for (key, value) in values {
        if key.is_empty() || value.len() > APP_SETTINGS_VALUE_MAX || value.chars().any(char::is_control) {
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
    let dir = format!("{}/state", app_package_dir(id));
    if mkdir_p(&dir) != 0 {
        return -1;
    }
    write_file_atomic(&app_settings_path(id), body.as_bytes())
}

pub fn app_settings_clear(id: &str) -> i32 {
    if !id_ok(id) {
        return -1;
    }
    match std::fs::remove_file(app_settings_path(id)) {
        Ok(()) => 0,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(_) => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> String {
        r#"name: Test App
id: test.app
type: app
version: 1.0.0
author: Tester
description: An app
platforms: [all]
artifact:
  path: app.wasm
  sha256: 0000000000000000000000000000000000000000000000000000000000000000
  size: 8
spec:
  runtime: wasm
  status_bar: false
capabilities: [storage:app-local, network:http]
settings:
  - key: color
    label: Colour
    type: choice
    choices: [Ink, Grey]
    default: Ink
"#.into()
    }

    #[test]
    fn an_app_manifest_is_an_app() {
        let p = widget_package_parse(&manifest()).unwrap();
        assert_eq!(p.type_, "app");
        assert!(!p.spec.status_bar);
        assert!(app_package_validate(&p, 1000, 1000).is_ok());
        let widget = manifest().replace("type: app", "type: widget");
        let p = widget_package_parse(&widget).unwrap();
        assert!(app_package_validate(&p, 1000, 1000).is_err());
    }

    #[test]
    fn install_update_remove_and_settings_keep_the_app_state_folder() {
        ui_init(300.0);
        let root = std::env::temp_dir().join(format!("grimoire-apppkg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let artifact = b"fixture app bytes";
        let sha = sha256_of(artifact);
        let m = manifest()
            .replace("0000000000000000000000000000000000000000000000000000000000000000", &sha)
            .replace("size: 8", &format!("size: {}", artifact.len()));
        let it = app_package_install(&m, artifact, None, "https://example.org/app.yaml", 1860, 2480).unwrap();
        assert_eq!(it.id, "test.app");
        assert_eq!(app_package_list().len(), 1);
        assert_eq!(app_package_installed_manifest(&it.id).unwrap().name, "Test App");
        let schema = app_package_installed_manifest(&it.id).unwrap().settings;
        let changed = vec![("color".to_string(), "Grey".to_string())];
        assert_eq!(app_settings_save(&it.id, &changed), 0);
        assert_eq!(app_settings_load(&it.id, &schema), changed);

        let m2 = m.replace("version: 1.0.0", "version: 1.1.0");
        assert_eq!(app_package_install(&m2, artifact, None, "https://example.org/app.yaml", 1860, 2480).unwrap().version, "1.1.0");
        assert_eq!(app_settings_load(&it.id, &schema), changed);
        app_package_remove(&it.id, true).unwrap();
        assert!(app_package_list().is_empty());
        assert!(std::path::Path::new(&app_settings_path(&it.id)).exists());
        app_settings_clear(&it.id);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_bundled_icon_is_checked_and_written() {
        ui_init(300.0);
        let root = std::env::temp_dir().join(format!("grimoire-appicon-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let artifact = b"fixture app bytes";
        let icon = b"fixture icon bytes";
        let sha = sha256_of(artifact);
        let icon_sha = sha256_of(icon);
        let m = manifest()
            .replace("0000000000000000000000000000000000000000000000000000000000000000", &sha)
            .replace("size: 8", &format!("size: {}", artifact.len()))
            .replace(
                "  status_bar: false",
                &format!("  icon: icon.ico\n  icon_sha256: {icon_sha}\n  icon_size: {}\n  status_bar: false", icon.len()),
            );
        let it = app_package_install(&m, artifact, Some(icon), "https://example.org/app.yaml", 1860, 2480).unwrap();
        let path = app_package_icon_path(&it).expect("the icon path is kept");
        assert_eq!(std::fs::read(path).unwrap(), icon);
        let err = app_package_install(&m, artifact, None, "https://example.org/app.yaml", 1860, 2480).unwrap_err();
        assert!(err.contains("icon"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
