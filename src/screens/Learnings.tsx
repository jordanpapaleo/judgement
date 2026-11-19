import { useMemo, useState } from "react";
import type { Item } from "../bindings/Item";
import type { Learning } from "../bindings/Learning";
import type { LearningKind } from "../bindings/LearningKind";
import type { Source } from "../bindings/Source";
import { ItemForm, type ItemFormValue } from "../components/ItemForm";
import { Markdown } from "../components/Markdown";
import {
  ActionDivider,
  Badge,
  Card,
  Empty,
  IconButton,
  IconConfirmButton,
  Loading,
  Meta,
  Notice,
  PageHeader,
  Select,
} from "../components/ui";
import { api, errorMessage } from "../lib/api";
import { formFromPayload, payloadFields, payloadFromForm } from "../lib/drafts";
import {
  capitalize,
  kindLabel,
  phasesLabel,
  relativeTime,
  scopeLabel,
  sourceLabel,
} from "../lib/format";
import { useItems, usePending, useStoreMutation } from "../lib/queries";

interface Filter {
  source: "all" | Source;
  kind: "all" | LearningKind;
  harness: string;
  model: string;
}

const ALL: Filter = { source: "all", kind: "all", harness: "all", model: "all" };

const kindTone = { new: "accent", reinforce: "primary", correct: "plain" } as const;

function sourceBadge(l: Learning): string {
  if (l.source !== "auto") return capitalize(l.source);
  return l.temperature ? `Auto · ${capitalize(l.temperature)}` : "Auto";
}

export function Learnings({ onOpenItem }: { onOpenItem: (id: string) => void }) {
  const pending = usePending();
  const items = useItems();
  const [filter, setFilter] = useState<Filter>(ALL);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const learnings = pending.data ?? [];
  const visible = learnings.filter(
    (l) =>
      (filter.source === "all" || l.source === filter.source) &&
      (filter.kind === "all" || l.kind === filter.kind) &&
      (filter.harness === "all" || l.harness === filter.harness) &&
      (filter.model === "all" || l.model === filter.model),
  );
  const selected = visible.find((l) => l.id === selectedId) ?? visible[0];
  const itemsById = useMemo(() => new Map((items.data ?? []).map((i) => [i.id, i])), [items.data]);
  const distinct = (f: (l: Learning) => string) => [...new Set(learnings.map(f))].sort();
  const options = (values: string[]) => [
    { value: "all", label: "All" },
    ...values.map((v) => ({ value: v, label: v })),
  ];

  return (
    <div>
      <PageHeader
        title="Learnings"
        subtitle={`${learnings.length} pending · oldest first · nothing becomes guidance until you confirm it`}
      >
        <div className="flex flex-wrap gap-3">
          <Select
            label="Source"
            value={filter.source}
            options={[
              { value: "all", label: "All" },
              { value: "learn", label: "Learn" },
              { value: "auto", label: "Inception mode" },
              { value: "manual", label: "Manual" },
            ]}
            onChange={(source) => setFilter({ ...filter, source })}
          />
          <Select
            label="Kind"
            value={filter.kind}
            options={[
              { value: "all", label: "All" },
              { value: "new", label: "New" },
              { value: "reinforce", label: "Reinforce" },
              { value: "correct", label: "Correct" },
            ]}
            onChange={(kind) => setFilter({ ...filter, kind })}
          />
          <Select
            label="Harness"
            value={filter.harness}
            options={options(distinct((l) => l.harness))}
            onChange={(harness) => setFilter({ ...filter, harness })}
          />
          <Select
            label="Model"
            value={filter.model}
            options={options(distinct((l) => l.model))}
            onChange={(model) => setFilter({ ...filter, model })}
          />
        </div>
      </PageHeader>

      {pending.isPending ? (
        <Loading what="learnings" />
      ) : pending.isError ? (
        <Notice>{errorMessage(pending.error)}</Notice>
      ) : learnings.length === 0 ? (
        <Empty title="Nothing to review">
          Learnings proposed by Learn, inception mode, or agents land here for you to confirm or
          deny.
        </Empty>
      ) : visible.length === 0 ? (
        <Empty title="No learnings match these filters" />
      ) : (
        <div className="grid grid-cols-[minmax(220px,280px)_1fr] gap-4">
          <ul className="space-y-2" aria-label="Pending learnings">
            {visible.map((l) => (
              <li key={l.id}>
                <button
                  type="button"
                  aria-current={l.id === selected?.id}
                  onClick={() => setSelectedId(l.id)}
                  className={`w-full rounded-lg border bg-base-100 p-3 text-left ${
                    l.id === selected?.id
                      ? "border-primary ring-1 ring-primary"
                      : "border-base-300 hover:border-base-content/30"
                  }`}
                >
                  <span className="flex gap-1.5">
                    <Badge tone={kindTone[l.kind]}>{capitalize(l.kind)}</Badge>
                    <Badge>{sourceBadge(l)}</Badge>
                  </span>
                  <span className="mt-1.5 block font-semibold">{learningTitle(l, itemsById)}</span>
                  <span className="mt-0.5 block text-xs text-muted">
                    {l.model} · {l.harness} · {relativeTime(l.created_at)}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          {selected && (
            <LearningDetail
              key={selected.id}
              learning={selected}
              target={selected.target_id ? itemsById.get(selected.target_id) : undefined}
              targetsLoaded={!items.isPending}
              onOpenItem={onOpenItem}
            />
          )}
        </div>
      )}
    </div>
  );
}

function learningTitle(l: Learning, items: Map<string, Item>): string {
  if (l.payload.title) return l.payload.title;
  const target = l.target_id ? items.get(l.target_id) : undefined;
  return target?.title ?? "Deleted item";
}

const FIELD_LABELS: Record<string, string> = {
  title: "Title",
  summary: "Summary",
  guidance: "Guidance",
  applies_when: "Applies when",
  exceptions: "Exceptions",
  tags: "Tags",
  phases: "Phases",
};

function show(field: string, value: unknown): string {
  if (Array.isArray(value))
    return field === "phases" ? phasesLabel(value) : value.join(", ") || "—";
  return (value as string) || "—";
}

function LearningDetail({
  learning: l,
  target,
  targetsLoaded,
  onOpenItem,
}: {
  learning: Learning;
  target: Item | undefined;
  targetsLoaded: boolean;
  onOpenItem: (id: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [form, setForm] = useState<ItemFormValue>(() => formFromPayload(l.payload, target));
  const confirm = useStoreMutation((edit: ReturnType<typeof payloadFromForm> | null) =>
    api.confirmLearning(l.id, edit),
  );
  const deny = useStoreMutation(() => api.denyLearning(l.id));
  const remove = useStoreMutation(() => api.deleteLearning(l.id));
  const busy = confirm.isPending || deny.isPending || remove.isPending;
  const error = confirm.error ?? deny.error ?? remove.error;
  const changed = payloadFields(l.payload);
  const targetMissing = l.kind !== "new" && targetsLoaded && !target;

  const startEdit = () => {
    setForm(formFromPayload(l.payload, target));
    setEditing(true);
  };
  const confirmEdited = () =>
    confirm.mutate(payloadFromForm(form, l.kind === "correct" ? changed : undefined));

  return (
    <Card>
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <p className="text-xs font-semibold text-muted">{kindLabel[l.kind]}</p>
          <h2 className="mt-1 text-xl font-bold">
            {l.payload.title ?? target?.title ?? "Deleted item"}
          </h2>
        </div>
        <div role="toolbar" aria-label="Learning actions" className="flex shrink-0 items-center">
          {editing ? (
            <>
              <IconButton
                icon="uil:check"
                label="Confirm Edited"
                tone="success"
                busy={confirm.isPending}
                disabled={busy}
                onClick={confirmEdited}
              />
              <IconButton
                icon="uil:times"
                label="Cancel editing"
                disabled={busy}
                onClick={() => setEditing(false)}
              />
            </>
          ) : (
            <>
              <IconButton
                icon="uil:check"
                label="Confirm"
                tone="success"
                busy={confirm.isPending}
                disabled={busy || targetMissing}
                onClick={() => confirm.mutate(null)}
              />
              {l.kind !== "reinforce" && (
                <IconButton
                  icon="uil:pen"
                  label="Edit Then Confirm"
                  disabled={busy || targetMissing}
                  onClick={startEdit}
                />
              )}
              <IconButton
                icon="uil:ban"
                label="Deny"
                title="Deny — kept so inception mode stops proposing similar ones"
                busy={deny.isPending}
                disabled={busy}
                onClick={() => deny.mutate(undefined)}
              />
            </>
          )}
          <ActionDivider />
          <IconConfirmButton
            icon="uil:trash-alt"
            label="Delete"
            confirmLabel="Delete learning"
            disabled={busy}
            onConfirm={() => remove.mutate(undefined)}
          />
        </div>
      </div>

      {error && (
        <div className="mt-4">
          <Notice>{errorMessage(error)}</Notice>
        </div>
      )}

      <dl className="mt-4 grid grid-cols-3 gap-x-6 gap-y-3">
        <Meta label="Source">{sourceLabel[l.source]}</Meta>
        <Meta label="Harness">{l.harness}</Meta>
        {l.signal_score !== null ? (
          <Meta label="Signal">
            {l.signal_score} of 4{l.temperature && ` · captured at ${capitalize(l.temperature)}`}
          </Meta>
        ) : (
          <Meta label="Proposed">{relativeTime(l.created_at)}</Meta>
        )}
        <Meta label="Observed by">{l.model}</Meta>
        {l.session_model && <Meta label="Session model">{l.session_model}</Meta>}
        {target && (
          <Meta label="Target item">
            <button
              type="button"
              className="text-primary underline underline-offset-2"
              onClick={() => onOpenItem(target.id)}
            >
              Returned {target.times_returned} {target.times_returned === 1 ? "time" : "times"}
            </button>
          </Meta>
        )}
      </dl>

      {targetMissing && (
        <div className="mt-4">
          <Notice tone="info">
            The item this learning targets was deleted. Deny or delete the learning.
          </Notice>
        </div>
      )}

      <div className="mt-5">
        {editing ? (
          <ItemForm
            value={form}
            onChange={setForm}
            fields={l.kind === "correct" ? changed : undefined}
            scopes={l.kind === "new" ? [l.payload.repo_root] : undefined}
          />
        ) : l.kind === "correct" && target ? (
          <div className="space-y-3">
            {changed.map((field) => (
              <div key={field} className="grid grid-cols-2 gap-3">
                <div className="rounded-lg bg-base-200 p-3">
                  <p className="mb-1 text-xs font-semibold text-muted">
                    Current {FIELD_LABELS[field]?.toLowerCase()}
                  </p>
                  {field === "guidance" ? (
                    <Markdown text={target.guidance} />
                  ) : (
                    <p className="selectable">{show(field, target[field as keyof Item])}</p>
                  )}
                </div>
                <div className="rounded-lg border border-primary p-3">
                  <p className="mb-1 text-xs font-semibold text-primary">
                    Proposed {FIELD_LABELS[field]?.toLowerCase()}
                  </p>
                  {field === "guidance" ? (
                    <Markdown text={l.payload.guidance ?? ""} />
                  ) : (
                    <p className="selectable">
                      {show(field, l.payload[field as keyof typeof l.payload])}
                    </p>
                  )}
                </div>
              </div>
            ))}
          </div>
        ) : l.kind === "reinforce" && target ? (
          <div className="rounded-lg bg-base-200 p-3">
            <p className="mb-1 text-xs font-semibold text-muted">
              Current guidance · reinforced {target.reinforced_count} times
            </p>
            <Markdown text={target.guidance} />
          </div>
        ) : l.kind === "new" ? (
          <ProposedItem learning={l} />
        ) : null}
      </div>

      <h3 className="mt-5 text-xs font-semibold">Evidence</h3>
      <p className="selectable mt-1 whitespace-pre-wrap text-muted">{l.evidence}</p>
    </Card>
  );
}

function ProposedItem({ learning: l }: { learning: Learning }) {
  const p = l.payload;
  return (
    <div className="rounded-lg border border-primary p-4">
      <p className="text-xs font-semibold text-primary">Proposed item</p>
      <p className="selectable mt-1 font-semibold">{p.summary}</p>
      <div className="mt-2">
        <Markdown text={p.guidance ?? ""} />
      </div>
      <dl className="mt-3 grid grid-cols-2 gap-x-6 gap-y-2 text-xs">
        {p.applies_when && <Meta label="Applies when">{p.applies_when}</Meta>}
        {p.exceptions && <Meta label="Exceptions">{p.exceptions}</Meta>}
        <Meta label="Phases">{phasesLabel(p.phases ?? [])}</Meta>
        <Meta label="Scope">{scopeLabel(p.repo_root)}</Meta>
        {p.tags && p.tags.length > 0 && <Meta label="Tags">{p.tags.join(", ")}</Meta>}
      </dl>
    </div>
  );
}
