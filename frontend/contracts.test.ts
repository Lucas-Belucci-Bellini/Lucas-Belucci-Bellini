import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  isProjectCatalog,
  isStackInventory,
  PROJECT_CATALOG_SCHEMA,
  STACK_INVENTORY_SCHEMA,
} from "./contracts.js";

const validProject = {
  name: "Public Demo",
  slug: "public-demo",
  description: "A public sample project.",
  category: "Web & SaaS",
  category_label: "Web",
  status: "🟢 Active",
  github: "https://github.com/example/public-demo",
  website: "https://example.com",
};

const validCatalog = () => ({
  schema: PROJECT_CATALOG_SCHEMA,
  project_count: 1,
  projects: [{ ...validProject }],
});

const validStack = () => ({
  schema: STACK_INVENTORY_SCHEMA,
  summary: {
    language_count: 1,
    public_repository_count: 1,
    total_code: "1 KB",
    updated_at: "2026-10-16 04:48 UTC",
  },
  languages: [{
    rank: 1,
    name: "TypeScript",
    bytes: 1024,
    size: "1 KB",
    share: 100,
    repositories: 1,
  }],
  tools: [{
    name: "Vite",
    category: "Frameworks & Web",
    family: "build tool",
    evidence: "A public repository documents Vite.",
    projects: [{
      name: "Public Demo",
      github: "https://github.com/example/public-demo",
      website: "https://example.com",
    }],
  }],
  categories: ["Frameworks & Web"],
  methodology: {
    languages: "Public language maps.",
    tools: "Curated public evidence.",
    proficiency: "Presence is not a proficiency claim.",
  },
});

describe("public project catalog contract", () => {
  it("accepts a valid public catalog", () => {
    assert.equal(isProjectCatalog(validCatalog()), true);
  });

  it("rejects unexpected fields that could leak private metadata", () => {
    const payload = validCatalog();
    payload.projects[0].private_repository_name = "must-not-leak";
    assert.equal(isProjectCatalog(payload), false);
  });

  it("rejects unsafe GitHub and website URLs", () => {
    const httpCatalog = validCatalog();
    httpCatalog.projects[0].github = "http://github.com/example/public-demo";
    assert.equal(isProjectCatalog(httpCatalog), false);

    const credentialCatalog = validCatalog();
    credentialCatalog.projects[0].website = "https://user:secret@example.com";
    assert.equal(isProjectCatalog(credentialCatalog), false);
  });

  it("rejects count mismatches and unknown schema versions", () => {
    const countMismatch = validCatalog();
    countMismatch.project_count = 2;
    assert.equal(isProjectCatalog(countMismatch), false);

    const wrongSchema = validCatalog();
    wrongSchema.schema = "unknown/schema@99";
    assert.equal(isProjectCatalog(wrongSchema), false);
  });
});

describe("professional stack contract", () => {
  it("accepts a valid public stack inventory", () => {
    assert.equal(isStackInventory(validStack()), true);
  });

  it("rejects out-of-range language shares", () => {
    const payload = validStack();
    payload.languages[0].share = 150;
    assert.equal(isStackInventory(payload), false);
  });

  it("rejects unknown tool and project fields", () => {
    const toolField = validStack();
    toolField.tools[0].internal_notes = "private";
    assert.equal(isStackInventory(toolField), false);

    const projectField = validStack();
    projectField.tools[0].projects[0].repository_visibility = "private";
    assert.equal(isStackInventory(projectField), false);
  });

  it("rejects non-HTTPS evidence links", () => {
    const payload = validStack();
    payload.tools[0].projects[0].github = "http://github.com/example/public-demo";
    assert.equal(isStackInventory(payload), false);
  });
});
