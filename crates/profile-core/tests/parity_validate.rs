//! `profile-core validate` contra `tests/fixtures/parity/validators.json`:
//! cada cenário é uma árvore sintética do perfil sobre a qual os
//! `scripts/validate_*.py` rodaram. O binário roda sobre a mesma árvore e tem
//! de imprimir o mesmo (stdout e stderr) e sair com o mesmo código — ou, onde
//! o script saiu com traceback, sair com `1` pela mesma exceção, com o mesmo
//! texto. O `validate catalog` confere a lista de erros do
//! `validate_profile.py` (sem os do YAML do workflow).

mod common;

use std::path::{Path, PathBuf};

use common::{profile_core, text};
use profile_core::validate::{self, BADGES_OFFLINE_NOTE, BADGES_ONLINE_NOTE};
use serde_json::{Map, Value};

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parity/validators.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

struct Root(PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write(root: &Path, files: &Map<String, Value>) {
    for (path, content) in files {
        let target = root.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let bytes = match content {
            Value::String(text) => text.as_bytes().to_vec(),
            Value::Object(raw) => {
                let hex = raw["hex"].as_str().unwrap();
                (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect()
            }
            other => panic!("conteúdo inesperado: {other}"),
        };
        std::fs::write(target, bytes).unwrap();
    }
}

fn materialize(index: usize, base: &Map<String, Value>, changes: &Value) -> Root {
    let dir = std::env::temp_dir().join(format!("parity-validate-{}-{index}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let root = dir.canonicalize().unwrap();
    write(&root, base);
    write(&root, changes["write"].as_object().unwrap());
    for path in changes["delete"].as_array().unwrap().iter().chain(changes["mkdir"].as_array().unwrap()) {
        let _ = std::fs::remove_file(root.join(path.as_str().unwrap()));
    }
    for path in changes["mkdir"].as_array().unwrap() {
        std::fs::create_dir_all(root.join(path.as_str().unwrap())).unwrap();
    }
    Root(root)
}

/// Compara uma execução do binário com o que o script fez.
fn compare(name: &str, label: &str, expected: &Value, output: &std::process::Output, root: &str) -> Option<String> {
    let clean = |bytes: &[u8]| text(bytes).replace(root, "<root>");
    let (stdout, stderr) = (clean(&output.stdout), clean(&output.stderr));
    let code = output.status.code().unwrap_or(-1);
    let expected_code = expected["code"].as_i64().unwrap() as i32;
    let expected_stdout = expected["stdout"].as_str().unwrap().replace(BADGES_ONLINE_NOTE, BADGES_OFFLINE_NOTE);
    let mut problems = Vec::new();
    if code != expected_code {
        problems.push(format!("código {code}, o Python saiu com {expected_code}"));
    }
    if stdout != expected_stdout {
        problems.push(format!("stdout\n    python: {expected_stdout:?}\n    rust:   {stdout:?}"));
    }
    match (expected.get("crash"), expected.get("stderr")) {
        (Some(crash), _) => {
            let prefix = format!("profile-core: erro: validate {label}: ");
            let got = stderr.trim_end().strip_prefix(&prefix).unwrap_or(&stderr);
            if got != crash.as_str().unwrap() {
                problems.push(format!("traceback\n    python: {crash}\n    rust:   {stderr:?}"));
            }
        }
        (None, Some(expected_stderr)) if expected_stderr.as_str().unwrap() != stderr => {
            problems.push(format!("stderr\n    python: {expected_stderr}\n    rust:   {stderr:?}"));
        }
        _ => {}
    }
    (!problems.is_empty()).then(|| format!("{name} · {label}: {}", problems.join("; ")))
}

#[test]
fn validadores_iguais_aos_do_python_em_cada_cenario() {
    let data = fixture();
    let base = data["base"].as_object().unwrap();
    let scenarios = data["scenarios"].as_array().unwrap();
    assert!(scenarios.len() >= 45, "o fixture perdeu cenários");
    let mut failures = Vec::new();
    let mut compared = 0;
    for (index, scenario) in scenarios.iter().enumerate() {
        let name = scenario["name"].as_str().unwrap();
        let expected = &scenario["expected"];
        let root = materialize(index, base, &scenario["changes"]);
        let root_text = root.0.to_str().unwrap().to_string();
        let before = root.0.join("README.before.md");
        let after = root.0.join("README.md");
        let runs: [(&str, Vec<&str>); 5] = [
            ("readme", vec!["readme", "--before", before.to_str().unwrap(), "--after", after.to_str().unwrap()]),
            ("exclusions", vec!["exclusions", "--root", &root_text]),
            ("links", vec!["links", "--root", &root_text]),
            ("visual", vec!["visual", "--root", &root_text]),
            ("badges", vec!["badges", "--offline", "--root", &root_text]),
        ];
        for (label, args) in runs {
            let mut command = vec!["validate"];
            command.extend(args);
            let output = profile_core(&command, None);
            failures.extend(compare(name, label, &expected[label], &output, &root_text));
            compared += 1;
        }

        let python = &expected["catalog"];
        let rust = validate::catalog(&root.0);
        let same = match (&rust, python.get("crash"), python.get("errors")) {
            (Err(crash), Some(line), _) => {
                format!("{}: {}", crash.kind, crash.message).replace(&root_text, "<root>") == *line
            }
            (Ok(errors), None, Some(list)) => {
                // O Python percorre os excluídos de um `set`: a ordem não é dele.
                let mut python: Vec<String> =
                    list.as_array().unwrap().iter().map(|e| e.as_str().unwrap().to_string()).collect();
                let mut rust: Vec<String> = errors.iter().map(|e| e.replace(&root_text, "<root>")).collect();
                python.sort();
                rust.sort();
                python == rust
            }
            _ => false,
        };
        if !same {
            failures.push(format!("{name} · catalog\n    python: {python}\n    rust:   {rust:?}"));
        }
        compared += 1;
    }
    assert!(failures.is_empty(), "{} de {compared} divergem:\n\n{}", failures.len(), failures.join("\n\n"));
}
