(() => {
  "use strict";

  const catalogUrl = "data/projects.json";
  const PUBLIC_SCHEMA = "lucas-belucci-bellini/public-portfolio@1";
  const grid = document.querySelector("#project-grid");
  const searchInput = document.querySelector("#project-search");
  const categorySelect = document.querySelector("#category-filter");
  const statusSelect = document.querySelector("#status-filter");
  const sortSelect = document.querySelector("#sort-projects");
  const featuredGrid = document.querySelector("#featured-grid");
  const categoryLinks = document.querySelector("#category-links");
  const count = document.querySelector("#project-count");
  const message = document.querySelector("#catalog-message");
  const year = document.querySelector("#current-year");
  const allowedStatuses = new Set(["🟢 Active", "🟡 In Development", "🔵 Experimental", "🟣 Academic", "⚪ Archived"]);

  if (year) year.textContent = String(new Date().getFullYear());

  let projects = [];

  const make = (tag, className, text) => {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  };

  const safeHttpsUrl = (value) => {
    if (typeof value !== "string" || !value || value !== value.trim() || /\s/.test(value)) return null;
    try {
      const url = new URL(value);
      const host = (url.hostname || "").toLowerCase();
      if (url.protocol !== "https:" || !host || url.username || url.password) return null;
      if (host === "localhost" || host.endsWith(".local") || host === "localhost.localdomain") return null;
      return url.href;
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
      return url.hostname === "github.com" && parts.length === 2
        && parts.every((part) => /^[A-Za-z0-9_.-]+$/.test(part) && !part.startsWith("."))
        ? safe : null;
    } catch {
      return null;
    }
  };

  const validPublicProject = (project) => Boolean(
    project && typeof project === "object" && !Array.isArray(project)
    && typeof project.name === "string" && project.name.trim()
    && typeof project.slug === "string" && /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(project.slug)
    && typeof project.description === "string"
    && typeof project.category === "string" && project.category.trim()
    && typeof project.status === "string" && allowedStatuses.has(project.status)
    && safeGitHubUrl(project.github)
  );

  function createProjectCard(project) {
    const card = make("article", "project-card");
    const top = make("div", "project-topline");
    top.append(make("span", "project-category", project.category_label || project.category));
    if (allowedStatuses.has(project.status)) top.append(make("span", "project-status", project.status));
    card.append(top);
    if (project.featured === true) card.classList.add("project-card-featured");
    card.append(make("h3", "", project.name));
    card.append(make("p", "", project.description.trim() ? project.description : "Descrição não informada."));

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
    if (categoryLinks) {
      categoryLinks.querySelectorAll("[data-category]").forEach((button) => {
        button.classList.toggle("is-active", button.dataset.category === (categorySelect.value || "all"));
      });
    }
    const query = (searchInput.value || "").trim().toLocaleLowerCase("pt-BR");
    const category = categorySelect.value || "all";
    const status = statusSelect.value || "all";
    const sort = sortSelect ? sortSelect.value : "name-asc";
    const filtered = projects.filter((project) => {
      const text = [project.name, project.description, project.category, project.category_label, project.status]
        .filter((value) => typeof value === "string").join(" ").toLocaleLowerCase("pt-BR");
      return (!query || text.includes(query))
        && (category === "all" || project.category === category)
        && (status === "all" || project.status === status);
    });
    filtered.sort((a, b) => {
      if (sort === "name-desc") return b.name.localeCompare(a.name, "pt-BR", { sensitivity: "base" });
      if (sort === "status") {
        const byStatus = a.status.localeCompare(b.status, "pt-BR");
        return byStatus || a.name.localeCompare(b.name, "pt-BR", { sensitivity: "base" });
      }
      return a.name.localeCompare(b.name, "pt-BR", { sensitivity: "base" });
    });

    grid.replaceChildren();
    if (!projects.length) {
      grid.append(make("div", "empty-card", "O catálogo está vazio no momento."));
    } else if (!filtered.length) {
      grid.append(make("div", "empty-card", "Nenhum projeto corresponde aos filtros atuais. Tente outro termo ou opção."));
    } else {
      const fragment = document.createDocumentFragment();
      filtered.forEach((project) => fragment.append(createProjectCard(project)));
      grid.append(fragment);
    }
    count.textContent = filtered.length + " de " + projects.length + " projetos";

    if (featuredGrid) {
      const featured = filtered.filter((project) => project.featured === true);
      featuredGrid.replaceChildren();
      if (!featured.length) {
        featuredGrid.append(make("div", "empty-card", "Nenhum destaque corresponde aos filtros atuais."));
      } else {
        const fragment = document.createDocumentFragment();
        featured.forEach((project) => fragment.append(createProjectCard(project)));
        featuredGrid.append(fragment);
      }
    }
  }

  function renderCategoryLinks() {
    if (!categoryLinks) return;
    categoryLinks.replaceChildren();
    const fragment = document.createDocumentFragment();
    const all = make("button", "category-chip is-active", "Todas");
    all.type = "button";
    all.dataset.category = "all";
    all.addEventListener("click", () => {
      searchInput.value = "";
      statusSelect.value = "all";
      categorySelect.value = "all";
      render();
      grid.scrollIntoView({ behavior: "smooth", block: "start" });
    });
    fragment.append(all);
    const categories = [...new Set(projects.map((project) => project.category))]
      .sort((a, b) => a.localeCompare(b, "pt-BR", { sensitivity: "base" }));
    categories.forEach((category) => {
      const button = make("button", "category-chip", category);
      button.type = "button";
      button.dataset.category = category;
      button.addEventListener("click", () => {
        searchInput.value = "";
        statusSelect.value = "all";
        categorySelect.value = category;
        render();
        grid.scrollIntoView({ behavior: "smooth", block: "start" });
      });
      fragment.append(button);
    });
    categoryLinks.append(fragment);
  }

  async function loadCatalog() {
    if (!grid || !count || !message || !categorySelect || !statusSelect) return;
    grid.setAttribute("aria-busy", "true");
    try {
      const response = await fetch(catalogUrl, { headers: { Accept: "application/json" } });
      if (!response.ok) throw new Error("O catálogo respondeu com HTTP " + response.status + ".");
      const data = await response.json();
      if (!data || data.schema !== PUBLIC_SCHEMA || !Array.isArray(data.projects)) {
        throw new Error("O formato do catálogo público é inválido ou incompatível.");
      }

      // O builder já publica uma projeção allowlisted. Validamos novamente no cliente.
      // Não usar innerHTML: todos os dados editoriais são inseridos com textContent.
      projects = data.projects.filter(validPublicProject);
      const categories = [...new Set(projects.map((project) => project.category))]
        .sort((a, b) => a.localeCompare(b, "pt-BR"));
      categories.forEach((category) => {
        const option = make("option", "", category);
        option.value = category;
        categorySelect.append(option);
      });
      const statuses = [...new Set(projects.map((project) => project.status))]
        .sort((a, b) => a.localeCompare(b, "pt-BR"));
      statuses.forEach((value) => {
        const option = make("option", "", value);
        option.value = value;
        statusSelect.append(option);
      });

      message.hidden = true;
      renderCategoryLinks();
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

  if (searchInput) searchInput.addEventListener("input", render);
  if (categorySelect) categorySelect.addEventListener("change", render);
  if (statusSelect) statusSelect.addEventListener("change", render);
  if (sortSelect) sortSelect.addEventListener("change", render);
  loadCatalog();
})();