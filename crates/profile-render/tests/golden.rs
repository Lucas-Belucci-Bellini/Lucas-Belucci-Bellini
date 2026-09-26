//! O README e o snapshot do Rust sobre a raiz sintética do teste golden do
//! Python (`tests/fixtures/profile/input`) têm de sair **byte a byte** iguais
//! ao `expected/` que o `update_profile.py` gerou.

use std::path::{Path, PathBuf};

use catalog::{Curadoria, candidates, discover, exclude, inventory, load_checks_fixture};
use profile_render::snapshot::generated_at;
use profile_render::{Inputs, LanguageMap, Profile};

const NOW: &str = "2026-09-25T12:00:00Z";

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/profile")
}

fn assert_same(name: &str, expected: &str, actual: &str) {
    if expected != actual {
        let line = expected
            .lines()
            .zip(actual.lines())
            .position(|(e, a)| e != a)
            .unwrap_or_else(|| expected.lines().count().min(actual.lines().count()));
        panic!(
            "{name} difere do golden na linha {}:\n python: {:?}\n rust:   {:?}",
            line + 1,
            expected.lines().nth(line),
            actual.lines().nth(line)
        );
    }
}

#[test]
fn readme_e_snapshot_iguais_aos_do_python() {
    let input = fixture().join("input");
    let raw = inventory::load_local_repositories(&input.join("repos.json")).unwrap();
    let excluded = catalog::load_excluded(&input).unwrap();
    let repos = exclude(inventory::facts(&raw).unwrap(), &excluded);
    let languages: LanguageMap = repos
        .iter()
        .map(|repo| {
            (repo.full_name.clone(), inventory::load_local_languages(&input.join("languages"), &repo.full_name))
        })
        .collect();
    let sites = discover(&repos, &catalog::load_site_overrides(&input));
    let checks = load_checks_fixture(&input.join("site-checks.json"), &candidates(&sites)).unwrap();
    let now = catalog::parse_now(NOW).unwrap();
    let presentations =
        catalog::presentations(&repos, &sites, &checks, &Curadoria::load(&input).unwrap(), now).unwrap();
    let featured = catalog::load_json_object(&input.join("docs/README_FEATURED.json")).unwrap();
    let stack = catalog::load_json_object(&input.join("docs/README_STACK.json")).unwrap();
    let stamp = generated_at(&repos, now.naive_utc()).unwrap();

    let profile = Profile::new(Inputs {
        repos: &repos,
        languages: &languages,
        presentations: &presentations,
        excluded: excluded.len(),
        now,
        featured: &featured,
        stack: &stack,
        generated_at: &stamp,
    });
    let template = std::fs::read_to_string(input.join("README.md")).unwrap();
    let expected = fixture().join("expected");
    assert_same(
        "README.md",
        &std::fs::read_to_string(expected.join("README.md")).unwrap(),
        &profile.readme(&template).unwrap(),
    );
    assert_same(
        "profile-snapshot.svg",
        &std::fs::read_to_string(expected.join("profile-snapshot.svg")).unwrap(),
        &profile.snapshot_svg(),
    );
}
