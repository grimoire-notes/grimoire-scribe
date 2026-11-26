// test_kit_fixtures.rs: the installed packages the package kit's scenarios copy (tests/fixtures/kit/,
// tests/fixtures/kitinput/ and tests/fixtures/kitpanel/) are what tests/kitfix makes now (as test_widget_fixtures.rs
// and test_app_fixtures.rs check the hosts'). `make kit-fixtures` writes them (GRIMOIRE_WRITE_KIT_FIXTURES=<tests/fixtures>).
mod kitfix;

fn dir(root: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures").join(root)
}

/// The fixture folder `root` holds what the writer makes of `packages`, no more; or, with GRIMOIRE_WRITE_KIT_FIXTURES,
/// is written so.
fn committed(root: &str, packages: &[kitfix::Package]) {
    let files = kitfix::scenario_files(packages);
    if let Ok(out) = std::env::var("GRIMOIRE_WRITE_KIT_FIXTURES") {
        // (the generator's own folder only: the folder named holds other fixtures too)
        let out = std::path::Path::new(&out).join(root);
        let _ = std::fs::remove_dir_all(&out);
        for (name, bytes) in files {
            let p = out.join(name);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, bytes).unwrap();
        }
        return;
    }
    fn walk(d: &std::path::Path, base: &std::path::Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            if e.path().is_dir() {
                walk(&e.path(), base, out);
            } else {
                out.push(e.path().strip_prefix(base).unwrap().to_string_lossy().into_owned());
            }
        }
    }
    // (and nothing else lies there: a stale fixture would be copied into a scenario)
    let mut have = Vec::new();
    walk(&dir(root), &dir(root), &mut have);
    have.sort();
    let mut want: Vec<String> = files.iter().map(|f| f.0.to_string()).collect();
    want.sort();
    assert_eq!(have, want, "tests/fixtures/{root} holds what the generator makes, no more: make kit-fixtures");
    for (name, bytes) in files {
        let on_disk = std::fs::read(dir(root).join(&name)).unwrap_or_else(|_| panic!("tests/fixtures/{root}/{name} is missing: make kit-fixtures"));
        assert!(on_disk == bytes, "tests/fixtures/{root}/{name} differs from what tests/kitfix makes: make kit-fixtures");
    }
}

#[test]
fn committed_kit_fixtures_are_what_the_generator_makes() {
    committed("kit", &kitfix::packages());
}

#[test]
fn committed_kit_input_fixtures_are_what_the_generator_makes() {
    committed("kitinput", &kitfix::input_packages());
}

#[test]
fn committed_kit_panel_fixtures_are_what_the_generator_makes() {
    committed("kitpanel", &kitfix::panel_packages());
}

#[test]
fn the_generator_is_deterministic_and_its_modules_are_webassembly() {
    let (one, two) = (kitfix::packages(), kitfix::packages());
    let (three, four) = (kitfix::input_packages(), kitfix::input_packages());
    let (five, six) = (kitfix::panel_packages(), kitfix::panel_packages());
    assert_eq!(one.len(), two.len());
    assert_eq!(three.len(), four.len());
    assert_eq!(five.len(), six.len());
    for (p, q) in one.iter().zip(&two).chain(three.iter().zip(&four)).chain(five.iter().zip(&six)) {
        assert!(p.module == q.module && p.manifest == q.manifest, "{}", p.id);
        assert!(p.module.starts_with(b"\0asm\x01\0\0\0"), "{}", p.id);
        assert!(p.manifest.contains(&format!("id: {}\n", p.id)));
        assert!(grimoire::widgetpkg::widget_id_ok(p.id));
    }
}
