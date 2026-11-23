// The Kindle notebook reader (rust/src/nbk.rs): container, fragments and strokes. The real sample
// is not part of the repository (it is the owner's writing): NBK_SAMPLE names one for a local run.


#[test]
fn a_real_kindle_notebook_reads() {
    let Ok(path) = std::env::var("NBK_SAMPLE") else { return };
    let data = std::fs::read(&path).expect("the sample reads");
    let nb = grimoire::nbk::nbk_notebook(&data).expect("the notebook reads");
    eprintln!("title={:?} pages={} strokes={} points={}", nb.title, nb.pages.len(), nb.strokes, nb.points);
    for (i, p) in nb.pages.iter().enumerate() {
        eprintln!("  page {}: {}x{} {} stroke(s)", i + 1, p.canvas_w, p.canvas_h, p.strokes.len());
        if let Some(s) = p.strokes.first() {
            eprintln!("    first: brush={} thickness={} points={} at {:?}", s.brush, s.thickness, s.points.len(), s.points.first());
        }
    }
    assert!(!nb.pages.is_empty());
}

// The whole way through the import child: a folder holding the sample's `nbk` becomes a notebook with its
// pages of ink, and the folder moves to import/done.
#[test]
fn a_real_kindle_notebook_imports_as_a_notebook() {
    let Ok(path) = std::env::var("NBK_SAMPLE") else { return };
    let tmp = std::env::temp_dir().join(format!("grimoire-nbk-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let root = tmp.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    let folder = root.join("import").join("My Scribe notebook");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::copy(&path, folder.join("nbk")).unwrap();
    assert_eq!(grimoire::prelude::store_init(root.to_str().unwrap()), 0);
    grimoire::prelude::store_set_device(300.0, 1860, 2480);
    assert_eq!(grimoire::bgwork_template::bg_import_folder(folder.to_str().unwrap(), 300.0), 0);
    let mut list = Vec::new();
    grimoire::prelude::store_list(&mut list);
    assert_eq!(list.len(), 1);
    let nb = &list[0];
    assert_eq!(nb.npages(), 2, "the notebook's own reading order, not its template pages");
    let mut page: Option<Box<grimoire::prelude::Page>> = None;
    assert_eq!(grimoire::prelude::store_load_page(nb, 0, nb.page_w, nb.page_h, &mut page), 0);
    let page = page.expect("page 1 reads");
    assert!(!page.strokes.is_empty(), "page 1 has ink");
    let points: usize = page.strokes.iter().map(|s| s.pts.len()).sum();
    assert!(points > 1000, "page 1's ink has its points ({points})");
    assert!(!folder.exists(), "the folder moved to import/done");
    let _ = std::fs::remove_dir_all(&tmp);
}
