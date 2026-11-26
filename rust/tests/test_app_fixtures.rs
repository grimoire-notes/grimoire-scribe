// test_app_fixtures.rs: the installed app packages the scenarios copy (tests/fixtures/apphost/) are what
// tests/appfix makes now (as test_widget_fixtures.rs checks the widgets'). `make app-fixtures` writes them
// (GRIMOIRE_WRITE_APP_FIXTURES=<tests/fixtures>).
mod appfix;

const ROOT: &str = "apphost";

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures").join(ROOT)
}

#[test]
fn committed_app_fixtures_are_what_the_generator_makes() {
    let files = appfix::scenario_files(&appfix::packages());
    if let Ok(out) = std::env::var("GRIMOIRE_WRITE_APP_FIXTURES") {
        // (the generator's own folder only: the folder named holds other fixtures too)
        let out = std::path::Path::new(&out).join(ROOT);
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
    walk(&dir(), &dir(), &mut have);
    have.sort();
    let mut want: Vec<String> = files.iter().map(|f| f.0.to_string()).collect();
    want.sort();
    assert_eq!(have, want, "tests/fixtures/{ROOT} holds what the generator makes, no more: make app-fixtures");
    for (name, bytes) in files {
        let on_disk = std::fs::read(dir().join(&name)).unwrap_or_else(|_| panic!("tests/fixtures/{ROOT}/{name} is missing: make app-fixtures"));
        assert!(on_disk == bytes, "tests/fixtures/{ROOT}/{name} differs from what tests/appfix makes: make app-fixtures");
    }
}

#[test]
fn the_generator_is_deterministic_and_its_modules_are_webassembly() {
    assert_eq!(appfix::counts(), appfix::counts());
    for p in appfix::packages() {
        assert!(p.module.starts_with(b"\0asm\x01\0\0\0"), "{}", p.id);
        assert!(p.manifest.contains(&format!("id: {}\n", p.id)));
        assert!(grimoire::widgetpkg::widget_id_ok(p.id));
    }
}
