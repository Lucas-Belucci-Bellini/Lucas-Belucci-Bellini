//! Fatos de repositório vindos do GitHub — o recorte que as regras usam.

use serde::Deserialize;

/// Um repositório como a API REST do GitHub o descreve (só os campos usados).
///
/// Os campos opcionais seguem a verdade do Python: `None` e `""` são
/// "ausentes" onde o código Python fazia `repo.get(...) or ...`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct RepoFacts {
    /// Nome curto (`Veritas`).
    #[serde(default)]
    pub name: String,
    /// `owner/nome`.
    #[serde(default)]
    pub full_name: String,
    /// Descrição pública.
    #[serde(default)]
    pub description: Option<String>,
    /// Visibilidade privada.
    #[serde(default)]
    pub private: bool,
    /// É fork.
    #[serde(default)]
    pub fork: bool,
    /// Está arquivado.
    #[serde(default)]
    pub archived: bool,
    /// `homepage` declarada no GitHub.
    #[serde(default)]
    pub homepage: Option<String>,
    /// Último push (ISO 8601, como o GitHub devolve).
    #[serde(default)]
    pub pushed_at: Option<String>,
    /// Última atualização (ISO 8601).
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Tamanho em KB.
    #[serde(default)]
    pub size: Option<u64>,
}

impl RepoFacts {
    /// `repo.get("pushed_at") or repo.get("updated_at")` do Python.
    pub fn activity_timestamp(&self) -> Option<&str> {
        match self.pushed_at.as_deref() {
            Some(value) if !value.is_empty() => Some(value),
            _ => self.updated_at.as_deref(),
        }
    }
}
