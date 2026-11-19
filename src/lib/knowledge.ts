// Knowledge screen filtering and sorting, kept pure so it is tested directly.

import type { Item } from "../bindings/Item";
import type { Phase } from "../bindings/Phase";
import type { Source } from "../bindings/Source";

export type SortKey =
  | "least-recent"
  | "most-returned"
  | "most-reinforced"
  | "never-used"
  | "newest";

export const SORT_LABELS: Record<SortKey, string> = {
  "least-recent": "Least recently returned",
  "most-returned": "Most returned",
  "most-reinforced": "Most reinforced",
  "never-used": "Never used first",
  newest: "Newest",
};

export interface KnowledgeFilter {
  search: string;
  status: "all" | "active" | "omitted";
  phase: "all" | Phase;
  /** "all", "personal", or a repo_root. */
  scope: string;
  tag: string;
  source: "all" | Source;
  model: string;
  sort: SortKey;
}

export const EMPTY_FILTER: KnowledgeFilter = {
  search: "",
  status: "all",
  phase: "all",
  scope: "all",
  tag: "all",
  source: "all",
  model: "all",
  sort: "least-recent",
};

function matches(item: Item, f: KnowledgeFilter): boolean {
  const q = f.search.trim().toLowerCase();
  if (q) {
    const haystack = [item.title, item.summary, item.guidance, ...item.tags]
      .join("\n")
      .toLowerCase();
    if (!haystack.includes(q)) return false;
  }
  if (f.status !== "all" && item.status !== f.status) return false;
  if (f.phase !== "all" && item.phases.length > 0 && !item.phases.includes(f.phase)) return false;
  if (f.scope === "personal" && item.repo_root !== null) return false;
  if (f.scope !== "all" && f.scope !== "personal" && item.repo_root !== f.scope) return false;
  if (f.tag !== "all" && !item.tags.includes(f.tag)) return false;
  if (f.source !== "all" && item.source !== f.source) return false;
  if (f.model !== "all" && item.source_model !== f.model) return false;
  return true;
}

const time = (iso: string | null) => (iso ? Date.parse(iso) : 0);

function compare(a: Item, b: Item, sort: SortKey): number {
  switch (sort) {
    case "least-recent":
      return time(a.last_returned_at) - time(b.last_returned_at);
    case "most-returned":
      return b.times_returned - a.times_returned;
    case "most-reinforced":
      return b.reinforced_count - a.reinforced_count;
    case "never-used":
      return a.times_returned - b.times_returned || time(a.created_at) - time(b.created_at);
    case "newest":
      return time(b.created_at) - time(a.created_at);
  }
}

export function filterItems(items: Item[], f: KnowledgeFilter): Item[] {
  return items
    .filter((i) => matches(i, f))
    .sort((a, b) => compare(a, b, f.sort) || a.id.localeCompare(b.id));
}

/** Distinct values for the filter dropdowns. */
export function facets(items: Item[]) {
  const unique = (values: string[]) => [...new Set(values)].sort();
  return {
    tags: unique(items.flatMap((i) => i.tags)),
    models: unique(items.map((i) => i.source_model)),
    repos: unique(items.flatMap((i) => (i.repo_root ? [i.repo_root] : []))),
  };
}
