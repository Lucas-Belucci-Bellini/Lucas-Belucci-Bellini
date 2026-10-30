/**
 * Public JSON contracts consumed by the portfolio frontend.
 *
 * Runtime validators intentionally accept unknown input and fail closed.
 * Do not add private/internal catalog fields to these public interfaces.
 */

export const PROJECT_CATALOG_SCHEMA =
  "lucas-belucci-bellini/public-portfolio@1" as const;
export const STACK_INVENTORY_SCHEMA =
  "lucas-belucci-bellini/professional-stack@1" as const;

export interface PublicProject {
  readonly name: string;
  readonly slug: string;
  readonly description: string;
  readonly category: string;
  readonly category_label: string;
  readonly status: string;
  readonly github: string;
  readonly website?: string;
}

export interface ProjectCatalog {
  readonly schema: typeof PROJECT_CATALOG_SCHEMA;
  readonly project_count: number;
  readonly projects: readonly PublicProject[];
}

export interface LanguageStat {
  readonly rank: number;
  readonly name: string;
  readonly bytes: number;
  readonly size: string;
  readonly share: number;
  readonly repositories: number;
}

export interface StackToolProject {
  readonly name: string;
  readonly github: string;
  readonly website?: string;
}

export interface StackTool {
  readonly name: string;
  readonly category: string;
  readonly family: string;
  readonly evidence: string;
  readonly projects: readonly StackToolProject[];
}

export interface StackInventory {
  readonly schema: typeof STACK_INVENTORY_SCHEMA;
  readonly summary: {
    readonly language_count: number;
    readonly public_repository_count: number;
    readonly total_code: string;
    readonly updated_at: string;
  };
  readonly languages: readonly LanguageStat[];
  readonly tools: readonly StackTool[];
  readonly categories: readonly string[];
  readonly methodology: {
    readonly languages: string;
    readonly tools: string;
    readonly proficiency: string;
  };
}

type UnknownRecord = Record<string, unknown>;

function isRecord(value: unknown): value is UnknownRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasOnlyKeys(value: UnknownRecord, allowed: readonly string[]): boolean {
  return Object.keys(value).every((key) => allowed.includes(key));
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isPositiveInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0;
}

function isSafeHttpsUrl(value: unknown): value is string {
  if (!isNonEmptyString(value) || value !== value.trim()) return false;
  try {
    const url = new URL(value);
    const host = url.hostname.toLowerCase();
    return url.protocol === "https:"
      && url.username.length === 0
      && url.password.length === 0
      && host !== "localhost"
      && host !== "localhost.localdomain"
      && !host.endsWith(".local");
  } catch {
    return false;
  }
}

function isGitHubRepositoryUrl(value: unknown): value is string {
  if (!isSafeHttpsUrl(value)) return false;
  try {
    const url = new URL(value);
    const segments = url.pathname.split("/").filter(Boolean);
    return url.hostname.toLowerCase() === "github.com"
      && segments.length === 2
      && segments.every((segment) => !segment.includes("."));
  } catch {
    return false;
  }
}

function isPublicProject(value: unknown): value is PublicProject {
  if (!isRecord(value)
    || !hasOnlyKeys(value, [
      "name", "slug", "description", "category", "category_label",
      "status", "github", "website",
    ])) return false;

  const baseFields = [
    value.name,
    value.slug,
    value.description,
    value.category,
    value.category_label,
    value.status,
  ].every(isNonEmptyString);

  return baseFields
    && isGitHubRepositoryUrl(value.github)
    && (value.website === undefined || isSafeHttpsUrl(value.website));
}

export function isProjectCatalog(value: unknown): value is ProjectCatalog {
  if (!isRecord(value)
    || !hasOnlyKeys(value, ["schema", "project_count", "projects"])
    || value.schema !== PROJECT_CATALOG_SCHEMA
    || !isNonNegativeInteger(value.project_count)
    || !Array.isArray(value.projects)) return false;

  return value.project_count === value.projects.length
    && value.projects.length > 0
    && value.projects.every(isPublicProject);
}

function isLanguageStat(value: unknown): value is LanguageStat {
  return isRecord(value)
    && hasOnlyKeys(value, ["rank", "name", "bytes", "size", "share", "repositories"])
    && isPositiveInteger(value.rank)
    && isNonEmptyString(value.name)
    && isNonNegativeInteger(value.bytes)
    && isNonEmptyString(value.size)
    && typeof value.share === "number"
    && Number.isFinite(value.share)
    && value.share >= 0
    && value.share <= 100
    && isNonNegativeInteger(value.repositories);
}

function isStackToolProject(value: unknown): value is StackToolProject {
  return isRecord(value)
    && hasOnlyKeys(value, ["name", "github", "website"])
    && isNonEmptyString(value.name)
    && isGitHubRepositoryUrl(value.github)
    && (value.website === undefined || isSafeHttpsUrl(value.website));
}

function isStackTool(value: unknown): value is StackTool {
  return isRecord(value)
    && hasOnlyKeys(value, ["name", "category", "family", "evidence", "projects"])
    && isNonEmptyString(value.name)
    && isNonEmptyString(value.category)
    && isNonEmptyString(value.family)
    && isNonEmptyString(value.evidence)
    && Array.isArray(value.projects)
    && value.projects.every(isStackToolProject);
}

export function isStackInventory(value: unknown): value is StackInventory {
  if (!isRecord(value)
    || !hasOnlyKeys(value, [
      "schema", "summary", "languages", "tools", "categories", "methodology",
    ])
    || value.schema !== STACK_INVENTORY_SCHEMA
    || !isRecord(value.summary)
    || !hasOnlyKeys(value.summary, [
      "language_count", "public_repository_count", "total_code", "updated_at",
    ])
    || !isNonNegativeInteger(value.summary.language_count)
    || !isNonNegativeInteger(value.summary.public_repository_count)
    || !isNonEmptyString(value.summary.total_code)
    || !isNonEmptyString(value.summary.updated_at)
    || !Array.isArray(value.languages)
    || !Array.isArray(value.tools)
    || !Array.isArray(value.categories)
    || !isRecord(value.methodology)
    || !hasOnlyKeys(value.methodology, ["languages", "tools", "proficiency"])) {
    return false;
  }

  return value.languages.length === value.summary.language_count
    && value.languages.every(isLanguageStat)
    && value.tools.every(isStackTool)
    && value.categories.length > 0
    && value.categories.every(isNonEmptyString)
    && isNonEmptyString(value.methodology.languages)
    && isNonEmptyString(value.methodology.tools)
    && isNonEmptyString(value.methodology.proficiency);
}
