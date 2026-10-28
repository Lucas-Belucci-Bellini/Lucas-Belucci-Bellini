(() => {
  "use strict";

  const catalogUrl = "data/projects.json";
  const PUBLIC_SCHEMA = "lucas-belucci-bellini/public-portfolio@1";
  const grid = document.querySelector("#project-grid");
  const searchInput = document.querySelector("#project-search");
  const categorySelect = document.querySelector("#category-filter");
  const statusSelect = document.querySelector("#status-filter");
  const sortSelect = document.querySelector("#sort-projects");
  const count = document.querySelector("#project-count");
  const message = document.querySelector("#catalog-message");
  const year = document.querySelector("#current-year");
  const statusOrder = ["🟢 Active", "🟡 In Development", "🔵 Experimental", "🟣 Academic", "⚪ Archived"];
  const allowedStatuses = new Set(statusOrder);
  const statusRank = new Map(statusOrder.map((status, index) => [status, index]));

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
    card.append(make("h3", "", project.name));
    card.append(make("p", "", project.description.trim() ? project.description : "Descrição não informada."));

    const actions = make("div", "project-actions");
    const github = safeGitHubUrl(project.github);
    // The public builder exports `website` only when website_status is verified.
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
        const byStatus = (statusRank.get(a.status) ?? Number.MAX_SAFE_INTEGER)
          - (statusRank.get(b.status) ?? Number.MAX_SAFE_INTEGER);
        return byStatus || a.name.localeCompare(b.name, "pt-BR", { sensitivity: "base" });
      }
      return a.name.localeCompare(b.name, "pt-BR", { sensitivity: "base" });
    });

    grid.replaceChildren();
    if (!projects.length) {
      const emptyState = make("div", "empty-card", "O catálogo está vazio no momento.");
      emptyState.setAttribute("role", "status");
      grid.append(emptyState);
    } else if (!filtered.length) {
      const emptyState = make("div", "empty-card", "Nenhum projeto corresponde aos filtros atuais. Tente outro termo ou opção.");
      emptyState.setAttribute("role", "status");
      grid.append(emptyState);
    } else {
      const fragment = document.createDocumentFragment();
      filtered.forEach((project) => fragment.append(createProjectCard(project)));
      grid.append(fragment);
    }
    count.textContent = filtered.length + " de " + projects.length + " projetos";
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
  if (sortSelect) sortSelect.addEventListener("change", render);
  loadCatalog();
})();