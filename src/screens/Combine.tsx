import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import type { CombinePrefill } from "../bindings/CombinePrefill";
import type { Item } from "../bindings/Item";
import { ItemForm, type ItemFormValue } from "../components/ItemForm";
import { Card, IconButton, Loading, Notice, PageHeader } from "../components/ui";
import { api, errorMessage } from "../lib/api";
import { draftFromForm, formFromDraft } from "../lib/drafts";
import { kb, phasesLabel, sourceLabel, utf8Bytes } from "../lib/format";
import { useItems, useStoreMutation } from "../lib/queries";
import { BackLink } from "./ItemEditor";

export function Combine({ ids, onDone }: { ids: string[]; onDone: () => void }) {
  // Fetched once: the prefill must not reset while the user edits.
  const prefill = useQuery({
    queryKey: ["combine-prefill", ...ids],
    queryFn: () => api.combinePrefill(ids),
    staleTime: Number.POSITIVE_INFINITY,
    refetchOnWindowFocus: false,
  });
  const items = useItems();

  return (
    <div>
      <BackLink onClick={onDone} />
      <PageHeader
        title={`Combine ${ids.length} Items`}
        subtitle="Edit the merged item down to one rule. The sources are omitted, not deleted, so you can undo this."
      />
      {prefill.isPending || items.isPending ? (
        <Loading what="the items" />
      ) : prefill.isError ? (
        <Notice>{errorMessage(prefill.error)}</Notice>
      ) : (
        <CombineEditor
          ids={ids}
          prefill={prefill.data}
          sources={ids.flatMap((id) => items.data?.find((i) => i.id === id) ?? [])}
          onDone={onDone}
        />
      )}
    </div>
  );
}

function CombineEditor({
  ids,
  prefill,
  sources,
  onDone,
}: {
  ids: string[];
  prefill: CombinePrefill;
  sources: Item[];
  onDone: () => void;
}) {
  const mixedScopes = prefill.scopes.length > 1;
  const [form, setForm] = useState<ItemFormValue>(() => ({
    ...formFromDraft(prefill.draft),
    repo_root: mixedScopes ? undefined : prefill.draft.repo_root,
  }));
  const save = useStoreMutation(() =>
    api.combineItems(ids, draftFromForm(form, prefill.draft.evidence)),
  );
  const evidenceBytes = prefill.draft.evidence.reduce((n, e) => n + utf8Bytes(e), 0);
  const needsScope = form.repo_root === undefined;

  return (
    <div className="grid grid-cols-[minmax(220px,280px)_1fr] gap-4">
      <div>
        <h2 className="mb-2 text-xs font-semibold">Sources</h2>
        <ul className="space-y-2">
          {sources.map((s) => (
            <li key={s.id} className="rounded-lg bg-base-200 p-3">
              <p className="font-semibold">{s.title}</p>
              <p className="mt-1 text-xs">{s.guidance}</p>
              <p className="mt-1 text-xs text-muted">
                {sourceLabel[s.source]} · {s.source_model} · returned {s.times_returned} times ·{" "}
                {phasesLabel(s.phases)}
              </p>
            </li>
          ))}
        </ul>
      </div>
      <Card>
        <div className="mb-4 flex items-start gap-3">
          <div className="min-w-0 flex-1">
            <h2 className="text-lg font-bold">Combined item</h2>
            <p className="text-xs text-muted">
              Saving creates one active item and omits the sources.
            </p>
          </div>
          <div role="toolbar" aria-label="Combine actions" className="flex shrink-0 items-center">
            <IconButton
              icon="uil:check"
              label="Save Combined Item"
              tone="success"
              busy={save.isPending}
              disabled={save.isPending || needsScope}
              onClick={() => save.mutate(undefined, { onSuccess: onDone })}
            />
            <IconButton
              icon="uil:times"
              label="Cancel"
              disabled={save.isPending}
              onClick={onDone}
            />
          </div>
        </div>
        <ItemForm value={form} onChange={setForm} scopes={prefill.scopes} />
        <div className="mt-4">
          <p className="text-xs font-semibold">
            Evidence{" "}
            <span className="font-normal text-muted">
              Merged from {sources.length} sources · {kb(evidenceBytes)} of 4 KB
            </span>
          </p>
          {prefill.evidence_truncated && (
            <p className="mt-1 text-xs text-warning">
              Some evidence didn't fit the 4 KB cap and was left out.
            </p>
          )}
          <div className="selectable mt-1 space-y-2 rounded-lg bg-base-200 p-3 text-muted">
            {prefill.draft.evidence.length === 0
              ? "No evidence recorded."
              : prefill.draft.evidence.map((e, i) => (
                  // biome-ignore lint/suspicious/noArrayIndexKey: entries are plain strings that may repeat
                  <p key={i}>{e}</p>
                ))}
          </div>
        </div>
        {mixedScopes && needsScope && (
          <div className="mt-4">
            <Notice tone="info">
              These items have different scopes. Choose one for the combined item.
            </Notice>
          </div>
        )}
        {save.error && (
          <div className="mt-4">
            <Notice>{errorMessage(save.error)}</Notice>
          </div>
        )}
      </Card>
    </div>
  );
}
