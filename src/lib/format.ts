import type { AutoLearn } from "../bindings/AutoLearn";
import type { LearningKind } from "../bindings/LearningKind";
import type { Phase } from "../bindings/Phase";
import type { Source } from "../bindings/Source";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

export function relativeTime(iso: string | null, now = Date.now()): string {
  if (!iso) return "Never";
  const diff = now - Date.parse(iso);
  if (diff < MINUTE) return "just now";
  if (diff < HOUR) return plural(Math.floor(diff / MINUTE), "minute");
  if (diff < DAY) return plural(Math.floor(diff / HOUR), "hour");
  const days = Math.floor(diff / DAY);
  if (days === 1) return "yesterday";
  if (days < 7) return `${days} days ago`;
  if (days < 14) return "last week";
  if (days < 60) return `${Math.floor(days / 7)} weeks ago`;
  return new Date(iso).toLocaleDateString();
}

function plural(n: number, unit: string) {
  return `${n} ${unit}${n === 1 ? "" : "s"} ago`;
}

export function kb(bytes: number): string {
  return `${(bytes / 1024).toFixed(1)} KB`;
}

export function utf8Bytes(text: string): number {
  return new TextEncoder().encode(text).length;
}

export const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

export const sourceLabel: Record<Source, string> = {
  manual: "Manual",
  learn: "Learn",
  auto: "Inception mode",
};

export const kindLabel: Record<LearningKind, string> = {
  new: "New item",
  reinforce: "Reinforcement of an existing item",
  correct: "Correction to an existing item",
};

export const PHASES: Phase[] = ["planning", "coding", "review"];
export const TEMPERATURES: AutoLearn[] = ["off", "cool", "warm", "hot"];

export function phasesLabel(phases: Phase[]): string {
  return phases.length === 0 ? "All phases" : phases.map(capitalize).join(", ");
}

/** The last two path segments of a repo's Git common dir, e.g. "jp/ui". */
export function repoName(repoRoot: string): string {
  const parts = repoRoot
    .replace(/\/\.git\/?$/, "")
    .split("/")
    .filter(Boolean);
  return parts.slice(-1)[0] ?? repoRoot;
}

export function scopeLabel(repoRoot: string | null): string {
  return repoRoot ? `Repo: ${repoName(repoRoot)}` : "Personal";
}
