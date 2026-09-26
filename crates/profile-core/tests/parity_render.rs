//! O `render readme` contra `tests/fixtures/parity/render.json`: cada cenário
//! é uma raiz sintética inteira sobre a qual o `update_profile.py` rodou; o
//! Rust roda sobre a mesma raiz e tem de produzir a mesma saída padrão e os
//! mesmos arquivos, byte a byte — ou sair com o mesmo código, pela mesma
//! exceção, sem gravar nada.

mod common;

use std::path::{Path, PathBuf};

use common::{profile_core_env, text};
use serde_json::Value;

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parity/render.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

struct Root(PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn materialize(index: usize, files: &serde_json::Map<String, Value>) -> Root {
    let root = std::env::temp_dir().join(format!("parity-render-{}-{index}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, content) in files {
        let target = root.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, content.as_str().unwrap()).unwrap();
    }
    std::fs::create_dir_all(root.join("languages")).unwrap();
    Root(root)
}

fn first_difference(expected: &str, actual: &str) -> String {
    let line = expected.lines().zip(actual.lines()).position(|(e, a)| e != a);
    match line {
        Some(line) => format!(
            "linha {}:\n python: {:?}\n rust:   {:?}",
            line + 1,
            expected.lines().nth(line),
            actual.lines().nth(line)
        ),
        None => format!("tamanhos {} × {} (fim de linha ou linhas a mais)", expected.len(), actual.len()),
    }
}

#[test]
fn render_igual_ao_python_em_cada_cenario() {
    let data = fixture();
    let cases = data["cases"].as_array().unwrap();
    assert!(cases.len() >= 17, "o fixture perdeu cenários");
    let mut failures = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let name = case["name"].as_str().unwrap();
        let expected = &case["expected"];
        let root = materialize(index, case["files"].as_object().unwrap());
        let root_text = root.0.to_str().unwrap().to_string();
        let mut args = vec!["render".to_string(), "readme".into(), "--root".into(), root_text.clone()];
        args.extend(case["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap().replace("{root}", &root_text)));
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = profile_core_env(&args, None, &[]);
        let (stdout, stderr) = (text(&output.stdout), text(&output.stderr));

        let exit = expected["exit"].as_i64().unwrap();
        if Some(exit as i32) != output.status.code() {
            failures.push(format!("{name}: código {:?}, o Python saiu com {exit}\n{stderr}", output.status.code()));
            continue;
        }
        if expected["stdout"].as_str().unwrap() != stdout {
            failures.push(format!(
                "{name}: saída padrão difere — {}",
                first_difference(expected["stdout"].as_str().unwrap(), &stdout)
            ));
        }
        for (path, content) in expected["files"].as_object().unwrap() {
            let actual = std::fs::read_to_string(root.0.join(path)).unwrap_or_default();
            let content = content.as_str().unwrap();
            if content != actual {
                failures.push(format!("{name}: {path} difere — {}", first_difference(content, &actual)));
            }
        }
        if let Some(kind) = expected["crash"].as_str()
            && !stderr.contains(kind)
        {
            failures.push(format!("{name}: o Python quebrou com {kind}; o Rust disse {stderr:?}"));
        }
        if let Some(message) = expected["message"].as_str() {
            let message = message.replace("{root}", &root_text);
            if !stderr.contains(&message) {
                failures.push(format!("{name}: esperava {message:?} no stderr; veio {stderr:?}"));
            }
        }
        if exit != 0 && root.0.join("assets").exists() {
            failures.push(format!("{name}: gravou assets/ antes de falhar"));
        }
    }
    assert!(failures.is_empty(), "{} cenário(s) divergem:\n\n{}", failures.len(), failures.join("\n\n"));
}
