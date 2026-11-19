import { useState } from "react";
import type { Item } from "../bindings/Item";
import {
  Button,
  ConfirmButton,
  Empty,
  inputClass,
  Loading,
  Notice,
  PageHeader,
  Select,
} from "../components/ui";
import { api, errorMessage } from "../lib/api";
import { kb, phasesLabel, relativeTime, repoName, scopeLabel } from "../lib/format";
import {
  facets,
  filterItems,
  type KnowledgeFilter,
  SORT_LABELS,
  type SortKey,
} from "../lib/knowledge";
import { useItems, useOverview, useStoreMutation } from "../lib/queries";
import { useUIStore } from "../store/ui";

const SOURCE_SHORT = { manual: "Manual", learn: "Learn", auto: "Auto" } as const;

export function Knowledge({
  onEdit,
  onCombine,
}: {
  onEdit: (id: string) => void;
  onCombine: (ids: string[]) => void;
}) {
  const items = useItems();
  const overview = useOverview();
  const { filter, setFilter } = useUIStore();
  const [selected, setSelected] = useState<string[]>([]);
  const toggle = useStoreMutation((item: Item) =>
    api.setItemStatus(item.id, item.status === "active" ? "omitted" : "active"),
  );
  const omitMany = useStoreMutation(async (ids: string[]) => {
    for (const id of ids) await api.setItemStatus(id, "omitted");
  });
  const remove = useStoreMutation((ids: string[]) => api.deleteItems(ids));
  const error = toggle.error ?? omitMany.error ?? remove.error;

  const all = items.data ?? [];
  const visible = filterItems(all, filter);
  const f = facets(all);
  const set = (patch: Partial<KnowledgeFilter>) => setFilter(patch);
  const isSelected = (id: string) => selected.includes(id);
  const stats = overview.data?.index;

  return (
    <div>
      <PageHeader
        title="Knowledge"
        subtitle={
          stats
            ? `${stats.total} ${stats.total === 1 ? "item" : "items"} · ${stats.total - stats.visible} omitted · index ${kb(stats.index_bytes)} · audit what never gets used`
            : "Audit what never gets used"
        }
      />

      <div className="mb-4 flex flex-wrap items-center gap-3">
        <label className="inline-flex items-center gap-2 text-xs text-muted">
          Search
          <input
            className={`${inputClass} h-8 w-48 py-1`}
            placeholder="Title, tag, guidance"
            value={filter.search}
            onChange={(e) => set({ search: e.target.value })}
          />
        </label>
        <Select
          label="Status"
          value={filter.status}
          options={[
            { value: "all", label: "All" },
            { value: "active", label: "Active" },
            { value: "omitted", label: "Omitted" },
          ]}
          onChange={(status) => set({ status })}
        />
        <Select
          label="Phase"
          value={filter.phase}
          options={[
            { value: "all", label: "All" },
            { value: "planning", label: "Planning" },
            { value: "coding", label: "Coding" },
            { value: "review", label: "Review" },
          ]}
          onChange={(phase) => set({ phase })}
        />
        <Select
          label="Scope"
          value={filter.scope}
          options={[
            { value: "all", label: "All" },
            { value: "personal", label: "Personal" },
            ...f.repos.map((r) => ({ value: r, label: `Repo: ${repoName(r)}` })),
          ]}
          onChange={(scope) => set({ scope })}
        />
        <Select
          label="Tag"
          value={filter.tag}
          options={[{ value: "all", label: "All" }, ...f.tags.map((t) => ({ value: t, label: t }))]}
          onChange={(tag) => set({ tag })}
        />
        <Select
          label="Source"
          value={filter.source}
          options={[
            { value: "all", label: "All" },
            { value: "learn", label: "Learn" },
            { value: "auto", label: "Auto" },
            { value: "manual", label: "Manual" },
          ]}
          onChange={(source) => set({ source })}
        />
        <Select
          label="Model"
          value={filter.model}
          options={[
            { value: "all", label: "All" },
            ...f.models.map((m) => ({ value: m, label: m })),
          ]}
          onChange={(model) => set({ model })}
        />
        <Select<SortKey>
          label="Sort"
          value={filter.sort}
          options={(Object.keys(SORT_LABELS) as SortKey[]).map((k) => ({
            value: k,
            label: SORT_LABELS[k],
          }))}
          onChange={(sort) => set({ sort })}
        />
      </div>

      {selected.length > 0 && (
        <div
          role="toolbar"
          aria-label="Selected items"
          className="mb-3 flex items-center gap-2 rounded-lg border border-base-300 bg-base-200 px-3 py-2"
        >
          <span className="text-sm font-semibold">{selected.length} selected</span>
          {selected.length < 2 && (
            <span className="text-xs text-muted">· select another to combine</span>
          )}
          <span className="ml-auto flex items-center gap-2">
            <Button
              variant="primary"
              disabled={selected.length < 2}
              onClick={() => onCombine(selected)}
            >
              Combine
            </Button>
            <Button
              variant="quiet"
              disabled={omitMany.isPending}
              onClick={() => omitMany.mutate(selected, { onSuccess: () => setSelected([]) })}
            >
              Omit
            </Button>
            <ConfirmButton
              render="button"
              label="Delete"
              confirmLabel={`Delete ${selected.length} permanently`}
              disabled={remove.isPending}
              onConfirm={() => remove.mutate(selected, { onSuccess: () => setSelected([]) })}
            />
            <button type="button" className="btn btn-ghost btn-sm" onClick={() => setSelected([])}>
              Clear
            </button>
          </span>
        </div>
      )}

      {error && (
        <div className="mb-3">
          <Notice>{errorMessage(error)}</Notice>
        </div>
      )}

      {items.isPending ? (
        <Loading what="knowledge" />
      ) : items.isError ? (
        <Notice>{errorMessage(items.error)}</Notice>
      ) : all.length === 0 ? (
        <Empty title="No items yet">
          Items appear here once you confirm a learning in Learnings.
        </Empty>
      ) : visible.length === 0 ? (
        <Empty title="No items match these filters" />
      ) : (
        <div className="overflow-hidden rounded-xl border border-base-300 bg-base-100">
          <table className="w-full border-collapse text-left">
            <thead className="bg-base-200 text-xs">
              <tr>
                <th className="w-10 px-3 py-2">
                  <input
                    type="checkbox"
                    className="checkbox checkbox-xs"
                    aria-label="Select all shown"
                    checked={visible.length > 0 && visible.every((i) => isSelected(i.id))}
                    onChange={(e) => setSelected(e.target.checked ? visible.map((i) => i.id) : [])}
                  />
                </th>
                <th className="px-3 py-2 font-semibold">Item</th>
                <th className="px-3 py-2 font-semibold">Scope · phases</th>
                <th className="px-3 py-2 font-semibold">From</th>
                <th className="px-3 py-2 text-right font-semibold">Returned</th>
                <th className="px-3 py-2 font-semibold">Last returned</th>
                <th className="px-3 py-2 text-right font-semibold">Reinforced</th>
                <th className="px-3 py-2 font-semibold">Omit</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((item) => (
                <tr
                  key={item.id}
                  className={`border-t border-base-300 align-top ${isSelected(item.id) ? "bg-primary/10" : ""} ${
                    item.status === "omitted" ? "text-muted" : ""
                  }`}
                >
                  <td className="px-3 py-3">
                    <input
                      type="checkbox"
                      className="checkbox checkbox-xs"
                      aria-label={`Select ${item.title}`}
                      checked={isSelected(item.id)}
                      onChange={(e) =>
                        setSelected(
                          e.target.checked
                            ? [...selected, item.id]
                            : selected.filter((id) => id !== item.id),
                        )
                      }
                    />
                  </td>
                  <td className="px-3 py-3">
                    <button
                      type="button"
                      className="text-left font-semibold hover:underline"
                      onClick={() => onEdit(item.id)}
                    >
                      {item.title}
                    </button>
                    <p className="text-xs text-muted">{item.summary}</p>
                  </td>
                  <td className="px-3 py-3 text-xs">
                    {scopeLabel(item.repo_root)}
                    <br />
                    {phasesLabel(item.phases)}
                  </td>
                  <td className="px-3 py-3 text-xs">
                    {SOURCE_SHORT[item.source]}
                    <br />
                    <span className="text-muted">
                      {item.source_model === "manual" ? "—" : item.source_model}
                    </span>
                  </td>
                  <td className="px-3 py-3 text-right font-semibold">{item.times_returned}</td>
                  <td className="px-3 py-3 text-xs">{relativeTime(item.last_returned_at)}</td>
                  <td className="px-3 py-3 text-right">{item.reinforced_count}</td>
                  <td className="px-3 py-3 text-xs">
                    <label className="inline-flex items-center gap-2">
                      <input
                        type="checkbox"
                        className="toggle toggle-sm"
                        aria-label={`Omit ${item.title}`}
                        checked={item.status === "omitted"}
                        disabled={toggle.isPending}
                        onChange={() => toggle.mutate(item)}
                      />
                      {item.status === "omitted" && relativeTime(item.omitted_at)}
                    </label>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="mt-3 text-xs text-muted">
        Omit hides an item from agents. Nothing is lost, and the omit date is kept so you can
        compare before and after.
      </p>
    </div>
  );
}
