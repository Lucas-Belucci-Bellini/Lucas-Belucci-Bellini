//! `profile-core import manifests` e `import legacy` de ponta a ponta, num
//! PostgreSQL descartável, depois de um `sync github` de arquivo.

mod common;

use common::{TempDb, TempTree, profile_core, profile_core_env, text};
use serde_json::{Value, json};
use sqlx::postgres::PgConnection;

const SHA: &str = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";

fn repo(id: i64, name: &str, extra: Value) -> Value {
    let mut value = json!({
        "id": id, "name": name, "full_name": format!("me/{name}"),
        "owner": {"id": 1, "login": "me", "type": "User"},
        "description": format!("Projeto {name} com descrição."), "private": false, "visibility": "public",
        "fork": false, "archived": false, "homepage": null, "size": 10,
        "pushed_at": "2026-09-20T10:00:00Z", "updated_at": "2026-09-20T10:00:00Z",
    });
    for (key, field) in extra.as_object().unwrap() {
        value[key] = field.clone();
    }
    value
}

fn write(tree: &TempTree, path: &str, value: &Value) {
    let target = tree.0.join(path);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(target, serde_json::to_string_pretty(value).unwrap()).unwrap();
}

fn featured(entries: &[(&str, i64)]) -> Value {
    json!({"projects": entries.iter().map(|(name, order)| json!({
        "name": name, "repository": format!("me/{name}"), "label": format!("RÓTULO {name}"),
        "order": order, "priority": 90, "focus": "foco", "website_required": *order == 2,
    })).collect::<Vec<_>>()})
}

fn root() -> TempTree {
    let tree = TempTree::empty();
    write(&tree, "docs/README_EXCLUDED.json", &json!({"reason": "Omitido.", "repositories": ["excluido"]}));
    write(&tree, "docs/README_FEATURED.json", &featured(&[("alpha", 1), ("beta", 2)]));
    write(
        &tree,
        "docs/README_STACK.json",
        &json!({"tools": [
            {"name": "Git", "category": "Infraestrutura & DevOps", "family": "versionamento", "evidence": "commits"},
            {"name": "Rust", "category": "Frameworks & Web", "family": "núcleo", "evidence": "crates/"},
        ]}),
    );
    write(
        &tree,
        "docs/README_SITES.json",
        &json!({
            "me/beta": "https://beta.example.org", "me/segredo": "https://segredo.example.org",
            "me/fantasma": "https://fantasma.example.org", "me/alpha": {"website": "não é url"},
        }),
    );
    write(
        &tree,
        "repos.json",
        &json!([
            repo(1, "alpha", json!({"homepage": "https://alpha.example.org"})),
            repo(2, "beta", json!({})),
            repo(3, "segredo", json!({"private": true, "visibility": "private"})),
            repo(4, "Projeto-Baluarte", json!({})),
            repo(5, "baluarte-x", json!({})),
            repo(6, "baluarte-obra-segura", json!({})),
            repo(7, "excluido", json!({})),
        ]),
    );
    write(
        &tree,
        "docs/project-catalog.json",
        &json!({"projects": [
            {"repository": "me/alpha", "website_declared": "https://alpha.example.org", "website_status": "verified",
             "website_http_status": 200, "website_source": "github_homepage", "website": "https://alpha.example.org",
             "website_final_url": "https://www.alpha.example.org/"},
            {"repository": "me/beta", "website_declared": "https://beta.example.org", "website_status": "unreachable",
             "website_http_status": 404, "website_source": "manifest"},
            {"repository": "me/sem-site"},
        ]}),
    );
    write(
        &tree,
        "docs/ECOSYSTEM-COMMIT-STATE.json",
        &json!({"schema": 4,
        "repositories": {
            "alpha": {"branch": "main", "sha": SHA, "date": "2026-09-20T10:00:00Z", "message": "m", "url": "u"},
            "beta": {"branch": "main", "error": "HTTP Error 409: Conflict"},
            "fora": {"branch": "main", "empty": true},
        },
        "metrics": {"tracked_commits": 1700, "project_commits": 1650, "monitor_commits": 50}}),
    );
    write(
        &tree,
        "docs/assets/contributions-timeline-data.json",
        &json!({"rows": [
            {"total": 5, "commits": 3, "pull_requests": 1, "issues": 0, "reviews": 0, "repositories": 1, "restricted": 0,
             "period_start": "2026-08-01", "period_end": "2026-08-31"},
            {"total": 2, "commits": 2, "pull_requests": 0, "issues": 0, "reviews": 0, "repositories": 0, "restricted": 0,
             "period_start": "2026-09-01", "period_end": "2026-09-25"},
        ]}),
    );
    tree
}

async fn scalar(conn: &mut PgConnection, sql: &'static str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(conn).await.expect(sql)
}

async fn strings(conn: &mut PgConnection, sql: &'static str) -> Vec<String> {
    sqlx::query_scalar(sql).fetch_all(conn).await.expect(sql)
}

#[tokio::test]
async fn manifestos_e_carga_legada() {
    let Some(db) = TempDb::create().await else { return };
    let url = Some(db.url.as_str());
    assert!(profile_core(&["db", "migrate"], url).status.success());
    let tree = root();
    let sync = profile_core(
        &["sync", "github", "--root", tree.path(), "--input-repos", &tree.join("repos.json"), "--trigger", "test"],
        url,
    );
    assert_eq!(Some(0), sync.status.code(), "{}", text(&sync.stderr));
    let mut conn = db.conn().await;
    assert_eq!(
        vec!["https://alpha.example.org|github_homepage|true"],
        strings(&mut conn, "SELECT url || '|' || source || '|' || is_primary FROM ecosystem.websites ORDER BY id")
            .await,
        "a homepage pública vira site do projeto; a do privado, não"
    );

    // Manifesto que cita projeto inexistente: recusa tudo.
    write(&tree, "docs/README_FEATURED.json", &featured(&[("alpha", 1), ("inexistente", 2)]));
    let refused = profile_core(&["import", "manifests", "--root", tree.path()], url);
    assert_eq!(Some(2), refused.status.code());
    assert!(
        text(&refused.stderr).contains("manifesto recusado: curadoria: me/inexistente"),
        "{}",
        text(&refused.stderr)
    );
    assert_eq!(0, scalar(&mut conn, "SELECT count(*) FROM ecosystem.featured_entries").await);
    assert_eq!(0, scalar(&mut conn, "SELECT count(*) FROM ecosystem.repository_exclusions").await, "nada entrou");

    write(&tree, "docs/README_FEATURED.json", &featured(&[("alpha", 1), ("beta", 2)]));
    let first = profile_core(&["import", "manifests", "--root", tree.path(), "--trigger", "test"], url);
    let (stdout, stderr) = (text(&first.stdout), text(&first.stderr));
    assert_eq!(Some(0), first.status.code(), "{stderr}");
    assert!(
        stdout.contains("exclusões 1 (−0), curadoria 2 (−0), arsenal 2 (−0), sites manuais 1 ativos (0 aposentados)"),
        "{stdout}"
    );
    assert!(stderr.contains("me/segredo (privado)") && stderr.contains("me/fantasma (fora do banco)"), "{stderr}");
    assert!(stderr.contains("site de me/alpha: \"não é url\""), "{stderr}");
    assert_eq!(
        vec!["alpha|1|RÓTULO alpha|false", "beta|2|RÓTULO beta|true"],
        strings(
            &mut conn,
            "SELECT p.slug || '|' || f.position || '|' || f.label || '|' || f.website_required \
             FROM ecosystem.featured_entries f JOIN ecosystem.projects p ON p.id = f.project_id ORDER BY f.position"
        )
        .await
    );
    assert_eq!(
        vec!["Git|1", "Rust|2"],
        strings(&mut conn, "SELECT name || '|' || position FROM ecosystem.stack_tools ORDER BY position").await
    );

    // Carga legada: o que só existe no JSON.
    let args = ["import", "legacy", "--root", tree.path(), "--trigger", "test"];
    let legacy = profile_core_env(&args, url, &[("GH_USER", "me")]);
    let out = text(&legacy.stdout);
    assert_eq!(Some(0), legacy.status.code(), "{}", text(&legacy.stderr));
    assert!(out.contains("resumos editoriais        2 gravados"), "Projeto-Baluarte e baluarte-obra-segura: {out}");
    assert!(out.contains("sobreposições de status   2 gravadas"), "{out}");
    assert!(out.contains("sites do catálogo         0 criados, 2 checagens iniciais (1 no ar no catálogo)"), "{out}");
    assert!(out.contains("estado do monitor         2 de 3"), "{out}");
    assert!(out.contains("amostras importadas       17 de 17"), "{out}");
    assert_eq!(
        vec!["https://alpha.example.org|verified|true", "https://beta.example.org|unreachable|false"],
        strings(
            &mut conn,
            "SELECT url || '|' || outcome || '|' || is_online FROM ecosystem.website_status_current ORDER BY url"
        )
        .await
    );
    assert_eq!(
        vec!["baluarte-x", "projeto-baluarte"],
        strings(
            &mut conn,
            "SELECT slug FROM ecosystem.projects WHERE lifecycle_override = 'in_development' ORDER BY slug"
        )
        .await
    );
    let tracked: String = sqlx::query_scalar(
        "SELECT value::text || '|' || provenance FROM ecosystem.metric_latest WHERE metric_key = 'ecosystem.commits.tracked_total'",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!("1700|legacy_import", tracked);

    // Idempotente.
    let again = text(&profile_core_env(&args, url, &[("GH_USER", "me")]).stdout);
    assert!(again.contains("0 criados, 0 checagens") && again.contains("amostras importadas       0 de 17"), "{again}");

    // O manifesto perde beta: sai da curadoria e o site manual é aposentado (as checagens ficam).
    write(&tree, "docs/README_FEATURED.json", &featured(&[("alpha", 1)]));
    write(&tree, "docs/README_SITES.json", &json!({}));
    let second = text(&profile_core(&["import", "manifests", "--root", tree.path()], url).stdout);
    assert!(
        second.contains("curadoria 1 (−1)") && second.contains("sites manuais 0 ativos (1 aposentados)"),
        "{second}"
    );
    assert_eq!(
        1,
        scalar(&mut conn, "SELECT count(*) FROM ecosystem.website_checks c JOIN ecosystem.websites w ON w.id = c.website_id WHERE w.url = 'https://beta.example.org' AND w.retired_at IS NOT NULL").await
    );
}
