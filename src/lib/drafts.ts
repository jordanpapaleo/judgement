// Conversions between store DTOs and the editor's form value.

import type { Item } from "../bindings/Item";
import type { ItemDraft } from "../bindings/ItemDraft";
import type { LearningPayload } from "../bindings/LearningPayload";
import { type ItemFormValue, parseTags } from "../components/ItemForm";

export function formFromDraft(d: ItemDraft | Item): ItemFormValue {
  return {
    title: d.title,
    summary: d.summary,
    guidance: d.guidance,
    applies_when: d.applies_when,
    exceptions: d.exceptions,
    tags: d.tags.join(", "),
    phases: d.phases,
    repo_root: d.repo_root,
  };
}

export function draftFromForm(f: ItemFormValue, evidence: string[]): ItemDraft {
  return {
    title: f.title,
    summary: f.summary,
    guidance: f.guidance,
    applies_when: f.applies_when,
    exceptions: f.exceptions,
    tags: parseTags(f.tags),
    phases: f.phases,
    repo_root: f.repo_root ?? null,
    evidence,
  };
}

/** The editor value for a learning: a new item's proposal, or a correction applied over its target. */
export function formFromPayload(p: LearningPayload, base?: Item): ItemFormValue {
  return {
    title: p.title ?? base?.title ?? "",
    summary: p.summary ?? base?.summary ?? "",
    guidance: p.guidance ?? base?.guidance ?? "",
    applies_when: p.applies_when ?? base?.applies_when ?? "",
    exceptions: p.exceptions ?? base?.exceptions ?? "",
    tags: (p.tags ?? base?.tags ?? []).join(", "),
    phases: p.phases ?? base?.phases ?? [],
    repo_root: base ? base.repo_root : p.repo_root,
  };
}

/** Which form fields a payload sets (corrections edit only those). */
export function payloadFields(p: LearningPayload): (keyof ItemFormValue)[] {
  return (Object.keys(p) as (keyof LearningPayload)[]).filter(
    (k) => p[k] !== null && p[k] !== undefined,
  ) as (keyof ItemFormValue)[];
}

/**
 * The payload to confirm with. For `new`, every field; for `correct`, only the
 * fields the learning proposed changing.
 */
export function payloadFromForm(
  f: ItemFormValue,
  onlyFields?: (keyof ItemFormValue)[],
): LearningPayload {
  const all: LearningPayload = {
    title: f.title,
    summary: f.summary,
    guidance: f.guidance,
    applies_when: f.applies_when,
    exceptions: f.exceptions,
    tags: parseTags(f.tags),
    phases: f.phases,
    repo_root: f.repo_root ?? null,
  };
  if (!onlyFields) return all;
  const out: LearningPayload = {
    title: null,
    summary: null,
    guidance: null,
    applies_when: null,
    exceptions: null,
    tags: null,
    phases: null,
    repo_root: null,
  };
  for (const k of onlyFields) {
    (out as Record<string, unknown>)[k] = all[k as keyof LearningPayload];
  }
  return out;
}
