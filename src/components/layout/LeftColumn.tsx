// Navigation column, styled after Helm's LeftColumn: views on top, Scopes and
// Tags sections that filter Knowledge, Settings in the footer. When the panel
// is closed it collapses to an icon rail.
import { Icon } from "@iconify/react";
import { repoName } from "../../lib/format";
import { facets } from "../../lib/knowledge";
import { useItems, useOverview } from "../../lib/queries";
import { type Section, useUIStore } from "../../store/ui";

const VIEWS: { id: Section; label: string; icon: string }[] = [
  { id: "learnings", label: "Learnings", icon: "uil:lightbulb-alt" },
  { id: "knowledge", label: "Knowledge", icon: "uil:book-open" },
];

const row = (active: boolean) =>
  `flex w-full items-center gap-2 px-2 py-1.5 text-sm transition-colors ${
    active
      ? "bg-base-300 text-base-content"
      : "text-base-content/60 hover:bg-base-200 hover:text-base-content"
  }`;

function useSection(): Section | null {
  const screen = useUIStore((s) => s.screen);
  if (screen.name === "help") return null;
  return screen.name === "learnings" ? "learnings" : "knowledge";
}

function useCounts(): Record<Section, number | undefined> {
  const overview = useOverview();
  return { learnings: overview.data?.pending, knowledge: overview.data?.index.total };
}

function HelpButton() {
  const { screen, navigate } = useUIStore();
  const active = screen.name === "help";
  return (
    <button
      type="button"
      onClick={() => navigate({ name: "help" })}
      title="Setup guide"
      aria-label="Help"
      aria-current={active ? "page" : undefined}
      className={`btn btn-ghost btn-xs btn-square hover:opacity-100 ${active ? "opacity-100" : "opacity-60"}`}
    >
      <Icon icon="uil:question-circle" className="h-4 w-4" aria-hidden="true" />
    </button>
  );
}

function SettingsButton() {
  const openSettings = useUIStore((s) => s.openSettings);
  return (
    <button
      type="button"
      onClick={() => openSettings()}
      title="Settings"
      aria-label="Settings"
      className="btn btn-ghost btn-xs btn-square opacity-60 hover:opacity-100"
    >
      <Icon icon="uil:setting" className="h-4 w-4" aria-hidden="true" />
    </button>
  );
}

export function LeftColumn() {
  const section = useSection();
  const counts = useCounts();
  const { navigate, filter, setFilter } = useUIStore();
  const items = useItems().data ?? [];
  const { tags, repos } = facets(items);
  const tagCount = (tag: string) => items.filter((i) => i.tags.includes(tag)).length;
  const repoCount = (repo: string) => items.filter((i) => i.repo_root === repo).length;
  const showKnowledge = (patch: Parameters<typeof setFilter>[0]) => {
    setFilter(patch);
    navigate({ name: "knowledge" });
  };
  const inKnowledge = section === "knowledge";

  return (
    <nav
      aria-label="Navigation"
      className="flex h-full min-w-0 flex-col overflow-hidden border-r border-base-300"
    >
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden py-2">
        <ul className="mb-4">
          {VIEWS.map((v) => (
            <li key={v.id}>
              <button
                type="button"
                aria-current={section === v.id ? "page" : undefined}
                onClick={() => navigate({ name: v.id })}
                className={row(section === v.id)}
              >
                <Icon icon={v.icon} className="h-4 w-4 shrink-0" aria-hidden="true" />
                <span className="flex-1 text-left">{v.label}</span>
                {counts[v.id] !== undefined && (
                  <span
                    className={`text-xs ${v.id === "learnings" && counts.learnings ? "text-primary" : "opacity-40"}`}
                  >
                    {counts[v.id]}
                  </span>
                )}
              </button>
            </li>
          ))}
        </ul>

        <div className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden border-t border-base-300 pt-4">
          <p className="mb-1 px-2 text-xs font-semibold uppercase tracking-wider opacity-40">
            Scopes
          </p>
          <ul className="mb-4">
            <li>
              <button
                type="button"
                onClick={() => showKnowledge({ scope: "all" })}
                className={row(inKnowledge && filter.scope === "all")}
              >
                <span className="flex-1 text-left">All Items</span>
                <span className="text-xs opacity-40">{items.length}</span>
              </button>
            </li>
            <li>
              <button
                type="button"
                onClick={() => showKnowledge({ scope: "personal" })}
                className={row(inKnowledge && filter.scope === "personal")}
              >
                <Icon
                  icon="uil:user"
                  className="h-3.5 w-3.5 shrink-0 opacity-60"
                  aria-hidden="true"
                />
                <span className="flex-1 text-left">Personal</span>
                <span className="text-xs opacity-40">
                  {items.filter((i) => !i.repo_root).length}
                </span>
              </button>
            </li>
            {repos.map((repo) => (
              <li key={repo}>
                <button
                  type="button"
                  title={repo}
                  onClick={() => showKnowledge({ scope: repo })}
                  className={row(inKnowledge && filter.scope === repo)}
                >
                  <Icon
                    icon="uil:folder"
                    className="h-3.5 w-3.5 shrink-0 opacity-60"
                    aria-hidden="true"
                  />
                  <span className="flex-1 truncate text-left">{repoName(repo)}</span>
                  <span className="text-xs opacity-40">{repoCount(repo)}</span>
                </button>
              </li>
            ))}
          </ul>

          {tags.length > 0 && (
            <>
              <p className="mb-1 px-2 text-xs font-semibold uppercase tracking-wider opacity-40">
                Tags
              </p>
              <ul className="mb-2">
                {tags.map((tag) => {
                  const active = inKnowledge && filter.tag === tag;
                  return (
                    <li key={tag}>
                      <button
                        type="button"
                        onClick={() => showKnowledge({ tag: active ? "all" : tag })}
                        className={row(active)}
                      >
                        <span className="text-base-content/40">#</span>
                        <span className="flex-1 truncate text-left">{tag}</span>
                        <span className="text-xs opacity-40">{tagCount(tag)}</span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            </>
          )}
        </div>
      </div>
      <div className="flex items-center gap-1 border-t border-base-300 px-2 py-2">
        <SettingsButton />
        <HelpButton />
      </div>
    </nav>
  );
}

/** The closed state: one icon per view, Settings at the bottom. */
export function IconRail() {
  const section = useSection();
  const counts = useCounts();
  const navigate = useUIStore((s) => s.navigate);
  return (
    <nav
      aria-label="Navigation"
      className="flex h-full w-12 shrink-0 flex-col items-center border-r border-base-300 py-2"
    >
      <ul className="flex flex-col items-center gap-1">
        {VIEWS.map((v) => (
          <li key={v.id}>
            <button
              type="button"
              title={counts[v.id] !== undefined ? `${v.label} (${counts[v.id]})` : v.label}
              aria-label={v.label}
              aria-current={section === v.id ? "page" : undefined}
              onClick={() => navigate({ name: v.id })}
              className={`relative flex h-9 w-9 items-center justify-center rounded-md transition-colors ${
                section === v.id
                  ? "bg-base-300 text-base-content"
                  : "text-base-content/60 hover:bg-base-200 hover:text-base-content"
              }`}
            >
              <Icon icon={v.icon} className="h-5 w-5" aria-hidden="true" />
              {v.id === "learnings" && !!counts.learnings && (
                <span
                  aria-hidden="true"
                  className="absolute top-1.5 right-1.5 h-2 w-2 rounded-full bg-primary"
                />
              )}
            </button>
          </li>
        ))}
      </ul>
      <div className="mt-auto flex flex-col items-center gap-1 border-t border-base-300 pt-2">
        <HelpButton />
        <SettingsButton />
      </div>
    </nav>
  );
}
