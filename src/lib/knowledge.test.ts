import { describe, expect, it } from "vitest";
import { item } from "../test/backend";
import { EMPTY_FILTER, filterItems } from "./knowledge";

const used = item({
  id: "A",
  title: "Used",
  times_returned: 9,
  last_returned_at: "2026-11-17T00:00:00Z",
});
const stale = item({
  id: "B",
  title: "Stale",
  times_returned: 2,
  last_returned_at: "2026-10-01T00:00:00Z",
});
const never = item({
  id: "C",
  title: "Never",
  times_returned: 0,
  last_returned_at: null,
  status: "omitted",
  phases: ["planning"],
  repo_root: "/work/ui/.git",
  tags: ["react"],
});
const all = [used, stale, never];
const titles = (items: typeof all) => items.map((i) => i.title);

describe("filterItems", () => {
  it("sorts least recently returned first, never-returned at the top", () => {
    expect(titles(filterItems(all, EMPTY_FILTER))).toEqual(["Never", "Stale", "Used"]);
  });

  it("sorts by usage counters", () => {
    expect(titles(filterItems(all, { ...EMPTY_FILTER, sort: "most-returned" }))).toEqual([
      "Used",
      "Stale",
      "Never",
    ]);
  });

  it("filters by status, scope, phase, and tag", () => {
    expect(titles(filterItems(all, { ...EMPTY_FILTER, status: "omitted" }))).toEqual(["Never"]);
    expect(titles(filterItems(all, { ...EMPTY_FILTER, scope: "personal" }))).toEqual([
      "Stale",
      "Used",
    ]);
    expect(titles(filterItems(all, { ...EMPTY_FILTER, phase: "planning" }))).toEqual(["Never"]);
    expect(titles(filterItems(all, { ...EMPTY_FILTER, tag: "react" }))).toEqual(["Never"]);
  });

  it("searches title, tags, and guidance", () => {
    expect(titles(filterItems(all, { ...EMPTY_FILTER, search: "REACT" }))).toEqual(["Never"]);
  });
});
