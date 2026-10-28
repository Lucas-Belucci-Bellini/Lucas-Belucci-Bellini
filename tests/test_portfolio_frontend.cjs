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
  }
  append(...nodes) { this.children.push(...nodes); }
  appendChild(node) { this.children.push(node); return node; }
  replaceChildren(...nodes) { this.children = [...nodes]; }
  setAttribute(name, value) { this.attributes[name] = String(value); }
  removeAttribute(name) { delete this.attributes[name]; }
  addEventListener(name, callback) { (this.listeners[name] ||= []).push(callback); }
  dispatch(name) { for (const callback of this.listeners[name] || []) callback({ target: this }); }
  get childElementCount() { return this.children.length; }
  getAttribute(name) { return this.attributes[name] ?? null; }
}

function harness(fetchImpl) {
  const ids = [
    "#project-grid", "#project-search", "#category-filter",
    "#status-filter", "#project-count", "#catalog-message", "#current-year",
  ];
  const nodes = Object.fromEntries(ids.map((id) => [id, new FakeNode()]));
  nodes["#category-filter"].value = "all";
  nodes["#status-filter"].value = "all";
  const document = {
    querySelector: (selector) => nodes[selector] || null,
    createElement: (tag) => new FakeNode(tag),
    createDocumentFragment: () => new FakeNode("#fragment"),
  };
  vm.runInNewContext(source, { document, fetch: fetchImpl, URL, Set, Date, String, Array, Error });
  return nodes;
}

const response = (projects, status = 200) => ({
  ok: status >= 200 && status < 300,
  status,
  json: async () => ({ projects }),
});
const tick = () => new Promise((resolve) => setImmediate(resolve));
const cards = (nodes) => nodes["#project-grid"].children.filter((node) => node.tagName === "article");
const cardTitles = (nodes) => cards(nodes).map((card) => card.children.find((node) => node.tagName === "h3")?.textContent);

test("loads eligible projects and renders safe links with noopener", async () => {
  const nodes = harness(async () => response([
    { name: "Alpha", slug: "alpha", description: "First", category: "Tools", category_label: "Tools", status: "🟢 Active", github: "https://github.com/owner/alpha", website: "https://example.com" },
    { name: "Hidden", slug: "hidden", github: "https://github.com/owner/hidden", private: true },
    { name: "Unsafe", slug: "unsafe", github: "https://github.com.evil.example/owner/unsafe" },
  ]));
  await tick();
  assert.deepEqual(cardTitles(nodes), ["Alpha"]);
  const card = cards(nodes)[0];
  const links = card.children.flatMap((child) => child.children || []).filter((node) => node.tagName === "a");
  assert.equal(links.length, 2);
  assert.ok(links.every((link) => link.rel === "noopener noreferrer"));
  assert.equal(nodes["#project-count"].textContent, "01 / 01 projetos");
  assert.equal(nodes["#project-grid"].getAttribute("aria-busy"), null);
});

test("search input and category/status filters change the visible result set", async () => {
  const nodes = harness(async () => response([
    { name: "Rust Engine", slug: "rust-engine", description: "Voxel", category: "Games", category_label: "Games", status: "🟢 Active", github: "https://github.com/owner/rust-engine" },
    { name: "Planner", slug: "planner", description: "Tasks", category: "Tools", category_label: "Tools", status: "🟣 Academic", github: "https://github.com/owner/planner" },
  ]));
  await tick();
  nodes["#project-search"].value = "voxel";
  nodes["#project-search"].dispatch("input");
  assert.deepEqual(cardTitles(nodes), ["Rust Engine"]);
  nodes["#project-search"].value = "";
  nodes["#category-filter"].value = "Tools";
  nodes["#category-filter"].dispatch("change");
  assert.deepEqual(cardTitles(nodes), ["Planner"]);
  nodes["#category-filter"].value = "all";
  nodes["#status-filter"].value = "🟢 Active";
  nodes["#status-filter"].dispatch("change");
  assert.deepEqual(cardTitles(nodes), ["Rust Engine"]);
  assert.ok(nodes["#category-filter"].children.some((option) => option.value === "Games"));
});

test("missing optional fields render safe fallbacks and omit unsafe URLs", async () => {
  const nodes = harness(async () => response([
    { github: "https://github.com/owner/minimal", website: "javascript:alert(1)" },
  ]));
  await tick();
  assert.deepEqual(cardTitles(nodes), ["Projeto sem nome"]);
  const card = cards(nodes)[0];
  assert.ok(card.children.some((node) => node.textContent === "Descrição não informada."));
  assert.equal(card.children.some((node) => node.tagName === "div" && node.children.some((child) => child.tagName === "a")), false);
});

test("HTTP and malformed payload failures show an accessible error state", async (t) => {
  await t.test("HTTP failure", async () => {
    const nodes = harness(async () => response([], 503));
    await tick();
    assert.equal(nodes["#project-count"].textContent, "Catálogo indisponível");
    assert.equal(nodes["#catalog-message"].hidden, false);
    assert.match(nodes["#catalog-message"].textContent, /HTTP 503/);
    assert.match(nodes["#project-grid"].children[0].textContent, /Não foi possível carregar/);
  });
  await t.test("invalid payload", async () => {
    const nodes = harness(async () => ({ ok: true, status: 200, json: async () => ({ projects: null }) }));
    await tick();
    assert.equal(nodes["#project-count"].textContent, "Catálogo indisponível");
    assert.match(nodes["#catalog-message"].textContent, /formato do catálogo é inválido/);
  });
});
