// The item editor shared by Edit, Combine, and Edit Then Confirm.

import type { Phase } from "../bindings/Phase";
import { capitalize, PHASES, scopeLabel } from "../lib/format";
import { Field, inputClass } from "./ui";

export interface ItemFormValue {
  title: string;
  summary: string;
  guidance: string;
  applies_when: string;
  exceptions: string;
  tags: string;
  phases: Phase[];
  /** `undefined` means the user hasn't picked a scope yet (mixed-scope combine). */
  repo_root: string | null | undefined;
}

export const parseTags = (text: string) =>
  text
    .split(/[,\s]+/)
    .map((t) => t.trim())
    .filter(Boolean);

const PERSONAL = "__personal__";
const UNSET = "__unset__";

export function ItemForm({
  value,
  onChange,
  scopes,
  fields,
}: {
  value: ItemFormValue;
  onChange: (value: ItemFormValue) => void;
  /** Scopes the user may choose; omit to hide the scope picker. */
  scopes?: (string | null)[];
  /** Limit which fields show (corrections only edit what they change). */
  fields?: (keyof ItemFormValue)[];
}) {
  const set = <K extends keyof ItemFormValue>(key: K, v: ItemFormValue[K]) =>
    onChange({ ...value, [key]: v });
  const shows = (key: keyof ItemFormValue) => !fields || fields.includes(key);
  const scopeOptions = scopes ? [...new Set([null, ...scopes])] : [];

  return (
    <div className="space-y-4">
      {shows("title") && (
        <Field label="Title">
          <input
            className={inputClass}
            value={value.title}
            onChange={(e) => set("title", e.target.value)}
          />
        </Field>
      )}
      {shows("summary") && (
        <Field label="Summary" hint="One line. This is what agents see in the list.">
          <input
            className={inputClass}
            value={value.summary}
            onChange={(e) => set("summary", e.target.value)}
          />
        </Field>
      )}
      {shows("guidance") && (
        <Field label="Guidance">
          <textarea
            className={`${inputClass} min-h-24`}
            value={value.guidance}
            onChange={(e) => set("guidance", e.target.value)}
          />
        </Field>
      )}
      {(shows("applies_when") || shows("exceptions")) && (
        <div className="grid grid-cols-2 gap-4">
          {shows("applies_when") && (
            <Field label="Applies when">
              <textarea
                className={`${inputClass} min-h-16`}
                value={value.applies_when}
                onChange={(e) => set("applies_when", e.target.value)}
              />
            </Field>
          )}
          {shows("exceptions") && (
            <Field label="Exceptions">
              <textarea
                className={`${inputClass} min-h-16`}
                value={value.exceptions}
                onChange={(e) => set("exceptions", e.target.value)}
              />
            </Field>
          )}
        </div>
      )}
      {shows("tags") && (
        <Field label="Tags" hint="Separate with commas or spaces.">
          <input
            className={inputClass}
            value={value.tags}
            onChange={(e) => set("tags", e.target.value)}
          />
        </Field>
      )}
      <div className="grid grid-cols-2 gap-4">
        {shows("phases") && (
          <fieldset>
            <legend className="text-xs font-semibold">Phases</legend>
            <span className="block text-xs text-muted">None checked means all phases.</span>
            <div className="mt-2 flex gap-4">
              {PHASES.map((p) => (
                <label key={p} className="inline-flex items-center gap-1.5">
                  <input
                    type="checkbox"
                    className="rounded"
                    checked={value.phases.includes(p)}
                    onChange={(e) =>
                      set(
                        "phases",
                        e.target.checked
                          ? [...value.phases, p]
                          : value.phases.filter((x) => x !== p),
                      )
                    }
                  />
                  {capitalize(p)}
                </label>
              ))}
            </div>
          </fieldset>
        )}
        {scopes && shows("repo_root") && (
          <Field label="Scope">
            <select
              className={inputClass}
              value={value.repo_root === undefined ? UNSET : (value.repo_root ?? PERSONAL)}
              onChange={(e) =>
                set("repo_root", e.target.value === PERSONAL ? null : e.target.value)
              }
            >
              {value.repo_root === undefined && (
                <option value={UNSET} disabled>
                  Choose a scope…
                </option>
              )}
              {scopeOptions.map((s) => (
                <option key={s ?? PERSONAL} value={s ?? PERSONAL}>
                  {s ? scopeLabel(s) : "Personal (all repos)"}
                </option>
              ))}
            </select>
          </Field>
        )}
      </div>
    </div>
  );
}
