const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");
const path = require("node:path");

const source = fs.readFileSync(path.join(__dirname, "..", "assets", "site.js"), "utf8");

class FakeNode {
  constructor(tag = "div") {
    this.tagName = tag;
    this.children = [];
    this.attributes = {};
    this.listeners = {};
    this.className = "";
    this.textContent = "";
    this.value = "";
    this.hidden = false;
    this.href = "";
    this.target = "";
    this.rel = "";
    this.id = "";
    this.classList = { values: new Set(), add: (name) => this.classList.values.add(name) };
  }
  append(...nodes) {
    for (const node of nodes) {
      this.children.push(...(node.tagName === "#fragment" ? node.children : [node]));
    }
  }
  appendChild(node) { this.append(node); return node; }
  prepend(...nodes) { this.children.unshift(...nodes); }
  replaceChildren(...nodes) { this.children = [...nodes]; }
  setAttribute(name, value) { this.attributes[name] = String(value); }
  removeAttribute(name) { delete this.attributes[name]; }
  addEventListener(name, callback) { (this.listeners[name] ||= []).push(callback); }
  dispatch(name) { for (const callback of this.listeners[name] || []) callback({ target: this }); }
  get childElementCount() { return this.children.length; }
  getAttribute(name) { return this.attributes[name] ?? null; }
}

const demoProjects = [
  {
    name: "Alpha Engine", slug: "alpha-engine", description: "Voxel rendering",
    category: "Games", category_label: "Games", status: "🟢 Active",
    github: "https://github.com/example/alpha-engine", website: "https://alpha.example",
  },
  {
    name: "Beta Planner", slug: "beta-planner", description: "Study tasks",
    category: "Tools", category_label: "Tools", status: "🟣 Academic",
    github: "https://github.com/example/beta-planner",
  },
  {
    name: "Hidden Project", slug: "hidden-project", description: "Private entry",
    category: "Tools", category_label: "Tools", status: "🟢 Active",
    github: "https://github.com/example/hidden-project", private: true,
  },
];

function harness({ projects = demoProjects, status = 200, payload, search = "" } = {}) {
  const selectors = [
    "#project-grid", "#project-search", "#category-filter", "#status-filter",
    "#project-count", "#catalog-message", "#current-year", "#main",
  ];
  const nodes = Object.fromEntries(selectors.map((selector) => [selector, new FakeNode()]));
  nodes["#category-filter"].value = "all";
  nodes["#status-filter"].value = "all";
  nodes["#main"].children = [new FakeNode("section"), new FakeNode("section")];
  const document = {
    title: "Portfólio",
    body: { classList: { values: new Set(), add(name) { this.values.add(name); } } },
    querySelector: (selector) => nodes[selector] || null,
    createElement: (tag) => new FakeNode(tag),
    createDocumentFragment: () => new FakeNode("#fragment"),
  };
  const window = { location: { search } };
  const fetch = async () => ({
    ok: status >= 200 && status < 300,
    status,
    json: async () => payload === undefined ? ({ projects }) : payload,
  });
  vm.runInNewContext(source, {
    document, window, fetch, URL, URLSearchParams, Set, Date, String, Array, Error,
  });
  return { nodes, document };
}

const tick = () => new Promise((resolve) => setImmediate(resolve));
const cards = (nodes) => nodes["#project-grid"].children.filter((node) => node.tagName === "article");
const cardTitles = (nodes) => cards(nodes).map((card) =>
  card.children.find((node) => node.tagName === "h3")?.textContent);
const allNodes = (root) => {
  const result = [];
  const visit = (node) => {
    result.push(node);
    for (const child of node.children || []) visit(child);
  };
  visit(root);
  return result;
};

test("opens the catalog and renders only eligible public projects", async () => {
  const { nodes } = harness();
  await tick();
  assert.deepEqual(cardTitles(nodes), ["Alpha Engine", "Beta Planner"]);
  assert.equal(nodes["#project-count"].textContent, "02 / 02 projetos");
  assert.equal(nodes["#project-grid"].getAttribute("aria-busy"), null);
});

test("search and category/status filters can be combined and reset", async () => {
  const { nodes } = harness();
  await tick();

  nodes["#project-search"].value = "voxel";
  nodes["#project-search"].dispatch("input");
  assert.deepEqual(cardTitles(nodes), ["Alpha Engine"]);

  nodes["#category-filter"].value = "Tools";
  nodes["#category-filter"].dispatch("change");
  assert.deepEqual(cardTitles(nodes), []);

  nodes["#project-search"].value = "";
  nodes["#project-search"].dispatch("input");
  assert.deepEqual(cardTitles(nodes), ["Beta Planner"]);

  nodes["#status-filter"].value = "🟢 Active";
  nodes["#status-filter"].dispatch("change");
  assert.deepEqual(cardTitles(nodes), []);

  // Reset the controls to their documented defaults and verify the full set returns.
  nodes["#category-filter"].value = "all";
  nodes["#status-filter"].value = "all";
  nodes["#status-filter"].dispatch("change");
  assert.deepEqual(cardTitles(nodes), ["Alpha Engine", "Beta Planner"]);
});

test("project detail route resolves a public slug and exposes safe GitHub/site links", async () => {
  const { nodes, document } = harness({ search: "?project=alpha-engine" });
  await tick();

  const detail = nodes["#main"].children[0];
  assert.equal(detail.id, "project-detail");
  assert.ok(allNodes(detail).some((node) => node.textContent === "Alpha Engine"));
  const links = allNodes(detail).filter((node) => node.tagName === "a");
  assert.ok(links.some((link) => link.href === "https://github.com/example/alpha-engine"));
  assert.ok(links.some((link) => link.href === "https://alpha.example/"));
  assert.ok(links.filter((link) => link.target === "_blank")
    .every((link) => link.rel === "noopener noreferrer"));
  assert.equal(document.title, "Alpha Engine — Portfólio");
  assert.ok(document.body.classList.values.has("project-detail-mode"));
  assert.ok(nodes["#main"].children.slice(1).every((node) => node.hidden === true));
});

test("unknown detail slug shows a not-found state without external project links", async () => {
  const { nodes, document } = harness({ search: "?project=not-public" });
  await tick();

  const detail = nodes["#main"].children[0];
  assert.ok(allNodes(detail).some((node) => node.textContent === "Projeto não encontrado."));
  assert.equal(allNodes(detail).filter((node) => node.tagName === "a" && node.target === "_blank").length, 0);
  assert.equal(document.title, "Projeto não encontrado — Portfólio");
});

test("unsafe website URLs are omitted while valid GitHub links remain available", async () => {
  const projects = [{
    name: "Safe Fallback", slug: "safe-fallback", description: "Test fixture",
    category: "Tools", category_label: "Tools", status: "🟢 Active",
    github: "https://github.com/example/safe-fallback", website: "javascript:alert(1)",
  }];
  const { nodes } = harness({ projects });
  await tick();

  const links = allNodes(nodes["#project-grid"]).filter((node) => node.tagName === "a");
  assert.ok(links.some((link) => link.href === "https://github.com/example/safe-fallback"));
  assert.ok(links.every((link) => !String(link.href).startsWith("javascript:")));
  assert.ok(links.every((link) => !String(link.href).includes("alpha.example")));
});

test("HTTP and malformed payload failures show an accessible error state", async (t) => {
  await t.test("HTTP failure", async () => {
    const { nodes } = harness({ status: 503 });
    await tick();
    assert.equal(nodes["#project-count"].textContent, "Catálogo indisponível");
    assert.equal(nodes["#catalog-message"].hidden, false);
    assert.match(nodes["#catalog-message"].textContent, /HTTP 503/);
  });
  await t.test("invalid payload", async () => {
    const { nodes } = harness({ payload: { projects: null } });
    await tick();
    assert.equal(nodes["#project-count"].textContent, "Catálogo indisponível");
    assert.match(nodes["#catalog-message"].textContent, /formato do catálogo é inválido/);
  });
});
