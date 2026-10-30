import {
  isProjectCatalog,
  isStackInventory,
  type PublicProject,
  type StackInventory,
  type StackTool,
} from "./contracts.js";

const allowedStatuses = new Set([
  "🟢 Active",
  "🟡 In Development",
  "🔵 Experimental",
  "🟣 Academic",
]);

const make = <K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className = "",
  text?: string,
): HTMLElementTagNameMap[K] => {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
};

const element = <T extends Element>(selector: string): T | null =>
  document.querySelector<T>(selector);

const safeHttpsUrl = (value: string | undefined): string | null => {
  if (!value || value !== value.trim()) return null;
  try {
    const url = new URL(value);
    const host = url.hostname.toLowerCase();
    if (url.protocol !== "https:"
      || url.username.length > 0
      || url.password.length > 0
      || host === "localhost"
      || host === "localhost.localdomain"
      || host.endsWith(".local")) return null;
    return url.href;
  } catch {
    return null;
  }
};

const setExternalLink = (link: HTMLAnchorElement, href: string): void => {
  link.href = href;
  link.target = "_blank";
  link.rel = "noopener noreferrer";
};

const createProjectCard = (project: PublicProject): HTMLElement => {
  const card = make("article", "project-card");
  const top = make("div", "project-topline");
  top.append(make(
    "span",
    "project-category",
    project.category_label || project.category,
  ));
  if (allowedStatuses.has(project.status)) {
    top.append(make("span", "project-status", project.status));
  }
  card.append(top);
  card.append(make("h3", "", project.name));
  card.append(make("p", "", project.description || "Descrição não informada."));

  const actions = make("div", "project-actions");
  const website = project.website ? safeHttpsUrl(project.website) : null;
  const github = safeHttpsUrl(project.github);
  if (website) {
    const siteLink = make("a", "site-link", "Site verificado ↗");
    setExternalLink(siteLink, website);
    actions.append(siteLink);
  }
  if (github) {
    const codeLink = make("a", "", "Código-fonte ↗");
    setExternalLink(codeLink, github);
    actions.append(codeLink);
  }
  if (actions.childElementCount > 0) card.append(actions);
  return card;
};

const searchInput = element<HTMLInputElement>("#project-search");
const categorySelect = element<HTMLSelectElement>("#category-filter");
const statusSelect = element<HTMLSelectElement>("#status-filter");
const projectGrid = element<HTMLElement>("#project-grid");
const projectCount = element<HTMLElement>("#project-count");
const catalogMessage = element<HTMLElement>("#catalog-message");
const currentYear = element<HTMLElement>("#current-year");

const stackOverview = element<HTMLElement>("#stack-overview");
const stackUpdated = element<HTMLElement>("#stack-updated");
const languageList = element<HTMLElement>("#language-list");
const languageSearch = element<HTMLInputElement>("#language-search");
const stackSearch = element<HTMLInputElement>("#stack-search");
const stackCategory = element<HTMLSelectElement>("#stack-category");
const toolGrid = element<HTMLElement>("#tool-grid");

let projects: readonly PublicProject[] = [];
let stackInventory: StackInventory | null = null;

if (currentYear) currentYear.textContent = String(new Date().getFullYear());

const renderProjects = (): void => {
  if (!projectGrid || !projectCount || !searchInput || !categorySelect || !statusSelect) return;

  const query = searchInput.value.trim().toLocaleLowerCase("pt-BR");
  const category = categorySelect.value;
  const status = statusSelect.value;

  const filtered = projects.filter((project) => {
    const haystack = [
      project.name,
      project.description,
      project.category,
      project.category_label,
      project.status,
    ].join(" ").toLocaleLowerCase("pt-BR");

    return (!query || haystack.includes(query))
      && (category === "all" || project.category === category)
      && (status === "all" || project.status === status);
  });

  projectGrid.replaceChildren();
  if (filtered.length === 0) {
    projectGrid.append(make(
      "div",
      "empty-card",
      "Nenhum projeto corresponde aos filtros atuais. Tente outro termo ou opção.",
    ));
  } else {
    const fragment = document.createDocumentFragment();
    filtered.forEach((project) => fragment.append(createProjectCard(project)));
    projectGrid.append(fragment);
  }
  projectCount.textContent = `${String(filtered.length).padStart(2, "0")} / ${String(projects.length).padStart(2, "0")} projetos`;
};

const loadCatalog = async (): Promise<void> => {
  if (!projectGrid || !projectCount || !catalogMessage || !categorySelect || !statusSelect) return;
  projectGrid.setAttribute("aria-busy", "true");

  try {
    const response = await fetch("data/projects.json", {
      headers: { Accept: "application/json" },
    });
    if (!response.ok) throw new Error(`O catálogo respondeu com HTTP ${response.status}.`);

    const payload: unknown = await response.json();
    if (!isProjectCatalog(payload)) {
      throw new Error("O catálogo não respeita o contrato público esperado.");
    }

    // The contract is an allowlist: unknown/private fields make validation fail.
    projects = payload.projects;
    const categories = [...new Set(projects.map((project) => project.category))]
      .sort((a, b) => a.localeCompare(b, "pt-BR"));
    categories.forEach((category) => {
      const option = make("option", "", category);
      option.value = category;
      categorySelect.append(option);
    });

    const statuses = [...new Set(projects
      .map((project) => project.status)
      .filter((value) => allowedStatuses.has(value)))]
      .sort((a, b) => a.localeCompare(b, "pt-BR"));
    statuses.forEach((value) => {
      const option = make("option", "", value);
      option.value = value;
      statusSelect.append(option);
    });

    catalogMessage.hidden = true;
    renderProjects();
  } catch (error) {
    projectGrid.replaceChildren(make(
      "div",
      "empty-card",
      "Não foi possível carregar o catálogo público de projetos.",
    ));
    projectCount.textContent = "Catálogo indisponível";
    catalogMessage.hidden = false;
    catalogMessage.textContent = error instanceof Error
      ? error.message
      : "Erro desconhecido ao validar o catálogo.";
  } finally {
    projectGrid.removeAttribute("aria-busy");
  }
};

const renderLanguages = (): void => {
  if (!languageList || !stackInventory) return;
  const query = (languageSearch?.value ?? "").trim().toLocaleLowerCase("pt-BR");
  const rows = stackInventory.languages.filter((language) =>
    language.name.toLocaleLowerCase("pt-BR").includes(query));

  languageList.replaceChildren();
  if (rows.length === 0) {
    languageList.append(make("p", "stack-empty", "Nenhuma linguagem corresponde à busca."));
    return;
  }

  const fragment = document.createDocumentFragment();
  rows.forEach((language) => {
    const row = make("article", "language-row");
    const heading = make("div", "language-row-heading");
    heading.append(make("span", "language-rank", String(language.rank).padStart(2, "0")));
    heading.append(make("strong", "language-name", language.name));
    heading.append(make("span", "language-share", `${language.share.toFixed(2)}%`));
    row.append(heading);

    const track = make("div", "language-track");
    const bar = make("span", "language-bar");
    bar.style.width = `${Math.max(0, Math.min(100, language.share))}%`;
    track.append(bar);
    row.append(track);
    row.append(make(
      "p",
      "language-meta",
      `${language.size} de código · presente em ${language.repositories} repositório(s) público(s)`,
    ));
    fragment.append(row);
  });
  languageList.append(fragment);
};

const renderToolCard = (tool: StackTool): HTMLElement => {
  const card = make("article", "tool-card");
  const top = make("div", "tool-card-top");
  top.append(make("span", "tool-category", tool.category));
  top.append(make("span", "tool-family", tool.family));
  card.append(top);
  card.append(make("h4", "", tool.name));
  card.append(make("p", "tool-evidence", tool.evidence));

  if (tool.projects.length > 0) {
    const evidence = make("div", "tool-projects");
    evidence.append(make("span", "tool-projects-label", "EVIDÊNCIA EM PROJETOS PÚBLICOS"));
    tool.projects.forEach((project) => {
      const links = make("div", "tool-project-links");
      const github = safeHttpsUrl(project.github);
      if (github) {
        const codeLink = make("a", "", `${project.name} · código ↗`);
        setExternalLink(codeLink, github);
        links.append(codeLink);
      }
      const website = project.website ? safeHttpsUrl(project.website) : null;
      if (website) {
        const siteLink = make("a", "", "site ↗");
        setExternalLink(siteLink, website);
        links.append(siteLink);
      }
      if (links.childElementCount > 0) evidence.append(links);
    });
    card.append(evidence);
  }
  return card;
};

const renderTools = (): void => {
  if (!toolGrid || !stackInventory) return;
  const query = (stackSearch?.value ?? "").trim().toLocaleLowerCase("pt-BR");
  const category = stackCategory?.value ?? "all";
  const filtered = stackInventory.tools.filter((tool) => {
    const haystack = [tool.name, tool.family, tool.category, tool.evidence]
      .join(" ")
      .toLocaleLowerCase("pt-BR");
    return (!query || haystack.includes(query))
      && (category === "all" || tool.category === category);
  });

  toolGrid.replaceChildren();
  if (filtered.length === 0) {
    toolGrid.append(make("p", "stack-empty", "Nenhuma tecnologia corresponde aos filtros atuais."));
    return;
  }
  const fragment = document.createDocumentFragment();
  filtered.forEach((tool) => fragment.append(renderToolCard(tool)));
  toolGrid.append(fragment);
};

const renderStackSummary = (): void => {
  if (!stackOverview || !stackInventory) return;
  const summary = stackInventory.summary;
  stackOverview.replaceChildren();

  const cards: readonly (readonly [string, string, string])[] = [
    ["LINGUAGENS", String(summary.language_count), "detectadas em repositórios públicos"],
    ["REPOSITÓRIOS PÚBLICOS", String(summary.public_repository_count), "base do snapshot"],
    ["CÓDIGO DETECTADO", summary.total_code, "soma dos bytes por linguagem"],
  ];
  cards.forEach(([label, value, note]) => {
    const card = make("article", "stack-stat");
    card.append(make("span", "", label));
    card.append(make("strong", "", value));
    card.append(make("small", "", note));
    stackOverview.append(card);
  });

  if (stackUpdated) {
    stackUpdated.textContent = `Snapshot de linguagens atualizado em ${summary.updated_at} · ${stackInventory.tools.length} ferramentas documentadas.`;
  }
};

const renderMethodology = (inventory: StackInventory): void => {
  const methodology = element<HTMLElement>("#stack-methodology-content");
  if (!methodology) return;
  methodology.replaceChildren();

  const rows: readonly (readonly [string, string])[] = [
    ["Linguagens", inventory.methodology.languages],
    ["Ferramentas", inventory.methodology.tools],
    ["Leitura correta", inventory.methodology.proficiency],
  ];
  rows.forEach(([label, value]) => {
    const paragraph = make("p");
    paragraph.append(make("strong", "", `${label}: `));
    paragraph.append(document.createTextNode(value));
    methodology.append(paragraph);
  });
};

const loadStackInventory = async (): Promise<void> => {
  if (!languageList || !toolGrid || !stackOverview || !stackUpdated || !stackCategory) return;
  try {
    const response = await fetch("data/stack-inventory.json", {
      headers: { Accept: "application/json" },
      cache: "no-store",
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    const payload: unknown = await response.json();
    if (!isStackInventory(payload)) {
      throw new Error("O inventário não respeita o contrato público esperado.");
    }

    stackInventory = payload;
    stackCategory.replaceChildren();
    stackCategory.append(new Option("Todas as áreas", "all"));
    payload.categories.forEach((category) => {
      stackCategory.append(new Option(category, category));
    });

    renderStackSummary();
    renderLanguages();
    renderTools();
    renderMethodology(payload);
  } catch (error) {
    languageList.replaceChildren(make(
      "p",
      "stack-empty",
      "Não foi possível carregar a matriz de linguagens.",
    ));
    toolGrid.replaceChildren(make(
      "p",
      "stack-empty",
      "Não foi possível carregar o inventário de ferramentas.",
    ));
    stackUpdated.textContent = "Inventário indisponível. "
      + (error instanceof Error ? error.message : "Erro desconhecido.");
  } finally {
    languageList.setAttribute("aria-busy", "false");
    toolGrid.setAttribute("aria-busy", "false");
  }
};

searchInput?.addEventListener("input", renderProjects);
categorySelect?.addEventListener("change", renderProjects);
statusSelect?.addEventListener("change", renderProjects);
languageSearch?.addEventListener("input", renderLanguages);
stackSearch?.addEventListener("input", renderTools);
stackCategory?.addEventListener("change", renderTools);

void loadCatalog();
void loadStackInventory();
