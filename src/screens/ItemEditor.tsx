import { useState } from "react";
import type { Item } from "../bindings/Item";
import { ItemForm, type ItemFormValue } from "../components/ItemForm";
import {
  ActionDivider,
  Card,
  IconButton,
  IconConfirmButton,
  Loading,
  Meta,
  Notice,
  PageHeader,
} from "../components/ui";
import { api, errorMessage } from "../lib/api";
import { draftFromForm, formFromDraft } from "../lib/drafts";
import { kb, relativeTime, sourceLabel, utf8Bytes } from "../lib/format";
import { useItems, useStoreMutation } from "../lib/queries";

export function ItemEditor({ id, onDone }: { id: string; onDone: () => void }) {
  const items = useItems();
  const item = items.data?.find((i) => i.id === id);

  if (items.isPending) return <Loading what="the item" />;
  if (items.isError) return <Notice>{errorMessage(items.error)}</Notice>;
  if (!item) {
    return (
      <div>
        <BackLink onClick={onDone} />
        <Notice tone="info">This item no longer exists.</Notice>
      </div>
    );
  }
  return <Editor key={item.id} item={item} onDone={onDone} />;
}

export function BackLink({ onClick }: { onClick: () => void }) {
  return (
    <button type="button" className="mb-1 text-xs text-primary hover:underline" onClick={onClick}>
      ← Knowledge
    </button>
  );
}

function Editor({ item, onDone }: { item: Item; onDone: () => void }) {
  const [form, setForm] = useState<ItemFormValue>(() => formFromDraft(item));
  const [evidence, setEvidence] = useState(item.evidence);
  const save = useStoreMutation(() => api.updateItem(item.id, draftFromForm(form, evidence)));
  const toggle = useStoreMutation(() =>
    api.setItemStatus(item.id, item.status === "active" ? "omitted" : "active"),
  );
  const remove = useStoreMutation(() => api.deleteItems([item.id]));
  const busy = save.isPending || toggle.isPending || remove.isPending;
  const error = save.error ?? toggle.error ?? remove.error;
  const evidenceBytes = evidence.reduce((n, e) => n + utf8Bytes(e), 0);

  return (
    <div>
      <BackLink onClick={onDone} />
      <PageHeader
        title="Edit Item"
        subtitle="Edits overwrite in place. There is no history; backups are the recovery path."
      >
        <div role="toolbar" aria-label="Item actions" className="flex items-center">
          <IconButton
            icon="uil:check"
            label="Save"
            tone="success"
            busy={save.isPending}
            disabled={busy}
            onClick={() => save.mutate(undefined, { onSuccess: onDone })}
          />
          <IconButton icon="uil:times" label="Cancel" disabled={busy} onClick={onDone} />
          <ActionDivider />
          <IconConfirmButton
            icon="uil:trash-alt"
            label="Delete"
            confirmLabel="Delete permanently"
            disabled={busy}
            onConfirm={() => remove.mutate(undefined, { onSuccess: onDone })}
          />
        </div>
      </PageHeader>
      {error && (
        <div className="mb-4">
          <Notice>{errorMessage(error)}</Notice>
        </div>
      )}
      <div className="grid grid-cols-[1fr_260px] gap-4">
        <Card>
          <ItemForm value={form} onChange={setForm} scopes={[item.repo_root]} />

          <div className="mt-4">
            <p className="text-xs font-semibold">
              Evidence{" "}
              <span className="font-normal text-muted">
                · {kb(evidenceBytes)} of 4 KB · oldest rolls off past the cap
              </span>
            </p>
            {evidence.length === 0 ? (
              <p className="mt-1 text-muted">No evidence recorded.</p>
            ) : (
              <ul className="mt-1 space-y-2">
                {evidence.map((entry, index) => (
                  // biome-ignore lint/suspicious/noArrayIndexKey: entries are plain strings that may repeat
                  <li key={index} className="flex items-start gap-3 rounded-lg bg-base-200 p-3">
                    <span className="selectable flex-1 whitespace-pre-wrap text-muted">
                      {entry}
                    </span>
                    <button
                      type="button"
                      className="text-xs underline hover:text-error"
                      onClick={() => setEvidence(evidence.filter((_, i) => i !== index))}
                    >
                      Remove
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </Card>

        <Card className="h-fit">
          <dl className="space-y-3">
            <Meta label="Source">{sourceLabel[item.source]}</Meta>
            <Meta label="Harness · model">
              {item.source_harness} · {item.source_model}
            </Meta>
            <Meta label="Returned">
              {item.times_returned} · last {relativeTime(item.last_returned_at).toLowerCase()}
            </Meta>
            <Meta label="Reinforced">{item.reinforced_count}</Meta>
            <Meta label="Created">{relativeTime(item.created_at)}</Meta>
            <Meta label="Updated">{relativeTime(item.updated_at)}</Meta>
          </dl>
          <label className="mt-4 flex items-center justify-between gap-2 border-t border-base-300 pt-4 text-sm">
            <span>
              <span className="font-semibold">Omit</span>
              {item.status === "omitted" && (
                <span className="block text-xs text-muted">
                  Omitted {relativeTime(item.omitted_at)}
                </span>
              )}
            </span>
            <input
              type="checkbox"
              className="toggle toggle-sm"
              checked={item.status === "omitted"}
              disabled={busy}
              onChange={() => toggle.mutate(undefined)}
            />
          </label>
        </Card>
      </div>
    </div>
  );
}
