(() => {
  "use strict";

  const catalogUrl = "data/projects.json";
  const grid = document.querySelector("#project-grid");
  const searchInput = document.querySelector("#project-search");
  const categorySelect = document.querySelector("#category-filter");
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

  const safeUrl = (value, allowedHosts) => {
    try {
      const url = new URL(value);
      if (url.protocol !== "https:") return null;
      if (allowedHosts && !allowedHosts.includes(url.hostname.toLowerCase())) return null;
      return url.href;
    } catch {
      return null;
    }
  };

  function createProjectCard(project) {
    const card = make("article", "project-card");
    const top = make("div", "project-topline");
    top.append(make("span", "project-category", project.category_label || project.category || "Projeto"));
    const status = allowedStatuses.has(project.status) ? project.status : "Em desenvolvimento";
    top.append(make("span", "project-status", status));
    card.append(top);
    card.append(make("h3", "", project.name || "Projeto sem nome"));
    card.append(make("p", "", project.description && project.description !== "Descrição pública não informada."
      ? project.description
      : "Projeto público em meu catálogo de desenvolvimento. Consulte o repositório para mais detalhes."));

    const actions = make("div", "project-actions");
    const github = safeUrl(project.github, ["github.com", "www.github.com"]);
    const website = project.website_status === "verified" ? safeUrl(project.website) : null;
    if (website) {
      const siteLink = make("a", "site-link", "Abrir site ↗");
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
    const query = (searchInput.value || "").trim().toLocaleLowerCase("pt-BR");
    const category = categorySelect.value;
    const filtered = projects.filter((project) => {
      const text = [project.name, project.description, project.category, project.category_label, project.status]
        .filter(Boolean).join(" ").toLocaleLowerCase("pt-BR");
      return (!query || text.includes(query)) && (category === "all" || project.category === category);
    });

    grid.replaceChildren();
    if (!filtered.length) {
      grid.append(make("div", "empty-card", "Nenhum projeto corresponde a essa busca. Tente outro termo ou categoria."));
    } else {
      const fragment = document.createDocumentFragment();
      filtered.forEach((project) => fragment.append(createProjectCard(project)));
      grid.append(fragment);
    }

    const total = String(projects.length).padStart(2, "0");
    const visible = String(filtered.length).padStart(2, "0");
    count.textContent = filtered.length
      ? visible + " / " + total + " projetos"
      : "Nenhum projeto encontrado · " + visible + " / " + total;
  }

  async function loadCatalog() {
    try {
      const response = await fetch(catalogUrl, { headers: { Accept: "application/json" } });
      if (!response.ok) throw new Error("O catálogo respondeu com HTTP " + response.status);
      const data = await response.json();
      if (!data || !Array.isArray(data.projects)) throw new Error("Formato do catálogo inválido.");

      // The catalog is public metadata. Private entries are intentionally excluded from the website.
      // Use textContent for catalog values and allow only HTTPS destinations; GitHub links must
      // resolve to GitHub itself. This keeps catalog metadata out of HTML parsing contexts.
      projects = data.projects.filter((project) =>
        project && project.private !== true && project.github_visible !== false &&
        safeUrl(project.github, ["github.com", "www.github.com"])
      );

      const categories = [...new Set(projects.map((project) => project.category).filter(Boolean))]
        .sort((a, b) => a.localeCompare(b, "pt-BR"));
      categories.forEach((category) => {
        const option = make("option", "", category);
        option.value = category;
        categorySelect.append(option);
      });

      if (!projects.length) throw new Error("O catálogo não contém projetos públicos para exibir.");
      message.hidden = true;
      render();
    } catch (error) {
      grid.replaceChildren(make("div", "empty-card", "Não foi possível carregar o catálogo de projetos."));
      count.textContent = "Catálogo indisponível";
      message.hidden = false;
      message.textContent = "Verifique se o site foi compilado e se data/projects.json está disponível. " + (error instanceof Error ? error.message : "");
    }
  }

  searchInput.addEventListener("input", render);
  categorySelect.addEventListener("change", render);
  loadCatalog();
})();