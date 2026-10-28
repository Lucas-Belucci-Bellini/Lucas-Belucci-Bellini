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
      const emptyState = make("div", "empty-card", "Nenhum projeto corresponde aos filtros atuais. Tente outro termo ou opção.");
      emptyState.setAttribute("role", "status");
      grid.append(emptyState);
    } else {
      const fragment = document.createDocumentFragment();
      filtered.forEach((project) => fragment.append(createProjectCard(project)));
      grid.append(fragment);
    }
    count.textContent = filtered.length + " de " + projects.length + " projetos exibidos";
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
      const errorState = make("div", "empty-card", "Não foi possível carregar o catálogo de projetos.");
      errorState.setAttribute("role", "status");
      grid.replaceChildren(errorState);
      count.textContent = "Catálogo indisponível";
      message.hidden = false;
      message.textContent = "Confira se data/projects.json está disponível e contém JSON válido. "
        + (error instanceof Error ? error.message : "Erro desconhecido.");
    } finally {
      grid.removeAttribute("aria-busy");
    }
  }

  if (searchInput) searchInput.addEventListener("input", render);
  if (categorySelect) categorySelect.addEventListener("change", render);
  if (statusSelect) statusSelect.addEventListener("change", render);
  loadCatalog();
})();