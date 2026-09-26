//! O catálogo do Rust sobre a raiz sintética do teste golden do Python
//! (`tests/fixtures/profile/input`) tem de sair **byte a byte** igual ao
//! `expected/project-catalog.json` que o `update_profile.py` gerou.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use catalog::{Curadoria, build, candidates, discover, exclude, inventory, load_checks_fixture, render};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/profile")
}

fn catalog_text(checks_file: Option<&str>) -> String {
    let input = fixture().join("input");
    let raw = inventory::load_local_repositories(&input.join("repos.json")).unwrap();
    let repos = exclude(inventory::facts(&raw).unwrap(), &catalog::load_excluded(&input).unwrap());
    let sites = discover(&repos, &catalog::load_site_overrides(&input));
    let checks = match checks_file {
        Some(file) => load_checks_fixture(&input.join(file), &candidates(&sites)).unwrap(),
        None => HashMap::new(),
    };
    let now = catalog::parse_now("2026-09-25T12:00:00Z").unwrap();
    let presentations =
        catalog::presentations(&repos, &sites, &checks, &Curadoria::load(&input).unwrap(), now).unwrap();
    render(&build(&presentations))
}

#[test]
fn catalogo_igual_ao_do_python() {
    let expected = std::fs::read_to_string(fixture().join("expected/project-catalog.json")).unwrap();
    let actual = catalog_text(Some("site-checks.json"));
    if expected != actual {
        let line = expected.lines().zip(actual.lines()).position(|(e, a)| e != a).unwrap_or(0);
        panic!(
            "catálogo difere do golden na linha {}:\n python: {:?}\n rust:   {:?}",
            line + 1,
            expected.lines().nth(line),
            actual.lines().nth(line)
        );
    }
}

#[test]
fn fixture_sem_resultado_para_uma_url_e_erro() {
    let input = fixture().join("input");
    let error = load_checks_fixture(&input.join("site-checks.json"), &["https://nova.example.org".into()]).unwrap_err();
    assert_eq!("site check fixture has no result for: https://nova.example.org", error.to_string());
}
