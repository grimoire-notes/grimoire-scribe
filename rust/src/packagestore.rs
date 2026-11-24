//! Package Store: the built-in catalog for Grimoire app and widget packages (2.3).
//!
//! The store is a GitHub-hosted `index.json` plus package folders. The catalog is cached
//! under `package-store/`, so the last downloaded store still opens offline. Screenshots
//! and store icons are cached the same way. Installing reuses the package manager's
//! manifest validation and installers; the manifest and artifact are checked by those
//! before anything is written.

use crate::globalui::{gui_button, gui_chip, gui_gap, gui_header, gui_icon_button, gui_margin, gui_radius};
use crate::prelude::*;
use crate::sha256::sha256_of;

/// The catalog this build publishes. A test or a mirror can point `GRIMOIRE_PACKAGE_STORE_URL`
/// at another `index.json`.
pub const DEFAULT_CATALOG_URL: &str = "https://raw.githubusercontent.com/grimoire-notes/package-store-contents/main/index.json";

const H_CLOSE: i32 = 801;
const H_BACK: i32 = 802;
const H_REFRESH: i32 = 803;
const H_FILTER: i32 = 804;
const H_CARD: i32 = 805;
const H_PREV: i32 = 806;
const H_NEXT: i32 = 807;
const H_INSTALL: i32 = 808;
const H_TRUST: i32 = 809;
const H_QR: i32 = 810;
const H_SHOT_PREV: i32 = 811;
const H_SHOT_NEXT: i32 = 812;
const H_RETRY: i32 = 813;

const INDEX_MAX: usize = 512 * 1024;
const INDEX_TOKENS: usize = 32 * 1024;
const PACKAGE_MAX: usize = 200;
const SCREENSHOT_MAX: usize = 8 * 1024 * 1024;
const INSTALL_ICON_MAX: usize = crate::apppkg::APP_ICON_MAX;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StoreView {
    #[default]
    List,
    Detail,
    Trust,
    Busy,
    Error,
    Qr,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct StoreAsset {
    url: String,
    /// Safe path under `package-store/assets/`, or a digest for an absolute URL.
    key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum StoreIconRef {
    Glyph(IconKind),
    Image(StoreAsset),
}

impl Default for StoreIconRef {
    fn default() -> Self {
        Self::Glyph(IconKind::Package)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorePackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: String,
    pub author: String,
    pub category: String,
    pub summary: String,
    pub description: String,
    pub license: String,
    pub repository: String,
    pub homepage: String,
    pub manifest_url: String,
    icon: StoreIconRef,
    screenshots: Vec<StoreAsset>,
    pub tags: Vec<String>,
    pub added: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoreIndex {
    pub name: String,
    pub tagline: String,
    pub updated: String,
    pub featured: String,
    pub packages: Vec<StorePackage>,
}

/// One installed package the catalog has a newer version of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageUpdate {
    pub package: StorePackage,
    pub installed_version: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum FetchStage {
    #[default]
    Catalog,
    Manifest,
    Artifact,
    Icon,
    Screenshot,
    InstallIcon,
    ListIcon,
}

struct FetchJob {
    stage: FetchStage,
    fetch: WidgetFetch,
    url: String,
    key: String,
}

#[derive(Default)]
struct StoreIcon {
    glyph: IconKind,
    image: Canvas,
}

impl StoreIcon {
    fn reset(&mut self) {
        canvas_free(&mut self.image);
        self.glyph = IconKind::Package;
    }

    fn draw(&self, c: &mut Canvas, r: Rect, ink: u8) {
        if !self.image.px.is_empty() {
            canvas_blit_fit(c, &self.image, r);
        } else {
            ui_icon(c, self.glyph, r, ink);
        }
    }
}

/// The screen's state, kept on `App` so the Library tick can poll its downloads.
#[derive(Default)]
pub struct PackageStoreUi {
    pub open: bool,
    pub view: StoreView,
    pub index: StoreIndex,
    pub filter: i32,
    page: i32,
    pub selected: Option<String>,
    pub hits: HitList,
    press: Press,
    pressed: Option<Rect>,
    pressed_hit: Option<Hit>,
    pub status: String,
    pub error: String,
    index_url: String,
    catalog_job: Option<FetchJob>,
    install_job: Option<FetchJob>,
    asset_job: Option<FetchJob>,
    manifest: Option<WidgetPackage>,
    manifest_url: String,
    manifest_yaml: String,
    artifact_url: String,
    icon_url: Option<String>,
    install_icon_key: String,
    pending_artifact: Vec<u8>,
    icon: StoreIcon,
    icon_for: String,
    icon_tried: bool,
    list_icons: Vec<Canvas>,
    list_tried: Vec<bool>,
    shot: Canvas,
    shot_for: String,
    shot_tried: bool,
    shot_index: i32,
    pub refreshing: bool,
    /// The General > Updates page uses this catalog, its icons and its install machinery without
    /// opening the Store screen.
    pub updates_center: bool,
    pub update_installing: bool,
    pub update_status: String,
    pub update_error: String,
    pub update_target: String,
}

impl PackageStoreUi {
    fn selected_package(&self) -> Option<&StorePackage> {
        let id = self.selected.as_deref()?;
        self.index.packages.iter().find(|p| p.id == id)
    }

    fn clear_selection(&mut self) {
        self.selected = None;
        self.manifest = None;
        self.manifest_url.clear();
        self.manifest_yaml.clear();
        self.artifact_url.clear();
        self.icon_url = None;
        self.install_icon_key.clear();
        self.pending_artifact.clear();
        self.icon.reset();
        self.icon_for.clear();
        self.icon_tried = false;
        canvas_free(&mut self.shot);
        self.shot_for.clear();
        self.shot_tried = false;
        self.shot_index = 0;
    }

    fn cancel_jobs(&mut self) {
        self.catalog_job = None;
        self.install_job = None;
        self.asset_job = None;
        self.refreshing = false;
    }

    fn reset_list_icons(&mut self) {
        for icon in &mut self.list_icons {
            canvas_free(icon);
        }
        self.list_icons = vec![Canvas::default(); self.index.packages.len()];
        self.list_tried = vec![false; self.index.packages.len()];
    }
}

fn catalog_url() -> String {
    std::env::var("GRIMOIRE_PACKAGE_STORE_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s.starts_with("https://") && s.len() <= WIDGET_PKG_URL_MAX)
        .unwrap_or_else(|| DEFAULT_CATALOG_URL.to_string())
}

fn offline() -> bool {
    std::env::var_os("GRIMOIRE_PACKAGE_STORE_OFFLINE").is_some_and(|v| !v.is_empty())
}

fn store_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && !id.starts_with('.')
        && !id.ends_with('.')
        && !id.contains("..")
}

fn image_name(s: &str) -> bool {
    let s = s.trim().to_ascii_lowercase();
    s.ends_with(".ico") || s.ends_with(".png") || s.ends_with(".jpg") || s.ends_with(".jpeg")
}

fn need_asset(s: &str) -> bool {
    s.contains('/') || image_name(s)
}

fn safe_key(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty()
        || s.len() > 512
        || s.starts_with('/')
        || s.contains('\\')
        || s.contains('?')
        || s.contains('#')
        || s.split('/').any(|p| p.is_empty() || p == "." || p == "..")
    {
        return None;
    }
    Some(s.to_string())
}

fn cache_key(value: &str) -> Option<String> {
    if let Some(key) = safe_key(value) {
        return Some(key);
    }
    if value.starts_with("https://") && value.len() <= WIDGET_PKG_URL_MAX {
        return Some(format!("url-{}", sha256_of(value.as_bytes())));
    }
    None
}

fn resolve_url(base: &str, value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > WIDGET_PKG_URL_MAX {
        return None;
    }
    if value.starts_with("https://") {
        return Some(value.to_string());
    }
    if value.starts_with("http://") || value.starts_with('/') || value.starts_with("//") {
        return None;
    }
    let (dir, _) = base.rsplit_once('/')?;
    Some(format!("{dir}/{value}"))
}

fn asset(base: &str, value: &str) -> Option<StoreAsset> {
    let url = resolve_url(base, value)?;
    let key = cache_key(value)?;
    Some(StoreAsset { url, key })
}

fn cache_root() -> String {
    format!("{}/package-store", store_root())
}

fn catalog_cache_path() -> String {
    format!("{}/index.json", cache_root())
}

fn asset_cache_path(key: &str) -> String {
    format!("{}/assets/{key}", cache_root())
}

fn read_cached_asset(key: &str, cap: usize) -> Option<Vec<u8>> {
    if safe_key(key).as_deref() != Some(key) {
        return None;
    }
    read_file_max(&asset_cache_path(key), cap)
}

fn write_cached_asset(key: &str, data: &[u8]) {
    let path = asset_cache_path(key);
    if let Some(parent) = path.rsplit_once('/').map(|(p, _)| p) {
        let _ = mkdir_p(parent);
    }
    let _ = write_file_atomic(&path, data);
}

fn write_cached_index(data: &[u8]) {
    let path = catalog_cache_path();
    if let Some(parent) = path.rsplit_once('/').map(|(p, _)| p) {
        let _ = mkdir_p(parent);
    }
    let _ = write_file_atomic(&path, data);
}

// ---- catalog parsing ----

fn jstr(text: &[u8], toks: &[JsonTok], obj: i32, key: &str, cap: usize) -> String {
    let v = json_get(text, toks, obj, key);
    if v < 0 {
        return String::new();
    }
    let mut b = Vec::new();
    if json_string(text, &toks[v as usize], &mut b, cap) {
        String::from_utf8_lossy(&b).trim().to_string()
    } else {
        String::new()
    }
}

fn jlist(text: &[u8], toks: &[JsonTok], obj: i32, key: &str, max: usize) -> Vec<String> {
    let arr = json_get(text, toks, obj, key);
    if arr < 0 || toks[arr as usize].type_ != JsonType::Array {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..toks[arr as usize].size.min(max as i32) {
        let it = json_item(toks, arr, i);
        if it < 0 {
            continue;
        }
        let mut b = Vec::new();
        if json_string(text, &toks[it as usize], &mut b, 1024) {
            let s = String::from_utf8_lossy(&b).trim().to_string();
            if !s.is_empty() {
                out.push(s);
            }
        }
    }
    out
}

fn icon_ref(base: &str, value: &str) -> StoreIconRef {
    let value = value.trim();
    if need_asset(value) {
        if let Some(a) = asset(base, value) {
            return StoreIconRef::Image(a);
        }
    }
    StoreIconRef::Glyph(if value.is_empty() { IconKind::Package } else { widget_icon(value) })
}

/// Parses a catalog from bytes, with every relative path resolved against `base` (the
/// catalog's own URL).
pub fn store_index_parse(data: &[u8], base: &str) -> Result<StoreIndex, String> {
    if data.len() > INDEX_MAX {
        return Err("The Store catalog is too large.".into());
    }
    let mut toks = vec![JsonTok::default(); INDEX_TOKENS];
    let n = json_parse(data, &mut toks);
    if n < 0 {
        return Err("The Store catalog is not valid JSON.".into());
    }
    let toks = &toks[..n as usize];
    if toks[0].type_ != JsonType::Object {
        return Err("The Store catalog is not an object.".into());
    }
    let version = json_get(data, toks, 0, "version");
    let mut v = 0i32;
    if version < 0 || !json_int(data, &toks[version as usize], &mut v) || v != 1 {
        return Err("This Store catalog needs a newer Grimoire.".into());
    }
    let arr = json_get(data, toks, 0, "packages");
    if arr < 0 || toks[arr as usize].type_ != JsonType::Array {
        return Err("The Store catalog has no packages list.".into());
    }
    let mut index = StoreIndex {
        name: jstr(data, toks, 0, "name", 160),
        tagline: jstr(data, toks, 0, "tagline", 512),
        updated: jstr(data, toks, 0, "updated", 64),
        featured: jstr(data, toks, 0, "featured", 128),
        ..Default::default()
    };
    if index.name.is_empty() {
        index.name = "Package Store".into();
    }
    let count = toks[arr as usize].size.min(PACKAGE_MAX as i32);
    for i in 0..count {
        let it = json_item(toks, arr, i);
        if it < 0 || toks[it as usize].type_ != JsonType::Object {
            continue;
        }
        let id = jstr(data, toks, it, "id", 128);
        let name = jstr(data, toks, it, "name", 160);
        let version = jstr(data, toks, it, "version", 64);
        let kind = jstr(data, toks, it, "type", 16);
        if !store_id_ok(&id) || name.is_empty() || version.is_empty() || !matches!(kind.as_str(), "app" | "widget") {
            continue;
        }
        if index.packages.iter().any(|p| p.id == id) {
            continue;
        }
        let manifest = jstr(data, toks, it, "manifest", 1024);
        let Some(manifest_url) = resolve_url(base, &manifest) else { continue };
        let icon = icon_ref(base, &jstr(data, toks, it, "icon", 1024));
        let mut screenshots = Vec::new();
        for s in jlist(data, toks, it, "screenshots", 12) {
            if let Some(a) = asset(base, &s) {
                screenshots.push(a);
            }
        }
        index.packages.push(StorePackage {
            id,
            name,
            version,
            kind,
            author: jstr(data, toks, it, "author", 160),
            category: jstr(data, toks, it, "category", 80),
            summary: jstr(data, toks, it, "summary", 512),
            description: jstr(data, toks, it, "description", 8192),
            license: jstr(data, toks, it, "license", 160),
            repository: jstr(data, toks, it, "repository", 1024),
            homepage: jstr(data, toks, it, "homepage", 1024),
            manifest_url,
            icon,
            screenshots,
            tags: jlist(data, toks, it, "tags", 12),
            added: jstr(data, toks, it, "added", 64),
        });
    }
    if index.packages.is_empty() {
        return Err("The Store catalog has no packages that this Grimoire can show.".into());
    }
    Ok(index)
}

// ---- opening and fetching ----

fn start_catalog(ui: &mut PackageStoreUi) {
    if ui.catalog_job.is_some() {
        return;
    }
    if offline() {
        if ui.index.packages.is_empty() {
            ui.error = tr("No saved catalog. Connect to the Internet, then tap Refresh.").to_string();
            ui.view = StoreView::Error;
        } else {
            ui.status = tr("Showing the saved catalog.").to_string();
        }
        return;
    }
    match WidgetFetch::start(&ui.index_url, INDEX_MAX) {
        Ok(fetch) => {
            ui.catalog_job = Some(FetchJob {
                stage: FetchStage::Catalog,
                fetch,
                url: ui.index_url.clone(),
                key: String::new(),
            });
            ui.refreshing = true;
            if ui.index.packages.is_empty() {
                ui.view = StoreView::Busy;
                ui.status = tr("Reading the Store...").to_string();
            } else {
                ui.status = tr("Refreshing the Store...").to_string();
            }
        }
        Err(e) => {
            ui.error = e;
            if ui.index.packages.is_empty() {
                ui.view = StoreView::Error;
            } else {
                ui.status = tr("Couldn't refresh the Store.").to_string();
            }
        }
    }
}

fn load_cached_index(ui: &mut PackageStoreUi) -> bool {
    let Some(data) = read_file_max(&catalog_cache_path(), INDEX_MAX) else {
        return false;
    };
    match store_index_parse(&data, &ui.index_url) {
        Ok(index) => {
            ui.index = index;
            true
        }
        Err(e) => {
            qlog!("package store: saved catalog refused: %s", &e);
            false
        }
    }
}

pub fn store_open(a: &mut App) {
    let ui = &mut a.store;
    ui.open = true;
    ui.view = StoreView::List;
    ui.filter = 0;
    ui.page = 0;
    ui.status.clear();
    ui.error.clear();
    ui.index = StoreIndex::default();
    ui.selected = None;
    ui.cancel_jobs();
    ui.clear_selection();
    ui.reset_list_icons();
    ui.index_url = catalog_url();
    let cached = load_cached_index(ui);
    if cached {
        ui.reset_list_icons();
        ui.status = tr("Showing the saved catalog.").to_string();
    }
    start_catalog(ui);
    if !cached && ui.index.packages.is_empty() && ui.view != StoreView::Error {
        ui.view = StoreView::Busy;
        ui.status = tr("Reading the Store...").to_string();
    }
}

/// Opens the package half of Settings > Updates. It uses the Store's cached catalog, icons and
/// install path, but never puts the Store screen in front of the Library.
pub fn updates_open(a: &mut App) {
    let ui = &mut a.store;
    ui.updates_center = true;
    ui.update_installing = false;
    ui.update_target.clear();
    ui.update_status.clear();
    ui.update_error.clear();
    ui.selected = None;
    ui.cancel_jobs();
    ui.clear_selection();
    ui.reset_list_icons();
    ui.index = StoreIndex::default();
    ui.status.clear();
    ui.error.clear();
    ui.index_url = catalog_url();
    let cached = load_cached_index(ui);
    if cached {
        ui.reset_list_icons();
        ui.status = tr("Showing the saved catalog.").to_string();
    }
    start_catalog(ui);
    if !cached && ui.index.packages.is_empty() && ui.view != StoreView::Error {
        ui.view = StoreView::Busy;
        ui.status = tr("Reading the Store...").to_string();
    }
}

/// Settings > Updates is leaving: nothing keeps downloading behind it. The catalog and its icons
/// stay cached for the next visit.
pub fn updates_close(a: &mut App) {
    let ui = &mut a.store;
    if !ui.updates_center {
        return;
    }
    ui.updates_center = false;
    ui.cancel_jobs();
    ui.selected = None;
    ui.clear_selection();
    ui.update_installing = false;
    ui.update_target.clear();
    ui.update_status.clear();
    ui.update_error.clear();
}

pub fn updates_tick(a: &mut App, _now: u32) {
    if !a.store.updates_center {
        return;
    }
    let mut changed = tick_catalog(a);
    changed |= tick_install(a);
    changed |= tick_asset(a);
    changed |= start_list_icons(a);
    if changed {
        crate::library::store_redraw(a);
    }
}

pub fn updates_refresh(a: &mut App) {
    if !a.store.updates_center {
        return;
    }
    a.store.error.clear();
    start_catalog(&mut a.store);
    crate::library::store_redraw(a);
}

/// Every installed package whose catalog listing is newer, by name.
pub fn package_updates(ui: &PackageStoreUi) -> Vec<PackageUpdate> {
    let mut out = Vec::new();
    for p in &ui.index.packages {
        let Some(it) = installed_for(p) else { continue };
        if update_version_cmp(&p.version, &it.version) <= 0 {
            continue;
        }
        out.push(PackageUpdate {
            package: p.clone(),
            installed_version: it.version,
        });
    }
    out.sort_by(|a, b| {
        a.package
            .name
            .to_lowercase()
            .cmp(&b.package.name.to_lowercase())
            .then_with(|| a.package.id.cmp(&b.package.id))
    });
    out
}

/// A package's catalog icon, once its cached image has been decoded (start_list_icons fills these).
pub fn update_icon<'a>(ui: &'a PackageStoreUi, id: &str) -> Option<&'a Canvas> {
    let i = ui.index.packages.iter().position(|p| p.id == id)?;
    ui.list_icons.get(i).filter(|icon| !icon.px.is_empty())
}

/// Starts installing the catalog's newer version of `id`, through the same manifest, artifact and
/// hash checks as a Store install. The Updates page stays in front while it downloads.
pub fn updates_install(a: &mut App, id: &str) {
    let ui = &mut a.store;
    if !ui.updates_center || ui.update_installing {
        return;
    }
    let Some(pkg) = ui.index.packages.iter().find(|p| p.id == id).cloned() else { return };
    let newer = installed_for(&pkg).is_some_and(|it| update_version_cmp(&pkg.version, &it.version) > 0);
    if !newer {
        return;
    }
    ui.selected = None;
    ui.clear_selection();
    ui.selected = Some(pkg.id.clone());
    ui.update_target = pkg.id.clone();
    ui.update_installing = true;
    ui.update_status = tr("Downloading update...").to_string();
    ui.update_error.clear();
    ui.status.clear();
    ui.error.clear();
    ui.manifest_url = pkg.manifest_url.clone();
    if let Err(e) = begin_fetch(ui, FetchSlot::Install, FetchStage::Manifest, &pkg.manifest_url, WIDGET_PKG_MANIFEST_MAX, "") {
        update_failed(a, e);
    }
    crate::library::store_redraw(a);
}

fn close(a: &mut App) {
    a.store.cancel_jobs();
    a.store.open = false;
    a.store.clear_selection();
    crate::library::store_closed(a);
}

/// The Library is leaving (a notebook opened, a turn): keep nothing downloading behind it.
pub fn store_leave(a: &mut App) {
    if a.store.open {
        a.store.cancel_jobs();
        a.store.open = false;
        a.store.clear_selection();
    }
}

fn redraw(a: &mut App) {
    crate::library::store_redraw(a);
}

fn filtered(ui: &PackageStoreUi) -> Vec<usize> {
    ui.index
        .packages
        .iter()
        .enumerate()
        .filter(|(_, p)| match ui.filter {
            1 => p.kind == "app",
            2 => p.kind == "widget",
            _ => true,
        })
        .map(|(i, _)| i)
        .collect()
}

fn installed_for(p: &StorePackage) -> Option<WidgetInstall> {
    if p.kind == "app" {
        app_package_list().into_iter().find(|x| x.id == p.id)
    } else {
        widget_package_list().into_iter().find(|x| x.id == p.id)
    }
}

fn install_label(p: &StorePackage) -> (String, ButtonStyle, bool) {
    match installed_for(p) {
        Some(it) if update_version_cmp(&p.version, &it.version) <= 0 => (tr("Installed").to_string(), ButtonStyle::Disabled, false),
        Some(_) => (format!("{} {}", tr("Update"), p.version), ButtonStyle::Primary, true),
        None => (tr("Install").to_string(), ButtonStyle::Primary, true),
    }
}

enum FetchSlot {
    Install,
    Asset,
}

fn begin_fetch(ui: &mut PackageStoreUi, slot: FetchSlot, stage: FetchStage, url: &str, cap: usize, key: &str) -> Result<(), String> {
    let fetch = WidgetFetch::start(url, cap)?;
    let job = FetchJob {
        stage,
        fetch,
        url: url.to_string(),
        key: key.to_string(),
    };
    match slot {
        FetchSlot::Install => ui.install_job = Some(job),
        FetchSlot::Asset => ui.asset_job = Some(job),
    }
    Ok(())
}

fn begin_install(a: &mut App) {
    let Some(pkg) = a.store.selected_package().cloned() else { return };
    let (_, style, enabled) = install_label(&pkg);
    if !enabled && style == ButtonStyle::Disabled {
        return;
    }
    let ui = &mut a.store;
    ui.error.clear();
    ui.status = tr("Reading package.yaml...").to_string();
    ui.manifest = None;
    ui.manifest_url = pkg.manifest_url.clone();
    ui.manifest_yaml.clear();
    ui.artifact_url.clear();
    ui.icon_url = None;
    ui.install_icon_key.clear();
    ui.pending_artifact.clear();
    ui.view = StoreView::Busy;
    if let Err(e) = begin_fetch(ui, FetchSlot::Install, FetchStage::Manifest, &pkg.manifest_url, WIDGET_PKG_MANIFEST_MAX, "") {
        ui.error = e;
        ui.view = StoreView::Error;
    }
    redraw(a);
}

fn start_artifact(a: &mut App) {
    let Some(m) = a.store.manifest.as_ref() else { return };
    let Some(url) = widget_package_artifact_url(&a.store.manifest_url, m) else {
        a.store.error = tr("The manifest names no artifact that can be downloaded.").to_string();
        a.store.view = StoreView::Error;
        redraw(a);
        return;
    };
    let ui = &mut a.store;
    ui.artifact_url = url.clone();
    ui.error.clear();
    ui.status = tr("Downloading the package...").to_string();
    ui.view = StoreView::Busy;
    if let Err(e) = begin_fetch(ui, FetchSlot::Install, FetchStage::Artifact, &url, WIDGET_PKG_ARTIFACT_MAX as usize, "") {
        ui.error = e;
        ui.view = StoreView::Error;
    }
    redraw(a);
}

fn start_install_icon(a: &mut App) {
    let icon_url = a.store.icon_url.clone();
    let Some(url) = icon_url else {
        install_pending(a, None);
        return;
    };
    let key = a.store.install_icon_key.clone();
    if let Some(data) = read_cached_asset(&key, INSTALL_ICON_MAX) {
        install_pending(a, Some(&data));
        return;
    }
    a.store.status = tr("Downloading the package icon...").to_string();
    a.store.view = StoreView::Busy;
    if let Err(e) = begin_fetch(&mut a.store, FetchSlot::Install, FetchStage::InstallIcon, &url, INSTALL_ICON_MAX, &key) {
        a.store.error = e;
        a.store.view = StoreView::Error;
    }
    redraw(a);
}

fn validate_for_kind(p: &WidgetPackage, kind: &str, w: i32, h: i32) -> Result<(), String> {
    match kind {
        "app" => app_package_validate(p, w, h),
        "widget" => widget_package_validate(p, w, h),
        _ => Err("The Store does not know this package kind.".into()),
    }
}

/// Installs what the last download left ready. The caller has validated the manifest; the
/// installers check the artifact's size and hash again before writing anything.
fn install_from_pending(a: &mut App, icon: Option<&[u8]>) -> Result<(String, String), String> {
    let Some(m) = a.store.manifest.clone() else {
        return Err(tr("The package's manifest was lost.").to_string());
    };
    let manifest = std::mem::take(&mut a.store.manifest_yaml);
    let artifact = std::mem::take(&mut a.store.pending_artifact);
    let source = a.store.manifest_url.clone();
    let listed = a
        .store
        .index
        .packages
        .iter()
        .find(|p| p.id == m.id)
        .cloned()
        .ok_or_else(|| tr("That package is no longer in the Store.").to_string())?;
    let it = if listed.kind == "app" {
        app_package_install(&manifest, &artifact, icon, &source, a.w, a.h - status_reserved(a))?
    } else {
        widget_package_install(&manifest, &artifact, &source, a.w, a.h - status_reserved(a))?
    };
    Ok((it.name, it.version))
}

fn install_pending(a: &mut App, icon: Option<&[u8]>) {
    match install_from_pending(a, icon) {
        Ok((name, _)) => {
            a.store.status = format!("{} {}", tr("Installed"), name);
            a.store.view = StoreView::Detail;
            a.store.error.clear();
        }
        Err(e) => {
            a.store.error = e;
            a.store.view = StoreView::Error;
        }
    }
}

fn install_update_pending(a: &mut App, icon: Option<&[u8]>) {
    match install_from_pending(a, icon) {
        Ok((name, version)) => {
            let ui = &mut a.store;
            ui.update_installing = false;
            ui.update_target.clear();
            ui.update_error.clear();
            ui.manifest = None;
            ui.manifest_url.clear();
            ui.manifest_yaml.clear();
            ui.artifact_url.clear();
            ui.icon_url = None;
            ui.install_icon_key.clear();
            ui.pending_artifact.clear();
            let mut msg = String::new();
            csnprintf!(&mut msg, 300, tr("Updated %s to %s."), &name, &version);
            ui.update_status = msg;
        }
        Err(e) => update_failed(a, e),
    }
}

fn start_selected_assets(a: &mut App) {
    let Some(pkg) = a.store.selected_package().cloned() else { return };
    if a.store.icon_for != pkg.id {
        a.store.icon.reset();
        a.store.icon_for = pkg.id.clone();
        a.store.icon_tried = false;
        if let StoreIconRef::Glyph(g) = pkg.icon.clone() {
            a.store.icon.glyph = g;
        }
    }
    if a.store.shot_for != pkg.id {
        canvas_free(&mut a.store.shot);
        a.store.shot_for = pkg.id.clone();
        a.store.shot_tried = false;
        a.store.shot_index = 0;
    }
    if a.store.asset_job.is_some() {
        return;
    }
    if !a.store.icon_tried {
        if let StoreIconRef::Image(asset) = pkg.icon.clone() {
            if let Some(data) = read_cached_asset(&asset.key, INSTALL_ICON_MAX) {
                apply_icon(&mut a.store, &data);
                a.store.icon_tried = true;
            } else {
                start_asset_job(a, FetchStage::Icon, &asset.url, INSTALL_ICON_MAX, &asset.key);
                return;
            }
        } else {
            a.store.icon_tried = true;
        }
    }
    if !a.store.shot_tried {
        if let Some(asset) = pkg.screenshots.get(a.store.shot_index as usize).cloned() {
            if let Some(data) = read_cached_asset(&asset.key, SCREENSHOT_MAX) {
                apply_screenshot(&mut a.store, &data);
                a.store.shot_tried = true;
            } else {
                start_asset_job(a, FetchStage::Screenshot, &asset.url, SCREENSHOT_MAX, &asset.key);
                return;
            }
        } else {
            a.store.shot_tried = true;
        }
    }
}

fn start_asset_job(a: &mut App, stage: FetchStage, url: &str, cap: usize, key: &str) {
    if let Err(e) = begin_fetch(&mut a.store, FetchSlot::Asset, stage, url, cap, key) {
        qlog!("package store: asset fetch failed to start: %s", &e);
        match stage {
            FetchStage::Icon => a.store.icon_tried = true,
            FetchStage::Screenshot => a.store.shot_tried = true,
            FetchStage::ListIcon => mark_list_icon_tried(&mut a.store, key),
            _ => {}
        }
    }
}

/// Load the catalog cards' own icons, one cached image at a time and one download at a time.
/// Returns whether an image was loaded without waiting on the network.
fn start_list_icons(a: &mut App) -> bool {
    let showing = a.store.updates_center || (a.store.open && a.store.view == StoreView::List);
    if !showing || a.store.asset_job.is_some() {
        return false;
    }
    if a.store.list_icons.len() != a.store.index.packages.len() || a.store.list_tried.len() != a.store.index.packages.len() {
        a.store.reset_list_icons();
    }
    let mut loaded = false;
    for i in 0..a.store.index.packages.len() {
        let has_image = a.store.list_icons.get(i).is_some_and(|icon| !icon.px.is_empty());
        if has_image || a.store.list_tried.get(i).copied().unwrap_or(false) {
            continue;
        }
        let Some((url, key)) = (match &a.store.index.packages[i].icon {
            StoreIconRef::Image(asset) => Some((asset.url.clone(), asset.key.clone())),
            StoreIconRef::Glyph(_) => None,
        }) else {
            continue;
        };
        if let Some(data) = read_cached_asset(&key, INSTALL_ICON_MAX) {
            if apply_list_icon_at(&mut a.store, i, &data) {
                loaded = true;
            } else {
                a.store.list_tried[i] = true;
            }
        } else {
            start_asset_job(a, FetchStage::ListIcon, &url, INSTALL_ICON_MAX, &key);
            return loaded;
        }
    }
    loaded
}

fn store_image(data: &[u8]) -> Option<Canvas> {
    if let Some(img) = crate::icofile::ico_gray(data) {
        let mut out = Canvas::default();
        if canvas_init(&mut out, img.w as i32, img.h as i32) != 0 {
            return None;
        }
        for y in 0..img.h as i32 {
            for x in 0..img.w as i32 {
                let px = img.gray[(y * img.w as i32 + x) as usize];
                canvas_fill(&mut out, Rect::new(x, y, 1, 1), px);
            }
        }
        return Some(out);
    }
    let mut pic = BgPicture::default();
    let mut why = "";
    if bgimage_decode(data, &mut pic, &mut why) != 0 {
        return None;
    }
    let mut out = Canvas::default();
    if bgimage_luma(&pic, &mut out) != 0 {
        canvas_free(&mut out);
        return None;
    }
    // Keep the decoded icon small enough for the detail header and the cards.
    if out.w > 256 || out.h > 256 {
        let mut small = Canvas::default();
        if canvas_init(&mut small, 256, 256) == 0 {
            canvas_blit_fit(&mut small, &out, Rect::new(0, 0, 256, 256));
            canvas_free(&mut out);
            return Some(small);
        }
        canvas_free(&mut small);
    }
    Some(out)
}

fn apply_icon(ui: &mut PackageStoreUi, data: &[u8]) {
    if let Some(image) = store_image(data) {
        canvas_free(&mut ui.icon.image);
        ui.icon.image = image;
    }
}

fn apply_list_icon_at(ui: &mut PackageStoreUi, index: usize, data: &[u8]) -> bool {
    let Some(image) = store_image(data) else { return false };
    let Some(slot) = ui.list_icons.get_mut(index) else { return false };
    canvas_free(slot);
    *slot = image;
    true
}

fn apply_list_icon(ui: &mut PackageStoreUi, key: &str, data: &[u8]) -> bool {
    let index = ui.index.packages.iter().position(|p| matches!(&p.icon, StoreIconRef::Image(a) if a.key == key));
    index.is_some_and(|i| apply_list_icon_at(ui, i, data))
}

fn mark_list_icon_tried(ui: &mut PackageStoreUi, key: &str) {
    for (i, p) in ui.index.packages.iter().enumerate() {
        if let StoreIconRef::Image(asset) = &p.icon {
            if asset.key == key {
                if let Some(tried) = ui.list_tried.get_mut(i) {
                    *tried = true;
                }
            }
        }
    }
}

fn apply_screenshot(ui: &mut PackageStoreUi, data: &[u8]) {
    let mut pic = BgPicture::default();
    let mut why = "";
    if bgimage_decode(data, &mut pic, &mut why) != 0 {
        qlog!("package store: screenshot refused: %s", why);
        return;
    }
    canvas_free(&mut ui.shot);
    if bgimage_luma(&pic, &mut ui.shot) != 0 {
        qlog!("package store: screenshot has no memory for its canvas");
    }
}

fn accept_manifest(ui: &mut PackageStoreUi, data: &[u8], w: i32, h: i32) -> Result<WidgetPackage, String> {
    let text = String::from_utf8_lossy(data);
    let selected = ui
        .selected_package()
        .cloned()
        .ok_or_else(|| tr("The selected package is gone.").to_string())?;
    let p = widget_package_parse(&text).and_then(|p| {
        validate_for_kind(&p, &selected.kind, w, h).map(|_| p)
    })?;
    if p.id != selected.id || p.name != selected.name {
        return Err(tr("The manifest does not match its Store listing.").to_string());
    }
    if update_version_cmp(&p.version, &selected.version) != 0 {
        return Err(tr("The manifest's version does not match its Store listing.").to_string());
    }
    ui.manifest_yaml = text.into_owned();
    ui.artifact_url = widget_package_artifact_url(&ui.manifest_url, &p).unwrap_or_default();
    ui.icon_url = if selected.kind == "app" { app_package_icon_url(&ui.manifest_url, &p) } else { None };
    ui.install_icon_key.clear();
    if ui.icon_url.is_some() {
        if let Some(key) = safe_key(&format!("{}/{}", selected.id, p.spec.icon)) {
            ui.install_icon_key = key;
        }
    }
    Ok(p)
}

fn after_manifest(ui: &mut PackageStoreUi, data: &[u8], w: i32, h: i32) {
    match accept_manifest(ui, data, w, h) {
        Ok(p) => {
            ui.manifest = Some(p);
            ui.status.clear();
            ui.error.clear();
            ui.view = StoreView::Trust;
        }
        Err(e) => {
            ui.error = e;
            ui.view = StoreView::Error;
        }
    }
}

fn update_failed(a: &mut App, error: String) {
    let ui = &mut a.store;
    qlog!("package update: %s", &error);
    ui.update_installing = false;
    ui.update_target.clear();
    ui.update_status.clear();
    ui.update_error = error;
    ui.pending_artifact.clear();
}

fn after_update_manifest(a: &mut App, data: &[u8]) {
    let w = a.w;
    let h = a.h - status_reserved(a);
    match accept_manifest(&mut a.store, data, w, h) {
        Ok(p) => {
            a.store.manifest = Some(p);
            a.store.update_status = tr("Downloading update...").to_string();
            a.store.update_error.clear();
            start_update_artifact(a);
        }
        Err(e) => update_failed(a, e),
    }
}

fn start_update_artifact(a: &mut App) {
    let Some(m) = a.store.manifest.as_ref() else {
        update_failed(a, tr("The package's manifest was lost.").to_string());
        return;
    };
    let Some(url) = widget_package_artifact_url(&a.store.manifest_url, m) else {
        update_failed(a, tr("The manifest names no artifact that can be downloaded.").to_string());
        return;
    };
    let ui = &mut a.store;
    ui.artifact_url = url.clone();
    ui.update_status = tr("Downloading update...").to_string();
    if let Err(e) = begin_fetch(ui, FetchSlot::Install, FetchStage::Artifact, &url, WIDGET_PKG_ARTIFACT_MAX as usize, "") {
        update_failed(a, e);
    }
}

fn start_update_icon(a: &mut App) {
    let icon_url = a.store.icon_url.clone();
    let Some(url) = icon_url else {
        install_update_pending(a, None);
        return;
    };
    let key = a.store.install_icon_key.clone();
    if let Some(data) = read_cached_asset(&key, INSTALL_ICON_MAX) {
        install_update_pending(a, Some(&data));
        return;
    }
    a.store.update_status = tr("Downloading the package icon...").to_string();
    if let Err(e) = begin_fetch(&mut a.store, FetchSlot::Install, FetchStage::InstallIcon, &url, INSTALL_ICON_MAX, &key) {
        update_failed(a, e);
    }
}

fn tick_catalog(a: &mut App) -> bool {
    let Some(mut job) = a.store.catalog_job.take() else { return false };
    let Some(res) = job.fetch.tick() else {
        a.store.catalog_job = Some(job);
        return false;
    };
    a.store.refreshing = false;
    match res {
        Ok(data) => match store_index_parse(&data, &job.url) {
            Ok(index) => {
                write_cached_index(&data);
                let selected = a.store.selected.clone();
                a.store.index = index;
                a.store.reset_list_icons();
                a.store.selected = selected.filter(|id| a.store.index.packages.iter().any(|p| &p.id == id));
                if a.store.selected.is_none() {
                    a.store.clear_selection();
                }
                if a.store.view == StoreView::Busy || a.store.view == StoreView::Error {
                    a.store.view = StoreView::List;
                }
                a.store.status = format!("{} {}", tr("Updated"), a.store.index.updated);
                a.store.error.clear();
                if !a.store.index.packages.is_empty() && a.store.view == StoreView::List {
                    a.store.page = 0;
                }
                true
            }
            Err(e) => {
                if a.store.index.packages.is_empty() {
                    a.store.error = e;
                    a.store.view = StoreView::Error;
                } else {
                    a.store.status = e;
                }
                true
            }
        },
        Err(e) => {
            if a.store.index.packages.is_empty() {
                a.store.error = e;
                a.store.view = StoreView::Error;
            } else {
                a.store.status = e;
            }
            true
        }
    }
}

fn tick_install(a: &mut App) -> bool {
    let Some(mut job) = a.store.install_job.take() else { return false };
    let Some(res) = job.fetch.tick() else {
        a.store.install_job = Some(job);
        return false;
    };
    match res {
        Ok(data) => match job.stage {
            FetchStage::Manifest => {
                if a.store.update_installing {
                    after_update_manifest(a, &data);
                } else {
                    let w = a.w;
                    let h = a.h - status_reserved(a);
                    after_manifest(&mut a.store, &data, w, h);
                }
            }
            FetchStage::Artifact => {
                a.store.pending_artifact = data;
                if a.store.update_installing {
                    if a.store.icon_url.is_some() {
                        start_update_icon(a);
                    } else {
                        install_update_pending(a, None);
                    }
                } else if a.store.icon_url.is_some() {
                    start_install_icon(a);
                } else {
                    install_pending(a, None);
                }
            }
            FetchStage::InstallIcon => {
                if !job.key.is_empty() {
                    write_cached_asset(&job.key, &data);
                }
                let icon = data.clone();
                if a.store.update_installing {
                    install_update_pending(a, Some(&icon));
                } else {
                    install_pending(a, Some(&icon));
                }
            }
            _ => {}
        },
        Err(e) => {
            if a.store.update_installing {
                update_failed(a, e);
            } else {
                a.store.error = e;
                a.store.view = StoreView::Error;
            }
        }
    }
    true
}

fn tick_asset(a: &mut App) -> bool {
    let Some(mut job) = a.store.asset_job.take() else { return false };
    let Some(res) = job.fetch.tick() else {
        a.store.asset_job = Some(job);
        return false;
    };
    match res {
        Ok(data) => {
            if !job.key.is_empty() {
                write_cached_asset(&job.key, &data);
            }
            match job.stage {
                FetchStage::Icon => {
                    apply_icon(&mut a.store, &data);
                    a.store.icon_tried = true;
                }
                FetchStage::Screenshot => {
                    apply_screenshot(&mut a.store, &data);
                    a.store.shot_tried = true;
                }
                FetchStage::ListIcon => {
                    if !apply_list_icon(&mut a.store, &job.key, &data) {
                        mark_list_icon_tried(&mut a.store, &job.key);
                    }
                }
                _ => {}
            }
        }
        Err(e) => {
            qlog!("package store: %s", &e);
            match job.stage {
                FetchStage::Icon => a.store.icon_tried = true,
                FetchStage::Screenshot => a.store.shot_tried = true,
                FetchStage::ListIcon => mark_list_icon_tried(&mut a.store, &job.key),
                _ => {}
            }
        }
    }
    if a.store.open && a.store.view == StoreView::Detail {
        start_selected_assets(a);
    } else {
        start_list_icons(a);
    }
    true
}

pub fn store_tick(a: &mut App, _now: u32) {
    if !a.store.open {
        return;
    }
    let mut changed = tick_catalog(a);
    changed |= tick_install(a);
    changed |= tick_asset(a);
    if a.store.view == StoreView::List {
        changed |= start_list_icons(a);
    }
    if changed {
        redraw(a);
    }
}

// ---- pointer ----

fn open_detail(a: &mut App, n: usize) {
    let Some(pkg) = a.store.index.packages.get(n).cloned() else { return };
    // A card icon still downloading must not hold up the page the tap asked for.
    if a.store.asset_job.as_ref().is_some_and(|job| job.stage == FetchStage::ListIcon) {
        a.store.asset_job = None;
    }
    a.store.selected = Some(pkg.id.clone());
    a.store.view = StoreView::Detail;
    a.store.status.clear();
    a.store.error.clear();
    a.store.shot_index = 0;
    a.store.icon_for.clear();
    a.store.shot_for.clear();
    a.store.icon_tried = false;
    a.store.shot_tried = false;
    start_selected_assets(a);
    redraw(a);
}

fn shot_move(a: &mut App, step: i32) {
    let Some(pkg) = a.store.selected_package().cloned() else { return };
    let n = pkg.screenshots.len() as i32;
    if n == 0 {
        return;
    }
    let next = (a.store.shot_index + step).clamp(0, n - 1);
    if next == a.store.shot_index {
        return;
    }
    a.store.shot_index = next;
    a.store.shot_tried = false;
    canvas_free(&mut a.store.shot);
    a.store.asset_job = None;
    start_selected_assets(a);
}

fn tapped(a: &mut App, h: Hit) {
    match h.id {
        H_CLOSE => close(a),
        H_BACK => {
            match a.store.view {
                StoreView::List => close(a),
                StoreView::Trust | StoreView::Error | StoreView::Qr => {
                    a.store.view = StoreView::Detail;
                    a.store.error.clear();
                    a.store.install_job = None;
                    redraw(a);
                }
                _ => {
                    a.store.view = StoreView::List;
                    a.store.error.clear();
                    a.store.install_job = None;
                    a.store.asset_job = None;
                    redraw(a);
                }
            }
        }
        H_REFRESH => {
            a.store.error.clear();
            start_catalog(&mut a.store);
            redraw(a);
        }
        H_FILTER => {
            a.store.filter = clampi(h.arg, 0, 2);
            a.store.page = 0;
            redraw(a);
        }
        H_CARD => open_detail(a, h.arg as usize),
        H_PREV => {
            a.store.page = (a.store.page - 1).max(0);
            redraw(a);
        }
        H_NEXT => {
            a.store.page += 1;
            redraw(a);
        }
        H_SHOT_PREV => {
            shot_move(a, -1);
            redraw(a);
        }
        H_SHOT_NEXT => {
            shot_move(a, 1);
            redraw(a);
        }
        H_INSTALL => begin_install(a),
        H_TRUST => start_artifact(a),
        H_QR => {
            a.store.view = StoreView::Qr;
            redraw(a);
        }
        H_RETRY => {
            if a.store.index.packages.is_empty() {
                start_catalog(&mut a.store);
            } else {
                a.store.view = StoreView::List;
                a.store.error.clear();
            }
            redraw(a);
        }
        _ => {}
    }
}

pub fn store_pointer(a: &mut App, ph: i32, ev: &InEvent) {
    if !a.store.open {
        return;
    }
    if ph == 0 {
        press_begin(&mut a.store.press, ev.x, ev.y, ev.t_ms);
        a.store.pressed = None;
        a.store.pressed_hit = None;
        if let Some(h) = hits_find(&a.store.hits, ev.x, ev.y).copied() {
            app_press_feedback(a, h.r);
            a.store.pressed = Some(h.r);
            a.store.pressed_hit = Some(h);
        }
        return;
    }
    press_move(&mut a.store.press, ev.x, ev.y);
    if ph != 2 {
        return;
    }
    let mut pressed = a.store.pressed.take();
    let back = app_press_release(a, &mut pressed);
    let cancelled = touch_cancelled(ev);
    let hit = a.store.pressed_hit.take().filter(|h| !cancelled && press_is_tap(&a.store.press, h.r));
    if let Some(h) = hit {
        tapped(a, h);
        return;
    }
    if !rect_is_empty(back) {
        app_invalidate(a, back, RefreshMode::Ui);
    }
}

// ---- layout and drawing ----

const PAGER_H: i32 = 110;

struct StorePage {
    content: Rect,
    back: Rect,
    close: Rect,
    refresh: Rect,
}

fn store_page(c: &mut Canvas, aw: i32, ah: i32, title: &str, back: bool, refresh: bool) -> StorePage {
    canvas_fill(c, Rect::new(0, 0, aw, ah), 255);
    let mut actions: Vec<(IconKind, i32)> = Vec::with_capacity(2);
    if refresh {
        actions.push((IconKind::Update, 0));
    }
    actions.push((IconKind::Close, 0));
    let (head, rects) = gui_header(c, aw, title, back, &actions);
    let pad = gui_margin();
    let close = rects.last().map_or(RECT_EMPTY, |r| r.0);
    let refresh_r = if refresh { rects.first().map_or(RECT_EMPTY, |r| r.0) } else { RECT_EMPTY };
    StorePage {
        content: Rect::new(pad, head.foot, aw - 2 * pad, ah - head.foot - pad),
        back: head.back,
        close,
        refresh: refresh_r,
    }
}

fn draw_header_hits(ui: &mut PackageStoreUi, page: &StorePage, refresh: bool) {
    if !rect_is_empty(page.back) {
        hits_add(&mut ui.hits, page.back, H_BACK, 0);
    }
    if !rect_is_empty(page.close) {
        hits_add(&mut ui.hits, page.close, H_CLOSE, 0);
    }
    if refresh && !rect_is_empty(page.refresh) {
        hits_add(&mut ui.hits, page.refresh, H_REFRESH, 0);
    }
}

fn draw_pager(c: &mut Canvas, ui: &mut PackageStoreUi, area: Rect, page: i32, pages: i32) {
    if pages <= 1 {
        return;
    }
    let bw = ui_px(110.0);
    let bh = ui_px(90.0);
    let y = area.y + area.h - bh;
    let prev = Rect::new(area.x, y, bw, bh);
    let next = Rect::new(area.x + area.w - bw, y, bw, bh);
    gui_icon_button(c, prev, IconKind::Prev, if page > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    gui_icon_button(c, next, IconKind::Next, if page + 1 < pages { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    if page > 0 {
        hits_add(&mut ui.hits, prev, H_PREV, 0);
    }
    if page + 1 < pages {
        hits_add(&mut ui.hits, next, H_NEXT, 0);
    }
    font_draw_centered(
        c,
        FontFace::Fixed,
        ui_small_size(),
        Rect::new(prev.x + prev.w, y, (next.x - prev.x - prev.w).max(0), bh),
        &format!("{}/{}", page + 1, pages),
        0,
    );
}

fn kind_word(p: &StorePackage) -> &'static str {
    if p.kind == "app" { tr("App") } else { tr("Widget") }
}

fn draw_tag(c: &mut Canvas, r: Rect, text: &str, selected: bool) {
    let rad = if ui_theme() == UiTheme::Gothic { ui_px(10.0) } else { r.h / 2 };
    ui_frame(c, r, rad, if selected { 0 } else { 255 }, true, 0);
    ui_frame(c, r, rad, if selected { 0 } else { 150 }, false, ui_px(2.0));
    font_draw_centered(c, FontFace::Fixed, ui_small_size() * 0.82, r, text, if selected { 255 } else { 80 });
}

fn draw_package_card(c: &mut Canvas, r: Rect, p: &StorePackage, featured: bool, icon_image: Option<&Canvas>) {
    ui_frame(c, r, gui_radius(), 255, true, 0);
    ui_frame(c, r, gui_radius(), 0, false, ui_px(3.0));
    let pad = ui_px(24.0);
    let icon_box = Rect::new(r.x + pad, r.y + pad, ui_px(120.0), ui_px(120.0));
    ui_frame(c, icon_box, gui_radius(), 255, true, 0);
    ui_frame(c, icon_box, gui_radius(), 0, false, ui_px(2.0));
    if let Some(image) = icon_image {
        canvas_blit_fit(c, image, rect_inflate(icon_box, -ui_px(12.0)));
    } else {
        let glyph = match &p.icon {
            StoreIconRef::Glyph(g) => *g,
            StoreIconRef::Image(_) => IconKind::Package,
        };
        ui_icon(c, glyph, rect_inflate(icon_box, -ui_px(22.0)), 0);
    }
    let tx = icon_box.x + icon_box.w + ui_px(18.0);
    let tw = (r.x + r.w - pad - tx).max(0);
    let title_h = ui_px(58.0);
    ui_text(c, FontFace::Bold, ui_text_size(), ui_small_size(), Rect::new(tx, r.y + pad, tw, title_h), &p.name, 1, false, 0);
    let sub = if p.category.is_empty() {
        format!("{} · {}", kind_word(p), p.version)
    } else {
        format!("{} · {} · {}", kind_word(p), p.category, p.version)
    };
    font_draw_left(c, FontFace::Regular, ui_small_size(), Rect::new(tx, r.y + pad + title_h, tw, ui_px(34.0)), &sub, 90);
    let body_y = icon_box.y + icon_box.h + ui_px(18.0);
    let body_h = (r.y + r.h - pad - body_y).max(ui_px(40.0));
    let summary = if p.summary.is_empty() { p.description.as_str() } else { p.summary.as_str() };
    let lh = (font_ascent(FontFace::Regular, ui_small_size()) + font_descent(FontFace::Regular, ui_small_size())).max(1);
    let lines = ((body_h - ui_px(52.0)) / lh).clamp(1, 3);
    font_draw_wrapped(c, FontFace::Regular, ui_small_size(), Rect::new(r.x + pad, body_y, r.w - 2 * pad, body_h - ui_px(50.0)), summary, lines, lh + ui_px(4.0), false, 0);
    let mut tag_x = r.x + pad;
    let tag_y = r.y + r.h - pad - ui_px(38.0);
    for tag in p.tags.iter().take(2) {
        let w = (font_measure(FontFace::Fixed, ui_small_size() * 0.82, tag) + ui_px(28.0)).min(ui_px(280.0));
        if tag_x + w > r.x + r.w - pad {
            break;
        }
        draw_tag(c, Rect::new(tag_x, tag_y, w, ui_px(38.0)), tag, false);
        tag_x += w + ui_px(10.0);
    }
    if featured {
        let label = tr("Featured");
        let w = font_measure(FontFace::Fixed, ui_small_size() * 0.82, label) + ui_px(28.0);
        draw_tag(c, Rect::new(r.x + r.w - pad - w, tag_y, w, ui_px(38.0)), label, true);
    }
}

fn draw_list(c: &mut Canvas, ui: &mut PackageStoreUi, aw: i32, ah: i32) {
    let page = store_page(c, aw, ah, tr("Package Store"), false, true);
    draw_header_hits(ui, &page, true);
    let area = page.content;
    let gap = gui_gap();
    let chips_h = ui_px(86.0);
    let labels = [tr("All"), tr("Apps"), tr("Widgets")];
    let mut widths = Vec::with_capacity(3);
    for label in &labels {
        widths.push((font_measure(FontFace::Bold, ui_text_size(), label) + ui_px(78.0)).max(ui_px(190.0)));
    }
    let total: i32 = widths.iter().sum::<i32>() + gap * 2;
    let max_w = (area.w - ui_px(12.0)).max(ui_px(240.0));
    if total > max_w {
        let each = ((max_w - gap * 2) / 3).max(ui_px(120.0));
        for w in &mut widths {
            *w = each;
        }
    }
    let mut x = area.x;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::new(x, area.y, widths[i], chips_h);
        gui_chip(c, r, label, ui.filter == i as i32);
        hits_add(&mut ui.hits, r, H_FILTER, i as i32);
        x += widths[i] + gap;
    }
    let top = area.y + chips_h + gap;
    let bottom = area.y + area.h;
    let pager = if filtered(ui).len() > 0 { PAGER_H + gap } else { 0 };
    let grid_h = (bottom - top - pager).max(ui_px(200.0));
    let cols = if area.w >= ui_px(2550.0) {
        4
    } else if area.w >= ui_px(1700.0) {
        3
    } else {
        2
    };
    let card_w = (area.w - gap * (cols - 1)) / cols;
    let rows = ((grid_h + gap) / (ui_px(330.0) + gap)).max(1);
    let card_h = ((grid_h - gap * (rows - 1)) / rows).clamp(ui_px(260.0), ui_px(430.0));
    let per = (cols * rows).max(1) as usize;
    let vis = filtered(ui);
    let pages = ((vis.len() + per - 1) / per).max(1) as i32;
    ui.page = ui.page.clamp(0, pages - 1);
    let first = ui.page as usize * per;
    if vis.is_empty() {
        let msg = if ui.refreshing { tr("Reading the Store...") } else { tr("Nothing in the Store yet.") };
        font_draw_centered(c, FontFace::Bold, ui_title_size(), Rect::new(area.x, top + ui_px(120.0), area.w, ui_px(90.0)), msg, 0);
        let hint = if ui.error.is_empty() { tr("Tap Refresh to check again.") } else { ui.error.as_str() };
        ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), Rect::new(area.x, top + ui_px(230.0), area.w, ui_px(180.0)), hint, 2, true, 70);
        if !ui.error.is_empty() {
            let r = Rect::new(area.x + area.w / 2 - ui_px(170.0), top + ui_px(430.0), ui_px(340.0), ui_px(100.0));
            gui_button(c, r, IconKind::Update, tr("Refresh"), ButtonStyle::Plain);
            hits_add(&mut ui.hits, r, H_RETRY, 0);
        }
        return;
    }
    let shown = vis.len().saturating_sub(first).min(per);
    for (k, n) in vis.iter().skip(first).take(shown).enumerate() {
        let row = (k / cols as usize) as i32;
        let in_row = (shown - row as usize * cols as usize).min(cols as usize) as i32;
        let row_w = in_row * card_w + (in_row - 1).max(0) * gap;
        let x0 = area.x + (area.w - row_w) / 2;
        let col = (k % cols as usize) as i32;
        let r = Rect::new(x0 + col * (card_w + gap), top + row * (card_h + gap), card_w, card_h);
        let p = &ui.index.packages[*n];
        let featured = p.id == ui.index.featured;
        let icon_image = ui.list_icons.get(*n).filter(|icon| !icon.px.is_empty());
        draw_package_card(c, r, p, featured, icon_image);
        hits_add(&mut ui.hits, r, H_CARD, *n as i32);
    }
    draw_pager(c, ui, Rect::new(area.x, bottom - PAGER_H, area.w, PAGER_H), ui.page, pages);
    if !ui.status.is_empty() {
        font_draw_centered(c, FontFace::Regular, ui_small_size(), Rect::new(area.x, bottom - PAGER_H - ui_px(38.0), area.w, ui_px(34.0)), &ui.status, 100);
    }
}

fn draw_shot(c: &mut Canvas, ui: &mut PackageStoreUi, r: Rect, p: &StorePackage) {
    ui_frame(c, r, gui_radius(), 255, true, 0);
    ui_frame(c, r, gui_radius(), 0, false, ui_px(3.0));
    let inner = rect_inflate(r, -ui_px(16.0));
    let n = p.screenshots.len() as i32;
    if n == 0 {
        ui_icon(c, IconKind::Image, rect_inflate(inner, -ui_px(120.0)), 150);
        font_draw_centered(c, FontFace::Regular, ui_text_size(), inner, tr("No screenshots yet."), 100);
        return;
    }
    let bar = ui_px(64.0);
    let image_r = Rect::new(inner.x, inner.y, inner.w, (inner.h - bar - ui_px(10.0)).max(ui_px(80.0)));
    if !ui.shot.px.is_empty() {
        canvas_blit_fit(c, &ui.shot, image_r);
    } else {
        ui_icon(c, IconKind::Image, rect_inflate(image_r, -ui_px(100.0)), 155);
        font_draw_centered(c, FontFace::Regular, ui_text_size(), image_r, tr("Loading screenshot..."), 100);
    }
    let y = image_r.y + image_r.h + ui_px(10.0);
    let label = format!("{} {}/{}", tr("Screenshot"), ui.shot_index + 1, n);
    font_draw_left(c, FontFace::Regular, ui_small_size(), Rect::new(inner.x, y, (inner.w - ui_px(230.0)).max(0), bar), &label, 90);
    let bw = ui_px(100.0);
    let prev = Rect::new(inner.x + inner.w - 2 * bw - ui_px(12.0), y, bw, bar);
    let next = Rect::new(inner.x + inner.w - bw, y, bw, bar);
    gui_icon_button(c, prev, IconKind::Prev, if ui.shot_index > 0 { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    gui_icon_button(c, next, IconKind::Next, if ui.shot_index + 1 < n { ButtonStyle::Plain } else { ButtonStyle::Disabled });
    if ui.shot_index > 0 {
        hits_add(&mut ui.hits, prev, H_SHOT_PREV, 0);
    }
    if ui.shot_index + 1 < n {
        hits_add(&mut ui.hits, next, H_SHOT_NEXT, 0);
    }
}

fn draw_meta(c: &mut Canvas, r: Rect, p: &StorePackage) {
    let mut y = r.y;
    let lh = ui_px(38.0);
    let mut row = |text: &str, gray: u8| {
        if y + lh <= r.y + r.h {
            font_draw_left(c, FontFace::Regular, ui_small_size(), Rect::new(r.x, y, r.w, lh), text, gray);
            y += lh;
        }
    };
    if !p.author.is_empty() {
        row(&format!("{} {}", tr("By"), p.author), 0);
    }
    if !p.category.is_empty() {
        row(&format!("{} {}", tr("Category:"), p.category), 70);
    }
    if !p.license.is_empty() {
        row(&format!("{} {}", tr("License:"), p.license), 70);
    }
    if !p.repository.is_empty() {
        row(&format!("{} {}", tr("GitHub:"), p.repository), 70);
    }
    if !p.homepage.is_empty() {
        row(&format!("{} {}", tr("Homepage:"), p.homepage), 70);
    }
    if !p.tags.is_empty() {
        row(&format!("{} {}", tr("Tags:"), p.tags.join(" · ")), 70);
    }
    if !p.added.is_empty() {
        row(&format!("{} {}", tr("Added:"), p.added), 70);
    }
}

fn draw_description(c: &mut Canvas, r: Rect, p: &StorePackage) {
    let px = ui_text_size();
    let lh = (font_ascent(FontFace::Regular, px) + font_descent(FontFace::Regular, px)).max(1) + ui_px(4.0);
    let lines = (r.h / lh).max(1);
    font_draw_wrapped(c, FontFace::Regular, px, r, &p.description, lines, lh, false, 0);
}

fn draw_detail(c: &mut Canvas, ui: &mut PackageStoreUi, aw: i32, ah: i32) {
    let title = tr("Package");
    let page = store_page(c, aw, ah, title, true, false);
    draw_header_hits(ui, &page, false);
    let Some(p) = ui.selected_package().cloned() else {
        ui.view = StoreView::List;
        return;
    };
    let area = page.content;
    let gap = gui_gap();
    let wide = area.w > area.h;
    let icon_sz = if wide { ui_px(150.0) } else { ui_px(140.0) };
    let header_h = icon_sz.max(ui_px(150.0));
    let action_h = ui_px(100.0);
    let action_gap = ui_px(14.0);
    let action_count = if p.repository.is_empty() { 1 } else { 2 };
    let actions_h = action_h;
    let meta_h = ui_px(228.0);
    let (info, shot, desc_h) = if wide {
        let info = Rect::new(area.x, area.y, area.w * 43 / 100, area.h);
        let shot = Rect::new(area.x + info.w + gap * 2, area.y, area.w - info.w - gap * 2, area.h);
        let desc_h = (info.h - header_h - ui_px(18.0) - actions_h - meta_h - ui_px(52.0))
            .clamp(ui_px(260.0), ui_px(760.0));
        (info, shot, desc_h)
    } else {
        let desc_h = ui_px(460.0);
        let fixed = header_h + ui_px(18.0) + desc_h + ui_px(8.0) + meta_h + ui_px(16.0) + actions_h + ui_px(14.0);
        let max_info = (area.h - gap - ui_px(520.0)).max(ui_px(760.0));
        let info_h = fixed.min(max_info).max(ui_px(760.0));
        let info = Rect::new(area.x, area.y, area.w, info_h);
        let shot = Rect::new(area.x, info.y + info.h + gap, area.w, (area.y + area.h - info.y - info.h - gap).max(ui_px(240.0)));
        (info, shot, desc_h)
    };
    let icon_r = Rect::new(info.x, info.y, icon_sz, icon_sz);
    ui_frame(c, icon_r, gui_radius(), 255, true, 0);
    ui_frame(c, icon_r, gui_radius(), 0, false, ui_px(3.0));
    if ui.icon_for == p.id {
        ui.icon.draw(c, rect_inflate(icon_r, -ui_px(20.0)), 0);
    } else {
        let glyph = match &p.icon {
            StoreIconRef::Glyph(g) => *g,
            StoreIconRef::Image(_) => IconKind::Package,
        };
        ui_icon(c, glyph, rect_inflate(icon_r, -ui_px(28.0)), 0);
    }
    let tx = icon_r.x + icon_r.w + ui_px(22.0);
    let tw = (info.x + info.w - tx).max(0);
    ui_text(c, FontFace::Bold, ui_title_size() * 0.82, ui_text_size(), Rect::new(tx, info.y, tw, ui_px(76.0)), &p.name, 2, false, 0);
    let sub = format!("{} · {} · {}", kind_word(&p), p.version, p.category);
    font_draw_left(c, FontFace::Regular, ui_small_size(), Rect::new(tx, info.y + ui_px(76.0), tw, ui_px(36.0)), &sub, 90);
    let y = info.y + header_h + ui_px(18.0);
    draw_description(c, Rect::new(info.x, y, info.w, desc_h), &p);
    let meta_y = y + desc_h + ui_px(8.0);
    draw_meta(c, Rect::new(info.x, meta_y, info.w, meta_h), &p);
    let ay = if wide {
        info.y + info.h - actions_h
    } else {
        (meta_y + meta_h + ui_px(14.0)).min(info.y + info.h - actions_h)
    };
    let (label, style, enabled) = install_label(&p);
    let action_w = if action_count == 2 { (info.w - action_gap) / 2 } else { info.w };
    let install_r = Rect::new(info.x, ay, action_w, action_h);
    gui_button(c, install_r, IconKind::Download, &label, style);
    if enabled {
        hits_add(&mut ui.hits, install_r, H_INSTALL, 0);
    }
    if !p.repository.is_empty() {
        let r = Rect::new(info.x + action_w + action_gap, ay, action_w, action_h);
        gui_button(c, r, IconKind::Link, tr("Repository QR"), ButtonStyle::Plain);
        hits_add(&mut ui.hits, r, H_QR, 0);
    }
    if !ui.status.is_empty() {
        font_draw_left(c, FontFace::Regular, ui_small_size(), Rect::new(info.x, ay + action_h, info.w, ui_px(44.0)), &ui.status, 90);
    }
    draw_shot(c, ui, shot, &p);
}

fn draw_trust(c: &mut Canvas, ui: &mut PackageStoreUi, aw: i32, ah: i32) {
    let page = store_page(c, aw, ah, tr("Trust this package?"), true, false);
    draw_header_hits(ui, &page, false);
    let area = page.content;
    let Some(p) = ui.selected_package().cloned() else { ui.view = StoreView::List; return };
    let m = ui.manifest.clone().unwrap_or_default();
    let icon_sz = ui_px(160.0);
    let icon = Rect::new(area.x, area.y, icon_sz, icon_sz);
    ui_frame(c, icon, gui_radius(), 255, true, 0);
    ui_frame(c, icon, gui_radius(), 0, false, ui_px(3.0));
    let glyph = match &p.icon {
        StoreIconRef::Glyph(g) => *g,
        StoreIconRef::Image(_) => IconKind::Package,
    };
    ui_icon(c, glyph, rect_inflate(icon, -ui_px(30.0)), 0);
    let tx = icon.x + icon.w + ui_px(22.0);
    let tw = (area.x + area.w - tx).max(0);
    font_draw_left(c, FontFace::Bold, ui_title_size() * 0.8, Rect::new(tx, area.y, tw, ui_px(76.0)), &p.name, 0);
    let who = if m.author.is_empty() { p.author.as_str() } else { m.author.as_str() };
    let ver = if m.version.is_empty() { p.version.as_str() } else { m.version.as_str() };
    let sub = format!("{} {} · {}", tr("by"), who, ver);
    font_draw_left(c, FontFace::Regular, ui_small_size(), Rect::new(tx, area.y + ui_px(78.0), tw, ui_px(38.0)), &sub, 90);
    let mut warn = String::new();
    csnprintf!(
        &mut warn,
        1400,
        // TRANSLATORS: the Package Store's trust page. %s is the package kind (app or widget).
        tr("You MUST trust the author of this package. Grimoire does not review, vet, or verify %s code and is not responsible for any damage, data loss, or malfunction caused by installing or running it."),
        if p.kind == "app" { "app" } else { "widget" }
    );
    let top = area.y + icon_sz + ui_px(34.0);
    draw_description(c, Rect::new(area.x, top, area.w, ui_px(360.0)), &StorePackage { description: warn, ..p.clone() });
    let mut meta = String::new();
    if !p.license.is_empty() {
        csnprintf!(&mut meta, 400, "{} {}", tr("License:"), &p.license);
        font_draw_left(c, FontFace::Regular, ui_small_size(), Rect::new(area.x, top + ui_px(380.0), area.w, ui_px(42.0)), &meta, 80);
    }
    let button = Rect::new(area.x, area.y + area.h - ui_px(110.0), area.w, ui_px(100.0));
    gui_button(c, button, IconKind::Download, tr("I trust this author, continue"), ButtonStyle::Primary);
    hits_add(&mut ui.hits, button, H_TRUST, 0);
    let cancel = Rect::new(area.x, button.y - ui_px(72.0), area.w, ui_px(58.0));
    ui_text(c, FontFace::Regular, ui_small_size(), ui_small_size(), cancel, tr("Back to the package"), 1, true, 120);
    hits_add(&mut ui.hits, cancel, H_BACK, 0);
}

fn draw_qr(c: &mut Canvas, ui: &mut PackageStoreUi, aw: i32, ah: i32) {
    let page = store_page(c, aw, ah, tr("Repository"), true, false);
    draw_header_hits(ui, &page, false);
    let Some(p) = ui.selected_package() else { ui.view = StoreView::Detail; return };
    let area = page.content;
    let side = (area.w.min(area.h) - ui_px(260.0)).max(ui_px(260.0)).min(ui_px(900.0));
    let x = area.x + (area.w - side) / 2;
    let qr = Rect::new(x, area.y + ui_px(50.0), side, side);
    let sym = crate::qr::ui_qr(c, qr, &p.repository, false);
    let ty = if rect_is_empty(sym) { qr.y } else { sym.y + sym.h + ui_px(24.0) };
    font_draw_centered(c, FontFace::Regular, ui_small_size(), Rect::new(area.x, ty, area.w, ui_px(36.0)), tr("Scan with a phone's camera"), 90);
    ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), Rect::new(x, ty + ui_px(52.0), side, ui_px(100.0)), &p.repository, 2, true, 0);
}

fn draw_error(c: &mut Canvas, ui: &mut PackageStoreUi, aw: i32, ah: i32) {
    let page = store_page(c, aw, ah, tr("Package Store"), true, false);
    draw_header_hits(ui, &page, false);
    let area = page.content;
    let title = if ui.index.packages.is_empty() { tr("Couldn't open the Store") } else { tr("Something went wrong") };
    font_draw_centered(c, FontFace::Bold, ui_title_size(), Rect::new(area.x, area.y + ui_px(80.0), area.w, ui_px(90.0)), title, 0);
    ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), Rect::new(area.x, area.y + ui_px(200.0), area.w, ui_px(220.0)), &ui.error, 4, true, 80);
    let r = Rect::new(area.x + area.w / 2 - ui_px(180.0), area.y + ui_px(480.0), ui_px(360.0), ui_px(100.0));
    gui_button(c, r, IconKind::Update, if ui.index.packages.is_empty() { tr("Try again") } else { tr("Back to the Store") }, ButtonStyle::Primary);
    hits_add(&mut ui.hits, r, H_RETRY, 0);
}

fn draw_busy(c: &mut Canvas, ui: &mut PackageStoreUi, aw: i32, ah: i32) {
    let page = store_page(c, aw, ah, tr("Package Store"), false, false);
    draw_header_hits(ui, &page, false);
    let area = page.content;
    ui_icon(c, IconKind::Hourglass, Rect::new(area.x + (area.w - ui_px(180.0)) / 2, area.y + ui_px(160.0), ui_px(180.0), ui_px(180.0)), 80);
    ui_text(c, FontFace::Bold, ui_title_size(), ui_text_size(), Rect::new(area.x, area.y + ui_px(400.0), area.w, ui_px(90.0)), &ui.status, 2, true, 0);
}

pub fn store_draw(a: &mut App) {
    if !a.store.open {
        return;
    }
    let App { screen: c, store: ui, w, h, .. } = a;
    ui.hits.h.clear();
    match ui.view {
        StoreView::List => draw_list(c, ui, *w, *h),
        StoreView::Detail => draw_detail(c, ui, *w, *h),
        StoreView::Trust => draw_trust(c, ui, *w, *h),
        StoreView::Qr => draw_qr(c, ui, *w, *h),
        StoreView::Error => draw_error(c, ui, *w, *h),
        StoreView::Busy => draw_busy(c, ui, *w, *h),
    }
}

// ---- probes and the scenario selector ----

pub fn store_probe(ui: &PackageStoreUi, key: &str) -> Option<String> {
    let view = match ui.view {
        StoreView::List => "list",
        StoreView::Detail => "detail",
        StoreView::Trust => "trust",
        StoreView::Busy => "busy",
        StoreView::Error => "error",
        StoreView::Qr => "qr",
    };
    Some(match key {
        "view" => view.to_string(),
        "open" => (ui.open as i32).to_string(),
        "filter" => ui.filter.to_string(),
        "featured" => ui.index.featured.clone(),
        "selected" => ui.selected.clone().unwrap_or_default(),
        "packages" => ui.index.packages.len().to_string(),
        "visible" => filtered(ui)
            .iter()
            .map(|i| ui.index.packages[*i].id.clone())
            .collect::<Vec<_>>()
            .join(" "),
        "screenshots" => ui
            .selected_package()
            .map_or(0, |p| p.screenshots.len())
            .to_string(),
        "shot" => ui.shot_index.to_string(),
        "shot.loaded" => (!ui.shot.px.is_empty() as i32).to_string(),
        "icon.loaded" => (!ui.icon.image.px.is_empty() as i32).to_string(),
        "list.icons" => ui.list_icons.iter().filter(|icon| !icon.px.is_empty()).count().to_string(),
        "status" => ui.status.clone(),
        "error" => ui.error.clone(),
        "page" => ui.page.to_string(),
        _ => return None,
    })
}

/// Named controls for the scenario runner: `store.package.ID`, `store.install`, `store.trust`, ...
pub fn store_control_point(ui: &PackageStoreUi, name: &str) -> Option<(f32, f32)> {
    let id = match name {
        "close" => H_CLOSE,
        "back" => H_BACK,
        "refresh" => H_REFRESH,
        "install" => H_INSTALL,
        "trust" => H_TRUST,
        "qr" => H_QR,
        "retry" => H_RETRY,
        "prev" => H_PREV,
        "next" => H_NEXT,
        "shot.prev" => H_SHOT_PREV,
        "shot.next" => H_SHOT_NEXT,
        _ => {
            if let Some(n) = name.strip_prefix("filter.") {
                let i = n.parse::<i32>().ok()?;
                return ui.hits.h.iter().find(|h| h.id == H_FILTER && h.arg == i).map(|h| ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32));
            }
            let n = name.strip_prefix("package.").or_else(|| name.strip_prefix("row."))?;
            let i = n
                .parse::<usize>()
                .ok()
                .or_else(|| ui.index.packages.iter().position(|p| p.id == n))?;
            return ui.hits.h.iter().find(|h| h.id == H_CARD && h.arg == i as i32).map(|h| ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32));
        }
    };
    ui.hits.h.iter().find(|h| h.id == id).map(|h| ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> &'static [u8] {
        br#"{
          "version": 1,
          "name": "Test Store",
          "tagline": "Small things",
          "updated": "2026-11-22",
          "featured": "com.example.app",
          "packages": [
            {"id":"com.example.app","name":"Example","version":"1.0.0","type":"app","author":"A",
             "category":"Tools","summary":"An app.","description":"A longer app.","license":"MIT",
             "repository":"https://github.com/example/app","manifest":"packages/com.example.app/package.yaml",
             "icon":"packages/com.example.app/icon.ico","screenshots":["packages/com.example.app/screens/1.png"],
             "tags":["one","two"]}
          ]
        }"#
    }

    #[test]
    fn a_catalog_resolves_its_local_contents() {
        let i = store_index_parse(catalog(), "https://example.org/store/index.json").unwrap();
        assert_eq!(i.name, "Test Store");
        assert_eq!(i.packages.len(), 1);
        let p = &i.packages[0];
        assert_eq!(p.manifest_url, "https://example.org/store/packages/com.example.app/package.yaml");
        assert_eq!(p.screenshots[0].url, "https://example.org/store/packages/com.example.app/screens/1.png");
        assert_eq!(p.screenshots[0].key, "packages/com.example.app/screens/1.png");
        assert!(matches!(p.icon, StoreIconRef::Image(_)));
    }

    #[test]
    fn a_bad_or_empty_catalog_is_refused() {
        assert!(store_index_parse(b"{}", "https://example.org/index.json").is_err());
        assert!(store_index_parse(b"{\"version\":1,\"packages\":[]}", "https://example.org/index.json").is_err());
        assert!(store_index_parse(b"{\"version\":2,\"packages\":[]}", "https://example.org/index.json").is_err());
    }
}
