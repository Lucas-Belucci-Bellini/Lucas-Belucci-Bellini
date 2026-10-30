(() => {
  "use strict";

  const catalogUrl = "data/projects.json";
  const grid = document.querySelector("#project-grid");
  const searchInput = document.querySelector("#project-search");
  const categorySelect = document.querySelector("#category-filter");
  const statusSelect = document.querySelector("#status-filter");
  const count = document.querySelector("#project-count");
  const message = document.querySelector("#catalog-message");
  const year = document.querySelector("#current-year");
  const stackOverview = document.querySelector("#stack-overview");
  const stackUpdated = document.querySelector("#stack-updated");
  const languageList = document.querySelector("#language-list");
  const languageSearch = document.querySelector("#language-search");
  const stackSearch = document.querySelector("#stack-search");
  const stackCategory = document.querySelector("#stack-category");
  const toolGrid = document.querySelector("#tool-grid");
  const allowedStatuses = new Set(["🟢 Active", "🟡 In Development", "🔵 Experimental", "🟣 Academic"]);

  if (year) year.textContent = String(new Date().getFullYear());

  let projects = [];

  const make = (tag, className, text) => {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  };

  const safeHttpsUrl = (value) => {
    if (typeof value !== "string" || !value.trim()) return null;
    try {
      const url = new URL(value);
      return url.protocol === "https:" ? url.href : null;
    } catch {
      return null;
    }
  };

  const safeGitHubUrl = (value) => {
    const safe = safeHttpsUrl(value);
    if (!safe) return null;
    try {
      const url = new URL(safe);
      const parts = url.pathname.split("/").filter(Boolean);
      return url.hostname === "github.com" && parts.length === 2 && parts.every((part) => !part.includes(".")) ? safe : null;
    } catch {
      return null;
    }
  };

  function createProjectCard(project) {
    const card = make("article", "project-card");
    const top = make("div", "project-topline");
    top.append(make("span", "project-category", project.category_label || project.category || "Categoria não informada"));
    if (allowedStatuses.has(project.status)) {
      top.append(make("span", "project-status", project.status));
    }
    card.append(top);
    card.append(make("h3", "", typeof project.name === "string" && project.name.trim() ? project.name : "Projeto sem nome"));
    const description = typeof project.description === "string" && project.description.trim()
      ? project.description
      : "Descrição não informada.";
    card.append(make("p", "", description));

    const actions = make("div", "project-actions");
    const github = safeGitHubUrl(project.github);
    const website = safeHttpsUrl(project.website);
    if (website) {
      const siteLink = make("a", "site-link", "Site verificado ↗");
      siteLink.href = website;
      siteLink.target = "_blank";
      siteLink.rel = "noopener noreferrer";
      actions.append(siteLink);
    }
    if (github) {
      const codeLink = make("a", "", "Código-fonte ↗");
      codeLink.href = github;
      codeLink.target = "_blank";
      codeLink.rel = "noopener noreferrer";
      actions.append(codeLink);
    }
    if (actions.childElementCount) card.append(actions);
    return card;
  }

  function render() {
    if (!grid || !searchInput || !categorySelect || !statusSelect || !count) return;
    const query = (searchInput.value || "").trim().toLocaleLowerCase("pt-BR");
    const category = categorySelect.value;
    const status = statusSelect.value;
    const filtered = projects.filter((project) => {
      const text = [project.name, project.description, project.category, project.category_label, project.status]
        .filter((value) => typeof value === "string").join(" ").toLocaleLowerCase("pt-BR");
      return (!query || text.includes(query))
        && (category === "all" || project.category === category)
        && (status === "all" || project.status === status);
    });

    grid.replaceChildren();
    if (!filtered.length) {
      grid.append(make("div", "empty-card", "Nenhum projeto corresponde aos filtros atuais. Tente outro termo ou opção."));
    } else {
      const fragment = document.createDocumentFragment();
      filtered.forEach((project) => fragment.append(createProjectCard(project)));
      grid.append(fragment);
    }
    count.textContent = String(filtered.length).padStart(2, "0") + " / " + String(projects.length).padStart(2, "0") + " projetos";
  }

  async function loadCatalog() {
    if (!grid || !count || !message || !categorySelect || !statusSelect) return;
    grid.setAttribute("aria-busy", "true");
    try {
      const response = await fetch(catalogUrl, { headers: { Accept: "application/json" } });
      if (!response.ok) throw new Error("O catálogo respondeu com HTTP " + response.status + ".");
      const data = await response.json();
      if (!data || !Array.isArray(data.projects)) throw new Error("O formato do catálogo é inválido.");

      // Fail closed: private, explicitly hidden, or invalid-GitHub entries are never rendered.
      projects = data.projects.filter((project) =>
        project && typeof project === "object"
        && project.private !== true
        && project.github_visible !== false
        && safeGitHubUrl(project.github)
      );

      const categories = [...new Set(projects.map((project) => project.category)
        .filter((value) => typeof value === "string" && value.trim()))]
        .sort((a, b) => a.localeCompare(b, "pt-BR"));
      categories.forEach((category) => {
        const option = make("option", "", category);
        option.value = category;
        categorySelect.append(option);
      });

      const statuses = [...new Set(projects.map((project) => project.status)
        .filter((value) => allowedStatuses.has(value)))];
      statuses.sort((a, b) => a.localeCompare(b, "pt-BR"));
      statuses.forEach((value) => {
        const option = make("option", "", value);
        option.value = value;
        statusSelect.append(option);
      });

      if (!projects.length) throw new Error("O catálogo não contém projetos públicos elegíveis para exibição.");
      message.hidden = true;
      render();
    } catch (error) {
      grid.replaceChildren(make("div", "empty-card", "Não foi possível carregar o catálogo de projetos."));
      count.textContent = "Catálogo indisponível";
      message.hidden = false;
      message.textContent = "Confira se data/projects.json está disponível e contém JSON válido. "
        + (error instanceof Error ? error.message : "Erro desconhecido.");
    } finally {
      grid.removeAttribute("aria-busy");
    }
  }

  
  let stackInventory = null;

  function renderLanguages() {
    if (!languageList || !stackInventory) return;
    const query = (languageSearch?.value || "").trim().toLocaleLowerCase("pt-BR");
    const rows = stackInventory.languages.filter((language) =>
      language.name.toLocaleLowerCase("pt-BR").includes(query)
    );
    languageList.replaceChildren();
    if (!rows.length) {
      languageList.append(make("p", "stack-empty", "Nenhuma linguagem corresponde à busca."));
      return;
    }
    const fragment = document.createDocumentFragment();
    rows.forEach((language) => {
      const row = make("article", "language-row");
      const heading = make("div", "language-row-heading");
      heading.append(make("span", "language-rank", String(language.rank).padStart(2, "0")));
      heading.append(make("strong", "language-name", language.name));
      heading.append(make("span", "language-share", Number(language.share).toFixed(2) + "%"));
      row.append(heading);
      const track = make("div", "language-track");
      const bar = make("span", "language-bar");
      const share = Math.max(0, Math.min(100, Number(language.share) || 0));
      bar.style.width = share + "%";
      track.append(bar);
      row.append(track);
      row.append(make("p", "language-meta", language.size + " de código · presente em "
        + language.repositories + " repositório(s) público(s)"));
      fragment.append(row);
    });
    languageList.append(fragment);
  }

  function renderTools() {
    if (!toolGrid || !stackInventory) return;
    const query = (stackSearch?.value || "").trim().toLocaleLowerCase("pt-BR");
    const category = stackCategory?.value || "all";
    const tools = stackInventory.tools.filter((tool) => {
      const haystack = [tool.name, tool.family, tool.category, tool.evidence]
        .filter((value) => typeof value === "string")
        .join(" ")
        .toLocaleLowerCase("pt-BR");
      return (!query || haystack.includes(query))
        && (category === "all" || tool.category === category);
    });
    toolGrid.replaceChildren();
    if (!tools.length) {
      toolGrid.append(make("p", "stack-empty", "Nenhuma tecnologia corresponde aos filtros atuais."));
      return;
    }
    const fragment = document.createDocumentFragment();
    tools.forEach((tool) => {
      const card = make("article", "tool-card");
      const top = make("div", "tool-card-top");
      top.append(make("span", "tool-category", tool.category));
      top.append(make("span", "tool-family", tool.family));
      card.append(top);
      card.append(make("h4", "", tool.name));
      card.append(make("p", "tool-evidence", tool.evidence));
      if (Array.isArray(tool.projects) && tool.projects.length) {
        const evidence = make("div", "tool-projects");
        evidence.append(make("span", "tool-projects-label", "EVIDÊNCIA EM PROJETOS PÚBLICOS"));
        tool.projects.forEach((project) => {
          const links = make("div", "tool-project-links");
          const github = safeGitHubUrl(project.github);
          if (github) {
            const codeLink = make("a", "", project.name + " · código ↗");
            codeLink.href = github;
            codeLink.target = "_blank";
            codeLink.rel = "noopener noreferrer";
            links.append(codeLink);
          }
          const website = safeHttpsUrl(project.website);
          if (website) {
            const siteLink = make("a", "", "site ↗");
            siteLink.href = website;
            siteLink.target = "_blank";
            siteLink.rel = "noopener noreferrer";
            links.append(siteLink);
          }
          if (links.childElementCount) evidence.append(links);
        });
        card.append(evidence);
      }
      fragment.append(card);
    });
    toolGrid.append(fragment);
  }

  function renderStackSummary() {
    if (!stackOverview || !stackInventory) return;
    const summary = stackInventory.summary;
    stackOverview.replaceChildren();
    [
      ["LINGUAGENS", String(summary.language_count), "detectadas em repositórios públicos"],
      ["REPOSITÓRIOS PÚBLICOS", String(summary.public_repository_count), "base do snapshot"],
      ["CÓDIGO DETECTADO", summary.total_code, "soma dos bytes por linguagem"],
    ].forEach(([label, value, note]) => {
      const card = make("article", "stack-stat");
      card.append(make("span", "", label));
      card.append(make("strong", "", value));
      card.append(make("small", "", note));
      stackOverview.append(card);
    });
    if (stackUpdated) stackUpdated.textContent = "Snapshot de linguagens atualizado em "
      + summary.updated_at + " · " + stackInventory.tools.length + " ferramentas documentadas.";
  }

  async function loadStackInventory() {
    if (!languageList || !toolGrid || !stackOverview || !stackUpdated || !stackCategory) return;
    try {
      const response = await fetch("data/stack-inventory.json", {
        headers: { Accept: "application/json" },
        cache: "no-store",
      });
      if (!response.ok) throw new Error("HTTP " + response.status);
      const data = await response.json();
      if (!data || data.schema !== "lucas-belucci-bellini/professional-stack@1"
        || !data.summary || !Array.isArray(data.languages)
        || !Array.isArray(data.tools) || !Array.isArray(data.categories)) {
        throw new Error("Formato de inventário inválido.");
      }
      stackInventory = data;
      stackCategory.replaceChildren();
      stackCategory.append(new Option("Todas as áreas", "all"));
      data.categories.forEach((category) => stackCategory.append(new Option(category, category)));
      renderStackSummary();
      renderLanguages();
      renderTools();
      languageList.setAttribute("aria-busy", "false");
      toolGrid.setAttribute("aria-busy", "false");

      const methodology = document.querySelector("#stack-methodology-content");
      if (methodology && data.methodology) {
        methodology.replaceChildren();
        [
          ["Linguagens", data.methodology.languages],
          ["Ferramentas", data.methodology.tools],
          ["Leitura correta", data.methodology.proficiency],
        ].forEach(([label, value]) => {
          const paragraph = make("p", "");
          paragraph.append(make("strong", "", label + ": "));
          paragraph.append(document.createTextNode(value));
          methodology.append(paragraph);
        });
      }
    } catch (error) {
      languageList.replaceChildren(make("p", "stack-empty", "Não foi possível carregar a matriz de linguagens."));
      toolGrid.replaceChildren(make("p", "stack-empty", "Não foi possível carregar o inventário de ferramentas."));
      stackUpdated.textContent = "Inventário indisponível. " + (error instanceof Error ? error.message : "");
      languageList.setAttribute("aria-busy", "false");
      toolGrid.setAttribute("aria-busy", "false");
    }
  }

  if (searchInput) searchInput.addEventListener("input", render);
  if (languageSearch) languageSearch.addEventListener("input", renderLanguages);
  if (stackSearch) stackSearch.addEventListener("input", renderTools);
  if (stackCategory) stackCategory.addEventListener("change", renderTools);
  if (categorySelect) categorySelect.addEventListener("change", render);
  if (statusSelect) statusSelect.addEventListener("change", render);
  loadCatalog();
  loadStackInventory();
})();