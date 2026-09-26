//! Os SVGs de `assets/` contra `tests/fixtures/parity/assets.json`: as funções
//! de desenho do `lang_stats.py` e do `profile_cards.py` chamadas sobre dados
//! de borda; o Rust tem de produzir os mesmos bytes.

use std::path::Path;

use ecosystem_domain::timestamps::{PyLocalDatetime, py_fromisoformat_local};
use profile_render::cards;
use profile_render::lang_stats::{self, LangStats};
use serde_json::Value;

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parity/assets.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn pairs(value: &Value) -> Vec<(String, i64)> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| (pair[0].as_str().unwrap().to_string(), pair[1].as_i64().unwrap()))
        .collect()
}

fn stats(input: &Value) -> LangStats {
    let generated = match py_fromisoformat_local(&input["generated"].as_str().unwrap().replace('Z', "+00:00")) {
        PyLocalDatetime::Aware { local, .. } => local,
        other => panic!("data do fixture: {other:?}"),
    };
    LangStats {
        per_lang: pairs(&input["per_lang"]),
        per_ext: pairs(&input["per_ext"]),
        repo_count: input["repo_count"].as_i64().unwrap(),
        arquivos_total: input["arquivos_total"].as_i64().unwrap(),
        total_bytes: input["total_bytes"].as_i64().unwrap(),
        generated,
    }
}

fn check(failures: &mut Vec<String>, label: String, expected: &Value, actual: &str) {
    let expected = expected.as_str().unwrap();
    if expected != actual {
        let line = expected.lines().zip(actual.lines()).position(|(e, a)| e != a).unwrap_or(0);
        failures.push(format!(
            "{label}: linha {}\n python: {:?}\n rust:   {:?}",
            line + 1,
            expected.lines().nth(line),
            actual.lines().nth(line)
        ));
    }
}

#[test]
fn svgs_iguais_aos_do_python() {
    let data = fixture();
    let mut failures = Vec::new();
    for case in data["lang_stats"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let input = stats(&case["input"]);
        check(
            &mut failures,
            format!("{name}: lang-stats.svg"),
            &case["expected"]["lang_stats"],
            &lang_stats::build_svg(&input),
        );
        check(
            &mut failures,
            format!("{name}: profile-top-langs.svg"),
            &case["expected"]["top_langs"],
            &lang_stats::build_top_langs_svg(&input),
        );
    }
    for case in data["cards"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let input = case["input"].as_object().unwrap();
        let expected = &case["expected"];
        check(&mut failures, format!("{name}: stats"), &expected["stats"], &cards::stats_svg(input));
        check(&mut failures, format!("{name}: streak"), &expected["streak"], &cards::streak_svg(input));
        check(&mut failures, format!("{name}: trophies"), &expected["trophies"], &cards::trophies_svg(input));
        check(&mut failures, format!("{name}: projects"), &expected["projects"], &cards::projects_svg());
    }
    assert!(failures.is_empty(), "{} SVG(s) divergem:\n\n{}", failures.len(), failures.join("\n\n"));
}

#[test]
fn extensao_carimbo_e_peso_como_no_python() {
    let data = fixture();
    for case in data["extensao"].as_array().unwrap() {
        let path = case[0].as_str().unwrap();
        assert_eq!(case[1].as_str().map(str::to_string), lang_stats::extension(path), "extensao({path:?})");
    }
    for case in data["mesmo_conteudo"].as_array().unwrap() {
        let (new, old) = (case[0].as_str().unwrap(), case[1].as_str().unwrap());
        assert_eq!(case[2].as_bool().unwrap(), lang_stats::same_content(new, old), "mesmo_conteudo({new:?}, {old:?})");
    }
    for case in data["human"].as_array().unwrap() {
        assert_eq!(case[1].as_str().unwrap(), lang_stats::human(case[0].as_i64().unwrap()));
    }
}
